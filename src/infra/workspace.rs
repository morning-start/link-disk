//! 工作区管理模块
//!
//! 负责工作区的初始化配置文件管理，包括：
//! - 工作区目录的创建
//! - 配置文件的生成和管理
//! - 目标路径的解析
//!
//! 注意：路径展开（`~` 前缀、占位符）功能见 [`crate::infra::path_resolver::PathResolver`]。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// 工作区管理工具类
pub struct Workspace;

impl Workspace {
    /// 默认配置模板（从外部文件加载）
    const DEFAULT_CONFIG_TEMPLATE: &'static str = include_str!("../../config-default.toml");

    /// 初始化工作区：创建工作区目录和默认配置文件
    ///
    /// # 参数
    /// - `path`: 工作区根目录路径
    /// - `force`: 为 true 时用模板覆盖已存在的配置文件
    ///
    /// # 返回值
    /// 返回工作区路径
    pub fn init(path: &Path, force: bool) -> Result<PathBuf> {
        Self::init_with_template(path, force, Self::DEFAULT_CONFIG_TEMPLATE)
    }

    /// 使用指定模板初始化工作区（`init` 的通用实现）
    ///
    /// # 参数
    /// - `path`: 工作区根目录路径
    /// - `force`: 为 true 时覆盖已存在的配置文件
    /// - `template`: 配置模板（`{}` 为工作区路径占位符）
    pub fn init_with_template(path: &Path, force: bool, template: &str) -> Result<PathBuf> {
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

        if force || !config_file.exists() {
            let workspace_path_str = path.to_string_lossy().replace("\\", "/");
            let config_content = template.replace("{}", &workspace_path_str);
            std::fs::write(&config_file, &config_content)
                .with_context(|| format!("Failed to create config file: {:?}", config_file))?;
            set_config_file_permissions(&config_file);
        }

        Ok(path.to_path_buf())
    }

    /// 获取配置文件所在目录（~/.link-disk）
    ///
    /// # Errors
    /// 当无法获取用户主目录时返回错误（例如 `HOME` 环境变量未设置）。
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to get home directory")?;

        Ok(home.join(".link-disk"))
    }

    /// 获取配置文件的完整路径（~/.link-disk/config.toml）
    ///
    /// # Errors
    /// 当无法获取用户主目录时返回错误（见 [`Self::config_dir`]）。
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// 解析目标路径：将相对路径与工作区路径拼接为绝对路径
    ///
    /// 不手动替换分隔符：`Path::join` 在各平台都能正确处理 `/` 和 `\`，
    /// 输出时使用平台原生分隔符。手动替换会破坏 Unix 兼容性。
    #[must_use]
    pub fn resolve_target(workspace: &Path, relative: &str) -> PathBuf {
        workspace.join(relative)
    }
}

/// 设置配置文件权限为仅当前用户可读写（Unix: 0o600）
///
/// 非 Unix 平台（如 Windows）当前不调整权限位。
fn set_config_file_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o600);
            let _ = std::fs::set_permissions(path, perms);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}
