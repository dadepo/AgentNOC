use crate::agents::{health, hijack};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::sse::{Event, Sse},
    routing::{get, post},
};
use color_eyre::Result;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tokio::sync::broadcast;
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::services::ServeDir;

use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub tx: broadcast::Sender<String>,
    pub config: AppConfig,
}

pub async fn start(tx: broadcast::Sender<String>, config: AppConfig) -> Result<()> {
    // Serve static files from web-ui/dist directory
    let serve_dir = ServeDir::new("web-ui/dist");

    let port = config.server_port;
    let state = AppState { tx, config };

    // build our application with routes
    // API routes must come before static file serving
    let app = Router::new()
        .route("/api/messages/stream", get(message_stream))
        .route("/api/health", get(health_check))
        .route("/api/alerts", post(create_alert))
        // Serve static files as fallback (must be last)
        .fallback_service(serve_dir)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("Server starting on http://0.0.0.0:{}", port);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn message_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(msg) => Some(Ok(Event::default().data(msg))),
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive-text"),
    )
}

async fn health_check(State(state): State<AppState>) -> Result<Json<String>, StatusCode> {
    match health::run("8.8.8.8".to_string(), &state.config).await {
        Ok(result) => {
            // Broadcast result to web clients
            let _ = state.tx.send(format!("Agent result: {}", result));
            Ok(Json(result))
        }
        Err(e) => {
            let _ = state.tx.send(format!("Agent error: {}", e));
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BGPAlerterAlert {
    message: String,
    description: String,
    details: Details,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Details {
    prefix: String,
    #[serde(default)]
    newprefix: Option<String>,
    #[serde(default)]
    neworigin: Option<String>,
    summary: String,
    earliest: String,
    latest: String,
    kind: String,
    asn: String,
    paths: String,
    peers: String,
}

async fn create_alert(
    State(state): State<AppState>,
    Json(payload): Json<BGPAlerterAlert>,
) -> Result<Json<String>, StatusCode> {
    tracing::debug!("Received alert: {:#?}", payload);

    match hijack::HijackAgent::run(payload, &state.config).await {
        Ok(result) => {
            // Broadcast result to web clients
            let _ = state.tx.send(format!("Hijack Agent result: {}", result));
            Ok(Json(result))
        }
        Err(e) => {
            let _ = state.tx.send(format!("Hijack Agent error: {}", e));
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
