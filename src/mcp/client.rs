//! MCP client
//!
//! This module provides the MCP client for connecting to MCP servers.

use crate::{error::HarnessError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// MCP client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientConfig {
    /// Server name
    pub name: String,

    /// Server type (stdio, sse, http)
    pub server_type: McpServerType,

    /// Command for stdio servers
    pub command: Option<String>,

    /// Arguments for stdio servers
    pub args: Option<Vec<String>>,

    /// Environment variables for stdio servers
    pub env: Option<HashMap<String, String>>,

    /// URL for SSE/HTTP servers
    pub url: Option<String>,

    /// Headers for HTTP requests
    pub headers: Option<HashMap<String, String>>,

    /// OAuth configuration
    pub oauth: Option<McpOAuthConfig>,
}

/// MCP server type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum McpServerType {
    /// Standard input/output
    Stdio,
    /// Server-Sent Events
    Sse,
    /// HTTP/HTTPS
    Http,
}

/// OAuth configuration for MCP servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpOAuthConfig {
    /// OAuth token endpoint
    pub token_endpoint: String,

    /// Client ID
    pub client_id: String,

    /// Client secret
    pub client_secret: String,

    /// Grant type (client_credentials, refresh_token)
    pub grant_type: String,

    /// Refresh token (for refresh_token grant type)
    pub refresh_token: Option<String>,
}

/// MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    /// Tool name
    pub name: String,

    /// Tool description
    pub description: String,

    /// Input schema (JSON Schema)
    pub input_schema: serde_json::Value,

    /// Server name
    pub server_name: String,
}

/// MCP tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    /// Tool result content
    pub content: Vec<McpContent>,

    /// Whether the call was successful
    pub is_error: bool,
}

/// MCP content block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpContent {
    /// Text content
    #[serde(rename = "text")]
    Text { text: String },

    /// Image content
    #[serde(rename = "image")]
    Image {
        data: String,
        mime_type: String,
    },

    /// Resource content
    #[serde(rename = "resource")]
    Resource {
        uri: String,
        mime_type: Option<String>,
        text: Option<String>,
        blob: Option<String>,
    },
}

/// MCP client for connecting to MCP servers
pub struct McpClient {
    /// Client configuration
    config: McpClientConfig,

    /// Available tools
    tools: Arc<RwLock<Vec<McpToolDefinition>>>,

    /// OAuth token (if using OAuth)
    oauth_token: Arc<RwLock<Option<String>>>,
}

impl McpClient {
    /// Create a new MCP client
    pub fn new(config: McpClientConfig) -> Self {
        Self {
            config,
            tools: Arc::new(RwLock::new(Vec::new())),
            oauth_token: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the client configuration
    pub fn config(&self) -> &McpClientConfig {
        &self.config
    }

    /// Get available tools
    pub async fn list_tools(&self) -> Result<Vec<McpToolDefinition>> {
        let tools = self.tools.read().await;
        Ok(tools.clone())
    }

    /// Call an MCP tool
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<McpToolResult> {
        // Check if tool exists
        let tools = self.tools.read().await;
        let tool = tools
            .iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| {
                HarnessError::other(format!("MCP tool not found: {}", tool_name))
            })?;
        drop(tools);

        // Execute the tool call
        match &self.config.server_type {
            McpServerType::Stdio => {
                self.call_tool_stdio(tool_name, arguments).await
            }
            McpServerType::Sse => {
                self.call_tool_sse(tool_name, arguments).await
            }
            McpServerType::Http => {
                self.call_tool_http(tool_name, arguments).await
            }
        }
    }

    /// Refresh OAuth token if needed
    pub async fn refresh_token(&self) -> Result<()> {
        let oauth = self
            .config
            .oauth
            .as_ref()
            .ok_or_else(|| HarnessError::other("No OAuth configuration found"))?;

        // TODO: Implement actual OAuth token refresh
        // For now, just store a placeholder token
        let mut token = self.oauth_token.write().await;
        *token = Some("placeholder_token".to_string());

        Ok(())
    }

    /// Call tool via stdio
    async fn call_tool_stdio(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<McpToolResult> {
        // TODO: Implement stdio tool calling
        // This would involve:
        // 1. Spawn the configured command
        // 2. Send JSON-RPC requests
        // 3. Read responses

        Err(HarnessError::other("Stdio MCP not yet implemented"))
    }

    /// Call tool via SSE
    async fn call_tool_sse(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<McpToolResult> {
        // TODO: Implement SSE tool calling
        Err(HarnessError::other("SSE MCP not yet implemented"))
    }

    /// Call tool via HTTP
    async fn call_tool_http(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<McpToolResult> {
        // TODO: Implement HTTP tool calling with OAuth
        Err(HarnessError::other("HTTP MCP not yet implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_server_type() {
        let stdio = McpServerType::Stdio;
        assert_eq!(stdio, McpServerType::Stdio);

        let sse = McpServerType::Sse;
        assert_eq!(sse, McpServerType::Sse);

        let http = McpServerType::Http;
        assert_eq!(http, McpServerType::Http);
    }

    #[test]
    fn test_mcp_client_config() {
        let config = McpClientConfig {
            name: "test-server".to_string(),
            server_type: McpServerType::Stdio,
            command: Some("node".to_string()),
            args: Some(vec!["server.js".to_string()]),
            env: None,
            url: None,
            headers: None,
            oauth: None,
        };

        assert_eq!(config.name, "test-server");
        assert_eq!(config.server_type, McpServerType::Stdio);
        assert_eq!(config.command, Some("node".to_string()));
    }

    #[tokio::test]
    async fn test_mcp_client_new() {
        let config = McpClientConfig {
            name: "test-server".to_string(),
            server_type: McpServerType::Stdio,
            command: Some("node".to_string()),
            args: Some(vec!["server.js".to_string()]),
            env: None,
            url: None,
            headers: None,
            oauth: None,
        };

        let client = McpClient::new(config);
        assert_eq!(client.config().name, "test-server");

        // List tools should return empty initially
        let tools = client.list_tools().await.unwrap();
        assert!(tools.is_empty());
    }
}
