//! Weft Terminal - AI-assisted shell environment

pub mod config_simple;
pub mod doctor;
pub mod plugin_store;
pub mod pty;
pub mod suggest;

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

/// Main Weft application structure
pub struct WeftApp {
    config: Arc<config_simple::Config>,
}

impl WeftApp {
    pub async fn new() -> Result<Self> {
        Self::new_with_config_path(config_simple::Config::config_file_path()).await
    }

    pub async fn new_with_config_path(config_path: PathBuf) -> Result<Self> {
        let config = Arc::new(config_simple::Config::load_from_path(&config_path)?);
        config.validate()?;
        Ok(Self { config })
    }

    pub fn config(&self) -> &config_simple::Config {
        &self.config
    }

    pub async fn initialize(&mut self) -> Result<()> {
        tracing::info!("Initializing Weft Terminal");
        tracing::debug!("Shell: {}", self.config.terminal.shell);
        tracing::debug!("PTY: {}", self.config.terminal.use_pty);
        tracing::debug!("AI: {} ({})", self.config.ai.model, self.config.ai.provider);

        if self.config.plugins.enabled && self.config.plugins.run_hooks_on_startup {
            let paths = plugin_store::PluginPaths::from_config(&self.config);
            plugin_store::run_startup_hooks(&paths)?;
        }

        tracing::info!("Weft Terminal initialized");
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let shell = self.config.terminal.shell.trim();
        if shell.is_empty() {
            anyhow::bail!("terminal.shell is empty");
        }

        if self.config.terminal.use_pty {
            tracing::info!("Launching shell with PTY: {}", shell);
            let shell = shell.to_string();
            let handle = tokio::task::spawn_blocking(move || pty::run_interactive_shell(&shell));

            tokio::select! {
                res = handle => {
                    res??;
                }
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Ctrl+C received, stopping shell");
                    pty::request_shutdown();
                }
            }
        } else {
            tracing::info!("Launching shell (no PTY): {}", shell);
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
