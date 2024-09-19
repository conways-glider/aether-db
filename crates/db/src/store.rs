use std::{collections::BTreeMap, sync::Arc};

use aether_common::Message;
use tokio::sync::{broadcast, watch, RwLock};


#[derive(Clone, Default)]
pub struct DataStore {
    pub watch_channel: Arc<RwLock<BTreeMap<String, watch::Sender<Message>>>>,
    pub broadcast_channel: Arc<RwLock<BTreeMap<String, broadcast::Sender<Message>>>>,
    pub dict: Arc<RwLock<BTreeMap<String, String>>>,
}
