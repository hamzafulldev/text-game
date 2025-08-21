use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::core::GameState;
use crate::utils::{GameError, GameResult};
use tracing::{info, warn, error, debug};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveGame {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub game_state: GameState,
    pub save_time: DateTime<Utc>,
    pub version: String,
    pub metadata: Option<serde_json::Value>,
}

pub struct SaveManager {
    saves_directory: PathBuf,
}

impl SaveManager {
    pub fn new<P: AsRef<Path>>(saves_directory: P) -> Self {
        Self {
            saves_directory: saves_directory.as_ref().to_path_buf(),
        }
    }

    pub async fn save_game(&self, name: String, game_state: GameState, description: Option<String>) -> GameResult<SaveGame> {
        info!("Saving game: {}", name);

        // Create saves directory if it doesn't exist
        if !self.saves_directory.exists() {
            fs::create_dir_all(&self.saves_directory)
                .await
                .map_err(|e| GameError::save_load(format!("Failed to create saves directory: {}", e)))?;
        }

        let save_game = SaveGame {
            id: Uuid::new_v4(),
            name: name.clone(),
            description,
            game_state,
            save_time: Utc::now(),
            version: crate::VERSION.to_string(),
            metadata: None,
        };

        let save_path = self.get_save_path(&save_game.id);
        let json = serde_json::to_string_pretty(&save_game)
            .map_err(|e| GameError::save_load(format!("Failed to serialize save game: {}", e)))?;

        fs::write(&save_path, json)
            .await
            .map_err(|e| GameError::save_load(format!("Failed to write save file: {}", e)))?;

        info!("Game saved successfully: {} ({})", name, save_game.id);
        debug!("Save file written to: {:?}", save_path);

        Ok(save_game)
    }

    pub async fn load_game(&self, save_id: Uuid) -> GameResult<SaveGame> {
        let save_path = self.get_save_path(&save_id);
        
        if !save_path.exists() {
            return Err(GameError::save_load(format!("Save file not found: {}", save_id)));
        }

        info!("Loading game: {}", save_id);

        let content = fs::read_to_string(&save_path)
            .await
            .map_err(|e| GameError::save_load(format!("Failed to read save file: {}", e)))?;

        let save_game: SaveGame = serde_json::from_str(&content)
            .map_err(|e| GameError::save_load(format!("Failed to parse save file: {}", e)))?;

        // Validate version compatibility (for now, just warn on mismatch)
        if save_game.version != crate::VERSION {
            warn!("Save game version mismatch: {} vs {}", save_game.version, crate::VERSION);
        }

        info!("Game loaded successfully: {}", save_game.name);
        Ok(save_game)
    }

    pub async fn list_save_games(&self) -> GameResult<Vec<SaveGameMetadata>> {
        debug!("Scanning for save games in: {:?}", self.saves_directory);

        if !self.saves_directory.exists() {
            return Ok(Vec::new());
        }

        let mut entries = fs::read_dir(&self.saves_directory)
            .await
            .map_err(|e| GameError::save_load(format!("Failed to read saves directory: {}", e)))?;

        let mut save_games = Vec::new();

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| GameError::save_load(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_save_metadata(&path).await {
                    Ok(metadata) => save_games.push(metadata),
                    Err(e) => {
                        warn!("Failed to load save metadata from {:?}: {}", path, e);
                        continue;
                    }
                }
            }
        }

        // Sort by save time (newest first)
        save_games.sort_by(|a, b| b.save_time.cmp(&a.save_time));
        
        info!("Found {} save games", save_games.len());
        Ok(save_games)
    }

    pub async fn delete_save(&self, save_id: Uuid) -> GameResult<()> {
        let save_path = self.get_save_path(&save_id);
        
        if !save_path.exists() {
            return Err(GameError::save_load(format!("Save file not found: {}", save_id)));
        }

        fs::remove_file(&save_path)
            .await
            .map_err(|e| GameError::save_load(format!("Failed to delete save file: {}", e)))?;

        info!("Deleted save game: {}", save_id);
        Ok(())
    }

    pub async fn save_exists(&self, save_id: Uuid) -> bool {
        self.get_save_path(&save_id).exists()
    }

    pub async fn get_save_count(&self) -> GameResult<usize> {
        if !self.saves_directory.exists() {
            return Ok(0);
        }

        let mut entries = fs::read_dir(&self.saves_directory)
            .await
            .map_err(|e| GameError::save_load(format!("Failed to read saves directory: {}", e)))?;

        let mut count = 0;
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| GameError::save_load(format!("Failed to read directory entry: {}", e)))? {
            
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                count += 1;
            }
        }

        Ok(count)
    }

    pub async fn cleanup_old_saves(&self, keep_count: usize) -> GameResult<usize> {
        let mut save_games = self.list_save_games().await?;
        
        if save_games.len() <= keep_count {
            return Ok(0);
        }

        // Sort by save time (oldest first for deletion)
        save_games.sort_by(|a, b| a.save_time.cmp(&b.save_time));
        
        let to_delete = save_games.len() - keep_count;
        let mut deleted = 0;

        for save_metadata in save_games.iter().take(to_delete) {
            match self.delete_save(save_metadata.id).await {
                Ok(()) => {
                    deleted += 1;
                    info!("Deleted old save: {}", save_metadata.name);
                }
                Err(e) => {
                    error!("Failed to delete old save {}: {}", save_metadata.name, e);
                }
            }
        }

        info!("Cleaned up {} old save games", deleted);
        Ok(deleted)
    }

    pub async fn export_save(&self, save_id: Uuid, export_path: &Path) -> GameResult<()> {
        let save_game = self.load_game(save_id).await?;
        
        let json = serde_json::to_string_pretty(&save_game)
            .map_err(|e| GameError::save_load(format!("Failed to serialize save for export: {}", e)))?;

        fs::write(export_path, json)
            .await
            .map_err(|e| GameError::save_load(format!("Failed to write export file: {}", e)))?;

        info!("Exported save game to: {:?}", export_path);
        Ok(())
    }

    pub async fn import_save(&self, import_path: &Path) -> GameResult<SaveGame> {
        if !import_path.exists() {
            return Err(GameError::save_load("Import file not found".to_string()));
        }

        let content = fs::read_to_string(import_path)
            .await
            .map_err(|e| GameError::save_load(format!("Failed to read import file: {}", e)))?;

        let mut save_game: SaveGame = serde_json::from_str(&content)
            .map_err(|e| GameError::save_load(format!("Failed to parse import file: {}", e)))?;

        // Generate new ID to avoid conflicts
        save_game.id = Uuid::new_v4();
        save_game.name = format!("{} (Imported)", save_game.name);

        // Save the imported game
        let save_path = self.get_save_path(&save_game.id);
        let json = serde_json::to_string_pretty(&save_game)
            .map_err(|e| GameError::save_load(format!("Failed to serialize imported save: {}", e)))?;

        fs::write(&save_path, json)
            .await
            .map_err(|e| GameError::save_load(format!("Failed to write imported save: {}", e)))?;

        info!("Imported save game: {}", save_game.name);
        Ok(save_game)
    }

    async fn load_save_metadata(&self, path: &Path) -> GameResult<SaveGameMetadata> {
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| GameError::save_load(format!("Failed to read save file: {}", e)))?;

        // Parse just the metadata we need
        let value: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| GameError::save_load(format!("Failed to parse save file: {}", e)))?;

        let id_str = value.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GameError::save_load("Save file missing ID".to_string()))?;

        let id = Uuid::parse_str(id_str)
            .map_err(|e| GameError::save_load(format!("Invalid save ID: {}", e)))?;

        let save_time_str = value.get("save_time")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GameError::save_load("Save file missing save_time".to_string()))?;

        let save_time = DateTime::parse_from_rfc3339(save_time_str)
            .map_err(|e| GameError::save_load(format!("Invalid save time format: {}", e)))?
            .with_timezone(&Utc);

        // Extract player name and level from game state
        let player_name = value.get("game_state")
            .and_then(|gs| gs.get("player"))
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let player_level = value.get("game_state")
            .and_then(|gs| gs.get("player"))
            .and_then(|p| p.get("stats"))
            .and_then(|s| s.get("level"))
            .and_then(|v| v.as_i64())
            .unwrap_or(1) as i32;

        let story_id = value.get("game_state")
            .and_then(|gs| gs.get("story_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let playtime = value.get("game_state")
            .and_then(|gs| gs.get("playtime_seconds"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        Ok(SaveGameMetadata {
            id,
            name: value.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string(),
            description: value.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            save_time,
            version: value.get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            story_id,
            player_name,
            player_level,
            playtime_seconds: playtime,
        })
    }

    fn get_save_path(&self, save_id: &Uuid) -> PathBuf {
        self.saves_directory.join(format!("{}.json", save_id))
    }
}

#[derive(Debug, Clone)]
pub struct SaveGameMetadata {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub save_time: DateTime<Utc>,
    pub version: String,
    pub story_id: String,
    pub player_name: String,
    pub player_level: i32,
    pub playtime_seconds: i64,
}

impl SaveGameMetadata {
    pub fn display_name(&self) -> String {
        format!(
            "{} - {} (Level {}) - {}",
            self.name,
            self.player_name,
            self.player_level,
            self.save_time.format("%Y-%m-%d %H:%M")
        )
    }

    pub fn get_playtime_formatted(&self) -> String {
        let hours = self.playtime_seconds / 3600;
        let minutes = (self.playtime_seconds % 3600) / 60;
        let seconds = self.playtime_seconds % 60;

        if hours > 0 {
            format!("{}h {}m", hours, minutes)
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
    use tempfile::tempdir;
    use crate::core::{Player, PlayerStats};

    #[tokio::test]
    async fn test_save_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let save_manager = SaveManager::new(temp_dir.path());
        
        let count = save_manager.get_save_count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_save_and_load_game() {
        let temp_dir = tempdir().unwrap();
        let save_manager = SaveManager::new(temp_dir.path());
        
        let player = Player::new("Test Player", Some(PlayerStats::default()));
        let game_state = crate::core::GameState::new(
            "test_story".to_string(),
            "start".to_string(),
            player,
        );

        // Save game
        let save_game = save_manager.save_game(
            "Test Save".to_string(),
            game_state,
            Some("A test save".to_string())
        ).await.unwrap();

        assert_eq!(save_game.name, "Test Save");
        assert_eq!(save_game.description, Some("A test save".to_string()));

        // Load game
        let loaded_save = save_manager.load_game(save_game.id).await.unwrap();
        assert_eq!(loaded_save.name, "Test Save");
        assert_eq!(loaded_save.game_state.player.name, "Test Player");
    }

    #[tokio::test]
    async fn test_list_save_games() {
        let temp_dir = tempdir().unwrap();
        let save_manager = SaveManager::new(temp_dir.path());
        
        // Initially empty
        let saves = save_manager.list_save_games().await.unwrap();
        assert!(saves.is_empty());

        // Add some saves
        for i in 0..3 {
            let player = Player::new(format!("Player {}", i), Some(PlayerStats::default()));
            let game_state = crate::core::GameState::new(
                "test_story".to_string(),
                "start".to_string(),
                player,
            );

            save_manager.save_game(
                format!("Save {}", i),
                game_state,
                None
            ).await.unwrap();
        }

        let saves = save_manager.list_save_games().await.unwrap();
        assert_eq!(saves.len(), 3);
        
        // Should be sorted by save time (newest first)
        assert_eq!(saves[0].name, "Save 2");
        assert_eq!(saves[1].name, "Save 1");
        assert_eq!(saves[2].name, "Save 0");
    }

    #[tokio::test]
    async fn test_delete_save() {
        let temp_dir = tempdir().unwrap();
        let save_manager = SaveManager::new(temp_dir.path());
        
        let player = Player::new("Test Player", Some(PlayerStats::default()));
        let game_state = crate::core::GameState::new(
            "test_story".to_string(),
            "start".to_string(),
            player,
        );

        let save_game = save_manager.save_game(
            "Test Save".to_string(),
            game_state,
            None
        ).await.unwrap();

        assert!(save_manager.save_exists(save_game.id).await);

        save_manager.delete_save(save_game.id).await.unwrap();
        assert!(!save_manager.save_exists(save_game.id).await);
    }

    #[tokio::test]
    async fn test_cleanup_old_saves() {
        let temp_dir = tempdir().unwrap();
        let save_manager = SaveManager::new(temp_dir.path());
        
        // Create 5 saves
        for i in 0..5 {
            let player = Player::new(format!("Player {}", i), Some(PlayerStats::default()));
            let game_state = crate::core::GameState::new(
                "test_story".to_string(),
                "start".to_string(),
                player,
            );

            save_manager.save_game(
                format!("Save {}", i),
                game_state,
                None
            ).await.unwrap();

            // Small delay to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        assert_eq!(save_manager.get_save_count().await.unwrap(), 5);

        // Keep only 3 newest saves
        let deleted = save_manager.cleanup_old_saves(3).await.unwrap();
        assert_eq!(deleted, 2);
        assert_eq!(save_manager.get_save_count().await.unwrap(), 3);
    }
}