//! Base middleware abstractions
//!
//! This module defines the core Middleware trait and associated types.

use crate::Result;
use async_trait::async_trait;

/// Middleware execution hook points
///
/// These hooks define when a middleware can intercept the agent execution flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MiddlewareHook {
    /// Before the model is invoked
    BeforeModel,
    /// After the model responds
    AfterModel,
    /// Before a tool is executed
    BeforeTool,
    /// After a tool is executed
    AfterTool,
    /// Before the agent completes
    BeforeCompletion,
    /// After the agent completes
    AfterCompletion,
}

/// Middleware execution context
///
/// Provides runtime information and state to middleware during execution.
#[derive(Debug, Clone)]
pub struct MiddlewareContext {
    /// Thread ID
    pub thread_id: String,

    /// Current state
    pub state: crate::agents::ThreadState,

    /// Hook that triggered this middleware execution
    pub hook: MiddlewareHook,

    /// Additional metadata
    pub metadata: serde_json::Value,
}

impl MiddlewareContext {
    /// Create a new middleware context
    pub fn new(thread_id: impl Into<String>, hook: MiddlewareHook) -> Self {
        Self {
            thread_id: thread_id.into(),
            state: crate::agents::ThreadState::default(),
            hook,
            metadata: serde_json::json!({}),
        }
    }

    /// Set the state
    pub fn with_state(mut self, state: crate::agents::ThreadState) -> Self {
        self.state = state;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.metadata[key.into()] = value.into();
        self
    }
}

/// Middleware trait for implementing agent middlewares
///
/// All middlewares must implement this trait to intercept agent execution.
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Get the middleware name
    fn name(&self) -> &str;

    /// Get the hooks this middleware responds to
    fn hooks(&self) -> &[MiddlewareHook];

    /// Execute the middleware
    ///
    /// The `context` provides information about the current execution state.
    /// Returns an error to halt execution or Ok(()) to continue.
    async fn execute(&self, context: &mut MiddlewareContext) -> Result<()>;

    /// Get as Any for downcasting (optional)
    fn as_any(&self) -> &dyn std::any::Any {
        &()
    }
}

/// Helper macro for implementing middleware hooks
#[macro_export]
macro_rules! impl_middleware {
    ($type:ty, $name:expr, [$($hook:expr),* $(,)?]) => {
        #[async_trait::async_trait]
        impl $crate::agents::middleware::Middleware for $type {
            fn name(&self) -> &str {
                $name
            }

            fn hooks(&self) -> &[$crate::agents::middleware::MiddlewareHook] {
                &[$($hook),*]
            }

            async fn execute(
                &self,
                context: &mut $crate::agents::middleware::MiddlewareContext,
            ) -> $crate::Result<()> {
                self.run(context).await
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_hook_equality() {
        assert_eq!(MiddlewareHook::BeforeModel, MiddlewareHook::BeforeModel);
        assert_ne!(MiddlewareHook::BeforeModel, MiddlewareHook::AfterModel);
    }

    #[test]
    fn test_middleware_context_builder() {
        let ctx = MiddlewareContext::new("test-thread", MiddlewareHook::BeforeModel)
            .with_metadata("key", "value");

        assert_eq!(ctx.thread_id, "test-thread");
        assert_eq!(ctx.hook, MiddlewareHook::BeforeModel);
        assert_eq!(ctx.metadata.get("key").and_then(|v| v.as_str()), Some("value"));
    }
}
