//! Weft Terminal - Next-generation AI-powered development environment
//!
//! This library provides the core functionality for Weft, a modern terminal
//! that combines powerful AI capabilities with collaborative features and
//! extensible plugin architecture.

pub mod config_simple;

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

/// Main Weft application structure
pub struct WeftApp {
    config: Arc<config_simple::Config>,
}

impl WeftApp {
    /// Create a new Weft application instance
    pub async fn new() -> Result<Self> {
        Self::new_with_config_path(config_simple::Config::config_file_path()).await
    }

    /// Create a new Weft application instance with a custom config path.
    pub async fn new_with_config_path(config_path: PathBuf) -> Result<Self> {
        let config = Arc::new(config_simple::Config::load_from_path(&config_path)?);
        config.validate()?;

        Ok(Self { config })
    }

    /// Initialize all subsystems
    pub async fn initialize(&mut self) -> Result<()> {
        tracing::info!("Initializing Weft Terminal");

        tracing::debug!("Using shell: {}", self.config.terminal.shell);
        tracing::debug!("AI model: {}", self.config.ai.model);
        tracing::info!("Weft Terminal initialized successfully");
        Ok(())
    }

    /// Run the main application loop
    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Starting Weft Terminal main loop");

        loop {
            // Small delay to prevent CPU spinning
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
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
