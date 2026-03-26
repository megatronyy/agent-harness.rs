# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Building
```bash
cargo build              # Debug build
cargo build --release    # Optimized release build
```

### Running
```bash
cargo run --example phase1_demo    # Run Phase 1 demo
```

### Testing
```bash
cargo test                           # Run all tests
cargo test --test integration_test   # Run integration tests only
cargo test --lib                     # Run unit tests only
cargo test -- --nocapture            # Show print output in tests
```

### Development
```bash
cargo check                           # Quick compile check
cargo clippy                          # Linter checks
cargo fmt                             # Format code
cargo doc --open                      # Generate and open documentation
```

### Feature Flags
```bash
cargo build --features anthropic      # Enable Anthropic provider
cargo build --features openai         # Enable OpenAI provider
cargo build --features docker         # Enable Docker sandbox support
```

## Architecture Overview

This is a **Rust reimplementation of the DeerFlow Harness** AI agent framework. The architecture is trait-based with async/await throughout, designed for high-performance AI agent execution.

### Core Architecture Layers

1. **Models** (`src/models/`) - LLM provider abstraction
   - `ChatModel` trait defines the interface for all LLM providers
   - `ModelFactory` creates model instances from config
   - Providers: Anthropic, OpenAI, DeepSeek (each in `models/providers/`)

2. **Agents** (`src/agents/`) - Agent execution engine
   - `AgentExecutor` - Main agent loop with tool calling
   - `AgentBuilder` - Fluent builder API for constructing agents
   - `AgentState` / `ThreadState` - Conversation state management

3. **Middleware** (`src/agents/middleware/`) - Interception hooks
   - `Middleware` trait with hook points: BeforeModel, AfterModel, BeforeTool, AfterTool
   - `MiddlewareChain` - Ordered execution of middlewares
   - Built-in middlewares in `middlewares/`: ThreadData, DanglingToolCall, Title

4. **Tools** (`src/tools/`) - Function calling system
   - `Tool` trait with `schema()` and `execute()` methods
   - `ToolRegistry` - Tool discovery and registration
   - `ToolExecutor` - Executes tools with JSON Schema validation
   - Built-in tools in `builtin/`: bash, read_file, write_file, str_replace, ls, view_image

5. **Sandbox** (`src/sandbox/`) - Isolated execution environments
   - `Sandbox` trait for isolation abstraction
   - `SandboxProvider` manages sandbox lifecycle
   - Local implementation for path-based isolation

6. **Messages** (`src/messages/`) - Type-safe message system
   - `Content` enum for text/image/mixed content
   - Message types: HumanMessage, AIMessage, ToolMessage

7. **Configuration** (`src/config/`) - YAML-based configuration
   - `AppConfig` - Top-level application configuration
   - Environment variable expansion support
   - Hot-reload via file watching (notify crate)

### Key Traits

```rust
// Model interface - all LLM providers implement this
#[async_trait]
pub trait ChatModel: Send + Sync {
    fn name(&self) -> &str;
    fn capabilities(&self) -> &ModelCapabilities;
    async fn invoke(&self, request: ModelRequest) -> Result<ModelResponse>;
    async fn stream(&self, request: ModelRequest) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>>;
}

// Tool interface - all tools implement this
#[async_trait]
pub trait Tool: Send + Sync {
    fn schema(&self) -> &ToolSchema;
    async fn execute(&self, args: &ToolArgs, context: &ToolContext) -> ToolResult;
    fn validate_args(&self, args: &ToolArgs) -> Result<()> { /* default impl */ }
}

// Middleware interface - all middlewares implement this
#[async_trait]
pub trait Middleware: Send + Sync {
    fn name(&self) -> &str;
    fn hooks(&self) -> &[MiddlewareHook];  // Which hooks to respond to
    async fn execute(&self, context: &mut MiddlewareContext) -> Result<()>;
}
```

## Error Handling Pattern

The codebase uses structured errors with `thiserror`:
- `HarnessError` is the top-level error enum
- Category-based sub-errors: `ModelError`, `ToolError`, `ConfigError`
- Use `?` operator for propagation
- Check `error.is_retryable()` before retrying operations

## Async/Await Patterns

- Uses `tokio` as async runtime
- `async-trait` macro for trait methods (Rust doesn't have native async traits yet)
- `Pin<Box<dyn Stream + Send>>` for streaming responses
- Use `.await` on all async operations

## Configuration Format

YAML configuration with environment variable substitution:
```yaml
models:
  - name: claude-opus-4-6
    provider: langchain_anthropic:ChatAnthropic
    supports_thinking: true
    config:
      anthropic_api_key: ${ANTHROPIC_API_KEY}

sandbox:
  provider: local  # or "docker"

memory:
  enabled: true
  debounce_seconds: 30
```

## Testing Conventions

- Unit tests in module-level `#[cfg(test)]` mod blocks
- Integration tests in `tests/` directory
- Test naming: `test_<component>_<behavior>`
- Use `cargo test -- --nocapture` for debugging

## Phase-Based Development

This project follows a phased implementation plan (see `docs/agent-harness-rs-plan.md`):
- **Phase 1**: Basic framework (error types, messages, config, runtime)
- **Phase 2**: Model system
- **Phase 3**: Tool system
- **Phase 4**: Middleware system
- **Phase 5**: Agent executor
- **Phase 6**: Sandbox system
- **Phase 7**: MCP integration
- **Phase 8**: Subagent system
- **Phase 9**: Memory system
- **Phase 10**: Skills system
- **Phase 11**: Guardrails
- **Phase 12**: Python bindings (PyO3)

Currently at Phase 1-2 level of implementation.
