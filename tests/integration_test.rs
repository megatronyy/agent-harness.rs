//! Integration tests for Phase 1 components
//!
//! These tests verify the complete workflow from configuration loading
//! to state initialization.

use agent_harness::{
    config::AppConfig,
    runtime::{RuntimeContext, RuntimeMetadata},
};

#[test]
fn test_config_loading_workflow() {
    // This test verifies that configuration loading works end-to-end
    // Since we don't have a real config file, we test the default config
    let config = AppConfig::default();
    assert!(config.models.is_empty());
    assert_eq!(config.sandbox.provider, "local");
}

#[test]
fn test_runtime_context_workflow() {
    // Test building a complete runtime context
    let ctx = RuntimeContext::with_thread_id("test-thread-123")
        .with_agent_name("test-agent")
        .with_metadata(
            RuntimeMetadata::new()
                .with_model_name("claude-opus-4-6")
                .with_thinking_enabled(true)
                .with_plan_mode(false)
        )
        .with_extra("custom_key", "custom_value");

    assert_eq!(ctx.thread_id.as_deref(), Some("test-thread-123"));
    assert_eq!(ctx.agent_name.as_deref(), Some("test-agent"));
    assert_eq!(ctx.metadata.model_name.as_deref(), Some("claude-opus-4-6"));
    assert_eq!(ctx.metadata.thinking_enabled, Some(true));
    assert_eq!(ctx.metadata.is_plan_mode, Some(false));
    assert_eq!(ctx.extra.get("custom_key"), Some(&"custom_value".to_string()));
}

#[test]
fn test_error_handling_chain() {
    // Test that errors propagate correctly through the chain
    use agent_harness::error::{HarnessError, ModelError};

    // Model error
    let err = HarnessError::Model(ModelError::NotFound("test-model".to_string()));
    assert!(err.to_string().contains("test-model"));
    assert_eq!(err.category(), agent_harness::error::ErrorCategory::Model);

    // Check retryable
    assert!(!err.is_retryable());

    // Retryable error
    let retryable_err = HarnessError::Model(ModelError::RateLimited(60));
    assert!(retryable_err.is_retryable());
}

#[test]
fn test_content_type_integration() {
    // Test Content type usage in realistic scenarios
    use agent_harness::Content;

    // Text content
    let text = Content::text("Hello, world!");
    assert!(!text.is_empty());
    assert_eq!(text.to_string(), "Hello, world!");

    // Mixed content (multiple text blocks)
    let blocks: Vec<String> = vec!["First".to_string(), "Second".to_string()];
    let mixed = Content::from(blocks);
    assert!(!mixed.is_empty());

    // Image content
    let image = Content::image_base64("image/png", "iVBORw0KGgo...");
    assert!(!image.is_empty());
}

#[test]
fn test_state_management_integration() {
    // Test state structures work together
    use agent_harness::agents::state::{AgentState, ThreadState};

    let agent_state = AgentState::default();
    assert!(agent_state.messages.is_empty());

    let thread_state = ThreadState::default();
    assert!(thread_state.thread_id.is_none());
    assert!(thread_state.sandbox_id.is_none());
    assert!(thread_state.artifacts.is_empty());
}

#[test]
fn test_configuration_structure() {
    // Test that configuration structures are properly organized
    use agent_harness::config::{app::MemoryConfig, app::ModelConfig, app::SandboxConfig, app::SkillsConfig};

    let model_config = ModelConfig {
        name: "claude-opus-4-6".to_string(),
        provider: "langchain_anthropic:ChatAnthropic".to_string(),
        supports_thinking: true,
        supports_vision: true,
        config: None,
    };
    assert!(model_config.supports_thinking);
    assert!(model_config.supports_vision);

    let sandbox_config = SandboxConfig::default();
    assert_eq!(sandbox_config.provider, "local");

    let skills_config = SkillsConfig::default();
    assert_eq!(skills_config.path.as_deref(), Some("skills"));

    let memory_config = MemoryConfig::default();
    assert!(!memory_config.enabled);
    assert_eq!(memory_config.debounce_seconds, 30);
}
