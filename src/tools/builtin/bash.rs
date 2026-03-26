//! Bash tool
//!
//! This tool executes bash commands.

use crate::{
    error::HarnessError,
    tools::base::{Tool, ToolArgs, ToolContext, ToolOutput, ToolSchema},
};
use async_trait::async_trait;
use serde_json::json;

/// Bash command execution tool
pub struct BashTool {
    schema: ToolSchema,
}

impl BashTool {
    /// Create a new bash tool
    pub fn new() -> Self {
        let schema = ToolSchema::new(
            "bash",
            "Execute a bash command in the terminal",
            json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The bash command to execute"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "The working directory for the command (optional)"
                    }
                },
                "required": ["command"]
            }),
        )
        .with_group("sandbox");

        Self { schema }
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BashTool {
    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn execute(&self, args: &ToolArgs, context: &ToolContext) -> crate::tools::base::ToolResult {
        // Extract and validate arguments
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| HarnessError::other("Missing 'command' argument"))?;

        let cwd = args
            .get("cwd")
            .and_then(|v| v.as_str())
            .or(context.working_dir.as_deref());

        // Execute the command
        let output = if let Some(dir) = cwd {
            tokio::process::Command::new("bash")
                .arg("-c")
                .arg(command)
                .current_dir(dir)
                .output()
                .await
        } else {
            tokio::process::Command::new("bash")
                .arg("-c")
                .arg(command)
                .output()
                .await
        };

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                if output.status.success() {
                    Ok(ToolOutput::text(stdout))
                } else {
                    // Command failed but not due to system error
                    let result = if !stderr.is_empty() {
                        stderr
                    } else {
                        format!("Command failed with exit code: {:?}", output.status.code())
                    };
                    Ok(ToolOutput::error(result))
                }
            }
            Err(e) => {
                // System error (command not found, permission denied, etc.)
                Err(HarnessError::other(format!("Failed to execute command: {}", e)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_tool_schema() {
        let tool = BashTool::new();
        assert_eq!(tool.schema().name, "bash");
        assert_eq!(tool.schema().group.as_deref(), Some("sandbox"));
    }

    #[tokio::test]
    async fn test_bash_tool_execute_echo() {
        let tool = BashTool::new();
        let context = ToolContext::new("test-thread");

        let args = json!({"command": "echo 'Hello, World!'"});
        let result = tool.execute(&args, &context).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.to_string(), "Hello, World!\n");
    }

    #[tokio::test]
    async fn test_bash_tool_execute_missing_command() {
        let tool = BashTool::new();
        let context = ToolContext::new("test-thread");

        let args = json!({});
        let result = tool.execute(&args, &context).await;

        assert!(result.is_err());
    }
}
