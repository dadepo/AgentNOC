use color_eyre::Result;
use sqlx::SqlitePool;
use std::sync::Arc;

use super::models::{CreateMcpServer, McpServer, UpdateMcpServer, get_current_timestamp};
use crate::native_mcps;

pub async fn init_database() -> Result<Arc<SqlitePool>> {
    // Database file location
    let db_path = "agent_noc.db";

    // Create connection pool
    let pool = SqlitePool::connect(&format!("sqlite://{db_path}?mode=rwc")).await?;

    // Run migrations
    run_migrations(&pool).await?;

    tracing::info!("Database initialized at {}", db_path);
    Ok(Arc::new(pool))
}

async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    // Alerts table
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
    .execute(pool)
    .await?;

    // Chat messages table
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
    .execute(pool)
    .await?;

    // MCP servers table
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
    .execute(pool)
    .await?;

    // Migration: Add is_native column if it doesn't exist (for existing databases)
    sqlx::query(
        r#"
        ALTER TABLE mcp_servers ADD COLUMN is_native INTEGER NOT NULL DEFAULT 0
        "#,
    )
    .execute(pool)
    .await
    .ok(); // Ignore error if column already exists

    // Migration: Add kind column if it doesn't exist (for existing databases)
    sqlx::query(
        r#"
        ALTER TABLE alerts ADD COLUMN kind TEXT NOT NULL DEFAULT 'bgp_alerter'
        "#,
    )
    .execute(pool)
    .await
    .ok(); // Ignore error if column already exists

    // Create indexes for performance
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_chat_messages_alert_id ON chat_messages(alert_id)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_chat_messages_created_at ON chat_messages(created_at)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_mcp_servers_enabled ON mcp_servers(enabled)
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

// ============================================================================
// MCP Server CRUD Operations
// ============================================================================

/// Get all MCP servers, optionally filtered by kind
pub async fn get_all_mcp_servers(pool: &SqlitePool, kind: Option<&str>) -> Result<Vec<McpServer>> {
    let query = match kind {
        Some("native") => {
            r#"
            SELECT id, name, description, transport_type, url, command, args, env, enabled, is_native, created_at, updated_at
            FROM mcp_servers
            WHERE is_native = 1
            ORDER BY name ASC
            "#
        }
        Some("custom") => {
            r#"
            SELECT id, name, description, transport_type, url, command, args, env, enabled, is_native, created_at, updated_at
            FROM mcp_servers
            WHERE is_native = 0
            ORDER BY name ASC
            "#
        }
        _ => {
            r#"
            SELECT id, name, description, transport_type, url, command, args, env, enabled, is_native, created_at, updated_at
            FROM mcp_servers
            ORDER BY name ASC
            "#
        }
    };

    let rows = sqlx::query(query).fetch_all(pool).await?;

    let mut servers = Vec::new();
    for row in rows {
        use sqlx::Row;
        let server = McpServer::from_row(
            row.get(0),
            row.get(1),
            row.get(2),
            row.get(3),
            row.get(4),
            row.get(5),
            row.get(6),
            row.get(7),
            row.get(8),
            row.get(9),
            row.get(10),
            row.get(11),
        )
        .map_err(|e| color_eyre::eyre::eyre!("Failed to parse MCP server: {}", e))?;
        servers.push(server);
    }

    Ok(servers)
}

/// Get only enabled MCP servers
pub async fn get_enabled_mcp_servers(pool: &SqlitePool) -> Result<Vec<McpServer>> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, description, transport_type, url, command, args, env, enabled, is_native, created_at, updated_at
        FROM mcp_servers
        WHERE enabled = 1
        ORDER BY name ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut servers = Vec::new();
    for row in rows {
        use sqlx::Row;
        let server = McpServer::from_row(
            row.get(0),
            row.get(1),
            row.get(2),
            row.get(3),
            row.get(4),
            row.get(5),
            row.get(6),
            row.get(7),
            row.get(8),
            row.get(9),
            row.get(10),
            row.get(11),
        )
        .map_err(|e| color_eyre::eyre::eyre!("Failed to parse MCP server: {}", e))?;
        servers.push(server);
    }

    Ok(servers)
}

/// Get a single MCP server by ID
pub async fn get_mcp_server_by_id(pool: &SqlitePool, id: i64) -> Result<Option<McpServer>> {
    let row = sqlx::query(
        r#"
        SELECT id, name, description, transport_type, url, command, args, env, enabled, is_native, created_at, updated_at
        FROM mcp_servers
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            use sqlx::Row;
            let server = McpServer::from_row(
                row.get(0),
                row.get(1),
                row.get(2),
                row.get(3),
                row.get(4),
                row.get(5),
                row.get(6),
                row.get(7),
                row.get(8),
                row.get(9),
                row.get(10),
                row.get(11),
            )
            .map_err(|e| color_eyre::eyre::eyre!("Failed to parse MCP server: {}", e))?;
            Ok(Some(server))
        }
        None => Ok(None),
    }
}

/// Create a new MCP server
pub async fn create_mcp_server(pool: &SqlitePool, server: &CreateMcpServer) -> Result<McpServer> {
    server
        .validate()
        .map_err(|e| color_eyre::eyre::eyre!("Validation error: {}", e))?;

    let timestamp = get_current_timestamp();

    // Extract fields based on variant
    let (name, description, transport_type, url, command, args_json, env_json, enabled) =
        match server {
            CreateMcpServer::Http {
                name,
                description,
                url,
                enabled,
            } => (
                name.clone(),
                description.clone(),
                "http",
                Some(url.clone()),
                None,
                None,
                None,
                *enabled,
            ),
            CreateMcpServer::Stdio {
                name,
                description,
                command,
                args,
                env,
                enabled,
            } => {
                let args_json = if args.is_empty() {
                    None
                } else {
                    Some(serde_json::to_string(args)?)
                };
                let env_json = if env.is_empty() {
                    None
                } else {
                    Some(serde_json::to_string(env)?)
                };
                (
                    name.clone(),
                    description.clone(),
                    "stdio",
                    None,
                    Some(command.clone()),
                    args_json,
                    env_json,
                    *enabled,
                )
            }
        };

    let id = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO mcp_servers (name, description, transport_type, url, command, args, env, enabled, is_native, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING id
        "#,
    )
    .bind(&name)
    .bind(&description)
    .bind(transport_type)
    .bind(&url)
    .bind(&command)
    .bind(&args_json)
    .bind(&env_json)
    .bind(enabled as i64)
    .bind(0) // is_native = 0 for user-created servers
    .bind(&timestamp)
    .bind(&timestamp)
    .fetch_one(pool)
    .await?;

    get_mcp_server_by_id(pool, id)
        .await?
        .ok_or_else(|| color_eyre::eyre::eyre!("Failed to retrieve created MCP server"))
}

/// Update an existing MCP server
/// Note: transport_type cannot be changed after creation
pub async fn update_mcp_server(
    pool: &SqlitePool,
    id: i64,
    update: &UpdateMcpServer,
) -> Result<Option<McpServer>> {
    // First check if server exists
    let existing = get_mcp_server_by_id(pool, id).await?;
    if existing.is_none() {
        return Ok(None);
    }
    let existing = existing.unwrap();

    let timestamp = get_current_timestamp();

    // Extract existing values based on variant
    let (
        existing_name,
        existing_description,
        _transport_type,
        existing_url,
        existing_command,
        existing_args,
        existing_env,
        existing_enabled,
    ) = match &existing {
        McpServer::Http { meta, url } => (
            meta.name.clone(),
            meta.description.clone(),
            "http",
            Some(url.clone()),
            None::<String>,
            Vec::new(),
            std::collections::HashMap::new(),
            meta.enabled,
        ),
        McpServer::Stdio {
            meta,
            command,
            args,
            env,
        } => (
            meta.name.clone(),
            meta.description.clone(),
            "stdio",
            None,
            Some(command.clone()),
            args.clone(),
            env.clone(),
            meta.enabled,
        ),
    };

    // Build updated values, using existing values as defaults
    let name = update.name.as_ref().unwrap_or(&existing_name);
    let description = update
        .description
        .as_ref()
        .or(existing_description.as_ref());
    let url = update.url.as_ref().or(existing_url.as_ref());
    let command = update.command.as_ref().or(existing_command.as_ref());
    let args = update.args.as_ref().unwrap_or(&existing_args);
    let env = update.env.as_ref().unwrap_or(&existing_env);
    let enabled = update.enabled.unwrap_or(existing_enabled);

    let args_json = if args.is_empty() {
        None
    } else {
        Some(serde_json::to_string(args)?)
    };
    let env_json = if env.is_empty() {
        None
    } else {
        Some(serde_json::to_string(env)?)
    };

    sqlx::query(
            r#"
        UPDATE mcp_servers
        SET name = ?, description = ?, url = ?, command = ?, args = ?, env = ?, enabled = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(name)
    .bind(description)
    .bind(url)
    .bind(command)
    .bind(&args_json)
    .bind(&env_json)
    .bind(enabled as i64)
    .bind(&timestamp)
    .bind(id)
    .execute(pool)
    .await?;

    // Note: transport_type is intentionally NOT updated - it cannot be changed after creation

    get_mcp_server_by_id(pool, id).await
}

/// Delete an MCP server
pub async fn delete_mcp_server(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM mcp_servers WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Enable or disable native MCP servers
/// If enabled=true: Insert all native MCPs from code (skip if already exist)
/// If enabled=false: Delete all native MCPs from DB
pub async fn enable_native_mcp_servers(pool: &SqlitePool, enabled: bool) -> Result<()> {
    if enabled {
        let timestamp = get_current_timestamp();
        let native_servers = native_mcps::get_native_mcp_servers();

        for server in native_servers {
            // Check if server already exists
            let exists: Option<i64> =
                sqlx::query_scalar("SELECT id FROM mcp_servers WHERE name = ? AND is_native = 1")
                    .bind(server.name())
                    .fetch_optional(pool)
                    .await?;

            if exists.is_some() {
                // Already exists, skip
                continue;
            }

            // Extract fields based on variant
            let (
                name,
                description,
                transport_type,
                url,
                command,
                args_json,
                env_json,
                enabled_flag,
            ) = match &server {
                CreateMcpServer::Http {
                    name,
                    description,
                    url,
                    enabled,
                } => (
                    name.clone(),
                    description.clone(),
                    "http",
                    Some(url.clone()),
                    None,
                    None,
                    None,
                    *enabled,
                ),
                CreateMcpServer::Stdio {
                    name,
                    description,
                    command,
                    args,
                    env,
                    enabled,
                } => {
                    let args_json = if args.is_empty() {
                        None
                    } else {
                        Some(serde_json::to_string(args)?)
                    };
                    let env_json = if env.is_empty() {
                        None
                    } else {
                        Some(serde_json::to_string(env)?)
                    };
                    (
                        name.clone(),
                        description.clone(),
                        "stdio",
                        None,
                        Some(command.clone()),
                        args_json,
                        env_json,
                        *enabled,
                    )
                }
            };

            sqlx::query(
                r#"
                INSERT INTO mcp_servers (name, description, transport_type, url, command, args, env, enabled, is_native, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&name)
            .bind(&description)
            .bind(transport_type)
            .bind(&url)
            .bind(&command)
            .bind(&args_json)
            .bind(&env_json)
            .bind(enabled_flag as i64)
            .bind(1) // is_native = 1
            .bind(&timestamp)
            .bind(&timestamp)
            .execute(pool)
            .await?;
        }

        tracing::info!("Native MCP servers enabled");
    } else {
        // Delete all native MCP servers
        sqlx::query("DELETE FROM mcp_servers WHERE is_native = 1")
            .execute(pool)
            .await?;

        tracing::info!("Native MCP servers disabled");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::models::AlertKind;
    use sqlx::Row;

    async fn create_test_db() -> Result<SqlitePool> {
        // Use in-memory database for tests
        let pool = SqlitePool::connect("sqlite::memory:").await?;

        // Run all migrations
        run_migrations(&pool).await?;

        Ok(pool)
    }

    #[tokio::test]
    async fn test_database_initialization() {
        let pool = create_test_db().await.unwrap();

        // Verify tables exist
        let table_check = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('alerts', 'chat_messages', 'mcp_servers')"
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(table_check.len(), 3);
    }

    // ========================================================================
    // MCP Server CRUD Tests
    // ========================================================================

    #[tokio::test]
    async fn test_create_mcp_server_http() {
        let pool = create_test_db().await.unwrap();

        let server = CreateMcpServer::Http {
            name: "test-http".to_string(),
            description: Some("Test HTTP server".to_string()),
            url: "https://example.com/mcp".to_string(),
            enabled: true,
        };

        let created = create_mcp_server(&pool, &server).await.unwrap();

        assert_eq!(created.name(), "test-http");
        assert!(created.meta().enabled);

        match created {
            McpServer::Http { url, .. } => {
                assert_eq!(url, "https://example.com/mcp");
            }
            _ => panic!("Expected HTTP variant"),
        }
    }

    #[tokio::test]
    async fn test_create_mcp_server_stdio() {
        let pool = create_test_db().await.unwrap();

        let server = CreateMcpServer::Stdio {
            name: "test-stdio".to_string(),
            description: Some("Test stdio server".to_string()),
            command: "uvx".to_string(),
            args: vec!["--from".to_string(), "test".to_string()],
            env: [("KEY".to_string(), "value".to_string())]
                .into_iter()
                .collect(),
            enabled: true,
        };

        let created = create_mcp_server(&pool, &server).await.unwrap();

        assert_eq!(created.name(), "test-stdio");

        match created {
            McpServer::Stdio {
                command, args, env, ..
            } => {
                assert_eq!(command, "uvx");
                assert_eq!(args, vec!["--from".to_string(), "test".to_string()]);
                assert_eq!(env.get("KEY"), Some(&"value".to_string()));
            }
            _ => panic!("Expected Stdio variant"),
        }
    }

    #[tokio::test]
    async fn test_create_mcp_server_validation_error() {
        let pool = create_test_db().await.unwrap();

        // HTTP server with empty URL
        let server = CreateMcpServer::Http {
            name: "test".to_string(),
            description: None,
            url: "".to_string(), // Empty URL
            enabled: true,
        };

        let result = create_mcp_server(&pool, &server).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_all_mcp_servers() {
        let pool = create_test_db().await.unwrap();

        // Create two servers
        let server1 = CreateMcpServer::Http {
            name: "alpha".to_string(),
            description: None,
            url: "https://example1.com".to_string(),
            enabled: true,
        };
        let server2 = CreateMcpServer::Http {
            name: "beta".to_string(),
            description: None,
            url: "https://example2.com".to_string(),
            enabled: false,
        };

        create_mcp_server(&pool, &server1).await.unwrap();
        create_mcp_server(&pool, &server2).await.unwrap();

        let servers = get_all_mcp_servers(&pool, None).await.unwrap();
        assert_eq!(servers.len(), 2);
        // Should be sorted by name
        assert_eq!(servers[0].name(), "alpha");
        assert_eq!(servers[1].name(), "beta");
    }

    #[tokio::test]
    async fn test_get_enabled_mcp_servers() {
        let pool = create_test_db().await.unwrap();

        // Create two servers, one enabled, one disabled
        let server1 = CreateMcpServer::Http {
            name: "enabled".to_string(),
            description: None,
            url: "https://example1.com".to_string(),
            enabled: true,
        };
        let server2 = CreateMcpServer::Http {
            name: "disabled".to_string(),
            description: None,
            url: "https://example2.com".to_string(),
            enabled: false,
        };

        create_mcp_server(&pool, &server1).await.unwrap();
        create_mcp_server(&pool, &server2).await.unwrap();

        let servers = get_enabled_mcp_servers(&pool).await.unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name(), "enabled");
    }

    #[tokio::test]
    async fn test_get_mcp_server_by_id() {
        let pool = create_test_db().await.unwrap();

        let server = CreateMcpServer::Http {
            name: "test".to_string(),
            description: Some("Test server".to_string()),
            url: "https://example.com".to_string(),
            enabled: true,
        };

        let created = create_mcp_server(&pool, &server).await.unwrap();

        let retrieved = get_mcp_server_by_id(&pool, created.meta().id)
            .await
            .unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name(), "test");

        // Non-existent ID
        let not_found = get_mcp_server_by_id(&pool, 9999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_update_mcp_server() {
        let pool = create_test_db().await.unwrap();

        let server = CreateMcpServer::Http {
            name: "original".to_string(),
            description: Some("Original description".to_string()),
            url: "https://original.com".to_string(),
            enabled: true,
        };

        let created = create_mcp_server(&pool, &server).await.unwrap();

        let update = UpdateMcpServer {
            name: Some("updated".to_string()),
            description: Some("Updated description".to_string()),
            enabled: Some(false),
            ..Default::default()
        };

        let updated = update_mcp_server(&pool, created.meta().id, &update)
            .await
            .unwrap();
        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert_eq!(updated.name(), "updated");
        assert_eq!(
            updated.meta().description.as_deref(),
            Some("Updated description")
        );
        assert!(!updated.meta().enabled);

        // URL should remain unchanged
        match updated {
            McpServer::Http { url, .. } => {
                assert_eq!(url, "https://original.com");
            }
            _ => panic!("Expected HTTP variant"),
        }
    }

    #[tokio::test]
    async fn test_update_mcp_server_not_found() {
        let pool = create_test_db().await.unwrap();

        let update = UpdateMcpServer {
            name: Some("new".to_string()),
            ..Default::default()
        };

        let result = update_mcp_server(&pool, 9999, &update).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_mcp_server() {
        let pool = create_test_db().await.unwrap();

        let server = CreateMcpServer::Http {
            name: "to-delete".to_string(),
            description: None,
            url: "https://example.com".to_string(),
            enabled: true,
        };

        let created = create_mcp_server(&pool, &server).await.unwrap();

        let server_id = created.meta().id;
        let deleted = delete_mcp_server(&pool, server_id).await.unwrap();
        assert!(deleted);

        // Verify it's gone
        let retrieved = get_mcp_server_by_id(&pool, server_id).await.unwrap();
        assert!(retrieved.is_none());

        // Try to delete again
        let deleted_again = delete_mcp_server(&pool, server_id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_enable_native_mcp_servers() {
        let pool = create_test_db().await.unwrap();

        // Enable native servers
        enable_native_mcp_servers(&pool, true).await.unwrap();

        let servers = get_all_mcp_servers(&pool, None).await.unwrap();
        assert_eq!(servers.len(), 2);

        // Find the servers by name
        let ripestat = servers.iter().find(|s| s.name() == "ripestat").unwrap();
        assert!(matches!(ripestat, McpServer::Http { .. }));
        assert!(ripestat.meta().is_native);

        let whois = servers.iter().find(|s| s.name() == "whois").unwrap();
        match whois {
            McpServer::Stdio { command, args, .. } => {
                assert_eq!(command, "uvx");
                assert!(!args.is_empty());
            }
            _ => panic!("Expected Stdio variant"),
        }
        assert!(whois.meta().is_native);

        // Running enable again should not add duplicates
        enable_native_mcp_servers(&pool, true).await.unwrap();
        let servers = get_all_mcp_servers(&pool, None).await.unwrap();
        assert_eq!(servers.len(), 2);

        // Disable native servers
        enable_native_mcp_servers(&pool, false).await.unwrap();
        let servers = get_all_mcp_servers(&pool, None).await.unwrap();
        assert_eq!(servers.len(), 0);
    }

    #[tokio::test]
    async fn test_insert_alert() {
        let pool = create_test_db().await.unwrap();

        let alert_data = r#"{"message":"test"}"#;
        let response = "Test response";
        let timestamp = "2025-01-15T10:30:00Z";

        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO alerts (alert_data, initial_response, kind, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(alert_data)
        .bind(response)
        .bind(AlertKind::BgpAlerter.as_str())
        .bind(timestamp)
        .bind(timestamp)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(id, 1);

        // Verify we can retrieve it
        let row =
            sqlx::query("SELECT id, alert_data, initial_response, kind FROM alerts WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(row.get::<i64, _>(0), id);
        assert_eq!(row.get::<String, _>(1), alert_data);
        assert_eq!(row.get::<String, _>(2), response);
        assert_eq!(row.get::<String, _>(3), AlertKind::BgpAlerter.as_str());
    }

    #[tokio::test]
    async fn test_insert_chat_message() {
        let pool = create_test_db().await.unwrap();

        // First create an alert
        let alert_id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO alerts (alert_data, initial_response, kind, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(r#"{"message":"test"}"#)
        .bind("response")
        .bind(AlertKind::BgpAlerter.as_str())
        .bind("2025-01-15T10:30:00Z")
        .bind("2025-01-15T10:30:00Z")
        .fetch_one(&pool)
        .await
        .unwrap();

        // Insert chat message
        let msg_id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO chat_messages (alert_id, role, content, created_at)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(alert_id)
        .bind("user")
        .bind("Hello")
        .bind("2025-01-15T10:30:00Z")
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(msg_id, 1);

        // Verify foreign key relationship
        let row = sqlx::query("SELECT alert_id, role, content FROM chat_messages WHERE id = ?")
            .bind(msg_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(row.get::<i64, _>(0), alert_id);
        assert_eq!(row.get::<String, _>(1), "user");
        assert_eq!(row.get::<String, _>(2), "Hello");
    }

    #[tokio::test]
    async fn test_cascade_delete() {
        let pool = create_test_db().await.unwrap();

        // Create alert
        let alert_id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO alerts (alert_data, initial_response, kind, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(r#"{"message":"test"}"#)
        .bind("response")
        .bind(AlertKind::BgpAlerter.as_str())
        .bind("2025-01-15T10:30:00Z")
        .bind("2025-01-15T10:30:00Z")
        .fetch_one(&pool)
        .await
        .unwrap();

        // Create chat messages
        sqlx::query(
            r#"
            INSERT INTO chat_messages (alert_id, role, content, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(alert_id)
        .bind("user")
        .bind("Message 1")
        .bind("2025-01-15T10:30:00Z")
        .execute(&pool)
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
        .bind("Response 1")
        .bind("2025-01-15T10:30:00Z")
        .execute(&pool)
        .await
        .unwrap();

        // Verify messages exist
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM chat_messages WHERE alert_id = ?")
                .bind(alert_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 2);

        // Delete alert
        sqlx::query("DELETE FROM alerts WHERE id = ?")
            .bind(alert_id)
            .execute(&pool)
            .await
            .unwrap();

        // Verify messages are cascade deleted
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM chat_messages WHERE alert_id = ?")
                .bind(alert_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 0);
    }
}
