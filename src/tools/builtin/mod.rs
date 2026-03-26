//! Built-in tools
//!
//! This module contains the standard built-in tools for the agent-harness system.

pub mod bash;
pub mod ls;
pub mod present_files;
pub mod read_file;
pub mod str_replace;
pub mod view_image;
pub mod write_file;

// Re-export built-in tools
pub use bash::BashTool;
pub use ls::LsTool;
pub use present_files::PresentFilesTool;
pub use read_file::ReadFileTool;
pub use str_replace::StrReplaceTool;
pub use view_image::ViewImageTool;
pub use write_file::WriteFileTool;

/// Register all built-in tools to the given registry
pub fn register_builtins(registry: &crate::tools::ToolRegistry) -> crate::Result<()> {
    use std::sync::Arc;

    registry.register(Arc::new(BashTool::new()))?;
    registry.register(Arc::new(LsTool::new()))?;
    registry.register(Arc::new(ReadFileTool::new()))?;
    registry.register(Arc::new(WriteFileTool::new()))?;
    registry.register(Arc::new(StrReplaceTool::new()))?;
    registry.register(Arc::new(ViewImageTool::new()))?;
    registry.register(Arc::new(PresentFilesTool::new()))?;

    Ok(())
}
