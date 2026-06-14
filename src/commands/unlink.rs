//! 解链命令处理

use anyhow::{Context, Result};

use crate::cli::{Cli, Commands};
use crate::commands::{Command, load_config};
use crate::domain::LinkOps;
use crate::infra::{
    AppConfig, Config, FileSystem, FsUtils, PathResolver, resolve_apps, resolve_paths,
};

/// Unlink 命令实现
pub struct UnlinkCommand;

impl Command for UnlinkCommand {
    fn execute(&self, cli: &Cli) -> Result<()> {
        let (apps, all, force, keep_files) = match &cli.command {
            Commands::Unlink {
                apps,
                all,
                force,
                keep_files,
            } => (apps, *all, *force, *keep_files),
            _ => unreachable!(),
        };

        if !force {
            println!("This will remove links and move files back. Use --force to confirm.");
            return Ok(());
        }

        let config = load_config(cli.config.as_ref())?;
        handle_unlink(&config, apps, all, keep_files, cli.verbose)
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
    let apps_to_unlink = resolve_apps(config, apps, all);

    for app_id in apps_to_unlink {
        let app_config = config.get_app(app_id).context("App not found in config")?;

        if verbose {
            println!("\nUnlinking app: {}", app_config.name);
        }

        unlink_app(config, app_id, app_config, &fs, keep_files, verbose)?;
    }

    Ok(())
}

/// 执行单个应用的链接删除
fn unlink_app(
    config: &Config,
    app_id: &str,
    app_config: &AppConfig,
    fs: &dyn FileSystem,
    keep_files: bool,
    verbose: bool,
) -> Result<()> {
    let workspace_path = &config.workspace.path;

    for source in &app_config.sources {
        let (source_path, target_path) = resolve_paths(app_config, source, workspace_path);
        let source_display = PathResolver::expand(&source.source);

        if verbose {
            println!("  Source: {}", source_path.display());
            println!("  Target: {}", target_path.display());
        }

        LinkOps::unlink_with_fs(&source_path, &target_path, keep_files, fs)
            .with_context(|| format!("Failed to unlink {}:{}", app_id, source_display))?;
    }

    Ok(())
}
