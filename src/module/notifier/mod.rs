use super::{Message, Notification};
use crate::msgbus::BusTx;
use crate::config::Config;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

// Import the Discord and Slack notifiers
mod discord_notifier;
mod slack_notifier;

pub use discord_notifier::Discord;
pub use slack_notifier::Slack;

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn send_notification(&self, notification: &Notification) -> Result<()>;
}

pub struct NotificationSystem {
    notifiers: Vec<Box<dyn Notifier>>,
}

impl NotificationSystem {
    pub fn new(config: Arc<RwLock<Config>>) -> Self {
        let discord = Box::new(Discord::new(config.clone())) as Box<dyn Notifier>;
        let slack = Box::new(Slack::new(config.clone())) as Box<dyn Notifier>;

        Self {
            notifiers: vec![discord, slack],
        }
    }

    pub async fn run(&self, _tx: &BusTx<Message>, rx: &mut mpsc::Receiver<Message>) -> Result<()> {
        while let Some(message) = rx.recv().await {
            let notification = match message {
                Message::ToNotify(notification) => notification,
                _ => continue,
            };

            for notifier in &self.notifiers {
                notifier.send_notification(&notification).await?;
            }
        }

        Ok(())
    }
}
