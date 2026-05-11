//! Filesystem-backed plugin registry (install path → data dir, enable/disable in config).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PluginPaths {
    pub plugins_dir: PathBuf,
    pub state_file: PathBuf,
}

impl PluginPaths {
    pub fn default_xdg() -> Self {
        let base_config = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("weft");
        let plugins_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("weft")
            .join("plugins");
        Self {
            plugins_dir,
            state_file: base_config.join("plugin-state.toml"),
        }
    }

    #[cfg(test)]
    fn for_tests(root: &Path) -> Self {
        Self {
            plugins_dir: root.join("plugins"),
            state_file: root.join("plugin-state.toml"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifestToml {
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PluginEntry {
    pub id: String,
    pub path: PathBuf,
    pub enabled: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct PluginStateFile {
    #[serde(default)]
    disabled: Vec<String>,
}

impl PluginStateFile {
    fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("read plugin state {}", path.display()))?;
        Ok(toml::from_str(&raw)?)
    }

    fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(self)?;
        fs::write(path, raw).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).with_context(|| format!("create_dir_all {}", dst.display()))?;
    for entry in fs::read_dir(src).with_context(|| format!("read_dir {}", src.display()))? {
        let entry = entry?;
        let src_p = entry.path();
        let dst_p = dst.join(entry.file_name());
        if src_p.is_dir() {
            copy_dir_all(&src_p, &dst_p)?;
        } else if src_p.is_file() {
            fs::copy(&src_p, &dst_p).with_context(|| {
                format!(
                    "copy {} -> {}",
                    src_p.display(),
                    dst_p.display()
                )
            })?;
        }
    }
    Ok(())
}

fn plugin_id_from_source(src: &Path) -> Result<String> {
    let manifest = src.join("plugin.toml");
    if manifest.is_file() {
        let raw = fs::read_to_string(&manifest)?;
        let m: PluginManifestToml = toml::from_str(&raw)?;
        if let Some(n) = m.name {
            if !n.trim().is_empty() {
                return Ok(n.trim().to_string());
            }
        }
    }
    src.file_name()
        .and_then(|s| s.to_str())
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("could not derive plugin id from {}", src.display()))
}

/// List installed plugins (one directory per plugin under `plugins_dir`).
pub fn list_plugins(paths: &PluginPaths) -> Result<Vec<PluginEntry>> {
    if !paths.plugins_dir.exists() {
        return Ok(Vec::new());
    }

    let state = PluginStateFile::load(&paths.state_file)?;
    let disabled: HashSet<String> = state.disabled.into_iter().collect();

    let mut out = Vec::new();
    for entry in fs::read_dir(&paths.plugins_dir)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        out.push(PluginEntry {
            enabled: !disabled.contains(&id),
            id,
            path: p,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Copy `src` (directory) into the plugin store under a derived or manifest name.
pub fn install_plugin(paths: &PluginPaths, src: &Path) -> Result<String> {
    if !src.is_dir() {
        anyhow::bail!("install source must be a directory: {}", src.display());
    }

    fs::create_dir_all(&paths.plugins_dir)?;

    let id = plugin_id_from_source(src)?;
    let dest = paths.plugins_dir.join(&id);
    if dest.exists() {
        anyhow::bail!(
            "plugin '{}' already installed at {}",
            id,
            dest.display()
        );
    }

    copy_dir_all(src, &dest)?;
    Ok(id)
}

pub fn remove_plugin(paths: &PluginPaths, id: &str) -> Result<()> {
    let dest = paths.plugins_dir.join(id);
    if !dest.exists() {
        anyhow::bail!("plugin not found: {}", id);
    }
    fs::remove_dir_all(&dest).with_context(|| format!("remove {}", dest.display()))?;

    let mut state = PluginStateFile::load(&paths.state_file)?;
    state.disabled.retain(|n| n != id);
    state.save(&paths.state_file)?;
    Ok(())
}

pub fn set_enabled(paths: &PluginPaths, id: &str, enabled: bool) -> Result<()> {
    let dest = paths.plugins_dir.join(id);
    if !dest.exists() {
        anyhow::bail!("plugin not found: {}", id);
    }

    let mut state = PluginStateFile::load(&paths.state_file)?;
    if enabled {
        state.disabled.retain(|n| n != id);
    } else if !state.disabled.iter().any(|n| n == id) {
        state.disabled.push(id.to_string());
    }
    state.disabled.sort();
    state.save(&paths.state_file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn install_list_remove_roundtrip() -> Result<()> {
        let root = tempdir()?;
        let paths = PluginPaths::for_tests(root.path());

        let src = root.path().join("src-plugin");
        fs::create_dir_all(&src)?;
        fs::write(src.join("plugin.toml"), r#"name = "demo-plugin""#)?;
        fs::write(src.join("README"), b"hi")?;

        let id = install_plugin(&paths, &src)?;
        assert_eq!(id, "demo-plugin");

        let list = list_plugins(&paths)?;
        assert_eq!(list.len(), 1);
        assert!(list[0].enabled);

        set_enabled(&paths, "demo-plugin", false)?;
        let list = list_plugins(&paths)?;
        assert!(!list[0].enabled);

        set_enabled(&paths, "demo-plugin", true)?;
        let list = list_plugins(&paths)?;
        assert!(list[0].enabled);

        remove_plugin(&paths, "demo-plugin")?;
        assert!(list_plugins(&paths)?.is_empty());
        Ok(())
    }
}
