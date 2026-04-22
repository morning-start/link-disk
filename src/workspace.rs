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
        let workspace_path_str = path.to_string_lossy().replace("\\", "/");
        let default_config = format!(r#"# link-disk 配置文件

[workspace]
path = "{}"

[apps.vscode]
name = "VSCode"
on_exists = "skip"

[[apps.vscode.sources]]
source = "<home>/AppData/Roaming/Code"
target = "vscode/Roaming"
link_type = "symlink"

[[apps.vscode.sources]]
source = "<home>/.vscode"
target = "vscode/config"
link_type = "symlink"

[apps.chrome]
name = "Chrome"
on_exists = "skip"

[[apps.chrome.sources]]
source = "<home>/AppData/Local/Google/Chrome"
target = "chrome/Local"
link_type = "symlink"
"#, workspace_path_str);

        if !config_file.exists() {
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
