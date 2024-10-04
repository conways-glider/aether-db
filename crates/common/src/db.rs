use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::{Duration, OffsetDateTime};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Value {
    pub data: Data,
    pub expiry: Option<OffsetDateTime>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Data {
    String(String),
    Json(serde_json::Value),
    Int(i64),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BroadcastMessage {
    pub client_id: String,
    pub channel: String,
    pub message: String,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("both expire and until_expiry were defined")]
    DoubleExpirationDefined,
}

impl From<crate::command::Value> for Value {
    fn from(value: crate::command::Value) -> Self {
        let expiry = value.expiry.and_then(|seconds| {
            OffsetDateTime::now_utc().checked_add(Duration::new(seconds as i64, 0))
        });
        Self {
            data: value.data,
            expiry,
        }
    }
}
