use colored::{Color, Colorize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub colors: HashMap<String, ColorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub style: Vec<String>,
}

pub struct ThemeManager {
    themes: HashMap<String, Theme>,
    current_theme: String,
}

impl ThemeManager {
    pub fn new() -> Self {
        let mut manager = Self {
            themes: HashMap::new(),
            current_theme: "default".to_string(),
        };
        
        manager.load_default_themes();
        manager
    }

    pub fn set_theme(&mut self, theme_name: &str) -> bool {
        if self.themes.contains_key(theme_name) {
            self.current_theme = theme_name.to_string();
            true
        } else {
            false
        }
    }

    pub fn get_current_theme(&self) -> &Theme {
        self.themes.get(&self.current_theme)
            .unwrap_or_else(|| self.themes.get("default").unwrap())
    }

    pub fn apply_style(&self, text: &str, style_name: &str) -> String {
        let theme = self.get_current_theme();
        
        if let Some(color_config) = theme.colors.get(style_name) {
            let mut styled_text = text.to_string();
            
            // Apply foreground color
            if let Some(fg_color) = &color_config.foreground {
                if let Some(color) = parse_color(fg_color) {
                    styled_text = styled_text.color(color).to_string();
                }
            }

            // Apply styles (bold, italic, underline, etc.)
            for style in &color_config.style {
                styled_text = match style.as_str() {
                    "bold" => styled_text.bold().to_string(),
                    "italic" => styled_text.italic().to_string(),
                    "underline" => styled_text.underline().to_string(),
                    "dimmed" => styled_text.dimmed().to_string(),
                    "strikethrough" => styled_text.strikethrough().to_string(),
                    _ => styled_text,
                };
            }
            
            styled_text
        } else {
            text.to_string()
        }
    }

    pub fn list_themes(&self) -> Vec<String> {
        self.themes.keys().cloned().collect()
    }

    fn load_default_themes(&mut self) {
        // Default theme
        let mut default_colors = HashMap::new();
        default_colors.insert("title".to_string(), ColorConfig {
            foreground: Some("cyan".to_string()),
            background: None,
            style: vec!["bold".to_string()],
        });
        default_colors.insert("scene_title".to_string(), ColorConfig {
            foreground: Some("blue".to_string()),
            background: None,
            style: vec!["bold".to_string()],
        });
        default_colors.insert("scene_description".to_string(), ColorConfig {
            foreground: Some("white".to_string()),
            background: None,
            style: vec![],
        });
        default_colors.insert("choice".to_string(), ColorConfig {
            foreground: Some("green".to_string()),
            background: None,
            style: vec![],
        });
        default_colors.insert("choice_disabled".to_string(), ColorConfig {
            foreground: Some("bright_black".to_string()),
            background: None,
            style: vec!["dimmed".to_string()],
        });
        default_colors.insert("stats".to_string(), ColorConfig {
            foreground: Some("yellow".to_string()),
            background: None,
            style: vec![],
        });
        default_colors.insert("health_high".to_string(), ColorConfig {
            foreground: Some("green".to_string()),
            background: None,
            style: vec!["bold".to_string()],
        });
        default_colors.insert("health_medium".to_string(), ColorConfig {
            foreground: Some("yellow".to_string()),
            background: None,
            style: vec!["bold".to_string()],
        });
        default_colors.insert("health_low".to_string(), ColorConfig {
            foreground: Some("red".to_string()),
            background: None,
            style: vec!["bold".to_string()],
        });
        default_colors.insert("error".to_string(), ColorConfig {
            foreground: Some("red".to_string()),
            background: None,
            style: vec!["bold".to_string()],
        });
        default_colors.insert("success".to_string(), ColorConfig {
            foreground: Some("green".to_string()),
            background: None,
            style: vec!["bold".to_string()],
        });
        default_colors.insert("warning".to_string(), ColorConfig {
            foreground: Some("yellow".to_string()),
            background: None,
            style: vec!["bold".to_string()],
        });
        default_colors.insert("info".to_string(), ColorConfig {
            foreground: Some("blue".to_string()),
            background: None,
            style: vec![],
        });
        default_colors.insert("separator".to_string(), ColorConfig {
            foreground: Some("bright_black".to_string()),
            background: None,
            style: vec!["dimmed".to_string()],
        });

        self.themes.insert("default".to_string(), Theme {
            name: "default".to_string(),
            colors: default_colors,
        });

        // Dark theme
        let mut dark_colors = HashMap::new();
        dark_colors.insert("title".to_string(), ColorConfig {
            foreground: Some("bright_cyan".to_string()),
            background: None,
            style: vec!["bold".to_string()],
        });
        dark_colors.insert("scene_title".to_string(), ColorConfig {
            foreground: Some("bright_blue".to_string()),
            background: None,
            style: vec!["bold".to_string()],
        });
        dark_colors.insert("scene_description".to_string(), ColorConfig {
            foreground: Some("bright_white".to_string()),
            background: None,
            style: vec![],
        });
        dark_colors.insert("choice".to_string(), ColorConfig {
            foreground: Some("bright_green".to_string()),
            background: None,
            style: vec![],
        });
        dark_colors.insert("choice_disabled".to_string(), ColorConfig {
            foreground: Some("black".to_string()),
            background: None,
            style: vec!["dimmed".to_string()],
        });
        dark_colors.insert("stats".to_string(), ColorConfig {
            foreground: Some("bright_yellow".to_string()),
            background: None,
            style: vec![],
        });
        dark_colors.insert("health_high".to_string(), ColorConfig {
            foreground: Some("bright_green".to_string()),
            background: None,
            style: vec!["bold".to_string()],
        });
        dark_colors.insert("health_medium".to_string(), ColorConfig {
            foreground: Some("bright_yellow".to_string()),
            background: None,
            style: vec!["bold".to_string()],
        });
        dark_colors.insert("health_low".to_string(), ColorConfig {
            foreground: Some("bright_red".to_string()),
            background: None,
            style: vec!["bold".to_string()],
        });

        self.themes.insert("dark".to_string(), Theme {
            name: "dark".to_string(),
            colors: dark_colors,
        });

        // Light theme
        let mut light_colors = HashMap::new();
        light_colors.insert("title".to_string(), ColorConfig {
            foreground: Some("blue".to_string()),
            background: None,
            style: vec!["bold".to_string()],
        });
        light_colors.insert("scene_title".to_string(), ColorConfig {
            foreground: Some("magenta".to_string()),
            background: None,
            style: vec!["bold".to_string()],
        });
        light_colors.insert("scene_description".to_string(), ColorConfig {
            foreground: Some("black".to_string()),
            background: None,
            style: vec![],
        });
        light_colors.insert("choice".to_string(), ColorConfig {
            foreground: Some("blue".to_string()),
            background: None,
            style: vec![],
        });

        self.themes.insert("light".to_string(), Theme {
            name: "light".to_string(),
            colors: light_colors,
        });
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_color(color_name: &str) -> Option<Color> {
    match color_name.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "bright_black" => Some(Color::BrightBlack),
        "bright_red" => Some(Color::BrightRed),
        "bright_green" => Some(Color::BrightGreen),
        "bright_yellow" => Some(Color::BrightYellow),
        "bright_blue" => Some(Color::BrightBlue),
        "bright_magenta" => Some(Color::BrightMagenta),
        "bright_cyan" => Some(Color::BrightCyan),
        "bright_white" => Some(Color::BrightWhite),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_manager_creation() {
        let manager = ThemeManager::new();
        assert_eq!(manager.current_theme, "default");
        assert!(manager.themes.contains_key("default"));
        assert!(manager.themes.contains_key("dark"));
        assert!(manager.themes.contains_key("light"));
    }

    #[test]
    fn test_set_theme() {
        let mut manager = ThemeManager::new();
        
        assert!(manager.set_theme("dark"));
        assert_eq!(manager.current_theme, "dark");
        
        assert!(!manager.set_theme("nonexistent"));
        assert_eq!(manager.current_theme, "dark"); // Should remain unchanged
    }

    #[test]
    fn test_apply_style() {
        let manager = ThemeManager::new();
        
        let styled = manager.apply_style("Test Title", "title");
        assert!(!styled.is_empty());
        
        // Test with nonexistent style
        let unstyled = manager.apply_style("Test", "nonexistent");
        assert_eq!(unstyled, "Test");
    }

    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("red"), Some(Color::Red));
        assert_eq!(parse_color("RED"), Some(Color::Red));
        assert_eq!(parse_color("bright_green"), Some(Color::BrightGreen));
        assert_eq!(parse_color("invalid"), None);
    }
}