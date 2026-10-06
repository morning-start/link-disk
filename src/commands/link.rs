//! 链接命令处理

use anyhow::{Context, Result};
use spinners::{Spinner, Spinners};

use crate::cli::{CliContext, LinkArgs};
use crate::commands::{Command, load_config, select_apps};
use crate::domain::{LinkOps, LinkRequest, build_link_request};
use crate::infra::{AppConfig, Config, FileSystem, FsUtils};

/// Link 子命令实现
impl Command for LinkArgs {
    fn execute(&self, ctx: &CliContext) -> Result<()> {
        let config = load_config(ctx)?;
        handle_link(
            &config,
            &self.apps,
            self.all,
            self.dry_run,
            self.force,
            ctx.verbose,
        )
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
    let fs = FsUtils;
    let apps_to_link = select_apps(config, apps, all)?;

    if apps_to_link.is_empty() {
        println!("No apps to link. Configure apps in config.toml or use --all");
        return Ok(());
    }

    // 确保工作区根目录存在（dry-run 不产生任何副作用）
    if !dry_run && !config.workspace.path.exists() {
        if verbose {
            println!(
                "Creating workspace directory: {}",
                config.workspace.path.display()
            );
        }
        std::fs::create_dir_all(&config.workspace.path).with_context(|| {
            format!(
                "Failed to create workspace directory: {:?}",
                config.workspace.path
            )
        })?;
    }

    for (app_id, app_config) in apps_to_link {
        if verbose {
            println!("\nLinking app: {}", app_config.name);
        }

        for source in &app_config.sources {
            let request = build_link_request(app_config, source, &config.workspace.path, force);
            link_source(app_id, app_config, &request, &fs, dry_run, verbose)?;
        }
    }

    Ok(())
}

/// 链接单个 source：dry-run 只打印计划，否则创建链接并给出进度反馈
fn link_source(
    app_id: &str,
    app_config: &AppConfig,
    request: &LinkRequest,
    fs: &dyn FileSystem,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    let source_display = request.source.to_string_lossy().into_owned();

    if verbose {
        println!("  Source: {source_display}");
        println!("  Target: {}", request.target.display());
    }

    if dry_run {
        println!(
            "  [DRY RUN] Would link {source_display} -> {}",
            request.target.display()
        );
        return Ok(());
    }

    // 用 Path::file_name() 取末段，同时支持 '/' 和 '\' 分隔符
    // （split('/') 在 Windows 反斜杠路径下会返回整条路径）
    let source_name = request
        .source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| source_display.clone());

    // verbose 模式下逐行输出已足够，仅在安静模式用 spinner 表示进度
    let mut spinner = (!verbose).then(|| {
        Spinner::new(
            Spinners::Dots12,
            format!("  Linking {app_id}/{source_name}..."),
        )
    });

    let result = LinkOps::link_with_fs(request, fs);

    if let Some(sp) = spinner.as_mut() {
        sp.stop();
    }

    match result {
        Ok(()) => println!("  ✓ Linked: {} ({})", app_config.name, source_name),
        Err(e) => {
            return Err(e.context(format!("Failed to link {}:{}", app_id, source_display)));
        }
    }

    Ok(())
}
