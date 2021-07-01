mod bridge;
mod spi;

use anyhow::{Context, Result};
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
}

#[rocket::main]
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
    let spi = spi::Spi::new(spi_path)
        .await
        .with_context(|| "failed to open SPI bus")?;
    info!("Configured SPI bus");

    info!("Serving on {}", opt.addr);
    rocket::build()
        .configure(rocket::Config {
            port: opt.addr.port(),
            address: opt.addr.ip(),
            ..rocket::Config::default()
        })
        .attach(bridge::stage(spi))
        .launch()
        .await
        .with_context(|| "HTTP server failed to launch")?;

    info!("Exiting");
    Ok(())
}
