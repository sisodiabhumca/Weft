//! Configuration management for Weft Terminal
//! 
//! This module handles loading, saving, and managing configuration
//! settings for the terminal application.

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub terminal: TerminalConfig,
    pub ai: AIConfig,
    pub rendering: RenderingConfig,
    pub plugins: PluginConfig,
    pub collaboration: CollaborationConfig,
    pub debugging: DebuggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CursorStyle {
    Block,
    Underline,
    Beam,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BellStyle {
    None,
    Visual,
    Audible,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RightClickAction {
    Paste,
    ContextMenu,
    ExtendSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub custom_prompts: Vec<CustomPrompt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIProvider {
    Ollama,
    OpenAI,
    Anthropic,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPrompt {
    pub name: String,
    pub template: String,
    pub triggers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingConfig {
    pub renderer: RendererType,
    pub vsync: bool,
    pub max_fps: u32,
    pub theme: String,
    pub transparency: f32,
    pub blur_background: bool,
    pub gpu_acceleration: bool,
    pub custom_shaders: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RendererType {
    Wgpu,
    OpenGL,
    Software,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub enabled: bool,
    pub auto_load: bool,
    pub plugin_directories: Vec<PathBuf>,
    pub security_policy: SecurityPolicy,
    pub trusted_plugins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityPolicy {
    AllowAll,
    AllowTrusted,
    Prompt,
    DenyAll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationConfig {
    pub enabled: bool,
    pub server_url: Option<String>,
    pub auto_sync: bool,
    pub session_sharing: bool,
    pub real_time_collaboration: bool,
    pub encryption_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebuggingConfig {
    pub enabled: bool,
    pub log_level: LogLevel,
    pub performance_monitoring: bool,
    pub memory_profiling: bool,
    pub network_inspection: bool,
    pub command_tracing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            terminal: TerminalConfig::default(),
            ai: AIConfig::default(),
            rendering: RenderingConfig::default(),
            plugins: PluginConfig::default(),
            collaboration: CollaborationConfig::default(),
            debugging: DebuggingConfig::default(),
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
            custom_prompts: Vec::new(),
        }
    }
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            renderer: RendererType::Wgpu,
            vsync: true,
            max_fps: 60,
            theme: "dark".to_string(),
            transparency: 1.0,
            blur_background: false,
            gpu_acceleration: true,
            custom_shaders: false,
        }
    }
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_load: true,
            plugin_directories: vec![
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/"))
                    .join(".weft")
                    .join("plugins"),
            ],
            security_policy: SecurityPolicy::Prompt,
            trusted_plugins: Vec::new(),
        }
    }
}

impl Default for CollaborationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_url: None,
            auto_sync: false,
            session_sharing: false,
            real_time_collaboration: false,
            encryption_enabled: true,
        }
    }
}

impl Default for DebuggingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            log_level: LogLevel::Info,
            performance_monitoring: false,
            memory_profiling: false,
            network_inspection: false,
            command_tracing: false,
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

    pub fn update_terminal_config<F>(&mut self, updater: F) -> bool
    where
        F: FnOnce(&mut TerminalConfig),
    {
        let old_config = self.terminal.clone();
        updater(&mut self.terminal);
        let changed = old_config != self.terminal;
        
        if changed {
            if let Err(e) = self.save() {
                warn!("Failed to save configuration: {}", e);
            }
        }
        
        changed
    }

    pub fn update_ai_config<F>(&mut self, updater: F) -> bool
    where
        F: FnOnce(&mut AIConfig),
    {
        let old_config = self.ai.clone();
        updater(&mut self.ai);
        let changed = old_config != self.ai;
        
        if changed {
            if let Err(e) = self.save() {
                warn!("Failed to save configuration: {}", e);
            }
        }
        
        changed
    }

    pub fn update_rendering_config<F>(&mut self, updater: F) -> bool
    where
        F: FnOnce(&mut RenderingConfig),
    {
        let old_config = self.rendering.clone();
        updater(&mut self.rendering);
        let changed = old_config != self.rendering;
        
        if changed {
            if let Err(e) = self.save() {
                warn!("Failed to save configuration: {}", e);
            }
        }
        
        changed
    }

    pub fn update_plugin_config<F>(&mut self, updater: F) -> bool
    where
        F: FnOnce(&mut PluginConfig),
    {
        let old_config = self.plugins.clone();
        updater(&mut self.plugins);
        let changed = old_config != self.plugins;
        
        if changed {
            if let Err(e) = self.save() {
                warn!("Failed to save configuration: {}", e);
            }
        }
        
        changed
    }

    pub fn update_collaboration_config<F>(&mut self, updater: F) -> bool
    where
        F: FnOnce(&mut CollaborationConfig),
    {
        let old_config = self.collaboration.clone();
        updater(&mut self.collaboration);
        let changed = old_config != self.collaboration;
        
        if changed {
            if let Err(e) = self.save() {
                warn!("Failed to save configuration: {}", e);
            }
        }
        
        changed
    }

    pub fn update_debugging_config<F>(&mut self, updater: F) -> bool
    where
        F: FnOnce(&mut DebuggingConfig),
    {
        let old_config = self.debugging.clone();
        updater(&mut self.debugging);
        let changed = old_config != self.debugging;
        
        if changed {
            if let Err(e) = self.save() {
                warn!("Failed to save configuration: {}", e);
            }
        }
        
        changed
    }

    pub fn validate(&self) -> Result<()> {
        // Validate terminal config
        if self.terminal.font_size <= 0.0 {
            return Err(anyhow::anyhow!("Font size must be positive"));
        }
        
        if self.terminal.line_height <= 0.0 {
            return Err(anyhow::anyhow!("Line height must be positive"));
        }
        
        // Validate AI config
        if self.ai.enabled && self.ai.context_window == 0 {
            return Err(anyhow::anyhow!("Context window must be positive when AI is enabled"));
        }
        
        if self.ai.prediction_threshold < 0.0 || self.ai.prediction_threshold > 1.0 {
            return Err(anyhow::anyhow!("Prediction threshold must be between 0.0 and 1.0"));
        }
        
        // Validate rendering config
        if self.rendering.max_fps == 0 {
            return Err(anyhow::anyhow!("Max FPS must be positive"));
        }
        
        if self.rendering.transparency < 0.0 || self.rendering.transparency > 1.0 {
            return Err(anyhow::anyhow!("Transparency must be between 0.0 and 1.0"));
        }
        
        Ok(())
    }

    pub fn get_theme_path(&self) -> PathBuf {
        Self::data_dir().join("themes").join(format!("{}.toml", self.rendering.theme))
    }

    pub fn get_plugin_path(&self, plugin_name: &str) -> Option<PathBuf> {
        self.plugins.plugin_directories
            .iter()
            .find_map(|dir| {
                let path = dir.join(format!("{}.weft", plugin_name));
                if path.exists() {
                    Some(path)
                } else {
                    None
                }
            })
    }

    pub fn add_custom_prompt(&mut self, prompt: CustomPrompt) {
        self.ai.custom_prompts.push(prompt);
        let _ = self.save();
    }

    pub fn remove_custom_prompt(&mut self, name: &str) -> bool {
        let original_len = self.ai.custom_prompts.len();
        self.ai.custom_prompts.retain(|p| p.name != name);
        let removed = self.ai.custom_prompts.len() < original_len;
        
        if removed {
            let _ = self.save();
        }
        
        removed
    }

    pub fn add_trusted_plugin(&mut self, plugin_name: String) {
        if !self.plugins.trusted_plugins.contains(&plugin_name) {
            self.plugins.trusted_plugins.push(plugin_name);
            let _ = self.save();
        }
    }

    pub fn remove_trusted_plugin(&mut self, plugin_name: &str) -> bool {
        let original_len = self.plugins.trusted_plugins.len();
        self.plugins.trusted_plugins.retain(|p| p != plugin_name);
        let removed = self.plugins.trusted_plugins.len() < original_len;
        
        if removed {
            let _ = self.save();
        }
        
        removed
    }
}

pub struct ConfigManager {
    config: Arc<RwLock<Config>>,
}

impl ConfigManager {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
        })
    }

    pub fn get_config(&self) -> Arc<RwLock<Config>> {
        self.config.clone()
    }

    pub fn reload(&self) -> Result<()> {
        let new_config = Config::load()?;
        *self.config.write() = new_config;
        info!("Configuration reloaded");
        Ok(())
    }

    pub fn save_current(&self) -> Result<()> {
        self.config.read().save()
    }
}
