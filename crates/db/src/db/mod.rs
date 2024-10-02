use std::{cmp::max, collections::HashMap, sync::Arc, time::Duration};

use aether_common::db::{BroadcastMessage, Value};
use dashmap::DashMap;
use time::OffsetDateTime;
use tokio::sync::{broadcast, Notify, RwLock};
use tracing::debug;

pub struct Database {
    // Data
    pub broadcast_channel: broadcast::Sender<BroadcastMessage>,
    // TODO: Add get current subscriptions command
    // TODO: Add clear all subscriptions command
    pub subscriptions: Arc<RwLock<HashMap<String, HashMap<String, SubscriptionOptions>>>>,
    pub db: Arc<DashMap<String, Value>>,
    expiry_notification: Notify,
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

    async fn next_expiration(&self) -> Option<OffsetDateTime> {
        let next_expiration = self
            .db
            .iter()
            .filter(|entry| entry.expiry.is_some())
            .min_by_key(|entry| entry.expiry);
        next_expiration.and_then(|entry| entry.expiry)
    }

    async fn remove_expired_values(&self) -> Option<Duration> {
        let now = OffsetDateTime::now_utc();
        debug!("removing expired values");

        // Remove expired values
        // TODO: This could be more efficient by caching expirations
        self.db
            .retain(|_, value| value.expiry.map_or(true, |expiry| now <= expiry));

        // Get the duration until the next expiration for tokio::time::sleep
        // TODO: This could be more efficient by caching expirations
        self.next_expiration().await.map(|expiration| {
            let expiration_offset: time::Duration = expiration - now;
            // We max here to floor it to zero and prevent a negative duration becoming a large positive duration via `.unsigned_abs()`.
            max(time::Duration::new(0, 0), expiration_offset).unsigned_abs()
        })
    }
}

pub(crate) async fn remove_expired_entries(database: Arc<Database>) {
    loop {
        if let Some(instant) = database.remove_expired_values().await {
            tokio::select! {
                // Hope to switch this call to `sleep_until` as it seems cleaner.
                // This depends on better time handling.
                _ = tokio::time::sleep(instant) => {}
                _ = database.expiry_notification.notified() => {}
            }
        } else {
            // There are no keys expiring in the future. Wait until the task is
            // notified.
            database.expiry_notification.notified().await;
        }
    }
}

impl Default for Database {
    fn default() -> Self {
        Self {
            broadcast_channel: broadcast::Sender::new(crate::CHANNEL_SIZE),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            db: Arc::new(DashMap::new()),
            expiry_notification: Notify::new(),
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
