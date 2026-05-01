//! 配置文件解析模块
//!
//! 负责加载和解析 TOML 格式的配置文件，包括：
//! - 工作区路径配置
//! - 应用配置（名称、启用状态、链接策略）
//! - 源文件/目录配置（源路径、目标路径、链接类型）

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

/// 顶层配置结构体
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// 工作区配置
    pub workspace: Workspace,
    /// 应用配置映射表 (key: 应用ID, value: 应用配置)
    #[serde(default)]
    pub apps: std::collections::HashMap<String, AppConfig>,
}

/// 工作区配置
#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    /// 工作区根目录路径
    pub path: PathBuf,
}

/// 单个应用的配置
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    /// 显示名称
    pub name: String,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 目标已存在时的处理策略
    #[serde(default)]
    pub on_exists: Option<String>,
    /// 源文件/目录列表
    #[serde(default)]
    pub sources: Vec<Source>,
}

/// 默认启用状态为 true
fn default_enabled() -> bool {
    true
}

/// 源文件/目录配置
#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    /// 源路径（支持占位符如 <home>、<appdata> 等）
    pub source: String,
    /// 相对于工作区的目标路径
    pub target: String,
    /// 链接类型：symlink 或 hardlink
    #[serde(default = "default_link_type")]
    pub link_type: String,
    /// 源类型：dir 或 file（内部使用）
    #[serde(default = "default_source_type")]
    pub _source_type: String,
}

/// 默认链接类型为符号链接
fn default_link_type() -> String {
    "symlink".to_string()
}

/// 默认源类型为目录
fn default_source_type() -> String {
    "dir".to_string()
}

impl Config {
    /// 从指定路径加载配置文件
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        let config: Config = toml::from_str(&content).context("Failed to parse config file")?;

        Ok(config)
    }

    /// 获取指定应用ID的配置
    pub fn get_app(&self, name: &str) -> Option<&AppConfig> {
        self.apps.get(name)
    }

    /// 获取所有启用的应用列表
    pub fn enabled_apps(&self) -> Vec<(&String, &AppConfig)> {
        self.apps.iter().filter(|(_, app)| app.enabled).collect()
    }
}

impl AppConfig {
    /// 获取目标已存在时的处理策略，默认返回 "skip"
    pub fn on_exists_strategy(&self) -> &str {
        self.on_exists.as_deref().unwrap_or("skip")
    }
}
