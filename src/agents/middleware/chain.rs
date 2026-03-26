//! Middleware chain
//!
//! This module provides the MiddlewareChain for executing middlewares in order.

use crate::{
    agents::{
        middleware::base::{Middleware, MiddlewareContext, MiddlewareHook},
        state::ThreadState,
    },
    error::{HarnessError, MiddlewareError},
    Result,
};
use std::sync::Arc;

/// Middleware chain for executing middlewares in sequence
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareChain {
    /// Create a new empty middleware chain
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Add a middleware to the chain
    ///
    /// Middlewares are executed in the order they are added.
    pub fn add(&mut self, middleware: Arc<dyn Middleware>) -> &mut Self {
        self.middlewares.push(middleware);
        self
    }

    /// Add a middleware from a type that implements Middleware
    pub fn add_middleware<T: Middleware + 'static>(&mut self, middleware: T) -> &mut Self {
        self.middlewares.push(Arc::new(middleware));
        self
    }
    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    /// Execute all middlewares for a specific hook
    ///
    /// Returns Ok(()) if all middlewares executed successfully,
    /// or an error if any middleware fails.
    pub async fn execute_hook(
        &self,
        hook: MiddlewareHook,
        thread_id: &str,
        state: &mut ThreadState,
    ) -> Result<()> {
        for middleware in &self.middlewares {
            if middleware.hooks().contains(&hook) {
                let mut context = MiddlewareContext::new(thread_id, hook)
                    .with_state(state.clone());

                middleware.execute(&mut context).await?;

                // Update state with any changes from middleware
                *state = context.state;
            }
        }
        Ok(())
    }

    /// Execute all middlewares with a custom context
    pub async fn execute_with_context(&self, context: &mut MiddlewareContext) -> Result<()> {
        let hook = context.hook;

        for middleware in &self.middlewares {
            if middleware.hooks().contains(&hook) {
                middleware.execute(context).await?;
            }
        }
        Ok(())
    }

    /// Find a middleware by name
    pub fn find_by_name(&self, name: &str) -> Option<&dyn Middleware> {
        self.middlewares
            .iter()
            .find(|m| m.name() == name)
            .map(|m| m.as_ref())
    }

    /// Remove a middleware by name
    pub fn remove_by_name(&mut self, name: &str) -> Result<Arc<dyn Middleware>> {
        let pos = self
            .middlewares
            .iter()
            .position(|m| m.name() == name)
            .ok_or_else(|| {
                HarnessError::Middleware(MiddlewareError::ExecutionFailed(
                    name.to_string(),
                    "Middleware not found".to_string(),
                ))
            })?;

        Ok(self.middlewares.remove(pos))
    }

    /// Clear all middlewares
    pub fn clear(&mut self) {
        self.middlewares.clear();
    }

    /// Get all middleware names
    pub fn list_names(&self) -> Vec<String> {
        self.middlewares
            .iter()
            .map(|m| m.name().to_string())
            .collect()
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    // Test middleware
    struct TestMiddleware {
        name: String,
        hooks: Vec<MiddlewareHook>,
    }

    #[async_trait]
    impl Middleware for TestMiddleware {
        fn name(&self) -> &str {
            &self.name
        }

        fn hooks(&self) -> &[MiddlewareHook] {
            &self.hooks
        }

        async fn execute(&self, context: &mut MiddlewareContext) -> Result<()> {
            context.metadata = serde_json::json!({
                "executed": context.metadata.get("executed")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) + 1
            });
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_middleware_chain_new() {
        let chain = MiddlewareChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[tokio::test]
    async fn test_middleware_chain_add() {
        let mut chain = MiddlewareChain::new();
        let middleware = TestMiddleware {
            name: "test".to_string(),
            hooks: vec![MiddlewareHook::BeforeModel],
        };

        chain.add_middleware(middleware);
        assert_eq!(chain.len(), 1);
    }

    #[tokio::test]
    async fn test_middleware_chain_execute_hook() {
        let mut chain = MiddlewareChain::new();
        chain.add_middleware(TestMiddleware {
            name: "test1".to_string(),
            hooks: vec![MiddlewareHook::BeforeModel],
        });
        chain.add_middleware(TestMiddleware {
            name: "test2".to_string(),
            hooks: vec![MiddlewareHook::AfterModel],
        });

        let mut state = ThreadState::default();
        chain
            .execute_hook(MiddlewareHook::BeforeModel, "test-thread", &mut state)
            .await
            .unwrap();

        // Only the BeforeModel middleware should execute
        assert_eq!(chain.len(), 2);
    }

    #[tokio::test]
    async fn test_middleware_chain_find_by_name() {
        let mut chain = MiddlewareChain::new();
        chain.add_middleware(TestMiddleware {
            name: "test".to_string(),
            hooks: vec![MiddlewareHook::BeforeModel],
        });

        let found = chain.find_by_name("test");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "test");
    }

    #[tokio::test]
    async fn test_middleware_chain_remove_by_name() {
        let mut chain = MiddlewareChain::new();
        chain.add_middleware(TestMiddleware {
            name: "test".to_string(),
            hooks: vec![MiddlewareHook::BeforeModel],
        });

        let removed = chain.remove_by_name("test");
        assert!(removed.is_ok());
        assert!(chain.is_empty());
    }

    #[tokio::test]
    async fn test_middleware_chain_clear() {
        let mut chain = MiddlewareChain::new();
        chain.add_middleware(TestMiddleware {
            name: "test".to_string(),
            hooks: vec![MiddlewareHook::BeforeModel],
        });

        chain.clear();
        assert!(chain.is_empty());
    }
}
