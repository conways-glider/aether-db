use std::{collections::HashMap, sync::Arc};

use tokio::{
    sync::{Notify, RwLock},
    time::Instant,
};

#[derive(Clone)]
pub struct Database {
    data: Arc<Store>,
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
            data: Arc::new(Store::new()),
        };
        db
    }
}

impl Store {
    fn new() -> Store {
        Store {
            data: RwLock::new(HashMap::new()),
            background_task: Notify::new(),
        }
    }

    async fn remove_expired_values(&self) -> Option<Instant> {
        let mut state = self.data.write().await;
        let now = Instant::now();
        state.retain(|_, value| value.expiry.map_or(true, |expiry| now <= expiry));

        let next_expiration = state
            .iter()
            .filter(|(_, value)| value.expiry.is_some())
            .min_by_key(|(_, value)| value.expiry);
        next_expiration.map(|(_, value)| value.expiry).flatten()
    }
}

async fn remove_expired_entries<V>(data: Arc<Store>) {
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
        let past_instant = Instant::now().checked_sub(Duration::from_secs(10));
        let future_instant = Instant::now().checked_add(Duration::from_secs(10));
        let store = Store::new();
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

        tokio::time::advance(Duration::from_secs(30)).await;

        let next_expiration = store.remove_expired_values().await;
        assert_eq!(next_expiration, None);

        let len = store.data.read().await.len();
        assert_eq!(len, 1);
    }
}
