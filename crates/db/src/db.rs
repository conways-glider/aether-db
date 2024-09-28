use std::{collections::HashMap, sync::Arc};

use tokio::{
    sync::{Notify, RwLock},
    time::Instant,
};
use tracing::debug;

#[derive(Clone)]
pub struct Database<V> {
    store: Arc<Store<V>>,
}

struct Store<V> {
    data: RwLock<HashMap<String, Value<V>>>,
    background_task: Notify,
}

struct Value<V> {
    pub value: V,
    pub expiry: Option<Instant>,
}

impl<V> Database<V>
where
    V: Clone + Send + Sync + 'static,
{
    pub fn new() -> Database<V> {
        let db = Database {
            store: Arc::new(Store::new()),
        };
        tokio::spawn(remove_expired_entries(db.store.clone()));
        db
    }

    pub async fn get(&self, key: &str) -> Option<V> {
        let data = self.store.data.read().await;
        data.get(key).map(|value| value.value.clone())
    }

    pub async fn set(&self, key: String, value: V, expiration: Option<Instant>) {
        // Get next expiration to see if notification is necessary
        let next_expiration = self.store.next_expiration().await;

        // Construct the database value
        let value = Value {
            value,
            expiry: expiration,
        };

        // Insert the new data
        let mut data = self.store.data.write().await;
        data.insert(key, value);

        // Drop data when we're done mutating state.
        drop(data);

        // Check to see if new expiration is the newest
        // If it is, notify the expiration checker
        let should_notify = match (next_expiration, expiration) {
            // No expirations at all => No notification
            (None, None) => false,
            // No current expirations, but there is an expiration in the new value => Notify
            (None, Some(_)) => true,
            // Current expirations exist, but no new expiration => No notification
            (Some(_), None) => false,
            // Both exist => Only notify if next expiration is the first to occur
            (Some(next_expiration), Some(expiration)) => expiration < next_expiration,
        };
        if should_notify {
            self.store.background_task.notify_one();
        }
    }
}

impl<V> Store<V> {
    fn new() -> Store<V> {
        Store {
            data: RwLock::new(HashMap::new()),
            background_task: Notify::new(),
        }
    }

    async fn next_expiration(&self) -> Option<Instant> {
        let state = self.data.read().await;
        let next_expiration = state
            .iter()
            .filter(|(_, value)| value.expiry.is_some())
            .min_by_key(|(_, value)| value.expiry);
        next_expiration.and_then(|(_, value)| value.expiry)
    }

    async fn remove_expired_values(&self) -> Option<Instant> {
        let mut state = self.data.write().await;
        let now = Instant::now();
        debug!("removing expired values");

        // Remove expired values
        // TODO: This could be more efficient by caching expirations
        state.retain(|_, value| value.expiry.map_or(true, |expiry| now <= expiry));

        // Drop state when unneeded and prevent deadlock with next_expiration read
        drop(state);

        // Get the next expiration instant
        // TODO: This could be more efficient by caching expirations
        self.next_expiration().await
    }
}

async fn remove_expired_entries<V>(data: Arc<Store<V>>) {
    loop {
        if let Some(instant) = data.remove_expired_values().await {
            tokio::select! {
                _ = tokio::time::sleep_until(instant) => {}
                _ = data.background_task.notified() => {}
            }
        } else {
            // There are no keys expiring in the future. Wait until the task is
            // notified.
            data.background_task.notified().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test(start_paused = true)]
    async fn test_database_set_expiration() {
        let short_expiration_time_jump = 5;
        let long_expiration_time_jump = 10;
        let time_to_jump = 6;

        let short_future_instant =
            Instant::now().checked_add(Duration::from_secs(short_expiration_time_jump));
        let long_future_instant =
            Instant::now().checked_add(Duration::from_secs(long_expiration_time_jump));

        let database = Database::new();

        // Insert base values, one expiring in the futre
        database
            .set(
                "expire soon".to_string(),
                "value".to_string(),
                long_future_instant,
            )
            .await;
        database
            .set("forever".to_string(), "value".to_string(), None)
            .await;
        assert_eq!(database.store.data.read().await.len(), 2);

        // Insert another value that expires first
        database
            .set(
                "expire sooner".to_string(),
                "value".to_string(),
                short_future_instant,
            )
            .await;
        assert_eq!(database.store.data.read().await.len(), 3);

        // Advance time until expiration occurs
        tokio::time::advance(Duration::from_secs(time_to_jump)).await;
        assert_eq!(database.store.data.read().await.len(), 2);

        // Advance time until expiration occurs
        tokio::time::advance(Duration::from_secs(time_to_jump)).await;

        // Advance again to allow for the expiration to actually occur
        // Zero millis to show this is just allowing for channel weirdness to occur
        tokio::time::advance(Duration::from_millis(0)).await;
        assert_eq!(database.store.data.read().await.len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn test_store_expiration() {
        let expiration_time_jump = 10;
        let wait_time_jump = 11;

        let past_instant = Instant::now().checked_sub(Duration::from_secs(expiration_time_jump));
        let future_instant = Instant::now().checked_add(Duration::from_secs(expiration_time_jump));
        let store = Store::new();

        // Insert test data within block to drop write guard when done
        {
            let mut data = store.data.write().await;
            data.insert(
                "expired".to_string(),
                Value {
                    value: "value".to_string(),
                    expiry: past_instant,
                },
            );
            data.insert(
                "unexpired_1".to_string(),
                Value {
                    value: "value".to_string(),
                    expiry: future_instant,
                },
            );
            data.insert(
                "unexpired_2".to_string(),
                Value {
                    value: "value".to_string(),
                    expiry: future_instant,
                },
            );
            data.insert(
                "forever".to_string(),
                Value {
                    value: "value".to_string(),
                    expiry: None,
                },
            );
        }

        let next_expiration = store.remove_expired_values().await;
        assert_eq!(next_expiration, future_instant);

        let len = store.data.read().await.len();
        assert_eq!(len, 3);

        // Jump ahead to test expirations
        tokio::time::advance(Duration::from_secs(wait_time_jump)).await;

        let next_expiration = store.remove_expired_values().await;
        assert_eq!(next_expiration, None);

        let len = store.data.read().await.len();
        assert_eq!(len, 1);
    }
}
