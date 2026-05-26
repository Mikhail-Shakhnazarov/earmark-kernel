use std::{env, fs, path::PathBuf};

use serde::Deserialize;

use crate::app::CliError;
use crate::cli::Cli;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CliConfig {
    pub root: Option<PathBuf>,
    pub default_system_id: Option<String>,
    pub json: Option<bool>,
    pub log_level: Option<String>,
    pub actor: Option<String>,
    pub trusted_actors: Option<Vec<String>>,
    pub provider_plugin_dirs: Option<Vec<PathBuf>>,
}

pub fn load_config(cli: &Cli) -> Result<CliConfig, CliError> {
    let config_path = if let Some(path) = &cli.config {
        Some(path.clone())
    } else if let Ok(path) = env::var("EM_CONFIG") {
        Some(PathBuf::from(path))
    } else if let Some(root) = &cli.root {
        let candidate = root.join(".earmark").join("config.toml");
        if candidate.exists() {
            Some(candidate)
        } else {
            None
        }
    } else {
        let fallback = PathBuf::from(".earmark/config.toml");
        if fallback.exists() {
            Some(fallback)
        } else {
            None
        }
    };

    let Some(path) = config_path else {
        return Ok(CliConfig::default());
    };
    let text = fs::read_to_string(path)?;
    Ok(toml::from_str(&text)?)
}

pub fn resolve_root(cli: &Cli, config: &CliConfig) -> PathBuf {
    if let Some(root) = &cli.root {
        return root.clone();
    }
    if let Ok(root) = env::var("EM_ROOT") {
        return PathBuf::from(root);
    }
    if let Some(root) = &config.root {
        return root.clone();
    }
    PathBuf::from(".")
}

pub fn resolve_system_id(cli_value: Option<&str>, config: &CliConfig) -> Option<String> {
    if let Some(value) = cli_value {
        return Some(value.to_string());
    }
    if let Ok(value) = env::var("EM_SYSTEM_ID") {
        if !value.trim().is_empty() {
            return Some(value);
        }
    }
    config.default_system_id.clone()
}

pub fn resolve_json_early(cli: &Cli) -> bool {
    if cli.json {
        return true;
    }
    if let Ok(value) = env::var("EM_JSON") {
        return matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES");
    }
    false
}

pub fn resolve_json(cli: &Cli, config: &CliConfig) -> bool {
    let mut resolved = config.json.unwrap_or(false);
    if let Ok(value) = env::var("EM_JSON") {
        resolved = matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES");
    }
    if cli.json {
        resolved = true;
    }
    resolved
}

pub fn resolve_actor(_cli: &Cli, config: &CliConfig) -> String {
    if let Ok(actor) = env::var("EM_ACTOR") {
        return actor;
    }
    if let Some(actor) = &config.actor {
        return actor.clone();
    }
    "operator".to_string()
}

pub fn resolve_trusted_actors(config: &CliConfig) -> Vec<String> {
    if let Ok(actors) = env::var("EM_TRUSTED_ACTORS") {
        return actors.split(',').map(|s| s.trim().to_string()).collect();
    }
    config.trusted_actors.clone().unwrap_or_default()
}

pub fn resolve_log_level(cli: &Cli, config: &CliConfig) -> Option<String> {
    if let Some(level) = &cli.log_level {
        return Some(level.clone());
    }
    if cli.verbose >= 2 {
        return Some("trace".to_string());
    }
    if cli.verbose == 1 {
        return Some("debug".to_string());
    }
    if let Ok(level) = env::var("EM_LOG_LEVEL") {
        return Some(level);
    }
    config.log_level.clone()
}

pub fn resolve_provider_plugin_dirs(root: &PathBuf, config: &CliConfig) -> Vec<PathBuf> {
    let mut dirs = vec![root.join(".earmark").join("plugins").join("providers")];

    if let Ok(value) = env::var("EM_PROVIDER_PLUGIN_DIRS") {
        for item in value.split(':').map(|item| item.trim()).filter(|item| !item.is_empty()) {
            let candidate = PathBuf::from(item);
            if !dirs.iter().any(|existing| existing == &candidate) {
                dirs.push(candidate);
            }
        }
    }

    if let Some(extra_dirs) = &config.provider_plugin_dirs {
        for candidate in extra_dirs {
            if !dirs.iter().any(|existing| existing == candidate) {
                dirs.push(candidate.clone());
            }
        }
    }

    dirs
}
