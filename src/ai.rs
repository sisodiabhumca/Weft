//! AI-powered terminal assistance and automation
//! 
//! This module provides intelligent command prediction, completion,
//! and automation features that enhance the terminal experience.

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

pub struct AIEngine {
    config: Arc<crate::config::Config>,
    model_manager: Arc<RwLock<ModelManager>>,
    prediction_cache: Arc<RwLock<std::collections::HashMap<String, Vec<CommandPrediction>>>>,
    event_tx: mpsc::UnboundedSender<AIEvent>,
    event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<AIEvent>>>>,
}

#[derive(Debug, Clone)]
pub enum AIEvent {
    CommandInput { session_id: String, command: String },
    CommandExecuted { session_id: String, command: String, exit_code: i32 },
    PredictionRequest { session_id: String, context: String },
    AutomationRequest { session_id: String, task: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPrediction {
    pub command: String,
    pub confidence: f32,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationSuggestion {
    pub name: String,
    pub description: String,
    pub script: String,
    pub triggers: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct ModelManager {
    pub active_model: String,
    pub available_models: Vec<ModelInfo>,
    pub context_window: usize,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub provider: ModelProvider,
    pub capabilities: Vec<ModelCapability>,
    pub max_tokens: usize,
}

#[derive(Debug, Clone)]
pub enum ModelProvider {
    Ollama,
    OpenAI,
    Anthropic,
    Local,
}

#[derive(Debug, Clone)]
pub enum ModelCapability {
    CommandPrediction,
    CodeGeneration,
    TextCompletion,
    CodeAnalysis,
}

impl AIEngine {
    pub fn new(config: &Arc<crate::config::Config>) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        let model_manager = ModelManager {
            active_model: "llama2".to_string(),
            available_models: vec![
                ModelInfo {
                    name: "llama2".to_string(),
                    provider: ModelProvider::Ollama,
                    capabilities: vec![
                        ModelCapability::CommandPrediction,
                        ModelCapability::TextCompletion,
                    ],
                    max_tokens: 4096,
                },
                ModelInfo {
                    name: "codellama".to_string(),
                    provider: ModelProvider::Ollama,
                    capabilities: vec![
                        ModelCapability::CommandPrediction,
                        ModelCapability::CodeGeneration,
                        ModelCapability::CodeAnalysis,
                    ],
                    max_tokens: 4096,
                },
            ],
            context_window: 4096,
        };

        Ok(Self {
            config: config.clone(),
            model_manager: Arc::new(RwLock::new(model_manager)),
            prediction_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing AI engine");
        
        // Initialize Ollama connection
        self.initialize_ollama().await?;
        
        // Load prediction models
        self.load_prediction_models().await?;
        
        info!("AI engine initialized successfully");
        Ok(())
    }

    async fn initialize_ollama(&self) -> Result<()> {
        info!("Initializing Ollama connection");
        
        // Check if Ollama is running
        match reqwest::get("http://localhost:11434/api/tags").await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Ollama is running and accessible");
                } else {
                    warn!("Ollama returned non-success status: {}", response.status());
                }
            }
            Err(e) => {
                warn!("Failed to connect to Ollama: {}", e);
                info!("AI features will be limited without Ollama");
            }
        }
        
        Ok(())
    }

    async fn load_prediction_models(&self) -> Result<()> {
        info!("Loading prediction models");
        
        // Pre-load some common command patterns
        let mut cache = self.prediction_cache.write();
        cache.insert("git".to_string(), vec![
            CommandPrediction {
                command: "git status".to_string(),
                confidence: 0.95,
                description: Some("Show working tree status".to_string()),
                tags: vec!["git".to_string(), "status".to_string()],
            },
            CommandPrediction {
                command: "git add .".to_string(),
                confidence: 0.90,
                description: Some("Add all changes to staging area".to_string()),
                tags: vec!["git".to_string(), "add".to_string()],
            },
            CommandPrediction {
                command: "git commit -m".to_string(),
                confidence: 0.85,
                description: Some("Commit changes with message".to_string()),
                tags: vec!["git".to_string(), "commit".to_string()],
            },
        ]);
        
        cache.insert("docker".to_string(), vec![
            CommandPrediction {
                command: "docker ps".to_string(),
                confidence: 0.95,
                description: Some("List running containers".to_string()),
                tags: vec!["docker".to_string(), "list".to_string()],
            },
            CommandPrediction {
                command: "docker build -t".to_string(),
                confidence: 0.90,
                description: Some("Build Docker image".to_string()),
                tags: vec!["docker".to_string(), "build".to_string()],
            },
        ]);
        
        Ok(())
    }

    pub async fn update(&self) -> Result<()> {
        let mut event_rx = self.event_rx.write().take().unwrap();
        
        while let Some(event) = event_rx.recv().await {
            debug!("Processing AI event: {:?}", event);
            
            match event {
                AIEvent::CommandInput { session_id, command } => {
                    self.handle_command_input(session_id, command).await?;
                }
                AIEvent::CommandExecuted { session_id, command, exit_code } => {
                    self.handle_command_executed(session_id, command, exit_code).await?;
                }
                AIEvent::PredictionRequest { session_id, context } => {
                    self.handle_prediction_request(session_id, context).await?;
                }
                AIEvent::AutomationRequest { session_id, task } => {
                    self.handle_automation_request(session_id, task).await?;
                }
            }
        }

        // Put the receiver back
        *self.event_rx.write() = Some(event_rx);
        Ok(())
    }

    async fn handle_command_input(&self, session_id: String, command: String) -> Result<()> {
        debug!("Handling command input for session {}: {}", session_id, command);
        
        // Generate predictions based on current input
        let predictions = self.generate_predictions(&command).await?;
        
        // Store predictions for this session
        let cache_key = format!("{}:{}", session_id, command);
        let mut cache = self.prediction_cache.write();
        cache.insert(cache_key, predictions);
        
        Ok(())
    }

    async fn handle_command_executed(&self, session_id: String, command: String, exit_code: i32) -> Result<()> {
        debug!("Handling command executed for session {}: '{}' (exit: {})", session_id, command, exit_code);
        
        // Learn from successful commands
        if exit_code == 0 {
            self.update_command_patterns(&command).await?;
        }
        
        Ok(())
    }

    async fn handle_prediction_request(&self, session_id: String, context: String) -> Result<()> {
        debug!("Handling prediction request for session {}: {}", session_id, context);
        
        let predictions = self.generate_predictions(&context).await?;
        
        // Send predictions back to terminal
        // TODO: Implement proper communication channel
        
        Ok(())
    }

    async fn handle_automation_request(&self, session_id: String, task: String) -> Result<()> {
        debug!("Handling automation request for session {}: {}", session_id, task);
        
        let suggestions = self.generate_automation_suggestions(&task).await?;
        
        // Send suggestions back to terminal
        // TODO: Implement proper communication channel
        
        Ok(())
    }

    async fn generate_predictions(&self, input: &str) -> Result<Vec<CommandPrediction>> {
        let cache = self.prediction_cache.read();
        
        // Check cache first
        for (key, predictions) in cache.iter() {
            if input.starts_with(key) {
                return Ok(predictions.clone());
            }
        }
        
        // Generate basic predictions based on input
        let mut predictions = Vec::new();
        
        if input.trim().is_empty() {
            // Common commands for empty input
            predictions.extend_from_slice(&[
                CommandPrediction {
                    command: "ls".to_string(),
                    confidence: 0.80,
                    description: Some("List directory contents".to_string()),
                    tags: vec!["file".to_string(), "list".to_string()],
                },
                CommandPrediction {
                    command: "cd".to_string(),
                    confidence: 0.75,
                    description: Some("Change directory".to_string()),
                    tags: vec!["navigation".to_string()],
                },
                CommandPrediction {
                    command: "git status".to_string(),
                    confidence: 0.70,
                    description: Some("Show git status".to_string()),
                    tags: vec!["git".to_string()],
                },
            ]);
        } else {
            // Predict based on partial input
            let words: Vec<&str> = input.split_whitespace().collect();
            if let Some(first_word) = words.first() {
                if let Some(cached_predictions) = cache.get(*first_word) {
                    predictions = cached_predictions.clone();
                }
            }
        }
        
        // Sort by confidence
        predictions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        Ok(predictions)
    }

    async fn generate_automation_suggestions(&self, task: &str) -> Result<Vec<AutomationSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Generate suggestions based on task description
        if task.contains("deploy") {
            suggestions.push(AutomationSuggestion {
                name: "Deploy Application".to_string(),
                description: "Automated deployment pipeline".to_string(),
                script: "docker build -t app . && docker push registry/app:latest".to_string(),
                triggers: vec!["deploy".to_string(), "push".to_string()],
                confidence: 0.85,
            });
        }
        
        if task.contains("test") {
            suggestions.push(AutomationSuggestion {
                name: "Run Tests".to_string(),
                description: "Comprehensive test suite".to_string(),
                script: "cargo test && npm test".to_string(),
                triggers: vec!["test".to_string(), "check".to_string()],
                confidence: 0.90,
            });
        }
        
        Ok(suggestions)
    }

    async fn update_command_patterns(&self, command: &str) -> Result<()> {
        debug!("Updating command patterns for: {}", command);
        
        // Extract command patterns for learning
        let words: Vec<&str> = command.split_whitespace().collect();
        if let Some(first_word) = words.first() {
            let mut cache = self.prediction_cache.write();
            
            if !cache.contains_key(*first_word) {
                cache.insert(first_word.to_string(), Vec::new());
            }
            
            if let Some(predictions) = cache.get_mut(*first_word) {
                // Add this command to predictions if not already present
                if !predictions.iter().any(|p| p.command == command) {
                    predictions.push(CommandPrediction {
                        command: command.to_string(),
                        confidence: 0.60, // Start with moderate confidence
                        description: None,
                        tags: vec![first_word.to_string()],
                    });
                }
            }
        }
        
        Ok(())
    }

    pub fn send_command_input(&self, session_id: &str, command: String) -> Result<()> {
        self.event_tx.send(AIEvent::CommandInput {
            session_id: session_id.to_string(),
            command,
        })?;
        Ok(())
    }

    pub fn request_prediction(&self, session_id: &str, context: String) -> Result<()> {
        self.event_tx.send(AIEvent::PredictionRequest {
            session_id: session_id.to_string(),
            context,
        })?;
        Ok(())
    }

    pub fn request_automation(&self, session_id: &str, task: String) -> Result<()> {
        self.event_tx.send(AIEvent::AutomationRequest {
            session_id: session_id.to_string(),
            task,
        })?;
        Ok(())
    }

    pub fn get_predictions(&self, input: &str) -> Vec<CommandPrediction> {
        let cache = self.prediction_cache.read();
        
        // Check for exact matches first
        if let Some(predictions) = cache.get(input) {
            return predictions.clone();
        }
        
        // Check for prefix matches
        for (key, predictions) in cache.iter() {
            if input.starts_with(key) {
                return predictions.clone();
            }
        }
        
        Vec::new()
    }
}
