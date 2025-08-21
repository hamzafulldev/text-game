pub mod core;
pub mod story;
pub mod ui;
pub mod config;
pub mod utils;

pub use core::{engine::GameEngine, player::Player, game_state::GameState};
pub use story::{Story, Scene, Choice};
pub use ui::GameInterface;
pub use config::Config;

// Re-export commonly used types
pub type Result<T> = anyhow::Result<T>;

// Game version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");