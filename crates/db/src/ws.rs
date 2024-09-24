use aether_common::{BroadcastMessage, Command, Message};
use axum::extract::ws::{CloseFrame, Message as WSMessage, WebSocket};
use futures::{SinkExt, StreamExt};
use std::{borrow::Cow, collections::HashMap, net::SocketAddr, ops::ControlFlow, sync::Arc};
use tokio::{select, sync::mpsc};
use tracing::{debug, error, info, instrument};

use crate::AppState;

#[instrument(skip(state, socket))]
pub async fn handle_socket(
    client_id: String,
    socket_address: SocketAddr,
    state: Arc<AppState>,
    socket: WebSocket,
) {
    info!(?client_id, ?socket_address, "Upgraded websocket");
    // By splitting socket we can send and receive at the same time. In this example we will send
    // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
    let (mut socket_sender, mut socket_receiver) = socket.split();
    let (command_tx, mut command_rx) = mpsc::channel(100);
    let broadcast_sender = state.data_store.broadcast_channel.clone();
    let mut broadcast_receiver = state.data_store.broadcast_channel.subscribe();

    // The bool represents if you receive your own messages.
    let mut subscriptions: HashMap<String, bool> = HashMap::new();

    // Spawn a task that will push several messages to the client (does not matter what client does)
    let mut send_task = tokio::spawn(async move {
        // send a ping (unsupported by some browsers) just to kick things off and get a response
        if let Err(err) = socket_sender.send(WSMessage::Ping(vec![1, 2, 3])).await {
            error!(?err, "Could not send ping");
            // no Error here since the only thing we can do is to close the connection.
            // If we can not send messages, there is no way to salvage the statemachine anyway.
            return;
        } else {
            debug!("Sent Ping");
        }

        let client_id_json = serde_json::to_string(&Message::ClientId(client_id.clone())).unwrap();

        socket_sender
            .send(WSMessage::Text(client_id_json))
            .await
            .unwrap();

        // Handle messages
        loop {
            select! {
                possible_command = command_rx.recv() => {
                    match possible_command {
                        Some(command) => {debug!(?command, "Processed command");
                        match command {
                            Command::SubscribeBroadcast{ channel, subscribe_to_self } => {subscriptions.insert(channel, subscribe_to_self);},
                            Command::UnsubscribeBroadcast(channel) => {subscriptions.remove(&channel);},
                            Command::SendBroadcast { channel, message } => match broadcast_sender.send(BroadcastMessage{ client_id: client_id.clone(), channel, message }) {
                                Ok(_) => info!("Sent broadcast"),
                                Err(err) => error!(?err, "Could not send broadcast"),
                            },
                        }},
                        None => {
                            info!(?socket_address, client_id, "WS receiver closed");
                            break;
                        },
                    }

                }
                Ok(message) = broadcast_receiver.recv() => {
                    // Write message
                    if should_send_message(&client_id, &message, &subscriptions) {
                        debug!(?message, "Sending message");
                        let client_message = Message::BroadcastMessage(message);
                        let text = serde_json::to_string(&client_message);
                        match text {
                            Ok(text) => {
                                // TODO: Handle this result beyond logging if possible
                                let _ = socket_sender.send(WSMessage::Text(text)).await.inspect_err(|err| error!(?err, "Could not send broadcast message"));
                            },
                            Err(err) => error!(?err, "Could not serialize broadcast message"),
                        }
                    }
                }
            }
        }

        // TODO: Add close broadcast for sanely closing clients
        info!("Sending close");
        if let Err(e) = socket_sender
            .send(WSMessage::Close(Some(CloseFrame {
                code: axum::extract::ws::close_code::NORMAL,
                reason: Cow::from("Goodbye"),
            })))
            .await
        {
            error!(?e, "Could not send Close, probably it is ok?");
        }
    });

    // This second task will receive messages from client and print them on server console
    let mut receive_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = socket_receiver.next().await {
            // print message and break if instructed to do so
            if process_command(msg, socket_address, &command_tx)
                .await
                .is_break()
            {
                return;
            }
        }
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(()) => info!("send_task returned Ok"),
                Err(err) => error!(?err, "Error sending messages")
            }
            receive_task.abort();
        },
        rv_b = (&mut receive_task) => {
            match rv_b {
                Ok(()) => info!("receive_task returned Ok"),
                Err(err) => error!(?err, "Error receiving messages")
            }
            send_task.abort();
        }
    }

    // returning from the handler closes the websocket connection
    info!("Websocket context destroyed");
}

#[instrument(skip(msg))]
async fn process_command(
    msg: WSMessage,
    who: SocketAddr,
    command_tx: &mpsc::Sender<Command>,
) -> ControlFlow<(), ()> {
    match msg {
        WSMessage::Text(t) => {
            // info!(?who, data = t, "sent str");
            let message = serde_json::from_str::<Command>(&t);
            match message {
                Ok(message) => match command_tx.send(message).await {
                    Ok(_) => debug!(?who, "Sent message to receive task"),
                    Err(err) => error!(?err, ?who, "Could not send message to receive task"),
                },
                Err(err) => error!(?err, ?who, "Could not deserialize message"),
            };
            ControlFlow::Continue(())
        }
        WSMessage::Binary(d) => {
            // info!(?who, data = ?d, bytes = d.len(), "sent bytes");
            let message = serde_json::from_slice::<Command>(&d);
            match message {
                Ok(message) => match command_tx.send(message).await {
                    Ok(_) => debug!(?who, "Sent message to receive task"),
                    Err(err) => error!(?err, ?who, "Could not send message to receive task"),
                },
                Err(err) => error!(?err, ?who, "Could not deserialize message"),
            };
            ControlFlow::Continue(())
        }
        WSMessage::Pong(v) => {
            debug!(?who, data = ?v, "Sent pong");
            ControlFlow::Continue(())
        }
        // You should never need to manually handle Message::Ping, as axum's websocket library
        // will do so for you automagically by replying with Pong and copying the v according to
        // spec. But if you need the contents of the pings you can see them here.
        WSMessage::Ping(v) => {
            debug!(?who, data = ?v, "Received ping");
            ControlFlow::Continue(())
        }
        WSMessage::Close(c) => {
            if let Some(cf) = c {
                info!(?who, code = cf.code, reason = ?cf.reason,
                    "Sent close"
                );
            } else {
                info!(?who, "Somehow sent close message without CloseFrame");
            }
            ControlFlow::Break(())
        }
    }
}

fn should_send_message(
    client_id: &String,
    message: &BroadcastMessage,
    subscriptions: &HashMap<String, bool>,
) -> bool {
    message.channel == "global"
        || (subscriptions.contains_key(&message.channel)
            && (message.client_id != *client_id
                || *subscriptions.get(&message.channel).unwrap_or(&false)))
}
