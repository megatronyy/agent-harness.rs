//! Memory data structures
//!
//! This module provides the data structures for storing memory information.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// User context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// Work context (job, projects, etc.)
    pub work_context: Option<String>,

    /// Personal context (preferences, habits, etc.)
    pub personal_context: Option<String>,

    /// Top of mind information (1-3 sentence summaries)
    pub top_of_mind: Option<String>,
}

impl Default for UserContext {
    fn default() -> Self {
        Self {
            work_context: None,
            personal_context: None,
            top_of_mind: None,
        }
    }
}

/// Conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationHistory {
    /// Recent months summary
    pub recent_months: Option<String>,

    /// Earlier context
    pub earlier_context: Option<String>,

    /// Long-term background
    pub long_term_background: Option<String>,
}

impl Default for ConversationHistory {
    fn default() -> Self {
        Self {
            recent_months: None,
            earlier_context: None,
            long_term_background: None,
        }
    }
}

/// Individual fact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    /// Unique fact ID
    pub id: String,

    /// Fact content
    pub content: String,

    /// Fact category (preference, knowledge, context, behavior, goal)
    pub category: FactCategory,

    /// Confidence score (0-1)
    pub confidence: f64,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Source of the fact
    pub source: FactSource,
}

/// Fact category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FactCategory {
    /// User preference
    Preference,
    /// Knowledge about the user
    Knowledge,
    /// Context information
    Context,
    /// Behavioral pattern
    Behavior,
    /// User goal
    Goal,
}

/// Fact source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FactSource {
    /// From conversation
    Conversation,
    /// From explicit user input
    Explicit,
    /// Inferred by the system
    Inferred,
    /// From external source
    External,
}

/// Memory data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryData {
    /// User context
    pub user_context: UserContext,

    /// Conversation history
    pub history: ConversationHistory,

    /// Discrete facts
    pub facts: Vec<Fact>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl Default for MemoryData {
    fn default() -> Self {
        Self {
            user_context: UserContext::default(),
            history: ConversationHistory::default(),
            facts: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

impl MemoryData {
    /// Create a new memory data structure
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a fact
    pub fn add_fact(&mut self, fact: Fact) {
        // Check for duplicates
        if !self.facts.iter().any(|f| f.content == fact.content) {
            self.facts.push(fact);
        }
    }

    /// Get facts by category
    pub fn facts_by_category(&self, category: &FactCategory) -> Vec<&Fact> {
        self.facts
            .iter()
            .filter(|f| &f.category == category)
            .collect()
    }

    /// Get high confidence facts (confidence >= threshold)
    pub fn high_confidence_facts(&self, threshold: f64) -> Vec<&Fact> {
        self.facts.iter().filter(|f| f.confidence >= threshold).collect()
    }

    /// Get recent facts (within last N days)
    pub fn recent_facts(&self, days: i64) -> Vec<&Fact> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        self.facts
            .iter()
            .filter(|f| f.created_at > cutoff)
            .collect()
    }

    /// Prune old/low-confidence facts to maintain size limit
    pub fn prune_facts(&mut self, max_facts: usize, min_confidence: f64) {
        // Sort by confidence and recency
        self.facts.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.created_at.cmp(&a.created_at))
        });

        // Keep only high-confidence facts within limit
        self.facts = self
            .facts
            .drain(..)
            .filter(|f| f.confidence >= min_confidence)
            .take(max_facts)
            .collect();
    }

    /// Format for injection into system prompt
    pub fn format_for_prompt(&self, max_facts: usize) -> String {
        let mut parts = Vec::new();

        // User context
        if let Some(work) = &self.user_context.work_context {
            parts.push(format!("Work Context: {}", work));
        }
        if let Some(personal) = &self.user_context.personal_context {
            parts.push(format!("Personal Context: {}", personal));
        }
        if let Some(top) = &self.user_context.top_of_mind {
            parts.push(format!("Top of Mind: {}", top));
        }

        // High confidence facts
        let facts: Vec<_> = self
            .high_confidence_facts(0.7)
            .into_iter()
            .take(max_facts)
            .collect();

        if !facts.is_empty() {
            parts.push("Relevant Facts:".to_string());
            for fact in facts {
                parts.push(format!("- {}: {}", fact.category_as_str(), fact.content));
            }
        }

        parts.join("\n")
    }
}

impl Fact {
    /// Create a new fact
    pub fn new(
        content: impl Into<String>,
        category: FactCategory,
        confidence: f64,
        source: FactSource,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content: content.into(),
            category,
            confidence,
            created_at: chrono::Utc::now(),
            source,
        }
    }

    /// Get category as string
    pub fn category_as_str(&self) -> &'static str {
        match self.category {
            FactCategory::Preference => "Preference",
            FactCategory::Knowledge => "Knowledge",
            FactCategory::Context => "Context",
            FactCategory::Behavior => "Behavior",
            FactCategory::Goal => "Goal",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_context_default() {
        let ctx = UserContext::default();
        assert!(ctx.work_context.is_none());
        assert!(ctx.personal_context.is_none());
        assert!(ctx.top_of_mind.is_none());
    }

    #[test]
    fn test_fact_new() {
        let fact = Fact::new(
            "User prefers Rust",
            FactCategory::Preference,
            0.9,
            FactSource::Conversation,
        );

        assert_eq!(fact.content, "User prefers Rust");
        assert_eq!(fact.category, FactCategory::Preference);
        assert_eq!(fact.confidence, 0.9);
    }

    #[test]
    fn test_memory_data_add_fact() {
        let mut memory = MemoryData::new();
        let fact1 = Fact::new(
            "User likes coffee",
            FactCategory::Preference,
            0.8,
            FactSource::Conversation,
        );
        let fact2 = fact1.clone();

        memory.add_fact(fact1);
        assert_eq!(memory.facts.len(), 1);

        // Adding duplicate should not increase count
        memory.add_fact(fact2);
        assert_eq!(memory.facts.len(), 1);
    }

    #[test]
    fn test_memory_data_facts_by_category() {
        let mut memory = MemoryData::new();

        memory.add_fact(Fact::new(
            "Likes coffee",
            FactCategory::Preference,
            0.8,
            FactSource::Conversation,
        ));

        memory.add_fact(Fact::new(
            "Knows Python",
            FactCategory::Knowledge,
            0.9,
            FactSource::Explicit,
        ));

        let prefs = memory.facts_by_category(&FactCategory::Preference);
        assert_eq!(prefs.len(), 1);
        assert_eq!(prefs[0].content, "Likes coffee");

        let knowledge = memory.facts_by_category(&FactCategory::Knowledge);
        assert_eq!(knowledge.len(), 1);
        assert_eq!(knowledge[0].content, "Knows Python");
    }

    #[test]
    fn test_memory_data_prune_facts() {
        let mut memory = MemoryData::new();

        // Add facts with varying confidence
        for i in 0..10 {
            let confidence = 0.5 + (i as f64 * 0.05);
            memory.add_fact(Fact::new(
                format!("Fact {}", i),
                FactCategory::Knowledge,
                confidence,
                FactSource::Conversation,
            ));
        }

        assert_eq!(memory.facts.len(), 10);

        // Prune to max 5 with min confidence 0.7
        memory.prune_facts(5, 0.7);

        assert!(memory.facts.len() <= 5);
        for fact in &memory.facts {
            assert!(fact.confidence >= 0.7);
        }
    }

    #[test]
    fn test_memory_format_for_prompt() {
        let mut memory = MemoryData::new();

        memory.user_context.work_context = Some("Software engineer".to_string());
        memory.add_fact(Fact::new(
            "Likes Rust",
            FactCategory::Preference,
            0.9,
            FactSource::Conversation,
        ));

        let prompt = memory.format_for_prompt(10);
        assert!(prompt.contains("Work Context: Software engineer"));
        assert!(prompt.contains("Relevant Facts"));
        assert!(prompt.contains("Preference: Likes Rust"));
    }
}
