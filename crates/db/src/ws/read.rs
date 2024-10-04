use std::{net::SocketAddr, ops::ControlFlow};

use aether_common::{command::Command, message::StatusMessage};
use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures::{stream::SplitStream, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument};

pub async fn read(
    mut receiver: SplitStream<WebSocket>,
    socket_address: SocketAddr,
    command_tx: &mpsc::Sender<Command>,
    status_tx: &mpsc::Sender<StatusMessage>,
) {
    while let Some(Ok(msg)) = receiver.next().await {
        // print message and break if instructed to do so
        if process_message(msg, socket_address, command_tx, status_tx)
            .await
            .is_break()
        {
            return;
        }
    }
}

#[instrument(skip(msg, command_tx, status_tx))]
async fn process_message(
    msg: WsMessage,
    socket_address: SocketAddr,
    command_tx: &mpsc::Sender<Command>,
    status_tx: &mpsc::Sender<StatusMessage>,
) -> ControlFlow<(), ()> {
    match msg {
        WsMessage::Text(t) => {
            let message = serde_json::from_str::<Command>(&t);
            match message {
                Ok(message) => match command_tx.send(message).await {
                    Ok(_) => debug!(?socket_address, "Sent message to receive task"),
                    Err(err) => error!(
                        ?err,
                        ?socket_address,
                        "Could not send message to receive task"
                    ),
                },
                // Err(err) => error!(?err, ?socket_address, "Could not deserialize message"),
                Err(err) => {
                    let error_message = "Could not deserialize string message";
                    error!(?err, error_message);
                    let _ = status_tx
                        .send(StatusMessage::Error {
                            message: error_message.to_string(),
                            operation: None,
                        })
                        .await
                        .inspect_err(|err| error!(?err, "Could not send error message"));
                }
            };
            ControlFlow::Continue(())
        }
        WsMessage::Binary(d) => {
            let message = serde_json::from_slice::<Command>(&d);
            match message {
                Ok(message) => match command_tx.send(message).await {
                    Ok(_) => debug!(?socket_address, "Sent message to receive task"),
                    Err(err) => error!(
                        ?err,
                        ?socket_address,
                        "Could not send message to receive task"
                    ),
                },
                Err(err) => {
                    let error_message = "Could not deserialize binary message";
                    error!(?err, error_message);
                    let _ = status_tx
                        .send(StatusMessage::Error {
                            message: error_message.to_string(),
                            operation: None,
                        })
                        .await
                        .inspect_err(|err| error!(?err, "Could not send error message"));
                }
            };
            ControlFlow::Continue(())
        }
        WsMessage::Pong(v) => {
            debug!(?socket_address, data = ?v, "Sent pong");
            ControlFlow::Continue(())
        }
        // You should never need to manually handle Message::Ping, as axum's websocket library
        // will do so for you automagically by replying with Pong and copying the v according to
        // spec. But if you need the contents of the pings you can see them here.
        WsMessage::Ping(v) => {
            debug!(?socket_address, data = ?v, "Received ping");
            ControlFlow::Continue(())
        }
        WsMessage::Close(c) => {
            if let Some(cf) = c {
                info!(?socket_address, code = cf.code, reason = ?cf.reason,
                    "Sent close"
                );
            } else {
                info!(
                    ?socket_address,
                    "Somehow sent close message without CloseFrame"
                );
            }
            ControlFlow::Break(())
        }
    }
}
