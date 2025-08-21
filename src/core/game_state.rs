use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::core::{Player, InventoryItem};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub id: Uuid,
    pub player: Player,
    pub current_scene_id: String,
    pub story_id: String,
    pub visited_scenes: Vec<String>,
    pub flags: HashMap<String, serde_json::Value>,
    pub game_start_time: DateTime<Utc>,
    pub last_save_time: Option<DateTime<Utc>>,
    pub playtime_seconds: i64,
}

impl GameState {
    pub fn new(story_id: String, current_scene_id: String, player: Player) -> Self {
        Self {
            id: Uuid::new_v4(),
            player,
            current_scene_id,
            story_id,
            visited_scenes: Vec::new(),
            flags: HashMap::new(),
            game_start_time: Utc::now(),
            last_save_time: None,
            playtime_seconds: 0,
        }
    }

    pub fn visit_scene(&mut self, scene_id: &str) {
        self.current_scene_id = scene_id.to_string();
        
        if !self.visited_scenes.contains(&scene_id.to_string()) {
            self.visited_scenes.push(scene_id.to_string());
        }
    }

    pub fn has_visited_scene(&self, scene_id: &str) -> bool {
        self.visited_scenes.contains(&scene_id.to_string())
    }

    pub fn set_flag<S: Into<String>>(&mut self, key: S, value: serde_json::Value) {
        self.flags.insert(key.into(), value);
    }

    pub fn get_flag(&self, key: &str) -> Option<&serde_json::Value> {
        self.flags.get(key)
    }

    pub fn get_flag_as_bool(&self, key: &str) -> bool {
        self.flags
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub fn get_flag_as_i64(&self, key: &str) -> i64 {
        self.flags
            .get(key)
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
    }

    pub fn get_flag_as_string(&self, key: &str) -> String {
        self.flags
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default()
    }

    pub fn remove_flag(&mut self, key: &str) -> Option<serde_json::Value> {
        self.flags.remove(key)
    }

    pub fn clear_flags(&mut self) {
        self.flags.clear();
    }

    pub fn update_playtime(&mut self) {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(self.game_start_time);
        self.playtime_seconds = elapsed.num_seconds();
    }

    pub fn mark_saved(&mut self) {
        self.update_playtime();
        self.last_save_time = Some(Utc::now());
    }

    pub fn get_playtime_formatted(&self) -> String {
        let hours = self.playtime_seconds / 3600;
        let minutes = (self.playtime_seconds % 3600) / 60;
        let seconds = self.playtime_seconds % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }

    pub fn get_scene_visit_count(&self, scene_id: &str) -> usize {
        self.visited_scenes
            .iter()
            .filter(|&id| id == scene_id)
            .count()
    }

    pub fn get_total_scenes_visited(&self) -> usize {
        self.visited_scenes.len()
    }

    pub fn get_unique_scenes_visited(&self) -> usize {
        let mut unique_scenes: std::collections::HashSet<&String> = std::collections::HashSet::new();
        for scene_id in &self.visited_scenes {
            unique_scenes.insert(scene_id);
        }
        unique_scenes.len()
    }

    // Helper methods for common flag operations
    pub fn increment_flag(&mut self, key: &str, amount: i64) {
        let current = self.get_flag_as_i64(key);
        self.set_flag(key, serde_json::Value::Number(serde_json::Number::from(current + amount)));
    }

    pub fn decrement_flag(&mut self, key: &str, amount: i64) {
        let current = self.get_flag_as_i64(key);
        let new_value = (current - amount).max(0);
        self.set_flag(key, serde_json::Value::Number(serde_json::Number::from(new_value)));
    }

    pub fn toggle_flag(&mut self, key: &str) {
        let current = self.get_flag_as_bool(key);
        self.set_flag(key, serde_json::Value::Bool(!current));
    }

    // Statistics methods
    pub fn get_statistics(&self) -> GameStatistics {
        GameStatistics {
            playtime_seconds: self.playtime_seconds,
            total_scenes_visited: self.get_total_scenes_visited(),
            unique_scenes_visited: self.get_unique_scenes_visited(),
            player_level: self.player.stats.level,
            total_experience: self.player.stats.experience,
            inventory_size: self.player.inventory.len(),
            total_inventory_value: self.player.get_inventory_value(),
            flags_set: self.flags.len(),
            game_start_time: self.game_start_time,
            last_save_time: self.last_save_time,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStatistics {
    pub playtime_seconds: i64,
    pub total_scenes_visited: usize,
    pub unique_scenes_visited: usize,
    pub player_level: i32,
    pub total_experience: i32,
    pub inventory_size: usize,
    pub total_inventory_value: i32,
    pub flags_set: usize,
    pub game_start_time: DateTime<Utc>,
    pub last_save_time: Option<DateTime<Utc>>,
}

impl GameStatistics {
    pub fn get_playtime_formatted(&self) -> String {
        let hours = self.playtime_seconds / 3600;
        let minutes = (self.playtime_seconds % 3600) / 60;
        let seconds = self.playtime_seconds % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Player, PlayerStats};

    #[test]
    fn test_game_state_creation() {
        let player = Player::new("Test Player", Some(PlayerStats::default()));
        let game_state = GameState::new(
            "test_story".to_string(),
            "start".to_string(),
            player,
        );

        assert_eq!(game_state.story_id, "test_story");
        assert_eq!(game_state.current_scene_id, "start");
        assert_eq!(game_state.player.name, "Test Player");
        assert!(game_state.visited_scenes.is_empty());
        assert!(game_state.flags.is_empty());
    }

    #[test]
    fn test_scene_visiting() {
        let player = Player::new("Test Player", Some(PlayerStats::default()));
        let mut game_state = GameState::new(
            "test_story".to_string(),
            "start".to_string(),
            player,
        );

        game_state.visit_scene("scene1");
        assert_eq!(game_state.current_scene_id, "scene1");
        assert!(game_state.has_visited_scene("scene1"));
        assert_eq!(game_state.get_total_scenes_visited(), 1);

        // Visit same scene again
        game_state.visit_scene("scene1");
        assert_eq!(game_state.get_total_scenes_visited(), 2);
        assert_eq!(game_state.get_unique_scenes_visited(), 1);
    }

    #[test]
    fn test_flag_operations() {
        let player = Player::new("Test Player", Some(PlayerStats::default()));
        let mut game_state = GameState::new(
            "test_story".to_string(),
            "start".to_string(),
            player,
        );

        // Test setting and getting flags
        game_state.set_flag("test_bool", serde_json::Value::Bool(true));
        assert!(game_state.get_flag_as_bool("test_bool"));

        game_state.set_flag("test_number", serde_json::Value::Number(serde_json::Number::from(42)));
        assert_eq!(game_state.get_flag_as_i64("test_number"), 42);

        game_state.set_flag("test_string", serde_json::Value::String("hello".to_string()));
        assert_eq!(game_state.get_flag_as_string("test_string"), "hello");

        // Test increment/decrement
        game_state.increment_flag("counter", 5);
        assert_eq!(game_state.get_flag_as_i64("counter"), 5);

        game_state.increment_flag("counter", 3);
        assert_eq!(game_state.get_flag_as_i64("counter"), 8);

        game_state.decrement_flag("counter", 2);
        assert_eq!(game_state.get_flag_as_i64("counter"), 6);

        // Test toggle
        game_state.toggle_flag("toggle_test");
        assert!(game_state.get_flag_as_bool("toggle_test"));

        game_state.toggle_flag("toggle_test");
        assert!(!game_state.get_flag_as_bool("toggle_test"));
    }

    #[test]
    fn test_statistics() {
        let player = Player::new("Test Player", Some(PlayerStats::default()));
        let mut game_state = GameState::new(
            "test_story".to_string(),
            "start".to_string(),
            player,
        );

        game_state.visit_scene("scene1");
        game_state.visit_scene("scene2");
        game_state.visit_scene("scene1"); // Revisit
        game_state.set_flag("test1", serde_json::Value::Bool(true));
        game_state.set_flag("test2", serde_json::Value::Number(serde_json::Number::from(10)));

        let stats = game_state.get_statistics();
        assert_eq!(stats.total_scenes_visited, 3);
        assert_eq!(stats.unique_scenes_visited, 2);
        assert_eq!(stats.flags_set, 2);
        assert_eq!(stats.player_level, 1);
    }
}