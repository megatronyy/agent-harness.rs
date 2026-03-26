//! Sandbox provider
//!
//! This module provides the SandboxProvider trait and holder for managing
//! sandbox instances with acquire/release lifecycle.

use crate::{Result, sandbox::base::Sandbox};
use std::sync::Arc;

/// Sandbox provider for managing sandbox instances
///
/// A provider handles the lifecycle of sandboxes, including acquisition
/// and release. This allows for pooling, caching, or dynamic creation
/// of sandbox instances.
#[async_trait::async_trait]
pub trait SandboxProvider: Send + Sync {
    /// Acquire a sandbox instance
    ///
    /// This may create a new sandbox or return an existing one from a pool.
    ///
    /// # Arguments
    /// * `thread_id` - Unique identifier for the thread/conversation
    async fn acquire(&self, thread_id: &str) -> Result<Arc<dyn Sandbox>>;

    /// Release a sandbox instance
    ///
    /// # Arguments
    /// * `sandbox_id` - The unique identifier of the sandbox to release
    async fn release(&self, sandbox_id: &str) -> Result<()>;

    /// Get a sandbox instance by ID
    ///
    /// # Arguments
    /// * `sandbox_id` - The unique identifier of the sandbox
    async fn get(&self, sandbox_id: &str) -> Result<Option<Arc<dyn Sandbox>>>;

    /// Clean up all sandboxes
    async fn cleanup_all(&self) -> Result<()>;
}

/// Holder for a sandbox provider
///
/// This wraps a provider in an Arc for easy sharing across the application.
#[derive(Clone)]
pub struct SandboxProviderHolder {
    /// The underlying provider
    provider: Arc<dyn SandboxProvider>,
}

impl SandboxProviderHolder {
    /// Create a new sandbox provider holder
    pub fn new(provider: Arc<dyn SandboxProvider>) -> Self {
        Self { provider }
    }

    /// Acquire a sandbox instance
    pub async fn acquire(&self, thread_id: &str) -> Result<Arc<dyn Sandbox>> {
        self.provider.acquire(thread_id).await
    }

    /// Release a sandbox instance
    pub async fn release(&self, sandbox_id: &str) -> Result<()> {
        self.provider.release(sandbox_id).await
    }

    /// Get a sandbox instance by ID
    pub async fn get(&self, sandbox_id: &str) -> Result<Option<Arc<dyn Sandbox>>> {
        self.provider.get(sandbox_id).await
    }

    /// Clean up all sandboxes
    pub async fn cleanup_all(&self) -> Result<()> {
        self.provider.cleanup_all().await
    }

    /// Get the underlying provider
    pub fn inner(&self) -> &Arc<dyn SandboxProvider> {
        &self.provider
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_provider_holder() {
        // Test holder creation
        let provider: Arc<dyn SandboxProvider> = Arc::new(MockProvider::new());
        let holder = SandboxProviderHolder::new(provider);
        assert!(Arc::strong_count(&holder.provider) >= 1);
    }

    // Mock provider for testing
    struct MockProvider;

    impl MockProvider {
        fn new() -> Self {
            Self
        }
    }

    #[async_trait::async_trait]
    impl SandboxProvider for MockProvider {
        async fn acquire(&self, _thread_id: &str) -> Result<Arc<dyn Sandbox>> {
            Err(crate::error::HarnessError::Sandbox(
                crate::error::SandboxError::NotFound("Mock provider".to_string()),
            ))
        }

        async fn release(&self, _sandbox_id: &str) -> Result<()> {
            Ok(())
        }

        async fn get(&self, _sandbox_id: &str) -> Result<Option<Arc<dyn Sandbox>>> {
            Ok(None)
        }

        async fn cleanup_all(&self) -> Result<()> {
            Ok(())
        }
    }
}
