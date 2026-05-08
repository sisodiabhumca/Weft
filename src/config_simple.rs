//! Simple configuration management for Weft Terminal

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use toml;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub terminal: TerminalConfig,
    pub ai: AIConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    pub shell: String,
    pub font_family: String,
    pub font_size: f32,
    pub cursor_blink: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AIConfig {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
    pub auto_suggestions: bool,
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
            cursor_blink: true,
        }
    }
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: "Ollama".to_string(),
            model: "llama2".to_string(),
            auto_suggestions: true,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_file_path();
        Self::load_from_path(&config_path)
    }

    pub fn load_from_path(config_path: &PathBuf) -> Result<Self> {
        if config_path.exists() {
            info!("Loading configuration from: {}", config_path.display());
            let content = std::fs::read_to_string(config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            warn!("Configuration file not found, creating default configuration");
            let config = Config::default();
            config.save_to_path(config_path)?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_file_path();
        self.save_to_path(&config_path)
    }

    pub fn save_to_path(&self, config_path: &PathBuf) -> Result<()> {
        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        info!("Saving configuration to: {}", config_path.display());
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;

        Ok(())
    }

    pub fn reset_to_default() -> Result<Self> {
        let config = Self::default();
        config.save()?;
        Ok(config)
    }

    pub fn get_value(&self, key: &str) -> Option<String> {
        match key {
            "terminal.shell" => Some(self.terminal.shell.clone()),
            "terminal.font_family" => Some(self.terminal.font_family.clone()),
            "terminal.font_size" => Some(self.terminal.font_size.to_string()),
            "terminal.cursor_blink" => Some(self.terminal.cursor_blink.to_string()),
            "ai.enabled" => Some(self.ai.enabled.to_string()),
            "ai.provider" => Some(self.ai.provider.clone()),
            "ai.model" => Some(self.ai.model.clone()),
            "ai.auto_suggestions" => Some(self.ai.auto_suggestions.to_string()),
            _ => None,
        }
    }

    pub fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "terminal.shell" => self.terminal.shell = value.to_string(),
            "terminal.font_family" => self.terminal.font_family = value.to_string(),
            "terminal.font_size" => {
                self.terminal.font_size = value
                    .parse::<f32>()
                    .map_err(|e| anyhow::anyhow!("Invalid terminal.font_size '{}': {}", value, e))?
            }
            "terminal.cursor_blink" => {
                self.terminal.cursor_blink = parse_bool(value, "terminal.cursor_blink")?
            }
            "ai.enabled" => self.ai.enabled = parse_bool(value, "ai.enabled")?,
            "ai.provider" => self.ai.provider = value.to_string(),
            "ai.model" => self.ai.model = value.to_string(),
            "ai.auto_suggestions" => {
                self.ai.auto_suggestions = parse_bool(value, "ai.auto_suggestions")?
            }
            _ => return Err(anyhow::anyhow!("Unknown config key '{}'", key)),
        }

        self.validate()?;
        self.save()?;
        Ok(())
    }

    pub fn config_file_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("weft")
            .join("config.toml")
    }

    pub fn validate(&self) -> Result<()> {
        if self.terminal.font_size <= 0.0 {
            return Err(anyhow::anyhow!("terminal.font_size must be > 0"));
        }

        if self.terminal.shell.trim().is_empty() {
            return Err(anyhow::anyhow!("terminal.shell cannot be empty"));
        }

        if self.ai.model.trim().is_empty() {
            return Err(anyhow::anyhow!("ai.model cannot be empty"));
        }

        Ok(())
    }
}

fn parse_bool(value: &str, key: &str) -> Result<bool> {
    value
        .parse::<bool>()
        .map_err(|e| anyhow::anyhow!("Invalid {} '{}': {}", key, value, e))
}
