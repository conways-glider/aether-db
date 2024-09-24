
use aether_common::BroadcastMessage;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct DataStore {
    // Data
    pub broadcast_channel: broadcast::Sender<BroadcastMessage>,
    // pub dict: Arc<RwLock<BTreeMap<String, String>>>,
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
        }
    }
}
