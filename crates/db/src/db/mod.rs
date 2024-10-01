use aether_common::BroadcastMessage;
use store::Database;
use tokio::sync::broadcast;

mod store;

#[derive(Clone)]
pub struct DataStore {
    // Data
    pub broadcast_channel: broadcast::Sender<BroadcastMessage>,
    pub string_db: Database<String>,
    pub json_db: Database<serde_json::Value>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("could not send on broadcast channel")]
    BroadcastSendMessage(#[from] broadcast::error::SendError<BroadcastMessage>),
}

impl Default for DataStore {
    fn default() -> Self {
        Self {
            broadcast_channel: broadcast::Sender::new(crate::CHANNEL_SIZE),
            string_db: Database::new(),
            json_db: Database::new(),
        }
    }
}
