use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use weft_terminal::config_simple::Config;
use weft_terminal::plugin_store::{self, PluginPaths};
use weft_terminal::WeftApp;

#[derive(Parser)]
#[command(name = "weft")]
#[command(about = "Next-generation AI-powered terminal")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the terminal
    Run {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(long)]
        debug: bool,
    },
    /// Show configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Plugin management
    Plugin {
        #[command(subcommand)]
        action: PluginAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    Show,
    Reset,
    Set { key: String, value: String },
    Get { key: String },
    Validate,
    Path,
}

#[derive(Subcommand)]
enum PluginAction {
    /// Installed plugins under the data directory (see `weft config path` for config location).
    List,
    /// Install by copying a plugin directory into the store (`plugin.toml` may set `name`).
    Install {
        /// Path to the plugin directory to copy.
        path: String,
    },
    Remove { name: String },
    Enable { name: String },
    Disable { name: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Commands::Run {
        config: None,
        debug: false,
    });

    // Initialize logging
    let log_level = if matches!(command, Commands::Run { debug: true, .. }) {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level.to_string())),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    match command {
        Commands::Run { config, debug: _ } => {
            run_terminal(config.map(PathBuf::from)).await?;
        }
        Commands::Config { action } => {
            handle_config(action).await?;
        }
        Commands::Plugin { action } => {
            handle_plugin(action)?;
        }
    }

    Ok(())
}

async fn run_terminal(config_path: Option<PathBuf>) -> Result<()> {
    tracing::info!("Starting Weft Terminal");

    let mut app = if let Some(path) = config_path {
        WeftApp::new_with_config_path(path).await?
    } else {
        WeftApp::new().await?
    };
    app.initialize().await?;

    // Shell handles signals while running; `WeftApp::run` also stops the child on Ctrl+C.
    app.run().await?;

    Ok(())
}

async fn handle_config(action: ConfigAction) -> Result<()> {
    let mut loaded_config = if config_action_needs_load(&action) {
        Some(Config::load()?)
    } else {
        None
    };

    match action {
        ConfigAction::Show => {
            let config = loaded_config
                .as_ref()
                .expect("config should be loaded for show");
            println!("{}", toml::to_string_pretty(&config)?);
        }
        ConfigAction::Reset => {
            Config::reset_to_default()?;
            println!("Configuration reset to defaults");
        }
        ConfigAction::Set { key, value } => {
            let config = loaded_config
                .as_mut()
                .expect("config should be loaded for set");
            config.set_value(&key, &value)?;
            println!("Updated {}={}", key, value);
        }
        ConfigAction::Get { key } => {
            let config = loaded_config
                .as_ref()
                .expect("config should be loaded for get");
            let value = config
                .get_value(&key)
                .ok_or_else(|| anyhow::anyhow!("Unknown config key '{}'", key))?;
            println!("{}", value);
        }
        ConfigAction::Validate => {
            let config = loaded_config
                .as_ref()
                .expect("config should be loaded for validate");
            config.validate()?;
            println!("Configuration is valid");
        }
        ConfigAction::Path => {
            println!("{}", Config::config_file_path().display());
        }
    }
    Ok(())
}

fn config_action_needs_load(action: &ConfigAction) -> bool {
    matches!(
        action,
        ConfigAction::Show
            | ConfigAction::Set { .. }
            | ConfigAction::Get { .. }
            | ConfigAction::Validate
    )
}

fn handle_plugin(action: PluginAction) -> Result<()> {
    let paths = PluginPaths::default_xdg();
    match action {
        PluginAction::List => {
            let plugins = plugin_store::list_plugins(&paths)?;
            if plugins.is_empty() {
                println!("(no plugins installed under {})", paths.plugins_dir.display());
                return Ok(());
            }
            println!("{:<24} {:<8} PATH", "ID", "ENABLED");
            for p in plugins {
                println!(
                    "{:<24} {:<8} {}",
                    p.id,
                    if p.enabled { "yes" } else { "no" },
                    p.path.display()
                );
            }
        }
        PluginAction::Install { path } => {
            let src = PathBuf::from(&path);
            let id = plugin_store::install_plugin(&paths, &src)?;
            println!("Installed plugin '{}' -> {}", id, paths.plugins_dir.join(&id).display());
        }
        PluginAction::Remove { name } => {
            plugin_store::remove_plugin(&paths, &name)?;
            println!("Removed plugin '{}'", name);
        }
        PluginAction::Enable { name } => {
            plugin_store::set_enabled(&paths, &name, true)?;
            println!("Enabled plugin '{}'", name);
        }
        PluginAction::Disable { name } => {
            plugin_store::set_enabled(&paths, &name, false)?;
            println!("Disabled plugin '{}'", name);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_actions_that_should_not_require_loading_are_recoverable() {
        assert!(!config_action_needs_load(&ConfigAction::Path));
        assert!(!config_action_needs_load(&ConfigAction::Reset));
    }

    #[test]
    fn config_actions_that_require_existing_config_are_marked_correctly() {
        assert!(config_action_needs_load(&ConfigAction::Show));
        assert!(config_action_needs_load(&ConfigAction::Get {
            key: "ai.model".to_string()
        }));
        assert!(config_action_needs_load(&ConfigAction::Set {
            key: "ai.model".to_string(),
            value: "llama3".to_string()
        }));
        assert!(config_action_needs_load(&ConfigAction::Validate));
    }
}
