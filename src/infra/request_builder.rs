//! 应用解析和链接请求构建
//!
//! 提供统一的应用列表解析和 LinkRequest 构建逻辑，避免在多个命令中重复。
//!
//! 本模块属于基础设施层，负责将配置数据转换为领域层请求对象。

use std::path::{Path, PathBuf};

use crate::infra::{AppConfig, Config, Source};
use crate::infra::{PathResolver, Workspace};
use crate::domain::{LinkRequest, LinkType, OnExists};

/// 解析需要处理的应用列表
///
/// # 参数
/// - `config`: 配置对象
/// - `apps`: 用户指定的应用名称列表（命令行参数）
/// - `all`: 是否处理所有启用的应用（--all 标志）
///
/// # 返回值
/// 应用 ID 列表
pub fn resolve_apps<'a>(config: &'a Config, apps: &'a [String], all: bool) -> Vec<&'a String> {
    if all || apps.is_empty() {
        config.enabled_apps().into_iter().map(|(n, _)| n).collect()
    } else {
        apps.iter().collect()
    }
}

/// 解析源路径和目标路径的公共逻辑
///
/// # 参数
/// - `app_config`: 应用配置
/// - `source`: 源配置
/// - `workspace_path`: 工作区根目录路径
///
/// # 返回值
/// 元组: (源路径, 目标路径)
fn resolve_source_target(
    app_config: &AppConfig,
    source: &Source,
    workspace_path: &Path,
) -> (PathBuf, PathBuf) {
    let source_path = PathResolver::resolve_if_exists(&source.source)
        .unwrap_or_else(|| PathResolver::expand(&source.source).into());
    let target_relative = format!("{}/{}", app_config.name, source.target);
    let target_path = Workspace::resolve_target(workspace_path, &target_relative);
    (source_path, target_path)
}

/// 从 Source 配置构建 LinkRequest
///
/// # 参数
/// - `app_config`: 应用配置（包含名称和 on_exists 策略）
/// - `source`: 源配置（包含源路径、目标路径、链接类型）
/// - `workspace_path`: 工作区根目录路径
/// - `force`: 是否强制覆盖已存在的链接
///
/// # 返回值
/// 元组: (LinkRequest, 源路径, 目标路径)
pub fn build_link_request(
    app_config: &AppConfig,
    source: &Source,
    workspace_path: &Path,
    force: bool,
) -> (LinkRequest, PathBuf, PathBuf) {
    let (source_path, target_path) = resolve_source_target(app_config, source, workspace_path);

    let request = LinkRequest {
        source: source_path.clone(),
        target: target_path.clone(),
        link_type: LinkType::from_str_lossy(&source.link_type),
        on_exists: OnExists::from_str_lossy(app_config.on_exists_strategy()),
        force,
    };

    (request, source_path, target_path)
}

/// 解析源路径和目标路径（不构建 LinkRequest）
///
/// 用于只需要路径信息的场景（如 status、unlink 命令）。
///
/// # 参数
/// - `app_config`: 应用配置
/// - `source`: 源配置
/// - `workspace_path`: 工作区根目录路径
///
/// # 返回值
/// 元组: (源路径, 目标路径)
pub fn resolve_paths(
    app_config: &AppConfig,
    source: &Source,
    workspace_path: &Path,
) -> (PathBuf, PathBuf) {
    resolve_source_target(app_config, source, workspace_path)
}
