//! Tool registry
//!
//! This module provides the ToolRegistry for managing and discovering tools.

use crate::{
    error::{HarnessError, ToolError},
    tools::base::Tool,
    Result,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Tool registry for managing available tools
///
/// The registry maintains a collection of tools that can be used by agents.
/// Tools are registered by name and can be retrieved for execution.
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    /// Create a new empty tool registry
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    /// Register a tool
    ///
    /// Returns an error if a tool with the same name is already registered.
    pub fn register(&self, tool: Arc<dyn Tool>) -> Result<()> {
        let name = tool.schema().name.clone();
        let mut tools = self.tools.write().map_err(|e| {
            HarnessError::other(format!("Failed to acquire write lock: {}", e))
        })?;

        if tools.contains_key(&name) {
            return Err(HarnessError::Tool(ToolError::NotFound(format!(
                "Tool '{}' already registered",
                name
            ))));
        }

        tools.insert(name, tool);
        Ok(())
    }

    /// Register or replace a tool
    ///
    /// If a tool with the same name exists, it will be replaced.
    pub fn register_or_replace(&self, tool: Arc<dyn Tool>) -> Result<()> {
        let name = tool.schema().name.clone();
        let mut tools = self.tools.write().map_err(|e| {
            HarnessError::other(format!("Failed to acquire write lock: {}", e))
        })?;

        tools.insert(name, tool);
        Ok(())
    }

    /// Unregister a tool by name
    pub fn unregister(&self, name: &str) -> Result<()> {
        let mut tools = self.tools.write().map_err(|e| {
            HarnessError::other(format!("Failed to acquire write lock: {}", e))
        })?;

        tools
            .remove(name)
            .ok_or_else(|| HarnessError::Tool(ToolError::NotFound(name.to_string())))?;
        Ok(())
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Result<Arc<dyn Tool>> {
        let tools = self.tools.read().map_err(|e| {
            HarnessError::other(format!("Failed to acquire read lock: {}", e))
        })?;

        tools
            .get(name)
            .cloned()
            .ok_or_else(|| HarnessError::Tool(ToolError::NotFound(name.to_string())))
    }

    /// Check if a tool is registered
    pub fn contains(&self, name: &str) -> bool {
        self.tools
            .read()
            .map(|tools| tools.contains_key(name))
            .unwrap_or(false)
    }

    /// List all registered tool names
    pub fn list(&self) -> Vec<String> {
        self.tools
            .read()
            .map(|tools| tools.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get the number of registered tools
    pub fn len(&self) -> usize {
        self.tools
            .read()
            .map(|tools| tools.len())
            .unwrap_or(0)
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// List tools by group
    pub fn list_by_group(&self, group: &str) -> Vec<String> {
        self.tools
            .read()
            .map(|tools| {
                tools
                    .iter()
                    .filter(|(_, tool)| {
                        tool.schema().group.as_deref() == Some(group)
                    })
                    .map(|(name, _)| name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Clear all registered tools
    pub fn clear(&self) -> Result<()> {
        let mut tools = self.tools.write().map_err(|e| {
            HarnessError::other(format!("Failed to acquire write lock: {}", e))
        })?;

        tools.clear();
        Ok(())
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::base::{ToolContext, ToolSchema, ToolOutput};
    use async_trait::async_trait;

    // Test tool implementation
    struct TestTool {
        schema: ToolSchema,
    }

    #[async_trait]
    impl Tool for TestTool {
        fn schema(&self) -> &ToolSchema {
            &self.schema
        }

        async fn execute(&self, _args: &crate::tools::base::ToolArgs, _context: &ToolContext) -> crate::tools::base::ToolResult {
            Ok(ToolOutput::text("Test output"))
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register() {
        let registry = ToolRegistry::new();
        let tool = Arc::new(TestTool {
            schema: ToolSchema::new("test", "Test tool", serde_json::json!({})),
        });

        assert!(registry.register(tool.clone()).is_ok());
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test"));
    }

    #[test]
    fn test_registry_register_duplicate() {
        let registry = ToolRegistry::new();
        let tool = Arc::new(TestTool {
            schema: ToolSchema::new("test", "Test tool", serde_json::json!({})),
        });

        assert!(registry.register(tool.clone()).is_ok());
        assert!(registry.register(tool).is_err());
    }

    #[test]
    fn test_registry_register_or_replace() {
        let registry = ToolRegistry::new();
        let tool1 = Arc::new(TestTool {
            schema: ToolSchema::new("test", "Test tool 1", serde_json::json!({})),
        });
        let tool2 = Arc::new(TestTool {
            schema: ToolSchema::new("test", "Test tool 2", serde_json::json!({})),
        });

        assert!(registry.register_or_replace(tool1).is_ok());
        assert!(registry.register_or_replace(tool2).is_ok());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_get() {
        let registry = ToolRegistry::new();
        let tool = Arc::new(TestTool {
            schema: ToolSchema::new("test", "Test tool", serde_json::json!({})),
        });

        assert!(registry.register(tool).is_ok());
        assert!(registry.get("test").is_ok());
        assert!(registry.get("nonexistent").is_err());
    }

    #[test]
    fn test_registry_unregister() {
        let registry = ToolRegistry::new();
        let tool = Arc::new(TestTool {
            schema: ToolSchema::new("test", "Test tool", serde_json::json!({})),
        });

        assert!(registry.register(tool).is_ok());
        assert!(registry.unregister("test").is_ok());
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_list() {
        let registry = ToolRegistry::new();

        let tool1 = Arc::new(TestTool {
            schema: ToolSchema::new("test1", "Test tool 1", serde_json::json!({}))
                .with_group("group1"),
        });
        let tool2 = Arc::new(TestTool {
            schema: ToolSchema::new("test2", "Test tool 2", serde_json::json!({}))
                .with_group("group2"),
        });

        assert!(registry.register(tool1).is_ok());
        assert!(registry.register(tool2).is_ok());

        let all_tools = registry.list();
        assert_eq!(all_tools.len(), 2);

        let group1_tools = registry.list_by_group("group1");
        assert_eq!(group1_tools.len(), 1);
        assert_eq!(group1_tools[0], "test1");
    }

    #[test]
    fn test_registry_clear() {
        let registry = ToolRegistry::new();
        let tool = Arc::new(TestTool {
            schema: ToolSchema::new("test", "Test tool", serde_json::json!({})),
        });

        assert!(registry.register(tool).is_ok());
        assert!(registry.clear().is_ok());
        assert!(registry.is_empty());
    }
}
