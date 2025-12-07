use super::TaskMap;
use crate::{
    config::Config,
    module::{Message, Task},
    msgbus::BusTx,
    youtube,
};
use actix_web::{
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorForbidden},
    get, post, put,
    web::{self, Data},
    HttpResponse, Responder,
};
use anyhow::anyhow;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use ts_rs::TS;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web/dist"]
#[allow(dead_code)]
struct StaticFiles;

/// Configure routes for the webserver
pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(get_tasks)
        .service(post_task)
        .service(get_version)
        .service(get_config)
        .service(get_config_toml)
        .service(put_config_toml)
        .service(reload_config)
        .service(serve_static);
}

#[get("/api/tasks")]
async fn get_tasks(data: TaskMap) -> actix_web::Result<impl Responder> {
    Ok(HttpResponse::Ok().json(
        data.read()
            .await
            .iter()
            .map(|(_, v)| v.to_owned())
            .collect::<Vec<_>>(),
    ))
}

#[derive(Deserialize, TS)]
#[ts(export)]
struct CreateTaskRequest {
    video_url: String,
    output_directory: String,
}

#[post("/api/task")]
async fn post_task(
    tx: Data<BusTx<Message>>,
    taskreq: web::Json<CreateTaskRequest>,
) -> actix_web::Result<impl Responder> {
    let taskreq = taskreq.into_inner();
    let client = reqwest::Client::new();

    // Make sure the video URL is valid
    let url =
        youtube::URL::parse(&taskreq.video_url).map_err(|e| ErrorBadRequest(format!("{:?}", e)))?;
    let video_id = url
        .video_id()
        .ok_or(ErrorBadRequest(anyhow!("Not a video URL")))?;
    let video_url = format!("https://www.youtube.com/watch?v={}", video_id);

    // Fetch video details
    let ipr = youtube::video::fetch_initial_player_response(client.clone(), &video_url)
        .await
        .map_err(|e| ErrorInternalServerError(format!("{:?}", e)))?;

    // Get the best thumbnail
    let mut thumbs = ipr.video_details.thumbnail.thumbnails;
    thumbs.sort_by_key(|t| (t.width, t.height));
    let best_thumb = thumbs.last().map(|t| t.url.clone()).unwrap_or("".into());

    // Fetch the channel image
    let channel_picture =
        youtube::channel::fetch_picture_url(client, &ipr.video_details.channel_id)
            .await
            .map_err(|e| {
                ErrorInternalServerError(anyhow!("Failed to fetch channel picture: {:?}", e))
            })?;

    // Create the task
    let task = Task {
        title: ipr.video_details.title,
        video_id: ipr.video_details.video_id,
        video_picture: best_thumb,
        channel_name: ipr.video_details.author,
        channel_id: ipr.video_details.channel_id,
        channel_picture: Some(channel_picture),
        output_directory: taskreq.output_directory,
    };

    // Broadcast it to the bus
    tx.send(Message::ToRecord(task))
        .await
        .map_err(|e| ErrorInternalServerError(format!("{:?}", e)))?;

    Ok(HttpResponse::Accepted().finish())
}

#[get("/api/version")]
async fn get_version() -> actix_web::Result<impl Responder> {
    Ok(HttpResponse::Ok().body(crate::APP_NAME.to_owned()))
}

#[get("/api/config")]
async fn get_config(config: Data<Arc<RwLock<Config>>>) -> actix_web::Result<impl Responder> {
    Ok(HttpResponse::Ok().json(config.read().await.to_owned()))
}

#[post("/api/config/reload")]
async fn reload_config(config: Data<Arc<RwLock<Config>>>) -> actix_web::Result<impl Responder> {
    config
        .write()
        .await
        .reload()
        .await
        .map_err(|e| ErrorInternalServerError(format!("{:?}", e)))?;
    Ok(HttpResponse::Ok().json("ok"))
}

#[get("/api/config/toml")]
async fn get_config_toml(config: Data<Arc<RwLock<Config>>>) -> actix_web::Result<impl Responder> {
    Ok(HttpResponse::Ok().body(
        config
            .read()
            .await
            .get_source_toml()
            .await
            .map_err(|e| ErrorInternalServerError(format!("{:?}", e)))?,
    ))
}

#[put("/api/config/toml")]
async fn put_config_toml(
    config: Data<Arc<RwLock<Config>>>,
    body: web::Bytes,
) -> actix_web::Result<impl Responder> {
    {
        let guard = config.read().await;
        if let Some(webserver) = &guard.webserver {
            if !webserver.allow_config_edit {
                return Err(ErrorForbidden("Editing the config file is not allowed"));
            }
        }
    }

    let body = std::str::from_utf8(&body).map_err(|e| ErrorBadRequest(format!("{:?}", e)))?;
    config
        .write()
        .await
        .set_source_toml(body)
        .await
        .map_err(|e| ErrorBadRequest(format!("{:?}", e)))?;
    Ok(HttpResponse::Ok().json("ok"))
}

#[get("/{_:.*}")]
async fn serve_static(path: web::Path<String>) -> impl Responder {
    let mut path = path.into_inner();
    if path.is_empty() {
        path = "index.html".to_string();
    }

    // If debug mode, serve the files from the static folder
    #[cfg(debug_assertions)]
    return tokio::fs::read(format!("web/dist/{}", path))
        .await
        .map(|bytes| {
            HttpResponse::Ok()
                .content_type(mime_guess::from_path(path).first_or_octet_stream().as_ref())
                .body(bytes)
        })
        .unwrap_or_else(|_| HttpResponse::NotFound().body("404"));

    // Otherwise serve the files from the embedded folder
    #[cfg(not(debug_assertions))]
    return match StaticFiles::get(&path) {
        Some(content) => HttpResponse::Ok()
            .content_type(mime_guess::from_path(path).first_or_octet_stream().as_ref())
            .body(content.data.into_owned()),
        None => HttpResponse::NotFound().body("404 Not Found"),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use crate::config::WebserverConfig;

    #[actix_web::test]
    async fn test_put_config_toml_invalid() {
        let config = Arc::new(RwLock::new(Config::default()));
        let app = test::init_service(
            App::new()
                .app_data(Data::new(config.clone()))
                .service(put_config_toml),
        )
        .await;

        let mut config_guard = config.write().await;
        config_guard.webserver = Some(WebserverConfig {
            allow_config_edit: true,
            ..WebserverConfig::default()
        });
        drop(config_guard);

        let req = test::TestRequest::put()
            .uri("/api/config/toml")
            .set_payload("test")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 400); // Invalid config file, but API is allowed
    }

    #[actix_web::test]
    async fn test_put_config_toml_forbidden() {
        let config = Arc::new(RwLock::new(Config::default()));
        let app = test::init_service(
            App::new()
                .app_data(Data::new(config.clone()))
                .service(put_config_toml),
        )
        .await;

        let mut config_guard = config.write().await;
        config_guard.webserver = Some(WebserverConfig {
            allow_config_edit: false,
            ..WebserverConfig::default()
        });
        drop(config_guard);

        let req = test::TestRequest::put()
            .uri("/api/config/toml")
            .set_payload("test")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 403);
    }
}
