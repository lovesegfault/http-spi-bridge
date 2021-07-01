use anyhow::{Context, Result};
use rocket::tokio::sync::Mutex;
use spidev::{Spidev, SpidevOptions};
use tracing::debug;

use std::{io::prelude::*, path::Path, sync::Arc};

#[derive(Clone)]
pub struct Spi {
    dev: Arc<Mutex<Spidev>>,
}

impl Spi {
    /// Opens the SPI device at `path` and configures it.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut dev = Spidev::open(&path).with_context(|| "failed to open SPI bus")?;
        // c.f. https://www.bootc.net/archives/2012/05/19/spi-on-the-raspberry-pi-again/
        // newer pi's should support faster speeds, but I don't want to figure out what the limit
        // is.
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(250 * 10_u32.pow(6))
            .build();

        dev.configure(&options)
            .with_context(|| "failed to configure spi bus")?;

        let dev = Arc::new(Mutex::new(dev));

        Ok(Self { dev })
    }

    /// Writes a byte stream to the SPI device.
    ///
    /// Returns the number of bytes written.
    #[tracing::instrument(skip(self, data))]
    pub async fn write_data(&self, data: &[u8]) -> Result<usize> {
        debug!("writing {} bytes to SPI", data.len());
        self.dev
            .lock()
            .await
            .write(data)
            .with_context(|| "failed to write data to SPI bus")
    }
}
