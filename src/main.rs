use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use weft_terminal::config_simple::Config;
use weft_terminal::doctor;
use weft_terminal::plugin_store::{self, PluginPaths};
use weft_terminal::suggest;
use weft_terminal::WeftApp;

#[derive(Parser)]
#[command(name = "weft")]
#[command(about = "AI-assisted terminal environment")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start an interactive shell session
    Run {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(long)]
        debug: bool,
    },
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Plugin management
    Plugin {
        #[command(subcommand)]
        action: PluginAction,
    },
    /// Command suggestions (static rules + optional Ollama)
    Suggest {
        /// Partial command or natural-language intent
        query: String,
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Health checks for shell, config, plugins, and AI
    Doctor {
        #[arg(short, long)]
        config: Option<String>,
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
    Install { path: String },
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

    let log_level = if matches!(command, Commands::Run { debug: true, .. }) {
        tracing::Level::DEBUG
    } else {
        tracing::Level::WARN
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level.to_string())),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    match command {
        Commands::Run { config, debug: _ } => run_terminal(config.map(PathBuf::from)).await?,
        Commands::Config { action } => handle_config(action).await?,
        Commands::Plugin { action } => handle_plugin(action)?,
        Commands::Suggest { query, config } => handle_suggest(&query, config).await?,
        Commands::Doctor { config } => handle_doctor(config).await?,
    }

    Ok(())
}

async fn run_terminal(config_path: Option<PathBuf>) -> Result<()> {
    let mut app = match config_path {
        Some(p) => WeftApp::new_with_config_path(p).await?,
        None => WeftApp::new().await?,
    };
    app.initialize().await?;
    app.run().await
}

async fn handle_suggest(query: &str, config_path: Option<String>) -> Result<()> {
    let config = load_config_opt(config_path)?;
    let items = suggest::suggest(&config, query).await?;
    if items.is_empty() {
        println!("(no suggestions)");
        return Ok(());
    }
    for s in items {
        println!(
            "{:.0}%  [{}]  {}",
            s.confidence * 100.0,
            s.source,
            s.command
        );
    }
    Ok(())
}

async fn handle_doctor(config_path: Option<String>) -> Result<()> {
    let config = load_config_opt(config_path)?;
    let results = doctor::run_doctor(&config).await;
    let code = doctor::print_report(&results);
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

fn load_config_opt(path: Option<String>) -> Result<Config> {
    match path {
        Some(p) => Config::load_from_path(&PathBuf::from(p)),
        None => Config::load(),
    }
}

async fn handle_config(action: ConfigAction) -> Result<()> {
    let mut loaded_config = if config_action_needs_load(&action) {
        Some(Config::load()?)
    } else {
        None
    };

    match action {
        ConfigAction::Show => {
            let config = loaded_config.as_ref().expect("config loaded");
            println!("{}", toml::to_string_pretty(&config)?);
        }
        ConfigAction::Reset => {
            Config::reset_to_default()?;
            println!("Configuration reset to defaults");
        }
        ConfigAction::Set { key, value } => {
            let config = loaded_config.as_mut().expect("config loaded");
            config.set_value(&key, &value)?;
            println!("Updated {}={}", key, value);
        }
        ConfigAction::Get { key } => {
            let config = loaded_config.as_ref().expect("config loaded");
            let value = config
                .get_value(&key)
                .ok_or_else(|| anyhow::anyhow!("Unknown config key '{}'", key))?;
            println!("{}", value);
        }
        ConfigAction::Validate => {
            let config = loaded_config.as_ref().expect("config loaded");
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
    let config = Config::load()?;
    let paths = PluginPaths::from_config(&config);
    match action {
        PluginAction::List => {
            let plugins = plugin_store::list_plugins(&paths)?;
            if plugins.is_empty() {
                println!("(no plugins under {})", paths.plugins_dir.display());
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
            let id = plugin_store::install_plugin(&paths, &PathBuf::from(path))?;
            println!(
                "Installed '{}' -> {}",
                id,
                paths.plugins_dir.join(&id).display()
            );
        }
        PluginAction::Remove { name } => {
            plugin_store::remove_plugin(&paths, &name)?;
            println!("Removed '{}'", name);
        }
        PluginAction::Enable { name } => {
            plugin_store::set_enabled(&paths, &name, true)?;
            println!("Enabled '{}'", name);
        }
        PluginAction::Disable { name } => {
            plugin_store::set_enabled(&paths, &name, false)?;
            println!("Disabled '{}'", name);
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
    }
}
