use super::{Message, Notification};
use crate::config::Config;
use crate::msgbus::BusTx;
use anyhow::{Context, Result};
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

/// A trait for notifiers that use webhooks for sending notifications.
#[async_trait]
pub trait WebhookNotifier: Notifier {
    type Config: HasWebhookUrl + Sync;

    /// Gets the webhook URL from either the URL option or from a file, prioritizing the file option
    async fn get_webhook_url(cfg: &Self::Config) -> Result<String> {
        if let Some(file) = cfg.webhook_url_file() {
            let url = tokio::fs::read_to_string(file)
                .await
                .with_context(|| format!("Failed to read webhook URL from file: {}", file))?;
            return Ok(url.trim().to_string());
        }

        if let Some(url) = cfg.webhook_url() {
            return Ok(url.clone());
        }

        Err(anyhow::anyhow!("No webhook URL configured"))
    }
}

/// A trait for config types that contain webhook URL fields
pub trait HasWebhookUrl {
    fn webhook_url(&self) -> Option<&String>;
    fn webhook_url_file(&self) -> Option<&String>;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[derive(Debug)]
    struct TestConfig {
        webhook_url: Option<String>,
        webhook_url_file: Option<String>,
    }

    impl HasWebhookUrl for TestConfig {
        fn webhook_url_file(&self) -> Option<&String> {
            self.webhook_url_file.as_ref()
        }

        fn webhook_url(&self) -> Option<&String> {
            self.webhook_url.as_ref()
        }
    }

    struct TestNotifier;

    #[async_trait]
    impl Notifier for TestNotifier {
        async fn send_notification(&self, _notification: &Notification) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl WebhookNotifier for TestNotifier {
        type Config = TestConfig;
    }

    #[tokio::test]
    async fn test_webhook_notifier_trait() {
        let cfg = TestConfig {
            webhook_url: Some("https://example.com/from_string".to_string()),
            webhook_url_file: None,
        };
        assert_eq!(
            TestNotifier::get_webhook_url(&cfg).await.unwrap(),
            "https://example.com/from_string"
        );

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "https://example.com/from_file").unwrap();
        let cfg = TestConfig {
            webhook_url: None,
            webhook_url_file: Some(file.path().to_str().unwrap().to_string()),
        };
        assert_eq!(
            TestNotifier::get_webhook_url(&cfg).await.unwrap(),
            "https://example.com/from_file"
        );

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "https://example.com/from_file").unwrap();
        let cfg = TestConfig {
            webhook_url: Some("https://example.com/from_string".to_string()),
            webhook_url_file: Some(file.path().to_str().unwrap().to_string()),
        };
        assert_eq!(
            TestNotifier::get_webhook_url(&cfg).await.unwrap(),
            "https://example.com/from_file"
        );

        let cfg = TestConfig {
            webhook_url: None,
            webhook_url_file: None,
        };
        assert!(TestNotifier::get_webhook_url(&cfg).await.is_err());
    }
}
