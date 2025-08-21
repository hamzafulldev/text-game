use std::path::{Path, PathBuf};
use tokio::fs;
use crate::story::Story;
use crate::utils::{GameError, GameResult};
use tracing::{info, warn, error};

pub struct StoryLoader {
    stories_directory: PathBuf,
}

impl StoryLoader {
    pub fn new<P: AsRef<Path>>(stories_directory: P) -> Self {
        Self {
            stories_directory: stories_directory.as_ref().to_path_buf(),
        }
    }

    pub async fn load_story(&self, story_id: &str) -> GameResult<Story> {
        let story_path = self.stories_directory.join(format!("{}.json", story_id));
        
        info!("Loading story from: {:?}", story_path);
        
        if !story_path.exists() {
            return Err(GameError::story(format!("Story file not found: {}", story_id)));
        }

        let content = fs::read_to_string(&story_path)
            .await
            .map_err(|e| GameError::story(format!("Failed to read story file: {}", e)))?;

        let story: Story = serde_json::from_str(&content)
            .map_err(|e| GameError::story(format!("Failed to parse story JSON: {}", e)))?;

        // Validate the story
        if let Err(errors) = story.validate() {
            let error_msg = errors.join("; ");
            return Err(GameError::story(format!("Story validation failed: {}", error_msg)));
        }

        info!("Successfully loaded story: {} ({})", story.title, story.id);
        Ok(story)
    }

    pub async fn list_available_stories(&self) -> GameResult<Vec<StoryMetadata>> {
        info!("Scanning for stories in: {:?}", self.stories_directory);
        
        if !self.stories_directory.exists() {
            warn!("Stories directory does not exist, creating: {:?}", self.stories_directory);
            fs::create_dir_all(&self.stories_directory)
                .await
                .map_err(|e| GameError::story(format!("Failed to create stories directory: {}", e)))?;
            return Ok(Vec::new());
        }

        let mut entries = fs::read_dir(&self.stories_directory)
            .await
            .map_err(|e| GameError::story(format!("Failed to read stories directory: {}", e)))?;

        let mut stories = Vec::new();

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| GameError::story(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_story_metadata(&path).await {
                    Ok(metadata) => stories.push(metadata),
                    Err(e) => {
                        warn!("Failed to load metadata for story at {:?}: {}", path, e);
                        continue;
                    }
                }
            }
        }

        // Sort by title
        stories.sort_by(|a, b| a.title.cmp(&b.title));
        
        info!("Found {} stories", stories.len());
        Ok(stories)
    }

    pub async fn story_exists(&self, story_id: &str) -> bool {
        let story_path = self.stories_directory.join(format!("{}.json", story_id));
        story_path.exists()
    }

    pub async fn save_story(&self, story: &Story) -> GameResult<()> {
        // Validate before saving
        if let Err(errors) = story.validate() {
            let error_msg = errors.join("; ");
            return Err(GameError::story(format!("Cannot save invalid story: {}", error_msg)));
        }

        let story_path = self.stories_directory.join(format!("{}.json", story.id));
        
        // Create directory if it doesn't exist
        if let Some(parent) = story_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| GameError::story(format!("Failed to create directory: {}", e)))?;
        }

        let json = serde_json::to_string_pretty(story)
            .map_err(|e| GameError::story(format!("Failed to serialize story: {}", e)))?;

        fs::write(&story_path, json)
            .await
            .map_err(|e| GameError::story(format!("Failed to write story file: {}", e)))?;

        info!("Saved story: {} to {:?}", story.id, story_path);
        Ok(())
    }

    pub async fn delete_story(&self, story_id: &str) -> GameResult<()> {
        let story_path = self.stories_directory.join(format!("{}.json", story_id));
        
        if !story_path.exists() {
            return Err(GameError::story(format!("Story not found: {}", story_id)));
        }

        fs::remove_file(&story_path)
            .await
            .map_err(|e| GameError::story(format!("Failed to delete story: {}", e)))?;

        info!("Deleted story: {}", story_id);
        Ok(())
    }

    pub async fn create_story_template(&self, story_id: &str, title: &str, author: &str) -> GameResult<Story> {
        if self.story_exists(story_id).await {
            return Err(GameError::story(format!("Story already exists: {}", story_id)));
        }

        let story = self.create_basic_story_template(story_id, title, author);
        self.save_story(&story).await?;
        
        info!("Created story template: {}", story_id);
        Ok(story)
    }

    async fn load_story_metadata(&self, path: &Path) -> GameResult<StoryMetadata> {
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| GameError::story(format!("Failed to read story file: {}", e)))?;

        // Parse just the metadata we need
        let value: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| GameError::story(format!("Failed to parse story JSON: {}", e)))?;

        Ok(StoryMetadata {
            id: value.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            title: value.get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string(),
            description: value.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("No description available")
                .to_string(),
            author: value.get("author")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string(),
            version: value.get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("1.0.0")
                .to_string(),
            scene_count: value.get("scenes")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0),
        })
    }

    fn create_basic_story_template(&self, story_id: &str, title: &str, author: &str) -> Story {
        use crate::story::{Scene, Choice};
        use crate::core::PlayerStats;

        let mut story = Story::new(story_id, title, "start", PlayerStats::default());
        story.author = author.to_string();
        story.description = "A new adventure awaits...".to_string();

        // Create starting scene
        let mut start_scene = Scene::new(
            "start",
            "The Beginning",
            "Your adventure starts here. What will you do?"
        );
        
        start_scene.add_choice(Choice::new(
            "explore",
            "Explore the area",
            "explore"
        ));
        
        start_scene.add_choice(Choice::new(
            "rest",
            "Rest and think",
            "rest"
        ));

        // Create exploration scene
        let mut explore_scene = Scene::new(
            "explore",
            "Exploration",
            "You decide to explore your surroundings."
        );
        
        explore_scene.add_choice(Choice::new(
            "return",
            "Return to the beginning",
            "start"
        ));

        // Create rest scene
        let mut rest_scene = Scene::new(
            "rest",
            "Contemplation",
            "You take a moment to rest and gather your thoughts."
        );
        
        rest_scene.add_choice(Choice::new(
            "continue",
            "Continue your journey",
            "start"
        ));

        story.add_scene(start_scene);
        story.add_scene(explore_scene);
        story.add_scene(rest_scene);

        story
    }
}

#[derive(Debug, Clone)]
pub struct StoryMetadata {
    pub id: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub scene_count: usize,
}

impl StoryMetadata {
    pub fn display_name(&self) -> String {
        format!("{} by {} (v{})", self.title, self.author, self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_story_loader_creation() {
        let temp_dir = tempdir().unwrap();
        let loader = StoryLoader::new(temp_dir.path());
        
        let stories = loader.list_available_stories().await.unwrap();
        assert!(stories.is_empty());
    }

    #[tokio::test]
    async fn test_story_template_creation() {
        let temp_dir = tempdir().unwrap();
        let loader = StoryLoader::new(temp_dir.path());
        
        let story = loader.create_story_template("test", "Test Story", "Test Author").await.unwrap();
        assert_eq!(story.id, "test");
        assert_eq!(story.title, "Test Story");
        assert_eq!(story.author, "Test Author");
        assert!(!story.scenes.is_empty());
    }
}