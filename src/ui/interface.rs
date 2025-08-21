use dialoguer::{Select, Input, Confirm, FuzzySelect};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use crate::core::GameEngine;
use crate::story::{StoryLoader, StoryMetadata};
use crate::utils::{SaveManager, SaveGameMetadata};
use crate::ui::{Display, ThemeManager};
use crate::config::Config;
use crate::utils::{GameError, GameResult};
use tracing::{info, warn, error};

pub struct GameInterface {
    engine: GameEngine,
    story_loader: StoryLoader,
    save_manager: SaveManager,
    display: Display,
    config: Config,
}

impl GameInterface {
    pub async fn new(config: Config) -> GameResult<Self> {
        info!("Initializing game interface");
        
        // Ensure directories exist
        config.ensure_directories()?;
        
        let theme_manager = ThemeManager::new();
        let mut display = Display::new(theme_manager, config.ui.text_width)
            .map_err(|e| GameError::configuration(format!("Failed to create display: {}", e)))?;
        
        // Set theme if configured
        if !display.set_theme(&config.ui.theme) {
            warn!("Unknown theme '{}', using default", config.ui.theme);
        }

        Ok(Self {
            engine: GameEngine::new(),
            story_loader: StoryLoader::new(config.get_stories_dir()),
            save_manager: SaveManager::new(config.get_saves_dir()),
            display,
            config,
        })
    }

    pub async fn run(&mut self) -> GameResult<()> {
        info!("Starting game interface");
        
        loop {
            match self.show_main_menu().await {
                Ok(should_continue) => {
                    if !should_continue {
                        break;
                    }
                }
                Err(e) => {
                    error!("Main menu error: {}", e);
                    self.display.show_error(&format!("An error occurred: {}", e)).ok();
                    self.display.wait_for_enter().ok();
                }
            }
        }

        self.display.show_message("Thank you for playing!", "success").ok();
        self.display.show_message("May your adventures continue in dreams and stories...", "info").ok();
        
        Ok(())
    }

    pub async fn show_main_menu(&mut self) -> GameResult<bool> {
        self.display.clear_screen().ok();
        self.show_game_title().await?;

        let choices = vec![
            "🎮 Start New Game",
            "📁 Load Game", 
            "⚙️ Settings",
            "📊 Statistics",
            "🚪 Exit"
        ];

        let selection = Select::new()
            .with_prompt("What would you like to do?")
            .items(&choices)
            .default(0)
            .interact()
            .map_err(|e| GameError::configuration(format!("Menu selection error: {}", e)))?;

        match selection {
            0 => self.start_new_game_menu().await?,
            1 => self.load_game_menu().await?,
            2 => self.settings_menu().await?,
            3 => self.statistics_menu().await?,
            4 => return Ok(false), // Exit
            _ => unreachable!(),
        }

        Ok(true)
    }

    async fn show_game_title(&mut self) -> GameResult<()> {
        // ASCII art title
        let title = r#"
╔╦╗┌─┐─┐ ┬┌┬┐  ╔═╗┌┬┐┬  ┬┌─┐┌┐┌┌┬┐┬ ┬┬─┐┌─┐  ╔═╗┌─┐┌┬┐┌─┐
 ║ ├┤ ┌┴┬┘ │   ╠═╣ │││└┐┌┘├┤ │││ │ │ │├┬┘├┤   ║ ╦├─┤│││├┤ 
 ╩ └─┘┴ └─ ╩   ╩ ╩─┴┘┴ └┘ └─┘┘└┘ ╩ └─┘┴└─└─┘  ╚═╝┴ ┴┴ ┴└─┘
"#;

        self.display.show_title(title)?;
        self.display.show_message("A professional text-based adventure experience", "info")?;
        self.display.show_message(&format!("Version {}", crate::VERSION), "info")?;
        
        let separator = "═".repeat(self.config.ui.text_width);
        self.display.show_message(&separator, "separator")?;
        println!();
        
        Ok(())
    }

    async fn start_new_game_menu(&mut self) -> GameResult<()> {
        let stories = self.story_loader.list_available_stories().await?;
        
        if stories.is_empty() {
            self.display.show_warning("No stories found! Please add story files to the stories directory.")?;
            self.display.wait_for_enter()?;
            return Ok(());
        }

        self.display.show_message("📚 Available Stories:", "scene_title")?;
        println!();

        let story_choices: Vec<String> = stories
            .iter()
            .map(|story| format!("{} - {}", story.title, story.description))
            .collect();

        let selection = Select::new()
            .with_prompt("Choose your adventure")
            .items(&story_choices)
            .interact()
            .map_err(|e| GameError::story(format!("Story selection error: {}", e)))?;

        let selected_story = &stories[selection];
        
        // Get player name
        let player_name: String = Input::new()
            .with_prompt("Enter your character's name")
            .default("Adventurer".to_string())
            .interact_text()
            .map_err(|e| GameError::configuration(format!("Name input error: {}", e)))?;

        // Load story and start game
        let story = self.story_loader.load_story(&selected_story.id).await?;
        self.engine.load_story(story).await?;
        self.engine.start_new_game(player_name).await?;

        self.display.show_success(&format!("Starting \"{}\"...", selected_story.title))?;
        sleep(Duration::from_millis(self.config.get_animation_delay_ms())).await;

        // Start game loop
        self.game_loop().await?;
        
        Ok(())
    }

    async fn load_game_menu(&mut self) -> GameResult<()> {
        let saves = self.save_manager.list_save_games().await?;
        
        if saves.is_empty() {
            self.display.show_warning("No save games found. Starting a new game instead...")?;
            self.display.wait_for_enter()?;
            self.start_new_game_menu().await?;
            return Ok(());
        }

        self.display.show_message("💾 Saved Games:", "scene_title")?;
        println!();

        let save_choices: Vec<String> = saves
            .iter()
            .map(|save| {
                format!("{} - {} ({})", 
                    save.name, 
                    save.save_time.format("%Y-%m-%d %H:%M"), 
                    save.get_playtime_formatted()
                )
            })
            .collect();

        let mut all_choices = save_choices;
        all_choices.push("🔙 Back to Main Menu".to_string());

        let selection = Select::new()
            .with_prompt("Choose a save game")
            .items(&all_choices)
            .interact()
            .map_err(|e| GameError::save_load(format!("Save selection error: {}", e)))?;

        if selection == all_choices.len() - 1 {
            // Back to main menu
            return Ok(());
        }

        let selected_save = &saves[selection];
        
        // Load the save
        let save_game = self.save_manager.load_game(selected_save.id).await?;
        let story = self.story_loader.load_story(&save_game.game_state.story_id).await?;
        
        self.engine.load_story(story).await?;
        self.engine.load_game(save_game.game_state).await?;

        self.display.show_success(&format!("Loaded \"{}\"", selected_save.name))?;
        sleep(Duration::from_millis(self.config.get_animation_delay_ms())).await;

        // Start game loop
        self.game_loop().await?;
        
        Ok(())
    }

    async fn game_loop(&mut self) -> GameResult<()> {
        while self.engine.is_game_active() && !self.engine.is_game_ended().await {
            self.display.clear_screen().ok();
            
            // Show current scene
            let scene = self.engine.get_current_scene().await?;
            self.display.show_scene(&scene)?;
            
            // Show player stats if configured
            if self.config.ui.show_stats_in_header {
                if let Some(game_state) = self.engine.get_game_state() {
                    self.display.show_player_stats(game_state)?;
                }
            }

            // Prepare choices (including system choices)
            let mut available_choices = scene.choices
                .iter()
                .filter(|choice| !choice.disabled.unwrap_or(false))
                .map(|choice| choice.text.clone())
                .collect::<Vec<_>>();

            // Add system choices
            available_choices.extend_from_slice(&[
                "💾 Save Game".to_string(),
                "🎒 View Inventory".to_string(),
                "📊 View Statistics".to_string(),
                "⚙️ Settings".to_string(),
                "🚪 Quit Game".to_string(),
            ]);

            self.display.show_choices(&scene.choices)?;

            let selection = Select::new()
                .with_prompt("What do you choose?")
                .items(&available_choices)
                .interact()
                .map_err(|e| GameError::configuration(format!("Choice selection error: {}", e)))?;

            // Handle choice
            let valid_scene_choices = scene.choices
                .iter()
                .filter(|choice| !choice.disabled.unwrap_or(false))
                .collect::<Vec<_>>();

            if selection < valid_scene_choices.len() {
                // Scene choice
                let chosen_choice = valid_scene_choices[selection];
                self.engine.make_choice(&chosen_choice.id).await?;
                
                // Show animation delay
                if self.config.get_animation_delay_ms() > 0 {
                    sleep(Duration::from_millis(self.config.get_animation_delay_ms())).await;
                }
                
                self.display.show_separator()?;
            } else {
                // System choice
                let system_choice_index = selection - valid_scene_choices.len();
                match system_choice_index {
                    0 => self.save_current_game().await?,
                    1 => self.show_inventory().await?,
                    2 => self.show_game_statistics().await?,
                    3 => self.quick_settings().await?,
                    4 => {
                        if self.confirm_quit().await? {
                            break;
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }

        // Check if game ended
        if self.engine.is_game_ended().await {
            let scene = self.engine.get_current_scene().await?;
            self.display.clear_screen().ok();
            self.display.show_scene(&scene)?;
            self.display.show_success("🎊 Adventure Complete! 🎊")?;
            self.display.wait_for_enter()?;
        }

        Ok(())
    }

    async fn save_current_game(&mut self) -> GameResult<()> {
        let save_name: String = Input::new()
            .with_prompt("Enter a name for your save")
            .default(format!("Save {}", chrono::Utc::now().format("%Y-%m-%d %H:%M")))
            .interact_text()
            .map_err(|e| GameError::save_load(format!("Save name input error: {}", e)))?;

        match self.engine.save_game(save_name.clone()).await {
            Ok(game_state) => {
                self.save_manager.save_game(save_name.clone(), game_state, None).await?;
                self.display.show_success(&format!("Game saved as \"{}\"", save_name))?;
            }
            Err(e) => {
                self.display.show_error(&format!("Failed to save game: {}", e))?;
            }
        }
        
        self.display.wait_for_enter()?;
        Ok(())
    }

    async fn show_inventory(&mut self) -> GameResult<()> {
        self.display.clear_screen().ok();
        
        if let Some(game_state) = self.engine.get_game_state() {
            self.display.show_inventory(game_state)?;
        }
        
        self.display.wait_for_enter()?;
        Ok(())
    }

    async fn show_game_statistics(&mut self) -> GameResult<()> {
        self.display.clear_screen().ok();
        
        if let Some(game_state) = self.engine.get_game_state() {
            let stats = game_state.get_statistics();
            
            self.display.show_message("📊 Game Statistics", "scene_title")?;
            let separator = "═".repeat(50);
            self.display.show_message(&separator, "separator")?;
            
            self.display.show_message(&format!("Playtime: {}", stats.get_playtime_formatted()), "info")?;
            self.display.show_message(&format!("Scenes Visited: {} (unique: {})", stats.total_scenes_visited, stats.unique_scenes_visited), "info")?;
            self.display.show_message(&format!("Player Level: {}", stats.player_level), "info")?;
            self.display.show_message(&format!("Total Experience: {}", stats.total_experience), "info")?;
            self.display.show_message(&format!("Inventory Items: {}", stats.inventory_size), "info")?;
            self.display.show_message(&format!("Total Inventory Value: {}", stats.total_inventory_value), "info")?;
            self.display.show_message(&format!("Flags Set: {}", stats.flags_set), "info")?;
            self.display.show_message(&format!("Game Started: {}", stats.game_start_time.format("%Y-%m-%d %H:%M:%S UTC")), "info")?;
            
            if let Some(last_save) = stats.last_save_time {
                self.display.show_message(&format!("Last Saved: {}", last_save.format("%Y-%m-%d %H:%M:%S UTC")), "info")?;
            }
            
            self.display.show_message(&separator, "separator")?;
        }
        
        self.display.wait_for_enter()?;
        Ok(())
    }

    async fn quick_settings(&mut self) -> GameResult<()> {
        let choices = vec![
            "🎨 Change Theme",
            "⚙️ Toggle Stats Display",
            "🔙 Back"
        ];

        let selection = Select::new()
            .with_prompt("Quick Settings")
            .items(&choices)
            .interact()
            .map_err(|e| GameError::configuration(format!("Settings selection error: {}", e)))?;

        match selection {
            0 => self.change_theme().await?,
            1 => self.toggle_stats_display(),
            2 => {} // Back
            _ => unreachable!(),
        }

        Ok(())
    }

    async fn change_theme(&mut self) -> GameResult<()> {
        let themes = self.display.get_available_themes();
        
        let selection = Select::new()
            .with_prompt("Choose theme")
            .items(&themes)
            .interact()
            .map_err(|e| GameError::configuration(format!("Theme selection error: {}", e)))?;

        let selected_theme = &themes[selection];
        
        if self.display.set_theme(selected_theme) {
            self.display.show_success(&format!("Theme changed to '{}'", selected_theme))?;
        } else {
            self.display.show_error(&format!("Failed to set theme '{}'", selected_theme))?;
        }
        
        self.display.wait_for_enter()?;
        Ok(())
    }

    fn toggle_stats_display(&mut self) {
        self.config.ui.show_stats_in_header = !self.config.ui.show_stats_in_header;
        let status = if self.config.ui.show_stats_in_header { "enabled" } else { "disabled" };
        self.display.show_success(&format!("Stats display {}", status)).ok();
        self.display.wait_for_enter().ok();
    }

    async fn confirm_quit(&mut self) -> GameResult<bool> {
        let confirmed = Confirm::new()
            .with_prompt("Are you sure you want to quit? (Progress will be lost unless saved)")
            .default(false)
            .interact()
            .map_err(|e| GameError::configuration(format!("Quit confirmation error: {}", e)))?;

        Ok(confirmed)
    }

    async fn settings_menu(&mut self) -> GameResult<()> {
        loop {
            let choices = vec![
                "🎨 Theme Settings",
                "💾 Save Management",
                "📊 View All Statistics", 
                "🧹 Cleanup Old Saves",
                "🔙 Back to Main Menu"
            ];

            let selection = Select::new()
                .with_prompt("Settings")
                .items(&choices)
                .interact()
                .map_err(|e| GameError::configuration(format!("Settings selection error: {}", e)))?;

            match selection {
                0 => self.theme_settings().await?,
                1 => self.save_management().await?,
                2 => self.all_statistics().await?,
                3 => self.cleanup_saves().await?,
                4 => break,
                _ => unreachable!(),
            }
        }

        Ok(())
    }

    async fn theme_settings(&mut self) -> GameResult<()> {
        let themes = self.display.get_available_themes();
        
        self.display.show_message("🎨 Available Themes:", "scene_title")?;
        
        for theme in &themes {
            self.display.show_message(&format!("• {}", theme), "choice")?;
        }
        
        self.change_theme().await?;
        Ok(())
    }

    async fn save_management(&mut self) -> GameResult<()> {
        let saves = self.save_manager.list_save_games().await?;
        
        if saves.is_empty() {
            self.display.show_info("No save games found.")?;
            self.display.wait_for_enter()?;
            return Ok(());
        }

        self.display.show_message(&format!("💾 Save Management ({} saves)", saves.len()), "scene_title")?;
        
        for save in &saves {
            self.display.show_message(
                &format!("• {} - {} ({})", 
                    save.display_name(), 
                    save.save_time.format("%Y-%m-%d %H:%M"), 
                    save.get_playtime_formatted()
                ), 
                "info"
            )?;
        }

        let choices = vec![
            "🗑️ Delete a Save",
            "📤 Export Save",
            "📥 Import Save",
            "🔙 Back"
        ];

        let selection = Select::new()
            .with_prompt("Save Management Options")
            .items(&choices)
            .interact()
            .map_err(|e| GameError::configuration(format!("Save management selection error: {}", e)))?;

        match selection {
            0 => self.delete_save().await?,
            1 => self.export_save().await?,
            2 => self.import_save().await?,
            3 => {} // Back
            _ => unreachable!(),
        }

        Ok(())
    }

    async fn delete_save(&mut self) -> GameResult<()> {
        let saves = self.save_manager.list_save_games().await?;
        
        if saves.is_empty() {
            self.display.show_info("No save games to delete.")?;
            self.display.wait_for_enter()?;
            return Ok(());
        }

        let save_choices: Vec<String> = saves
            .iter()
            .map(|save| save.display_name())
            .collect();

        let selection = Select::new()
            .with_prompt("Choose save to delete")
            .items(&save_choices)
            .interact()
            .map_err(|e| GameError::save_load(format!("Delete save selection error: {}", e)))?;

        let selected_save = &saves[selection];
        
        let confirmed = Confirm::new()
            .with_prompt(&format!("Are you sure you want to delete '{}'?", selected_save.name))
            .default(false)
            .interact()
            .map_err(|e| GameError::configuration(format!("Delete confirmation error: {}", e)))?;

        if confirmed {
            self.save_manager.delete_save(selected_save.id).await?;
            self.display.show_success(&format!("Deleted save '{}'", selected_save.name))?;
        } else {
            self.display.show_info("Delete cancelled.")?;
        }
        
        self.display.wait_for_enter()?;
        Ok(())
    }

    async fn export_save(&mut self) -> GameResult<()> {
        // Implementation for save export
        self.display.show_info("Export functionality not yet implemented.")?;
        self.display.wait_for_enter()?;
        Ok(())
    }

    async fn import_save(&mut self) -> GameResult<()> {
        // Implementation for save import  
        self.display.show_info("Import functionality not yet implemented.")?;
        self.display.wait_for_enter()?;
        Ok(())
    }

    async fn all_statistics(&mut self) -> GameResult<()> {
        let save_count = self.save_manager.get_save_count().await?;
        let stories = self.story_loader.list_available_stories().await?;
        
        self.display.show_message("📊 Global Statistics", "scene_title")?;
        let separator = "═".repeat(50);
        self.display.show_message(&separator, "separator")?;
        
        self.display.show_message(&format!("Available Stories: {}", stories.len()), "info")?;
        self.display.show_message(&format!("Total Save Games: {}", save_count), "info")?;
        self.display.show_message(&format!("Game Version: {}", crate::VERSION), "info")?;
        
        self.display.show_message(&separator, "separator")?;
        self.display.wait_for_enter()?;
        
        Ok(())
    }

    async fn cleanup_saves(&mut self) -> GameResult<()> {
        let keep_count = self.config.saves.max_saves_per_story;
        
        let confirmed = Confirm::new()
            .with_prompt(&format!("This will keep only the {} most recent saves per story. Continue?", keep_count))
            .default(false)
            .interact()
            .map_err(|e| GameError::configuration(format!("Cleanup confirmation error: {}", e)))?;

        if confirmed {
            let deleted_count = self.save_manager.cleanup_old_saves(keep_count).await?;
            self.display.show_success(&format!("Cleaned up {} old save games", deleted_count))?;
        } else {
            self.display.show_info("Cleanup cancelled.")?;
        }
        
        self.display.wait_for_enter()?;
        Ok(())
    }

    async fn statistics_menu(&mut self) -> GameResult<()> {
        self.all_statistics().await
    }

    // Public API for CLI usage
    pub async fn load_story(&mut self, story_id: &str) -> GameResult<()> {
        let story = self.story_loader.load_story(story_id).await?;
        self.engine.load_story(story).await?;
        Ok(())
    }

    pub async fn start_new_game(&mut self) -> GameResult<()> {
        let player_name = "Player".to_string(); // Default for CLI usage
        self.engine.start_new_game(player_name).await?;
        self.game_loop().await?;
        Ok(())
    }
}

// Extension trait for display to add missing methods
trait DisplayExt {
    fn show_info(&self, message: &str) -> std::io::Result<()>;
}

impl DisplayExt for Display {
    fn show_info(&self, message: &str) -> std::io::Result<()> {
        self.show_message(&format!("ℹ️ {}", message), "info")
    }
}