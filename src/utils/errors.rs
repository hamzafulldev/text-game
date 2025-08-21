use thiserror::Error;

pub type GameResult<T> = Result<T, GameError>;

#[derive(Error, Debug)]
pub enum GameError {
    #[error("Story error: {message}")]
    Story { message: String },
    
    #[error("Scene not found: {scene_id}")]
    SceneNotFound { scene_id: String },
    
    #[error("Choice not found: {choice_id}")]
    ChoiceNotFound { choice_id: String },
    
    #[error("Save/Load error: {message}")]
    SaveLoad { message: String },
    
    #[error("Player error: {message}")]
    Player { message: String },
    
    #[error("Configuration error: {message}")]
    Configuration { message: String },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),
    
    #[error("UUID error: {0}")]
    Uuid(#[from] uuid::Error),
    
    #[error("Config error: {0}")]
    Config(#[from] config::ConfigError),
}

impl GameError {
    pub fn story<S: Into<String>>(message: S) -> Self {
        Self::Story {
            message: message.into(),
        }
    }
    
    pub fn scene_not_found<S: Into<String>>(scene_id: S) -> Self {
        Self::SceneNotFound {
            scene_id: scene_id.into(),
        }
    }
    
    pub fn choice_not_found<S: Into<String>>(choice_id: S) -> Self {
        Self::ChoiceNotFound {
            choice_id: choice_id.into(),
        }
    }
    
    pub fn save_load<S: Into<String>>(message: S) -> Self {
        Self::SaveLoad {
            message: message.into(),
        }
    }
    
    pub fn player<S: Into<String>>(message: S) -> Self {
        Self::Player {
            message: message.into(),
        }
    }
    
    pub fn configuration<S: Into<String>>(message: S) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = GameError::story("Test story error");
        assert!(matches!(error, GameError::Story { .. }));
        assert_eq!(error.to_string(), "Story error: Test story error");
    }

    #[test]
    fn test_scene_not_found_error() {
        let error = GameError::scene_not_found("test_scene");
        assert!(matches!(error, GameError::SceneNotFound { .. }));
        assert_eq!(error.to_string(), "Scene not found: test_scene");
    }
}