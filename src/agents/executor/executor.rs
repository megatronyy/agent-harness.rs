//! Agent executor
//!
//! This module provides the AgentExecutor for running agent interactions.

use crate::{
    agents::{
        middleware::{MiddlewareChain, MiddlewareContext, MiddlewareHook},
        state::ThreadState,
    },
    error::{HarnessError, ModelError},
    models::base::{ChatModel, Message, MessageRole, ModelRequest, ToolCall},
    runtime::RuntimeContext,
    tools::{ToolArgs, ToolContext, ToolExecutor},
    Result,
};
use std::pin::Pin;
use std::sync::Arc;

/// Agent executor for running agent interactions
pub struct AgentExecutor {
    /// Model instance
    pub model: Arc<dyn ChatModel>,

    /// Tool executor
    pub tool_executor: ToolExecutor,

    /// Middleware chain
    pub middlewares: MiddlewareChain,

    /// Runtime context
    pub runtime_context: RuntimeContext,

    /// System prompt template
    pub system_prompt: Option<String>,

    /// Whether thinking is enabled
    pub thinking_enabled: bool,

    /// Maximum iterations
    pub max_iterations: Option<usize>,
}

impl AgentExecutor {
    /// Create a new agent executor with the given configuration
    pub fn new(
        model: Arc<dyn ChatModel>,
        tool_executor: ToolExecutor,
        middlewares: MiddlewareChain,
        runtime_context: RuntimeContext,
        system_prompt: Option<String>,
        thinking_enabled: bool,
        max_iterations: Option<usize>,
    ) -> Self {
        Self {
            model,
            tool_executor,
            middlewares,
            runtime_context,
            system_prompt,
            thinking_enabled,
            max_iterations,
        }
    }

    /// Get the model
    pub fn model(&self) -> &Arc<dyn ChatModel> {
        &self.model
    }

    /// Get the tool executor
    pub fn tool_executor(&self) -> &ToolExecutor {
        &self.tool_executor
    }

    /// Get the middleware chain
    pub fn middlewares(&self) -> &MiddlewareChain {
        &self.middlewares
    }

    /// Get the runtime context
    pub fn runtime_context(&self) -> &RuntimeContext {
        &self.runtime_context
    }

    /// Execute the agent with a user message
    pub async fn run(&self, user_message: impl Into<String>) -> Result<String> {
        let user_message = user_message.into();
        let thread_id = self.runtime_context.require_thread_id().to_string();

        // Initialize thread state
        let mut state = ThreadState::default();
        state.thread_id = Some(thread_id.clone());
        state.messages.push(Message {
            role: MessageRole::Human,
            content: crate::messages::Content::text(user_message),
            tool_calls: vec![],
            tool_call_id: None,
            metadata: None,
            usage: None,
        });

        // Run before_model middlewares
        let mut context = MiddlewareContext::new(&thread_id, MiddlewareHook::BeforeModel)
            .with_state(state.clone());
        self.middlewares.execute_with_context(&mut context).await?;
        state = context.state;

        // Execute model invocation loop
        let max_iters = self.max_iterations.unwrap_or(10);
        for _ in 0..max_iters {
            // Build request
            let mut request_messages = state.messages.clone();

            // Add system prompt if available
            if let Some(ref prompt) = self.system_prompt {
                let system_msg = Message {
                    role: MessageRole::System,
                    content: crate::messages::Content::text(prompt.clone()),
                    tool_calls: vec![],
                    tool_call_id: None,
                    metadata: None,
                    usage: None,
                };
                request_messages.insert(0, system_msg);
            }

            let request = ModelRequest {
                messages: request_messages,
                temperature: Some(0.7),
                max_tokens: Some(4096),
                top_p: None,
                top_k: None,
                stop: vec![],
            };

            // Invoke model
            let response = self.model.invoke(request).await?;

            // Add AI response to state
            state.messages.push(response.message.clone());

            // Run after_model middlewares
            let mut context = MiddlewareContext::new(&thread_id, MiddlewareHook::AfterModel)
                .with_state(state.clone());
            self.middlewares.execute_with_context(&mut context).await?;
            state = context.state;

            // Check if there are tool calls to execute
            if response.message.tool_calls.is_empty() {
                // No tool calls, we're done
                break;
            }

            // Execute tool calls
            for tool_call in &response.message.tool_calls {
                // Run before_tool middlewares
                let mut context = MiddlewareContext::new(&thread_id, MiddlewareHook::BeforeTool)
                    .with_state(state.clone());
                self.middlewares.execute_with_context(&mut context).await?;
                state = context.state;

                // Execute the tool
                let tool_args: ToolArgs = tool_call.arguments.clone();
                let tool_context = ToolContext::new(&thread_id);
                let tool_result = self
                    .tool_executor
                    .execute(&tool_call.name, &tool_args, &tool_context)
                    .await?;

                // Create tool response message
                let tool_response = Message {
                    role: MessageRole::Tool,
                    content: crate::messages::Content::text(tool_result.to_string()),
                    tool_calls: vec![],
                    tool_call_id: Some(tool_call.id.clone()),
                    metadata: None,
                    usage: None,
                };

                state.messages.push(tool_response);

                // Run after_tool middlewares
                let mut context = MiddlewareContext::new(&thread_id, MiddlewareHook::AfterTool)
                    .with_state(state.clone());
                self.middlewares.execute_with_context(&mut context).await?;
                state = context.state;
            }

            // Run before_completion middlewares
            let mut context = MiddlewareContext::new(&thread_id, MiddlewareHook::BeforeCompletion)
                .with_state(state.clone());
            self.middlewares.execute_with_context(&mut context).await?;
            state = context.state;
        }

        // Run after_completion middlewares
        let mut context = MiddlewareContext::new(&thread_id, MiddlewareHook::AfterCompletion)
            .with_state(state.clone());
        self.middlewares.execute_with_context(&mut context).await?;

        // Extract final response
        let last_message = state
            .messages
            .iter()
            .filter(|m| m.role == MessageRole::Ai)
            .last()
            .ok_or_else(|| {
                HarnessError::Model(ModelError::InvalidResponse(
                    "No AI response generated".to_string(),
                ))
            })?;

        Ok(last_message.content.to_string())
    }

    /// Stream the agent execution
    pub async fn stream(
        &self,
        user_message: impl Into<String>,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<String>> + Send>>> {
        use futures::stream;
        use std::pin::Pin;

        // For now, just call run and stream the result
        let result = self.run(user_message).await?;
        let stream = stream::once(async move { Ok(result) });
        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolRegistry;

    #[test]
    fn test_agent_executor_new() {
        // This test requires a model, so we'll just test the structure
        let tools = ToolRegistry::new();
        let tool_executor = ToolExecutor::new(tools);
        let middlewares = MiddlewareChain::new();
        let runtime_context = RuntimeContext::with_thread_id("test");

        // We can't create a real executor without a model, but we can test the structure
        assert_eq!(runtime_context.require_thread_id(), "test");
        assert!(middlewares.is_empty());
    }
}
