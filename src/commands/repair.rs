//! 修复命令处理

use anyhow::Result;

use crate::cli::{Cli, Commands};
use crate::commands::{load_config, Command};
use crate::domain::{LinkOps, LinkStatus};
use crate::infra::{resolve_apps, build_link_request, resolve_paths, FsUtils, FileSystem, Config, AppConfig};

/// Repair 命令实现
pub struct RepairCommand;

impl Command for RepairCommand {
    fn name(&self) -> &str {
        "repair"
    }

    fn execute(&self, cli: &Cli) -> Result<()> {
        let (apps, force) = match &cli.command {
            Commands::Repair { apps, force } => (apps, *force),
            _ => unreachable!(),
        };

        let config = load_config(&cli.config)?;
        handle_repair(&config, apps, force, cli.verbose)
    }
}

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
        let (source_path, target_path) = resolve_paths(app_config, source, workspace_path);
        let source_display = source_path.to_string_lossy().to_string();
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