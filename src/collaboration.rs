//! Real-time collaboration and session sharing features
//! 
//! This module provides collaborative terminal sessions, allowing
//! multiple users to work together in shared environments.

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use tracing::{debug, info, warn};
use uuid::Uuid;

pub struct CollaborationEngine {
    config: Arc<crate::config::Config>,
    sessions: Arc<RwLock<HashMap<String, CollaborativeSession>>>,
    connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
    event_tx: mpsc::UnboundedSender<CollaborationEvent>,
    event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<CollaborationEvent>>>>,
}

#[derive(Debug, Clone)]
pub enum CollaborationEvent {
    SessionCreated { session_id: String, owner: String },
    SessionJoined { session_id: String, user: String },
    SessionLeft { session_id: String, user: String },
    MessageReceived { session_id: String, user: String, message: CollaborationMessage },
    TerminalInputShared { session_id: String, user: String, input: String },
    TerminalOutputShared { session_id: String, output: String },
    CursorMoved { session_id: String, user: String, position: (usize, usize) },
}

#[derive(Debug, Clone)]
pub struct CollaborativeSession {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub participants: Vec<SessionParticipant>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub settings: SessionSettings,
    pub terminal_state: TerminalState,
}

#[derive(Debug, Clone)]
pub struct SessionParticipant {
    pub id: String,
    pub name: String,
    pub role: ParticipantRole,
    pub joined_at: chrono::DateTime<chrono::Utc>,
    pub cursor_position: Option<(usize, usize)>,
    pub color: (u8, u8, u8),
    pub permissions: ParticipantPermissions,
}

#[derive(Debug, Clone)]
pub enum ParticipantRole {
    Owner,
    Editor,
    Viewer,
}

#[derive(Debug, Clone)]
pub struct ParticipantPermissions {
    pub can_input: bool,
    pub can_execute_commands: bool,
    pub can_modify_settings: bool,
    pub can_invite_others: bool,
}

#[derive(Debug, Clone)]
pub struct SessionSettings {
    pub read_only: bool,
    pub require_approval_for_commands: bool,
    pub auto_share_output: bool,
    pub encryption_enabled: bool,
    pub max_participants: usize,
    pub session_timeout_minutes: u64,
}

#[derive(Debug, Clone)]
pub struct TerminalState {
    pub current_directory: String,
    pub environment: HashMap<String, String>,
    pub command_history: Vec<String>,
    pub scrollback: Vec<String>,
    pub cursor_position: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationMessage {
    pub id: String,
    pub message_type: MessageType,
    pub sender: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub content: serde_json::Value,
    pub encrypted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Chat,
    Command,
    Output,
    FileTransfer,
    CursorUpdate,
    SessionInfo,
    UserJoined,
    UserLeft,
}

#[derive(Debug)]
pub struct WebSocketConnection {
    pub session_id: String,
    pub user_id: String,
    pub write_tx: mpsc::UnboundedSender<Message>,
    pub last_ping: chrono::DateTime<chrono::Utc>,
}

impl CollaborationEngine {
    pub fn new(config: &Arc<crate::config::Config>) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            config: config.clone(),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing collaboration engine");
        
        if !self.config.collaboration.enabled {
            info!("Collaboration features are disabled");
            return Ok(());
        }

        // Initialize WebSocket server if configured
        if let Some(server_url) = &self.config.collaboration.server_url {
            self.start_websocket_server(server_url).await?;
        }

        info!("Collaboration engine initialized");
        Ok(())
    }

    async fn start_websocket_server(&self, server_url: &str) -> Result<()> {
        info!("Starting WebSocket server at: {}", server_url);
        
        // In a real implementation, this would start a WebSocket server
        // For now, we'll just log that it would start
        
        Ok(())
    }

    pub async fn process_events(&self) -> Result<()> {
        let mut event_rx = self.event_rx.write().take().unwrap();
        
        while let Some(event) = event_rx.recv().await {
            debug!("Processing collaboration event: {:?}", event);
            
            match event {
                CollaborationEvent::SessionCreated { session_id, owner } => {
                    self.handle_session_created(session_id, owner).await?;
                }
                CollaborationEvent::SessionJoined { session_id, user } => {
                    self.handle_session_joined(session_id, user).await?;
                }
                CollaborationEvent::SessionLeft { session_id, user } => {
                    self.handle_session_left(session_id, user).await?;
                }
                CollaborationEvent::MessageReceived { session_id, user, message } => {
                    self.handle_message_received(session_id, user, message).await?;
                }
                CollaborationEvent::TerminalInputShared { session_id, user, input } => {
                    self.handle_terminal_input_shared(session_id, user, input).await?;
                }
                CollaborationEvent::TerminalOutputShared { session_id, output } => {
                    self.handle_terminal_output_shared(session_id, output).await?;
                }
                CollaborationEvent::CursorMoved { session_id, user, position } => {
                    self.handle_cursor_moved(session_id, user, position).await?;
                }
            }
        }

        // Put the receiver back
        *self.event_rx.write() = Some(event_rx);
        Ok(())
    }

    async fn handle_session_created(&self, session_id: String, owner: String) -> Result<()> {
        info!("Collaborative session created: {} by {}", session_id, owner);
        
        let session = CollaborativeSession {
            id: session_id.clone(),
            name: format!("Session {}", &session_id[..8]),
            owner: owner.clone(),
            participants: vec![],
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            settings: SessionSettings::default(),
            terminal_state: TerminalState::default(),
        };

        self.sessions.write().insert(session_id.clone(), session);
        Ok(())
    }

    async fn handle_session_joined(&self, session_id: String, user: String) -> Result<()> {
        info!("User {} joined session: {}", user, session_id);
        
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(&session_id) {
            let participant = SessionParticipant {
                id: Uuid::new_v4().to_string(),
                name: user.clone(),
                role: ParticipantRole::Editor,
                joined_at: chrono::Utc::now(),
                cursor_position: None,
                color: self.generate_user_color(),
                permissions: ParticipantPermissions::default(),
            };
            
            session.participants.push(participant);
            session.last_activity = chrono::Utc::now();
        }
        
        Ok(())
    }

    async fn handle_session_left(&self, session_id: String, user: String) -> Result<()> {
        info!("User {} left session: {}", user, session_id);
        
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(&session_id) {
            session.participants.retain(|p| p.name != user);
            session.last_activity = chrono::Utc::now();
        }
        
        Ok(())
    }

    async fn handle_message_received(&self, session_id: String, user: String, message: CollaborationMessage) -> Result<()> {
        debug!("Message received in session {} from {}: {:?}", session_id, user, message.message_type);
        
        // Broadcast message to all participants
        self.broadcast_message(&session_id, &message).await?;
        
        Ok(())
    }

    async fn handle_terminal_input_shared(&self, session_id: String, user: String, input: String) -> Result<()> {
        debug!("Terminal input shared in session {} by {}: {}", session_id, user, input);
        
        let message = CollaborationMessage {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::Command,
            sender: user,
            timestamp: chrono::Utc::now(),
            content: serde_json::json!({ "input": input }),
            encrypted: self.config.collaboration.encryption_enabled,
        };
        
        self.broadcast_message(&session_id, &message).await?;
        Ok(())
    }

    async fn handle_terminal_output_shared(&self, session_id: String, output: String) -> Result<()> {
        debug!("Terminal output shared in session {}: {} bytes", session_id, output.len());
        
        let message = CollaborationMessage {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::Output,
            sender: "system".to_string(),
            timestamp: chrono::Utc::now(),
            content: serde_json::json!({ "output": output }),
            encrypted: self.config.collaboration.encryption_enabled,
        };
        
        self.broadcast_message(&session_id, &message).await?;
        Ok(())
    }

    async fn handle_cursor_moved(&self, session_id: String, user: String, position: (usize, usize)) -> Result<()> {
        debug!("Cursor moved in session {} by {}: {:?}", session_id, user, position);
        
        let message = CollaborationMessage {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::CursorUpdate,
            sender: user,
            timestamp: chrono::Utc::now(),
            content: serde_json::json!({ "position": position }),
            encrypted: false,
        };
        
        self.broadcast_message(&session_id, &message).await?;
        Ok(())
    }

    async fn broadcast_message(&self, session_id: &str, message: &CollaborationMessage) -> Result<()> {
        let connections = self.connections.read();
        
        for (connection_id, connection) in connections.iter() {
            if connection.session_id == session_id {
                let message_json = serde_json::to_string(message)?;
                if let Err(e) = connection.write_tx.send(Message::Text(message_json)) {
                    warn!("Failed to send message to connection {}: {}", connection_id, e);
                }
            }
        }
        
        Ok(())
    }

    pub async fn create_session(&self, name: String, owner: String) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        
        if let Err(e) = self.event_tx.send(CollaborationEvent::SessionCreated {
            session_id: session_id.clone(),
            owner,
        }) {
            warn!("Failed to send session created event: {}", e);
        }
        
        Ok(session_id)
    }

    pub async fn join_session(&self, session_id: String, user: String, server_url: Option<String>) -> Result<()> {
        if let Some(server_url) = server_url {
            // Connect to remote session
            self.connect_to_remote_session(&session_id, &user, &server_url).await?;
        } else {
            // Join local session
            if let Err(e) = self.event_tx.send(CollaborationEvent::SessionJoined {
                session_id: session_id.clone(),
                user,
            }) {
                warn!("Failed to send session joined event: {}", e);
            }
        }
        
        Ok(())
    }

    async fn connect_to_remote_session(&self, session_id: &str, user: &str, server_url: &str) -> Result<()> {
        info!("Connecting to remote session {} at {}", session_id, server_url);
        
        let ws_url = format!("{}/session/{}/join/{}", server_url, session_id, user);
        let (ws_stream, _) = connect_async(&ws_url).await?;
        let (write_tx, write_rx) = mpsc::unbounded_channel();
        
        let connection = WebSocketConnection {
            session_id: session_id.to_string(),
            user_id: user.to_string(),
            write_tx,
            last_ping: chrono::Utc::now(),
        };
        
        let connection_id = Uuid::new_v4().to_string();
        self.connections.write().insert(connection_id.clone(), connection);
        
        // Handle WebSocket messages
        let mut ws_stream = ws_stream;
        tokio::spawn(async move {
            while let Some(msg) = ws_stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        debug!("Received WebSocket message: {}", text);
                        // Handle incoming message
                    }
                    Ok(Message::Binary(data)) => {
                        debug!("Received binary data: {} bytes", data.len());
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket connection closed");
                        break;
                    }
                    Err(e) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });
        
        Ok(())
    }

    pub async fn leave_session(&self, session_id: String, user: String) -> Result<()> {
        if let Err(e) = self.event_tx.send(CollaborationEvent::SessionLeft {
            session_id: session_id.clone(),
            user,
        }) {
            warn!("Failed to send session left event: {}", e);
        }
        
        Ok(())
    }

    pub async fn send_chat_message(&self, session_id: String, user: String, content: String) -> Result<()> {
        let message = CollaborationMessage {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::Chat,
            sender: user,
            timestamp: chrono::Utc::now(),
            content: serde_json::json!({ "content": content }),
            encrypted: false,
        };
        
        if let Err(e) = self.event_tx.send(CollaborationEvent::MessageReceived {
            session_id,
            user: message.sender.clone(),
            message,
        }) {
            warn!("Failed to send message received event: {}", e);
        }
        
        Ok(())
    }

    pub async fn share_terminal_input(&self, session_id: String, user: String, input: String) -> Result<()> {
        if let Err(e) = self.event_tx.send(CollaborationEvent::TerminalInputShared {
            session_id,
            user,
            input,
        }) {
            warn!("Failed to send terminal input shared event: {}", e);
        }
        
        Ok(())
    }

    pub async fn share_terminal_output(&self, session_id: String, output: String) -> Result<()> {
        if let Err(e) = self.event_tx.send(CollaborationEvent::TerminalOutputShared {
            session_id,
            output,
        }) {
            warn!("Failed to send terminal output shared event: {}", e);
        }
        
        Ok(())
    }

    pub async fn update_cursor_position(&self, session_id: String, user: String, position: (usize, usize)) -> Result<()> {
        if let Err(e) = self.event_tx.send(CollaborationEvent::CursorMoved {
            session_id,
            user,
            position,
        }) {
            warn!("Failed to send cursor moved event: {}", e);
        }
        
        Ok(())
    }

    pub fn get_session(&self, session_id: &str) -> Option<CollaborativeSession> {
        self.sessions.read().get(session_id).cloned()
    }

    pub fn list_sessions(&self) -> Vec<CollaborativeSession> {
        self.sessions.read().values().cloned().collect()
    }

    fn generate_user_color(&self) -> (u8, u8, u8) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        std::thread::current().id().hash(&mut hasher);
        let hash = hasher.finish();
        
        let r = ((hash >> 16) & 0xFF) as u8;
        let g = ((hash >> 8) & 0xFF) as u8;
        let b = (hash & 0xFF) as u8;
        
        (r, g, b)
    }
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            read_only: false,
            require_approval_for_commands: false,
            auto_share_output: true,
            encryption_enabled: true,
            max_participants: 10,
            session_timeout_minutes: 120,
        }
    }
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            current_directory: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("/"))
                .to_string_lossy()
                .to_string(),
            environment: std::env::vars().collect(),
            command_history: Vec::new(),
            scrollback: Vec::new(),
            cursor_position: (0, 0),
        }
    }
}

impl Default for ParticipantPermissions {
    fn default() -> Self {
        Self {
            can_input: true,
            can_execute_commands: true,
            can_modify_settings: false,
            can_invite_others: false,
        }
    }
}
