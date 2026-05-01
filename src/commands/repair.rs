//! 修复命令处理

use anyhow::Result;
use std::path::Path;

use crate::config::{AppConfig, Config};
use crate::fs_utils::{FsUtils, FileSystem};
use crate::link_ops::{LinkOps, LinkRequest, LinkStatus};
use crate::path_resolver::PathResolver;
use crate::workspace::Workspace;

/// 处理 repair 命令：修复损坏的链接
pub fn handle_repair(config: &Config, apps: &[String], force: bool, verbose: bool) -> Result<()> {
    let apps_to_repair: Vec<String> = if apps.is_empty() {
        config
            .enabled_apps()
            .into_iter()
            .map(|(n, _)| n.clone())
            .collect()
    } else {
        apps.to_vec()
    };

    for app_id in &apps_to_repair {
        if let Some(app_config) = config.get_app(app_id) {
            repair_app(config, app_config, force, verbose)?;
        }
    }

    Ok(())
}

/// 从 Source 配置构建 LinkRequest
fn build_link_request(
    app_config: &AppConfig,
    source: &crate::config::Source,
    workspace_path: &Path,
    force: bool,
) -> (LinkRequest, std::path::PathBuf, std::path::PathBuf) {
    let source_path = PathResolver::resolve_if_exists(&source.source)
        .unwrap_or_else(|| PathResolver::expand(&source.source).into());
    let target_relative = format!("{}/{}", app_config.name, source.target);
    let target_path = Workspace::resolve_target(workspace_path, &target_relative);

    let request = LinkRequest {
        source: source_path.clone(),
        target: target_path.clone(),
        link_type: crate::link_ops::LinkType::from_str(&source.link_type),
        on_exists: crate::link_ops::OnExists::from_str(app_config.on_exists_strategy()),
        force,
    };

    (request, source_path, target_path)
}

/// 修复应用的所有损坏链接
fn repair_app(config: &Config, app_config: &AppConfig, force: bool, verbose: bool) -> Result<()> {
    let workspace_path = &config.workspace.path;

    for source in &app_config.sources {
        let source_path: std::path::PathBuf = PathResolver::expand(&source.source).into();
        let target_relative = format!("{}/{}", app_config.name, source.target);
        let target_path = Workspace::resolve_target(workspace_path, &target_relative);
        let status = LinkOps::check_status(&source_path, &target_path);

        match status {
            LinkStatus::Broken => {
                if verbose {
                    println!("  Repairing broken link: {}", source.source);
                }

                let fs = FsUtils;
                fs.remove_if_exists(&source_path, verbose)?;

                let (request, _, _) = build_link_request(app_config, source, workspace_path, true);

                LinkOps::link(&request, verbose)?;
            }
            LinkStatus::TargetOnly => {
                if force {
                    if verbose {
                        println!("  Creating link for orphaned target: {}", source.source);
                    }

                    let fs = FsUtils;
                    fs.create_symlink(&target_path, &source_path)?;
                } else {
                    println!(
                        "  Target exists without link. Use --force to create link: {}",
                        source.source
                    );
                }
            }
            _ => {
                if verbose {
                    println!("  Skipping {} (status: {})", source.source, status.as_str());
                }
            }
        }
    }

    Ok(())
}
