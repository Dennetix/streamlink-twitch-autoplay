use std::{
    fs::File,
    io::{BufReader, Write},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamlinkConfig {
    pub streamlink_exec_name: String,
    pub streamlink_quality: String,
    pub streamlink_args: Vec<String>,
    pub player_exec_name: String,
    pub player_args: Vec<String>,
}

impl Default for StreamlinkConfig {
    fn default() -> Self {
        Self {
            streamlink_exec_name: String::from("streamlink"),
            streamlink_quality: String::from("best"),
            streamlink_args: vec![String::from("--twitch-low-latency")],
            player_exec_name: String::from("mpv"),
            player_args: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub name: String,
    #[serde(default = "default_probability")]
    pub probability: f32,
    #[serde(skip)]
    pub online_since: Option<DateTime<Local>>,
}

fn default_probability() -> f32 {
    1.0
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub streamlink_config: StreamlinkConfig,
    pub streams: Vec<StreamConfig>,
}

impl Config {
    pub fn load() -> Result<Self> {
        info!("Loading config...");

        let mut config_path =
            dirs::config_dir().context("Couldn't find config directory for platform")?;
        config_path.push("streamlink-twitch-autoplay.json");

        let config_exist = config_path
            .try_exists()
            .with_context(|| format!("Couldn't access config file {}", config_path.display()))?;

        if config_exist {
            let config_file = File::open(config_path.as_path())
                .with_context(|| format!("Failed to open config file {}", config_path.display()))?;

            let mut config = serde_json::from_reader::<_, Config>(BufReader::new(config_file))
                .context("Failed to parse config")?;

            config.update_stream_states()?;
            Ok(config)
        } else {
            info!("Config file not found. Creating default config.");

            let config = Self::default();

            let mut config_file = File::create(config_path.as_path()).with_context(|| {
                format!(
                    "Failed to create default config file {}",
                    config_path.display()
                )
            })?;
            config_file
                .write_all(serde_json::to_string_pretty(&config)?.as_bytes())
                .context("Failed to write default config file")?;

            Ok(config)
        }
    }

    pub fn update_stream_states(&mut self) -> Result<()> {
        info!("Updating stream states...");

        // Twitch has no public api to query stream state without a client id, so we use the
        // GraphQL query that their client is using
        let operations = self.streams.iter().map(|stream| {
            json!({
                "operationName": "UseLive",
                "variables": {
                    "channelLogin": stream.name
                },
                "extensions": {
                    "persistedQuery": {
                        "version": 1,
                        "sha256Hash": "639d5f11bfb8bf3053b424d9ef650d04c4ebb7d94711d644afb08fe9a0fad5d9"
                    }
                }
            })
        }).collect::<Vec<_>>();

        // The client id used in this request is the same that twitch is using for everybody
        let response = ureq::post("https://gql.twitch.tv/gql")
            .set("Client-id", "kimne78kx3ncx6brgo4mv6wki5h1ko")
            .send_json(operations)
            .context("Failed to sed GraphQL query.")?
            .into_json::<Vec<serde_json::Value>>()
            .context("Failed to parse GraphQL query response.")?;

        response
            .iter()
            .zip(&mut self.streams)
            .for_each(|(json, stream_config)| {
                let stream = json
                    .get("data")
                    .and_then(|data| data.get("user"))
                    .and_then(|user| user.get("stream"));
                if let Some(stream) = stream {
                    let time_utc = stream
                        .get("createdAt")
                        .and_then(|created_at| created_at.as_str())
                        .and_then(|created_at| DateTime::parse_from_rfc3339(created_at).ok());
                    if let Some(time_utc) = time_utc {
                        stream_config.online_since = Some(DateTime::<Local>::from(time_utc));
                    }
                } else {
                    error!("User {} does not exist!", stream_config.name);
                }
            });

        Ok(())
    }
}
