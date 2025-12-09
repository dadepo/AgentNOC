use crate::database::{db, models};
use crate::mcp_clients;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::alerts::http::server::AppState;

use models::{CreateMcpServer, McpServer, UpdateMcpServer};

/// List all MCP servers
#[derive(Deserialize, ToSchema, IntoParams)]
pub struct ListMcpServersQuery {
    pub kind: Option<String>,
}

#[derive(IntoParams)]
pub struct McpServerId {
    /// MCP Server ID
    #[allow(dead_code)]
    pub id: i64,
}

/// List all MCP servers
#[utoipa::path(
    get,
    path = "/api/mcps",
    params(ListMcpServersQuery),
    responses(
        (status = 200, description = "List of MCP servers", body = Vec<McpServer>),
        (status = 500, description = "Internal server error")
    ),
    tag = "mcp"
)]
pub async fn list_mcp_servers(
    State(state): State<AppState>,
    Query(query): Query<ListMcpServersQuery>,
) -> Result<Json<Vec<models::McpServer>>, StatusCode> {
    let kind = query.kind.as_deref();
    let servers = db::get_all_mcp_servers(&state.db_pool, kind)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(servers))
}

/// Get a single MCP server by ID
#[utoipa::path(
    get,
    path = "/api/mcps/{id}",
    params(McpServerId),
    responses(
        (status = 200, description = "MCP server found", body = McpServer),
        (status = 404, description = "MCP server not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "mcp"
)]
pub async fn get_mcp_server(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<models::McpServer>, StatusCode> {
    let server = db::get_mcp_server_by_id(&state.db_pool, id)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match server {
        Some(s) => Ok(Json(s)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Create a new MCP server
#[utoipa::path(
    post,
    path = "/api/mcps",
    request_body = CreateMcpServer,
    responses(
        (status = 201, description = "MCP server created successfully", body = McpServer),
        (status = 400, description = "Bad request - validation error", body = serde_json::Value),
        (status = 409, description = "Conflict - server with this name already exists", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value)
    ),
    tag = "mcp"
)]
pub async fn create_mcp_server(
    State(state): State<AppState>,
    Json(payload): Json<models::CreateMcpServer>,
) -> Result<(StatusCode, Json<models::McpServer>), (StatusCode, Json<serde_json::Value>)> {
    // Validate the payload
    if let Err(e) = payload.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ));
    }

    let server = db::create_mcp_server(&state.db_pool, &payload)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            // Check if it's a unique constraint violation
            let error_msg = e.to_string();
            if error_msg.contains("UNIQUE constraint") {
                (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({ "error": "A server with this name already exists" })),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": "Failed to create server" })),
                )
            }
        })?;

    Ok((StatusCode::CREATED, Json(server)))
}

/// Update an existing MCP server
#[utoipa::path(
    put,
    path = "/api/mcps/{id}",
    params(McpServerId),
    request_body = UpdateMcpServer,
    responses(
        (status = 200, description = "MCP server updated successfully", body = McpServer),
        (status = 404, description = "MCP server not found", body = serde_json::Value),
        (status = 409, description = "Conflict - server with this name already exists", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value)
    ),
    tag = "mcp"
)]
pub async fn update_mcp_server(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<models::UpdateMcpServer>,
) -> Result<Json<models::McpServer>, (StatusCode, Json<serde_json::Value>)> {
    let server = db::update_mcp_server(&state.db_pool, id, &payload)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            let error_msg = e.to_string();
            if error_msg.contains("UNIQUE constraint") {
                (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({ "error": "A server with this name already exists" })),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": "Failed to update server" })),
                )
            }
        })?;

    match server {
        Some(s) => Ok(Json(s)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Server not found" })),
        )),
    }
}

/// Delete an MCP server
#[utoipa::path(
    delete,
    path = "/api/mcps/{id}",
    params(McpServerId),
    responses(
        (status = 204, description = "MCP server deleted successfully"),
        (status = 404, description = "MCP server not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "mcp"
)]
pub async fn delete_mcp_server(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    let deleted = db::delete_mcp_server(&state.db_pool, id)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Test connection response
#[derive(Serialize, ToSchema)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub tool_count: Option<usize>,
    pub error: Option<String>,
}

/// Test connection to an MCP server
#[utoipa::path(
    post,
    path = "/api/mcps/{id}/test",
    params(McpServerId),
    responses(
        (status = 200, description = "Connection test result", body = TestConnectionResponse),
        (status = 404, description = "MCP server not found", body = TestConnectionResponse),
        (status = 500, description = "Internal server error", body = TestConnectionResponse)
    ),
    tag = "mcp"
)]
pub async fn test_mcp_server(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<TestConnectionResponse>, (StatusCode, Json<TestConnectionResponse>)> {
    // First get the server
    let server = db::get_mcp_server_by_id(&state.db_pool, id)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TestConnectionResponse {
                    success: false,
                    tool_count: None,
                    error: Some("Database error".to_string()),
                }),
            )
        })?;

    let server = match server {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(TestConnectionResponse {
                    success: false,
                    tool_count: None,
                    error: Some("Server not found".to_string()),
                }),
            ));
        }
    };

    // Try to connect
    match mcp_clients::test_connection(&server).await {
        Ok(tool_count) => Ok(Json(TestConnectionResponse {
            success: true,
            tool_count: Some(tool_count),
            error: None,
        })),
        Err(e) => {
            tracing::warn!(
                "Connection test failed for server {}: {:?}",
                server.name(),
                e
            );
            Ok(Json(TestConnectionResponse {
                success: false,
                tool_count: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct EnableNativeRequest {
    pub enabled: bool,
}

/// Enable or disable native MCP servers
#[utoipa::path(
    post,
    path = "/api/mcps/enable-native",
    request_body = EnableNativeRequest,
    responses(
        (status = 204, description = "Native MCP servers enabled/disabled successfully"),
        (status = 500, description = "Internal server error", body = serde_json::Value)
    ),
    tag = "mcp"
)]
pub async fn enable_native_mcp_servers(
    State(state): State<AppState>,
    Json(payload): Json<EnableNativeRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    db::enable_native_mcp_servers(&state.db_pool, payload.enabled)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to enable/disable native MCP servers" })),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}
