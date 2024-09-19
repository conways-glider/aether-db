use serde::{Deserialize, Serialize};

/// Commands sent from the Client to the Server
#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    SubscribeWatch(String),
    UnsubscribeWatch(String),
    SubscribeBroadcast(String),
    UnsubscribeBroadcast(String),
    SendWatch { channel: String, message: String },
    SendBroadcast { channel: String, message: String },
}

/// Messages sent from the Server to Clients
#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    ClientId(String),
    WatchMessage { channel: String, message: String },
    BroadcastMessage { channel: String, message: String },
}
