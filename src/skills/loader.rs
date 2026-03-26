//! Skills loader
//!
//! This module provides the skills loader for discovering and loading
//! agent skill definitions from the filesystem.

use crate::{error::HarnessError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Skill metadata from frontmatter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// Skill name
    pub name: String,

    /// Skill description
    pub description: String,

    /// Skill version
    pub version: Option<String>,

    /// Skill author
    pub author: Option<String>,

    /// Skill license
    pub license: Option<String>,

    /// Required tool groups
    pub required_tools: Option<Vec<String>>,

    /// Skill tags
    pub tags: Option<Vec<String>>,

    /// Whether the skill is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Skill definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Skill metadata
    pub metadata: SkillMetadata,

    /// Skill content (markdown)
    pub content: String,

    /// Skill file path
    #[serde(skip)]
    pub path: PathBuf,

    /// Container path for the skill
    #[serde(skip)]
    pub container_path: Option<PathBuf>,
}

/// Skills loader for discovering and loading skills
pub struct SkillLoader {
    /// Skills directory
    skills_dir: PathBuf,

    /// Custom skills directory
    custom_dir: Option<PathBuf>,

    /// Loaded skills
    skills: HashMap<String, Skill>,
}

impl SkillLoader {
    /// Create a new skills loader
    pub fn new(skills_dir: impl Into<PathBuf>) -> Self {
        Self {
            skills_dir: skills_dir.into(),
            custom_dir: None,
            skills: HashMap::new(),
        }
    }

    /// Set the custom skills directory
    pub fn with_custom_dir(mut self, custom_dir: impl Into<PathBuf>) -> Self {
        self.custom_dir = Some(custom_dir.into());
        self
    }

    /// Load all skills from the skills directories
    pub async fn load_all(&mut self) -> Result<Vec<Skill>> {
        let mut all_skills = Vec::new();

        // Load from public skills directory
        if let Ok(mut skills) = self.load_from_dir(&self.skills_dir).await {
            all_skills.append(&mut skills);
        }

        // Load from custom skills directory
        if let Some(ref custom_dir) = self.custom_dir {
            if let Ok(mut skills) = self.load_from_dir(custom_dir).await {
                all_skills.append(&mut skills);
            }
        }

        // Update the skills map
        for skill in &all_skills {
            self.skills
                .insert(skill.metadata.name.clone(), skill.clone());
        }

        Ok(all_skills)
    }

    /// Load skills from a specific directory
    async fn load_from_dir(&self, dir: &Path) -> Result<Vec<Skill>> {
        let mut skills = Vec::new();

        if !dir.exists() {
            return Ok(skills);
        }

        let mut entries = fs::read_dir(dir).await.map_err(|e| {
            HarnessError::other(format!("Failed to read skills directory: {}", e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            HarnessError::other(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();

            // Skip if not a directory
            if !path.is_dir() {
                continue;
            }

            // Look for SKILL.md or README.md
            let skill_file = path.join("SKILL.md");
            let readme_file = path.join("README.md");

            let skill_path = if skill_file.exists() {
                skill_file
            } else if readme_file.exists() {
                readme_file
            } else {
                continue;
            };

            // Load the skill
            match self.load_skill_file(&skill_path, &path).await {
                Ok(skill) => skills.push(skill),
                Err(e) => {
                    // Log error but continue loading other skills
                    eprintln!("Failed to load skill {:?}: {}", path, e);
                }
            }
        }

        Ok(skills)
    }

    /// Load a skill from a file
    async fn load_skill_file(&self, skill_file: &Path, skill_dir: &Path) -> Result<Skill> {
        let content = fs::read_to_string(skill_file).await.map_err(|e| {
            HarnessError::other(format!("Failed to read skill file: {}", e))
        })?;

        // Parse frontmatter
        let (metadata, content) = self.parse_frontmatter(&content)?;

        Ok(Skill {
            metadata,
            content,
            path: skill_file.to_path_buf(),
            container_path: Some(skill_dir.to_path_buf()),
        })
    }

    /// Parse YAML frontmatter from markdown content
    fn parse_frontmatter(&self, content: &str) -> Result<(SkillMetadata, String)> {
        // Check for frontmatter delimiter
        if !content.starts_with("---") {
            return Ok((
                SkillMetadata {
                    name: "unknown".to_string(),
                    description: String::new(),
                    version: None,
                    author: None,
                    license: None,
                    required_tools: None,
                    tags: None,
                    enabled: true,
                },
                content.to_string(),
            ));
        }

        // Find the end of frontmatter
        let parts = content.splitn(3, "---").collect::<Vec<&str>>();
        if parts.len() < 3 {
            return Err(HarnessError::other("Invalid frontmatter format"));
        }

        let frontmatter = parts[1].trim();
        let markdown = parts[2].to_string();

        // Parse YAML metadata
        let metadata: SkillMetadata = serde_yaml::from_str(frontmatter)
            .map_err(|e| HarnessError::other(format!("Failed to parse skill metadata: {}", e)))?;

        Ok((metadata, markdown))
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// List all skill names
    pub fn list(&self) -> Vec<String> {
        self.skills.keys().cloned().collect()
    }

    /// List enabled skills
    pub fn list_enabled(&self) -> Vec<String> {
        self.skills
            .iter()
            .filter(|(_, s)| s.metadata.enabled)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get skills that require specific tools
    pub fn get_skills_for_tools(&self, tool_groups: &[String]) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|skill| {
                if let Some(ref required) = skill.metadata.required_tools {
                    required.iter().any(|t| tool_groups.contains(t))
                } else {
                    true
                }
            })
            .filter(|skill| skill.metadata.enabled)
            .collect()
    }

    /// Format skills for system prompt
    pub fn format_for_prompt(&self, max_skills: usize) -> String {
        let enabled_skills: Vec<_> = self
            .skills
            .values()
            .filter(|s| s.metadata.enabled)
            .take(max_skills)
            .collect();

        if enabled_skills.is_empty() {
            return String::new();
        }

        let mut parts = Vec::new();

        for skill in enabled_skills {
            let container = skill
                .container_path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            parts.push(format!(
                "- **{}** ({}): {}",
                skill.metadata.name, container, skill.metadata.description
            ));
        }

        format!("Available Skills:\n{}", parts.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_skill_loader_new() {
        let loader = SkillLoader::new("/tmp/skills");
        assert_eq!(loader.skills_dir, PathBuf::from("/tmp/skills"));
        assert!(loader.custom_dir.is_none());
        assert!(loader.skills.is_empty());
    }

    #[tokio::test]
    async fn test_skill_loader_list_empty() {
        let loader = SkillLoader::new("/nonexistent/skills");
        let names = loader.list();
        assert!(names.is_empty());
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let loader = SkillLoader::new("/tmp");
        let content = "# Just markdown\n\nSome content";
        let result = loader.parse_frontmatter(content);
        assert!(result.is_ok());
        let (metadata, markdown) = result.unwrap();
        assert_eq!(metadata.name, "unknown");
        assert_eq!(markdown, content);
    }

    #[test]
    fn test_parse_frontmatter_valid() {
        let loader = SkillLoader::new("/tmp");
        let content = r#"---
name: "test-skill"
description: "A test skill"
version: "1.0.0"
enabled: true
---

# Skill Content

Some content here."#;

        let result = loader.parse_frontmatter(content);
        assert!(result.is_ok());
        let (metadata, markdown) = result.unwrap();
        assert_eq!(metadata.name, "test-skill");
        assert_eq!(metadata.description, "A test skill");
        assert_eq!(metadata.version, Some("1.0.0".to_string()));
        assert!(metadata.enabled);
        assert!(markdown.contains("Skill Content"));
    }
}
