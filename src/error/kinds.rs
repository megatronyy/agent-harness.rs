//! Core error types for the agent-harness library.
//!
//! This module defines the main `HarnessError` enum which encompasses
//! all possible errors that can occur in the library.

use std::fmt;

use thiserror::Error;

/// Main error type for the agent-harness library
///
/// This enum represents all possible errors that can occur when using
/// the agent-harness library, organized by category.
#[derive(Debug, Error)]
pub enum HarnessError {
    /// Configuration-related errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Model-related errors
    #[error("Model error: {0}")]
    Model(#[from] ModelError),

    /// Tool-related errors
    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    /// Middleware-related errors
    #[error("Middleware error: {0}")]
    Middleware(#[from] MiddlewareError),

    /// Sandbox-related errors
    #[error("Sandbox error: {0}")]
    Sandbox(#[from] SandboxError),

    /// Memory-related errors
    #[error("Memory error: {0}")]
    Memory(#[from] MemoryError),

    /// IO-related errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML parsing errors
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// HTTP-related errors
    #[error("HTTP error: {0}")]
    Http(String),

    /// Generic errors with context
    #[error("{0}")]
    Other(String),
}

impl HarnessError {
    /// Create a new generic error with a message
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }

    /// Create a new HTTP error with a message
    pub fn http(msg: impl Into<String>) -> Self {
        Self::Http(msg.into())
    }

    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Http(_) | Self::Model(ModelError::RateLimited(_) | ModelError::Timeout)
        )
    }

    /// Get the error category for logging/monitoring
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::Config(_) => ErrorCategory::Config,
            Self::Model(_) => ErrorCategory::Model,
            Self::Tool(_) => ErrorCategory::Tool,
            Self::Middleware(_) => ErrorCategory::Middleware,
            Self::Sandbox(_) => ErrorCategory::Sandbox,
            Self::Memory(_) => ErrorCategory::Memory,
            Self::Io(_) => ErrorCategory::Io,
            Self::Json(_) | Self::Yaml(_) => ErrorCategory::Serialization,
            Self::Http(_) => ErrorCategory::Network,
            Self::Other(_) => ErrorCategory::Other,
        }
    }
}

/// Error categories for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// Configuration errors
    Config,
    /// Model/LLM errors
    Model,
    /// Tool execution errors
    Tool,
    /// Middleware errors
    Middleware,
    /// Sandbox errors
    Sandbox,
    /// Memory system errors
    Memory,
    /// IO errors
    Io,
    /// Serialization errors
    Serialization,
    /// Network errors
    Network,
    /// Other uncategorized errors
    Other,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config => write!(f, "config"),
            Self::Model => write!(f, "model"),
            Self::Tool => write!(f, "tool"),
            Self::Middleware => write!(f, "middleware"),
            Self::Sandbox => write!(f, "sandbox"),
            Self::Memory => write!(f, "memory"),
            Self::Io => write!(f, "io"),
            Self::Serialization => write!(f, "serialization"),
            Self::Network => write!(f, "network"),
            Self::Other => write!(f, "other"),
        }
    }
}

/// Configuration-related errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to load configuration from {0}: {1}")]
    LoadError(String, String),

    #[error("Missing required configuration field: {0}")]
    MissingField(String),

    #[error("Invalid configuration value for {0}: {1}")]
    InvalidValue(String, String),

    #[error("Configuration file not found: {0}")]
    FileNotFound(String),

    #[error("Failed to parse environment variable {0}: {1}")]
    EnvVarError(String, String),

    #[error("Configuration version mismatch: expected {0}, found {1}")]
    VersionMismatch(String, String),
}

/// Model-related errors
#[derive(Debug, Error)]
pub enum ModelError {
    #[error("Model not found: {0}")]
    NotFound(String),

    #[error("Model initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Model invocation failed: {0}")]
    InvocationFailed(String),

    #[error("Rate limited: retry after {0} seconds")]
    RateLimited(u64),

    #[error("Request timeout")]
    Timeout,

    #[error("Invalid API key for provider {0}")]
    InvalidApiKey(String),

    #[error("Token limit exceeded: used {0}, limit {1}")]
    TokenLimitExceeded(u32, u32),

    #[error("Thinking mode not supported by model {0}")]
    ThinkingNotSupported(String),

    #[error("Vision mode not supported by model {0}")]
    VisionNotSupported(String),

    #[error("Invalid response from model: {0}")]
    InvalidResponse(String),

    #[error("Stream error: {0}")]
    StreamError(String),
}

/// Tool-related errors
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Invalid tool arguments: {0}")]
    InvalidArguments(String),

    #[error("Tool timeout: {0}")]
    Timeout(String),

    #[error("Tool returned error: {0}")]
    ToolErrorResult(String),

    #[error("Tool call interrupted")]
    Interrupted,

    #[error("Tool validation failed: {0}")]
    ValidationFailed(String),

    #[error("Too many tool calls: {0}")]
    TooManyCalls(usize),
}

/// Middleware-related errors
#[derive(Debug, Error)]
pub enum MiddlewareError {
    #[error("Middleware execution failed in {0}: {1}")]
    ExecutionFailed(String, String),

    #[error("Middleware chain error: {0}")]
    ChainError(String),

    #[error("Invalid middleware state: {0}")]
    InvalidState(String),

    #[error("Loop detected: {0}")]
    LoopDetected(String),

    #[error("Clarification required: {0}")]
    ClarificationRequired(String),
}

/// Sandbox-related errors
#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("Sandbox not found: {0}")]
    NotFound(String),

    #[error("Sandbox initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    #[error("Path traversal detected: {0}")]
    PathTraversal(String),

    #[error("File not found in sandbox: {0}")]
    FileNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Sandbox cleanup failed: {0}")]
    CleanupFailed(String),
}

/// Memory-related errors
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Memory storage error: {0}")]
    StorageError(String),

    #[error("Memory update failed: {0}")]
    UpdateFailed(String),

    #[error("Memory injection failed: {0}")]
    InjectionFailed(String),

    #[error("Memory queue full")]
    QueueFull,

    #[error("Memory not found: {0}")]
    NotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_category() {
        let err = HarnessError::Config(ConfigError::MissingField("test".to_string()));
        assert_eq!(err.category(), ErrorCategory::Config);

        let err = HarnessError::Model(ModelError::NotFound("gpt-4".to_string()));
        assert_eq!(err.category(), ErrorCategory::Model);
    }

    #[test]
    fn test_error_display() {
        let err = HarnessError::Model(ModelError::NotFound("gpt-4".to_string()));
        assert!(err.to_string().contains("gpt-4"));
        assert!(err.to_string().contains("Model error"));
    }

    #[test]
    fn test_retryable() {
        let err = HarnessError::http("Connection refused");
        assert!(err.is_retryable());

        let err = HarnessError::Model(ModelError::RateLimited(60));
        assert!(err.is_retryable());

        let err = HarnessError::Config(ConfigError::MissingField("test".to_string()));
        assert!(!err.is_retryable());
    }
}
