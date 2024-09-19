use std::{collections::BTreeMap, sync::Arc};

use aether_common::Message;
use tokio::sync::{broadcast, watch, RwLock};

#[derive(Clone, Default)]
pub struct DataStore {
    // Data
    pub watch_channel: Arc<RwLock<BTreeMap<String, watch::Sender<Message>>>>,
    pub broadcast_channel: Arc<RwLock<BTreeMap<String, broadcast::Sender<Message>>>>,
    pub dict: Arc<RwLock<BTreeMap<String, String>>>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("channel does not exist: `{0}`")]
    ChannelDoesNotExist(String),

    #[error("could not send on watch channel")]
    WatchSendMessage(#[from] watch::error::SendError<Message>),

    #[error("could not send on broadcast channel")]
    BroadcastSendMessage(#[from] broadcast::error::SendError<Message>),
}

impl DataStore {
    pub async fn send_to_watch_channel(
        &self,
        channel_name: String,
        message: Message,
    ) -> Result<(), Error> {
        let watch_channels = self.watch_channel.read().await;
        let channel = watch_channels.get(&channel_name);
        match channel {
            Some(channel) => channel.send(message).map_err(Error::WatchSendMessage),
            None => Err(Error::ChannelDoesNotExist(channel_name)),
        }
    }

    pub async fn send_to_broadcast_channel(
        &self,
        channel_name: String,
        message: Message,
    ) -> Result<usize, Error> {
        let watch_channels = self.broadcast_channel.read().await;
        let channel = watch_channels.get(&channel_name);
        match channel {
            Some(channel) => channel.send(message).map_err(Error::BroadcastSendMessage),
            None => Err(Error::ChannelDoesNotExist(channel_name)),
        }
    }
}
