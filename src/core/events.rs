use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::core::GameState;
use crate::story::{Scene, Choice};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub id: Uuid,
    pub event_type: GameEventType,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEventType {
    GameStarted,
    GameLoaded,
    GameSaved,
    GameEnded,
    SceneEntered,
    ChoiceMade,
    EffectApplied,
    StatModified,
    ItemAdded,
    ItemRemoved,
    ItemUsed,
    LevelUp,
    FlagSet,
    PlayerDied,
    Custom(String),
}

impl GameEvent {
    pub fn new(event_type: GameEventType, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            timestamp: Utc::now(),
            data,
        }
    }

    // Convenience constructors for common events
    pub fn game_started(story_id: &str, player_name: &str) -> Self {
        let data = serde_json::json!({
            "story_id": story_id,
            "player_name": player_name
        });
        Self::new(GameEventType::GameStarted, data)
    }

    pub fn game_loaded(save_name: &str) -> Self {
        let data = serde_json::json!({
            "save_name": save_name
        });
        Self::new(GameEventType::GameLoaded, data)
    }

    pub fn game_saved(save_name: &str) -> Self {
        let data = serde_json::json!({
            "save_name": save_name
        });
        Self::new(GameEventType::GameSaved, data)
    }

    pub fn game_ended(ending_scene_id: &str) -> Self {
        let data = serde_json::json!({
            "ending_scene_id": ending_scene_id
        });
        Self::new(GameEventType::GameEnded, data)
    }

    pub fn scene_entered(scene: &Scene) -> Self {
        let data = serde_json::json!({
            "scene_id": scene.id,
            "scene_title": scene.title
        });
        Self::new(GameEventType::SceneEntered, data)
    }

    pub fn choice_made(choice: &Choice, from_scene: &str) -> Self {
        let data = serde_json::json!({
            "choice_id": choice.id,
            "choice_text": choice.text,
            "from_scene": from_scene,
            "target_scene": choice.target_scene_id
        });
        Self::new(GameEventType::ChoiceMade, data)
    }

    pub fn stat_modified(stat_name: &str, old_value: i32, new_value: i32) -> Self {
        let data = serde_json::json!({
            "stat_name": stat_name,
            "old_value": old_value,
            "new_value": new_value,
            "change": new_value - old_value
        });
        Self::new(GameEventType::StatModified, data)
    }

    pub fn item_added(item_id: &str, item_name: &str, quantity: i32) -> Self {
        let data = serde_json::json!({
            "item_id": item_id,
            "item_name": item_name,
            "quantity": quantity
        });
        Self::new(GameEventType::ItemAdded, data)
    }

    pub fn item_removed(item_id: &str, item_name: &str, quantity: i32) -> Self {
        let data = serde_json::json!({
            "item_id": item_id,
            "item_name": item_name,
            "quantity": quantity
        });
        Self::new(GameEventType::ItemRemoved, data)
    }

    pub fn item_used(item_id: &str, item_name: &str) -> Self {
        let data = serde_json::json!({
            "item_id": item_id,
            "item_name": item_name
        });
        Self::new(GameEventType::ItemUsed, data)
    }

    pub fn level_up(old_level: i32, new_level: i32, experience: i32) -> Self {
        let data = serde_json::json!({
            "old_level": old_level,
            "new_level": new_level,
            "experience": experience
        });
        Self::new(GameEventType::LevelUp, data)
    }

    pub fn flag_set(flag_name: &str, value: &serde_json::Value) -> Self {
        let data = serde_json::json!({
            "flag_name": flag_name,
            "value": value
        });
        Self::new(GameEventType::FlagSet, data)
    }

    pub fn player_died(cause: &str) -> Self {
        let data = serde_json::json!({
            "cause": cause
        });
        Self::new(GameEventType::PlayerDied, data)
    }

    pub fn custom<S: Into<String>>(event_name: S, data: serde_json::Value) -> Self {
        Self::new(GameEventType::Custom(event_name.into()), data)
    }
}

pub trait GameEventHandler {
    fn handle_event(&mut self, event: &GameEvent);
}

pub struct EventLogger {
    events: Vec<GameEvent>,
    max_events: usize,
}

impl EventLogger {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Vec::new(),
            max_events,
        }
    }

    pub fn get_events(&self) -> &[GameEvent] {
        &self.events
    }

    pub fn get_events_by_type(&self, event_type: &GameEventType) -> Vec<&GameEvent> {
        self.events
            .iter()
            .filter(|event| std::mem::discriminant(&event.event_type) == std::mem::discriminant(event_type))
            .collect()
    }

    pub fn get_recent_events(&self, count: usize) -> Vec<&GameEvent> {
        self.events
            .iter()
            .rev()
            .take(count)
            .collect()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn export_events(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.events)
    }

    pub fn get_event_count(&self) -> usize {
        self.events.len()
    }

    pub fn get_event_count_by_type(&self, event_type: &GameEventType) -> usize {
        self.events
            .iter()
            .filter(|event| std::mem::discriminant(&event.event_type) == std::mem::discriminant(event_type))
            .count()
    }
}

impl Default for EventLogger {
    fn default() -> Self {
        Self::new(1000) // Default max 1000 events
    }
}

impl GameEventHandler for EventLogger {
    fn handle_event(&mut self, event: &GameEvent) {
        self.events.push(event.clone());
        
        // Remove oldest events if we exceed max capacity
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
    }
}

// Multiple event handlers can be combined
pub struct CompositeEventHandler {
    handlers: Vec<Box<dyn GameEventHandler>>,
}

impl CompositeEventHandler {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn add_handler<H: GameEventHandler + 'static>(&mut self, handler: H) {
        self.handlers.push(Box::new(handler));
    }
}

impl Default for CompositeEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl GameEventHandler for CompositeEventHandler {
    fn handle_event(&mut self, event: &GameEvent) {
        for handler in &mut self.handlers {
            handler.handle_event(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::{Scene, Choice};

    #[test]
    fn test_game_event_creation() {
        let event = GameEvent::game_started("test_story", "Test Player");
        
        assert!(matches!(event.event_type, GameEventType::GameStarted));
        assert_eq!(event.data["story_id"], "test_story");
        assert_eq!(event.data["player_name"], "Test Player");
    }

    #[test]
    fn test_scene_entered_event() {
        let scene = Scene::new("test_scene", "Test Scene", "A test scene");
        let event = GameEvent::scene_entered(&scene);
        
        assert!(matches!(event.event_type, GameEventType::SceneEntered));
        assert_eq!(event.data["scene_id"], "test_scene");
        assert_eq!(event.data["scene_title"], "Test Scene");
    }

    #[test]
    fn test_choice_made_event() {
        let choice = Choice::new("test_choice", "Test Choice", "target_scene");
        let event = GameEvent::choice_made(&choice, "from_scene");
        
        assert!(matches!(event.event_type, GameEventType::ChoiceMade));
        assert_eq!(event.data["choice_id"], "test_choice");
        assert_eq!(event.data["from_scene"], "from_scene");
        assert_eq!(event.data["target_scene"], "target_scene");
    }

    #[test]
    fn test_event_logger() {
        let mut logger = EventLogger::new(3);
        
        // Add events
        logger.handle_event(&GameEvent::game_started("story1", "player1"));
        logger.handle_event(&GameEvent::game_started("story2", "player2"));
        logger.handle_event(&GameEvent::game_started("story3", "player3"));
        
        assert_eq!(logger.get_event_count(), 3);
        
        // Add one more - should remove the oldest
        logger.handle_event(&GameEvent::game_started("story4", "player4"));
        
        assert_eq!(logger.get_event_count(), 3);
        assert_eq!(logger.get_events()[0].data["story_id"], "story2"); // First event should be story2 now
    }

    #[test]
    fn test_event_filtering() {
        let mut logger = EventLogger::default();
        
        logger.handle_event(&GameEvent::game_started("story", "player"));
        logger.handle_event(&GameEvent::game_saved("save1"));
        logger.handle_event(&GameEvent::game_saved("save2"));
        
        let save_events = logger.get_events_by_type(&GameEventType::GameSaved);
        assert_eq!(save_events.len(), 2);
        
        let start_events = logger.get_events_by_type(&GameEventType::GameStarted);
        assert_eq!(start_events.len(), 1);
    }

    #[test]
    fn test_composite_event_handler() {
        let mut composite = CompositeEventHandler::new();
        let logger1 = EventLogger::new(10);
        let logger2 = EventLogger::new(10);
        
        composite.add_handler(logger1);
        composite.add_handler(logger2);
        
        let event = GameEvent::game_started("story", "player");
        composite.handle_event(&event);
        
        // Both handlers should have received the event
        // (We can't easily test this without making the handlers accessible)
    }
}