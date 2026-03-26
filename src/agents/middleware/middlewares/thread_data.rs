//! Thread data middleware
//!
//! This middleware initializes per-thread directories for file operations.

use crate::{
    agents::{
        middleware::base::{Middleware, MiddlewareContext, MiddlewareHook},
        state::ThreadState,
    },
    error::{HarnessError, MiddlewareError},
    Result,
};
use async_trait::async_trait;
use std::fs;

/// Thread data middleware
///
/// Creates and initializes per-thread directories for workspace, uploads, and outputs.
pub struct ThreadDataMiddleware {
    name: String,
    hooks: Vec<MiddlewareHook>,
    base_path: String,
}

impl ThreadDataMiddleware {
    /// Create a new thread data middleware
    pub fn new() -> Self {
        Self {
            name: "thread_data".to_string(),
            hooks: vec![MiddlewareHook::BeforeModel],
            base_path: ".deer-flow/threads".to_string(),
        }
    }

    /// Set the base path for thread directories
    pub fn with_base_path(mut self, path: impl Into<String>) -> Self {
        self.base_path = path.into();
        self
    }

    /// Ensure thread directories exist
    fn ensure_thread_directories(&self, thread_id: &str) -> Result<Vec<String>> {
        let thread_path = format!("{}/{}", self.base_path, thread_id);

        let workspace_path = format!("{}/user-data/workspace", thread_path);
        let uploads_path = format!("{}/user-data/uploads", thread_path);
        let outputs_path = format!("{}/user-data/outputs", thread_path);

        // Create all directories
        for path in &[&workspace_path, &uploads_path, &outputs_path] {
            fs::create_dir_all(path).map_err(|e| {
                HarnessError::Middleware(MiddlewareError::ExecutionFailed(
                    self.name.clone(),
                    format!("Failed to create directory {}: {}", path, e),
                ))
            })?;
        }

        Ok(vec![workspace_path, uploads_path, outputs_path])
    }
}

impl Default for ThreadDataMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for ThreadDataMiddleware {
    fn name(&self) -> &str {
        &self.name
    }

    fn hooks(&self) -> &[MiddlewareHook] {
        &self.hooks
    }

    async fn execute(&self, context: &mut MiddlewareContext) -> Result<()> {
        let thread_id = &context.thread_id;

        // Ensure thread directories exist
        let paths = self.ensure_thread_directories(thread_id)?;

        // Update context metadata with paths
        context.metadata = serde_json::json!({
            "thread_directories": {
                "workspace": paths[0],
                "uploads": paths[1],
                "outputs": paths[2],
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_data_middleware_name() {
        let middleware = ThreadDataMiddleware::new();
        assert_eq!(middleware.name(), "thread_data");
    }

    #[test]
    fn test_thread_data_middleware_hooks() {
        let middleware = ThreadDataMiddleware::new();
        assert_eq!(middleware.hooks(), &[MiddlewareHook::BeforeModel]);
    }

    #[test]
    fn test_thread_data_middleware_with_base_path() {
        let middleware = ThreadDataMiddleware::new()
            .with_base_path("/tmp/test-threads");
        assert_eq!(middleware.base_path, "/tmp/test-threads");
    }
}
