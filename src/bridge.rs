use rocket::{
    catch, catchers,
    fairing::AdHoc,
    post, routes,
    serde::{
        json::{json, Json, Value},
        Deserialize,
    },
    State,
};
use tracing::error;

use crate::spi::Spi;

/// The JSON data we expect to receive via POST on the `update_raw` endpoint.
#[derive(Debug, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Data {
    #[serde(alias = "raw_data")]
    raw: Vec<u8>,
}

#[post("/update_raw", format = "json", data = "<data>")]
async fn write_data(data: Json<Data>, dev: &'_ State<Spi>) -> Value {
    if data.raw.len() != Spi::DATA_LEN as usize {
        error!("raw_data not {} bytes, refusing to write", Spi::DATA_LEN);
        return json!({
            "status": "error",
            "reason": format!("raw_data is not {} bytes long", Spi::DATA_LEN)
        });
    }

    let bytes_written = match dev.write_data(&data.raw).await {
        Ok(b) => b,
        Err(e) => {
            error!("{:?}", e);
            return json!({
                "status": "error",
                "reason": format!("{:?}", e)
            });
        }
    };

    json!({
        "status": "ok",
        "bytes_written": bytes_written
    })
}

#[catch(404)]
fn not_found() -> Value {
    json!({
        "status": "error",
        "reason": "Resource not found."
    })
}

pub fn stage(spi: Spi) -> AdHoc {
    AdHoc::on_ignite("HTTP-SPI-Bridge", |rocket| async {
        rocket
            .mount("/", routes![write_data])
            .register("/", catchers![not_found])
            .manage(spi)
    })
}
