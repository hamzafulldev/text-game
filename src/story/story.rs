use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::core::PlayerStats;
use crate::story::{Condition, Effect};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Story {
    pub id: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub starting_scene_id: String,
    pub scenes: Vec<Scene>,
    pub initial_player_stats: PlayerStats,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub title: String,
    pub description: String,
    pub choices: Vec<Choice>,
    pub conditions: Option<Vec<Condition>>,
    pub effects: Option<Vec<Effect>>,
    pub is_ending: Option<bool>,
    pub background_music: Option<String>,
    pub image: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub id: String,
    pub text: String,
    pub target_scene_id: String,
    pub conditions: Option<Vec<Condition>>,
    pub effects: Option<Vec<Effect>>,
    pub disabled: Option<bool>,
    pub disabled_reason: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl Story {
    pub fn new<S: Into<String>>(
        id: S, 
        title: S, 
        starting_scene_id: S,
        initial_stats: PlayerStats
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: String::new(),
            author: String::new(),
            version: "1.0.0".to_string(),
            starting_scene_id: starting_scene_id.into(),
            scenes: Vec::new(),
            initial_player_stats: initial_stats,
            metadata: None,
        }
    }

    pub fn add_scene(&mut self, scene: Scene) {
        self.scenes.push(scene);
    }

    pub fn get_scene(&self, scene_id: &str) -> Option<&Scene> {
        self.scenes.iter().find(|s| s.id == scene_id)
    }

    pub fn get_starting_scene(&self) -> Option<&Scene> {
        self.get_scene(&self.starting_scene_id)
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check if starting scene exists
        if self.get_starting_scene().is_none() {
            errors.push(format!("Starting scene '{}' not found", self.starting_scene_id));
        }

        // Validate each scene
        for scene in &self.scenes {
            if let Err(mut scene_errors) = scene.validate(&self.scenes) {
                errors.append(&mut scene_errors);
            }
        }

        // Check for duplicate scene IDs
        let mut scene_ids = std::collections::HashSet::new();
        for scene in &self.scenes {
            if !scene_ids.insert(&scene.id) {
                errors.push(format!("Duplicate scene ID: '{}'", scene.id));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn get_endings(&self) -> Vec<&Scene> {
        self.scenes
            .iter()
            .filter(|scene| scene.is_ending.unwrap_or(false))
            .collect()
    }

    pub fn get_scene_count(&self) -> usize {
        self.scenes.len()
    }
}

impl Scene {
    pub fn new<S: Into<String>>(id: S, title: S, description: S) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            choices: Vec::new(),
            conditions: None,
            effects: None,
            is_ending: None,
            background_music: None,
            image: None,
            metadata: None,
        }
    }

    pub fn add_choice(&mut self, choice: Choice) {
        self.choices.push(choice);
    }

    pub fn get_choice(&self, choice_id: &str) -> Option<&Choice> {
        self.choices.iter().find(|c| c.id == choice_id)
    }

    pub fn is_ending(&self) -> bool {
        self.is_ending.unwrap_or(false)
    }

    pub fn validate(&self, all_scenes: &[Scene]) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate each choice
        for choice in &self.choices {
            if let Err(mut choice_errors) = choice.validate(all_scenes) {
                errors.append(&mut choice_errors);
            }
        }

        // Check for duplicate choice IDs within the scene
        let mut choice_ids = std::collections::HashSet::new();
        for choice in &self.choices {
            if !choice_ids.insert(&choice.id) {
                errors.push(format!("Scene '{}': Duplicate choice ID: '{}'", self.id, choice.id));
            }
        }

        // Ending scenes should have no choices (or only meta choices)
        if self.is_ending() && !self.choices.is_empty() {
            let non_meta_choices = self.choices.iter()
                .filter(|c| c.target_scene_id != "END" && c.target_scene_id != "RESTART")
                .count();
            if non_meta_choices > 0 {
                errors.push(format!("Ending scene '{}' should not have regular choices", self.id));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Choice {
    pub fn new<S: Into<String>>(id: S, text: S, target_scene_id: S) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            target_scene_id: target_scene_id.into(),
            conditions: None,
            effects: None,
            disabled: None,
            disabled_reason: None,
            metadata: None,
        }
    }

    pub fn with_conditions(mut self, conditions: Vec<Condition>) -> Self {
        self.conditions = Some(conditions);
        self
    }

    pub fn with_effects(mut self, effects: Vec<Effect>) -> Self {
        self.effects = Some(effects);
        self
    }

    pub fn disabled_with_reason<S: Into<String>>(mut self, reason: S) -> Self {
        self.disabled = Some(true);
        self.disabled_reason = Some(reason.into());
        self
    }

    pub fn validate(&self, all_scenes: &[Scene]) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check if target scene exists (unless it's a special target)
        let special_targets = ["END", "RESTART", "MAIN_MENU"];
        if !special_targets.contains(&self.target_scene_id.as_str()) {
            if !all_scenes.iter().any(|s| s.id == self.target_scene_id) {
                errors.push(format!(
                    "Choice '{}': Target scene '{}' not found", 
                    self.id, 
                    self.target_scene_id
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_story_creation() {
        let stats = PlayerStats::default();
        let story = Story::new("test", "Test Story", "start", stats);
        
        assert_eq!(story.id, "test");
        assert_eq!(story.title, "Test Story");
        assert_eq!(story.starting_scene_id, "start");
    }

    #[test]
    fn test_scene_creation() {
        let scene = Scene::new("test", "Test Scene", "A test scene");
        
        assert_eq!(scene.id, "test");
        assert_eq!(scene.title, "Test Scene");
        assert_eq!(scene.description, "A test scene");
        assert!(!scene.is_ending());
    }

    #[test]
    fn test_choice_creation() {
        let choice = Choice::new("test", "Test Choice", "target");
        
        assert_eq!(choice.id, "test");
        assert_eq!(choice.text, "Test Choice");
        assert_eq!(choice.target_scene_id, "target");
    }

    #[test]
    fn test_story_validation() {
        let mut story = Story::new("test", "Test Story", "start", PlayerStats::default());
        
        // Should fail - no starting scene
        assert!(story.validate().is_err());
        
        // Add starting scene
        story.add_scene(Scene::new("start", "Start", "Starting scene"));
        
        // Should pass
        assert!(story.validate().is_ok());
    }
}