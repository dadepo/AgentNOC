pub mod alerts;
pub mod mcp;

use crate::agents::health;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::sse::{Event, Sse},
};
use futures::stream::Stream;
use std::convert::Infallible;
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;

use crate::alerts::http::server::{AppState, SseEvent};

/// Server-sent events stream for real-time updates
#[utoipa::path(
    get,
    path = "/api/messages/stream",
    responses(
        (status = 200, description = "SSE stream", content_type = "text/event-stream")
    ),
    tag = "streaming"
)]
pub async fn message_stream(
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

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Health status", body = health::HealthStatus),
        (status = 500, description = "Health check failed")
    ),
    tag = "health"
)]
pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<health::HealthStatus>, StatusCode> {
    match health::run(&state.config, &state.db_pool).await {
        Ok(status) => {
            // Broadcast health status to web clients
            let status_json =
                serde_json::to_string(&status).unwrap_or_else(|_| "unknown".to_string());
            let event = SseEvent::HealthCheck {
                status: status_json,
            };
            let _ = state
                .tx
                .send(serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string()));
            Ok(Json(status))
        }
        Err(e) => {
            let event = SseEvent::Error {
                message: format!("Health check error: {e}"),
            };
            let _ = state
                .tx
                .send(serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string()));
            tracing::error!("Health check failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
