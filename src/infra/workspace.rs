//! 工作区管理模块
//!
//! 负责工作区的初始化和配置文件管理，包括：
//! - 工作区目录的创建
//! - 配置文件的生成和管理
//! - 目标路径的解析
//!
//! 注意：路径展开（~ 前缀）功能已移至 [`crate::path_resolver::PathResolver`]。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// 工作区管理工具类
pub struct Workspace;

impl Workspace {
    /// 默认配置模板（从外部文件加载）
    const DEFAULT_CONFIG_TEMPLATE: &str = include_str!("../../config-default.toml");

    /// 初始化工作区：创建工作区目录和默认配置文件
    ///
    /// # 参数
    /// - `path`: 工作区根目录路径
    ///
    /// # 返回值
    /// 返回工作区路径
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

        if !config_file.exists() {
            let workspace_path_str = path.to_string_lossy().replace("\\", "/");
            let default_config = Self::DEFAULT_CONFIG_TEMPLATE.replace("{}", &workspace_path_str);
            std::fs::write(&config_file, default_config).with_context(|| {
                format!("Failed to create default config file: {:?}", config_file)
            })?;
            set_config_file_permissions(&config_file)?;
        }

        Ok(std::path::PathBuf::from(path))
    }

    /// 使用自定义模板初始化工作区（高级用法）
    ///
    /// # 参数
    /// - `path`: 工作区根目录路径
    /// - `template`: 自定义配置模板（使用 `{}` 作为工作区路径占位符）
    #[allow(dead_code)]
    pub fn init_with_template(path: &Path, template: &str) -> Result<PathBuf> {
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
            let workspace_path_str = path.to_string_lossy().replace("\\", "/");
            let config_content = template.replace("{}", &workspace_path_str);
            std::fs::write(&config_file, config_content).with_context(|| {
                format!(
                    "Failed to create config file with custom template: {:?}",
                    config_file
                )
            })?;
            set_config_file_permissions(&config_file)?;
        }

        Ok(std::path::PathBuf::from(path))
    }

    /// 获取配置文件所在目录（~/.link-disk）
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to get home directory")?;

        Ok(home.join(".link-disk"))
    }

    /// 获取配置文件的完整路径（~/.link-disk/config.toml）
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// 解析目标路径：将相对路径与工作区路径拼接为绝对路径
    ///
    /// 不手动替换分隔符：`Path::join` 在各平台都能正确处理 `/` 和 `\`，
    /// 输出时使用平台原生分隔符。手动替换会破坏 Unix 兼容性。
    pub fn resolve_target(workspace: &Path, relative: &str) -> PathBuf {
        workspace.join(relative)
    }
}

/// 设置配置文件权限为仅当前用户可读写（Unix: 0o600）
fn set_config_file_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to read metadata: {:?}", path))?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms)
            .with_context(|| format!("Failed to set permissions: {:?}", path))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}
