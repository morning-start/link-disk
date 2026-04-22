use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub struct Workspace;

impl Workspace {
    pub fn init(path: &Path) -> Result<PathBuf> {
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
        let default_config = format!(
            r#"# link-disk 配置文件
# on_exists: skip | overwrite | fail
#   - skip: 不处理已存在的链接
#   - overwrite: 覆盖已存在的链接
#   - fail: 抛出错误
# link_type: symlink | hardlink
#   - symlink: 创建符号链接
#   - hardlink: 创建硬链接


[workspace]
path = "{}"

[apps.vscode]
name = "VSCode"
on_exists = "skip" # 不处理已存在的链接

[[apps.vscode.sources]]
source = "<home>/AppData/Roaming/Code"
target = "vscode/Roaming"
link_type = "symlink" # 创建符号链接

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
"#,
            workspace_path_str
        );

        if !config_file.exists() {
            std::fs::write(&config_file, default_config).with_context(|| {
                format!("Failed to create default config file: {:?}", config_file)
            })?;
        }

        Ok(std::path::PathBuf::from(path))
    }

    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to get home directory")?;

        Ok(home.join(".link-disk"))
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn expand_path(path: &str) -> PathBuf {
        if path.starts_with("~")
            && let Some(home) = dirs::home_dir()
        {
            return home.join(
                path.trim_start_matches("~")
                    .trim_start_matches('/')
                    .trim_start_matches('\\'),
            );
        }
        PathBuf::from(path)
    }

    pub fn resolve_target(workspace: &Path, relative: &str) -> PathBuf {
        let normalized = relative.replace("/", "\\");
        workspace.join(&normalized)
    }
}
