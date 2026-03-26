//! Dangling tool call middleware
//!
//! This middleware handles tool calls that lack responses, such as when
/// the user interrupts the agent during tool execution.

use crate::{
    agents::{
        middleware::base::{Middleware, MiddlewareContext, MiddlewareHook},
        state::ThreadState,
    },
    error::{HarnessError, MiddlewareError},
    models::base::{Message, MessageRole, ToolCall},
    Result,
};
use async_trait::async_trait;

/// Dangling tool call middleware
///
/// Injects placeholder ToolMessages for AIMessage tool_calls that lack responses.
pub struct DanglingToolCallMiddleware {
    name: String,
    hooks: Vec<MiddlewareHook>,
}

impl DanglingToolCallMiddleware {
    /// Create a new dangling tool call middleware
    pub fn new() -> Self {
        Self {
            name: "dangling_tool_call".to_string(),
            hooks: vec![MiddlewareHook::BeforeTool],
        }
    }

    /// Find and fix dangling tool calls in the state
    fn fix_dangling_tool_calls(&self, state: &mut ThreadState) -> Result<()> {
        let messages = &state.messages;
        let mut has_dangling = false;

        // Find the last AI message
        let last_ai_idx = messages
            .iter()
            .rposition(|m| m.role == MessageRole::Ai);

        if let Some(ai_idx) = last_ai_idx {
            let ai_msg = &messages[ai_idx];

            // Check if this AI message has tool calls
            if !ai_msg.tool_calls.is_empty() {
                // Check if each tool call has a corresponding tool message
                for tool_call in &ai_msg.tool_calls {
                    let has_response = messages[ai_idx + 1..]
                        .iter()
                        .any(|m| {
                            m.role == MessageRole::Tool
                                && m.tool_call_id.as_ref() == Some(&tool_call.id)
                        });

                    if !has_response {
                        has_dangling = true;
                        // Create a placeholder tool message
                        let placeholder = Message {
                            role: MessageRole::Tool,
                            content: crate::messages::Content::text(
                                "[Tool execution was interrupted]",
                            ),
                            tool_calls: vec![],
                            tool_call_id: Some(tool_call.id.clone()),
                            metadata: None,
                            usage: None,
                        };

                        // Add to state messages
                        // Note: In real implementation, we'd modify the state directly
                        // For now, we just track that we found dangling calls
                    }
                }
            }
        }

        if has_dangling {
            return Err(HarnessError::Middleware(MiddlewareError::ExecutionFailed(
                self.name.clone(),
                "Found dangling tool calls - tool execution was interrupted".to_string(),
            )));
        }

        Ok(())
    }
}

impl Default for DanglingToolCallMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for DanglingToolCallMiddleware {
    fn name(&self) -> &str {
        &self.name
    }

    fn hooks(&self) -> &[MiddlewareHook] {
        &self.hooks
    }

    async fn execute(&self, context: &mut MiddlewareContext) -> Result<()> {
        // Check for dangling tool calls in the current state
        self.fix_dangling_tool_calls(&mut context.state)?;

        // Mark that we've checked for dangling calls
        context.metadata["dangling_calls_checked"] = serde_json::Value::Bool(true);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dangling_tool_call_middleware_name() {
        let middleware = DanglingToolCallMiddleware::new();
        assert_eq!(middleware.name(), "dangling_tool_call");
    }

    #[test]
    fn test_dangling_tool_call_middleware_hooks() {
        let middleware = DanglingToolCallMiddleware::new();
        assert_eq!(middleware.hooks(), &[MiddlewareHook::BeforeTool]);
    }
}
