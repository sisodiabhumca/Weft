# Weft Terminal Architecture

## Overview

Weft is an AI-assisted terminal environment built in Rust, designed to provide an enhanced shell experience with PTY support, plugin system, command suggestions, and health checks.

## Core Components

### 1. Main Entry Point (`main.rs`)

The CLI entry point that handles command parsing and dispatch:
- `weft` / `weft run` - Interactive shell (PTY by default)
- `weft config` - Configuration management (show, get, set, validate, path)
- `weft plugin` - Plugin management (install, list, enable/disable, remove)
- `weft suggest <query>` - Command suggestions (static + Ollama)
- `weft doctor` - Health checks (shell, config, plugins, AI)

### 2. Configuration System (`config_simple.rs`)

Centralized configuration management using TOML:
- **TerminalConfig**: Shell, font settings, PTY configuration, cursor style
- **AIConfig**: AI provider settings, model selection, context window, prediction threshold
- **PluginConfig**: Plugin directories, startup hooks, enabled state

Configuration is stored at `~/.config/weft/config.toml` by default.

### 3. Terminal Engine (`terminal.rs`)

Core terminal emulation with session management:
- **TerminalEngine**: Manages multiple terminal sessions
- **TerminalSession**: Tracks working directory, environment, command history
- **CommandHistory**: Records commands with timestamps, exit codes, duration
- **CommandValidation**: Security validation for shell commands

### 4. PTY Backend (`pty.rs`)

Pseudo-terminal implementation for proper TUI support:
- Uses `portable-pty` crate for cross-platform PTY support
- Handles terminal size, I/O relay, and graceful shutdown
- Supports Ctrl+C signal handling

### 5. Plugin System (`plugin_store.rs`)

Filesystem-based plugin registry with security validation:
- **PluginPaths**: Manages plugin directories and state file
- **SecurityValidation**: Validates plugins before installation (blocks native libraries, checks executables)
- **Startup Hooks**: Executes plugin hooks on startup
- Plugin state persisted in `~/.local/share/weft/plugin-state.toml`

### 6. AI Integration (`ai.rs`)

AI-powered command prediction and automation (feature-gated):
- **AIEngine**: Manages AI models and prediction cache
- **CommandPrediction**: Suggested commands with confidence scores
- **AutomationSuggestion**: Automated task suggestions
- Event-driven architecture for real-time predictions

### 7. Command Suggestions (`suggest.rs`)

Hybrid suggestion system with rate limiting:
- **Static Suggestions**: Pre-defined command patterns for git, docker, cargo
- **Ollama Integration**: AI-powered suggestions via local LLM
- **Rate Limiting**: 10 requests per minute using governor crate

### 8. Health Checks (`doctor.rs`)

System health verification:
- Config validation
- Shell executable check
- Plugin directory verification
- Plugin state inspection
- Ollama connectivity test

### 9. Feature-Gated Modules

The following modules are behind feature flags and represent future functionality:

- **rendering.rs** (gpu-acceleration): GPU-accelerated rendering with wgpu
- **collaboration.rs** (collaboration): Real-time session sharing via WebSocket
- **debugging.rs**: Performance monitoring and command tracing
- **performance.rs**: Performance metrics and profiling
- **plugins.rs** (plugins): Native plugin ABI with FFI (currently using shell hooks)

## Data Flow

### Startup Flow

1. Parse CLI arguments
2. Load configuration from `~/.config/weft/config.toml`
3. Initialize logging based on debug flag
4. Run startup hooks if plugins enabled
5. Launch shell with or without PTY

### Command Execution Flow

1. User inputs command
2. Command validation checks for dangerous patterns
3. Execute command in shell
4. Record in session history with timestamp
5. Update last activity timestamp
6. Send events to AI engine for learning

### Plugin Installation Flow

1. Validate plugin security (check for native libraries, executables)
2. Copy plugin to `~/.local/share/weft/plugins/<name>`
3. Read plugin.toml manifest
4. Update plugin state file
5. Run startup hooks if enabled

## Security Model

### Command Validation
- Blocks dangerous patterns (rm -rf /, fork bombs, disk formatting)
- Warns about system-modifying commands (sudo, chmod, chown)
- Detects command chaining (&&, ||, ;)

### Plugin Security
- Blocks native libraries (.so, .dll, .dylib)
- Validates executable file locations
- Checks manifest for suspicious URLs
- Sandboxed execution via shell hooks only

### Rate Limiting
- AI suggestions limited to 10 requests per minute
- Prevents abuse of Ollama API

## Feature Flags

- `default`: Core functionality only
- `gpu-acceleration`: GPU rendering with wgpu/winit
- `ai-features`: AI engine integration
- `collaboration`: WebSocket-based collaboration
- `plugins`: Native plugin ABI
- `full`: All features enabled

## Testing Strategy

- Unit tests for config validation, suggestions, PTY shutdown
- Integration tests for plugin installation/removal
- Security validation tests for commands and plugins
- CI includes format check, clippy, tests, security audit, coverage

## Dependencies

### Core
- `tokio`: Async runtime
- `serde`/`serde_json`/`toml`: Serialization
- `anyhow`: Error handling
- `tracing`: Structured logging
- `clap`: CLI parsing
- `dirs`: Cross-platform directories
- `portable-pty`: PTY support
- `reqwest`: HTTP client (rustls-tls)

### Additional
- `chrono`: Timestamps
- `uuid`: Session IDs
- `parking_lot`: Fast RwLock
- `governor` + `lazy_static`: Rate limiting

### Optional (feature-gated)
- `wgpu`/`winit`: GPU rendering
- `tokio-tungstenite`/`futures-util`: WebSockets
- `libloading`: Native plugin loading

## Future Enhancements

1. Native GPU terminal UI (wgpu)
2. Real-time collaboration / session sharing
3. Dynamic native plugin ABI (.so sandbox)
4. Plugin marketplace
5. Enhanced AI integration with context awareness
