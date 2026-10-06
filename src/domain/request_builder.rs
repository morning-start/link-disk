//! 链接请求构建
//!
//! 将配置数据（[`AppConfig`] / [`Source`]）解析为领域层请求对象
//! （[`LinkRequest`]）或已解析的源/目标路径对。
//!
//! 本模块属于领域层：它依赖 infra 层的配置与路径解析设施，
//! 产出领域层的请求类型，方向严格向下（domain → infra）。

use std::path::{Path, PathBuf};

use crate::domain::LinkRequest;
use crate::infra::{AppConfig, PathResolver, Source, Workspace};

/// 解析单个 source 配置对应的源路径与目标路径
///
/// - 源路径：展开占位符（如 `<home>`）后的实际路径
/// - 目标路径：`<workspace>/<app.name>/<source.target>`
///
/// # 返回值
/// 元组: (源路径, 目标路径)
pub fn resolve_source_target(
    app_config: &AppConfig,
    source: &Source,
    workspace_path: &Path,
) -> (PathBuf, PathBuf) {
    let source_path = PathBuf::from(PathResolver::expand(&source.source));
    let target_relative = format!("{}/{}", app_config.name, source.target);
    let target_path = Workspace::resolve_target(workspace_path, &target_relative);
    (source_path, target_path)
}

/// 从 Source 配置构建 [`LinkRequest`]
///
/// 策略优先级由 [`AppConfig::effective_strategy`] 统一定义：
/// 源级 `on_exists` > 应用级 `on_exists` > 默认 `Skip`。
pub fn build_link_request(
    app_config: &AppConfig,
    source: &Source,
    workspace_path: &Path,
    force: bool,
) -> LinkRequest {
    let (source_path, target_path) = resolve_source_target(app_config, source, workspace_path);

    LinkRequest {
        source: source_path,
        target: target_path,
        link_type: source.link_type,
        on_exists: app_config.effective_strategy(source),
        force,
    }
}
