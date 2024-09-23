use serde::{Deserialize, Serialize};

/// Commands sent from the Client to the Server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Command {
    SubscribeBroadcast(String),
    UnsubscribeBroadcast(String),
    SendWatch { channel: String, message: String },
    SendBroadcast { channel: String, message: String },
}

/// Messages sent from the Server to Clients
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Message {
    ClientId(String),
    BroadcastMessage { channel: String, message: String },
}
