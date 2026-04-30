//! Extensible plugin system for Weft Terminal
//! 
//! This module provides a secure and flexible plugin architecture
//! that allows users to extend terminal functionality.

use anyhow::Result;
use libloading::{Library, Symbol};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

pub struct PluginManager {
    config: Arc<crate::config::Config>,
    plugins: Arc<RwLock<HashMap<String, LoadedPlugin>>>,
    event_tx: mpsc::UnboundedSender<PluginEvent>,
    event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<PluginEvent>>>>,
}

#[derive(Debug, Clone)]
pub enum PluginEvent {
    PluginLoaded { name: String },
    PluginUnloaded { name: String },
    PluginError { name: String, error: String },
    HookExecuted { hook: String, result: HookResult },
}

#[derive(Debug)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub library: Library,
    pub api_vtable: PluginAPIVTable,
    pub enabled: bool,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub weft_version: String,
    pub permissions: Vec<PluginPermission>,
    pub hooks: Vec<PluginHook>,
    pub dependencies: Vec<String>,
    pub entry_point: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginPermission {
    FileSystem { read: bool, write: bool },
    Network { domains: Vec<String> },
    System { commands: Vec<String> },
    Terminal { sessions: bool, input: bool, output: bool },
    AI { predictions: bool, training: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginHook {
    PreCommand,
    PostCommand,
    OnStartup,
    OnShutdown,
    OnInput,
    OnOutput,
    OnSessionCreate,
    OnSessionClose,
}

#[derive(Debug, Clone)]
pub struct HookResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[repr(C)]
pub struct PluginAPIVTable {
    pub initialize: extern "C" fn() -> u32,
    pub shutdown: extern "C" fn() -> u32,
    pub get_info: extern "C" fn() -> *const std::os::raw::c_char,
    pub execute_hook: extern "C" fn(hook: u32, data: *const u8, len: usize) -> *mut HookResult,
    pub handle_event: extern "C" fn(event_type: u32, data: *const u8, len: usize) -> u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub manifest: PluginManifest,
    pub loaded: bool,
    pub enabled: bool,
    pub error: Option<String>,
    pub stats: PluginStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStats {
    pub hooks_executed: u64,
    pub execution_time_ms: u64,
    pub memory_usage_bytes: u64,
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
}

impl PluginManager {
    pub fn new(config: &Arc<crate::config::Config>) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            config: config.clone(),
            plugins: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
        })
    }

    pub async fn load_plugins(&self) -> Result<()> {
        info!("Loading plugins");
        
        if !self.config.plugins.enabled {
            info!("Plugin system is disabled");
            return Ok(());
        }

        // Scan plugin directories
        for plugin_dir in &self.config.plugins.plugin_directories {
            if plugin_dir.exists() {
                self.scan_plugin_directory(plugin_dir).await?;
            } else {
                warn!("Plugin directory does not exist: {}", plugin_dir.display());
            }
        }

        // Auto-load trusted plugins if enabled
        if self.config.plugins.auto_load {
            self.auto_load_trusted_plugins().await?;
        }

        info!("Plugin loading completed");
        Ok(())
    }

    async fn scan_plugin_directory(&self, plugin_dir: &PathBuf) -> Result<()> {
        debug!("Scanning plugin directory: {}", plugin_dir.display());
        
        let entries = std::fs::read_dir(plugin_dir)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Check for plugin manifest
                let manifest_path = path.join("plugin.toml");
                if manifest_path.exists() {
                    if let Err(e) = self.discover_plugin(&path).await {
                        warn!("Failed to discover plugin at {}: {}", path.display(), e);
                    }
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("weft") {
                // Direct plugin file
                if let Err(e) = self.discover_plugin(&path.parent().unwrap_or(&path)).await {
                    warn!("Failed to discover plugin at {}: {}", path.display(), e);
                }
            }
        }
        
        Ok(())
    }

    async fn discover_plugin(&self, plugin_path: &PathBuf) -> Result<()> {
        debug!("Discovering plugin at: {}", plugin_path.display());
        
        let manifest_path = plugin_path.join("plugin.toml");
        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&manifest_content)?;
        
        // Check compatibility
        if !self.is_plugin_compatible(&manifest) {
            warn!("Plugin {} is not compatible with current Weft version", manifest.name);
            return Ok(());
        }

        // Check permissions
        if !self.check_plugin_permissions(&manifest)? {
            warn!("Plugin {} requires excessive permissions", manifest.name);
            return Ok(());
        }

        // Check dependencies
        if !self.check_plugin_dependencies(&manifest)? {
            warn!("Plugin {} has unmet dependencies", manifest.name);
            return Ok(());
        }

        info!("Discovered plugin: {} v{}", manifest.name, manifest.version);
        
        // Load if auto-load is enabled and plugin is trusted
        if self.config.plugins.auto_load && 
           self.config.plugins.trusted_plugins.contains(&manifest.name) {
            self.load_plugin_internal(&manifest.name, plugin_path).await?;
        }
        
        Ok(())
    }

    async fn auto_load_trusted_plugins(&self) -> Result<()> {
        info!("Auto-loading trusted plugins");
        
        for plugin_name in &self.config.plugins.trusted_plugins.clone() {
            if let Some(plugin_path) = self.config.get_plugin_path(plugin_name) {
                if let Err(e) = self.load_plugin_internal(plugin_name, &plugin_path).await {
                    warn!("Failed to auto-load plugin {}: {}", plugin_name, e);
                }
            }
        }
        
        Ok(())
    }

    async fn load_plugin_internal(&self, plugin_name: &str, plugin_path: &PathBuf) -> Result<()> {
        debug!("Loading plugin: {}", plugin_name);
        
        // Load plugin library
        let library_path = plugin_path.join("libplugin.so");
        let library = unsafe { Library::new(&library_path) }
            .map_err(|e| anyhow::anyhow!("Failed to load plugin library: {}", e))?;

        // Get API vtable
        let api_vtable = unsafe {
            let get_api: Symbol<extern "C" fn() -> *const PluginAPIVTable> = 
                library.get(b"get_plugin_api")?;
            (*get_api()).clone()
        };

        // Initialize plugin
        let init_result = unsafe { (api_vtable.initialize)() };
        if init_result != 0 {
            return Err(anyhow::anyhow!("Plugin initialization failed with code: {}", init_result));
        }

        // Load manifest
        let manifest_path = plugin_path.join("plugin.toml");
        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&manifest_content)?;

        let loaded_plugin = LoadedPlugin {
            manifest: manifest.clone(),
            library,
            api_vtable,
            enabled: true,
            last_activity: chrono::Utc::now(),
        };

        {
            let mut plugins = self.plugins.write();
            plugins.insert(plugin_name.to_string(), loaded_plugin);
        }

        // Send event
        if let Err(e) = self.event_tx.send(PluginEvent::PluginLoaded {
            name: plugin_name.to_string(),
        }) {
            warn!("Failed to send plugin loaded event: {}", e);
        }

        info!("Successfully loaded plugin: {} v{}", manifest.name, manifest.version);
        Ok(())
    }

    pub async fn process_events(&self) -> Result<()> {
        let mut event_rx = self.event_rx.write().take().unwrap();
        
        while let Some(event) = event_rx.recv().await {
            debug!("Processing plugin event: {:?}", event);
            
            match event {
                PluginEvent::PluginLoaded { name } => {
                    debug!("Plugin loaded: {}", name);
                }
                PluginEvent::PluginUnloaded { name } => {
                    debug!("Plugin unloaded: {}", name);
                }
                PluginEvent::PluginError { name, error } => {
                    warn!("Plugin error in {}: {}", name, error);
                }
                PluginEvent::HookExecuted { hook, result } => {
                    debug!("Hook {} executed: {:?}", hook, result);
                }
            }
        }

        // Put the receiver back
        *self.event_rx.write() = Some(event_rx);
        Ok(())
    }

    pub async fn execute_hook(&self, hook_name: &str, data: Option<serde_json::Value>) -> Vec<HookResult> {
        let mut results = Vec::new();
        let plugins = self.plugins.read();
        
        for (name, plugin) in plugins.iter() {
            if !plugin.enabled {
                continue;
            }

            // Check if plugin handles this hook
            if plugin.manifest.hooks.iter().any(|h| self.hook_to_string(h) == hook_name) {
                let hook_id = self.string_to_hook_id(hook_name);
                
                let data_bytes = if let Some(d) = data {
                    serde_json::to_vec(&d).unwrap_or_default()
                } else {
                    Vec::new()
                };

                let result_ptr = unsafe {
                    (plugin.api_vtable.execute_hook)(
                        hook_id,
                        data_bytes.as_ptr(),
                        data_bytes.len(),
                    )
                };

                let result = unsafe { *result_ptr };
                results.push(result);
                
                // Clean up the result
                unsafe {
                    libc::free(result_ptr as *mut libc::c_void);
                }
                
                // Update stats
                plugin.last_activity = chrono::Utc::now();
            }
        }
        
        results
    }

    pub fn unload_plugin(&self, plugin_name: &str) -> Result<()> {
        debug!("Unloading plugin: {}", plugin_name);
        
        let mut plugins = self.plugins.write();
        if let Some(plugin) = plugins.remove(plugin_name) {
            // Shutdown plugin
            let shutdown_result = unsafe { (plugin.api_vtable.shutdown)() };
            if shutdown_result != 0 {
                warn!("Plugin shutdown returned error code: {}", shutdown_result);
            }
            
            // Library will be dropped when plugin goes out of scope
            
            if let Err(e) = self.event_tx.send(PluginEvent::PluginUnloaded {
                name: plugin_name.to_string(),
            }) {
                warn!("Failed to send plugin unloaded event: {}", e);
            }
            
            info!("Successfully unloaded plugin: {}", plugin_name);
        } else {
            warn!("Plugin not found: {}", plugin_name);
        }
        
        Ok(())
    }

    pub fn enable_plugin(&self, plugin_name: &str) -> Result<()> {
        let mut plugins = self.plugins.write();
        if let Some(plugin) = plugins.get_mut(plugin_name) {
            plugin.enabled = true;
            info!("Plugin enabled: {}", plugin_name);
        } else {
            return Err(anyhow::anyhow!("Plugin not found: {}", plugin_name));
        }
        Ok(())
    }

    pub fn disable_plugin(&self, plugin_name: &str) -> Result<()> {
        let mut plugins = self.plugins.write();
        if let Some(plugin) = plugins.get_mut(plugin_name) {
            plugin.enabled = false;
            info!("Plugin disabled: {}", plugin_name);
        } else {
            return Err(anyhow::anyhow!("Plugin not found: {}", plugin_name));
        }
        Ok(())
    }

    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read();
        let mut metadata = Vec::new();
        
        for (name, plugin) in plugins.iter() {
            metadata.push(PluginMetadata {
                manifest: plugin.manifest.clone(),
                loaded: true,
                enabled: plugin.enabled,
                error: None,
                stats: PluginStats {
                    hooks_executed: 0,
                    execution_time_ms: 0,
                    memory_usage_bytes: 0,
                    last_execution: Some(plugin.last_activity),
                },
            });
        }
        
        metadata
    }

    fn is_plugin_compatible(&self, manifest: &PluginManifest) -> bool {
        // Simple version compatibility check
        // In a real implementation, this would be more sophisticated
        manifest.weft_version == "0.1.0" || manifest.weft_version == "*"
    }

    fn check_plugin_permissions(&self, manifest: &PluginManifest) -> Result<bool> {
        match self.config.plugins.security_policy {
            crate::config::SecurityPolicy::AllowAll => Ok(true),
            crate::config::SecurityPolicy::AllowTrusted => {
                Ok(self.config.plugins.trusted_plugins.contains(&manifest.name))
            }
            crate::config::SecurityPolicy::Prompt => {
                // In a real implementation, this would prompt the user
                Ok(false)
            }
            crate::config::SecurityPolicy::DenyAll => Ok(false),
        }
    }

    fn check_plugin_dependencies(&self, manifest: &PluginManifest) -> Result<bool> {
        for dep in &manifest.dependencies {
            // Check if dependency is available
            if !self.plugins.read().contains_key(dep) {
                warn!("Missing dependency: {}", dep);
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn hook_to_string(&self, hook: &PluginHook) -> String {
        match hook {
            PluginHook::PreCommand => "pre_command".to_string(),
            PluginHook::PostCommand => "post_command".to_string(),
            PluginHook::OnStartup => "on_startup".to_string(),
            PluginHook::OnShutdown => "on_shutdown".to_string(),
            PluginHook::OnInput => "on_input".to_string(),
            PluginHook::OnOutput => "on_output".to_string(),
            PluginHook::OnSessionCreate => "on_session_create".to_string(),
            PluginHook::OnSessionClose => "on_session_close".to_string(),
        }
    }

    fn string_to_hook_id(&self, hook_name: &str) -> u32 {
        match hook_name {
            "pre_command" => 0,
            "post_command" => 1,
            "on_startup" => 2,
            "on_shutdown" => 3,
            "on_input" => 4,
            "on_output" => 5,
            "on_session_create" => 6,
            "on_session_close" => 7,
            _ => 999,
        }
    }
}

// Plugin API for plugin developers
#[repr(C)]
pub struct WeftAPI {
    pub terminal_input: extern "C" fn(session_id: *const std::os::raw::c_char, data: *const u8, len: usize) -> u32,
    pub terminal_output: extern "C" fn(session_id: *const std::os::raw::c_char, data: *const u8, len: usize) -> u32,
    pub get_config: extern "C" fn(key: *const std::os::raw::c_char) -> *const std::os::raw::c_char,
    pub set_config: extern "C" fn(key: *const std::os::raw::c_char, value: *const std::os::raw::c_char) -> u32,
    pub log_message: extern "C" fn(level: u32, message: *const std::os::raw::c_char),
    pub register_command: extern "C" fn(name: *const std::os::raw::c_char, handler: extern "C" fn(*const std::os::raw::c_char) -> u32) -> u32,
}
