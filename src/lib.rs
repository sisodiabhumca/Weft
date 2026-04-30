//! Weft Terminal - Next-generation AI-powered development environment
//! 
//! This library provides the core functionality for Weft, a modern terminal
//! that combines powerful AI capabilities with collaborative features and
//! extensible plugin architecture.

pub mod terminal;
pub mod ai;
pub mod rendering;
pub mod config;
pub mod plugins;
pub mod collaboration;
pub mod debugging;
pub mod performance;

use anyhow::Result;
use std::sync::Arc;

/// Main Weft application structure
pub struct WeftApp {
    terminal: Arc<terminal::TerminalEngine>,
    ai_engine: Arc<ai::AIEngine>,
    renderer: Arc<rendering::Renderer>,
    config: Arc<config::Config>,
    plugin_manager: Arc<plugins::PluginManager>,
}

impl WeftApp {
    /// Create a new Weft application instance
    pub async fn new() -> Result<Self> {
        let config = Arc::new(config::Config::load()?);
        let terminal = Arc::new(terminal::TerminalEngine::new(&config)?);
        let ai_engine = Arc::new(ai::AIEngine::new(&config)?);
        let renderer = Arc::new(rendering::Renderer::new(&config)?);
        let plugin_manager = Arc::new(plugins::PluginManager::new(&config)?);

        Ok(Self {
            terminal,
            ai_engine,
            renderer,
            config,
            plugin_manager,
        })
    }

    /// Initialize all subsystems
    pub async fn initialize(&mut self) -> Result<()> {
        tracing::info!("Initializing Weft Terminal");
        
        // Initialize terminal engine
        self.terminal.initialize().await?;
        
        // Initialize AI engine
        self.ai_engine.initialize().await?;
        
        // Initialize renderer
        self.renderer.initialize().await?;
        
        // Load plugins
        self.plugin_manager.load_plugins().await?;
        
        tracing::info!("Weft Terminal initialized successfully");
        Ok(())
    }

    /// Run the main application loop
    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Starting Weft Terminal main loop");
        
        loop {
            // Process terminal input/output
            self.terminal.process_events().await?;
            
            // Update AI predictions and suggestions
            self.ai_engine.update().await?;
            
            // Render the interface
            self.renderer.render().await?;
            
            // Handle plugin events
            self.plugin_manager.process_events().await?;
            
            // Small delay to prevent CPU spinning
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_weft_app_creation() {
        let app = WeftApp::new().await;
        assert!(app.is_ok());
    }
}
