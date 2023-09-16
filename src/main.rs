use anyhow::{Context, Result};
use config::Config;
use state::update_stream_states;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod config;
mod state;

fn main() -> Result<()> {
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish(),
    )
    .context("Failed to set default subscriber")?;

    let mut config = Config::load()?;

    update_stream_states(&mut config.streams)?;

    config.streams.iter().for_each(|stream| {
        info!("{}: {:?}", stream.name, stream.online_since);
    });

    Ok(())
}
