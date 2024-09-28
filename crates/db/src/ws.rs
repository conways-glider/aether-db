use aether_common::{BroadcastMessage, Command, Message};
use axum::{
    extract::{
        ws::{CloseFrame, Message as WSMessage, WebSocket},
        ConnectInfo, Query, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use std::{
    borrow::Cow, collections::HashMap, net::SocketAddr, ops::ControlFlow, sync::Arc, time::Duration,
};
use tokio::{select, sync::mpsc, time::Instant};
use tracing::{debug, error, info, instrument};

use crate::{AppState, ClientID};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(client_id): Query<ClientID>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let client_id = client_id
        .client_id
        .unwrap_or(uuid::Uuid::new_v4().to_string());
    info!(?addr, ?client_id, "Connected on websocket");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(client_id, addr, state, socket))
}

#[instrument(skip(state, socket))]
async fn handle_socket(
    client_id: String,
    socket_address: SocketAddr,
    state: Arc<AppState>,
    socket: WebSocket,
) {
    info!(?client_id, ?socket_address, "Upgraded websocket");
    // By splitting socket we can send and receive at the same time. In this example we will send
    // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
    let (mut socket_sender, mut socket_receiver) = socket.split();
    let (command_tx, mut command_rx) = mpsc::channel(crate::CHANNEL_SIZE);
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

        if let Err(err) = socket_sender
            .send(WSMessage::Text(client_id_json))
            .await {
                error!(?err, "Could not send client_id");
                return;
            }

        // Handle messages
        loop {
            select! {
                possible_command = command_rx.recv() => {
                    match possible_command {
                        Some(command) => {
                        match command {
                            Command::SubscribeBroadcast{ channel, subscribe_to_self } => {subscriptions.insert(channel, subscribe_to_self);},
                            Command::UnsubscribeBroadcast(channel) => {subscriptions.remove(&channel);},
                            Command::SendBroadcast { channel, message } => match broadcast_sender.send(BroadcastMessage{ client_id: client_id.clone(), channel, message }) {
                                Ok(_) => info!("Sent broadcast"),
                                Err(err) => error!(?err, "Could not send broadcast"),
                            },
                            Command::SetString { key, value, expiration } => {
                                let expiration = expiration.and_then(|expiration_seconds| {
                                    let expiration_duration = Duration::from_secs(expiration_seconds as u64);
                                    Instant::now().checked_add(expiration_duration)
                                });
                                state.data_store.string_db.set(key, value, expiration).await;
                            },
                            Command::GetString { key } => {
                                let value = state.data_store.string_db.get(&key).await;
                                let text = serde_json::to_string(&Message::GetString(value));
                                match text {
                                    Ok(text) => {
                                        // TODO: Handle this result beyond logging if possible
                                        let _ = socket_sender.send(WSMessage::Text(text)).await.inspect_err(|err| error!(?err, "Could not send get string message"));
                                    },
                                    Err(err) => error!(?err, "Could not serialize broadcast message"),
                                }

                            },
                            Command::SetJson { key, value, expiration } => {
                                let expiration = expiration.and_then(|expiration_seconds| {
                                    let expiration_duration = Duration::from_secs(expiration_seconds as u64);
                                    Instant::now().checked_add(expiration_duration)
                                });
                                state.data_store.json_db.set(key, value, expiration).await;
                            },
                            Command::GetJson { key } => {
                                let value = state.data_store.json_db.get(&key).await;
                                let text = serde_json::to_string(&Message::GetJson(value));
                                match text {
                                    Ok(text) => {
                                        // TODO: Handle this result beyond logging if possible
                                        let _ = socket_sender.send(WSMessage::Text(text)).await.inspect_err(|err| error!(?err, "Could not send get string message"));
                                    },
                                    Err(err) => error!(?err, "Could not serialize broadcast message"),
                                }

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
            error!(?e, "Could not send Close, most likely okay");
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

#[instrument(skip(msg, command_tx))]
async fn process_command(
    msg: WSMessage,
    socket_address: SocketAddr,
    command_tx: &mpsc::Sender<Command>,
) -> ControlFlow<(), ()> {
    match msg {
        WSMessage::Text(t) => {
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
                Err(err) => error!(?err, ?socket_address, "Could not deserialize message"),
            };
            ControlFlow::Continue(())
        }
        WSMessage::Binary(d) => {
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
                Err(err) => error!(?err, ?socket_address, "Could not deserialize message"),
            };
            ControlFlow::Continue(())
        }
        WSMessage::Pong(v) => {
            debug!(?socket_address, data = ?v, "Sent pong");
            ControlFlow::Continue(())
        }
        // You should never need to manually handle Message::Ping, as axum's websocket library
        // will do so for you automagically by replying with Pong and copying the v according to
        // spec. But if you need the contents of the pings you can see them here.
        WSMessage::Ping(v) => {
            debug!(?socket_address, data = ?v, "Received ping");
            ControlFlow::Continue(())
        }
        WSMessage::Close(c) => {
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
