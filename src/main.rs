mod bridge;
mod spi;

use anyhow::{Context, Result};
use axum::{AddExtensionLayer, Router};
use structopt::StructOpt;
use tracing::{info, Level};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use std::{net::SocketAddr, path::PathBuf};

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short, long, default_value = "/dev/spidev0.0", parse(from_os_str))]
    device: PathBuf,
    #[structopt(short, long, default_value = "127.0.0.1:8000")]
    addr: SocketAddr,
    #[structopt(short, long, default_value = "300000")]
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

    let opt = Opt::from_args_safe().with_context(|| "failed to parse command line arguments")?;

    // canonicalize the spi path to ensure it exists
    let spi_path = opt
        .device
        .canonicalize()
        .with_context(|| format!("invalid path {:?} for SPI device", opt.device))?;
    info!("Opening {:?}", spi_path);

    // create the spi device, see src/spi.rs
    let spi = spi::Spi::new(spi_path, opt.speed)
        .await
        .with_context(|| "failed to open SPI bus")?;
    info!("Configured SPI bus");

    info!("Serving on {}", opt.addr);
    let app = Router::new()
        .route("/update_raw", axum::routing::post(bridge::write_data))
        .layer(AddExtensionLayer::new(spi));

    axum::Server::bind(&opt.addr)
        .serve(app.into_make_service())
        .await
        .with_context(|| "HTTP server failed")?;

    info!("Exiting");
    Ok(())
}
