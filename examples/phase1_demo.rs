//! Simple example demonstrating Phase 1 features
//!
//! This example shows how to use the basic agent-harness components:
//! - Configuration system
//! - Runtime context
//! - Content types
//! - Error handling

use agent_harness::{
    config::AppConfig,
    runtime::{RuntimeContext, RuntimeMetadata},
    Content,
};

fn main() -> agent_harness::Result<()> {
    println!("=== agent-harness Phase 1 Example ===\n");

    // 1. Configuration
    println!("1. Configuration System:");
    let config = AppConfig::default();
    println!("   Default models: {}", config.models.is_empty());
    println!("   Sandbox provider: {}", config.sandbox.provider);
    println!("   Memory enabled: {}\n", config.memory.enabled);

    // 2. Runtime Context
    println!("2. Runtime Context:");
    let ctx = RuntimeContext::with_thread_id("example-thread-123")
        .with_agent_name("example-agent")
        .with_metadata(
            RuntimeMetadata::new()
                .with_model_name("claude-opus-4-6")
                .with_thinking_enabled(true)
                .with_plan_mode(false)
        )
        .with_extra("environment", "development");
    println!("   Thread ID: {}", ctx.require_thread_id());
    println!("   Agent: {}", ctx.agent_name.as_deref().unwrap());
    println!("   Model: {}", ctx.metadata.model_name.as_deref().unwrap());
    println!("   Extra: {:#?}\n", ctx.extra);

    // 3. Content Types
    println!("3. Content Types:");
    let text = Content::text("Hello, agent-harness!");
    println!("   Text content: {}", text.to_string());

    let image = Content::image_base64("image/png", "iVBORw0KGgo...");
    println!("   Image: {}", image.is_empty());

    let blocks: Vec<String> = vec!["First line".into(), "Second line".into()];
    let mixed = Content::from(blocks);
    println!("   Mixed content: {}\n", mixed.is_empty());

    // 4. Error Handling
    println!("4. Error Handling:");
    use agent_harness::error::{HarnessError, ModelError};

    let err = HarnessError::Model(ModelError::NotFound("test-model".to_string()));
    println!("   Error: {}", err);
    println!("   Category: {}", err.category());
    println!("   Retryable: {}", err.is_retryable());

    println!("\n=== All Phase 1 features working! ===");
    Ok(())
}
