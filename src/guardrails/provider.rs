//! Guardrail provider
//!
//! This module provides the guardrail provider trait for implementing
//! tool call validation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Result of a guardrail check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailResult {
    /// Whether the check passed
    pub allowed: bool,

    /// Reason for denial (if not allowed)
    pub denial_reason: Option<String>,

    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

impl GuardrailResult {
    /// Create an allowed result
    pub fn allowed() -> Self {
        Self {
            allowed: true,
            denial_reason: None,
            metadata: None,
        }
    }

    /// Create a denied result
    pub fn denied(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            denial_reason: Some(reason.into()),
            metadata: None,
        }
    }

    /// Create a denied result with metadata
    pub fn denied_with_metadata(
        reason: impl Into<String>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            allowed: false,
            denial_reason: Some(reason.into()),
            metadata: Some(metadata),
        }
    }
}

/// Information about a guardrail check
#[derive(Debug, Clone)]
pub struct GuardrailCheck<'a> {
    /// Tool name being called
    pub tool_name: &'a str,

    /// Tool arguments
    pub arguments: &'a serde_json::Value,

    /// Thread ID
    pub thread_id: &'a str,

    /// User ID (if available)
    pub user_id: Option<&'a str>,
}

impl<'a> GuardrailCheck<'a> {
    /// Create a new guardrail check
    pub fn new(
        tool_name: &'a str,
        arguments: &'a serde_json::Value,
        thread_id: &'a str,
    ) -> Self {
        Self {
            tool_name,
            arguments,
            thread_id,
            user_id: None,
        }
    }

    /// Set the user ID
    pub fn with_user_id(mut self, user_id: &'a str) -> Self {
        self.user_id = Some(user_id);
        self
    }
}

/// Guardrail provider for validating tool calls
#[async_trait]
pub trait GuardrailProvider: Send + Sync {
    /// Check if a tool call is allowed
    ///
    /// # Arguments
    /// * `check` - Information about the tool call being checked
    ///
    /// # Returns
    /// A GuardrailResult indicating whether the call is allowed
    async fn check(&self, check: GuardrailCheck<'_>) -> std::result::Result<GuardrailResult, Error>;

    /// Get the provider name
    fn name(&self) -> &str;

    /// Check if the provider is enabled
    fn enabled(&self) -> bool {
        true
    }
}

/// Error type for guardrails
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Check failed: {0}")]
    CheckFailed(String),
}

/// Result type for guardrails
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guardrail_result_allowed() {
        let result = GuardrailResult::allowed();
        assert!(result.allowed);
        assert!(result.denial_reason.is_none());
    }

    #[test]
    fn test_guardrail_result_denied() {
        let result = GuardrailResult::denied("Tool not allowed");
        assert!(!result.allowed);
        assert_eq!(result.denial_reason, Some("Tool not allowed".to_string()));
    }

    #[test]
    fn test_guardrail_check_new() {
        let args = serde_json::json!({"param": "value"});
        let check = GuardrailCheck::new("test_tool", &args, "thread-123");

        assert_eq!(check.tool_name, "test_tool");
        assert_eq!(check.arguments, &args);
        assert_eq!(check.thread_id, "thread-123");
        assert!(check.user_id.is_none());
    }

    #[test]
    fn test_guardrail_check_with_user_id() {
        let args = serde_json::json!({"param": "value"});
        let check = GuardrailCheck::new("test_tool", &args, "thread-123")
            .with_user_id("user-456");

        assert_eq!(check.user_id, Some("user-456"));
    }
}
