use super::Notifier;
use crate::{config::Config, module::{Notification, TaskStatus}, APP_USER_AGENT};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Slack {
    config: Arc<RwLock<Config>>,
    client: Client,
}

#[derive(Serialize)]
struct SlackMessage {
    text: String,
    attachments: Vec<SlackAttachment>,
}

#[derive(Serialize)]
struct SlackAttachment {
    fallback: String,
    color: String,
    pretext: String,
    title: String,
    title_link: String,
    image_url: String,
}

impl Slack {
    pub fn new(config: Arc<RwLock<Config>>) -> Self {
        let client = Client::builder()
            .user_agent(APP_USER_AGENT)
            .build()
            .expect("Failed to create client");
        Self { config, client }
    }
}

#[async_trait]
impl Notifier for Slack {
    async fn send_notification(&self, notification: &Notification) -> Result<()> {
        let cfg = {
            let cfg = self.config.read().await;
            let not = cfg.notifier.clone();
            match (|| not?.slack)() {
                Some(cfg) => cfg,
                None => return Ok(()),
            }
        };

        if !cfg.notify_on.contains(&notification.status) {
            debug!("Not notifying on status {:?}", notification.status);
            return Ok(());
        }

        let (pretext, color) = match notification.status {
            TaskStatus::Waiting => ("Waiting for Live", "#ebd045"),
            TaskStatus::Recording => ("Recording", "#58b9ff"),
            TaskStatus::Done => ("Done", "#45eb45"),
            TaskStatus::Failed => ("Failed", "#eb4545"),
        };

        let message = SlackMessage {
            text: "".into(),
            attachments: vec![SlackAttachment {
                fallback: format!("{} - {}", pretext, notification.task.title),
                color: color.into(),
                pretext: pretext.into(),
                title: format!("{} - {}", notification.task.channel_name, notification.task.title.clone()),
                title_link: format!("https://youtu.be/{}", notification.task.video_id),
                image_url: notification.task.video_picture.clone(),
            }],
        };

        let res = self
            .client
            .post(&cfg.webhook_url)
            .header("Content-Type", "application/json")
            .json(&message)
            .send()
            .await;

        match res {
            Ok(res) => {
                if res.status().is_success() {
                    debug!("Sent Slack webhook");
                } else {
                    error!("Failed to send Slack webhook: {}", res.status());
                }
            }
            Err(e) => error!("Failed to send Slack webhook: {}", e),
        }

        Ok(())
    }
}
