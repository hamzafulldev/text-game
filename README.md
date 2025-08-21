# Text Adventure Game

A professional, production-ready text-based adventure game engine built in Rust, featuring branching narratives, character progression, and comprehensive save/load functionality.

[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-red.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## 🎮 Features

### Core Gameplay
- **Interactive Storytelling**: Rich, branching narratives with meaningful choices
- **Character System**: Detailed player statistics, leveling, and inventory management
- **Dynamic Content**: Conditional scenes and choices based on player progress
- **Multiple Endings**: Different story outcomes based on player decisions

### Technical Features
- **Save/Load System**: Comprehensive game state persistence
- **Configuration**: Customizable settings via TOML configuration files
- **Theming**: Multiple UI themes for different visual preferences
- **Logging**: Structured logging for debugging and monitoring
- **Error Handling**: Robust error handling with user-friendly messages

### User Interface
- **Colorized Output**: Beautiful terminal interface with syntax highlighting
- **Progress Tracking**: Visual health bars, statistics display, and inventory management
- **Interactive Menus**: Intuitive navigation with fuzzy search capabilities
- **Accessibility**: Clear text formatting and consistent UI patterns

## 🚀 Quick Start

### Prerequisites

- **Rust**: Version 1.70 or higher
- **Cargo**: Included with Rust installation

### Installation

1. **Clone the repository:**
   ```bash
   git clone https://github.com/hamzafulldev/Text-Game.git
   cd Text-Game
   ```

2. **Build the project:**
   ```bash
   cargo build --release
   ```

3. **Run the game:**
   ```bash
   cargo run --release
   ```

## 🎯 Usage

### Basic Commands

```bash
# Start the game with default settings
cargo run

# Enable debug logging
cargo run -- --debug

# Specify a custom configuration file
cargo run -- --config ./my-config.toml

# Load a specific story directly
cargo run -- --story mystic-forest

# Show help
cargo run -- --help
```

## 📖 Creating Stories

Stories are defined in JSON format. Check `assets/stories/` for examples.

## 👨‍💻 Author

**Hamza Younas**
- Email: hamzafulldev@gmail.com
- GitHub: [@hamzafulldev](https://github.com/hamzafulldev)
- Project: [Text-Game](https://github.com/hamzafulldev/Text-Game)

---

*Built with ❤️ and Rust*
