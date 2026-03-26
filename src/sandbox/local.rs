//! Local sandbox provider
//!
//! This module provides a local filesystem sandbox implementation.

use crate::{
    error::SandboxError,
    sandbox::base::{Sandbox, SandboxCommandResult, SandboxDirEntry, SandboxFileResult},
    Result,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::{fs, process::Command, time::timeout};

/// Local sandbox ID
pub const LOCAL_SANDBOX_ID: &str = "local";

/// Virtual path mappings for local sandbox
///
/// Maps virtual paths (seen by agent) to real paths (on host filesystem).
#[derive(Debug, Clone)]
pub struct VirtualPathMapping {
    /// Virtual path prefix
    pub virtual_prefix: String,
    /// Real path on host
    pub real_path: PathBuf,
}

impl VirtualPathMapping {
    /// Create a new virtual path mapping
    pub fn new(virtual_prefix: impl Into<String>, real_path: impl Into<PathBuf>) -> Self {
        Self {
            virtual_prefix: virtual_prefix.into(),
            real_path: real_path.into(),
        }
    }
}

/// Local sandbox implementation
///
/// This sandbox executes commands directly on the host filesystem
/// with path translation for virtual paths.
pub struct LocalSandbox {
    /// Unique identifier (always "local")
    id: String,
    /// Virtual path mappings
    mappings: Vec<VirtualPathMapping>,
    /// Root directory for this sandbox
    root: PathBuf,
}

impl LocalSandbox {
    /// Create a new local sandbox
    ///
    /// # Arguments
    /// * `root` - Root directory for the sandbox
    /// * `mappings` - Optional virtual path mappings
    pub fn new(root: impl Into<PathBuf>, mappings: Option<Vec<VirtualPathMapping>>) -> Self {
        Self {
            id: LOCAL_SANDBOX_ID.to_string(),
            mappings: mappings.unwrap_or_default(),
            root: root.into(),
        }
    }

    /// Create a new local sandbox with default mappings
    ///
    /// # Arguments
    /// * `thread_id` - Thread identifier for creating thread-specific directories
    /// * `base_dir` - Base directory for thread data
    pub fn with_thread_dir(thread_id: &str, base_dir: impl Into<PathBuf>) -> Self {
        let base = base_dir.into();
        let thread_root = base.join(thread_id);

        // Create default virtual path mappings
        let mappings = vec![
            VirtualPathMapping::new("/mnt/user-data", thread_root.join("user-data")),
            VirtualPathMapping::new("/mnt/skills", base.join("../skills")),
        ];

        Self {
            id: format!("local:{}", thread_id),
            mappings,
            root: thread_root,
        }
    }

    /// Translate a virtual path to a real path
    fn translate_virtual(&self, virtual_path: &str) -> PathBuf {
        // Check if path matches any virtual mapping
        for mapping in &self.mappings {
            if virtual_path.starts_with(&mapping.virtual_prefix) {
                let suffix = virtual_path[mapping.virtual_prefix.len()..].trim_start_matches('/');
                return mapping.real_path.join(suffix);
            }
        }

        // No mapping match, return path relative to root
        self.root.join(virtual_path.trim_start_matches('/'))
    }

    /// Translate a real path back to virtual
    fn translate_real(&self, real_path: &PathBuf) -> String {
        // Check if path matches any real mapping
        for mapping in &self.mappings {
            if let Ok(suffix) = real_path.strip_prefix(&mapping.real_path) {
                if suffix.as_os_str().is_empty() {
                    return mapping.virtual_prefix.clone();
                }
                return format!("{}/{}", mapping.virtual_prefix, suffix.to_string_lossy());
            }
        }

        // No mapping match
        real_path.to_string_lossy().to_string()
    }
}

#[async_trait::async_trait]
impl Sandbox for LocalSandbox {
    fn id(&self) -> &str {
        &self.id
    }

    async fn execute_command(
        &self,
        command: &str,
        cwd: Option<&str>,
        timeout_secs: Option<u64>,
    ) -> Result<SandboxCommandResult> {
        // Translate command to use real paths
        let translated = self.translate_command_paths(command);

        // Determine working directory
        let work_dir = if let Some(cwd) = cwd {
            self.translate_virtual(cwd)
        } else {
            self.root.clone()
        };

        // Create the command
        let timeout_duration = Duration::from_secs(timeout_secs.unwrap_or(30));

        // Execute with timeout
        let result = timeout(timeout_duration, async {
            let output = if cfg!(windows) {
                Command::new("cmd")
                    .args(["/C", &translated])
                    .current_dir(&work_dir)
                    .output()
                    .await
            } else {
                Command::new("sh")
                    .args(["-c", &translated])
                    .current_dir(&work_dir)
                    .output()
                    .await
            };

            output.map_err(|e| {
                crate::error::HarnessError::Sandbox(SandboxError::CommandFailed(format!(
                    "Failed to execute command: {}",
                    e
                )))
            })
        })
        .await;

        match result {
            Ok(Ok(output)) => Ok(SandboxCommandResult {
                exit_code: output.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                timed_out: false,
            }),
            Ok(Err(e)) => Err(e),
            Err(_) => Ok(SandboxCommandResult {
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Command timed out after {:?}", timeout_duration),
                timed_out: true,
            }),
        }
    }

    async fn read_file(&self, path: &str) -> Result<SandboxFileResult> {
        let real_path = self.translate_virtual(path);

        let content = match fs::read_to_string(&real_path).await {
            Ok(content) => content,
            Err(_) => String::new(),
        };

        let exists = fs::try_exists(&real_path).await.unwrap_or(false);

        Ok(SandboxFileResult {
            content,
            path: real_path,
            exists,
        })
    }

    async fn write_file(&self, path: &str, content: &str, append: bool) -> Result<()> {
        let real_path = self.translate_virtual(path);

        // Create parent directories if they don't exist
        if let Some(parent) = real_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                crate::error::HarnessError::Sandbox(SandboxError::InitializationFailed(format!(
                    "Failed to create directory: {}",
                    e
                )))
            })?;
        }

        if append {
            fs::write(&real_path, content).await.map_err(|e| {
                crate::error::HarnessError::Sandbox(SandboxError::CommandFailed(format!(
                    "Failed to write file: {}",
                    e
                )))
            })?;
        } else {
            fs::write(&real_path, content).await.map_err(|e| {
                crate::error::HarnessError::Sandbox(SandboxError::CommandFailed(format!(
                    "Failed to write file: {}",
                    e
                )))
            })?;
        }

        Ok(())
    }

    async fn list_dir(
        &self,
        path: &str,
        recursive: bool,
        max_depth: usize,
    ) -> Result<Vec<SandboxDirEntry>> {
        let real_path = self.translate_virtual(path);

        if !real_path.exists() {
            return Ok(vec![]);
        }

        let mut entries = Vec::new();
        self.list_dir_iterative(&real_path, &mut entries, recursive, max_depth)
            .await?;

        Ok(entries)
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let real_path = self.translate_virtual(path);
        Ok(fs::try_exists(&real_path).await.unwrap_or(false))
    }

    async fn delete(&self, path: &str, recursive: bool) -> Result<()> {
        let real_path = self.translate_virtual(path);

        if recursive {
            fs::remove_dir_all(&real_path).await.map_err(|e| {
                crate::error::HarnessError::Sandbox(SandboxError::CleanupFailed(format!(
                    "Failed to delete directory: {}",
                    e
                )))
            })?;
        } else {
            // Try file removal first
            let file_result = fs::remove_file(&real_path).await;
            if file_result.is_err() {
                // If file removal fails, try directory removal
                fs::remove_dir(&real_path).await.map_err(|e2| {
                    crate::error::HarnessError::Sandbox(SandboxError::CleanupFailed(format!(
                        "Failed to delete: {} / {:?}",
                        file_result.unwrap_err(), e2
                    )))
                })?;
            }
        }

        Ok(())
    }

    async fn create_dir(&self, path: &str, parents: bool) -> Result<()> {
        let real_path = self.translate_virtual(path);

        if parents {
            fs::create_dir_all(&real_path).await.map_err(|e| {
                crate::error::HarnessError::Sandbox(SandboxError::InitializationFailed(format!(
                    "Failed to create directory: {}",
                    e
                )))
            })?;
        } else {
            fs::create_dir(&real_path).await.map_err(|e| {
                crate::error::HarnessError::Sandbox(SandboxError::InitializationFailed(format!(
                    "Failed to create directory: {}",
                    e
                )))
            })?;
        }

        Ok(())
    }

    fn real_path(&self, virtual_path: &str) -> PathBuf {
        self.translate_virtual(virtual_path)
    }

    fn virtual_path(&self, real_path: &PathBuf) -> String {
        self.translate_real(real_path)
    }

    async fn cleanup(&self) -> Result<()> {
        // Local sandbox doesn't need cleanup
        Ok(())
    }
}

impl LocalSandbox {
    /// Translate virtual paths in a command to real paths
    fn translate_command_paths(&self, command: &str) -> String {
        let mut translated = command.to_string();

        // Sort mappings by length (longest first) to avoid partial matches
        let mut sorted_mappings = self.mappings.clone();
        sorted_mappings.sort_by(|a, b| b.virtual_prefix.len().cmp(&a.virtual_prefix.len()));

        // Replace virtual paths with real paths
        for mapping in &sorted_mappings {
            translated = translated.replace(
                &mapping.virtual_prefix,
                &mapping.real_path.to_string_lossy(),
            );
        }

        translated
    }

    /// Iterative directory listing helper
    async fn list_dir_iterative(
        &self,
        root_path: &Path,
        entries: &mut Vec<SandboxDirEntry>,
        recursive: bool,
        max_depth: usize,
    ) -> Result<()> {
        use std::collections::VecDeque;

        // Stack of (path, depth) pairs to process
        let mut stack: VecDeque<(PathBuf, usize)> = VecDeque::new();
        stack.push_back((root_path.to_path_buf(), 0));

        while let Some((path, depth)) = stack.pop_front() {
            // Check depth limit
            if max_depth > 0 && depth >= max_depth {
                continue;
            }

            let mut dir = match fs::read_dir(&path).await {
                Ok(d) => d,
                Err(_) => continue, // Skip directories we can't read
            };

            while let Some(entry) = dir.next_entry().await.map_err(|e| {
                crate::error::HarnessError::Sandbox(SandboxError::InitializationFailed(format!(
                    "Failed to read directory entry: {}",
                    e
                )))
            })? {
                let entry_path = entry.path();
                let name = entry
                    .file_name()
                    .to_string_lossy()
                    .to_string();

                // Skip hidden directories
                if name.starts_with('.') && entry_path.is_dir() {
                    continue;
                }

                let is_dir = match entry.file_type().await {
                    Ok(ft) => ft.is_dir(),
                    Err(_) => false,
                };

                let metadata = match entry.metadata().await {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                entries.push(SandboxDirEntry {
                    name: name.clone(),
                    path: entry_path.clone(),
                    is_dir,
                    size: metadata.len(),
                });

                // Add subdirectories to stack for recursive listing
                if recursive && is_dir && !name.starts_with('.') {
                    stack.push_back((entry_path, depth + 1));
                }
            }
        }

        Ok(())
    }
}

/// Local sandbox provider
///
/// Provides local sandbox instances.
pub struct LocalSandboxProvider {
    /// Base directory for all sandboxes
    base_dir: PathBuf,
}

impl LocalSandboxProvider {
    /// Create a new local sandbox provider
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }
}

#[async_trait::async_trait]
impl crate::sandbox::provider::SandboxProvider for LocalSandboxProvider {
    async fn acquire(&self, thread_id: &str) -> Result<Arc<dyn Sandbox>> {
        Ok(Arc::new(LocalSandbox::with_thread_dir(
            thread_id,
            &self.base_dir,
        )))
    }

    async fn release(&self, _sandbox_id: &str) -> Result<()> {
        // Local sandboxes don't need explicit release
        Ok(())
    }

    async fn get(&self, _sandbox_id: &str) -> Result<Option<Arc<dyn Sandbox>>> {
        // For local sandbox, always return a new instance
        Ok(None)
    }

    async fn cleanup_all(&self) -> Result<()> {
        // Clean up all thread directories
        let mut entries = fs::read_dir(&self.base_dir).await.map_err(|e| {
            crate::error::HarnessError::Sandbox(SandboxError::CleanupFailed(format!(
                "Failed to read base directory: {}",
                e
            )))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            crate::error::HarnessError::Sandbox(SandboxError::CleanupFailed(format!(
                "Failed to read directory entry: {}",
                e
            )))
        })? {
            if entry.file_type().await.map_err(|e| {
                crate::error::HarnessError::Sandbox(SandboxError::CleanupFailed(format!(
                    "Failed to get file type: {}",
                    e
                )))
            })?
            .is_dir()
            {
                fs::remove_dir_all(entry.path()).await.map_err(|e| {
                    crate::error::HarnessError::Sandbox(SandboxError::CleanupFailed(format!(
                        "Failed to remove directory: {}",
                        e
                    )))
                })?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_sandbox_id() {
        let sandbox = LocalSandbox::new("/tmp/test", None);
        assert_eq!(sandbox.id(), LOCAL_SANDBOX_ID);
    }

    #[test]
    fn test_virtual_path_mapping() {
        let mapping = VirtualPathMapping::new("/mnt/data", "/real/data");
        assert_eq!(mapping.virtual_prefix, "/mnt/data");
        assert_eq!(mapping.real_path, PathBuf::from("/real/data"));
    }

    #[test]
    fn test_local_sandbox_new() {
        let sandbox = LocalSandbox::new("/tmp/test", None);
        assert_eq!(sandbox.root, PathBuf::from("/tmp/test"));
        assert!(sandbox.mappings.is_empty());
    }

    #[test]
    fn test_translate_virtual_no_mapping() {
        let sandbox = LocalSandbox::new("/tmp/test", None);
        let real = sandbox.translate_virtual("subdir/file.txt");
        assert_eq!(real, PathBuf::from("/tmp/test/subdir/file.txt"));
    }

    #[test]
    fn test_translate_virtual_with_mapping() {
        let mappings = vec![VirtualPathMapping::new("/mnt/data", "/real/data")];
        let sandbox = LocalSandbox::new("/tmp/test", Some(mappings));
        let real = sandbox.translate_virtual("/mnt/data/file.txt");
        assert_eq!(real, PathBuf::from("/real/data/file.txt"));
    }
}
