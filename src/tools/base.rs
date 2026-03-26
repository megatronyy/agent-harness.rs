//! Base tool abstractions
//!
//! This module defines the core Tool trait and associated types
//! for implementing tools in the agent-harness system.

use crate::{HarnessError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool input arguments
pub type ToolArgs = serde_json::Value;

/// Tool output result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolOutput {
    /// Text output
    Text(String),
    /// JSON output
    Json(serde_json::Value),
    /// Error output
    Error { message: String },
    /// Mixed output with multiple parts
    Mixed { parts: Vec<OutputPart> },
}

/// A part of mixed output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPart {
    /// Part type (text, image, etc.)
    #[serde(rename = "type")]
    pub part_type: String,
    /// Part content
    pub content: serde_json::Value,
}

impl ToolOutput {
    /// Create text output
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// Create JSON output
    pub fn json(value: serde_json::Value) -> Self {
        Self::Json(value)
    }

    /// Create error output
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
        }
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Json(v) => v.to_string(),
            Self::Error { message } => format!("Error: {}", message),
            Self::Mixed { parts } => {
                parts
                    .iter()
                    .filter_map(|p| {
                        if p.part_type == "text" {
                            p.content.as_str().map(|s| s.to_string())
                        } else {
                            Some(format!("[{}]", p.part_type))
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        }
    }
}

impl From<String> for ToolOutput {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<&str> for ToolOutput {
    fn from(s: &str) -> Self {
        Self::Text(s.to_string())
    }
}

impl From<serde_json::Value> for ToolOutput {
    fn from(v: serde_json::Value) -> Self {
        Self::Json(v)
    }
}

/// Tool execution result
pub type ToolResult = std::result::Result<ToolOutput, HarnessError>;

/// Tool schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Tool name
    pub name: String,

    /// Tool description
    pub description: String,

    /// Input parameters schema (JSON Schema)
    pub input_schema: serde_json::Value,

    /// Tool group/category
    pub group: Option<String>,

    /// Whether tool is async
    pub is_async: bool,
}

impl ToolSchema {
    /// Create a new tool schema
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            group: None,
            is_async: true,
        }
    }

    /// Set the tool group
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// Set whether the tool is async
    pub fn with_is_async(mut self, is_async: bool) -> Self {
        self.is_async = is_async;
        self
    }
}

/// Tool trait for implementing tools
///
/// All tools must implement this trait to be used with the agent-harness system.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool schema
    fn schema(&self) -> &ToolSchema;

    /// Execute the tool with given arguments
    async fn execute(&self, args: &ToolArgs, context: &ToolContext) -> ToolResult;

    /// Validate tool arguments before execution
    fn validate_args(&self, args: &ToolArgs) -> Result<()> {
        // Basic JSON schema validation
        if let Err(e) = self.validate_json_schema(args) {
            return Err(HarnessError::other(format!(
                "Invalid arguments for tool {}: {}",
                self.schema().name,
                e
            )));
        }
        Ok(())
    }

    /// Validate arguments against JSON schema
    fn validate_json_schema(&self, args: &ToolArgs) -> Result<()> {
        let schema = &self.schema().input_schema;

        // Check if args is an object when schema expects object
        if let Some(obj_schema) = schema.get("type") {
            if obj_schema.as_str() == Some("object") && !args.is_object() {
                return Err(HarnessError::other(format!(
                    "Expected object arguments, got: {}",
                    args
                )));
            }
        }

        // Check required fields
        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            if let Some(obj) = args.as_object() {
                for field in required {
                    if let Some(field_name) = field.as_str() {
                        if !obj.contains_key(field_name) {
                            return Err(HarnessError::other(format!(
                                "Missing required field: {}",
                                field_name
                            )));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Tool execution context
///
/// Provides runtime information to tools during execution.
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Thread ID
    pub thread_id: String,

    /// Sandbox ID (if available)
    pub sandbox_id: Option<String>,

    /// Working directory
    pub working_dir: Option<String>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ToolContext {
    /// Create a new tool context
    pub fn new(thread_id: impl Into<String>) -> Self {
        Self {
            thread_id: thread_id.into(),
            sandbox_id: None,
            working_dir: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the sandbox ID
    pub fn with_sandbox_id(mut self, sandbox_id: impl Into<String>) -> Self {
        self.sandbox_id = Some(sandbox_id.into());
        self
    }

    /// Set the working directory
    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_output_text() {
        let output = ToolOutput::text("Hello, world!");
        assert_eq!(output.to_string(), "Hello, world!");
    }

    #[test]
    fn test_tool_output_error() {
        let output = ToolOutput::error("Something went wrong");
        assert!(output.to_string().contains("Error"));
        assert!(output.to_string().contains("Something went wrong"));
    }

    #[test]
    fn test_tool_output_from_string() {
        let output: ToolOutput = "Test".to_string().into();
        assert_eq!(output.to_string(), "Test");
    }

    #[test]
    fn test_tool_schema_builder() {
        let schema = ToolSchema::new(
            "test_tool",
            "A test tool",
            serde_json::json!({"type": "object"}),
        )
        .with_group("test")
        .with_is_async(false);

        assert_eq!(schema.name, "test_tool");
        assert_eq!(schema.group.as_deref(), Some("test"));
        assert!(!schema.is_async);
    }

    #[test]
    fn test_tool_context_builder() {
        let ctx = ToolContext::new("thread-123")
            .with_sandbox_id("sandbox-456")
            .with_working_dir("/workspace")
            .with_metadata("key", "value");

        assert_eq!(ctx.thread_id, "thread-123");
        assert_eq!(ctx.sandbox_id.as_deref(), Some("sandbox-456"));
        assert_eq!(ctx.working_dir.as_deref(), Some("/workspace"));
        assert_eq!(ctx.metadata.get("key"), Some(&"value".to_string()));
    }
}
