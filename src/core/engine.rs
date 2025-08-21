use std::sync::Arc;
use tokio::sync::Mutex;
use crate::core::{GameState, Player, PlayerStats, GameEvent, GameEventHandler, EventLogger};
use crate::story::{Story, Scene, Choice, Condition, Effect, ConditionType, ComparisonOperator, EffectType, EffectOperation};
use crate::utils::{GameError, GameResult};
use tracing::{info, warn, error, debug};

pub struct GameEngine {
    story: Option<Story>,
    game_state: Option<GameState>,
    event_handler: Arc<Mutex<EventLogger>>,
}

impl GameEngine {
    pub fn new() -> Self {
        Self {
            story: None,
            game_state: None,
            event_handler: Arc::new(Mutex::new(EventLogger::default())),
        }
    }

    pub async fn load_story(&mut self, story: Story) -> GameResult<()> {
        info!("Loading story: {} ({})", story.title, story.id);
        
        // Validate story
        if let Err(errors) = story.validate() {
            let error_msg = errors.join("; ");
            return Err(GameError::story(format!("Story validation failed: {}", error_msg)));
        }

        self.story = Some(story);
        self.emit_event(GameEvent::custom("story_loaded", serde_json::json!({
            "story_id": self.story.as_ref().unwrap().id
        }))).await;
        
        Ok(())
    }

    pub async fn start_new_game(&mut self, player_name: String) -> GameResult<()> {
        let story = self.story.as_ref()
            .ok_or_else(|| GameError::story("No story loaded".to_string()))?;

        info!("Starting new game for player: {}", player_name);
        
        let player = Player::new(player_name.clone(), Some(story.initial_player_stats.clone()));
        let mut game_state = GameState::new(
            story.id.clone(),
            story.starting_scene_id.clone(),
            player,
        );

        // Visit the starting scene
        game_state.visit_scene(&story.starting_scene_id);
        
        // Apply starting scene effects if any
        if let Some(starting_scene) = story.get_scene(&story.starting_scene_id) {
            if let Some(effects) = &starting_scene.effects {
                self.apply_effects(&mut game_state, effects).await?;
            }
        }

        self.game_state = Some(game_state);
        
        self.emit_event(GameEvent::game_started(&story.id, &player_name)).await;
        
        Ok(())
    }

    pub async fn load_game(&mut self, game_state: GameState) -> GameResult<()> {
        let story = self.story.as_ref()
            .ok_or_else(|| GameError::story("No story loaded".to_string()))?;

        if game_state.story_id != story.id {
            return Err(GameError::story("Game state story ID does not match loaded story".to_string()));
        }

        info!("Loading game state for player: {}", game_state.player.name);
        
        self.game_state = Some(game_state);
        self.emit_event(GameEvent::game_loaded("loaded_game")).await;
        
        Ok(())
    }

    pub async fn get_current_scene(&self) -> GameResult<Scene> {
        let story = self.story.as_ref()
            .ok_or_else(|| GameError::story("No story loaded".to_string()))?;
        
        let game_state = self.game_state.as_ref()
            .ok_or_else(|| GameError::story("No active game".to_string()))?;

        let scene = story.get_scene(&game_state.current_scene_id)
            .ok_or_else(|| GameError::scene_not_found(&game_state.current_scene_id))?
            .clone();

        // Process the scene (filter choices based on conditions, etc.)
        Ok(self.process_scene(scene, game_state).await?)
    }

    pub async fn make_choice(&mut self, choice_id: &str) -> GameResult<()> {
        let current_scene = self.get_current_scene().await?;
        
        let choice = current_scene.get_choice(choice_id)
            .ok_or_else(|| GameError::choice_not_found(choice_id))?;

        if choice.disabled.unwrap_or(false) {
            return Err(GameError::story(format!(
                "Choice is disabled: {}", 
                choice.disabled_reason.as_deref().unwrap_or("Unknown reason")
            )));
        }

        info!("Player chose: {} ({})", choice.text, choice_id);

        let game_state = self.game_state.as_mut()
            .ok_or_else(|| GameError::story("No active game".to_string()))?;

        // Emit choice made event
        self.emit_event(GameEvent::choice_made(choice, &current_scene.id)).await;

        // Apply choice effects
        if let Some(effects) = &choice.effects {
            self.apply_effects(game_state, effects).await?;
        }

        // Move to target scene
        let old_scene_id = game_state.current_scene_id.clone();
        game_state.visit_scene(&choice.target_scene_id);

        // Apply target scene effects
        if let Some(story) = &self.story {
            if let Some(target_scene) = story.get_scene(&choice.target_scene_id) {
                self.emit_event(GameEvent::scene_entered(target_scene)).await;
                
                if let Some(effects) = &target_scene.effects {
                    self.apply_effects(game_state, effects).await?;
                }
            }
        }

        debug!("Moved from scene '{}' to '{}'", old_scene_id, choice.target_scene_id);
        Ok(())
    }

    pub fn get_game_state(&self) -> Option<&GameState> {
        self.game_state.as_ref()
    }

    pub fn get_game_state_mut(&mut self) -> Option<&mut GameState> {
        self.game_state.as_mut()
    }

    pub fn is_game_active(&self) -> bool {
        self.story.is_some() && self.game_state.is_some()
    }

    pub async fn is_game_ended(&self) -> bool {
        if let Ok(current_scene) = self.get_current_scene().await {
            current_scene.is_ending()
        } else {
            false
        }
    }

    pub async fn save_game(&mut self, save_name: String) -> GameResult<GameState> {
        let game_state = self.game_state.as_mut()
            .ok_or_else(|| GameError::save_load("No active game to save".to_string()))?;

        game_state.mark_saved();
        
        self.emit_event(GameEvent::game_saved(&save_name)).await;
        info!("Game saved: {}", save_name);
        
        Ok(game_state.clone())
    }

    async fn process_scene(&self, mut scene: Scene, game_state: &GameState) -> GameResult<Scene> {
        // Process choices - filter and update based on conditions
        let mut processed_choices = Vec::new();
        
        for choice in scene.choices {
            let mut processed_choice = choice.clone();
            
            // Check if choice should be disabled based on conditions
            if let Some(conditions) = &choice.conditions {
                if !self.check_conditions(conditions, game_state).await? {
                    processed_choice.disabled = Some(true);
                    if processed_choice.disabled_reason.is_none() {
                        processed_choice.disabled_reason = Some("Requirements not met".to_string());
                    }
                }
            }
            
            processed_choices.push(processed_choice);
        }
        
        scene.choices = processed_choices;
        Ok(scene)
    }

    async fn check_conditions(&self, conditions: &[Condition], game_state: &GameState) -> GameResult<bool> {
        for condition in conditions {
            if !self.check_condition(condition, game_state).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn check_condition(&self, condition: &Condition, game_state: &GameState) -> GameResult<bool> {
        let actual_value = match &condition.condition_type {
            ConditionType::Flag => {
                game_state.get_flag(&condition.key).cloned()
                    .unwrap_or(serde_json::Value::Null)
            }
            ConditionType::Stat => {
                let stat_value = match condition.key.as_str() {
                    "health" => game_state.player.stats.health,
                    "max_health" => game_state.player.stats.max_health,
                    "experience" => game_state.player.stats.experience,
                    "level" => game_state.player.stats.level,
                    "strength" => game_state.player.stats.strength,
                    "intelligence" => game_state.player.stats.intelligence,
                    "charisma" => game_state.player.stats.charisma,
                    _ => return Err(GameError::story(format!("Unknown stat: {}", condition.key))),
                };
                serde_json::Value::Number(serde_json::Number::from(stat_value))
            }
            ConditionType::Inventory => {
                let quantity = game_state.player.get_item(&condition.key)
                    .map(|item| item.quantity)
                    .unwrap_or(0);
                serde_json::Value::Number(serde_json::Number::from(quantity))
            }
            ConditionType::SceneVisited => {
                serde_json::Value::Bool(game_state.has_visited_scene(&condition.key))
            }
            ConditionType::Level => {
                serde_json::Value::Number(serde_json::Number::from(game_state.player.stats.level))
            }
            ConditionType::Custom => {
                // For custom conditions, we'll just return the flag value or false
                game_state.get_flag(&condition.key).cloned()
                    .unwrap_or(serde_json::Value::Bool(false))
            }
        };

        self.compare_values(&actual_value, &condition.operator, &condition.value)
    }

    fn compare_values(
        &self,
        actual: &serde_json::Value,
        operator: &ComparisonOperator,
        expected: &serde_json::Value,
    ) -> GameResult<bool> {
        match operator {
            ComparisonOperator::Equals => Ok(actual == expected),
            ComparisonOperator::NotEquals => Ok(actual != expected),
            ComparisonOperator::GreaterThan => {
                match (actual.as_i64(), expected.as_i64()) {
                    (Some(a), Some(e)) => Ok(a > e),
                    _ => Ok(false),
                }
            }
            ComparisonOperator::LessThan => {
                match (actual.as_i64(), expected.as_i64()) {
                    (Some(a), Some(e)) => Ok(a < e),
                    _ => Ok(false),
                }
            }
            ComparisonOperator::GreaterEqual => {
                match (actual.as_i64(), expected.as_i64()) {
                    (Some(a), Some(e)) => Ok(a >= e),
                    _ => Ok(false),
                }
            }
            ComparisonOperator::LessEqual => {
                match (actual.as_i64(), expected.as_i64()) {
                    (Some(a), Some(e)) => Ok(a <= e),
                    _ => Ok(false),
                }
            }
            ComparisonOperator::Has => Ok(!actual.is_null()),
            ComparisonOperator::NotHas => Ok(actual.is_null()),
            ComparisonOperator::Contains => {
                match (actual.as_str(), expected.as_str()) {
                    (Some(a), Some(e)) => Ok(a.contains(e)),
                    _ => Ok(false),
                }
            }
            ComparisonOperator::NotContains => {
                match (actual.as_str(), expected.as_str()) {
                    (Some(a), Some(e)) => Ok(!a.contains(e)),
                    _ => Ok(true),
                }
            }
        }
    }

    async fn apply_effects(&mut self, game_state: &mut GameState, effects: &[Effect]) -> GameResult<()> {
        for effect in effects {
            self.apply_effect(game_state, effect).await?;
        }
        Ok(())
    }

    async fn apply_effect(&mut self, game_state: &mut GameState, effect: &Effect) -> GameResult<()> {
        match &effect.effect_type {
            EffectType::SetFlag => {
                let old_value = game_state.get_flag(&effect.key).cloned();
                game_state.set_flag(&effect.key, effect.value.clone());
                self.emit_event(GameEvent::flag_set(&effect.key, &effect.value)).await;
                debug!("Set flag '{}' to {:?} (was: {:?})", effect.key, effect.value, old_value);
            }
            EffectType::ModifyStat => {
                if let Some(value) = effect.value.as_i64() {
                    let operation = match effect.operation.as_ref().unwrap_or(&EffectOperation::Set) {
                        EffectOperation::Set => crate::core::player::StatOperation::Set,
                        EffectOperation::Add => crate::core::player::StatOperation::Add,
                        EffectOperation::Subtract => crate::core::player::StatOperation::Subtract,
                        EffectOperation::Multiply => crate::core::player::StatOperation::Multiply,
                    };

                    let old_value = match effect.key.as_str() {
                        "health" => game_state.player.stats.health,
                        "max_health" => game_state.player.stats.max_health,
                        "experience" => game_state.player.stats.experience,
                        "level" => game_state.player.stats.level,
                        "strength" => game_state.player.stats.strength,
                        "intelligence" => game_state.player.stats.intelligence,
                        "charisma" => game_state.player.stats.charisma,
                        _ => 0,
                    };

                    game_state.player.modify_stat(&effect.key, value as i32, operation)?;

                    let new_value = match effect.key.as_str() {
                        "health" => game_state.player.stats.health,
                        "max_health" => game_state.player.stats.max_health,
                        "experience" => game_state.player.stats.experience,
                        "level" => game_state.player.stats.level,
                        "strength" => game_state.player.stats.strength,
                        "intelligence" => game_state.player.stats.intelligence,
                        "charisma" => game_state.player.stats.charisma,
                        _ => 0,
                    };

                    self.emit_event(GameEvent::stat_modified(&effect.key, old_value, new_value)).await;

                    // Check for level up
                    if effect.key == "experience" && new_value != old_value {
                        let current_level = game_state.player.stats.level;
                        if current_level > old_value {
                            self.emit_event(GameEvent::level_up(old_value, current_level, game_state.player.stats.experience)).await;
                        }
                    }

                    // Check for player death
                    if effect.key == "health" && new_value <= 0 {
                        self.emit_event(GameEvent::player_died("Health reached zero")).await;
                    }
                }
            }
            EffectType::AddItem => {
                if let Ok(item) = serde_json::from_value::<crate::core::InventoryItem>(effect.value.clone()) {
                    game_state.player.add_item(item.clone());
                    self.emit_event(GameEvent::item_added(&item.id, &item.name, item.quantity)).await;
                    debug!("Added item '{}' ({})", item.name, item.quantity);
                }
            }
            EffectType::RemoveItem => {
                if let Some(item_data) = effect.value.as_object() {
                    if let (Some(item_id), Some(quantity)) = (
                        item_data.get("id").and_then(|v| v.as_str()),
                        item_data.get("quantity").and_then(|v| v.as_i64())
                    ) {
                        let item_name = game_state.player.get_item(item_id)
                            .map(|item| item.name.clone())
                            .unwrap_or_else(|| item_id.to_string());

                        if game_state.player.remove_item(item_id, quantity as i32).is_ok() {
                            self.emit_event(GameEvent::item_removed(item_id, &item_name, quantity as i32)).await;
                            debug!("Removed item '{}' ({})", item_name, quantity);
                        }
                    }
                }
            }
            EffectType::ModifyHealth => {
                if let Some(value) = effect.value.as_i64() {
                    let operation = match effect.operation.as_ref().unwrap_or(&EffectOperation::Add) {
                        EffectOperation::Set => crate::core::player::StatOperation::Set,
                        EffectOperation::Add => crate::core::player::StatOperation::Add,
                        EffectOperation::Subtract => crate::core::player::StatOperation::Subtract,
                        EffectOperation::Multiply => crate::core::player::StatOperation::Multiply,
                    };

                    let old_health = game_state.player.stats.health;
                    game_state.player.modify_stat("health", value as i32, operation)?;
                    let new_health = game_state.player.stats.health;

                    self.emit_event(GameEvent::stat_modified("health", old_health, new_health)).await;

                    if new_health <= 0 {
                        self.emit_event(GameEvent::player_died("Health reached zero")).await;
                    }
                }
            }
            EffectType::Custom => {
                // Custom effects can be handled by the game or ignored
                debug!("Applied custom effect: {} -> {:?}", effect.key, effect.value);
                self.emit_event(GameEvent::custom(&format!("custom_effect_{}", effect.key), effect.value.clone())).await;
            }
        }

        Ok(())
    }

    async fn emit_event(&self, event: GameEvent) {
        if let Ok(mut handler) = self.event_handler.try_lock() {
            handler.handle_event(&event);
        }
    }

    pub async fn get_event_history(&self) -> Vec<GameEvent> {
        if let Ok(handler) = self.event_handler.try_lock() {
            handler.get_events().to_vec()
        } else {
            Vec::new()
        }
    }

    pub async fn get_recent_events(&self, count: usize) -> Vec<GameEvent> {
        if let Ok(handler) = self.event_handler.try_lock() {
            handler.get_recent_events(count).into_iter().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for GameEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::{Scene, Choice};

    #[tokio::test]
    async fn test_game_engine_creation() {
        let engine = GameEngine::new();
        assert!(!engine.is_game_active());
    }

    #[tokio::test]
    async fn test_load_story() {
        let mut engine = GameEngine::new();
        let story = Story::new("test", "Test Story", "start", PlayerStats::default());
        
        // Should fail - no starting scene
        assert!(engine.load_story(story).await.is_err());
        
        // Create valid story
        let mut story = Story::new("test", "Test Story", "start", PlayerStats::default());
        story.add_scene(Scene::new("start", "Start", "Starting scene"));
        
        assert!(engine.load_story(story).await.is_ok());
    }

    #[tokio::test]
    async fn test_start_new_game() {
        let mut engine = GameEngine::new();
        
        // Should fail - no story loaded
        assert!(engine.start_new_game("Test Player".to_string()).await.is_err());
        
        // Load story and try again
        let mut story = Story::new("test", "Test Story", "start", PlayerStats::default());
        story.add_scene(Scene::new("start", "Start", "Starting scene"));
        engine.load_story(story).await.unwrap();
        
        assert!(engine.start_new_game("Test Player".to_string()).await.is_ok());
        assert!(engine.is_game_active());
        
        let game_state = engine.get_game_state().unwrap();
        assert_eq!(game_state.player.name, "Test Player");
        assert_eq!(game_state.current_scene_id, "start");
    }

    #[tokio::test]
    async fn test_make_choice() {
        let mut engine = GameEngine::new();
        
        // Create story with choices
        let mut story = Story::new("test", "Test Story", "start", PlayerStats::default());
        
        let mut start_scene = Scene::new("start", "Start", "Starting scene");
        start_scene.add_choice(Choice::new("go_forward", "Go forward", "next"));
        
        let next_scene = Scene::new("next", "Next Scene", "You moved forward");
        
        story.add_scene(start_scene);
        story.add_scene(next_scene);
        
        engine.load_story(story).await.unwrap();
        engine.start_new_game("Test Player".to_string()).await.unwrap();
        
        // Make choice
        assert!(engine.make_choice("go_forward").await.is_ok());
        
        let game_state = engine.get_game_state().unwrap();
        assert_eq!(game_state.current_scene_id, "next");
        assert!(game_state.has_visited_scene("start"));
        assert!(game_state.has_visited_scene("next"));
    }
}