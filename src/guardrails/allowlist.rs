//! Allowlist provider
//!
//! This module provides the allowlist guardrail provider for
//! restricting tool calls to a predefined set.

use crate::guardrails::provider::{Error, GuardrailCheck, GuardrailProvider, GuardrailResult, Result};
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;

/// Allowlist guardrail provider
///
/// This provider only allows tool calls that match a predefined
/// set of allowed tools and optional patterns.
pub struct AllowlistProvider {
    /// Provider name
    name: String,

    /// Allowed tools
    allowed_tools: Arc<HashSet<String>>,

    /// Whether the provider is enabled
    enabled: bool,

    /// Denial message
    denial_message: String,
}

impl AllowlistProvider {
    /// Create a new allowlist provider
    pub fn new(allowed_tools: Vec<String>) -> Self {
        Self {
            name: "allowlist".to_string(),
            allowed_tools: Arc::new(allowed_tools.into_iter().collect()),
            enabled: true,
            denial_message: "Tool is not in the allowlist".to_string(),
        }
    }

    /// Set the provider name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set whether the provider is enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the denial message
    pub fn with_denial_message(mut self, message: impl Into<String>) -> Self {
        self.denial_message = message.into();
        self
    }

    /// Add a tool to the allowlist
    pub fn add_tool(&mut self, tool: impl Into<String>) {
        let mut tools = (*self.allowed_tools).clone();
        tools.insert(tool.into());
        self.allowed_tools = Arc::new(tools);
    }

    /// Remove a tool from the allowlist
    pub fn remove_tool(&mut self, tool: &str) -> bool {
        let mut tools = (*self.allowed_tools).clone();
        let removed = tools.remove(tool);
        self.allowed_tools = Arc::new(tools);
        removed
    }

    /// Check if a tool is in the allowlist
    pub fn is_allowed(&self, tool: &str) -> bool {
        self.allowed_tools.contains(tool)
    }

    /// Get the list of allowed tools
    pub fn allowed_tools(&self) -> Vec<String> {
        self.allowed_tools.iter().cloned().collect()
    }

    /// Get the number of tools in the allowlist
    pub fn len(&self) -> usize {
        self.allowed_tools.len()
    }

    /// Check if the allowlist is empty
    pub fn is_empty(&self) -> bool {
        self.allowed_tools.is_empty()
    }
}

impl Default for AllowlistProvider {
    fn default() -> Self {
        Self {
            name: "allowlist".to_string(),
            allowed_tools: Arc::new(HashSet::new()),
            enabled: true,
            denial_message: "Tool is not in the allowlist".to_string(),
        }
    }
}

#[async_trait]
impl GuardrailProvider for AllowlistProvider {
    async fn check(&self, check: GuardrailCheck<'_>) -> Result<GuardrailResult> {
        if !self.enabled {
            return Ok(GuardrailResult::allowed());
        }

        if self.allowed_tools.contains(check.tool_name) {
            Ok(GuardrailResult::allowed())
        } else {
            Ok(GuardrailResult::denied(format!(
                "{}: {}",
                self.denial_message, check.tool_name
            )))
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_allowlist_provider_default() {
        let provider = AllowlistProvider::default();
        assert_eq!(provider.name(), "allowlist");
        assert!(provider.enabled());
        assert!(provider.is_empty());
        assert_eq!(provider.len(), 0);
    }

    #[tokio::test]
    async fn test_allowlist_provider_new() {
        let provider = AllowlistProvider::new(vec!["tool1".to_string(), "tool2".to_string()]);

        assert_eq!(provider.len(), 2);
        assert!(provider.is_allowed("tool1"));
        assert!(provider.is_allowed("tool2"));
        assert!(!provider.is_allowed("tool3"));
    }

    #[tokio::test]
    async fn test_allowlist_provider_check() {
        let provider = AllowlistProvider::new(vec!["bash".to_string(), "read_file".to_string()]);

        let args = serde_json::json!({"command": "ls"});
        let check = GuardrailCheck::new("bash", &args, "thread-1");

        let result = provider.check(check).await.unwrap();
        assert!(result.allowed);

        let check2 = GuardrailCheck::new("delete_file", &args, "thread-1");
        let result2 = provider.check(check2).await.unwrap();
        assert!(!result2.allowed);
        assert!(result2.denial_reason.unwrap().contains("delete_file"));
    }

    #[tokio::test]
    async fn test_allowlist_provider_disabled() {
        let provider = AllowlistProvider::new(vec!["tool1".to_string()])
            .with_enabled(false);

        let args = serde_json::json!({});
        let check = GuardrailCheck::new("any_tool", &args, "thread-1");

        // Even tools not in the allowlist should be allowed when disabled
        let result = provider.check(check).await.unwrap();
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_allowlist_provider_add_remove() {
        let mut provider = AllowlistProvider::new(vec!["tool1".to_string()]);

        assert_eq!(provider.len(), 1);
        assert!(provider.is_allowed("tool1"));

        provider.add_tool("tool2");
        assert_eq!(provider.len(), 2);
        assert!(provider.is_allowed("tool2"));

        assert!(provider.remove_tool("tool1"));
        assert_eq!(provider.len(), 1);
        assert!(!provider.is_allowed("tool1"));

        assert!(!provider.remove_tool("nonexistent"));
    }

    #[tokio::test]
    async fn test_allowlist_provider_custom_denial() {
        let provider = AllowlistProvider::new(vec!["tool1".to_string()])
            .with_denial_message("Custom denial");

        let args = serde_json::json!({});
        let check = GuardrailCheck::new("tool2", &args, "thread-1");

        let result = provider.check(check).await.unwrap();
        assert!(!result.allowed);
        assert!(result.denial_reason.unwrap().contains("Custom denial"));
    }

    #[tokio::test]
    async fn test_allowlist_provider_with_name() {
        let provider = AllowlistProvider::new(vec![])
            .with_name("custom_allowlist");

        assert_eq!(provider.name(), "custom_allowlist");
    }
}
