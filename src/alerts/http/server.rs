use crate::agents::{alert_analyzer, health};
use crate::database::{db, models};
use axum::{
    Json, Router,
    extract::Path,
    extract::State,
    http::StatusCode,
    response::sse::{Event, Sse},
    routing::{get, post},
};
use color_eyre::Result;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::services::ServeDir;

use crate::config::{AppConfig, PrefixesConfig};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum SseEvent {
    #[serde(rename = "new_alert")]
    NewAlert { alert_id: i64 },
    #[serde(rename = "chat_message")]
    ChatMessage { alert_id: i64, message_id: i64 },
    #[serde(rename = "alert_deleted")]
    AlertDeleted { alert_id: i64 },
    #[serde(rename = "health_check")]
    HealthCheck { status: String },
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Clone)]
pub struct AppState {
    pub tx: broadcast::Sender<String>,
    pub config: AppConfig,
    pub prefixes_config: PrefixesConfig,
    pub db_pool: Arc<SqlitePool>,
}

pub async fn start(tx: broadcast::Sender<String>, config: AppConfig) -> Result<()> {
    // Initialize database
    let db_pool = db::init_database().await?;

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
        db_pool,
    };

    // build our application with routes
    // API routes must come before static file serving
    let app = Router::new()
        .route("/api/messages/stream", get(message_stream))
        .route("/api/health", get(health_check))
        .route("/api/alerts", get(list_alerts).post(process_alert))
        .route("/api/alerts/{id}", get(get_alert).delete(delete_alert))
        .route("/api/alerts/{id}/chat", post(chat_with_alert))
        // Serve static files as fallback (must be last)
        .fallback_service(serve_dir)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
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

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct BGPAlerterAlert {
    pub message: String,
    pub description: String,
    pub details: Details,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
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

async fn process_alert(
    State(state): State<AppState>,
    Json(payload): Json<BGPAlerterAlert>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::debug!("Received alert: {:#?}", payload);

    // Check if alert is relevant to our monitored resources
    if !state.prefixes_config.is_alert_relevant(&payload) {
        tracing::debug!(
            "Alert for prefix {} (ASN: {}) is not relevant to monitored resources, skipping",
            payload.details.prefix,
            payload.details.asn
        );
        let event = SseEvent::Error {
            message: format!(
                "Alert ignored: prefix {} not in monitored resources",
                payload.details.prefix
            ),
        };
        let _ = state
            .tx
            .send(serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string()));
        return Ok(Json(serde_json::json!({
            "error": "Alert ignored: not relevant to monitored resources"
        })));
    }

    match alert_analyzer::AlertAnalyzer::run(payload.clone(), &state.config).await {
        Ok(result) => {
            // Save alert and initial response to database
            let alert_data_json = serde_json::to_string(&payload).map_err(|e| {
                tracing::error!("Failed to serialize alert: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            let timestamp = models::get_current_timestamp();

            let alert_id = sqlx::query_scalar::<_, i64>(
                r#"
                INSERT INTO alerts (alert_data, initial_response, created_at, updated_at)
                VALUES (?, ?, ?, ?)
                RETURNING id
                "#,
            )
            .bind(&alert_data_json)
            .bind(&result)
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

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
}

async fn list_alerts(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let rows = sqlx::query(
        r#"
        SELECT id, alert_data, created_at
        FROM alerts
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(&*state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut alerts = Vec::new();
    for row in rows {
        let id: i64 = row.get(0);
        let alert_data: String = row.get(1);
        let created_at: String = row.get(2);

        let alert_json: serde_json::Value =
            serde_json::from_str(&alert_data).unwrap_or_else(|_| serde_json::json!({}));

        alerts.push(serde_json::json!({
            "id": id,
            "alert_data": alert_json,
            "created_at": created_at
        }));
    }

    Ok(Json(alerts))
}

async fn get_alert(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Get alert
    let alert_row = sqlx::query(
        r#"
        SELECT id, alert_data, initial_response, created_at, updated_at
        FROM alerts
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(&*state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let alert_row = match alert_row {
        Some(row) => row,
        None => return Err(StatusCode::NOT_FOUND),
    };

    let alert_data: String = alert_row.get(1);
    let initial_response: String = alert_row.get(2);
    let created_at: String = alert_row.get(3);
    let updated_at: String = alert_row.get(4);

    let alert_json: serde_json::Value = serde_json::from_str(&alert_data).map_err(|e| {
        tracing::error!("Failed to parse alert data: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get chat messages
    let chat_rows = sqlx::query(
        r#"
        SELECT id, alert_id, role, content, created_at
        FROM chat_messages
        WHERE alert_id = ?
        ORDER BY created_at ASC
        "#,
    )
    .bind(id)
    .fetch_all(&*state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut chat_messages = Vec::new();
    for row in chat_rows {
        let msg_id: i64 = row.get(0);
        let alert_id: i64 = row.get(1);
        let role: String = row.get(2);
        let content: String = row.get(3);
        let created_at: String = row.get(4);

        chat_messages.push(serde_json::json!({
            "id": msg_id,
            "alert_id": alert_id,
            "role": role,
            "content": content,
            "created_at": created_at
        }));
    }

    Ok(Json(serde_json::json!({
        "alert": alert_json,
        "initial_response": initial_response,
        "chat_messages": chat_messages,
        "created_at": created_at,
        "updated_at": updated_at
    })))
}

async fn chat_with_alert(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<ChatRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Get alert and chat history
    let alert_row = sqlx::query(
        r#"
        SELECT id, alert_data, initial_response
        FROM alerts
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(&*state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let alert_row = match alert_row {
        Some(row) => row,
        None => return Err(StatusCode::NOT_FOUND),
    };

    let alert_data: String = alert_row.get(1);
    let initial_response: String = alert_row.get(2);

    let alert: BGPAlerterAlert = serde_json::from_str(&alert_data).map_err(|e| {
        tracing::error!("Failed to parse alert data: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get chat history
    let chat_rows = sqlx::query(
        r#"
        SELECT id, alert_id, role, content, created_at
        FROM chat_messages
        WHERE alert_id = ?
        ORDER BY created_at ASC
        "#,
    )
    .bind(id)
    .fetch_all(&*state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let chat_history: Vec<models::ChatMessage> = chat_rows
        .into_iter()
        .map(|row| {
            models::ChatMessage::from_row(
                row.get(0),
                row.get(1),
                row.get(2),
                row.get(3),
                row.get(4),
            )
        })
        .collect();

    // Save user message
    let timestamp = models::get_current_timestamp();
    sqlx::query(
        r#"
        INSERT INTO chat_messages (alert_id, role, content, created_at)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind("user")
    .bind(&payload.message)
    .bind(&timestamp)
    .execute(&*state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Run chat agent
    let assistant_response = match crate::agents::chat::Chat::run(
        alert,
        &initial_response,
        &chat_history,
        &payload.message,
        &state.config,
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
    let timestamp = models::get_current_timestamp();
    let message_id = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO chat_messages (alert_id, role, content, created_at)
        VALUES (?, ?, ?, ?)
        RETURNING id
        "#,
    )
    .bind(id)
    .bind("assistant")
    .bind(&assistant_response)
    .bind(&timestamp)
    .fetch_one(&*state.db_pool)
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

async fn delete_alert(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    // Delete alert (chat_messages will be deleted via CASCADE)
    let result = sqlx::query(
        r#"
        DELETE FROM alerts
        WHERE id = ?
        "#,
    )
    .bind(id)
    .execute(&*state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    // Broadcast SSE notification
    let event = SseEvent::AlertDeleted { alert_id: id };
    let event_json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
    let _ = state.tx.send(event_json);

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_state() -> AppState {
        // Create in-memory database
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        // Run migrations
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS alerts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                alert_data TEXT NOT NULL,
                initial_response TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS chat_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                alert_id INTEGER NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (alert_id) REFERENCES alerts(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_chat_messages_alert_id ON chat_messages(alert_id)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let (tx, _) = broadcast::channel(100);
        let config = AppConfig {
            server_port: 7654,
            llm_model_name: "test-model".to_string(),
        };
        let prefixes_config = PrefixesConfig::load("prefixes.test.yml").unwrap();

        AppState {
            tx,
            config,
            prefixes_config,
            db_pool: Arc::new(pool),
        }
    }

    #[tokio::test]
    async fn test_list_alerts_empty() {
        let state = create_test_state().await;
        let result = list_alerts(State(state)).await;

        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json.len(), 0);
    }

    #[tokio::test]
    async fn test_list_alerts_with_data() {
        let state = create_test_state().await;

        // Insert test alert
        let alert_data = r#"{"message":"test"}"#;
        let timestamp = models::get_current_timestamp();
        sqlx::query(
            r#"
            INSERT INTO alerts (alert_data, initial_response, created_at, updated_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(alert_data)
        .bind("Test response")
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&*state.db_pool)
        .await
        .unwrap();

        let result = list_alerts(State(state)).await;

        assert!(result.is_ok());
        let alerts = result.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0]["id"], 1);
    }

    #[tokio::test]
    async fn test_get_alert_not_found() {
        let state = create_test_state().await;
        let result = get_alert(State(state), Path(999)).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_alert_with_chat() {
        let state = create_test_state().await;

        // Insert alert
        let alert_data = r#"{"message":"test","description":"test","details":{"prefix":"192.0.2.0/24","asn":"3333"}}"#;
        let timestamp = models::get_current_timestamp();
        let alert_id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO alerts (alert_data, initial_response, created_at, updated_at)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(alert_data)
        .bind("Initial response")
        .bind(&timestamp)
        .bind(&timestamp)
        .fetch_one(&*state.db_pool)
        .await
        .unwrap();

        // Insert chat messages
        sqlx::query(
            r#"
            INSERT INTO chat_messages (alert_id, role, content, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(alert_id)
        .bind("user")
        .bind("Question 1")
        .bind(&timestamp)
        .execute(&*state.db_pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            INSERT INTO chat_messages (alert_id, role, content, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(alert_id)
        .bind("assistant")
        .bind("Answer 1")
        .bind(&timestamp)
        .execute(&*state.db_pool)
        .await
        .unwrap();

        let result = get_alert(State(state), Path(alert_id)).await;

        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["initial_response"], "Initial response");
        assert_eq!(json["chat_messages"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_delete_alert_not_found() {
        let state = create_test_state().await;
        let result = delete_alert(State(state), Path(999)).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_alert_success() {
        let state = create_test_state().await;

        // Insert alert
        let alert_data = r#"{"message":"test"}"#;
        let timestamp = models::get_current_timestamp();
        let alert_id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO alerts (alert_data, initial_response, created_at, updated_at)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(alert_data)
        .bind("response")
        .bind(&timestamp)
        .bind(&timestamp)
        .fetch_one(&*state.db_pool)
        .await
        .unwrap();

        // Insert chat message
        sqlx::query(
            r#"
            INSERT INTO chat_messages (alert_id, role, content, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(alert_id)
        .bind("user")
        .bind("test message")
        .bind(&timestamp)
        .execute(&*state.db_pool)
        .await
        .unwrap();

        let db_pool = Arc::clone(&state.db_pool);
        let result = delete_alert(State(state), Path(alert_id)).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);

        // Verify alert is deleted
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM alerts WHERE id = ?")
            .bind(alert_id)
            .fetch_one(&*db_pool)
            .await
            .unwrap();
        assert_eq!(count, 0);

        // Verify chat messages are cascade deleted
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM chat_messages WHERE alert_id = ?")
                .bind(alert_id)
                .fetch_one(&*db_pool)
                .await
                .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_sse_event_serialization() {
        let event = SseEvent::NewAlert { alert_id: 123 };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("new_alert"));
        assert!(json.contains("123"));

        let event = SseEvent::ChatMessage {
            alert_id: 123,
            message_id: 456,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("chat_message"));
        assert!(json.contains("123"));
        assert!(json.contains("456"));

        let event = SseEvent::AlertDeleted { alert_id: 123 };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("alert_deleted"));
        assert!(json.contains("123"));

        let event = SseEvent::Error {
            message: "test error".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("test error"));
    }

    #[test]
    fn test_sse_event_deserialization() {
        let json = r#"{"type":"new_alert","alert_id":123}"#;
        let event: SseEvent = serde_json::from_str(json).unwrap();
        match event {
            SseEvent::NewAlert { alert_id } => assert_eq!(alert_id, 123),
            _ => panic!("Wrong event type"),
        }

        let json = r#"{"type":"chat_message","alert_id":123,"message_id":456}"#;
        let event: SseEvent = serde_json::from_str(json).unwrap();
        match event {
            SseEvent::ChatMessage {
                alert_id,
                message_id,
            } => {
                assert_eq!(alert_id, 123);
                assert_eq!(message_id, 456);
            }
            _ => panic!("Wrong event type"),
        }
    }
}
