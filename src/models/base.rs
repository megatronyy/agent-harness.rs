//! Base model abstractions
//!
//! This module defines the core traits and types for interacting
//! with LLM model providers.

use async_trait::async_trait;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use crate::{Content, Result};

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenUsage {
    /// Input tokens consumed
    pub input_tokens: u32,
    /// Output tokens generated
    pub output_tokens: u32,
    /// Total tokens
    pub total_tokens: u32,
}

impl TokenUsage {
    /// Create new token usage
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
        }
    }
}

/// Model capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModelCapabilities {
    /// Supports extended thinking
    pub thinking: bool,
    /// Supports vision/image input
    pub vision: bool,
    /// Supports function calling
    pub function_calling: bool,
    /// Supports streaming responses
    pub streaming: bool,
}

impl ModelCapabilities {
    /// Create new capabilities (all false except streaming and function_calling)
    pub fn new() -> Self {
        Self {
            thinking: false,
            vision: false,
            function_calling: true,
            streaming: true,
        }
    }

    /// Set thinking capability
    pub fn with_thinking(mut self) -> Self {
        self.thinking = true;
        self
    }

    /// Set vision capability
    pub fn with_vision(mut self) -> Self {
        self.vision = true;
        self
    }
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// User message
    Human,
    /// AI message
    Ai,
    /// Tool message
    Tool,
    /// System message
    System,
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    /// Message role
    pub role: MessageRole,
    /// Message content
    pub content: Content,
    /// Tool calls (for AI messages)
    pub tool_calls: Vec<ToolCall>,
    /// Tool call ID (for tool messages)
    pub tool_call_id: Option<String>,
    /// Additional metadata
    pub metadata: Option<MessageMetadata>,
    /// Token usage (for AI messages)
    pub usage: Option<TokenUsage>,
}

/// Tool call in an AI message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    /// Tool call ID
    pub id: String,
    /// Tool name
    pub name: String,
    /// Tool arguments (JSON)
    pub arguments: serde_json::Value,
}

/// Message metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageMetadata {
    /// Message timestamp
    pub timestamp: Option<i64>,
    /// Additional custom data
    #[serde(flatten)]
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

impl Message {
    /// Create a new human message
    pub fn human(content: impl Into<Content>) -> Self {
        Self {
            role: MessageRole::Human,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            metadata: None,
            usage: None,
        }
    }

    /// Create a new system message
    pub fn system(content: impl Into<Content>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            metadata: None,
            usage: None,
        }
    }

    /// Create a new tool message
    pub fn tool(
        tool_call_id: impl Into<String>,
        _tool_name: impl Into<String>,
        content: impl Into<Content>,
    ) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: Some(tool_call_id.into()),
            metadata: None,
            usage: None,
        }
    }

    /// Check if this is a human message
    pub fn is_human(&self) -> bool {
        self.role == MessageRole::Human
    }

    /// Check if this is an AI message
    pub fn is_ai(&self) -> bool {
        self.role == MessageRole::Ai
    }

    /// Check if this is a tool message
    pub fn is_tool(&self) -> bool {
        self.role == MessageRole::Tool
    }
}

/// Model request
#[derive(Debug, Clone)]
pub struct ModelRequest {
    /// Messages to send to the model
    pub messages: Vec<Message>,
    /// Temperature (0.0 to 1.0)
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Top-p sampling
    pub top_p: Option<f32>,
    /// Top-k sampling
    pub top_k: Option<u32>,
    /// Stop sequences
    pub stop: Vec<String>,
}

/// Model response
#[derive(Debug, Clone)]
pub struct ModelResponse {
    /// Response message
    pub message: Message,
    /// Token usage (if available)
    pub usage: Option<TokenUsage>,
    /// Finish reason
    pub finish_reason: Option<String>,
}

/// Stream event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum StreamEvent {
    /// Token delta
    TokenDelta { delta: String, index: u32 },
    /// Message complete
    MessageComplete { message: Message },
    /// Tool call
    ToolCall { tool_calls: Vec<ToolCall> },
    /// End of stream
    End,
    /// Error
    Error { error: String },
}

/// Chat model trait
///
/// This trait defines the interface for LLM model providers.
#[async_trait]
pub trait ChatModel: Send + Sync {
    /// Get model name
    fn name(&self) -> &str;

    /// Get model capabilities
    fn capabilities(&self) -> &ModelCapabilities;

    /// Invoke the model synchronously
    async fn invoke(&self, request: ModelRequest) -> Result<ModelResponse>;

    /// Stream model responses
    async fn stream(
        &self,
        request: ModelRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>>;

    /// Check if model supports a specific feature
    fn supports_thinking(&self) -> bool {
        self.capabilities().thinking
    }

    /// Check if model supports vision
    fn supports_vision(&self) -> bool {
        self.capabilities().vision
    }

    /// Get token usage from last invocation (if available)
    fn get_usage(&self) -> Option<TokenUsage> {
        None
    }
}
