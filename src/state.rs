use anyhow::Result;
use chrono::{DateTime, Local};
use serde_json::json;
use tracing::error;

use crate::config::StreamConfig;

pub fn update_stream_states(streams: &mut [StreamConfig]) -> Result<()> {
    let operations = streams.iter().map(|stream| {
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

    let body = serde_json::to_string_pretty(&operations)?;

    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://gql.twitch.tv/gql")
        .header("Client-id", "kimne78kx3ncx6brgo4mv6wki5h1ko")
        .body(body)
        .send()?;

    let results = serde_json::from_str::<Vec<serde_json::Value>>(&res.text()?)?;

    results
        .iter()
        .zip(streams)
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
