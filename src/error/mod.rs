//! Error types for the agent-harness library.
//!
//! This module defines all error types used throughout the library,
//! organized by category (model, tool, config, etc.).

pub mod kinds;
pub mod result;

// Re-export common error types
pub use kinds::{
    ConfigError, ErrorCategory, HarnessError, MemoryError, MiddlewareError,
    ModelError, SandboxError, ToolError,
};
pub use result::Result;

use std::fmt;

/// Error context for additional debugging information
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// The operation that failed
    pub operation: String,
    /// Additional context
    pub context: Option<String>,
    /// Source error message
    pub source: Option<String>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            context: None,
            source: None,
        }
    }

    /// Add context to the error
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Set the source error message
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Format the error context for display
    pub fn format(&self) -> String {
        let mut parts = vec![format!("operation: {}", self.operation)];

        if let Some(context) = &self.context {
            parts.push(format!("context: {}", context));
        }

        if let Some(source) = &self.source {
            parts.push(format!("source: {}", source));
        }

        parts.join(", ")
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Macro for creating error contexts
#[macro_export]
macro_rules! error_context {
    ($operation:expr) => {
        $crate::error::ErrorContext::new($operation)
    };
    ($operation:expr, $context:expr) => {
        $crate::error::ErrorContext::new($operation).with_context($context)
    };
}
