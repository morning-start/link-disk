//! 解链命令处理

use anyhow::Result;

use crate::cli::{CliContext, UnlinkArgs};
use crate::commands::{Command, load_config, select_apps};
use crate::domain::{LinkOps, resolve_source_target};
use crate::infra::{Config, FsUtils};

/// Unlink 子命令实现
impl Command for UnlinkArgs {
    fn execute(&self, ctx: &CliContext) -> Result<()> {
        // unlink 会把数据从工作区移回原位置，需要显式 --force 确认
        if !self.force {
            println!("This will remove links and move files back. Use --force to confirm.");
            return Ok(());
        }

        let config = load_config(ctx)?;
        handle_unlink(&config, &self.apps, self.all, self.keep_files, ctx.verbose)
    }
}

/// 处理 unlink 命令：删除链接并可选择移回文件
pub fn handle_unlink(
    config: &Config,
    apps: &[String],
    all: bool,
    keep_files: bool,
    verbose: bool,
) -> Result<()> {
    let fs = FsUtils;
    let apps_to_unlink = select_apps(config, apps, all)?;

    for (app_id, app_config) in apps_to_unlink {
        if verbose {
            println!("\nUnlinking app: {}", app_config.name);
        }

        for source in &app_config.sources {
            let (source_path, target_path) =
                resolve_source_target(app_config, source, &config.workspace.path);

            if verbose {
                println!("  Source: {}", source_path.display());
                println!("  Target: {}", target_path.display());
            }

            LinkOps::unlink_with_fs(&source_path, &target_path, keep_files, &fs).map_err(|e| {
                e.context(format!(
                    "Failed to unlink {}:{}",
                    app_id,
                    source_path.display()
                ))
            })?;
        }
    }

    Ok(())
}
