use std::path::PathBuf;

use anyhow::{Context, Result};

pub struct Workspace;

impl Workspace {
    pub fn init(path: &PathBuf) -> Result<PathBuf> {
        if !path.exists() {
            std::fs::create_dir_all(path)
                .with_context(|| format!("Failed to create workspace directory: {:?}", path))?;
        }

        let config_dir = Self::config_dir()?;
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir)
                .with_context(|| format!("Failed to create config directory: {:?}", config_dir))?;
        }

        let config_file = config_dir.join("config.toml");
        if !config_file.exists() {
            let default_config = include_str!("../config-example.toml");
            std::fs::write(&config_file, default_config)
                .with_context(|| format!("Failed to create default config file: {:?}", config_file))?;
        }

        Ok(path.clone())
    }

    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .context("Failed to get home directory")?;

        Ok(home.join(".link-disk"))
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn resolve_target(workspace: &PathBuf, relative: &str) -> PathBuf {
        let normalized = relative.replace("/", "\\");
        workspace.join(&normalized)
    }
}
