//! 修复命令处理

use anyhow::Result;

use crate::cli::{Cli, Commands};
use crate::commands::{load_config, Command};
use crate::domain::{LinkOps, LinkStatus};
use crate::infra::{build_link_request, resolve_apps, FsUtils, FileSystem, Config, AppConfig};

/// Repair 命令实现
pub struct RepairCommand;

impl Command for RepairCommand {
    fn execute(&self, cli: &Cli) -> Result<()> {
        let (apps, all, force) = match &cli.command {
            Commands::Repair { apps, all, force } => (apps, *all, *force),
            _ => unreachable!(),
        };

        let config = load_config(&cli.config)?;
        handle_repair(&config, apps, all, force, cli.verbose)
    }
}

/// 处理 repair 命令：修复损坏的链接
pub fn handle_repair(config: &Config, apps: &[String], all: bool, force: bool, verbose: bool) -> Result<()> {
    let fs = FsUtils;
    let apps_to_repair = resolve_apps(config, apps, all);

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
        let (request, source_path, _) = build_link_request(app_config, source, workspace_path, force);
        let source_display = source_path.to_string_lossy().to_string();
        let status = LinkOps::check_status(&source_path, &request.target);

        match status {
            LinkStatus::Broken => {
                if verbose {
                    println!("  Repairing broken link: {}", source_display);
                }

                fs.remove_if_exists(&source_path)?;

                LinkOps::link_with_fs(&request, fs, verbose)?;
            }
            LinkStatus::TargetOnly => {
                if force {
                    if verbose {
                        println!("  Creating link for orphaned target: {}", source_display);
                    }

                    LinkOps::link_with_fs(&request, fs, verbose)?;
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