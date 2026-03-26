//! View image tool
//!
//! This tool reads and displays images as base64.

use crate::{
    error::HarnessError,
    tools::base::{Tool, ToolArgs, ToolContext, ToolOutput, ToolSchema},
};
use async_trait::async_trait;
use serde_json::json;
use std::path::Path;

/// Image viewing tool
pub struct ViewImageTool {
    schema: ToolSchema,
}

impl ViewImageTool {
    /// Create a new view_image tool
    pub fn new() -> Self {
        let schema = ToolSchema::new(
            "view_image",
            "Read an image file and return it as base64 for vision models",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the image file"
                    }
                },
                "required": ["path"]
            }),
        )
        .with_group("vision");

        Self { schema }
    }
}

impl Default for ViewImageTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ViewImageTool {
    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn execute(&self, args: &ToolArgs, _context: &ToolContext) -> crate::tools::base::ToolResult {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| HarnessError::other("Missing 'path' argument"))?;

        let path_obj = Path::new(path);

        // Check if file exists
        if !path_obj.exists() {
            return Ok(ToolOutput::error(format!("Image file not found: {}", path)));
        }

        // Determine MIME type
        let mime_type = mime_guess::from_path(path)
            .first()
            .map(|m| m.to_string())
            .unwrap_or_else(|| {
                // Fallback based on extension
                match path_obj.extension().and_then(|e| e.to_str()) {
                    Some("png") => "image/png".to_string(),
                    Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
                    Some("gif") => "image/gif".to_string(),
                    Some("webp") => "image/webp".to_string(),
                    _ => "image/png".to_string(),
                }
            });

        // Read file and encode to base64
        let bytes = std::fs::read(path).map_err(|e| {
            HarnessError::other(format!("Failed to read image file {}: {}", path, e))
        })?;

        use base64::prelude::*;
        let base64_data = BASE64_STANDARD.encode(&bytes);

        Ok(ToolOutput::json(json!({
            "path": path,
            "mime_type": mime_type,
            "data": base64_data,
            "size": bytes.len()
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_image_tool_schema() {
        let tool = ViewImageTool::new();
        assert_eq!(tool.schema().name, "view_image");
        assert_eq!(tool.schema().group.as_deref(), Some("vision"));
    }
}
