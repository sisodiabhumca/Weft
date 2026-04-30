//! Terminal engine implementation with advanced features
//! 
//! This module provides the core terminal emulation functionality,
//! including command processing, session management, and output handling.

use anyhow::Result;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

pub struct TerminalEngine {
    config: Arc<crate::config::Config>,
    sessions: Arc<RwLock<Vec<TerminalSession>>>,
    event_tx: mpsc::UnboundedSender<TerminalEvent>,
    event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<TerminalEvent>>>>,
}

#[derive(Debug, Clone)]
pub enum TerminalEvent {
    Input { session_id: String, data: Vec<u8> },
    Output { session_id: String, data: Vec<u8> },
    SessionCreated { session_id: String },
    SessionClosed { session_id: String },
    CommandExecuted { session_id: String, command: String, exit_code: i32 },
}

#[derive(Debug, Clone)]
pub struct TerminalSession {
    pub id: String,
    pub working_directory: String,
    pub environment: std::collections::HashMap<String, String>,
    pub history: Vec<CommandHistory>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct CommandHistory {
    pub command: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub exit_code: Option<i32>,
    pub duration: Option<std::time::Duration>,
    pub output_size: usize,
}

impl TerminalEngine {
    pub fn new(config: &Arc<crate::config::Config>) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            config: config.clone(),
            sessions: Arc::new(RwLock::new(Vec::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing terminal engine");
        
        // Create default session
        let session_id = uuid::Uuid::new_v4().to_string();
        let session = TerminalSession {
            id: session_id.clone(),
            working_directory: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("/"))
                .to_string_lossy()
                .to_string(),
            environment: std::env::vars().collect(),
            history: Vec::new(),
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
        };

        self.sessions.write().push(session);
        
        if let Err(e) = self.event_tx.send(TerminalEvent::SessionCreated {
            session_id: session_id.clone(),
        }) {
            warn!("Failed to send session created event: {}", e);
        }

        info!("Terminal engine initialized with session: {}", session_id);
        Ok(())
    }

    pub async fn process_events(&self) -> Result<()> {
        let mut event_rx = self.event_rx.write().take().unwrap();
        
        while let Some(event) = event_rx.recv().await {
            debug!("Processing terminal event: {:?}", event);
            
            match event {
                TerminalEvent::Input { session_id, data } => {
                    self.handle_input(session_id, data).await?;
                }
                TerminalEvent::Output { session_id, data } => {
                    self.handle_output(session_id, data).await?;
                }
                TerminalEvent::SessionCreated { session_id } => {
                    debug!("Session created: {}", session_id);
                }
                TerminalEvent::SessionClosed { session_id } => {
                    self.handle_session_closed(session_id).await?;
                }
                TerminalEvent::CommandExecuted { session_id, command, exit_code } => {
                    self.handle_command_executed(session_id, command, exit_code).await?;
                }
            }
        }

        // Put the receiver back
        *self.event_rx.write() = Some(event_rx);
        Ok(())
    }

    async fn handle_input(&self, session_id: String, data: Vec<u8>) -> Result<()> {
        debug!("Handling input for session {}: {:?}", session_id, data);
        
        // Parse command from input
        let input_str = String::from_utf8_lossy(&data);
        if let Some(command) = input_str.lines().next() {
            if !command.trim().is_empty() {
                self.execute_command(session_id, command.to_string()).await?;
            }
        }
        
        Ok(())
    }

    async fn handle_output(&self, session_id: String, data: Vec<u8>) -> Result<()> {
        debug!("Handling output for session {}: {} bytes", session_id, data.len());
        
        // Store output in session history
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.iter_mut().find(|s| s.id == session_id) {
            session.last_activity = chrono::Utc::now();
            
            // Update last command's output size
            if let Some(last_command) = session.history.last_mut() {
                last_command.output_size += data.len();
            }
        }
        
        Ok(())
    }

    async fn handle_session_closed(&self, session_id: String) -> Result<()> {
        info!("Session closed: {}", session_id);
        
        let mut sessions = self.sessions.write();
        sessions.retain(|s| s.id != session_id);
        
        Ok(())
    }

    async fn handle_command_executed(&self, session_id: String, command: String, exit_code: i32) -> Result<()> {
        info!("Command executed in session {}: '{}' (exit: {})", session_id, command, exit_code);
        
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.iter_mut().find(|s| s.id == session_id) {
            session.last_activity = chrono::Utc::now();
            
            // Update command history
            if let Some(last_command) = session.history.last_mut() {
                last_command.exit_code = Some(exit_code);
            }
        }
        
        Ok(())
    }

    async fn execute_command(&self, session_id: String, command: String) -> Result<()> {
        info!("Executing command in session {}: '{}'", session_id, command);
        
        let start_time = std::time::Instant::now();
        let timestamp = chrono::Utc::now();
        
        // Add to history
        {
            let mut sessions = self.sessions.write();
            if let Some(session) = sessions.iter_mut().find(|s| s.id == session_id) {
                session.history.push(CommandHistory {
                    command: command.clone(),
                    timestamp,
                    exit_code: None,
                    duration: None,
                    output_size: 0,
                });
            }
        }
        
        // Execute command (simplified for now)
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .current_dir({
                let sessions = self.sessions.read();
                sessions.iter()
                    .find(|s| s.id == session_id)
                    .map(|s| &s.working_directory)
                    .unwrap_or(&std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/")).to_string_lossy().to_string())
                    .clone()
            })
            .output()
            .await?;
            
        let duration = start_time.elapsed();
        let exit_code = output.status.code().unwrap_or(-1);
        
        // Send output event
        if let Err(e) = self.event_tx.send(TerminalEvent::Output {
            session_id: session_id.clone(),
            data: output.stdout,
        }) {
            warn!("Failed to send output event: {}", e);
        }
        
        if let Err(e) = self.event_tx.send(TerminalEvent::Output {
            session_id: session_id.clone(),
            data: output.stderr,
        }) {
            warn!("Failed to send stderr event: {}", e);
        }
        
        // Send command executed event
        if let Err(e) = self.event_tx.send(TerminalEvent::CommandExecuted {
            session_id,
            command,
            exit_code,
        }) {
            warn!("Failed to send command executed event: {}", e);
        }
        
        Ok(())
    }

    pub fn create_session(&self, working_directory: Option<String>) -> Result<String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let session = TerminalSession {
            id: session_id.clone(),
            working_directory: working_directory.unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("/"))
                    .to_string_lossy()
                    .to_string()
            }),
            environment: std::env::vars().collect(),
            history: Vec::new(),
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
        };

        self.sessions.write().push(session);
        
        if let Err(e) = self.event_tx.send(TerminalEvent::SessionCreated {
            session_id: session_id.clone(),
        }) {
            warn!("Failed to send session created event: {}", e);
        }

        Ok(session_id)
    }

    pub fn get_session(&self, session_id: &str) -> Option<TerminalSession> {
        self.sessions.read().iter()
            .find(|s| s.id == session_id)
            .cloned()
    }

    pub fn get_all_sessions(&self) -> Vec<TerminalSession> {
        self.sessions.read().clone()
    }

    pub fn send_input(&self, session_id: &str, data: Vec<u8>) -> Result<()> {
        self.event_tx.send(TerminalEvent::Input {
            session_id: session_id.to_string(),
            data,
        })?;
        Ok(())
    }
}
