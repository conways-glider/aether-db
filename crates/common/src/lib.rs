use serde::{Deserialize, Serialize};

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
    SendBroadcast { channel: String, message: String },

    /// This sets a value in the string database
    ///
    /// `expiration` is the expiration time in seconds.
    SetString {
        key: String,
        value: String,
        expiration: Option<u32>,
    },

    /// This retrieves a value from the string database
    GetString { key: String },

    /// This sets a value in the json database
    ///
    /// `expiration` is the expiration time in seconds.
    SetJson {
        key: String,
        value: serde_json::Value,
        expiration: Option<u32>,
    },

    /// This retrieves a value from the json database
    GetJson { key: String },
}

/// Messages sent from the Server to Clients
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Message {
    /// This is sent at the opening of a websocket with your client id
    ClientId(String),

    /// This contains broadcast messages sent to your subscriptions or to the `general` channel
    BroadcastMessage(BroadcastMessage),

    /// This contains the result of a GetString command
    GetString(Option<String>),

    /// This contains the result of a GetJson command
    GetJson(Option<serde_json::Value>),

    /// This contains an status state
    Status(StatusMessage),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BroadcastMessage {
    pub client_id: String,
    pub channel: String,
    pub message: String,
}

/// This contains an status state
///
/// `operation` may not be set if it is a serialization error or the operation is unknown for some reason.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusMessage {
    Ok,
    Error {
        message: String,
        operation: Option<Command>,
    },
}
