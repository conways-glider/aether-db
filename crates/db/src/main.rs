use axum::{routing::get, Router};
use db::Database;
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::debug;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod ws;

// TODO: Make this configurable
// Also, maybe split it out into a few different values
const CHANNEL_SIZE: usize = 1000;

#[derive(Clone)]
struct AppState {
    pub data_store: Database,
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
        data_store: Database::default(),
    });

    // set up routing and middleware
    let app = Router::new()
        .route("/ws", get(ws::ws_handler))
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default()))
        .with_state(app_state);

    // run the server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("could not bind to address");
    debug!(
        "Listening on {}",
        listener.local_addr().expect("could not get local address")
    );
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("could not start server");
}
