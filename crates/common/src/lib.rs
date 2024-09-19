use std::{fmt, path::Display};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Command {
    SubscribeWatch(String),
    UnsubscribeWatch(String),
    SubscribeBroadcast(String),
    UnsubscribeBroadcast(String),
    SendWatch{
        channel: String,
        message: String,
    },
    SendBroadcast{
        channel: String,
        message: String,
    },
}

#[derive(Serialize, Deserialize)]
pub enum Message {
    Command(Command),
    ClientId(String),
    WatchMessage{
        channel: String,
        message: String,
    },
    BroadcastMessage{
        channel: String,
        message: String,
    },
}
