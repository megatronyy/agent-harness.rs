//! Agent builder
//!
//! This module provides the AgentBuilder for constructing agent configurations.

use crate::{
    agents::{
        middleware::MiddlewareChain,
        state::ThreadState,
    },
    config::ModelConfig,
    models::base::ChatModel,
    runtime::RuntimeContext,
    tools::ToolRegistry,
    Result,
};
use std::sync::Arc;

/// Agent builder for constructing agent configurations
pub struct AgentBuilder {
    /// Model configuration
    model_config: Option<ModelConfig>,
    /// Model instance (if created)
    model: Option<Arc<dyn ChatModel>>,
    /// Tool registry
    tools: ToolRegistry,
    /// Middleware chain
    middlewares: MiddlewareChain,
    /// Runtime context
    runtime_context: Option<RuntimeContext>,
    /// System prompt template
    system_prompt: Option<String>,
    /// Whether thinking is enabled
    thinking_enabled: bool,
    /// Maximum iterations
    max_iterations: Option<usize>,
}

impl AgentBuilder {
    /// Create a new agent builder
    pub fn new() -> Self {
        Self {
            model_config: None,
            model: None,
            tools: ToolRegistry::new(),
            middlewares: MiddlewareChain::new(),
            runtime_context: None,
            system_prompt: None,
            thinking_enabled: false,
            max_iterations: Some(10),
        }
    }

    /// Set the model configuration
    pub fn with_model_config(mut self, config: ModelConfig) -> Self {
        self.model_config = Some(config);
        self
    }

    /// Set the model instance directly
    pub fn with_model(mut self, model: Arc<dyn ChatModel>) -> Self {
        self.model = Some(model);
        self
    }

    /// Set the runtime context
    pub fn with_runtime_context(mut self, ctx: RuntimeContext) -> Self {
        self.runtime_context = Some(ctx);
        self
    }

    /// Set the system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Enable or disable thinking mode
    pub fn with_thinking(mut self, enabled: bool) -> Self {
        self.thinking_enabled = enabled;
        self
    }

    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = Some(max);
        self
    }

    /// Add a middleware to the chain
    pub fn add_middleware(mut self, middleware: Arc<dyn crate::agents::middleware::Middleware>) -> Self {
        self.middlewares.add(middleware);
        self
    }

    /// Register a tool
    pub fn register_tool(self, tool: Arc<dyn crate::tools::base::Tool>) -> Result<Self> {
        // Create a new builder with the tool registered
        let mut new_tools = ToolRegistry::new();
        for name in self.tools.list() {
            let tool = self.tools.get(&name)?;
            new_tools.register(tool)?;
        }
        new_tools.register(tool)?;

        Ok(Self {
            model_config: self.model_config,
            model: self.model,
            tools: new_tools,
            middlewares: self.middlewares,
            runtime_context: self.runtime_context,
            system_prompt: self.system_prompt,
            thinking_enabled: self.thinking_enabled,
            max_iterations: self.max_iterations,
        })
    }

    /// Build the agent executor
    pub fn build(self) -> Result<super::AgentExecutor> {
        use crate::tools::ToolExecutor;

        // Determine which model to use
        let model = if let Some(model) = self.model {
            model
        } else if let Some(config) = self.model_config {
            crate::models::ModelFactory::create_model(&config)?
        } else {
            return Err(crate::error::HarnessError::other(
                "Either model or model_config must be set",
            ));
        };

        // Create runtime context if not provided
        let runtime_context = self.runtime_context.unwrap_or_else(|| {
            RuntimeContext::with_thread_id("default-thread")
        });

        Ok(super::AgentExecutor {
            model,
            tool_executor: ToolExecutor::new(self.tools),
            middlewares: self.middlewares,
            runtime_context,
            system_prompt: self.system_prompt,
            thinking_enabled: self.thinking_enabled,
            max_iterations: self.max_iterations,
        })
    }
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_builder_new() {
        let builder = AgentBuilder::new();
        assert!(builder.model.is_none());
        assert!(builder.model_config.is_none());
        assert!(builder.system_prompt.is_none());
    }

    #[test]
    fn test_agent_builder_with_system_prompt() {
        let builder = AgentBuilder::new().with_system_prompt("You are a helpful assistant.");
        assert_eq!(builder.system_prompt.as_deref(), Some("You are a helpful assistant."));
    }

    #[test]
    fn test_agent_builder_with_thinking() {
        let builder = AgentBuilder::new().with_thinking(true);
        assert!(builder.thinking_enabled);
    }

    #[test]
    fn test_agent_builder_with_max_iterations() {
        let builder = AgentBuilder::new().with_max_iterations(100);
        assert_eq!(builder.max_iterations, Some(100));
    }
}
