use color_eyre::Result;
use sqlx::SqlitePool;
use std::sync::Arc;

pub async fn init_database() -> Result<Arc<SqlitePool>> {
    // Database file location
    let db_path = "noc_agent.db";

    // Create connection pool
    let pool = SqlitePool::connect(&format!("sqlite://{db_path}?mode=rwc")).await?;

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
    .await?;

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
    .await?;

    // Create indexes for performance
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_chat_messages_alert_id ON chat_messages(alert_id)
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_chat_messages_created_at ON chat_messages(created_at)
        "#,
    )
    .execute(&pool)
    .await?;

    tracing::info!("Database initialized at {}", db_path);
    Ok(Arc::new(pool))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row;

    async fn create_test_db() -> Result<SqlitePool> {
        // Use in-memory database for tests
        let pool = SqlitePool::connect("sqlite::memory:").await?;

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
        .await?;

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
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_chat_messages_alert_id ON chat_messages(alert_id)
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_chat_messages_created_at ON chat_messages(created_at)
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(pool)
    }

    #[tokio::test]
    async fn test_database_initialization() {
        let pool = create_test_db().await.unwrap();

        // Verify tables exist
        let table_check = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('alerts', 'chat_messages')"
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(table_check.len(), 2);
    }

    #[tokio::test]
    async fn test_insert_alert() {
        let pool = create_test_db().await.unwrap();

        let alert_data = r#"{"message":"test"}"#;
        let response = "Test response";
        let timestamp = "2025-01-15T10:30:00Z";

        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO alerts (alert_data, initial_response, created_at, updated_at)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(alert_data)
        .bind(response)
        .bind(timestamp)
        .bind(timestamp)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(id, 1);

        // Verify we can retrieve it
        let row = sqlx::query("SELECT id, alert_data, initial_response FROM alerts WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(row.get::<i64, _>(0), id);
        assert_eq!(row.get::<String, _>(1), alert_data);
        assert_eq!(row.get::<String, _>(2), response);
    }

    #[tokio::test]
    async fn test_insert_chat_message() {
        let pool = create_test_db().await.unwrap();

        // First create an alert
        let alert_id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO alerts (alert_data, initial_response, created_at, updated_at)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(r#"{"message":"test"}"#)
        .bind("response")
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
            INSERT INTO alerts (alert_data, initial_response, created_at, updated_at)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(r#"{"message":"test"}"#)
        .bind("response")
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
