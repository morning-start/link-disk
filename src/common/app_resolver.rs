//! 应用列表解析器
//!
//! 提供统一的应用列表解析逻辑，避免在多个命令中重复。

use crate::config::Config;

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
