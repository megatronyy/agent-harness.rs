//! Agent state structures

use crate::models::base::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Base agent state
///
/// This is the minimal state structure that will be expanded
/// as we implement more features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    /// Messages in the conversation
    pub messages: Vec<Message>,
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}

/// Thread-specific state
///
/// Extends AgentState with thread-specific information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadState {
    /// Messages in the conversation (flattened for convenience)
    pub messages: Vec<Message>,

    /// Thread identifier
    pub thread_id: Option<String>,

    /// Sandbox identifier
    pub sandbox_id: Option<String>,

    /// Thread title
    pub title: Option<String>,

    /// Artifacts produced
    pub artifacts: Vec<String>,

    /// Viewed images
    pub viewed_images: HashMap<String, ImageData>,
}

/// Image data for vision models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    /// MIME type
    pub mime_type: String,

    /// Base64 encoded data
    pub base64: String,
}

impl Default for ThreadState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            thread_id: None,
            sandbox_id: None,
            title: None,
            artifacts: Vec::new(),
            viewed_images: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_state_default() {
        let state = AgentState::default();
        assert!(state.messages.is_empty());
    }

    #[test]
    fn test_thread_state_default() {
        let state = ThreadState::default();
        assert!(state.messages.is_empty());
        assert!(state.thread_id.is_none());
        assert!(state.sandbox_id.is_none());
        assert!(state.title.is_none());
        assert!(state.artifacts.is_empty());
        assert!(state.viewed_images.is_empty());
    }
}
