use console::{Term, Key};
use std::io::{self, Write};
use crate::ui::ThemeManager;
use crate::core::{GameState, PlayerStats};
use crate::story::{Scene, Choice};

pub struct Display {
    term: Term,
    theme_manager: ThemeManager,
    text_width: usize,
}

impl Display {
    pub fn new(theme_manager: ThemeManager, text_width: usize) -> io::Result<Self> {
        Ok(Self {
            term: Term::stdout(),
            theme_manager,
            text_width,
        })
    }

    pub fn clear_screen(&self) -> io::Result<()> {
        self.term.clear_screen()
    }

    pub fn show_title(&self, title: &str) -> io::Result<()> {
        let styled_title = self.theme_manager.apply_style(title, "title");
        
        // Create a border
        let border = "═".repeat(self.text_width);
        let styled_border = self.theme_manager.apply_style(&border, "separator");
        
        writeln!(io::stdout(), "{}", styled_title)?;
        writeln!(io::stdout(), "{}", styled_border)?;
        writeln!(io::stdout())?;
        
        Ok(())
    }

    pub fn show_scene(&self, scene: &Scene) -> io::Result<()> {
        // Scene title
        let styled_title = self.theme_manager.apply_style(&scene.title, "scene_title");
        writeln!(io::stdout(), "📍 {}", styled_title)?;
        
        let separator = "─".repeat(40);
        let styled_separator = self.theme_manager.apply_style(&separator, "separator");
        writeln!(io::stdout(), "{}", styled_separator)?;
        
        // Scene description with word wrapping
        self.show_wrapped_text(&scene.description, "scene_description")?;
        writeln!(io::stdout())?;
        
        Ok(())
    }

    pub fn show_player_stats(&self, game_state: &GameState) -> io::Result<()> {
        let stats = &game_state.player.stats;
        
        // Health bar
        let health_bar = self.create_health_bar(stats.health, stats.max_health);
        let health_style = self.get_health_style(stats.health, stats.max_health);
        let styled_health = self.theme_manager.apply_style(&health_bar, &health_style);
        
        let stats_text = format!(
            "📊 Player Stats: {} Health: {} {}/{} | Level: {} | XP: {} | STR: {} | INT: {} | CHA: {}",
            game_state.player.name,
            styled_health,
            stats.health,
            stats.max_health,
            stats.level,
            stats.experience,
            stats.strength,
            stats.intelligence,
            stats.charisma
        );
        
        let styled_stats = self.theme_manager.apply_style(&stats_text, "stats");
        writeln!(io::stdout(), "{}", styled_stats)?;
        writeln!(io::stdout())?;
        
        Ok(())
    }

    pub fn show_choices(&self, choices: &[Choice]) -> io::Result<()> {
        writeln!(io::stdout(), "Choose your action:")?;
        
        for (index, choice) in choices.iter().enumerate() {
            let choice_text = format!("{}. {}", index + 1, choice.text);
            
            if choice.disabled.unwrap_or(false) {
                let reason = choice.disabled_reason.as_deref().unwrap_or("Requirements not met");
                let disabled_text = format!("{} ({})", choice_text, reason);
                let styled = self.theme_manager.apply_style(&disabled_text, "choice_disabled");
                writeln!(io::stdout(), "   {}", styled)?;
            } else {
                let styled = self.theme_manager.apply_style(&choice_text, "choice");
                writeln!(io::stdout(), "   {}", styled)?;
            }
        }
        
        writeln!(io::stdout())?;
        Ok(())
    }

    pub fn show_inventory(&self, game_state: &GameState) -> io::Result<()> {
        let styled_title = self.theme_manager.apply_style("🎒 Inventory", "scene_title");
        writeln!(io::stdout(), "{}", styled_title)?;
        
        let separator = "═".repeat(50);
        let styled_separator = self.theme_manager.apply_style(&separator, "separator");
        writeln!(io::stdout(), "{}", styled_separator)?;
        
        if game_state.player.inventory.is_empty() {
            let empty_msg = self.theme_manager.apply_style("   Your inventory is empty.", "info");
            writeln!(io::stdout(), "{}", empty_msg)?;
        } else {
            for item in &game_state.player.inventory {
                let quantity_text = if item.quantity > 1 {
                    format!(" ({})", item.quantity)
                } else {
                    String::new()
                };
                
                let item_text = format!("   {} {}{}", 
                    self.get_item_icon(&item.item_type), 
                    item.name, 
                    quantity_text
                );
                let styled_item = self.theme_manager.apply_style(&item_text, "choice");
                writeln!(io::stdout(), "{}", styled_item)?;
                
                let description = format!("      {}", item.description);
                let styled_desc = self.theme_manager.apply_style(&description, "info");
                writeln!(io::stdout(), "{}", styled_desc)?;
            }
        }
        
        writeln!(io::stdout(), "{}", styled_separator)?;
        Ok(())
    }

    pub fn show_message(&self, message: &str, style: &str) -> io::Result<()> {
        let styled_message = self.theme_manager.apply_style(message, style);
        writeln!(io::stdout(), "{}", styled_message)?;
        Ok(())
    }

    pub fn show_error(&self, error: &str) -> io::Result<()> {
        self.show_message(&format!("❌ {}", error), "error")
    }

    pub fn show_success(&self, message: &str) -> io::Result<()> {
        self.show_message(&format!("✅ {}", message), "success")
    }

    pub fn show_warning(&self, message: &str) -> io::Result<()> {
        self.show_message(&format!("⚠️ {}", message), "warning")
    }

    pub fn show_separator(&self) -> io::Result<()> {
        let separator = "━".repeat(self.text_width);
        let styled = self.theme_manager.apply_style(&separator, "separator");
        writeln!(io::stdout(), "{}", styled)?;
        Ok(())
    }

    pub fn prompt_input(&self, prompt: &str) -> io::Result<String> {
        let styled_prompt = self.theme_manager.apply_style(prompt, "info");
        print!("{}", styled_prompt);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    pub fn prompt_yes_no(&self, prompt: &str, default: bool) -> io::Result<bool> {
        let default_text = if default { " [Y/n]" } else { " [y/N]" };
        let full_prompt = format!("{}{} ", prompt, default_text);
        
        loop {
            let input = self.prompt_input(&full_prompt)?;
            
            match input.to_lowercase().as_str() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                "" => return Ok(default),
                _ => {
                    self.show_error("Please enter 'y' for yes or 'n' for no.")?;
                    continue;
                }
            }
        }
    }

    pub fn prompt_number(&self, prompt: &str, min: usize, max: usize) -> io::Result<usize> {
        loop {
            let input = self.prompt_input(prompt)?;
            
            match input.parse::<usize>() {
                Ok(num) if num >= min && num <= max => return Ok(num),
                Ok(_) => {
                    self.show_error(&format!("Please enter a number between {} and {}.", min, max))?;
                }
                Err(_) => {
                    self.show_error("Please enter a valid number.")?;
                }
            }
        }
    }

    pub fn wait_for_key(&self) -> io::Result<Key> {
        self.term.read_key()
    }

    pub fn wait_for_enter(&self) -> io::Result<()> {
        let styled_prompt = self.theme_manager.apply_style("Press Enter to continue...", "info");
        print!("{}", styled_prompt);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(())
    }

    fn show_wrapped_text(&self, text: &str, style: &str) -> io::Result<()> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut current_line = String::new();
        
        for word in words {
            if current_line.len() + word.len() + 1 > self.text_width {
                if !current_line.is_empty() {
                    let styled_line = self.theme_manager.apply_style(&current_line, style);
                    writeln!(io::stdout(), "{}", styled_line)?;
                    current_line.clear();
                }
            }
            
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
        
        if !current_line.is_empty() {
            let styled_line = self.theme_manager.apply_style(&current_line, style);
            writeln!(io::stdout(), "{}", styled_line)?;
        }
        
        Ok(())
    }

    fn create_health_bar(&self, current: i32, max: i32) -> String {
        let percentage = current as f32 / max as f32;
        let bar_length = 10;
        let filled_length = (percentage * bar_length as f32) as usize;
        let empty_length = bar_length - filled_length;
        
        format!("{}{}", "█".repeat(filled_length), "░".repeat(empty_length))
    }

    fn get_health_style(&self, current: i32, max: i32) -> String {
        let percentage = current as f32 / max as f32;
        
        if percentage > 0.6 {
            "health_high".to_string()
        } else if percentage > 0.3 {
            "health_medium".to_string()
        } else {
            "health_low".to_string()
        }
    }

    fn get_item_icon(&self, item_type: &crate::core::ItemType) -> &str {
        match item_type {
            crate::core::ItemType::Weapon => "⚔️",
            crate::core::ItemType::Armor => "🛡️",
            crate::core::ItemType::Consumable => "🧪",
            crate::core::ItemType::KeyItem => "🔑",
            crate::core::ItemType::Treasure => "💎",
        }
    }

    pub fn set_theme(&mut self, theme_name: &str) -> bool {
        self.theme_manager.set_theme(theme_name)
    }

    pub fn get_available_themes(&self) -> Vec<String> {
        self.theme_manager.list_themes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Player, InventoryItem, ItemType};
    use std::collections::HashMap;

    #[test]
    fn test_display_creation() {
        let theme_manager = ThemeManager::new();
        let display = Display::new(theme_manager, 80);
        assert!(display.is_ok());
    }

    #[test]
    fn test_health_bar_creation() {
        let theme_manager = ThemeManager::new();
        let display = Display::new(theme_manager, 80).unwrap();
        
        let health_bar = display.create_health_bar(50, 100);
        assert_eq!(health_bar.len(), 10);
        
        let health_bar_full = display.create_health_bar(100, 100);
        assert_eq!(health_bar_full, "██████████");
        
        let health_bar_empty = display.create_health_bar(0, 100);
        assert_eq!(health_bar_empty, "░░░░░░░░░░");
    }

    #[test]
    fn test_health_style() {
        let theme_manager = ThemeManager::new();
        let display = Display::new(theme_manager, 80).unwrap();
        
        assert_eq!(display.get_health_style(80, 100), "health_high");
        assert_eq!(display.get_health_style(50, 100), "health_medium");
        assert_eq!(display.get_health_style(20, 100), "health_low");
    }

    #[test]
    fn test_item_icons() {
        let theme_manager = ThemeManager::new();
        let display = Display::new(theme_manager, 80).unwrap();
        
        assert_eq!(display.get_item_icon(&ItemType::Weapon), "⚔️");
        assert_eq!(display.get_item_icon(&ItemType::Armor), "🛡️");
        assert_eq!(display.get_item_icon(&ItemType::Consumable), "🧪");
        assert_eq!(display.get_item_icon(&ItemType::KeyItem), "🔑");
        assert_eq!(display.get_item_icon(&ItemType::Treasure), "💎");
    }
}