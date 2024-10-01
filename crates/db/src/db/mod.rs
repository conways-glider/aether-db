use std::{collections::HashMap, sync::Arc};

use aether_common::BroadcastMessage;
use table::Table;
use tokio::sync::{broadcast, RwLock};

mod table;

#[derive(Clone)]
pub struct Database {
    // Data
    pub broadcast_channel: broadcast::Sender<BroadcastMessage>,
    // TODO: Add get current subscriptions command
    // TODO: Add clear all subscriptions command
    pub subscriptions: Arc<RwLock<HashMap<String, Vec<String>>>>,
    pub string_db: Table<String>,
    pub json_db: Table<serde_json::Value>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("could not send on broadcast channel")]
    BroadcastSendMessage(#[from] broadcast::error::SendError<BroadcastMessage>),
}

impl Database {
    pub async fn add_subscription(&self, client_id: String, subscription: String) {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions
            .entry(client_id)
            .or_default()
            .push(subscription);
    }

    pub async fn get_subscriptions(&self, client_id: String) -> Vec<String> {
        let subscriptions = self.subscriptions.read().await;
        subscriptions.get(&client_id).cloned().unwrap_or_default()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subscriptions() {
        let client_id = "client".to_string();
        let subscription = "subscription".to_string();
        let database = Database::default();
        database
            .add_subscription(client_id.clone(), subscription.clone())
            .await;
        let subscriptions = database.get_subscriptions(client_id).await;
        assert_eq!(subscriptions, vec![subscription])
    }
}
