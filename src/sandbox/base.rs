//! Sandbox base traits and types
//!
//! This module provides the core Sandbox trait for isolated execution.

use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Result of executing a command in a sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxCommandResult {
    /// Exit code (0 for success)
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Whether the command timed out
    pub timed_out: bool,
}

impl SandboxCommandResult {
    /// Create a new successful command result
    pub fn success(stdout: String) -> Self {
        Self {
            exit_code: 0,
            stdout,
            stderr: String::new(),
            timed_out: false,
        }
    }

    /// Create a new failed command result
    pub fn failure(exit_code: i32, stdout: String, stderr: String) -> Self {
        Self {
            exit_code,
            stdout,
            stderr,
            timed_out: false,
        }
    }

    /// Create a new timeout result
    pub fn timeout(stdout: String, stderr: String) -> Self {
        Self {
            exit_code: -1,
            stdout,
            stderr,
            timed_out: true,
        }
    }

    /// Check if the command succeeded
    pub fn is_success(&self) -> bool {
        self.exit_code == 0 && !self.timed_out
    }
}

/// Result of reading a file in a sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxFileResult {
    /// File content
    pub content: String,
    /// File path
    pub path: PathBuf,
    /// Whether the file exists
    pub exists: bool,
}

/// Result of listing a directory in a sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxDirEntry {
    /// Entry name
    pub name: String,
    /// Full path
    pub path: PathBuf,
    /// Whether it's a directory
    pub is_dir: bool,
    /// File size in bytes (0 for directories)
    pub size: u64,
}

/// Sandbox trait for isolated execution
///
/// A sandbox provides an isolated environment for executing commands
/// and performing file operations.
#[async_trait::async_trait]
pub trait Sandbox: Send + Sync {
    /// Get the unique identifier for this sandbox instance
    fn id(&self) -> &str;

    /// Execute a command in the sandbox
    ///
    /// # Arguments
    /// * `command` - The command to execute (e.g., "ls -la")
    /// * `cwd` - Optional working directory relative to sandbox root
    /// * `timeout_secs` - Optional timeout in seconds (default: 30)
    async fn execute_command(
        &self,
        command: &str,
        cwd: Option<&str>,
        timeout_secs: Option<u64>,
    ) -> Result<SandboxCommandResult>;

    /// Read a file from the sandbox
    ///
    /// # Arguments
    /// * `path` - Path relative to sandbox root
    async fn read_file(&self, path: &str) -> Result<SandboxFileResult>;

    /// Write content to a file in the sandbox
    ///
    /// # Arguments
    /// * `path` - Path relative to sandbox root
    /// * `content` - Content to write
    /// * `append` - Whether to append to existing file
    async fn write_file(&self, path: &str, content: &str, append: bool) -> Result<()>;

    /// List a directory in the sandbox
    ///
    /// # Arguments
    /// * `path` - Path relative to sandbox root
    /// * `recursive` - Whether to list recursively
    /// * `max_depth` - Maximum recursion depth (0 = unlimited)
    async fn list_dir(
        &self,
        path: &str,
        recursive: bool,
        max_depth: usize,
    ) -> Result<Vec<SandboxDirEntry>>;

    /// Check if a path exists in the sandbox
    ///
    /// # Arguments
    /// * `path` - Path relative to sandbox root
    async fn exists(&self, path: &str) -> Result<bool>;

    /// Delete a file or directory in the sandbox
    ///
    /// # Arguments
    /// * `path` - Path relative to sandbox root
    /// * `recursive` - Whether to recursively delete directories
    async fn delete(&self, path: &str, recursive: bool) -> Result<()>;

    /// Create a directory in the sandbox
    ///
    /// # Arguments
    /// * `path` - Path relative to sandbox root
    /// * `parents` - Whether to create parent directories
    async fn create_dir(&self, path: &str, parents: bool) -> Result<()>;

    /// Get the real (physical) path for a virtual sandbox path
    ///
    /// This translates sandbox virtual paths to host filesystem paths.
    ///
    /// # Arguments
    /// * `virtual_path` - Virtual path within the sandbox
    fn real_path(&self, virtual_path: &str) -> PathBuf;

    /// Get the virtual path for a real (physical) path
    ///
    /// This translates host filesystem paths to sandbox virtual paths.
    ///
    /// # Arguments
    /// * `real_path` - Real path on the host filesystem
    fn virtual_path(&self, real_path: &PathBuf) -> String;

    /// Clean up resources used by the sandbox
    async fn cleanup(&self) -> Result<()>;
}
