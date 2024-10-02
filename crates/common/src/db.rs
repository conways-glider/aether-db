use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

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

impl TryFrom<crate::command::Value> for Value {
    type Error = Error;

    fn try_from(value: crate::command::Value) -> Result<Self, Self::Error> {
        if value.expire.is_some() && value.until_expiry.is_some() {
            Err(Error::DoubleExpirationDefined)
        } else {
            let expiry = value.expire.or(value
                .until_expiry
                .and_then(|duration| OffsetDateTime::now_utc().checked_add(duration)));
            Ok(Value {
                data: value.data,
                expiry,
            })
        }
    }
}
