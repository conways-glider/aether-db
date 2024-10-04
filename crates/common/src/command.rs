use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::db::Data;

/// Commands sent from the Client to the Server
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    /// This subscribes your client to the given channel
    ///
    /// If `subscribe_to_self` is true, you will receive messages sent to channels that were sent by yourself.
    SubscribeBroadcast {
        channel: String,

        // Default this to false
        #[serde(default)]
        subscribe_to_self: bool,
    },

    /// This unsubscribes your client from the given channel
    UnsubscribeBroadcast(String),

    /// This sends a broadcast to the given channel
    ///
    /// If the channel is `general`, all clients will receive this message.
    SendBroadcast {
        channel: String,
        message: String,
    },

    Set {
        key: String,
        value: Value,
    },

    /// This retrieves a value
    Get {
        key: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Value {
    pub data: Data,
    pub expiry: Option<u32>,
    #[serde(with = "time::serde::iso8601::option")]
    pub expire_at: Option<OffsetDateTime>
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialization() {
        let expected_value = Value { data: Data::Int(1), expiry: None, expire_at: Some(OffsetDateTime::from_unix_timestamp(1728043200).unwrap()) };
        let input = "{\"data\":{\"int\":1},\"expiry\":null,\"expire_at\":\"2024-10-04T12:00:00Z\"}";
        let value: Value = serde_json::from_str(input).unwrap();
        assert_eq!(value.expire_at, expected_value.expire_at);
    }
}
