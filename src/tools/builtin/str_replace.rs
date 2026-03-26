//! String replace tool
//!
//! This tool performs string replacements in files.

use crate::{
    error::HarnessError,
    tools::base::{Tool, ToolArgs, ToolContext, ToolOutput, ToolSchema},
};
use async_trait::async_trait;
use serde_json::json;

/// String replacement tool
pub struct StrReplaceTool {
    schema: ToolSchema,
}

impl StrReplaceTool {
    /// Create a new str_replace tool
    pub fn new() -> Self {
        let schema = ToolSchema::new(
            "str_replace",
            "Replace occurrences of a string in a file",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file"
                    },
                    "old": {
                        "type": "string",
                        "description": "The string to replace"
                    },
                    "new": {
                        "type": "string",
                        "description": "The replacement string"
                    },
                    "replace_all": {
                        "type": "boolean",
                        "description": "Replace all occurrences instead of just the first (default: false)"
                    }
                },
                "required": ["path", "old", "new"]
            }),
        )
        .with_group("sandbox");

        Self { schema }
    }
}

impl Default for StrReplaceTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for StrReplaceTool {
    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn execute(&self, args: &ToolArgs, _context: &ToolContext) -> crate::tools::base::ToolResult {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| HarnessError::other("Missing 'path' argument"))?;

        let old_str = args
            .get("old")
            .and_then(|v| v.as_str())
            .ok_or_else(|| HarnessError::other("Missing 'old' argument"))?;

        let new_str = args
            .get("new")
            .and_then(|v| v.as_str())
            .ok_or_else(|| HarnessError::other("Missing 'new' argument"))?;

        let replace_all = args
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Check if file exists
        if !std::path::Path::new(path).exists() {
            return Ok(ToolOutput::error(format!("File not found: {}", path)));
        }

        // Read file contents
        let content = std::fs::read_to_string(path).map_err(|e| {
            HarnessError::other(format!("Failed to read file {}: {}", path, e))
        })?;

        // Perform replacement
        let new_content = if replace_all {
            content.replace(old_str, new_str)
        } else {
            content.replacen(old_str, new_str, 1)
        };

        // Check if replacement was made
        if new_content == content {
            return Ok(ToolOutput::error(format!(
                "String '{}' not found in file",
                old_str
            )));
        }

        // Write back to file
        std::fs::write(path, new_content).map_err(|e| {
            HarnessError::other(format!("Failed to write to file {}: {}", path, e))
        })?;

        Ok(ToolOutput::text(format!(
            "Successfully replaced '{}' in file: {}",
            old_str, path
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_replace_tool_schema() {
        let tool = StrReplaceTool::new();
        assert_eq!(tool.schema().name, "str_replace");
        assert_eq!(tool.schema().group.as_deref(), Some("sandbox"));
    }
}
