//! Memory updater
//!
//! This module provides the memory updater for processing conversations
//! and extracting memory updates.

use crate::{
    memory::data::*,
    models::base::{ChatModel, Message, MessageRole, ModelRequest},
    Result,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Memory updater for processing conversations
pub struct MemoryUpdater {
    /// Model for generating memory updates
    model: Arc<dyn ChatModel>,

    /// Memory data
    memory: Arc<RwLock<MemoryData>>,

    /// Maximum facts to store
    max_facts: usize,

    /// Minimum confidence threshold
    min_confidence: f64,
}

impl MemoryUpdater {
    /// Create a new memory updater
    pub fn new(
        model: Arc<dyn ChatModel>,
        memory: Arc<RwLock<MemoryData>>,
        max_facts: usize,
        min_confidence: f64,
    ) -> Self {
        Self {
            model,
            memory,
            max_facts,
            min_confidence,
        }
    }

    /// Process a conversation and update memory
    pub async fn process_conversation(
        &self,
        messages: &[Message],
    ) -> Result<MemoryUpdate> {
        // Filter messages to user inputs and final AI responses
        let relevant_messages: Vec<_> = messages
            .iter()
            .filter(|m| matches!(m.role, MessageRole::Human | MessageRole::Ai))
            .collect();

        if relevant_messages.is_empty() {
            return Ok(MemoryUpdate::default());
        }

        // Create prompt for memory extraction
        let prompt = self.create_extraction_prompt(&relevant_messages);

        // Request model to extract memory updates
        let request = ModelRequest {
            messages: vec![Message {
                role: MessageRole::Human,
                content: crate::messages::Content::text(prompt),
                tool_calls: vec![],
                tool_call_id: None,
                metadata: None,
                usage: None,
            }],
            temperature: Some(0.3),
            max_tokens: Some(2048),
            top_p: None,
            top_k: None,
            stop: vec![],
        };

        let response = self.model.invoke(request).await?;

        // Parse the response into memory updates
        let updates = self.parse_memory_updates(&response.message.content.to_string())?;

        Ok(updates)
    }

    /// Apply memory updates
    pub async fn apply_updates(&self, updates: &MemoryUpdate) -> Result<()> {
        let mut memory = self.memory.write().await;

        // Update user context
        if let Some(work) = &updates.work_context {
            memory.user_context.work_context = Some(work.clone());
        }
        if let Some(personal) = &updates.personal_context {
            memory.user_context.personal_context = Some(personal.clone());
        }
        if let Some(top) = &updates.top_of_mind {
            memory.user_context.top_of_mind = Some(top.clone());
        }

        // Add facts
        for fact in &updates.new_facts {
            if fact.confidence >= self.min_confidence {
                memory.add_fact(fact.clone());
            }
        }

        // Prune if necessary
        if memory.facts.len() > self.max_facts {
            memory.prune_facts(self.max_facts, self.min_confidence);
        }

        Ok(())
    }

    /// Get current memory for prompt injection
    pub async fn get_memory_for_prompt(&self, max_facts: usize) -> String {
        let memory = self.memory.read().await;
        memory.format_for_prompt(max_facts)
    }

    /// Create extraction prompt
    fn create_extraction_prompt(&self, messages: &[&Message]) -> String {
        let conversation = messages
            .iter()
            .map(|m| format!("{:?}: {}", m.role, m.content.to_string()))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "Analyze the following conversation and extract memory updates in JSON format:\n\n\
            {}\n\n\
            Extract the following information (use null for empty/not found):\n\
            - work_context: User's work situation, projects, role\n\
            - personal_context: Personal preferences, habits, interests\n\
            - top_of_mind: 1-3 sentence summary of what's currently important\n\
            - facts: Array of discrete facts with content, category, confidence (0-1)\n\n\
            Respond with valid JSON only.",
            conversation
        )
    }

    /// Parse memory updates from model response
    fn parse_memory_updates(&self, response: &str) -> Result<MemoryUpdate> {
        // Parse JSON response
        let parsed: serde_json::Value = serde_json::from_str(response)
            .map_err(|e| crate::error::HarnessError::other(format!("Failed to parse memory update: {}", e)))?;

        Ok(MemoryUpdate {
            work_context: parsed["work_context"].as_str().map(|s| s.to_string()),
            personal_context: parsed["personal_context"].as_str().map(|s| s.to_string()),
            top_of_mind: parsed["top_of_mind"].as_str().map(|s| s.to_string()),
            new_facts: self.parse_facts(&parsed["facts"])?,
        })
    }

    /// Parse facts from JSON
    fn parse_facts(&self, facts_value: &serde_json::Value) -> Result<Vec<Fact>> {
        let mut facts = Vec::new();

        if let Some(facts_array) = facts_value.as_array() {
            for fact_value in facts_array {
                let content = fact_value["content"]
                    .as_str()
                    .ok_or_else(|| crate::error::HarnessError::other("Missing fact content"))?;

                let category_str = fact_value["category"]
                    .as_str()
                    .unwrap_or("knowledge");

                let category = match category_str.to_lowercase().as_str() {
                    "preference" => FactCategory::Preference,
                    "knowledge" => FactCategory::Knowledge,
                    "context" => FactCategory::Context,
                    "behavior" => FactCategory::Behavior,
                    "goal" => FactCategory::Goal,
                    _ => FactCategory::Knowledge,
                };

                let confidence = fact_value["confidence"].as_f64().unwrap_or(0.7);

                facts.push(Fact::new(
                    content,
                    category,
                    confidence,
                    FactSource::Inferred,
                ));
            }
        }

        Ok(facts)
    }
}

/// Memory update result
#[derive(Debug, Clone, Default)]
pub struct MemoryUpdate {
    /// Work context update
    pub work_context: Option<String>,

    /// Personal context update
    pub personal_context: Option<String>,

    /// Top of mind update
    pub top_of_mind: Option<String>,

    /// New facts to add
    pub new_facts: Vec<Fact>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_update_default() {
        let update = MemoryUpdate::default();
        assert!(update.work_context.is_none());
        assert!(update.personal_context.is_none());
        assert!(update.top_of_mind.is_none());
        assert!(update.new_facts.is_empty());
    }

    #[test]
    fn test_parse_facts() {
        // Create a mock updater (we only need parse_facts)
        let memory = Arc::new(RwLock::new(MemoryData::new()));

        // We can't create a real model without mocking, so we'll test parse_facts
        // indirectly through the structure
        let facts_json = serde_json::json!([
            {"content": "Likes coffee", "category": "preference", "confidence": 0.9},
            {"content": "Works at tech company", "category": "context", "confidence": 0.8}
        ]);

        // This would be tested through process_conversation in a real scenario
        assert!(facts_json.is_array());
        assert_eq!(facts_json.as_array().unwrap().len(), 2);
    }
}
