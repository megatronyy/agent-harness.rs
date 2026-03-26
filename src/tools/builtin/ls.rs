//! LS tool
//!
//! This tool lists directory contents.

use crate::{
    tools::base::{Tool, ToolArgs, ToolContext, ToolOutput, ToolSchema},
    HarnessError,
};
use async_trait::async_trait;
use serde_json::json;
use std::path::Path;

/// Directory listing tool
pub struct LsTool {
    schema: ToolSchema,
}

impl LsTool {
    /// Create a new ls tool
    pub fn new() -> Self {
        let schema = ToolSchema::new(
            "ls",
            "List the contents of a directory",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The directory path to list (default: current directory)"
                    },
                    "depth": {
                        "type": "number",
                        "description": "Maximum depth to display (1-2, default: 2)"
                    }
                }
            }),
        )
        .with_group("sandbox");

        Self { schema }
    }
}

impl Default for LsTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for LsTool {
    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn execute(&self, args: &ToolArgs, _context: &ToolContext) -> crate::tools::base::ToolResult {
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let depth = args
            .get("depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as u32;

        let path = Path::new(path_str);

        if !path.exists() {
            return Ok(ToolOutput::error(format!("Path not found: {}", path_str)));
        }

        let mut output = Vec::new();
        output.push(format!("{}:\n", path_str));

        list_directory(path, depth, 0, &mut output)?;

        Ok(ToolOutput::text(output.join("\n")))
    }
}

/// Recursively list directory contents
fn list_directory(
    path: &Path,
    max_depth: u32,
    current_depth: u32,
    output: &mut Vec<String>,
) -> std::result::Result<(), std::io::Error> {
    let entries = std::fs::read_dir(path)?;

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        if entry.file_type()?.is_dir() {
            dirs.push(name);
        } else {
            files.push(name);
        }
    }

    // Sort alphabetically
    dirs.sort();
    files.sort();

    // Print directories first
    for dir in dirs {
        output.push(format!("  {}/", dir));

        // Recursively list subdirectories if depth allows
        if current_depth < max_depth {
            let subdir = path.join(&dir);
            if let Ok(()) = list_directory(&subdir, max_depth, current_depth + 1, output) {
                // Continue
            }
        }
    }

    // Print files
    for file in files {
        output.push(format!("  {}", file));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ls_tool_schema() {
        let tool = LsTool::new();
        assert_eq!(tool.schema().name, "ls");
        assert_eq!(tool.schema().group.as_deref(), Some("sandbox"));
    }

    #[tokio::test]
    async fn test_ls_tool_execute_current_dir() {
        let tool = LsTool::new();
        let context = ToolContext::new("test-thread");

        let args = json!({});
        let result = tool.execute(&args, &context).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.to_string().contains("."));
    }
}
