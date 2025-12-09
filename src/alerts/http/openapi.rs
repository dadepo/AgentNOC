use utoipa::OpenApi;

use crate::agents::health::HealthStatus;
use crate::alerts::http::routes::alerts::ChatRequest;
use crate::alerts::http::routes::mcp::{
    EnableNativeRequest, ListMcpServersQuery, TestConnectionResponse,
};
use crate::alerts::http::server::{BGPAlerterAlert, Details, SseEvent};
use crate::database::models::{
    Alert, AlertKind, ChatMessage, CreateMcpServer, McpServer, McpServerDetails, UpdateMcpServer,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::alerts::http::routes::health_check,
        crate::alerts::http::routes::message_stream,
        crate::alerts::http::routes::alerts::list_alerts,
        crate::alerts::http::routes::alerts::get_alert,
        crate::alerts::http::routes::alerts::process_alert,
        crate::alerts::http::routes::alerts::delete_alert,
        crate::alerts::http::routes::alerts::chat_with_alert,
        crate::alerts::http::routes::mcp::list_mcp_servers,
        crate::alerts::http::routes::mcp::get_mcp_server,
        crate::alerts::http::routes::mcp::create_mcp_server,
        crate::alerts::http::routes::mcp::update_mcp_server,
        crate::alerts::http::routes::mcp::delete_mcp_server,
        crate::alerts::http::routes::mcp::test_mcp_server,
        crate::alerts::http::routes::mcp::enable_native_mcp_servers,
    ),
    components(schemas(
        HealthStatus,
        BGPAlerterAlert,
        Details,
        Alert,
        AlertKind,
        ChatMessage,
        ChatRequest,
        McpServer,
        McpServerDetails,
        CreateMcpServer,
        UpdateMcpServer,
        ListMcpServersQuery,
        TestConnectionResponse,
        EnableNativeRequest,
        SseEvent,
    )),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "alerts", description = "Alert management endpoints"),
        (name = "mcp", description = "MCP server management endpoints"),
        (name = "streaming", description = "Server-sent events streaming"),
    ),
    info(
        title = "Agent NOC API",
        description = "API for managing AgentNOC",
        version = "0.1.0"
    ),
    servers(
        (url = "http://localhost:7654", description = "Local development server"),
    ),
)]
pub struct ApiDoc;
