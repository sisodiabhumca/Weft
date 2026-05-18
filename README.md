# Weft Terminal

AI-assisted shell environment built in Rust. Weft wraps your configured shell with optional PTY support, plugin startup hooks, command suggestions, and health checks.

## Current features (v0.2)

| Command | Description |
|---------|-------------|
| `weft` / `weft run` | Interactive shell (PTY by default) |
| `weft config` | Show, get, set, validate configuration |
| `weft plugin` | Install, list, enable/disable, remove plugins |
| `weft suggest <query>` | Command suggestions (static + Ollama) |
| `weft doctor` | Health checks (shell, config, plugins, AI) |

## Installation

```bash
git clone https://github.com/sisodiabhumca/Weft.git
cd Weft
cargo build --release
./target/release/weft
```

## Quick start

```bash
# Run interactive shell
weft

# Check environment
weft doctor

# Get command suggestions
weft suggest "git status"
weft suggest docker

# Configuration
weft config show
weft config set terminal.use_pty true
weft config set ai.enabled true
weft config set ai.model llama3.2
```

## Configuration

Default path: `~/.config/weft/config.toml`

```toml
[terminal]
shell = "/bin/zsh"
font_family = "JetBrains Mono"
font_size = 14.0
cursor_blink = true
use_pty = true

[ai]
enabled = true
provider = "ollama"
model = "llama3.2"
auto_suggestions = true
endpoint = "http://127.0.0.1:11434"

[plugins]
enabled = true
run_hooks_on_startup = true
# plugins_dir = "/custom/path/plugins"  # optional
```

### Config keys (CLI)

```bash
weft config get terminal.shell
weft config set plugins.enabled false
weft config validate
weft config path
```

## Plugins

Plugins are directories copied into the data directory (`~/.local/share/weft/plugins` by default).

```bash
weft plugin install ./my-plugin
weft plugin list
weft plugin disable my-plugin
weft plugin enable my-plugin
weft plugin remove my-plugin
```

### Plugin layout

```
my-plugin/
  plugin.toml
  hooks/
    on_startup.sh   # runs when `weft run` starts (if enabled)
```

Example `plugin.toml`:

```toml
name = "my-plugin"

[hooks]
on_startup = "hooks/on_startup.sh"
```

## Roadmap (not yet implemented)

- Native GPU terminal UI (wgpu)
- Real-time collaboration / session sharing
- Dynamic native plugin ABI (`.so` sandbox)
- Plugin marketplace

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```

## License

Proprietary — see [LICENSE](LICENSE).
