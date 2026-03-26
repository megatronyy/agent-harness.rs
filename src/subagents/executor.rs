//! Subagent executor
//!
//! This module provides the subagent executor for running subagent tasks.

use crate::{error::HarnessError, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Subagent task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentTask {
    /// Unique task ID
    pub id: String,

    /// Agent ID to use
    pub agent_id: String,

    /// Task description/prompt
    pub prompt: String,

    /// Task status
    #[serde(skip)]
    pub status: TaskStatus,

    /// Task result
    #[serde(skip)]
    pub result: Option<String>,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TaskStatus {
    /// Task is pending
    #[default]
    Pending,
    /// Task is running
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task timed out
    TimedOut,
}

/// Subagent executor for running subagent tasks
pub struct SubagentExecutor {
    /// Agent registry
    registry: Arc<crate::subagents::registry::AgentRegistry>,

    /// Running tasks
    tasks: Arc<RwLock<std::collections::HashMap<String, SubagentTask>>>,

    /// Maximum concurrent tasks
    max_concurrent: usize,

    /// Current running task count
    running_count: Arc<Mutex<usize>>,
}

impl SubagentExecutor {
    /// Create a new subagent executor
    pub fn new(
        registry: Arc<crate::subagents::registry::AgentRegistry>,
        max_concurrent: usize,
    ) -> Self {
        Self {
            registry,
            tasks: Arc::new(RwLock::new(std::collections::HashMap::new())),
            max_concurrent,
            running_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Execute a subagent task
    pub async fn execute(
        &self,
        agent_id: &str,
        prompt: impl Into<String>,
    ) -> Result<String> {
        // Check if agent exists
        let agent = self
            .registry
            .get(agent_id)
            .ok_or_else(|| HarnessError::other(format!("Agent not found: {}", agent_id)))?;

        // Check concurrent limit
        {
            let mut count = self.running_count.lock().await;
            if *count >= self.max_concurrent {
                return Err(HarnessError::other(format!(
                    "Maximum concurrent tasks ({}) exceeded",
                    self.max_concurrent
                )));
            }
            *count += 1;
        }

        // Create task
        let task_id = uuid::Uuid::new_v4().to_string();
        let prompt = prompt.into();

        let task = SubagentTask {
            id: task_id.clone(),
            agent_id: agent_id.to_string(),
            prompt: prompt.clone(),
            status: TaskStatus::Running,
            result: None,
        };

        // Store task
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task_id.clone(), task);
        }

        // Execute the task (placeholder for now)
        let result = self.execute_task(agent, &prompt).await;

        // Update task status
        {
            let mut tasks = self.tasks.write().await;
            if let Some(t) = tasks.get_mut(&task_id) {
                match &result {
                    Ok(r) => {
                        t.status = TaskStatus::Completed;
                        t.result = Some(r.clone());
                    }
                    Err(_) => {
                        t.status = TaskStatus::Failed;
                        t.result = None;
                    }
                }
            }
        }

        // Decrement running count
        {
            let mut count = self.running_count.lock().await;
            *count -= 1;
        }

        result
    }

    /// Get a task by ID
    pub async fn get_task(&self, task_id: &str) -> Option<SubagentTask> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    /// List all tasks
    pub async fn list_tasks(&self) -> Vec<SubagentTask> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// Execute a task (placeholder implementation)
    async fn execute_task(
        &self,
        _agent: &crate::subagents::registry::AgentDefinition,
        prompt: &str,
    ) -> Result<String> {
        // TODO: Implement actual task execution
        // This would involve:
        // 1. Creating an agent executor with the agent's configuration
        // 2. Running the agent with the prompt
        // 3. Returning the result

        // For now, return a placeholder
        Ok(format!(
            "[Subagent execution placeholder for prompt: {}]",
            prompt
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subagents::registry::{AgentDefinition, AgentRegistry};

    #[test]
    fn test_task_status() {
        assert_eq!(TaskStatus::Pending, TaskStatus::Pending);
        assert_eq!(TaskStatus::Running, TaskStatus::Running);
        assert_eq!(TaskStatus::Completed, TaskStatus::Completed);
        assert_eq!(TaskStatus::Failed, TaskStatus::Failed);
        assert_eq!(TaskStatus::TimedOut, TaskStatus::TimedOut);
    }

    #[tokio::test]
    async fn test_subagent_executor_new() {
        let registry = Arc::new(AgentRegistry::new());
        let executor = SubagentExecutor::new(registry, 3);

        assert_eq!(executor.max_concurrent, 3);

        let tasks = executor.list_tasks().await;
        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn test_subagent_executor_execute_agent_not_found() {
        let registry = Arc::new(AgentRegistry::new());
        let executor = SubagentExecutor::new(registry, 3);

        let result = executor.execute("nonexistent", "test prompt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_subagent_executor_execute() {
        let mut registry = AgentRegistry::new();
        registry
            .register(AgentDefinition::new(
                "test-agent",
                "Test",
                "Test agent",
                "general-purpose",
                "You are helpful.",
            ))
            .unwrap();

        let executor = SubagentExecutor::new(Arc::new(registry), 3);

        let result = executor.execute("test-agent", "test prompt").await;
        assert!(result.is_ok());

        // Should have placeholder result
        assert!(result.unwrap().contains("Subagent execution placeholder"));
    }

    #[tokio::test]
    async fn test_subagent_executor_concurrent_limit() {
        let mut registry = AgentRegistry::new();
        registry
            .register(AgentDefinition::new(
                "test-agent",
                "Test",
                "Test agent",
                "general-purpose",
                "You are helpful.",
            ))
            .unwrap();

        let executor = SubagentExecutor::new(Arc::new(registry), 1);

        // Test that max_concurrent is set correctly
        assert_eq!(executor.max_concurrent, 1);

        // Test that executor runs tasks successfully
        let result = executor.execute("test-agent", "prompt1").await;
        assert!(result.is_ok());

        // Verify task was stored
        let tasks = executor.list_tasks().await;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::Completed);
    }
}
