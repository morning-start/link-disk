//! 修复命令处理

use anyhow::Result;

use crate::cli::{CliContext, RepairArgs};
use crate::commands::{Command, load_config, select_apps};
use crate::domain::{LinkOps, LinkStatus, build_link_request};
use crate::infra::{Config, FileSystem, FsUtils};

/// Repair 子命令实现
impl Command for RepairArgs {
    fn execute(&self, ctx: &CliContext) -> Result<()> {
        let config = load_config(ctx)?;
        handle_repair(&config, &self.apps, self.all, self.force, ctx.verbose)
    }
}

/// 处理 repair 命令：修复损坏的链接
pub fn handle_repair(
    config: &Config,
    apps: &[String],
    all: bool,
    force: bool,
    verbose: bool,
) -> Result<()> {
    let fs = FsUtils;

    for (app_id, app_config) in select_apps(config, apps, all)? {
        if verbose {
            println!("\nRepairing app: {}", app_config.name);
        }

        for source in &app_config.sources {
            let request = build_link_request(app_config, source, &config.workspace.path, force);
            repair_source(&request, &fs, force, verbose).map_err(|e| {
                e.context(format!(
                    "Failed to repair {}:{}",
                    app_id,
                    request.source.display()
                ))
            })?;
        }
    }

    Ok(())
}

/// 修复单个 source 的链接状态
fn repair_source(
    request: &crate::domain::LinkRequest,
    fs: &dyn FileSystem,
    force: bool,
    verbose: bool,
) -> Result<()> {
    let source_display = request.source.display().to_string();

    match LinkOps::check_status(&request.source, &request.target, fs) {
        // 源链接指向已消失的目标：删除旧链接后重建（目标缺失时由 link 流程补建空目录）
        LinkStatus::Broken => {
            fs.remove_if_exists(&request.source)?;
            LinkOps::link_with_fs(request, fs)?;
            println!("  ✓ Repaired broken link: {source_display}");
        }
        // 目标孤立存在（无链接）：--force 时补建链接
        LinkStatus::TargetOnly => {
            if force {
                LinkOps::link_with_fs(request, fs)?;
                println!("  ✓ Linked orphaned target: {source_display}");
            } else {
                println!("  Target exists without link, use --force: {source_display}");
            }
        }
        // 其余状态（linked / both_exist / source_only / none）无需修复
        other => {
            if verbose {
                println!("  Skipping {source_display} (status: {})", other.as_str());
            }
        }
    }

    Ok(())
}
