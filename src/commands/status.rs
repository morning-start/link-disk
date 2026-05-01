//! 状态检查命令处理

use crate::config::{AppConfig, Config};
use crate::link_ops::{LinkOps, LinkStatus};
use crate::path_resolver::PathResolver;

/// 处理 status 命令：检查应用链接状态
pub fn handle_status(config: &Config, apps: &[String]) {
    let apps_to_check: Vec<&AppConfig> = if apps.is_empty() {
        config.enabled_apps().into_iter().map(|(_, c)| c).collect()
    } else {
        apps.iter().filter_map(|a| config.get_app(a)).collect()
    };

    for app_config in apps_to_check {
        check_app_status(app_config);
    }
}

/// 检查应用的所有链接状态
fn check_app_status(app_config: &AppConfig) {
    println!("App: {}", app_config.name);

    for source in &app_config.sources {
        let source_path: std::path::PathBuf = PathResolver::expand(&source.source).into();
        let status = LinkOps::check_status(
            &source_path,
            &std::path::PathBuf::from(&source.target),
        );

        let status_icon = match status {
            LinkStatus::Linked => "✓",
            LinkStatus::Broken => "✗",
            _ => "?",
        };

        println!("  {} {} -> {}", status_icon, source.source, status.as_str());
    }
}
