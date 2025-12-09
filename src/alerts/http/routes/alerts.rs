use crate::agents::{alert_analyzer, chat};
use crate::database::db;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::alerts::http::server::{AppState, BGPAlerterAlert, SseEvent};

#[derive(Deserialize, ToSchema)]
pub struct ChatRequest {
    pub message: String,
}

#[derive(IntoParams)]
pub struct AlertId {
    /// Alert ID
    #[allow(dead_code)]
    pub id: i64,
}

/// Process a new BGP alert
#[utoipa::path(
    post,
    path = "/api/alerts",
    request_body = BGPAlerterAlert,
    responses(
        (status = 200, description = "Alert processed successfully", body = serde_json::Value),
        (status = 500, description = "Internal server error")
    ),
    tag = "alerts"
)]
pub async fn process_alert(
    State(state): State<AppState>,
    Json(payload): Json<BGPAlerterAlert>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!(
        "Received alert: prefix={}, asn={}, neworigin={:?}",
        payload.details.prefix,
        payload.details.asn,
        payload.details.neworigin
    );

    // Check if alert is relevant to our monitored resources
    if !state.prefixes_config.is_alert_relevant(&payload) {
        tracing::warn!(
            "Alert for prefix {} (ASN: {}) is not relevant to monitored resources, skipping. \
            Check prefixes.yml to ensure this prefix or ASN is monitored.",
            payload.details.prefix,
            payload.details.asn
        );
        let event = SseEvent::Error {
            message: format!(
                "Alert ignored: prefix {} (ASN: {}) not in monitored resources. \
                Add it to prefixes.yml to process alerts for this prefix/ASN.",
                payload.details.prefix, payload.details.asn
            ),
        };
        let _ = state
            .tx
            .send(serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string()));
        return Ok(Json(serde_json::json!({
            "error": format!(
                "Alert ignored: prefix {} (ASN: {}) not in monitored resources. \
                Add it to prefixes.yml to process alerts for this prefix/ASN.",
                payload.details.prefix,
                payload.details.asn
            ),
            "ignored": true
        })));
    }

    tracing::info!(
        "Processing alert for prefix {} (ASN: {})",
        payload.details.prefix,
        payload.details.asn
    );

    match alert_analyzer::AlertAnalyzer::run(payload.clone(), &state.config, &state.db_pool).await {
        Ok(result) => {
            // Save alert and initial response to database
            let alert_data_json = serde_json::to_string(&payload).map_err(|e| {
                tracing::error!("Failed to serialize alert: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            let timestamp = crate::database::models::get_current_timestamp();

            let alert_id = sqlx::query_scalar::<_, i64>(
                r#"
                INSERT INTO alerts (alert_data, initial_response, kind, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?)
                RETURNING id
                "#,
            )
            .bind(&alert_data_json)
            .bind(&result)
            .bind(crate::database::models::AlertKind::BgpAlerter.as_str())
            .bind(&timestamp)
            .bind(&timestamp)
            .fetch_one(&*state.db_pool)
            .await
            .map_err(|e| {
                tracing::error!("Database error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            // Broadcast SSE notification
            let event = SseEvent::NewAlert { alert_id };
            let event_json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
            let _ = state.tx.send(event_json);

            Ok(Json(serde_json::json!({
                "alert_id": alert_id,
                "response": result
            })))
        }
        Err(e) => {
            let event = SseEvent::Error {
                message: format!("Alert Analysis Agent error: {e}"),
            };
            let _ = state
                .tx
                .send(serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string()));
            tracing::error!("Alert processing failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// List all alerts
#[utoipa::path(
    get,
    path = "/api/alerts",
    responses(
        (status = 200, description = "List of all alerts", body = Vec<serde_json::Value>),
        (status = 500, description = "Internal server error")
    ),
    tag = "alerts"
)]
pub async fn list_alerts(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let alerts = db::list_alerts(&state.db_pool).await.map_err(|e| {
        tracing::error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(alerts))
}

/// Get a specific alert by ID
#[utoipa::path(
    get,
    path = "/api/alerts/{id}",
    params(AlertId),
    responses(
        (status = 200, description = "Alert found", body = serde_json::Value),
        (status = 404, description = "Alert not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "alerts"
)]
pub async fn get_alert(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let alert = db::get_alert_by_id(&state.db_pool, id).await.map_err(|e| {
        tracing::error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match alert {
        Some(alert) => Ok(Json(alert)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Chat with an alert using AI
#[utoipa::path(
    post,
    path = "/api/alerts/{id}/chat",
    params(AlertId),
    request_body = ChatRequest,
    responses(
        (status = 200, description = "Chat response", body = serde_json::Value),
        (status = 404, description = "Alert not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "alerts"
)]
pub async fn chat_with_alert(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<ChatRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Get alert data and initial response
    let (alert_data, initial_response) = db::get_alert_for_chat(&state.db_pool, id)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let alert: BGPAlerterAlert = serde_json::from_str(&alert_data).map_err(|e| {
        tracing::error!("Failed to parse alert data: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get chat history
    let chat_history = db::get_chat_history(&state.db_pool, id)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Save user message
    db::insert_chat_message(&state.db_pool, id, "user", &payload.message)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Run chat agent
    let assistant_response = match chat::Chat::run(
        alert,
        &initial_response,
        &chat_history,
        &payload.message,
        &state.config,
        &state.db_pool,
    )
    .await
    {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Chat agent error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Save assistant response
    let message_id = db::insert_chat_message(&state.db_pool, id, "assistant", &assistant_response)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Broadcast SSE notification
    let event = SseEvent::ChatMessage {
        alert_id: id,
        message_id,
    };
    let event_json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
    let _ = state.tx.send(event_json);

    Ok(Json(serde_json::json!({
        "response": assistant_response,
        "message_id": message_id
    })))
}

/// Delete an alert
#[utoipa::path(
    delete,
    path = "/api/alerts/{id}",
    params(AlertId),
    responses(
        (status = 204, description = "Alert deleted successfully"),
        (status = 404, description = "Alert not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "alerts"
)]
pub async fn delete_alert(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    let deleted = db::delete_alert(&state.db_pool, id).await.map_err(|e| {
        tracing::error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if !deleted {
        return Err(StatusCode::NOT_FOUND);
    }

    // Broadcast SSE notification
    let event = SseEvent::AlertDeleted { alert_id: id };
    let event_json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
    let _ = state.tx.send(event_json);

    Ok(StatusCode::NO_CONTENT)
}
