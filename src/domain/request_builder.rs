//! 链接请求构建器
//!
//! 提供统一的 LinkRequest 构建逻辑，避免在多个命令中重复。

use std::path::{Path, PathBuf};

use crate::infra::{AppConfig, Source};
use crate::domain::{LinkRequest, LinkType, OnExists};
use crate::infra::{PathResolver, Workspace};

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
    let source_path = PathResolver::resolve_if_exists(&source.source)
        .unwrap_or_else(|| PathResolver::expand(&source.source).into());
    let target_relative = format!("{}/{}", app_config.name, source.target);
    let target_path = Workspace::resolve_target(workspace_path, &target_relative);

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
    let source_path = PathResolver::resolve_if_exists(&source.source)
        .unwrap_or_else(|| PathResolver::expand(&source.source).into());
    let target_relative = format!("{}/{}", app_config.name, source.target);
    let target_path = Workspace::resolve_target(workspace_path, &target_relative);
    (source_path, target_path)
}
