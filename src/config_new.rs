//! Configuration management for Weft Terminal
//! 
//! This module handles loading, saving, and managing configuration
//! settings for the terminal application.

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info, warn};
use toml;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub terminal: TerminalConfig,
    pub ai: AIConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalConfig {
    pub shell: String,
    pub font_family: String,
    pub font_size: f32,
    pub line_height: f32,
    pub cursor_blink: bool,
    pub cursor_style: CursorStyle,
    pub scrollback_size: usize,
    pub bell_style: BellStyle,
    pub copy_on_select: bool,
    pub right_click_action: RightClickAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CursorStyle {
    Block,
    Underline,
    Beam,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BellStyle {
    None,
    Visual,
    Audible,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RightClickAction {
    Paste,
    ContextMenu,
    ExtendSelection,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            terminal: TerminalConfig::default(),
            ai: AIConfig::default(),
        }
    }
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()),
            font_family: "JetBrains Mono".to_string(),
            font_size: 14.0,
            line_height: 1.2,
            cursor_blink: true,
            cursor_style: CursorStyle::Block,
            scrollback_size: 10000,
            bell_style: BellStyle::Visual,
            copy_on_select: false,
            right_click_action: RightClickAction::ContextMenu,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AIConfig {
    pub enabled: bool,
    pub provider: AIProvider,
    pub model: String,
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub context_window: usize,
    pub prediction_threshold: f32,
    pub auto_suggestions: bool,
    pub learning_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AIProvider {
    Ollama,
    OpenAI,
    Anthropic,
    Custom,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: AIProvider::Ollama,
            model: "llama2".to_string(),
            api_key: None,
            endpoint: Some("http://localhost:11434".to_string()),
            context_window: 4096,
            prediction_threshold: 0.7,
            auto_suggestions: true,
            learning_enabled: true,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_file_path();
        
        if config_path.exists() {
            info!("Loading configuration from: {}", config_path.display());
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            warn!("Configuration file not found, creating default configuration");
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_file_path();
        
        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        info!("Saving configuration to: {}", config_path.display());
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        
        Ok(())
    }

    pub fn config_file_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("weft")
            .join("config.toml")
    }

    pub fn data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("weft")
    }

    pub fn cache_dir() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("weft")
    }
}
