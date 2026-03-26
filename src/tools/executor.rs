//! Tool executor
//!
//! This module provides the ToolExecutor for executing tools with validation and error handling.

use crate::{
    error::{HarnessError, ToolError},
    tools::{
        base::{ToolArgs, ToolContext, ToolOutput},
        registry::ToolRegistry,
    },
    Result,
};
use std::sync::Arc;

/// Tool executor for running tools with validation
#[derive(Clone)]
pub struct ToolExecutor {
    registry: Arc<ToolRegistry>,
}

impl ToolExecutor {
    /// Create a new tool executor with a registry
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }

    /// Get the registry
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    /// Execute a tool by name with the given arguments
    pub async fn execute(
        &self,
        tool_name: &str,
        args: &ToolArgs,
        context: &ToolContext,
    ) -> Result<ToolOutput> {
        // Get the tool
        let tool = self.registry.get(tool_name)?;

        // Validate arguments
        tool.validate_args(args)?;

        // Execute the tool
        let result = tool.execute(args, context).await;

        // Map tool errors
        result.map_err(|e| match e {
            HarnessError::Tool(te) => HarnessError::Tool(te),
            other => HarnessError::Tool(ToolError::ExecutionFailed(other.to_string())),
        })
    }

    /// Execute a tool by name with the given arguments, returning a string result
    pub async fn execute_to_string(
        &self,
        tool_name: &str,
        args: &ToolArgs,
        context: &ToolContext,
    ) -> Result<String> {
        let output = self.execute(tool_name, args, context).await?;
        Ok(output.to_string())
    }

    /// Execute multiple tools in parallel
    pub async fn execute_parallel(
        &self,
        calls: Vec<(String, ToolArgs, ToolContext)>,
    ) -> Vec<Result<ToolOutput>> {
        let mut futures = Vec::new();

        for (tool_name, args, context) in calls {
            let executor = self.clone();
            let future = async move {
                executor.execute(&tool_name, &args, &context).await
            };
            futures.push(future);
        }

        futures::future::join_all(futures).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::base::{Tool, ToolSchema};
    use async_trait::async_trait;
    use serde_json::json;

    // Test tool
    struct TestTool {
        schema: ToolSchema,
    }

    #[async_trait]
    impl Tool for TestTool {
        fn schema(&self) -> &ToolSchema {
            &self.schema
        }

        async fn execute(&self, args: &ToolArgs, _context: &ToolContext) -> crate::tools::base::ToolResult {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("World");
            Ok(ToolOutput::text(format!("Hello, {}!", name)))
        }
    }

    #[tokio::test]
    async fn test_tool_executor_execute() {
        let registry = ToolRegistry::new();
        let tool = std::sync::Arc::new(TestTool {
            schema: ToolSchema::new(
                "test",
                "Test tool",
                json!({"type": "object"}),
            ),
        });

        registry.register(tool).unwrap();
        let executor = ToolExecutor::new(registry);

        let context = ToolContext::new("test-thread");
        let args = json!({"name": "Claude"});
        let result = executor.execute("test", &args, &context).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), "Hello, Claude!");
    }

    #[tokio::test]
    async fn test_tool_execute_to_string() {
        let registry = ToolRegistry::new();
        let tool = std::sync::Arc::new(TestTool {
            schema: ToolSchema::new(
                "test",
                "Test tool",
                json!({"type": "object"}),
            ),
        });

        registry.register(tool).unwrap();
        let executor = ToolExecutor::new(registry);

        let context = ToolContext::new("test-thread");
        let args = json!({});
        let result = executor.execute_to_string("test", &args, &context).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[tokio::test]
    async fn test_tool_executor_clone() {
        let registry = ToolRegistry::new();
        let tool = std::sync::Arc::new(TestTool {
            schema: ToolSchema::new(
                "test",
                "Test tool",
                json!({"type": "object"}),
            ),
        });

        registry.register(tool).unwrap();
        let executor = ToolExecutor::new(registry);

        // Test that executor can be cloned
        let executor2 = executor.clone();

        let context = ToolContext::new("test-thread");
        let args = json!({});

        let result1 = executor.execute("test", &args, &context).await;
        let result2 = executor2.execute("test", &args, &context).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }
}
