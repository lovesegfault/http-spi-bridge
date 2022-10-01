mod bridge;
mod spi;

use anyhow::{Context, Result};
use axum::{Extension, Router};
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use std::{net::SocketAddr, path::PathBuf};

#[derive(Debug, Parser)]
struct Cli {
    #[clap(short, long, value_parser, default_value = "/dev/spidev0.0")]
    device: PathBuf,
    #[clap(short, long, value_parser, default_value = "127.0.0.1:8000")]
    addr: SocketAddr,
    #[clap(short, long, value_parser, default_value = "300000")]
    speed: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    // setup the tracing subscriber to enable logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .with_context(|| "Unable to set global default subscriber")?;
    info!("Starting");

    let cli = Cli::parse();

    // canonicalize the spi path to ensure it exists
    let spi_path = cli
        .device
        .canonicalize()
        .with_context(|| format!("invalid path {:?} for SPI device", cli.device))?;
    info!("Opening {:?}", spi_path);

    // create the spi device, see src/spi.rs
    let spi = spi::Spi::new(spi_path, cli.speed)
        .await
        .with_context(|| "failed to open SPI bus")?;
    info!("Configured SPI bus");

    info!("Serving on {}", cli.addr);
    let app = Router::new()
        .route("/update_raw", axum::routing::post(bridge::write_data))
        .layer(Extension(spi));

    axum::Server::bind(&cli.addr)
        .serve(app.into_make_service())
        .await
        .with_context(|| "HTTP server failed")?;

    info!("Exiting");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}
