use std::{
    process::{Child, Command},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use config::{Config, StreamConfig, StreamlinkConfig};
use rand::{distributions::WeightedIndex, prelude::Distribution, thread_rng};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod config;

struct StreamlinkProcess {
    stream: StreamConfig,
    process: Child,
}

impl Drop for StreamlinkProcess {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

fn main() -> Result<()> {
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish(),
    )
    .context("Failed to set default subscriber")?;

    let mut config;
    let mut rng = thread_rng();

    let mut last_updated = Instant::now();
    let mut streamlink_process: Option<StreamlinkProcess> = None;

    loop {
        // Check if the streamlink process has exited every 5 seconds
        if let Some(StreamlinkProcess { process, .. }) = &mut streamlink_process {
            if let Ok(Some(_)) = process.try_wait() {
                streamlink_process = None;
            }
        }

        // Update only if no stream is running or every 90 seconds
        if streamlink_process.is_none() || last_updated.elapsed() > Duration::from_secs(90) {
            last_updated = Instant::now();

            config = Config::load()?;

            let online_streams = config
                .streams
                .iter()
                .filter(|stream| stream.online_since.is_some())
                .collect::<Vec<_>>();

            info!(
                "Online streams: {}",
                online_streams
                    .iter()
                    .map(|stream| stream.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            if !online_streams.is_empty() {
                // Choose the first stream with weight zero
                let mut stream = online_streams
                    .iter()
                    .find(|stream| stream.weight == 0)
                    .cloned();

                // If there is no stream with weight zero randomly choose one by weight
                // Only do this if no stream is currently running to not end up constantly switching streams
                if stream.is_none() && streamlink_process.is_none() {
                    let weights = online_streams.iter().map(|stream| stream.weight);
                    let dist = WeightedIndex::new(weights)?;
                    stream = Some(online_streams[dist.sample(&mut rng)]);
                }

                if let Some(stream) = stream {
                    // If a stream is currently running only stop it if the new one is different
                    let should_open = if let Some(streamlink_process) = &mut streamlink_process {
                        streamlink_process.stream.name != stream.name
                    } else {
                        true
                    };

                    if should_open {
                        streamlink_process = Some(StreamlinkProcess {
                            stream: stream.clone(),
                            process: spawn_streamlink_process(
                                &stream.name,
                                &config.streamlink_config,
                            )?,
                        });
                    }
                }
            }
        }

        std::thread::sleep(Duration::from_secs(5));
    }
}

fn spawn_streamlink_process(name: &str, config: &StreamlinkConfig) -> Result<Child> {
    let mut command = Command::new(&config.streamlink_exec_name);
    command
        .args(&config.streamlink_args)
        .arg(format!("twitch.tv/{name}"))
        .arg(&config.streamlink_quality)
        .arg("--player")
        .arg(&config.player_exec_name);

    if !config.player_args.is_empty() {
        command
            .arg("--player-args")
            .arg(&config.player_args.join(" "));
    }

    info!("Starting streamlink process: {:?}", command);

    command
        .spawn()
        .context("Failed to spawn streamlink process.")
}
