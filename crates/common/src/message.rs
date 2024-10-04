use serde::{Deserialize, Serialize};

use crate::{
    command::Command,
    db::{BroadcastMessage, Value},
};

/// Messages sent from the Server to Clients
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Message {
    /// This is sent at the opening of a websocket with your client id
    ClientId(String),

    /// This contains broadcast messages sent to your subscriptions or to the `general` channel
    BroadcastMessage(BroadcastMessage),

    /// This contains the result of a GetString command
    Get(Option<Value>),

    /// This contains an status state
    Status(StatusMessage),
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
