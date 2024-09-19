use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Command {
    Subscribe(String),
    SendWatch(String),
    SendBroadcast(String),
}

#[derive(Serialize, Deserialize)]
pub enum Message {
    String(String),
}
