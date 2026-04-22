use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub workspace: Workspace,
    #[serde(default)]
    pub apps: std::collections::HashMap<String, AppConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub on_exists: Option<String>,
    #[serde(default)]
    pub sources: Vec<Source>,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    pub source: String,
    pub target: String,
    #[serde(default = "default_link_type")]
    pub link_type: String,
    #[serde(default = "default_source_type")]
    pub _source_type: String,
}

fn default_link_type() -> String {
    "symlink".to_string()
}

fn default_source_type() -> String {
    "dir".to_string()
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        let config: Config = toml::from_str(&content).context("Failed to parse config file")?;

        Ok(config)
    }

    pub fn get_app(&self, name: &str) -> Option<&AppConfig> {
        self.apps.get(name)
    }

    pub fn enabled_apps(&self) -> Vec<(&String, &AppConfig)> {
        self.apps.iter().filter(|(_, app)| app.enabled).collect()
    }
}

impl AppConfig {
    pub fn on_exists_strategy(&self) -> &str {
        self.on_exists.as_deref().unwrap_or("skip")
    }
}
