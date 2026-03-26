//! MCP tool adapter
//!
//! This module provides an adapter for integrating MCP tools with the
//! agent-harness tool system.

use crate::{
    error::{ToolError, HarnessError, Result},
    mcp::client::{McpClient, McpToolDefinition},
    tools::{base::*, ToolContext},
};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Adapter for MCP tools to work with the agent-harness Tool interface
pub struct McpToolAdapter {
    /// Tool definition
    definition: McpToolDefinition,

    /// MCP client for executing the tool
    client: Arc<McpClient>,

    /// Tool schema
    schema: ToolSchema,
}

impl McpToolAdapter {
    /// Create a new MCP tool adapter
    pub fn new(definition: McpToolDefinition, client: Arc<McpClient>) -> Self {
        // Convert MCP tool definition to our schema format
        let schema = ToolSchema::new(
            &definition.name,
            &definition.description,
            definition.input_schema.clone(),
        )
        .with_group(format!("mcp:{}", definition.server_name));

        Self {
            definition,
            client,
            schema,
        }
    }

    /// Create multiple tool adapters from a client
    pub async fn from_client(
        client: Arc<McpClient>,
    ) -> Result<Vec<Arc<dyn Tool>>> {
        let tools = client.list_tools().await?;
        let mut adapters = Vec::new();

        for tool_def in tools {
            let adapter = Self::new(tool_def, client.clone());
            adapters.push(Arc::new(adapter) as Arc<dyn Tool>);
        }

        Ok(adapters)
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn execute(&self, args: &ToolArgs, _context: &ToolContext) -> ToolResult {
        // Call the MCP tool
        let result = self
            .client
            .call_tool(&self.definition.name, args)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("MCP tool call failed: {}", e)))?;

        // Check if the tool returned an error
        if result.is_error {
            let error_msg = result
                .content
                .iter()
                .filter_map(|c| match c {
                    crate::mcp::client::McpContent::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            return Ok(ToolOutput::error(error_msg));
        }

        // Convert MCP content to tool output
        let output_parts: Vec<String> = result
            .content
            .iter()
            .map(|c| match c {
                crate::mcp::client::McpContent::Text { text } => text.clone(),
                crate::mcp::client::McpContent::Image { data, .. } => {
                    format!("[Image: {} bytes]", data.len())
                }
                crate::mcp::client::McpContent::Resource {
                    uri, text, blob, ..
                } => {
                    if let Some(t) = text {
                        t.clone()
                    } else if let Some(b) = blob {
                        format!("[Binary resource: {} bytes]", b.len())
                    } else {
                        uri.clone()
                    }
                }
            })
            .collect();

        Ok(ToolOutput::text(output_parts.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::client::{McpClient, McpClientConfig, McpServerType};

    #[test]
    fn test_mcp_tool_adapter_schema() {
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

        let client = Arc::new(McpClient::new(config));

        let definition = McpToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "arg1": {
                        "type": "string",
                        "description": "First argument"
                    }
                }
            }),
            server_name: "test-server".to_string(),
        };

        let adapter = McpToolAdapter::new(definition, client);
        assert_eq!(adapter.schema().name, "test_tool");
        assert_eq!(adapter.schema().description, "A test tool");
        assert_eq!(adapter.schema().group, Some("mcp:test-server".to_string()));
    }
}
