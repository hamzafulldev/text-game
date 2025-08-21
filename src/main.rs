use anyhow::Result;
use clap::Parser;
use text_adventure_game::{GameInterface, Config, VERSION};
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "text-game")]
#[command(about = "A professional text-based adventure game")]
#[command(version = VERSION)]
struct Cli {
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
    
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,
    
    /// Story to load directly
    #[arg(short, long)]
    story: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let log_level = if cli.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("text_adventure_game={},warn", log_level))
        .init();
    
    info!("Starting Text Adventure Game v{}", VERSION);
    
    // Load configuration
    let config = match cli.config {
        Some(config_path) => Config::from_file(&config_path)?,
        None => Config::default(),
    };
    
    // Create and start the game interface
    let mut game_interface = GameInterface::new(config).await?;
    
    match cli.story {
        Some(story_id) => {
            info!("Loading story: {}", story_id);
            game_interface.load_story(&story_id).await?;
            game_interface.start_new_game().await?;
        }
        None => {
            game_interface.show_main_menu().await?;
        }
    }
    
    // Start the game loop
    if let Err(e) = game_interface.run().await {
        error!("Game error: {}", e);
        eprintln!("An error occurred: {}", e);
        std::process::exit(1);
    }
    
    info!("Game session ended");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::try_parse_from(&["text-game", "--debug"]).unwrap();
        assert!(cli.debug);
    }
}