use crate::database::db;
use axum::body::Body;
use axum::{
    Router,
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::config::{AppConfig, PrefixesConfig};

use super::routes;

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
    pub config: Arc<AppConfig>,
    pub prefixes_config: PrefixesConfig,
    pub db_pool: Arc<SqlitePool>,
}

pub async fn start(tx: broadcast::Sender<String>, config: Arc<AppConfig>) -> Result<()> {
    // Initialize database
    let db_pool = db::init_database().await?;

    // Load prefixes configuration
    let prefixes_config = PrefixesConfig::load("prefixes.yml")
        .map_err(|e| color_eyre::eyre::eyre!("Failed to load prefixes.yml: {}", e))?;

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
        .route("/api/messages/stream", get(routes::message_stream))
        .route("/api/health", get(routes::health_check))
        .route(
            "/api/alerts",
            get(routes::alerts::list_alerts).post(routes::alerts::process_alert),
        )
        .route(
            "/api/alerts/{id}",
            get(routes::alerts::get_alert).delete(routes::alerts::delete_alert),
        )
        .route(
            "/api/alerts/{id}/chat",
            post(routes::alerts::chat_with_alert),
        )
        // MCP server management routes
        .route(
            "/api/mcps",
            get(routes::mcp::list_mcp_servers).post(routes::mcp::create_mcp_server),
        )
        .route(
            "/api/mcps/{id}",
            get(routes::mcp::get_mcp_server)
                .put(routes::mcp::update_mcp_server)
                .delete(routes::mcp::delete_mcp_server),
        )
        .route("/api/mcps/{id}/test", post(routes::mcp::test_mcp_server))
        .route(
            "/api/mcps/enable-native",
            post(routes::mcp::enable_native_mcp_servers),
        )
        // Serve static files as fallback (must be last)
        // For SPA routing, serve index.html for all non-API routes
        .fallback(serve_spa)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("Server starting on http://0.0.0.0:{}", port);
    axum::serve(listener, app).await?;

    Ok(())
}

/// SPA fallback handler: serves index.html for all non-API routes
async fn serve_spa(uri: Uri) -> Response {
    use axum::http::HeaderMap;
    use axum::http::header::HeaderValue;
    use std::path::PathBuf;

    // Try to serve the requested file first
    let file_path = uri.path().trim_start_matches('/');
    let dist_path = PathBuf::from("web-ui/dist").join(file_path);

    // If the file exists and is not a directory, serve it
    if dist_path.exists()
        && dist_path.is_file()
        && !file_path.is_empty()
        && let Ok(contents) = tokio::fs::read(&dist_path).await
    {
        let mut headers = HeaderMap::new();
        // Set appropriate content type based on file extension
        if file_path.ends_with(".html") {
            headers.insert("content-type", HeaderValue::from_static("text/html"));
        } else if file_path.ends_with(".js") {
            headers.insert(
                "content-type",
                HeaderValue::from_static("application/javascript"),
            );
        } else if file_path.ends_with(".css") {
            headers.insert("content-type", HeaderValue::from_static("text/css"));
        }
        return Response::builder()
            .status(StatusCode::OK)
            .header(
                "content-type",
                headers
                    .get("content-type")
                    .unwrap_or(&HeaderValue::from_static("application/octet-stream")),
            )
            .body(Body::from(contents))
            .unwrap()
            .into_response();
    }

    // For SPA routing, serve index.html for all non-API routes
    let index_path = PathBuf::from("web-ui/dist/index.html");
    if let Ok(contents) = tokio::fs::read(&index_path).await {
        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", HeaderValue::from_static("text/html"))
            .body(Body::from(contents))
            .unwrap()
            .into_response()
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("index.html not found"))
            .unwrap()
            .into_response()
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

#[cfg(test)]
mod tests {
    use super::routes;
    use super::*;
    use crate::database::models;
    use axum::extract::{Path, Query, State};
    use axum::{Json, http::StatusCode};

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
                kind TEXT NOT NULL DEFAULT 'bgp_alerter',
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
            CREATE TABLE IF NOT EXISTS mcp_servers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                transport_type TEXT NOT NULL CHECK(transport_type IN ('http', 'stdio')),
                url TEXT,
                command TEXT,
                args TEXT,
                env TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                is_native INTEGER NOT NULL DEFAULT 0,
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
            CREATE INDEX IF NOT EXISTS idx_chat_messages_alert_id ON chat_messages(alert_id)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_mcp_servers_enabled ON mcp_servers(enabled)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let (tx, _) = broadcast::channel(100);
        let config = Arc::new(AppConfig {
            server_port: 7654,
            llm_model_name: "test-model".to_string(),
        });
        let prefixes_config = PrefixesConfig::load("prefixes.test.yml").unwrap();

        AppState {
            tx,
            config,
            prefixes_config,
            db_pool: Arc::new(pool),
        }
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

    // ========================================================================
    // MCP Server API Tests
    // ========================================================================

    #[tokio::test]
    async fn test_list_mcp_servers_empty() {
        let state = create_test_state().await;
        let query = Query(routes::mcp::ListMcpServersQuery { kind: None });
        let result = routes::mcp::list_mcp_servers(State(state), query).await;

        assert!(result.is_ok());
        let servers = result.unwrap();
        assert_eq!(servers.len(), 0);
    }

    #[tokio::test]
    async fn test_create_mcp_server_http() {
        let state = create_test_state().await;

        let payload = models::CreateMcpServer::Http {
            name: "test-http".to_string(),
            description: Some("Test HTTP server".to_string()),
            url: "https://example.com/mcp".to_string(),
            enabled: true,
        };

        let result = routes::mcp::create_mcp_server(State(state), Json(payload)).await;

        assert!(result.is_ok());
        let (status, Json(server)) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(server.name(), "test-http");
        assert!(matches!(server, models::McpServer::Http { .. }));
    }

    #[tokio::test]
    async fn test_create_mcp_server_stdio() {
        let state = create_test_state().await;

        let payload = models::CreateMcpServer::Stdio {
            name: "test-stdio".to_string(),
            description: Some("Test stdio server".to_string()),
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            env: std::collections::HashMap::new(),
            enabled: true,
        };

        let result = routes::mcp::create_mcp_server(State(state), Json(payload)).await;

        assert!(result.is_ok());
        let (status, Json(server)) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(server.name(), "test-stdio");
        match server {
            models::McpServer::Stdio { command, .. } => {
                assert_eq!(command, "echo");
            }
            _ => panic!("Expected Stdio variant"),
        }
    }

    #[tokio::test]
    async fn test_create_mcp_server_validation_error() {
        let state = create_test_state().await;

        // HTTP server with empty URL
        let payload = models::CreateMcpServer::Http {
            name: "test".to_string(),
            description: None,
            url: "".to_string(), // Empty URL
            enabled: true,
        };

        let result = routes::mcp::create_mcp_server(State(state), Json(payload)).await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_mcp_server_duplicate_name() {
        let state = create_test_state().await;

        let payload = models::CreateMcpServer::Http {
            name: "duplicate".to_string(),
            description: None,
            url: "https://example.com".to_string(),
            enabled: true,
        };

        // Create first server
        let _ = routes::mcp::create_mcp_server(State(state.clone()), Json(payload.clone())).await;

        // Try to create duplicate
        let result = routes::mcp::create_mcp_server(State(state), Json(payload)).await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_get_mcp_server_by_id() {
        let state = create_test_state().await;

        // Create a server first
        let payload = models::CreateMcpServer::Http {
            name: "test".to_string(),
            description: None,
            url: "https://example.com".to_string(),
            enabled: true,
        };

        let (_, Json(created)) =
            routes::mcp::create_mcp_server(State(state.clone()), Json(payload))
                .await
                .unwrap();

        // Get the server
        let created_id = created.meta().id;
        let result = routes::mcp::get_mcp_server(State(state), Path(created_id)).await;

        assert!(result.is_ok());
        let Json(server) = result.unwrap();
        assert_eq!(server.meta().id, created_id);
        assert_eq!(server.name(), "test");
    }

    #[tokio::test]
    async fn test_get_mcp_server_not_found() {
        let state = create_test_state().await;
        let result = routes::mcp::get_mcp_server(State(state), Path(9999)).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_mcp_server() {
        let state = create_test_state().await;

        // Create a server first
        let payload = models::CreateMcpServer::Http {
            name: "original".to_string(),
            description: Some("Original".to_string()),
            url: "https://example.com".to_string(),
            enabled: true,
        };

        let (_, Json(created)) =
            routes::mcp::create_mcp_server(State(state.clone()), Json(payload))
                .await
                .unwrap();

        // Update the server
        let update = models::UpdateMcpServer {
            name: Some("updated".to_string()),
            description: Some("Updated description".to_string()),
            enabled: Some(false),
            ..Default::default()
        };

        let result =
            routes::mcp::update_mcp_server(State(state), Path(created.meta().id), Json(update))
                .await;

        assert!(result.is_ok());
        let Json(updated) = result.unwrap();
        assert_eq!(updated.name(), "updated");
        assert_eq!(
            updated.meta().description.as_deref(),
            Some("Updated description")
        );
        assert!(!updated.meta().enabled);
        // URL should remain unchanged
        match updated {
            models::McpServer::Http { url, .. } => {
                assert_eq!(url, "https://example.com");
            }
            _ => panic!("Expected HTTP variant"),
        }
    }

    #[tokio::test]
    async fn test_update_mcp_server_not_found() {
        let state = create_test_state().await;

        let update = models::UpdateMcpServer {
            name: Some("new".to_string()),
            ..Default::default()
        };

        let result = routes::mcp::update_mcp_server(State(state), Path(9999), Json(update)).await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_mcp_server_success() {
        let state = create_test_state().await;

        // Create a server first
        let payload = models::CreateMcpServer::Http {
            name: "to-delete".to_string(),
            description: None,
            url: "https://example.com".to_string(),
            enabled: true,
        };

        let (_, Json(created)) =
            routes::mcp::create_mcp_server(State(state.clone()), Json(payload))
                .await
                .unwrap();

        // Delete the server
        let created_id = created.meta().id;
        let result = routes::mcp::delete_mcp_server(State(state.clone()), Path(created_id)).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);

        // Verify it's gone
        let get_result = routes::mcp::get_mcp_server(State(state), Path(created_id)).await;
        assert!(get_result.is_err());
        assert_eq!(get_result.unwrap_err(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_mcp_server_not_found() {
        let state = create_test_state().await;
        let result = routes::mcp::delete_mcp_server(State(state), Path(9999)).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_mcp_servers_with_data() {
        let state = create_test_state().await;

        // Create two servers
        let server1 = models::CreateMcpServer::Http {
            name: "alpha".to_string(),
            description: None,
            url: "https://alpha.com".to_string(),
            enabled: true,
        };
        let server2 = models::CreateMcpServer::Stdio {
            name: "beta".to_string(),
            description: None,
            command: "echo".to_string(),
            args: vec![],
            env: std::collections::HashMap::new(),
            enabled: false,
        };

        let _ = routes::mcp::create_mcp_server(State(state.clone()), Json(server1)).await;
        let _ = routes::mcp::create_mcp_server(State(state.clone()), Json(server2)).await;

        let query = Query(routes::mcp::ListMcpServersQuery { kind: None });
        let result = routes::mcp::list_mcp_servers(State(state), query).await;

        assert!(result.is_ok());
        let servers = result.unwrap();
        assert_eq!(servers.len(), 2);
        // Should be sorted by name
        assert_eq!(servers[0].name(), "alpha");
        assert_eq!(servers[1].name(), "beta");
    }
}
