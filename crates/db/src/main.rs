use axum::{
    extract::{ConnectInfo, Query, State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use store::DataStore;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::{debug, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod store;
mod ws;

#[derive(Clone)]
struct AppState {
    pub data_store: DataStore,
}

#[derive(Debug, Deserialize)]
struct ClientID {
    client_id: Option<String>,
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
    let app_state = Arc::new(AppState {
        data_store: DataStore::default(),
    });

    // set up routing and middleware
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default()))
        .with_state(app_state);

    // run the server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("could not bind to address");
    debug!(
        "listening on {}",
        listener.local_addr().expect("could not get local address")
    );
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("could not start server");
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(client_id): Query<ClientID>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let client_id = client_id
        .client_id
        .unwrap_or(uuid::Uuid::new_v4().to_string());
    info!(?addr, "connected");
    info!(?client_id, "client id");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| ws::handle_socket(client_id, addr, state, socket))
}
