//! Subagent registry
//!
//! This module provides the agent registry for managing available subagents.

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Subagent definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Unique agent identifier
    pub id: String,

    /// Agent display name
    pub name: String,

    /// Agent description
    pub description: String,

    /// Agent type (general-purpose, bash, etc.)
    pub agent_type: String,

    /// System prompt for this agent
    pub system_prompt: String,

    /// Maximum number of turns for this agent
    pub max_turns: Option<usize>,

    /// Available tool groups for this agent
    pub tool_groups: Vec<String>,
}

impl AgentDefinition {
    /// Create a new agent definition
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        agent_type: impl Into<String>,
        system_prompt: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            agent_type: agent_type.into(),
            system_prompt: system_prompt.into(),
            max_turns: None,
            tool_groups: Vec::new(),
        }
    }

    /// Set the maximum turns
    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = Some(max_turns);
        self
    }

    /// Add a tool group
    pub fn with_tool_group(mut self, group: impl Into<String>) -> Self {
        self.tool_groups.push(group.into());
        self
    }
}

/// Registry for available subagents
pub struct AgentRegistry {
    /// Registered agents by ID
    agents: HashMap<String, AgentDefinition>,
}

impl AgentRegistry {
    /// Create a new agent registry
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    /// Register an agent
    pub fn register(&mut self, agent: AgentDefinition) -> Result<()> {
        let id = agent.id.clone();
        self.agents.insert(id, agent);
        Ok(())
    }

    /// Get an agent by ID
    pub fn get(&self, id: &str) -> Option<&AgentDefinition> {
        self.agents.get(id)
    }

    /// List all agent IDs
    pub fn list(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }

    /// Get agents by type
    pub fn list_by_type(&self, agent_type: &str) -> Vec<&AgentDefinition> {
        self.agents
            .values()
            .filter(|a| a.agent_type == agent_type)
            .collect()
    }

    /// Remove an agent
    pub fn remove(&mut self, id: &str) -> Result<()> {
        self.agents
            .remove(id)
            .ok_or_else(|| crate::error::HarnessError::other(format!("Agent not found: {}", id)))?;
        Ok(())
    }

    /// Check if an agent exists
    pub fn contains(&self, id: &str) -> bool {
        self.agents.contains_key(id)
    }

    /// Get the number of registered agents
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_definition_new() {
        let agent = AgentDefinition::new(
            "test-agent",
            "Test Agent",
            "A test agent",
            "general-purpose",
            "You are a helpful assistant.",
        );

        assert_eq!(agent.id, "test-agent");
        assert_eq!(agent.name, "Test Agent");
        assert_eq!(agent.agent_type, "general-purpose");
        assert_eq!(agent.max_turns, None);
        assert!(agent.tool_groups.is_empty());
    }

    #[test]
    fn test_agent_definition_builder() {
        let agent = AgentDefinition::new(
            "test-agent",
            "Test Agent",
            "A test agent",
            "general-purpose",
            "You are a helpful assistant.",
        )
        .with_max_turns(10)
        .with_tool_group("sandbox");

        assert_eq!(agent.max_turns, Some(10));
        assert_eq!(agent.tool_groups, vec!["sandbox"]);
    }

    #[test]
    fn test_agent_registry() {
        let mut registry = AgentRegistry::new();

        let agent = AgentDefinition::new(
            "test-agent",
            "Test Agent",
            "A test agent",
            "general-purpose",
            "You are a helpful assistant.",
        );

        registry.register(agent).unwrap();
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test-agent"));

        let retrieved = registry.get("test-agent").unwrap();
        assert_eq!(retrieved.id, "test-agent");

        let ids = registry.list();
        assert_eq!(ids, vec!["test-agent"]);

        registry.remove("test-agent").unwrap();
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_agent_registry_list_by_type() {
        let mut registry = AgentRegistry::new();

        registry
            .register(AgentDefinition::new(
                "agent1",
                "Agent 1",
                "First agent",
                "general-purpose",
                "prompt1",
            ))
            .unwrap();

        registry
            .register(AgentDefinition::new(
                "agent2",
                "Agent 2",
                "Second agent",
                "bash",
                "prompt2",
            ))
            .unwrap();

        let general_agents = registry.list_by_type("general-purpose");
        assert_eq!(general_agents.len(), 1);
        assert_eq!(general_agents[0].id, "agent1");

        let bash_agents = registry.list_by_type("bash");
        assert_eq!(bash_agents.len(), 1);
        assert_eq!(bash_agents[0].id, "agent2");
    }
}
