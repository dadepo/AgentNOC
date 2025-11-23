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

use crate::config::{AppConfig, PrefixesConfig};

#[derive(Clone)]
pub struct AppState {
    pub tx: broadcast::Sender<String>,
    pub config: AppConfig,
    pub prefixes_config: PrefixesConfig,
}

pub async fn start(tx: broadcast::Sender<String>, config: AppConfig) -> Result<()> {
    // Load prefixes configuration
    let prefixes_config = PrefixesConfig::load("prefixes.yml")
        .map_err(|e| color_eyre::eyre::eyre!("Failed to load prefixes.yml: {}", e))?;

    // Serve static files from web-ui/dist directory
    let serve_dir = ServeDir::new("web-ui/dist");

    let port = config.server_port;
    let state = AppState {
        tx,
        config,
        prefixes_config,
    };

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

async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<health::HealthStatus>, StatusCode> {
    match health::run(&state.config).await {
        Ok(status) => {
            // Broadcast health status to web clients
            let status_json =
                serde_json::to_string(&status).unwrap_or_else(|_| "unknown".to_string());
            let _ = state.tx.send(format!("Health check: {}", status_json));
            Ok(Json(status))
        }
        Err(e) => {
            let _ = state.tx.send(format!("Health check error: {}", e));
            tracing::error!("Health check failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BGPAlerterAlert {
    pub message: String,
    pub description: String,
    pub details: Details,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Details {
    pub prefix: String,
    #[serde(default)]
    pub newprefix: Option<String>,
    #[serde(default)]
    pub neworigin: Option<String>,
    pub summary: String,
    pub earliest: String,
    pub latest: String,
    pub kind: String,
    pub asn: String,
    pub paths: String,
    pub peers: String,
}

async fn create_alert(
    State(state): State<AppState>,
    Json(payload): Json<BGPAlerterAlert>,
) -> Result<Json<String>, StatusCode> {
    tracing::debug!("Received alert: {:#?}", payload);

    // Check if alert is relevant to our monitored resources
    if !state.prefixes_config.is_alert_relevant(&payload) {
        tracing::debug!(
            "Alert for prefix {} (ASN: {}) is not relevant to monitored resources, skipping",
            payload.details.prefix,
            payload.details.asn
        );
        let _ = state.tx.send(format!(
            "Alert ignored: prefix {} not in monitored resources",
            payload.details.prefix
        ));
        return Ok(Json(
            "Alert ignored: not relevant to monitored resources".to_string(),
        ));
    }

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
