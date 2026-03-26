//! Model system
//!
//! This module provides abstractions for LLM model providers
//! and a factory for creating model instances.

pub mod base;
pub mod factory;
pub mod providers;

// Re-export common types
pub use base::{ChatModel, ModelCapabilities, ModelRequest, ModelResponse, StreamEvent};
pub use factory::ModelFactory;

// Re-export provider implementations
pub use providers::{AnthropicModel, DeepSeekModel, OpenAIModel};
