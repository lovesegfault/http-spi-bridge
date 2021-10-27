use axum::{extract::Extension, response::IntoResponse, Json};
use serde_json::json;
use tracing::error;

use crate::spi::Spi;

/// The JSON data we expect to receive via POST on the `update_raw` endpoint.
#[derive(Debug, serde::Deserialize, Clone)]
pub struct Data {
    #[serde(alias = "raw_data")]
    raw: Vec<u8>,
}

pub(crate) async fn write_data(
    Json(data): Json<Data>,
    Extension(dev): Extension<Spi>,
) -> impl IntoResponse {
    if data.raw.len() != Spi::DATA_LEN as usize {
        error!("raw_data not {} bytes, refusing to write", Spi::DATA_LEN);
        return Json(json!({
            "status": "error",
            "reason": format!("raw_data is not {} bytes long", Spi::DATA_LEN)
        }));
    }

    let bytes_written = match dev.write_data(&data.raw).await {
        Ok(b) => b,
        Err(e) => {
            error!("{:?}", e);
            return Json(json!({
                "status": "error",
                "reason": format!("{:?}", e)
            }));
        }
    };

    Json(json!({
        "status": "ok",
        "bytes_written": bytes_written
    }))
}
