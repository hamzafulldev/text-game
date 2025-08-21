use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::utils::{GameError, GameResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub game: GameConfig,
    pub ui: UiConfig,
    pub paths: PathConfig,
    pub logging: LoggingConfig,
    pub saves: SaveConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub auto_save: bool,
    pub auto_save_interval_minutes: u32,
    pub max_recent_saves: usize,
    pub confirm_dangerous_choices: bool,
    pub show_choice_effects: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub show_stats_in_header: bool,
    pub show_scene_numbers: bool,
    pub animation_speed: AnimationSpeed,
    pub text_width: usize,
    pub page_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    pub stories_dir: PathBuf,
    pub saves_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub config_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub log_to_file: bool,
    pub max_log_files: usize,
    pub max_log_size_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveConfig {
    pub max_saves_per_story: usize,
    pub auto_cleanup_saves: bool,
    pub compress_saves: bool,
    pub backup_saves: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnimationSpeed {
    None,
    Slow,
    Medium,
    Fast,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            game: GameConfig {
                auto_save: true,
                auto_save_interval_minutes: 5,
                max_recent_saves: 10,
                confirm_dangerous_choices: true,
                show_choice_effects: false,
            },
            ui: UiConfig {
                theme: "default".to_string(),
                show_stats_in_header: true,
                show_scene_numbers: false,
                animation_speed: AnimationSpeed::Medium,
                text_width: 80,
                page_size: 10,
            },
            paths: PathConfig {
                stories_dir: PathBuf::from("./assets/stories"),
                saves_dir: PathBuf::from("./assets/saves"),
                logs_dir: PathBuf::from("./assets/logs"),
                config_dir: PathBuf::from("./assets/config"),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                log_to_file: true,
                max_log_files: 10,
                max_log_size_mb: 10,
            },
            saves: SaveConfig {
                max_saves_per_story: 50,
                auto_cleanup_saves: true,
                compress_saves: false,
                backup_saves: false,
            },
        }
    }
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> GameResult<Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            // Create default config file
            let default_config = Self::default();
            default_config.save_to_file(path)?;
            return Ok(default_config);
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| GameError::configuration(format!("Failed to read config file: {}", e)))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| GameError::configuration(format!("Failed to parse config file: {}", e)))?;

        Ok(config)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> GameResult<()> {
        let path = path.as_ref();
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| GameError::configuration(format!("Failed to create config directory: {}", e)))?;
        }

        let toml_content = toml::to_string_pretty(self)
            .map_err(|e| GameError::configuration(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(path, toml_content)
            .map_err(|e| GameError::configuration(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    pub fn get_stories_dir(&self) -> &Path {
        &self.paths.stories_dir
    }

    pub fn get_saves_dir(&self) -> &Path {
        &self.paths.saves_dir
    }

    pub fn get_logs_dir(&self) -> &Path {
        &self.paths.logs_dir
    }

    pub fn get_config_dir(&self) -> &Path {
        &self.paths.config_dir
    }

    pub fn ensure_directories(&self) -> GameResult<()> {
        let dirs = [
            &self.paths.stories_dir,
            &self.paths.saves_dir,
            &self.paths.logs_dir,
            &self.paths.config_dir,
        ];

        for dir in &dirs {
            if !dir.exists() {
                std::fs::create_dir_all(dir)
                    .map_err(|e| GameError::configuration(format!("Failed to create directory {:?}: {}", dir, e)))?;
            }
        }

        Ok(())
    }

    pub fn validate(&self) -> GameResult<()> {
        // Validate logging level
        match self.logging.level.as_str() {
            "error" | "warn" | "info" | "debug" | "trace" => {}
            _ => return Err(GameError::configuration("Invalid logging level")),
        }

        // Validate paths are not empty
        if self.paths.stories_dir.as_os_str().is_empty() {
            return Err(GameError::configuration("Stories directory path cannot be empty"));
        }
        if self.paths.saves_dir.as_os_str().is_empty() {
            return Err(GameError::configuration("Saves directory path cannot be empty"));
        }

        // Validate numeric values
        if self.game.auto_save_interval_minutes == 0 {
            return Err(GameError::configuration("Auto-save interval must be greater than 0"));
        }
        if self.game.max_recent_saves == 0 {
            return Err(GameError::configuration("Max recent saves must be greater than 0"));
        }
        if self.ui.text_width < 40 {
            return Err(GameError::configuration("Text width must be at least 40"));
        }
        if self.ui.page_size == 0 {
            return Err(GameError::configuration("Page size must be greater than 0"));
        }
        if self.saves.max_saves_per_story == 0 {
            return Err(GameError::configuration("Max saves per story must be greater than 0"));
        }

        Ok(())
    }

    pub fn merge_with_cli(&mut self, cli_config: CliConfig) {
        if let Some(stories_dir) = cli_config.stories_dir {
            self.paths.stories_dir = stories_dir;
        }
        if let Some(saves_dir) = cli_config.saves_dir {
            self.paths.saves_dir = saves_dir;
        }
        if let Some(log_level) = cli_config.log_level {
            self.logging.level = log_level;
        }
        if cli_config.debug {
            self.logging.level = "debug".to_string();
        }
        if let Some(theme) = cli_config.theme {
            self.ui.theme = theme;
        }
    }

    pub fn get_animation_delay_ms(&self) -> u64 {
        match self.ui.animation_speed {
            AnimationSpeed::None => 0,
            AnimationSpeed::Slow => 150,
            AnimationSpeed::Medium => 75,
            AnimationSpeed::Fast => 25,
        }
    }
}

// Configuration that can be overridden by CLI arguments
#[derive(Debug, Default)]
pub struct CliConfig {
    pub stories_dir: Option<PathBuf>,
    pub saves_dir: Option<PathBuf>,
    pub log_level: Option<String>,
    pub debug: bool,
    pub theme: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        
        assert!(config.game.auto_save);
        assert_eq!(config.game.auto_save_interval_minutes, 5);
        assert_eq!(config.ui.theme, "default");
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        // Test invalid logging level
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());

        // Reset and test invalid auto-save interval
        config = Config::default();
        config.game.auto_save_interval_minutes = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_save_and_load() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let original_config = Config::default();
        original_config.save_to_file(&config_path).unwrap();

        let loaded_config = Config::from_file(&config_path).unwrap();
        
        assert_eq!(original_config.game.auto_save, loaded_config.game.auto_save);
        assert_eq!(original_config.ui.theme, loaded_config.ui.theme);
        assert_eq!(original_config.logging.level, loaded_config.logging.level);
    }

    #[test]
    fn test_cli_config_merge() {
        let mut config = Config::default();
        let cli_config = CliConfig {
            log_level: Some("debug".to_string()),
            debug: false,
            theme: Some("dark".to_string()),
            ..Default::default()
        };

        config.merge_with_cli(cli_config);
        
        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.ui.theme, "dark");
    }

    #[test]
    fn test_animation_delay() {
        let mut config = Config::default();
        
        config.ui.animation_speed = AnimationSpeed::None;
        assert_eq!(config.get_animation_delay_ms(), 0);
        
        config.ui.animation_speed = AnimationSpeed::Slow;
        assert_eq!(config.get_animation_delay_ms(), 150);
        
        config.ui.animation_speed = AnimationSpeed::Medium;
        assert_eq!(config.get_animation_delay_ms(), 75);
        
        config.ui.animation_speed = AnimationSpeed::Fast;
        assert_eq!(config.get_animation_delay_ms(), 25);
    }
}