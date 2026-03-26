//! Model configuration

// Re-export from app module
pub use super::app::ModelConfig;

/// Model capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModelCapabilities {
    /// Supports extended thinking
    pub thinking: bool,
    /// Supports vision/image input
    pub vision: bool,
    /// Supports function calling
    pub function_calling: bool,
    /// Supports streaming
    pub streaming: bool,
}

impl ModelCapabilities {
    /// Create new capabilities
    pub fn new() -> Self {
        Self {
            thinking: false,
            vision: false,
            function_calling: true,
            streaming: true,
        }
    }

    /// Builder pattern for setting thinking capability
    pub fn with_thinking(mut self) -> Self {
        self.thinking = true;
        self
    }

    /// Builder pattern for setting vision capability
    pub fn with_vision(mut self) -> Self {
        self.vision = true;
        self
    }

    /// Check if model has thinking capability
    pub fn supports_thinking(&self) -> bool {
        self.thinking
    }

    /// Check if model has vision capability
    pub fn supports_vision(&self) -> bool {
        self.vision
    }
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_default() {
        let caps = ModelCapabilities::default();
        assert!(!caps.thinking);
        assert!(!caps.vision);
        assert!(caps.function_calling);
        assert!(caps.streaming);
    }

    #[test]
    fn test_capabilities_builder() {
        let caps = ModelCapabilities::new()
            .with_thinking()
            .with_vision();
        assert!(caps.thinking);
        assert!(caps.vision);
    }
}
