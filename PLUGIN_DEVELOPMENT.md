# Plugin Development Guide

## Overview

Weft supports shell-based plugins that extend terminal functionality through startup hooks. Plugins are installed to `~/.local/share/weft/plugins/<name>` and can execute scripts on startup.

## Plugin Structure

A typical plugin directory:

```
my-plugin/
├── plugin.toml          # Plugin manifest
├── hooks/
│   └── on_startup.sh   # Startup hook script
└── README.md           # Optional documentation
```

## Plugin Manifest (plugin.toml)

```toml
name = "my-plugin"
version = "1.0.0"
description = "A sample plugin"
author = "Your Name"

[hooks]
on_startup = "hooks/on_startup.sh"
```

### Manifest Fields

- `name`: Unique plugin identifier (required)
- `version`: Semantic version (required)
- `description`: Short description (optional)
- `author`: Plugin author (optional)
- `hooks.on_startup`: Path to startup script (optional)

## Security Considerations

### What's Allowed

- Shell scripts (.sh, .bash, .zsh)
- Executable files in the `hooks/` directory
- Configuration files (TOML, JSON, YAML)

### What's Blocked

- Native libraries (.so, .dll, .dylib) - blocked for security
- Executable files outside the `hooks/` directory - warning issued
- Non-localhost HTTP URLs in manifest - warning issued

### Validation

Plugins are validated before installation:
1. Scans for native libraries (blocked)
2. Checks executable file locations (warned if outside hooks/)
3. Validates manifest for suspicious patterns (warned)

## Creating a Plugin

### Step 1: Create Directory Structure

```bash
mkdir -p my-plugin/hooks
```

### Step 2: Write the Manifest

Create `plugin.toml`:

```toml
name = "greeting-plugin"
version = "1.0.0"
description = "Displays a greeting on startup"
author = "Your Name"

[hooks]
on_startup = "hooks/on_startup.sh"
```

### Step 3: Write the Startup Hook

Create `hooks/on_startup.sh`:

```bash
#!/bin/bash
echo "Welcome to Weft Terminal!"
echo "Today is $(date)"
```

Make it executable:

```bash
chmod +x hooks/on_startup.sh
```

### Step 4: Install the Plugin

```bash
weft plugin install /path/to/my-plugin
```

## Plugin Lifecycle

### Installation

1. Security validation runs
2. Plugin directory copied to `~/.local/share/weft/plugins/<name>`
3. Manifest parsed and validated
4. Plugin state recorded in `~/.local/share/weft/plugin-state.toml`

### Startup

If `run_hooks_on_startup` is enabled in config:
1. Weft loads plugin state
2. For each enabled plugin with `on_startup` hook:
   - Executes the hook script
   - Captures output for logging
   - Reports errors without blocking other plugins

### Removal

```bash
weft plugin remove my-plugin
```

This removes the plugin directory and updates state.

## Plugin State

Plugin state is persisted in `~/.local/share/weft/plugin-state.toml`:

```toml
[[plugins]]
id = "greeting-plugin"
name = "greeting-plugin"
version = "1.0.0"
enabled = true
installed_at = "2024-01-01T00:00:00Z"
```

## Best Practices

### 1. Keep Hooks Fast

Startup hooks run synchronously. Keep them quick:

```bash
#!/bin/bash
# Good: Simple echo
echo "Plugin loaded"

# Bad: Long-running operation
sleep 10
echo "Done"
```

### 2. Handle Errors Gracefully

```bash
#!/bin/bash
set -e  # Exit on error

# Check for required tools
if ! command -v git &> /dev/null; then
    echo "git not found, skipping" >&2
    exit 0
fi

# Your plugin logic
git status
```

### 3. Use Environment Variables

Access Weft environment:

```bash
#!/bin/bash
echo "Weft config dir: $WEFT_CONFIG_DIR"
echo "Weft plugins dir: $WEFT_PLUGINS_DIR"
```

### 4. Provide Useful Output

```bash
#!/bin/bash
echo "[my-plugin] Initializing..."
# ... work ...
echo "[my-plugin] Done"
```

### 5. Document Dependencies

In your README.md:

```markdown
# My Plugin

## Dependencies
- git
- jq

## Installation
```bash
weft plugin install /path/to/my-plugin
```
```

## Example Plugins

### Environment Setup Plugin

```bash
# hooks/on_startup.sh
#!/bin/bash
export MY_PROJECT_ROOT="$HOME/projects"
export EDITOR="vim"
echo "[env-plugin] Environment variables set"
```

### Git Status Plugin

```bash
# hooks/on_startup.sh
#!/bin/bash
if git rev-parse --git-dir > /dev/null 2>&1; then
    echo "[git-plugin] Current branch: $(git branch --show-current)"
fi
```

### Notification Plugin

```bash
# hooks/on_startup.sh
#!/bin/bash
if command -v notify-send &> /dev/null; then
    notify-send "Weft Terminal" "Session started"
fi
```

## Debugging Plugins

### Enable Debug Logging

```bash
WEFT_LOG=debug weft run
```

### Check Plugin State

```bash
cat ~/.local/share/weft/plugin-state.toml
```

### Test Hook Manually

```bash
~/.local/share/weft/plugins/my-plugin/hooks/on_startup.sh
```

### View Plugin Logs

Startup hook output is logged with tracing. Check the logs for errors.

## Troubleshooting

### Plugin Not Running

1. Check if plugin is enabled:
   ```bash
   weft plugin list
   ```

2. Check if hooks are enabled in config:
   ```toml
   [plugins]
   run_hooks_on_startup = true
   ```

3. Check hook script permissions:
   ```bash
   ls -l ~/.local/share/weft/plugins/my-plugin/hooks/
   ```

### Hook Failing Silently

1. Run hook manually to see errors
2. Check Weft logs with debug enabled
3. Ensure hook has proper shebang (`#!/bin/bash`)

### Security Validation Failed

1. Remove native libraries (.so, .dll, .dylib)
2. Move executables into `hooks/` directory
3. Remove non-localhost HTTP URLs from manifest

## Future Plugin Support

Future versions of Weft will support:

- Native plugin ABI with sandboxed .so/.dll loading
- Plugin marketplace for distribution
- Hot-reloading of plugins
- Plugin permissions system
- Inter-plugin communication

## Contributing

Share your plugins with the community:

1. Create a GitHub repository
2. Include a clear README
3. Add installation instructions
4. Document dependencies and requirements
5. Test on multiple platforms (Linux, macOS)

## Support

For plugin development questions:
- Check the ARCHITECTURE.md for system details
- Review existing plugins for examples
- Open an issue on the Weft repository
