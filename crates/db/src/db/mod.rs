use std::{collections::HashMap, sync::{Arc, RwLock}};

use aether_common::BroadcastMessage;
use table::Table;
use tokio::sync::broadcast;

mod table;

#[derive(Clone)]
pub struct Database {
    // Data
    pub broadcast_channel: broadcast::Sender<BroadcastMessage>,
    pub subscriptions: Arc<RwLock<HashMap<String, Vec<String>>>>,
    pub string_db: Table<String>,
    pub json_db: Table<serde_json::Value>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("could not send on broadcast channel")]
    BroadcastSendMessage(#[from] broadcast::error::SendError<BroadcastMessage>),
}

impl Default for Database {
    fn default() -> Self {
        Self {
            broadcast_channel: broadcast::Sender::new(crate::CHANNEL_SIZE),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            string_db: Table::new(),
            json_db: Table::new(),
        }
    }
}
