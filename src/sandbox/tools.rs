//! Sandbox tools
//!
//! This module provides tools that work with the sandbox system,
//! replacing the standalone tool implementations with sandbox-aware versions.

use crate::{
    error::ToolError,
    sandbox::Sandbox,
    tools::{base::*, ToolContext},
    Result,
};
use serde_json::json;
use std::sync::Arc;

/// Bash tool for executing commands in a sandbox
pub struct BashTool {
    /// Sandbox to execute commands in
    sandbox: Arc<dyn Sandbox>,
    /// Schema for this tool
    schema: ToolSchema,
}

impl BashTool {
    /// Create a new bash tool
    pub fn new(sandbox: Arc<dyn Sandbox>) -> Self {
        let schema = ToolSchema::new(
            "bash",
            "Execute bash commands in the sandbox environment. Use this to run shell commands, scripts, and system utilities.",
            json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The bash command to execute"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory relative to sandbox root"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 30)",
                        "default": 30
                    }
                },
                "required": ["command"]
            }),
        )
        .with_group("sandbox");

        Self { sandbox, schema }
    }
}

#[async_trait::async_trait]
impl Tool for BashTool {
    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn execute(&self, args: &ToolArgs, _context: &ToolContext) -> ToolResult {
        // Extract arguments
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'command' parameter".to_string()))?;

        let cwd = args.get("cwd").and_then(|v| v.as_str());

        let timeout_secs = args
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        // Execute command
        let result = self
            .sandbox
            .execute_command(command, cwd, Some(timeout_secs))
            .await
            .map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to execute command: {}", e))
            })?;

        // Build response
        let output = if result.timed_out {
            format!(
                "Command timed out after {} seconds\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
                timeout_secs, result.stdout, result.stderr
            )
        } else if result.is_success() {
            if result.stderr.is_empty() {
                result.stdout
            } else {
                format!("{}\n\nSTDERR:\n{}", result.stdout, result.stderr)
            }
        } else {
            format!(
                "Command failed with exit code {}\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
                result.exit_code, result.stdout, result.stderr
            )
        };

        Ok(ToolOutput::text(output))
    }
}

/// Read file tool
pub struct ReadFileTool {
    /// Sandbox to read files from
    sandbox: Arc<dyn Sandbox>,
    /// Schema for this tool
    schema: ToolSchema,
}

impl ReadFileTool {
    /// Create a new read file tool
    pub fn new(sandbox: Arc<dyn Sandbox>) -> Self {
        let schema = ToolSchema::new(
            "read_file",
            "Read the contents of a file in the sandbox. Supports reading a specific range of lines.",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file relative to sandbox root"
                    },
                    "start_line": {
                        "type": "integer",
                        "description": "Starting line number (1-indexed, inclusive)"
                    },
                    "end_line": {
                        "type": "integer",
                        "description": "Ending line number (1-indexed, inclusive)"
                    }
                },
                "required": ["path"]
            }),
        )
        .with_group("sandbox");

        Self { sandbox, schema }
    }
}

#[async_trait::async_trait]
impl Tool for ReadFileTool {
    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn execute(&self, args: &ToolArgs, _context: &ToolContext) -> ToolResult {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".to_string()))?;

        let start_line = args.get("start_line").and_then(|v| v.as_u64()).map(|v| v as usize);
        let end_line = args.get("end_line").and_then(|v| v.as_u64()).map(|v| v as usize);

        // Read file
        let result = self.sandbox.read_file(path).await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to read file: {}", e))
        })?;

        if !result.exists {
            return Ok(ToolOutput::text(format!("File not found: {}", path)));
        }

        // Extract line range if specified
        let content = if let (Some(start), Some(end)) = (start_line, end_line) {
            let lines: Vec<&str> = result.content.lines().collect();
            if start > lines.len() {
                return Ok(ToolOutput::text(format!(
                    "Start line {} exceeds file length {}",
                    start,
                    lines.len()
                )));
            }
            let end = end.min(lines.len());
            lines[start - 1..end].join("\n")
        } else if let Some(start) = start_line {
            let lines: Vec<&str> = result.content.lines().collect();
            if start > lines.len() {
                return Ok(ToolOutput::text(format!(
                    "Start line {} exceeds file length {}",
                    start,
                    lines.len()
                )));
            }
            lines[start - 1..].join("\n")
        } else {
            result.content
        };

        Ok(ToolOutput::text(content))
    }
}

/// Write file tool
pub struct WriteFileTool {
    /// Sandbox to write files to
    sandbox: Arc<dyn Sandbox>,
    /// Schema for this tool
    schema: ToolSchema,
}

impl WriteFileTool {
    /// Create a new write file tool
    pub fn new(sandbox: Arc<dyn Sandbox>) -> Self {
        let schema = ToolSchema::new(
            "write_file",
            "Write content to a file in the sandbox. Creates parent directories if they don't exist.",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file relative to sandbox root"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    },
                    "append": {
                        "type": "boolean",
                        "description": "Whether to append to existing file (default: false)",
                        "default": false
                    }
                },
                "required": ["path", "content"]
            }),
        )
        .with_group("sandbox");

        Self { sandbox, schema }
    }
}

#[async_trait::async_trait]
impl Tool for WriteFileTool {
    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn execute(&self, args: &ToolArgs, _context: &ToolContext) -> ToolResult {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".to_string()))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'content' parameter".to_string()))?;

        let append = args.get("append").and_then(|v| v.as_bool()).unwrap_or(false);

        self.sandbox
            .write_file(path, content, append)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;

        Ok(ToolOutput::text(format!(
            "Successfully wrote {} bytes to {}",
            content.len(),
            path
        )))
    }
}

/// String replace tool
pub struct StrReplaceTool {
    /// Sandbox to modify files in
    sandbox: Arc<dyn Sandbox>,
    /// Schema for this tool
    schema: ToolSchema,
}

impl StrReplaceTool {
    /// Create a new str_replace tool
    pub fn new(sandbox: Arc<dyn Sandbox>) -> Self {
        let schema = ToolSchema::new(
            "str_replace",
            "Replace a substring in a file with new content. Supports replacing all occurrences or just the first.",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file relative to sandbox root"
                    },
                    "old": {
                        "type": "string",
                        "description": "Substring to replace"
                    },
                    "new": {
                        "type": "string",
                        "description": "New content to replace with"
                    },
                    "replace_all": {
                        "type": "boolean",
                        "description": "Replace all occurrences (default: false)",
                        "default": false
                    }
                },
                "required": ["path", "old", "new"]
            }),
        )
        .with_group("sandbox");

        Self { sandbox, schema }
    }
}

#[async_trait::async_trait]
impl Tool for StrReplaceTool {
    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn execute(&self, args: &ToolArgs, _context: &ToolContext) -> ToolResult {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".to_string()))?;

        let old = args
            .get("old")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'old' parameter".to_string()))?;

        let new = args
            .get("new")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'new' parameter".to_string()))?;

        let replace_all = args.get("replace_all").and_then(|v| v.as_bool()).unwrap_or(false);

        // Read current file content
        let result = self.sandbox.read_file(path).await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to read file: {}", e))
        })?;

        if !result.exists {
            return Ok(ToolOutput::text(format!("File not found: {}", path)));
        }

        // Perform replacement
        let new_content = if replace_all {
            result.content.replace(old, new)
        } else {
            result.content.replacen(old, new, 1)
        };

        // Check if anything was replaced
        if new_content == result.content {
            return Ok(ToolOutput::text(format!(
                "Substring '{}' not found in {}",
                old, path
            )));
        }

        // Write back
        self.sandbox
            .write_file(path, &new_content, false)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;

        Ok(ToolOutput::text(format!(
            "Replaced '{}' with '{}' in {}",
            old, new, path
        )))
    }
}

/// List directory tool
pub struct ListDirectoryTool {
    /// Sandbox to list directories in
    sandbox: Arc<dyn Sandbox>,
    /// Schema for this tool
    schema: ToolSchema,
}

impl ListDirectoryTool {
    /// Create a new list directory tool
    pub fn new(sandbox: Arc<dyn Sandbox>) -> Self {
        let schema = ToolSchema::new(
            "ls",
            "List the contents of a directory in the sandbox. Supports recursive listing with depth control.",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the directory relative to sandbox root",
                        "default": "."
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "List directories recursively",
                        "default": false
                    },
                    "max_depth": {
                        "type": "integer",
                        "description": "Maximum recursion depth (0 = unlimited)",
                        "default": 2
                    }
                },
                "required": []
            }),
        )
        .with_group("sandbox");

        Self { sandbox, schema }
    }
}

#[async_trait::async_trait]
impl Tool for ListDirectoryTool {
    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn execute(&self, args: &ToolArgs, _context: &ToolContext) -> ToolResult {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let recursive = args.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false);

        let max_depth = args
            .get("max_depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize;

        let entries = self
            .sandbox
            .list_dir(path, recursive, max_depth)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list directory: {}", e)))?;

        if entries.is_empty() {
            return Ok(ToolOutput::text(format!("Directory is empty or does not exist: {}", path)));
        }

        // Format output
        let mut output = String::new();
        for entry in &entries {
            let prefix = if entry.is_dir { "[DIR] " } else { "[FILE] " };
            output.push_str(&format!(
                "{}{} ({} bytes)\n",
                prefix, entry.name, entry.size
            ));
        }

        Ok(ToolOutput::text(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_tool_schema() {
        // Mock sandbox for testing schema
        struct MockSandbox;
        #[async_trait::async_trait]
        impl Sandbox for MockSandbox {
            fn id(&self) -> &str {
                "mock"
            }
            async fn execute_command(
                &self,
                _command: &str,
                _cwd: Option<&str>,
                _timeout_secs: Option<u64>,
            ) -> Result<crate::sandbox::base::SandboxCommandResult> {
                Ok(crate::sandbox::base::SandboxCommandResult::success(
                    "test".to_string(),
                ))
            }
            async fn read_file(
                &self,
                _path: &str,
            ) -> Result<crate::sandbox::base::SandboxFileResult> {
                Ok(crate::sandbox::base::SandboxFileResult {
                    content: String::new(),
                    path: std::path::PathBuf::new(),
                    exists: false,
                })
            }
            async fn write_file(&self, _path: &str, _content: &str, _append: bool) -> Result<()> {
                Ok(())
            }
            async fn list_dir(
                &self,
                _path: &str,
                _recursive: bool,
                _max_depth: usize,
            ) -> Result<Vec<crate::sandbox::base::SandboxDirEntry>> {
                Ok(vec![])
            }
            async fn exists(&self, _path: &str) -> Result<bool> {
                Ok(false)
            }
            async fn delete(&self, _path: &str, _recursive: bool) -> Result<()> {
                Ok(())
            }
            async fn create_dir(&self, _path: &str, _parents: bool) -> Result<()> {
                Ok(())
            }
            fn real_path(&self, _virtual_path: &str) -> std::path::PathBuf {
                std::path::PathBuf::new()
            }
            fn virtual_path(&self, _real_path: &std::path::PathBuf) -> String {
                String::new()
            }
            async fn cleanup(&self) -> Result<()> {
                Ok(())
            }
        }

        let tool = BashTool::new(Arc::new(MockSandbox));
        assert_eq!(tool.schema().name, "bash");
        assert_eq!(tool.schema().group.as_deref(), Some("sandbox"));
    }
}
