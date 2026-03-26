//! Read file tool
//!
//! This tool reads the contents of a file.

use crate::{
    error::HarnessError,
    tools::base::{Tool, ToolArgs, ToolContext, ToolOutput, ToolSchema},
};
use async_trait::async_trait;
use serde_json::json;

/// File reading tool
pub struct ReadFileTool {
    schema: ToolSchema,
}

impl ReadFileTool {
    /// Create a new read_file tool
    pub fn new() -> Self {
        let schema = ToolSchema::new(
            "read_file",
            "Read the contents of a file",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file to read"
                    },
                    "start_line": {
                        "type": "number",
                        "description": "Starting line number (1-indexed, optional)"
                    },
                    "end_line": {
                        "type": "number",
                        "description": "Ending line number (1-indexed, optional)"
                    }
                },
                "required": ["path"]
            }),
        )
        .with_group("sandbox");

        Self { schema }
    }
}

impl Default for ReadFileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn execute(&self, args: &ToolArgs, _context: &ToolContext) -> crate::tools::base::ToolResult {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| HarnessError::other("Missing 'path' argument"))?;

        let start_line = args.get("start_line").and_then(|v| v.as_u64()).map(|n| n as usize);
        let end_line = args.get("end_line").and_then(|v| v.as_u64()).map(|n| n as usize);

        // Check if file exists
        if !std::path::Path::new(path).exists() {
            return Ok(ToolOutput::error(format!("File not found: {}", path)));
        }

        // Read file contents
        let content = std::fs::read_to_string(path).map_err(|e| {
            HarnessError::other(format!("Failed to read file {}: {}", path, e))
        })?;

        // Apply line range if specified
        let result = if let Some(start) = start_line {
            let lines: Vec<&str> = content.lines().collect();
            let end = end_line.unwrap_or(lines.len());

            if start > lines.len() || start < 1 {
                return Ok(ToolOutput::error(format!(
                    "Invalid start_line: {} (file has {} lines)",
                    start,
                    lines.len()
                )));
            }

            let start_idx = start - 1;
            let end_idx = end.min(lines.len());

            lines[start_idx..end_idx]
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{}: {}", start + i, line))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            content
        };

        Ok(ToolOutput::text(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_file_tool_schema() {
        let tool = ReadFileTool::new();
        assert_eq!(tool.schema().name, "read_file");
        assert_eq!(tool.schema().group.as_deref(), Some("sandbox"));
    }
}
