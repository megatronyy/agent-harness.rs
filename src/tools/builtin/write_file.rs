//! Write file tool
//!
//! This tool writes content to a file.

use crate::{
    error::HarnessError,
    tools::base::{Tool, ToolArgs, ToolContext, ToolOutput, ToolSchema},
};
use async_trait::async_trait;
use serde_json::json;
use std::path::Path;

/// File writing tool
pub struct WriteFileTool {
    schema: ToolSchema,
}

impl WriteFileTool {
    /// Create a new write_file tool
    pub fn new() -> Self {
        let schema = ToolSchema::new(
            "write_file",
            "Write content to a file, creating parent directories if needed",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file"
                    },
                    "append": {
                        "type": "boolean",
                        "description": "Whether to append to the file instead of overwriting (default: false)"
                    }
                },
                "required": ["path", "content"]
            }),
        )
        .with_group("sandbox");

        Self { schema }
    }
}

impl Default for WriteFileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn execute(&self, args: &ToolArgs, _context: &ToolContext) -> crate::tools::base::ToolResult {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| HarnessError::other("Missing 'path' argument"))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| HarnessError::other("Missing 'content' argument"))?;

        let append = args
            .get("append")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Create parent directories if they don't exist
        if let Some(parent) = Path::new(path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    HarnessError::other(format!("Failed to create directory {}: {}", parent.display(), e))
                })?;
            }
        }

        // Write or append to file
        if append {
            std::fs::write(path, content).map_err(|e| {
                HarnessError::other(format!("Failed to write to file {}: {}", path, e))
            })?;
        } else {
            std::fs::write(path, content).map_err(|e| {
                HarnessError::other(format!("Failed to write to file {}: {}", path, e))
            })?;
        }

        Ok(ToolOutput::text(format!("Successfully wrote to file: {}", path)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_file_tool_schema() {
        let tool = WriteFileTool::new();
        assert_eq!(tool.schema().name, "write_file");
        assert_eq!(tool.schema().group.as_deref(), Some("sandbox"));
    }
}
