//! 修复命令处理

use anyhow::Result;

use crate::common::app_resolver::resolve_apps;
use crate::common::request_builder::build_link_request;
use crate::config::{AppConfig, Config};
use crate::fs_utils::{FsUtils, FileSystem};
use crate::link_ops::{LinkOps, LinkStatus};
use crate::path_resolver::PathResolver;
use crate::workspace::Workspace;

/// 处理 repair 命令：修复损坏的链接
pub fn handle_repair(config: &Config, apps: &[String], force: bool, verbose: bool) -> Result<()> {
    let fs = FsUtils;
    let apps_to_repair = resolve_apps(config, apps, false);

    for app_id in apps_to_repair {
        if let Some(app_config) = config.get_app(app_id) {
            repair_app(config, app_config, &fs, force, verbose)?;
        }
    }

    Ok(())
}

/// 修复应用的所有损坏链接
fn repair_app(
    config: &Config,
    app_config: &AppConfig,
    fs: &(dyn FileSystem + 'static),
    force: bool,
    verbose: bool,
) -> Result<()> {
    let workspace_path = &config.workspace.path;

    for source in &app_config.sources {
        let source_path: std::path::PathBuf = PathResolver::expand(&source.source).into();
        let source_display = PathResolver::expand(&source.source);
        let target_relative = format!("{}/{}", app_config.name, source.target);
        let target_path = Workspace::resolve_target(workspace_path, &target_relative);
        let status = LinkOps::check_status(&source_path, &target_path);

        match status {
            LinkStatus::Broken => {
                if verbose {
                    println!("  Repairing broken link: {}", source_display);
                }

                fs.remove_if_exists(&source_path)?;

                let (request, _, _) = build_link_request(app_config, source, workspace_path, true);

                LinkOps::link_with_fs(&request, fs, verbose)?;
            }
            LinkStatus::TargetOnly => {
                if force {
                    if verbose {
                        println!("  Creating link for orphaned target: {}", source_display);
                    }

                    fs.create_symlink(&target_path, &source_path)?;
                } else {
                    println!(
                        "  Target exists without link. Use --force to create link: {}",
                        source_display
                    );
                }
            }
            _ => {
                if verbose {
                    println!("  Skipping {} (status: {})", source_display, status.as_str());
                }
            }
        }
    }

    Ok(())
}
