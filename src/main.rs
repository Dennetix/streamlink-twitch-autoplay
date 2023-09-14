use anyhow::{Context, Result};
use config::Config;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod config;

fn main() -> Result<()> {
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish(),
    )
    .context("Failed to set default subscriber")?;

    let config = Config::load();

    info!("{config:?}");

    Ok(())
}
