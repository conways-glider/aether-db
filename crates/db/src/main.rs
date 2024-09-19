use aether_common::Command;
use axum::{
    extract::{
        ws::{CloseFrame, Message, WebSocket}, ConnectInfo, Query, State, WebSocketUpgrade
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use store::DataStore;
use tokio::sync::mpsc;
use std::{borrow::Cow, net::SocketAddr, ops::ControlFlow, sync::Arc};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::{debug, error, info, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod store;

#[derive(Clone)]
struct AppState {
    pub data_store: DataStore
}

#[derive(Debug, Deserialize)]
struct ClientID {
    client_id: Option<String>,
}

enum Channels {
    Commands(tokio::sync::mpsc::Receiver<Command>)
}

#[tokio::main]
async fn main() {
    // set up tracing subsciber
    // this let's us get our logs
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // set up our app state
    // this contains our runtime data and configs
    let app_state = Arc::new(AppState{ data_store: DataStore::default() });

    // set up routing and middleware
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default()))
        .with_state(app_state);

    // run the server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(client_id): Query<ClientID>,
    State(state): State<Arc<AppState>>
) -> impl IntoResponse {
    let client_id = client_id.client_id.unwrap_or(uuid::Uuid::new_v4().to_string());
    info!(?addr, "connected");
    info!(?client_id, "client id");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(client_id, addr, state, socket))
}

#[instrument(skip(state, socket))]
async fn handle_socket(client_id: String, socket_address: SocketAddr, state: Arc<AppState>, mut socket: WebSocket) {
    info!("upgraded");
    // By splitting socket we can send and receive at the same time. In this example we will send
    // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel(100);

    // Spawn a task that will push several messages to the client (does not matter what client does)
    let mut send_task = tokio::spawn(async move {
        // send a ping (unsupported by some browsers) just to kick things off and get a response
        if sender.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
            info!("pinged");
        } else {
            error!("could not send ping");
            // no Error here since the only thing we can do is to close the connection.
            // If we can not send messages, there is no way to salvage the statemachine anyway.
            return 0;
        }

        let client_id_json = serde_json::to_string(&aether_common::Message::ClientId(client_id)).unwrap();

        sender.send(Message::Text(client_id_json)).await.unwrap();

        let mut subscriptions: Vec<Channels> = vec![Channels::Commands(rx)];

        // Send dummy messages
        let n_msg = 20;
        for i in 0..n_msg {
            // In case of any websocket error, we exit.
            if sender
                .send(Message::Text(format!("Server message {i}")))
                .await
                .is_err()
            {
                return i;
            }

            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }

        info!("Sending close");
        if let Err(e) = sender
            .send(Message::Close(Some(CloseFrame {
                code: axum::extract::ws::close_code::NORMAL,
                reason: Cow::from("Goodbye"),
            })))
            .await
        {
            error!(?e, "Could not send Close, probably it is ok?");
        }
        n_msg
    });

    // This second task will receive messages from client and print them on server console
    let mut receive_task = tokio::spawn(async move {
        let mut count = 0;
        while let Some(Ok(msg)) = receiver.next().await {
            count += 1;
            // print message and break if instructed to do so
            if process_message(msg, socket_address).is_break() {
                break;
            }
        }
        count
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(a) => info!(messages = a, "messages sent"),
                Err(err) => error!(?err, "Error sending messages")
            }
            receive_task.abort();
        },
        rv_b = (&mut receive_task) => {
            match rv_b {
                Ok(b) => info!(messages = b, "Received messages"),
                Err(err) => error!(?err, "Error receiving messages")
            }
            send_task.abort();
        }
    }

    // returning from the handler closes the websocket connection
    info!("Websocket context destroyed");
}

#[instrument(skip(msg))]
fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            info!(?who, data = t, "sent str");
        }
        Message::Binary(d) => {
            info!(?who, data = ?d, bytes = d.len(), "sent bytes");
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                info!(?who, code = cf.code, reason = ?cf.reason,
                    "sent close"
                );
            } else {
                info!(?who, "somehow sent close message without CloseFrame");
            }
            return ControlFlow::Break(());
        }

        Message::Pong(v) => {
            debug!(?who, data = ?v, "sent pong");
        }
        // You should never need to manually handle Message::Ping, as axum's websocket library
        // will do so for you automagically by replying with Pong and copying the v according to
        // spec. But if you need the contents of the pings you can see them here.
        Message::Ping(v) => {
            debug!(?who, data = ?v, "received ping");
        }
    }
    ControlFlow::Continue(())
}
