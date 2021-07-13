use anyhow::{Context, Result};
use spidev::{Spidev, SpidevOptions};
use tokio::{fs::File, io::AsyncReadExt, process::Command, sync::Mutex};
use tracing::{debug, error};

use std::{io::prelude::*, path::Path, process::Stdio, sync::Arc};

#[derive(Clone)]
pub struct Spi {
    dev: Arc<Mutex<Spidev>>,
}

impl Spi {
    pub const DATA_LEN: u16 = 7868;

    /// Check whether the `spidev` module exists.
    ///
    /// This will fail if `spidev` is linked into the kernel, or not available at all.
    #[tracing::instrument]
    async fn check_spidev_exists() -> Result<bool> {
        let status = Command::new("modinfo")
            .arg("spidev")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .with_context(|| "failed to run modinfo")?;

        // if modinfo returns 1, then spidev isn't a module at all.
        let exists = status.success();
        debug!(exists);
        Ok(exists)
    }

    /// Check whether the `spidev` module is loaded.
    #[tracing::instrument]
    async fn check_spidev_loaded() -> Result<bool> {
        let out = Command::new("lsmod")
            .output()
            .await
            .with_context(|| "failed to run lsmod")?;

        if !out.status.success() {
            anyhow::bail!("lsmod exited with status {}", out.status);
        }

        let stdout = String::from_utf8_lossy(&out.stdout);

        let loaded = stdout.contains("spidev");
        debug!(loaded);
        Ok(loaded)
    }

    /// Retrieves the current `spidev` buffer size (`bufsiz`) parameter from
    /// `/sys/module/spidev/parameters/bufsiz`.
    #[tracing::instrument]
    async fn get_spidev_buffer_size() -> Result<u16> {
        let file = File::open("/sys/module/spidev/parameters/bufsiz")
            .await
            .with_context(|| "failed to open spidev bufsiz file")?;

        let mut contents = Vec::new();

        // u16::LIMIT is 65535, which has 5 characters, and so we only want to read at most 5 bytes.
        const U16_MAX_CHARS: u64 = 5;
        file.take(U16_MAX_CHARS)
            .read_to_end(&mut contents)
            .await
            .with_context(|| "failed to read spidev buffer size")?;

        let buffer_size: u16 = String::from_utf8_lossy(&contents)
            .trim()
            .parse()
            .with_context(|| "failed to parse spidev buffer size")?;

        debug!(buffer_size);
        Ok(buffer_size)
    }

    /// Unloads the `spidev` kernel module, if loaded.
    #[tracing::instrument]
    async fn unload_spidev() -> Result<()> {
        // if the module isn't loaded we don't even try to do anything
        if !Self::check_spidev_loaded().await? {
            return Ok(());
        }

        let out = Command::new("modprobe")
            .arg("-r")
            .arg("spidev")
            .output()
            .await
            .with_context(|| "failed to unload spidev")?;

        if !out.status.success() {
            let e = String::from_utf8_lossy(&out.stderr);
            error!("failed to unload spidev: {}", e);
            anyhow::bail!("failed to unload spidev: {}", e);
        }

        debug!("unloaded spidev");
        Ok(())
    }

    /// Loads the `spidev` kernel module, optionally applying the buffer size (`bufsiz`) parameter
    /// on load.
    #[tracing::instrument]
    async fn load_spidev(buffer_size: Option<u16>) -> Result<()> {
        let mut cmd = Command::new("modprobe");
        cmd.arg("spidev");
        if let Some(size) = buffer_size {
            cmd.arg(format!("bufsiz={}", size));
        }

        let out = cmd
            .output()
            .await
            .with_context(|| "failed to load spidev")?;

        if !out.status.success() {
            let e = String::from_utf8_lossy(&out.stderr);
            error!("failed to load spidev: {}", e);
            anyhow::bail!("failed to load spidev: {}", e);
        }

        debug!("loaded spidev");
        Ok(())
    }

    /// Sets the `spidev` buffer size (`bufsiz`) parameter to the specified value.
    ///
    /// This works by unloading the module, and reloading with the parameter specified, and only
    /// works if `spidev` is built as a module and not linked into the kernel.
    #[tracing::instrument]
    async fn set_spidev_buffer_size(size: u16) -> Result<()> {
        // First we check what the buffer size of spidev is currently, if it's already correct we
        // don't need to do anything
        if size == Self::get_spidev_buffer_size().await? {
            return Ok(());
        };
        // Next we check whether spidev is a module, or whether it was linked into the kernel. If
        // it isn't a module there's nothing we can do to change the buffer size.
        anyhow::ensure!(Self::check_spidev_exists().await?);
        // Since we need to change the module's parameters, we need to unload it
        Self::unload_spidev().await?;
        // And now we can re-load it with the right size
        Self::load_spidev(Some(size)).await?;
        // Finally, we check whether the module has the right bufsiz now
        anyhow::ensure!(Self::get_spidev_buffer_size().await? == size);

        Ok(())
    }

    /// Opens the SPI device at `path` and configures it.
    pub async fn new<P: AsRef<Path>>(path: P, speed: u32) -> Result<Self> {
        Self::set_spidev_buffer_size(Self::DATA_LEN)
            .await
            .with_context(|| "failed to configure SPI device's bufsiz")?;

        let mut dev = Spidev::open(&path).with_context(|| "failed to open SPI bus")?;
        // c.f. https://www.bootc.net/archives/2012/05/19/spi-on-the-raspberry-pi-again/
        // newer pi's should support faster speeds, but I don't want to figure out what the limit
        // is.
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(speed)
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
