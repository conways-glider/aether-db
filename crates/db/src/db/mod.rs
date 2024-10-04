use aether_common::db::BroadcastMessage;
use std::{collections::HashMap, sync::Arc};

use table::Table;
use tokio::sync::{broadcast, RwLock};

mod table;

#[derive(Clone)]
pub struct Database {
    // Data
    pub db: Table,
    pub broadcast_channel: broadcast::Sender<BroadcastMessage>,
    // TODO: Add get current subscriptions command
    // TODO: Add clear all subscriptions command
    subscriptions: Arc<RwLock<HashMap<String, HashMap<String, SubscriptionOptions>>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubscriptionOptions {
    pub subscribe_to_self: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("could not send on broadcast channel")]
    BroadcastSendMessage(#[from] broadcast::error::SendError<BroadcastMessage>),
}

impl Database {
    pub async fn add_subscription(
        &self,
        client_id: String,
        channel: String,
        subscription: SubscriptionOptions,
    ) {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions
            .entry(client_id)
            .or_default()
            .insert(channel, subscription);
    }

    pub async fn remove_subscription(&self, client_id: String, channel: &str) {
        let mut subscriptions = self.subscriptions.write().await;
        let client_subscriptions = subscriptions.entry(client_id).or_default();
        client_subscriptions.remove(channel);
    }

    pub async fn get_subscriptions(&self, client_id: &str) -> HashMap<String, SubscriptionOptions> {
        let subscriptions = self.subscriptions.read().await;
        subscriptions.get(client_id).cloned().unwrap_or_default()
    }

    pub async fn clear_subscriptions(&self, client_id: &str) {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.remove(client_id);
    }
}

impl Default for Database {
    fn default() -> Self {
        Self {
            broadcast_channel: broadcast::Sender::new(crate::CHANNEL_SIZE),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            db: Table::new(),
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
        let subscription_options = SubscriptionOptions {
            subscribe_to_self: false,
        };
        let database = Database::default();
        database
            .add_subscription(
                client_id.clone(),
                subscription.clone(),
                subscription_options.clone(),
            )
            .await;
        let subscriptions = database.get_subscriptions(&client_id).await;
        assert_eq!(
            subscriptions.get(&subscription),
            Some(&subscription_options)
        );

        database.clear_subscriptions(&client_id).await;
        let subscriptions = database.get_subscriptions(&client_id).await;
        assert_eq!(subscriptions.len(), 0);
    }
}
