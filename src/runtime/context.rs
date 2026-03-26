//! Runtime context
//!
//! This module defines the runtime context that is passed through
//! the agent execution pipeline.

use std::collections::HashMap;
use std::sync::Arc;

use uuid::Uuid;

/// Runtime context for agent execution
///
/// This structure holds contextual information that is available
/// throughout the agent's execution lifecycle.
#[derive(Debug, Clone)]
pub struct RuntimeContext {
    /// Thread identifier
    pub thread_id: Option<String>,

    /// Agent name
    pub agent_name: Option<String>,

    /// Request identifier
    pub request_id: Uuid,

    /// Runtime metadata
    pub metadata: RuntimeMetadata,

    /// Additional context data
    pub extra: HashMap<String, String>,
}

impl RuntimeContext {
    /// Create a new runtime context
    pub fn new() -> Self {
        Self {
            thread_id: None,
            agent_name: None,
            request_id: Uuid::new_v4(),
            metadata: RuntimeMetadata::default(),
            extra: HashMap::new(),
        }
    }

    /// Create a new runtime context with a thread ID
    pub fn with_thread_id(thread_id: impl Into<String>) -> Self {
        Self {
            thread_id: Some(thread_id.into()),
            ..Self::new()
        }
    }

    /// Get the thread ID, panicking if not set
    pub fn require_thread_id(&self) -> &str {
        self.thread_id
            .as_deref()
            .expect("thread_id is required but not set")
    }

    /// Set the agent name
    pub fn with_agent_name(mut self, name: impl Into<String>) -> Self {
        self.agent_name = Some(name.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: RuntimeMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Add extra context data
    pub fn with_extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime metadata
///
/// Additional metadata about the current execution context.
#[derive(Debug, Clone, Default)]
pub struct RuntimeMetadata {
    /// Model name being used
    pub model_name: Option<String>,

    /// Whether thinking mode is enabled
    pub thinking_enabled: Option<bool>,

    /// Reasoning effort (for thinking models)
    pub reasoning_effort: Option<String>,

    /// Whether plan mode is enabled
    pub is_plan_mode: Option<bool>,

    /// Whether subagents are enabled
    pub subagent_enabled: Option<bool>,

    /// Maximum concurrent subagents
    pub max_concurrent_subagents: Option<usize>,

    /// Additional custom metadata
    pub custom: HashMap<String, String>,
}

impl RuntimeMetadata {
    /// Create new metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Set model name
    pub fn with_model_name(mut self, name: impl Into<String>) -> Self {
        self.model_name = Some(name.into());
        self
    }

    /// Set thinking enabled
    pub fn with_thinking_enabled(mut self, enabled: bool) -> Self {
        self.thinking_enabled = Some(enabled);
        self
    }

    /// Set plan mode
    pub fn with_plan_mode(mut self, enabled: bool) -> Self {
        self.is_plan_mode = Some(enabled);
        self
    }

    /// Add custom metadata
    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }
}

/// Shared runtime context wrapper
///
/// Provides thread-safe access to runtime context via Arc.
pub type SharedRuntimeContext = Arc<RuntimeContext>;

/// Create a shared runtime context
pub fn shared_context() -> SharedRuntimeContext {
    Arc::new(RuntimeContext::new())
}

/// Create a shared runtime context with thread ID
pub fn shared_context_with_thread(thread_id: impl Into<String>) -> SharedRuntimeContext {
    Arc::new(RuntimeContext::with_thread_id(thread_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_context_new() {
        let ctx = RuntimeContext::new();
        assert!(ctx.thread_id.is_none());
        assert!(ctx.agent_name.is_none());
    }

    #[test]
    fn test_runtime_context_with_thread_id() {
        let ctx = RuntimeContext::with_thread_id("test-thread");
        assert_eq!(ctx.thread_id.as_deref(), Some("test-thread"));
    }

    #[test]
    fn test_runtime_context_builder() {
        let ctx = RuntimeContext::new()
            .with_agent_name("test-agent")
            .with_extra("key", "value");

        assert_eq!(ctx.agent_name.as_deref(), Some("test-agent"));
        assert_eq!(ctx.extra.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_require_thread_id() {
        let ctx = RuntimeContext::with_thread_id("test-thread");
        assert_eq!(ctx.require_thread_id(), "test-thread");
    }

    #[test]
    #[should_panic(expected = "thread_id is required")]
    fn test_require_thread_id_panic() {
        let ctx = RuntimeContext::new();
        ctx.require_thread_id();
    }

    #[test]
    fn test_runtime_metadata() {
        let metadata = RuntimeMetadata::new()
            .with_model_name("claude-opus-4-6")
            .with_thinking_enabled(true)
            .with_plan_mode(false)
            .with_custom("custom_key", "custom_value");

        assert_eq!(metadata.model_name.as_deref(), Some("claude-opus-4-6"));
        assert_eq!(metadata.thinking_enabled, Some(true));
        assert_eq!(metadata.is_plan_mode, Some(false));
        assert_eq!(
            metadata.custom.get("custom_key"),
            Some(&"custom_value".to_string())
        );
    }

    #[test]
    fn test_shared_context() {
        let ctx = shared_context_with_thread("test-thread");
        assert_eq!(ctx.thread_id.as_deref(), Some("test-thread"));
    }
}
