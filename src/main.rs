use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use weft_terminal::config_simple::Config;
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
    List,
    Install { name: String },
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
            handle_plugin(action).await?;
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

    // Set up signal handlers for graceful shutdown
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        // Handle Ctrl+C
        if let Ok(()) = tokio::signal::ctrl_c().await {
            tracing::info!("Received shutdown signal");
            let _ = shutdown_tx.send(());
        }
    });

    // Run the application until shutdown
    tokio::select! {
        result = app.run() => {
            result?;
        }
        _ = &mut shutdown_rx => {
            tracing::info!("Shutting down gracefully");
        }
    }

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

async fn handle_plugin(action: PluginAction) -> Result<()> {
    match action {
        PluginAction::List => {
            // TODO: Implement plugin listing
            println!("Plugin listing not yet implemented");
        }
        PluginAction::Install { name } => {
            // TODO: Implement plugin installation
            println!("Installing plugin: {}", name);
        }
        PluginAction::Remove { name } => {
            // TODO: Implement plugin removal
            println!("Removing plugin: {}", name);
        }
        PluginAction::Enable { name } => {
            // TODO: Implement plugin enabling
            println!("Enabling plugin: {}", name);
        }
        PluginAction::Disable { name } => {
            // TODO: Implement plugin disabling
            println!("Disabling plugin: {}", name);
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
