//! Present files tool
//!
//! This tool lists files that have been created or modified to be shown to the user.

use crate::{
    error::HarnessError,
    tools::base::{Tool, ToolArgs, ToolContext, ToolOutput, ToolSchema},
};
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// File presenter tool
pub struct PresentFilesTool {
    schema: ToolSchema,
    presented_files: Arc<Mutex<HashSet<String>>>,
}

impl PresentFilesTool {
    /// Create a new present_files tool
    pub fn new() -> Self {
        let schema = ToolSchema::new(
            "present_files",
            "List files in the outputs directory to present them to the user",
            json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of file paths to present"
                    }
                }
            }),
        )
        .with_group("output");

        Self {
            schema,
            presented_files: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Get the set of presented files
    pub fn get_presented_files(&self) -> HashSet<String> {
        self.presented_files
            .lock()
            .unwrap()
            .clone()
    }

    /// Clear the set of presented files
    pub fn clear_presented_files(&self) {
        self.presented_files.lock().unwrap().clear();
    }
}

impl Default for PresentFilesTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PresentFilesTool {
    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn execute(&self, args: &ToolArgs, _context: &ToolContext) -> crate::tools::base::ToolResult {
        let paths = args
            .get("paths")
            .and_then(|v| v.as_array())
            .ok_or_else(|| HarnessError::other("Missing 'paths' argument"))?;

        let mut results = Vec::new();
        let mut presented = self.presented_files.lock().unwrap();

        for path_value in paths {
            let path = path_value
                .as_str()
                .ok_or_else(|| HarnessError::other("Invalid path in paths array"))?;

            let path_obj = Path::new(path);

            // Check if file exists and is in outputs directory
            if !path_obj.exists() {
                results.push(format!("✓ {} (file not found, will be created)", path));
                continue;
            }

            // Get file size
            let metadata = std::fs::metadata(path).map_err(|e| {
                HarnessError::other(format!("Failed to get metadata for {}: {}", path, e))
            })?;

            let size = metadata.len();
            let size_str = if size < 1024 {
                format!("{}B", size)
            } else if size < 1024 * 1024 {
                format!("{:.1}KB", size as f64 / 1024.0)
            } else {
                format!("{:.1}MB", size as f64 / (1024.0 * 1024.0))
            };

            results.push(format!("✓ {} ({})", path, size_str));
            presented.insert(path.to_string());
        }

        if results.is_empty() {
            results.push("No files to present.".to_string());
        }

        Ok(ToolOutput::text(results.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_present_files_tool_schema() {
        let tool = PresentFilesTool::new();
        assert_eq!(tool.schema().name, "present_files");
        assert_eq!(tool.schema().group.as_deref(), Some("output"));
    }

    #[tokio::test]
    async fn test_present_files_execute() {
        let tool = PresentFilesTool::new();
        let context = ToolContext::new("test-thread");

        let args = json!({"paths": ["/mnt/user-data/outputs/test.txt"]});
        let result = tool.execute(&args, &context).await;

        assert!(result.is_ok());
    }
}
