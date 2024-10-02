// #![doc = include_str!("../README.md")]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use axum::{routing::get, Router};
use db::{remove_expired_entries, Database};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::{debug, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod ws;

// TODO: Make this configurable
// Also, maybe split it out into a few different values
const CHANNEL_SIZE: usize = 1000;

#[derive(Clone)]
struct AppState {
    pub database: Arc<Database>,
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

    let database = Arc::new(Database::default());

    // set up our app state
    // this contains our runtime data and configs
    let app_state = Arc::new(AppState {
        database: database.clone(),
    });

    tokio::spawn(remove_expired_entries(database));

    // set up routing and middleware
    let app = Router::new()
        .route("/ws", get(ws::ws_handler))
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default()))
        .with_state(app_state);

    // run the server
    match tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await {
            Ok(listener) => {
                debug!(
                    "Listening on {:?}",
                    listener.local_addr()
                );
                let _ = axum::serve(
                    listener,
                    app.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await.inspect_err(|err| error!(?err, "could not start server"));
            },
            Err(err) => error!(?err, "could not bind to address"),
        };

}
