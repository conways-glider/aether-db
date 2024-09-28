use std::{sync::Arc, time::Duration};

use aether_common::BroadcastMessage;
use dashmap::DashMap;
use tokio::{sync::broadcast, time::{sleep, Instant}};

use crate::db::DatabaseValue;

#[derive(Clone)]
pub struct DataStore {
    // Data
    pub broadcast_channel: broadcast::Sender<BroadcastMessage>,
    pub string_db: Arc<DashMap<String, DatabaseValue<String>>>,
    pub json_db: Arc<DashMap<String, DatabaseValue<serde_json::Value>>>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("could not send on broadcast channel")]
    BroadcastSendMessage(#[from] broadcast::error::SendError<BroadcastMessage>),
}

impl Default for DataStore {
    fn default() -> Self {
        Self {
            broadcast_channel: broadcast::Sender::new(100),
            string_db: Arc::new(DashMap::new()),
            json_db: Arc::new(DashMap::new()),
        }
    }
}

impl DataStore {
    pub async fn handle_expiry(&self) {
        let sleep_duration = Duration::from_secs(1);
        loop {
            todo!();
        }
    }

    pub fn insert_string_db(&self, key: String, value: String,) {
        let value = DatabaseValue{ value, expiry: None };
        self.string_db.insert(key, value);
    }

    pub fn insert_json_db(&self, key: String, value: serde_json::Value) {
        let value = DatabaseValue{ value, expiry: None };
        self.json_db.insert(key, value);
    }

    pub fn get_string_db(&self, key: &str) -> Option<String> {
        self.string_db.get(key).map(|e| e.value.clone())
    }

    pub fn get_json_db(&self, key: &str) -> Option<serde_json::Value> {
        self.json_db.get(key).map(|e| e.value.clone())
    }
}
