use serde::{Deserialize, Serialize};
use tracing::{error, info};
use warp::{http, Filter};

use std::net::SocketAddr;

use crate::spi::Spi;

/// The JSON data we expect to receive via POST on the `update_raw` endpoint.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Data {
    #[serde(alias = "raw_data")]
    raw: Vec<u8>,
}

/// Wrapper struct for the HTTP server, holds the thread-safe SPI handler.
pub struct HttpServer {
    addr: SocketAddr,
    spi: Spi,
}

impl HttpServer {
    const DATA_LEN: usize = 7868;

    pub fn new(addr: SocketAddr, spi: Spi) -> Self {
        Self { addr, spi }
    }

    // this is just here to help the compiler infer the type we want the JSON to deserialize to
    // (Data)
    fn parse_json() -> impl Filter<Extract = (Data,), Error = warp::Rejection> + Clone {
        warp::body::json()
    }

    /// Handles incoming data on the update_raw endpoint, writing it to the SPI bus.
    #[tracing::instrument(skip(data, spi))]
    async fn dispatch_command(
        data: Data,
        mut spi: Spi,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        // should avoid some malformed writes
        if data.raw.len() != Self::DATA_LEN {
            error!("raw_data not {} bytes, refusing to write", Self::DATA_LEN);
            return Result::Err(warp::reject::custom(SpiWriteError));
        }

        // write the data to the spi bus
        let bytes = spi.write_data(&data.raw).await.map_err(|e| {
            error!("{:?}", e);
            warp::reject::custom(SpiWriteError)
        })?;

        // reply with how much was written to the bus
        Ok(warp::reply::with_status(
            format!("Wrote {} bytes of data to SPI", bytes),
            http::StatusCode::OK,
        ))
    }

    /// Runs the HTTP server, waiting for events on the update_raw endpoint.
    #[tracing::instrument(skip(self))]
    pub async fn run(self) {
        let spi = self.spi.clone();
        let spi_filter = warp::any().map(move || spi.clone());

        let update_raw = warp::post()
            .and(warp::path("update_raw"))
            .and(warp::path::end())
            .and(Self::parse_json())
            .and(spi_filter.clone())
            .and_then(Self::dispatch_command);

        info!("Serving on {}", self.addr);
        warp::serve(update_raw).run(self.addr).await;
    }
}

// this is just here because warp::reject::custom sucks.
#[derive(Debug)]
struct SpiWriteError;

impl warp::reject::Reject for SpiWriteError {}
