use serde::{Deserialize, Serialize};

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
}
