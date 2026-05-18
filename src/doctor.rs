//! Health checks for shell, config, plugins, and AI backend.

use crate::config_simple::Config;
use crate::plugin_store::{self, PluginPaths};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
}

pub async fn run_doctor(config: &Config) -> Vec<CheckResult> {
    let mut results = Vec::new();

    results.push(check_config_valid(config));
    results.push(check_shell(&config.terminal.shell));

    let paths = PluginPaths::from_config(config);
    results.push(check_plugins_dir(&paths.plugins_dir));
    results.push(check_plugin_state(&paths));

    if config.ai.enabled {
        results.push(check_ollama(&config.ai.endpoint).await);
    } else {
        results.push(CheckResult {
            name: "ai.ollama".to_string(),
            status: CheckStatus::Pass,
            message: "AI disabled in config (skipped)".to_string(),
        });
    }

    results
}

fn check_config_valid(config: &Config) -> CheckResult {
    match config.validate() {
        Ok(()) => CheckResult {
            name: "config.validate".to_string(),
            status: CheckStatus::Pass,
            message: format!("Config OK ({})", Config::config_file_path().display()),
        },
        Err(e) => CheckResult {
            name: "config.validate".to_string(),
            status: CheckStatus::Fail,
            message: e.to_string(),
        },
    }
}

fn check_shell(shell: &str) -> CheckResult {
    let path = Path::new(shell);
    if path.is_file() && is_executable(path) {
        CheckResult {
            name: "terminal.shell".to_string(),
            status: CheckStatus::Pass,
            message: format!("Shell executable: {}", shell),
        }
    } else if which_shell(shell) {
        CheckResult {
            name: "terminal.shell".to_string(),
            status: CheckStatus::Pass,
            message: format!("Shell found on PATH: {}", shell),
        }
    } else {
        CheckResult {
            name: "terminal.shell".to_string(),
            status: CheckStatus::Fail,
            message: format!("Shell not found or not executable: {}", shell),
        }
    }
}

fn which_shell(shell: &str) -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {}", shell))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn check_plugins_dir(dir: &Path) -> CheckResult {
    match std::fs::create_dir_all(dir) {
        Ok(()) => CheckResult {
            name: "plugins.dir".to_string(),
            status: CheckStatus::Pass,
            message: format!("Plugins directory writable: {}", dir.display()),
        },
        Err(e) => CheckResult {
            name: "plugins.dir".to_string(),
            status: CheckStatus::Fail,
            message: format!("Cannot create plugins dir {}: {}", dir.display(), e),
        },
    }
}

fn check_plugin_state(paths: &PluginPaths) -> CheckResult {
    match plugin_store::list_plugins(paths) {
        Ok(plugins) => {
            let enabled = plugins.iter().filter(|p| p.enabled).count();
            CheckResult {
                name: "plugins.installed".to_string(),
                status: CheckStatus::Pass,
                message: format!("{} plugin(s) installed, {} enabled", plugins.len(), enabled),
            }
        }
        Err(e) => CheckResult {
            name: "plugins.installed".to_string(),
            status: CheckStatus::Warn,
            message: format!("Could not list plugins: {}", e),
        },
    }
}

async fn check_ollama(endpoint: &str) -> CheckResult {
    let url = format!("{}/api/tags", endpoint.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return CheckResult {
                name: "ai.ollama".to_string(),
                status: CheckStatus::Fail,
                message: format!("HTTP client error: {}", e),
            };
        }
    };

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => CheckResult {
            name: "ai.ollama".to_string(),
            status: CheckStatus::Pass,
            message: format!("Ollama reachable at {}", endpoint),
        },
        Ok(resp) => CheckResult {
            name: "ai.ollama".to_string(),
            status: CheckStatus::Warn,
            message: format!("Ollama at {} returned {}", endpoint, resp.status()),
        },
        Err(e) => CheckResult {
            name: "ai.ollama".to_string(),
            status: CheckStatus::Warn,
            message: format!("Ollama not reachable at {}: {}", endpoint, e),
        },
    }
}

pub fn print_report(results: &[CheckResult]) -> i32 {
    let mut fails = 0u32;
    let mut warns = 0u32;

    for r in results {
        let icon = match r.status {
            CheckStatus::Pass => "ok",
            CheckStatus::Warn => "warn",
            CheckStatus::Fail => "fail",
        };
        println!("[{icon}] {} — {}", r.name, r.message);
        match r.status {
            CheckStatus::Fail => fails += 1,
            CheckStatus::Warn => warns += 1,
            CheckStatus::Pass => {}
        }
    }

    if fails > 0 {
        println!("\nDoctor: {fails} failure(s), {warns} warning(s)");
        1
    } else if warns > 0 {
        println!("\nDoctor: passed with {warns} warning(s)");
        0
    } else {
        println!("\nDoctor: all checks passed");
        0
    }
}
