use crate::database::models::CreateMcpServer;
use std::collections::HashMap;

/// Native MCP server definitions - hardcoded and updated with releases
pub fn get_native_mcp_servers() -> Vec<CreateMcpServer> {
    vec![
        CreateMcpServer::Http {
            name: "ripestat".to_string(),
            description: Some("RIPEstat MCP Server for BGP and routing information".to_string()),
            url: "https://mcp-ripestat.taihen.org/mcp".to_string(),
            enabled: true,
        },
        CreateMcpServer::Stdio {
            name: "whois".to_string(),
            description: Some("WHOIS MCP Server for domain and IP lookups".to_string()),
            command: "uvx".to_string(),
            args: vec![
                "--from".to_string(),
                "git+https://github.com/dadepo/whois-mcp.git".to_string(),
                "whois-mcp".to_string(),
            ],
            env: HashMap::new(),
            enabled: true,
        },
    ]
}
