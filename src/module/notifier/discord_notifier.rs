use super::{Notifier, WebhookNotifier};
use crate::{
    config::Config,
    module::{Notification, TaskStatus},
    APP_NAME, APP_USER_AGENT,
};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Discord {
    config: Arc<RwLock<Config>>,
    client: Client,
}

#[derive(Serialize)]
struct WebhookMessage {
    content: String,
    embeds: Vec<DiscordEmbed>,
}

#[derive(Serialize)]
struct DiscordEmbed {
    title: String,
    description: String,
    color: u32,
    author: DiscordEmbedAuthor,
    footer: DiscordEmbedFooter,
    timestamp: String,
    thumbnail: DiscordEmbedThumbnail,
}

#[derive(Serialize)]
struct DiscordEmbedAuthor {
    name: String,
    url: String,
    icon_url: Option<String>,
}

#[derive(Serialize)]
struct DiscordEmbedFooter {
    text: String,
}

#[derive(Serialize)]
struct DiscordEmbedThumbnail {
    url: String,
}

impl Discord {
    pub fn new(config: Arc<RwLock<Config>>) -> Self {
        let client = Client::builder()
            .user_agent(APP_USER_AGENT)
            .build()
            .expect("Failed to create client");
        Self { config, client }
    }
}

#[async_trait]
impl WebhookNotifier for Discord {
    type Config = crate::config::DiscordConfig;
}

#[async_trait]
impl Notifier for Discord {
    async fn send_notification(&self, notification: &Notification) -> Result<()> {
        let cfg = {
            let cfg = self.config.read().await;
            let not = cfg.notifier.clone();
            match (|| not?.discord)() {
                Some(cfg) => cfg,
                None => return Ok(()), // Skip if no Discord config
            }
        };

        if !cfg.notify_on.contains(&notification.status) {
            debug!("Not notifying on status {:?}", notification.status);
            return Ok(());
        }

        let webhook_url = Self::get_webhook_url(&cfg).await?;

        let (title, color) = match notification.status {
            TaskStatus::Waiting => ("Waiting for Live", 0xebd045),
            TaskStatus::Recording => ("Recording", 0x58b9ff),
            TaskStatus::Done => ("Done", 0x45eb45),
            TaskStatus::Failed => ("Failed", 0xeb4545),
        };
        let timestamp = chrono::Utc::now().to_rfc3339();

        let message = WebhookMessage {
            content: "".into(),
            embeds: vec![DiscordEmbed {
                title: title.into(),
                description: format!(
                    "[{}](https://youtu.be/{})",
                    notification.task.title, notification.task.video_id
                ),
                color,
                author: DiscordEmbedAuthor {
                    name: notification.task.channel_name.clone(),
                    url: format!(
                        "https://www.youtube.com/channel/{}",
                        notification.task.channel_id
                    ),
                    icon_url: notification.task.channel_picture.clone(),
                },
                footer: DiscordEmbedFooter {
                    text: APP_NAME.into(),
                },
                timestamp: timestamp,
                thumbnail: DiscordEmbedThumbnail {
                    url: notification.task.video_picture.clone(),
                },
            }],
        };

        let res = self
            .client
            .post(&webhook_url)
            .header("Content-Type", "application/json")
            .json(&message)
            .send()
            .await;

        match res {
            Ok(res) => {
                if res.status().is_success() {
                    debug!("Sent Discord webhook");
                } else {
                    error!("Failed to send Discord webhook: {}", res.status());
                }
            }
            Err(e) => error!("Failed to send Discord webhook: {}", e),
        }

        Ok(())
    }
}
