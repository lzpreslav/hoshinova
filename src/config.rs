use crate::module::TaskStatus;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, TS, Serialize, Deserialize, Debug)]
#[ts(export)]
pub struct Config {
    #[serde(default)]
    pub ytarchive: YtarchiveConfig,
    #[serde(default)]
    pub scraper: ScraperConfig,
    pub notifier: Option<NotifierConfig>,
    #[serde(default)]
    pub webserver: Option<WebserverConfig>,
    #[serde(default)]
    pub channel: Vec<ChannelConfig>,

    #[serde(skip)]
    #[ts(skip)]
    config_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ytarchive: YtarchiveConfig::default(),
            scraper: ScraperConfig::default(),
            notifier: None,
            webserver: None,
            channel: Vec::new(),
            config_path: String::new(),
        }
    }
}

#[derive(Clone, TS, Serialize, Deserialize, Debug, PartialEq)]
#[ts(export)]
pub struct YtarchiveConfig {
    pub executable_path: String,
    pub working_directory: String,
    pub args: Vec<String>,
    pub quality: String,
    #[serde(with = "humantime_serde")]
    #[serde(default = "default_delay_start")]
    #[ts(type = "string")]
    pub delay_start: std::time::Duration,
}

impl Default for YtarchiveConfig {
    fn default() -> Self {
        YtarchiveConfig {
            executable_path: String::default(),
            working_directory: String::default(),
            args: Vec::default(),
            quality: String::default(),
            delay_start: std::time::Duration::default(),
        }
    }
}

fn default_delay_start() -> std::time::Duration {
    std::time::Duration::from_secs(1)
}

#[derive(Clone, TS, Serialize, Deserialize, Debug, PartialEq)]
#[ts(export)]
pub struct ScraperConfig {
    #[serde(default)]
    pub rss: ScraperRSSConfig,
}

impl Default for ScraperConfig {
    fn default() -> Self {
        ScraperConfig {
            rss: ScraperRSSConfig::default(),
        }
    }
}

#[derive(Clone, TS, Serialize, Deserialize, Debug, PartialEq)]
#[ts(export)]
pub struct ScraperRSSConfig {
    #[serde(with = "humantime_serde")]
    #[ts(type = "string")]
    pub poll_interval: std::time::Duration,
    #[serde(with = "humantime_serde")]
    #[serde(default = "default_ignore_older_than")]
    #[ts(type = "string")]
    pub ignore_older_than: std::time::Duration,
}

fn default_ignore_older_than() -> std::time::Duration {
    std::time::Duration::from_secs(60 * 60 * 24)
}

impl Default for ScraperRSSConfig {
    fn default() -> Self {
        ScraperRSSConfig {
            poll_interval: std::time::Duration::default(),
            ignore_older_than: std::time::Duration::default(),
        }
    }
}

#[derive(Clone, TS, Serialize, Deserialize, Debug, PartialEq)]
#[ts(export)]
pub struct NotifierConfig {
    pub discord: Option<DiscordConfig>,
    pub slack: Option<SlackConfig>,
}

#[derive(Clone, TS, Serialize, Deserialize, Debug, PartialEq)]
#[ts(export)]
pub struct DiscordConfig {
    pub webhook_url: String,
    pub notify_on: Vec<TaskStatus>,
}

#[derive(Clone, TS, Serialize, Deserialize, Debug, PartialEq)]
#[ts(export)]
pub struct SlackConfig {
    pub webhook_url: String,
    pub notify_on: Vec<TaskStatus>,
}

#[derive(Clone, TS, Serialize, Deserialize, Debug)]
#[ts(export)]
pub struct WebserverConfig {
    pub bind_address: Option<String>,
    pub unix_path: Option<String>,
}

impl Default for WebserverConfig {
    fn default() -> Self {
        WebserverConfig {
            bind_address: None,
            unix_path: None,
        }
    }
}

#[derive(Clone, TS, Serialize, Deserialize, Debug)]
#[ts(export)]
pub struct ChannelConfig {
    pub id: String,
    pub name: String,
    #[serde(with = "serde_regex")]
    #[ts(type = "string[]")]
    pub filters: Vec<regex::Regex>,
    #[serde(default)]
    pub match_description: bool,
    pub outpath: String,
    /// If not present, will be fetched during runtime.
    pub picture_url: Option<String>,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        ChannelConfig {
            id: String::default(),
            name: String::default(),
            filters: Vec::default(),
            match_description: bool::default(),
            outpath: String::default(),
            picture_url: Option::default(),
        }
    }
}

pub async fn load_config(path: &str) -> Result<Config> {
    let config = tokio::fs::read_to_string(path).await?;
    let mut config: Config = toml::from_str(&config)?;
    config.config_path = path.to_string();
    Ok(config)
}

impl Config {
    /// Reads the config file and replaces the current config with the new one.
    pub async fn reload(&mut self) -> Result<()> {
        info!("Reloading config");
        let config = load_config(&self.config_path)
            .await
            .context("Failed to load config")?;
        *self = config;
        Ok(())
    }

    /// Reads and returns the source TOML file from the config path. There are
    /// no guarantees that the returned TOML corresponds to the current config,
    /// as it might have been changed since the last time it was read.
    pub async fn get_source_toml(&self) -> Result<String> {
        tokio::fs::read_to_string(&self.config_path)
            .await
            .map_err(|e| e.into())
    }

    /// Writes the provided TOML string to the config path, and reloads the
    /// config.
    pub async fn set_source_toml(&mut self, source_toml: &str) -> Result<()> {
        // Try to deserialize the provided TOML string. If it fails, we don't
        // want to write it to the config file.
        let _: Config =
            toml::from_str(source_toml).context("Failed to deserialize provided TOML")?;

        // Write the provided TOML string to the config file.
        tokio::fs::write(&self.config_path, source_toml)
            .await
            .context("Failed to write config file")?;

        // Reload the config.
        self.reload().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.notifier.is_none());
        assert!(config.webserver.is_none());
        assert!(config.channel.is_empty());
        assert!(config.config_path.is_empty());
    }

    #[test]
    fn test_ytarchive_config_default() {
        let yt = YtarchiveConfig::default();
        assert!(yt.executable_path.is_empty());
        assert!(yt.working_directory.is_empty());
        assert!(yt.args.is_empty());
        assert!(yt.quality.is_empty());
        assert_eq!(yt.delay_start, Duration::default());
    }

    #[test]
    fn test_scraper_config_default() {
        let scraper = ScraperConfig::default();
        assert_eq!(scraper.rss.poll_interval, Duration::default());
        assert_eq!(scraper.rss.ignore_older_than, Duration::default());
    }

    #[test]
    fn test_webserver_config_default() {
        let ws = WebserverConfig::default();
        assert!(ws.bind_address.is_none());
        assert!(ws.unix_path.is_none());
    }

    #[test]
    fn test_channel_config_default() {
        let ch = ChannelConfig::default();
        assert!(ch.id.is_empty());
        assert!(ch.name.is_empty());
        assert!(ch.filters.is_empty());
        assert!(!ch.match_description);
        assert!(ch.outpath.is_empty());
        assert!(ch.picture_url.is_none());
    }

    #[test]
    fn test_deserialize_ytarchive_with_missing_delay_start() {
        let toml_str = r#"
            executable_path = "/usr/bin/ytarchive"
            working_directory = "/tmp"
            args = ["--merge"]
            quality = "best"
        "#;

        let config: YtarchiveConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.delay_start, Duration::from_secs(1));
    }

    #[test]
    fn test_deserialize_scraper_rss_with_missing_ignore_older_than() {
        let toml_str = r#"
            poll_interval = "30s"
        "#;

        let config: ScraperRSSConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.ignore_older_than, Duration::from_secs(60 * 60 * 24));
    }

    #[test]
    fn test_notifier_with_only_discord() {
        let toml_str = r#"
            [notifier.discord]
            webhook_url = "http://discord.example.com"
            notify_on = ["recording"]
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.notifier.is_some());
        assert!(config.notifier.as_ref().unwrap().discord.is_some());
        assert!(config.notifier.as_ref().unwrap().slack.is_none());
    }

    #[test]
    fn test_deserialize_channel_config() {
        let toml_str = r#"
            [[channel]]
            id = "123"
            name = "Test Channel 1"
            filters = ["(?i)test", "another"]
            match_description = true
            outpath = "./downloads1"
            picture_url = "http://example.com/pic1.jpg"

            [[channel]]
            id = "456"
            name = "Test Channel 2"
            filters = [".*?"]
            outpath = "./downloads2"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();

        let channel1 = &config.channel[0];
        assert_eq!(channel1.id, "123");
        assert_eq!(channel1.name, "Test Channel 1");
        assert_eq!(channel1.filters.len(), 2);
        assert!(channel1.filters[0].is_match("tEsT"));
        assert!(channel1.filters[1].is_match("another test"));
        assert_eq!(channel1.outpath, "./downloads1");
        assert_eq!(channel1.match_description, true);
        assert_eq!(channel1.picture_url, Some("http://example.com/pic1.jpg".to_string()));

        let channel2 = &config.channel[1];
        assert_eq!(channel2.id, "456");
        assert_eq!(channel2.name, "Test Channel 2");
        assert_eq!(channel2.filters.len(), 1);
        assert!(channel2.filters[0].is_match("anything"));
        assert_eq!(channel2.outpath, "./downloads2");
        assert_eq!(channel2.picture_url, None);
        assert_eq!(channel2.match_description, false);
    }

    #[test]
    fn test_deserialize_config_with_partial_fields() {
        let toml_str = r#"
            [ytarchive]
            executable_path = "/bin/test"
            working_directory = "/downloads"
            args = ["--test"]
            quality = "high"
            delay_start = "5s"

            [scraper.rss]
            poll_interval = "10s"

            [[channel]]
            id = "123"
            name = "Test Channel"
            filters = ["title.*"]
            outpath = "./downloads"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();

        assert_eq!(config.ytarchive.delay_start, Duration::from_secs(5));
        assert_eq!(config.scraper.rss.poll_interval, Duration::from_secs(10));
        assert_eq!(config.channel.len(), 1);
        assert_eq!(config.channel[0].match_description, false);
        assert_eq!(config.notifier, None);
    }

    #[tokio::test]
    async fn test_reload_invalid_config() {
        let mut config = Config::default();
        // Simulate a corrupted config file by writing invalid TOML
        let invalid_toml = "lalala";
        let _ = tokio::fs::write(&config.config_path, invalid_toml).await;

        let result = config.reload().await;
        assert!(result.is_err());
    }
}
