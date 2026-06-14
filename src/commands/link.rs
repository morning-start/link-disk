//! 链接命令处理

use anyhow::{Context, Result};

use crate::cli::{Cli, Commands};
use crate::commands::{Command, load_config};
use crate::domain::LinkOps;
use crate::infra::{AppConfig, Config, FileSystem, FsUtils, build_link_request, resolve_apps};
use spinners::{Spinner, Spinners};

/// Link 命令实现
pub struct LinkCommand;

impl Command for LinkCommand {
    fn execute(&self, cli: &Cli) -> Result<()> {
        let (apps, all, dry_run, force) = match &cli.command {
            Commands::Link {
                apps,
                all,
                dry_run,
                force,
            } => (apps, *all, *dry_run, *force),
            _ => unreachable!(),
        };

        let config = load_config(cli.config.as_ref())?;
        handle_link(&config, apps, all, dry_run, force, cli.verbose)
    }
}

/// 处理 link 命令：为应用创建链接
pub fn handle_link(
    config: &Config,
    apps: &[String],
    all: bool,
    dry_run: bool,
    force: bool,
    verbose: bool,
) -> Result<()> {
    let workspace_path = &config.workspace.path;
    let fs = FsUtils;

    if !workspace_path.exists() {
        if verbose {
            println!("Creating workspace directory: {}", workspace_path.display());
        }
        fs.ensure_parent_exists(workspace_path)?;
    }

    let apps_to_link = resolve_apps(config, apps, all);

    if apps_to_link.is_empty() {
        println!("No apps to link. Configure apps in config.toml or use --all");
        return Ok(());
    }

    for app_id in apps_to_link {
        let app_config = config.get_app(app_id).context("App not found in config")?;

        if verbose {
            println!("\nLinking app: {}", app_config.name);
        }

        link_app(config, app_id, app_config, &fs, dry_run, force, verbose)?;
    }

    Ok(())
}

/// 执行单个应用的链接创建
fn link_app(
    config: &Config,
    app_id: &str,
    app_config: &AppConfig,
    fs: &dyn FileSystem,
    dry_run: bool,
    force: bool,
    verbose: bool,
) -> Result<()> {
    let workspace_path = &config.workspace.path;

    for source in &app_config.sources {
        let (request, source_path, _) =
            build_link_request(app_config, source, workspace_path, force);
        let source_path_str = source_path.to_string_lossy().to_string();

        if verbose {
            println!("  Source: {}", source_path_str);
            println!("  Target: {}", request.target.display());
        }

        if dry_run {
            println!(
                "  [DRY RUN] Would link {} -> {}",
                source_path_str,
                request.target.display()
            );
            continue;
        }

        // 用 Path::file_name() 取末段，同时支持 '/' 和 '\' 分隔符
        // （原 split('/') 在 Windows 反斜杠路径下会返回整条路径）
        let source_name = source_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| source_path.to_string_lossy().into_owned());
        let display_name = &app_config.name;
        let mut sp = Spinner::new(Spinners::Dots12, format!("  Linking {}...", display_name));

        match LinkOps::link_with_fs(&request, fs, verbose) {
            Ok(_) => {
                sp.stop();
                println!("  ✓ Linked: {} ({})", display_name, source_name);
            }
            Err(e) => {
                sp.stop();
                anyhow::bail!("Failed to link {}:{} - {}", app_id, source_path_str, e);
            }
        }
    }

    Ok(())
}
