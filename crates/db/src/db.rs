use std::{collections::HashMap, sync::Arc};

use tokio::{
    sync::{Notify, RwLock},
    time::Instant,
};

#[derive(Clone)]
pub struct Database {
    store: Arc<Store>,
}

struct Store {
    data: RwLock<HashMap<String, Value<String>>>,
    background_task: Notify,
}

struct Value<V> {
    pub value: V,
    pub expiry: Option<Instant>,
}

impl Database {
    pub fn new() -> Database {
        let db = Database {
            store: Arc::new(Store::new()),
        };
        tokio::spawn(remove_expired_entries(db.store.clone()));
        db
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let data = self.store.data.read().await;
        data.get(key).map(|value| value.value.clone())
    }

    pub async fn set(&self, key: String, value: String, expiration: Option<Instant>) {
        let mut data = self.store.data.write().await;
        let value = Value {
            value,
            expiry: expiration,
        };
        data.insert(key, value);
    }
}

impl Store {
    fn new() -> Store {
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

async fn remove_expired_entries(data: Arc<Store>) {
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
    async fn test_expiration() {
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
