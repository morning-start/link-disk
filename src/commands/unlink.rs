//! 解链命令处理

use anyhow::{Context, Result};

use crate::config::{AppConfig, Config};
use crate::link_ops::LinkOps;
use crate::path_resolver::PathResolver;
use crate::workspace::Workspace;

/// 处理 unlink 命令：删除链接并可选择移回文件
pub fn handle_unlink(
    config: &Config,
    apps: &[String],
    all: bool,
    keep_files: bool,
    verbose: bool,
) -> Result<()> {
    let apps_to_unlink: Vec<&String> = if all || apps.is_empty() {
        config.enabled_apps().into_iter().map(|(n, _)| n).collect()
    } else {
        apps.iter().collect()
    };

    for app_id in apps_to_unlink {
        let app_config = config.get_app(app_id).context("App not found in config")?;

        if verbose {
            println!("\nUnlinking app: {}", app_config.name);
        }

        unlink_app(config, app_id, app_config, keep_files, verbose)?;
    }

    Ok(())
}

/// 执行单个应用的链接删除
fn unlink_app(
    config: &Config,
    app_id: &str,
    app_config: &AppConfig,
    keep_files: bool,
    verbose: bool,
) -> Result<()> {
    let workspace_path = &config.workspace.path;

    for source in &app_config.sources {
        let source_path = PathResolver::resolve_if_exists(&source.source)
            .unwrap_or_else(|| PathResolver::expand(&source.source).into());
        let source_display = PathResolver::expand(&source.source);

        let target_relative = format!("{}/{}", app_config.name, source.target);
        let target_path = Workspace::resolve_target(workspace_path, &target_relative);

        if verbose {
            println!("  Source: {:?}", source_path);
            println!("  Target: {:?}", target_path);
        }

        LinkOps::unlink(&source_path, &target_path, keep_files, verbose)
            .with_context(|| format!("Failed to unlink {}:{}", app_id, source_display))?;
    }

    Ok(())
}
