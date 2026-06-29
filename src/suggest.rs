//! Command suggestions: static rules + optional Ollama.

use crate::config_simple::{AIConfig, Config};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub command: String,
    pub confidence: f32,
    pub source: String,
}

// Simple rate limiter for AI requests (10 requests per minute)
lazy_static::lazy_static! {
    static ref LAST_REQUEST_TIME: Arc<std::sync::Mutex<Option<Instant>>> = Arc::new(std::sync::Mutex::new(None));
}

fn check_rate_limit() -> bool {
    let mut last = LAST_REQUEST_TIME.lock().unwrap();
    if let Some(time) = *last {
        if time.elapsed() < Duration::from_secs(6) {
            return false;
        }
    }
    *last = Some(Instant::now());
    true
}

pub async fn suggest(config: &Config, query: &str) -> Result<Vec<Suggestion>> {
    let mut out = static_suggestions(query);

    if config.ai.enabled
        && config.ai.auto_suggestions
        && config.ai.provider.eq_ignore_ascii_case("ollama")
    {
        // Check rate limit before making AI request
        if !check_rate_limit() {
            tracing::warn!("Rate limit exceeded for AI suggestions");
            return Ok(out);
        }

        if let Ok(ai) = ollama_suggestions(&config.ai, query).await {
            for s in ai {
                if !out.iter().any(|x| x.command == s.command) {
                    out.push(s);
                }
            }
        }
    }

    out.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(10);
    Ok(out)
}

fn static_suggestions(query: &str) -> Vec<Suggestion> {
    static RULES: &[(&str, &[(&str, f32)])] = &[
        (
            "git",
            &[
                ("git status", 0.95),
                ("git add .", 0.9),
                ("git commit -m \"\"", 0.85),
                ("git push", 0.8),
                ("git pull", 0.8),
            ],
        ),
        (
            "docker",
            &[
                ("docker ps", 0.95),
                ("docker compose up -d", 0.9),
                ("docker build -t app .", 0.85),
            ],
        ),
        (
            "cargo",
            &[
                ("cargo build", 0.95),
                ("cargo test", 0.9),
                ("cargo run", 0.85),
                ("cargo clippy", 0.8),
            ],
        ),
    ];

    let q = query.trim().to_lowercase();
    let mut map: HashMap<String, Suggestion> = HashMap::new();

    if q.is_empty() {
        for (cmd, conf) in [("ls -la", 0.7), ("pwd", 0.65), ("git status", 0.6)] {
            map.insert(
                cmd.to_string(),
                Suggestion {
                    command: cmd.to_string(),
                    confidence: conf,
                    source: "static".to_string(),
                },
            );
        }
        return map.into_values().collect();
    }

    let first = q.split_whitespace().next().unwrap_or(&q);
    for (prefix, cmds) in RULES {
        if q.starts_with(prefix) || first == *prefix {
            for (cmd, conf) in *cmds {
                if cmd.starts_with(query.trim()) || query.trim().is_empty() {
                    map.insert(
                        (*cmd).to_string(),
                        Suggestion {
                            command: (*cmd).to_string(),
                            confidence: *conf,
                            source: "static".to_string(),
                        },
                    );
                }
            }
        }
    }

    if map.is_empty() && !q.is_empty() {
        map.insert(
            query.trim().to_string(),
            Suggestion {
                command: query.trim().to_string(),
                confidence: 0.5,
                source: "static".to_string(),
            },
        );
    }

    map.into_values().collect()
}

#[derive(Serialize)]
struct OllamaGenerateRequest<'a> {
    model: &'a str,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

async fn ollama_suggestions(ai: &AIConfig, query: &str) -> Result<Vec<Suggestion>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let prompt = format!(
        "You are a shell assistant. Given this user intent or partial command: \"{query}\" \
         Reply with up to 3 shell commands only, one per line, no numbering or explanation."
    );

    let url = format!("{}/api/generate", ai.endpoint.trim_end_matches('/'));

    let body = OllamaGenerateRequest {
        model: &ai.model,
        prompt,
        stream: false,
    };

    let resp = client.post(&url).json(&body).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("ollama returned {}", resp.status());
    }

    let parsed: OllamaGenerateResponse = resp.json().await?;
    let mut out = Vec::new();
    for line in parsed.response.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cmd = line
            .trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == '-' || c == ')')
            .trim()
            .to_string();
        if !cmd.is_empty() {
            out.push(Suggestion {
                command: cmd,
                confidence: 0.75,
                source: "ollama".to_string(),
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_simple::Config;

    #[test]
    fn test_static_suggestions_git() {
        let suggestions = static_suggestions("git");
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.command.contains("git status")));
    }

    #[test]
    fn test_static_suggestions_docker() {
        let suggestions = static_suggestions("docker");
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.command.contains("docker ps")));
    }

    #[test]
    fn test_static_suggestions_cargo() {
        let suggestions = static_suggestions("cargo");
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.command.contains("cargo build")));
    }

    #[test]
    fn test_static_suggestions_empty() {
        let suggestions = static_suggestions("");
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.command.contains("ls")));
    }

    #[test]
    fn test_static_suggestions_no_match() {
        let suggestions = static_suggestions("xyz123");
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].command, "xyz123");
    }

    #[test]
    fn test_suggestion_source_static() {
        let suggestions = static_suggestions("git");
        assert!(suggestions.iter().all(|s| s.source == "static"));
    }

    #[test]
    fn test_suggestion_confidence_sorted() {
        let mut suggestions = static_suggestions("git");
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        for i in 1..suggestions.len() {
            assert!(suggestions[i-1].confidence >= suggestions[i].confidence);
        }
    }

    #[tokio::test]
    async fn test_suggest_ai_disabled() {
        let mut config = Config::default();
        config.ai.enabled = false;
        let suggestions = suggest(&config, "git").await.unwrap();
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().all(|s| s.source == "static"));
    }

    #[tokio::test]
    async fn test_suggest_auto_suggestions_disabled() {
        let mut config = Config::default();
        config.ai.auto_suggestions = false;
        let suggestions = suggest(&config, "git").await.unwrap();
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().all(|s| s.source == "static"));
    }
}
