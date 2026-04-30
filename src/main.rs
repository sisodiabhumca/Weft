use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
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

    // Initialize logging
    let log_level = if let Some(Commands::Run { debug: true, .. }) = cli.command {
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

    match cli.command.unwrap_or(Commands::Run {
        config: None,
        debug: false,
    }) {
        Commands::Run { config: _, debug: _ } => {
            run_terminal().await?;
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

async fn run_terminal() -> Result<()> {
    tracing::info!("Starting Weft Terminal");
    
    let mut app = WeftApp::new().await?;
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
    match action {
        ConfigAction::Show => {
            // TODO: Implement config display
            println!("Configuration display not yet implemented");
        }
        ConfigAction::Reset => {
            // TODO: Implement config reset
            println!("Configuration reset not yet implemented");
        }
        ConfigAction::Set { key, value } => {
            // TODO: Implement config setting
            println!("Setting config: {} = {}", key, value);
        }
    }
    Ok(())
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
