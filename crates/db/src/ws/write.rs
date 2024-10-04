use std::net::SocketAddr;

use aether_common::message::{Message, StatusMessage};
use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures::{stream::SplitSink, SinkExt};
use tokio::{select, sync::mpsc};
use tracing::{debug, error};

pub async fn write(
    mut sender: SplitSink<WebSocket, WsMessage>,
    client_id: String,
    _: SocketAddr,
    mut message_rx: mpsc::Receiver<WsMessage>,
    mut status_rx: mpsc::Receiver<StatusMessage>,
) {
    // send a ping (unsupported by some browsers) just to kick things off and get a response
    if let Err(err) = sender.send(WsMessage::Ping(vec![1, 2, 3])).await {
        error!(?err, "Could not send ping");
        // no Error here since the only thing we can do is to close the connection.
        // If we can not send messages, there is no way to salvage the statemachine anyway.
        return;
    } else {
        debug!("Sent Ping");
    }

    let client_id_json = serde_json::to_string(&Message::ClientId(client_id.clone()));

    match client_id_json {
        Ok(json) => {
            // Try sending json and handle the error case
            if let Err(err) = sender.send(WsMessage::Text(json)).await {
                error!(?err, "Could not send client_id to client");
                return;
            }
        }
        Err(err) => {
            error!(?err, "Could not generate client_id json");
            return;
        }
    };
    loop {
        select! {
            Some(status) = status_rx.recv() => send_status_message(&mut sender, status).await,
            Some(message) = message_rx.recv() => send_message(&mut sender, message).await,
        }
    }
}

async fn send_status_message(sender: &mut SplitSink<WebSocket, WsMessage>, status: StatusMessage) {
    debug!(?status, "Sending status");
    let status_message = Message::Status(status);
    let text = serde_json::to_string(&status_message);
    match text {
        Ok(text) => send_message(sender, WsMessage::Text(text)).await,
        Err(err) => error!(?err, "Could not serialize error message"),
    }
}

async fn send_message(sender: &mut SplitSink<WebSocket, WsMessage>, message: WsMessage) {
    debug!(?message, "Sending message");
    // TODO: Handle this result beyond logging if possible
    let _ = sender
        .send(message)
        .await
        .inspect_err(|err| error!(?err, "Could not send message"));
}
