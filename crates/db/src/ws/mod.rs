use aether_common::{
    command::Command,
    db::{BroadcastMessage, Value},
    message::{Message, StatusMessage},
};
use axum::{
    extract::{
        ws::{CloseFrame, Message as WSMessage, WebSocket},
        ConnectInfo, Query, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use std::{borrow::Cow, collections::HashMap, net::SocketAddr, ops::ControlFlow, sync::Arc};
use tokio::{select, sync::mpsc};
use tracing::{debug, error, info, instrument};

use crate::{db::SubscriptionOptions, AppState, ClientID};

mod compute;
mod read;
mod write;

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
    let (message_tx, mut message_rx) = mpsc::channel(crate::CHANNEL_SIZE);
    let (command_tx, mut command_rx) = mpsc::channel(crate::CHANNEL_SIZE);
    let (status_tx, mut status_rx) = mpsc::channel(crate::CHANNEL_SIZE);
    let broadcast_sender = state.data_store.broadcast_channel.clone();
    let mut broadcast_receiver = state.data_store.broadcast_channel.subscribe();

    // Load subscriptions from the database
    let mut subscriptions: HashMap<String, SubscriptionOptions> =
        state.data_store.get_subscriptions(&client_id).await;

    // Spawn a task that will push several messages to the client (does not matter what client does)
    // let mut send_task_temp = tokio::spawn(async move {
    //     // send a ping (unsupported by some browsers) just to kick things off and get a response
    //     if let Err(err) = socket_sender.send(WSMessage::Ping(vec![1, 2, 3])).await {
    //         error!(?err, "Could not send ping");
    //         // no Error here since the only thing we can do is to close the connection.
    //         // If we can not send messages, there is no way to salvage the statemachine anyway.
    //         return;
    //     } else {
    //         debug!("Sent Ping");
    //     }

    //     let client_id_json = serde_json::to_string(&Message::ClientId(client_id.clone()));

    //     match client_id_json {
    //         Ok(json) => {
    //             // Try sending json and handle the error case
    //             if let Err(err) = socket_sender.send(WSMessage::Text(json)).await {
    //                 error!(?err, "Could not send client_id to client");
    //                 return;
    //             }
    //         }
    //         Err(err) => {
    //             error!(?err, "Could not generate client_id json");
    //             return;
    //         }
    //     };

    //     // Handle messages
    //     loop {
    //         select! {
    //             possible_command = command_rx.recv() => {
    //                 match possible_command {
    //                     Some(command) => {
    //                     match command {
    //                         Command::SubscribeBroadcast{ channel, subscribe_to_self } => {
    //                             let subscription = SubscriptionOptions { subscribe_to_self };
    //                             subscriptions.insert(channel.clone(), subscription.clone());
    //                             state.data_store.add_subscription(client_id.clone(), channel, subscription).await;
    //                         },
    //                         Command::UnsubscribeBroadcast(channel) => {
    //                             subscriptions.remove(&channel);
    //                             state.data_store.remove_subscription(client_id.clone(), &channel).await;

    //                         },
    //                         Command::SendBroadcast { channel, message } => match broadcast_sender.send(BroadcastMessage{ client_id: client_id.clone(), channel, message }) {
    //                             Ok(_) => info!("Sent broadcast"),
    //                             Err(err) => error!(?err, "Could not send broadcast"),
    //                         },
    //                         Command::Set { key, value } => {
    //                             let db_value = Value::from(value.clone());
    //                             state.data_store.db.set(key, db_value).await;

    //                             // Return Ok
    //                             let text = serde_json::to_string(&Message::Status(StatusMessage::Ok));
    //                             // TODO: Handle this result beyond logging if possible
    //                             match text {
    //                                 Ok(text) => {
    //                                     // TODO: Handle this result beyond logging if possible
    //                                     let _ = socket_sender.send(WSMessage::Text(text)).await.inspect_err(|err| error!(?err, "Could not send set string message"));
    //                                 },
    //                                 Err(err) => error!(?err, "Could not serialize set string message"),
    //                             }

    //                         },
    //                         Command::Get { key } => {
    //                             let value = state.data_store.db.get(&key).await;
    //                             let text = serde_json::to_string(&Message::Get(value));
    //                             match text {
    //                                 Ok(text) => {
    //                                     // TODO: Handle this result beyond logging if possible
    //                                     let _ = socket_sender.send(WSMessage::Text(text)).await.inspect_err(|err| error!(?err, "Could not send get string message"));
    //                                 },
    //                                 Err(err) => error!(?err, "Could not serialize broadcast message"),
    //                             }

    //                         },
    //                     }},
    //                     None => {
    //                         info!(?socket_address, client_id, "WS receiver closed");
    //                         break;
    //                     },
    //                 }

    //             }
    //             Ok(message) = broadcast_receiver.recv() => {
    //                 // Write message
    //                 if should_send_message(&client_id, &message, &subscriptions) {
    //                     debug!(?message, "Sending message");
    //                     let client_message = Message::BroadcastMessage(message);
    //                     let text = serde_json::to_string(&client_message);
    //                     match text {
    //                         Ok(text) => {
    //                             // TODO: Handle this result beyond logging if possible
    //                             let _ = socket_sender.send(WSMessage::Text(text)).await.inspect_err(|err| error!(?err, "Could not send broadcast message"));
    //                         },
    //                         Err(err) => error!(?err, "Could not serialize broadcast message"),
    //                     }
    //                 }
    //             }
    //             Some(error) = status_rx.recv() => {
    //                 debug!(?error, "Sending error");
    //                 let client_error = Message::Status(error);
    //                 let text = serde_json::to_string(&client_error);
    //                 match text {
    //                     Ok(text) => {
    //                         // TODO: Handle this result beyond logging if possible
    //                         let _ = socket_sender.send(WSMessage::Text(text)).await.inspect_err(|err| error!(?err, "Could not send error message"));
    //                     },
    //                     Err(err) => error!(?err, "Could not serialize error message"),
    //                 }
    //             }
    //         }
    //     }

    //     // TODO: Add close send for sanely closing clients on early returns above
    //     info!("Sending close");
    //     if let Err(e) = socket_sender
    //         .send(WSMessage::Close(Some(CloseFrame {
    //             code: axum::extract::ws::close_code::NORMAL,
    //             reason: Cow::from("Goodbye"),
    //         })))
    //         .await
    //     {
    //         error!(?e, "Could not send Close, most likely okay");
    //     }
    // });

    let mut send_task = tokio::spawn(async move {
        write::write(
            socket_sender,
            client_id,
            socket_address,
            message_rx,
            status_rx,
        )
        .await
    });

    // This second task will receive messages from client and print them on server console
    let mut receive_task = tokio::spawn(async move {
        read::read(socket_receiver, socket_address, &command_tx, &status_tx).await
    });

    // If any one of the tasks exit, abort the other.
    select! {
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

fn should_send_message(
    client_id: &String,
    message: &BroadcastMessage,
    subscriptions: &HashMap<String, SubscriptionOptions>,
) -> bool {
    // This is some nasty logic
    // If the channel is `global`, just send the message
    // Otherwise, check if you're subscribed
    // If you're subscribed and it's not from you, send it
    // If you're subscribed and it is from you, check to see if you subscribed to self messages
    // If you're not subscribed and the channel is not `global`, ignore it
    message.channel == "global"
        || (subscriptions.contains_key(&message.channel)
            && (message.client_id != *client_id
                || *subscriptions
                    .get(&message.channel)
                    .map(|sub| &sub.subscribe_to_self)
                    .unwrap_or(&false)))
}
