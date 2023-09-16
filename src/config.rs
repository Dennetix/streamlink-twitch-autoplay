use std::{
    fs::File,
    io::{BufReader, Write},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamlinkConfig {
    pub streamlink_exec_name: String,
    pub streamlink_args: Vec<String>,
    pub player_exec_name: String,
    pub player_args: Vec<String>,
}

impl Default for StreamlinkConfig {
    fn default() -> Self {
        Self {
            streamlink_exec_name: String::from("streamlink"),
            streamlink_args: vec![String::from("--twitch-low-latency")],
            player_exec_name: String::from("mpv"),
            player_args: vec![],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamConfig {
    pub name: String,
    #[serde(default)]
    pub probability: f32,
    #[serde(skip)]
    pub online_since: Option<DateTime<Local>>,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            name: String::from(""),
            probability: 1.0,
            online_since: None,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub streamlink_config: StreamlinkConfig,
    pub streams: Vec<StreamConfig>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let mut config_path =
            dirs::config_dir().context("Couldn't find config directory for platform")?;
        config_path.push("streamlink-twitch-autoplay.json");

        let config_exist = config_path
            .try_exists()
            .with_context(|| format!("Couldn't access config file {}", config_path.display()))?;

        if config_exist {
            let config_file = File::open(config_path.as_path())
                .with_context(|| format!("Failed to open config file {}", config_path.display()))?;

            Ok(serde_json::from_reader(BufReader::new(config_file))
                .context("Failed to parse config")?)
        } else {
            info!("Config file not found. Creating default config.");

            let config = Self::default();

            let mut config_file = File::create(config_path.as_path()).with_context(|| {
                format!("Failed to crate config file {}", config_path.display())
            })?;
            config_file
                .write_all(serde_json::to_string_pretty(&config)?.as_bytes())
                .context("Failed to write default config file")?;

            Ok(config)
        }
    }
}
