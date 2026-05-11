//! Weft Terminal - Next-generation AI-powered development environment
//!
//! This library provides the core functionality for Weft, a modern terminal
//! that combines powerful AI capabilities with collaborative features and
//! extensible plugin architecture.

pub mod config_simple;
pub mod plugin_store;

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Stdio;
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

    /// Run an interactive session: spawn the configured shell with inherited stdio and wait.
    pub async fn run(&mut self) -> Result<()> {
        let shell = self.config.terminal.shell.trim();
        if shell.is_empty() {
            anyhow::bail!("terminal.shell is empty");
        }

        tracing::info!("Launching shell: {}", shell);

        let mut cmd = tokio::process::Command::new(shell);
        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .with_context(|| format!("failed to spawn shell '{}'", shell))?;

        tokio::select! {
            status = child.wait() => {
                let status = status?;
                if !status.success() {
                    tracing::info!("shell exited with code {:?}", status.code());
                }
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Ctrl+C received, stopping shell");
                let _ = child.start_kill();
                let _ = child.wait().await;
            }
        }

        Ok(())
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
