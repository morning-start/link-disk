//! 状态检查命令处理

use crate::cli::{Cli, Commands};
use crate::commands::{load_config, Command};
use crate::infra::{Config, AppConfig};
use crate::domain::{LinkOps, LinkStatus};
use crate::infra::resolve_paths;
use anyhow::Result;
use std::path::Path;

/// Status 命令实现
pub struct StatusCommand;

impl Command for StatusCommand {
    fn name(&self) -> &str {
        "status"
    }

    fn execute(&self, cli: &Cli) -> Result<()> {
        let apps = match &cli.command {
            Commands::Status { apps } => apps,
            _ => unreachable!(),
        };

        let config = load_config(&cli.config)?;
        handle_status(&config, apps);
        Ok(())
    }
}

/// 处理 status 命令：检查应用链接状态
pub fn handle_status(config: &Config, apps: &[String]) {
    let workspace_path = &config.workspace.path;
    let apps_to_check: Vec<&AppConfig> = if apps.is_empty() {
        config.enabled_apps().into_iter().map(|(_, c)| c).collect()
    } else {
        apps.iter().filter_map(|a| config.get_app(a)).collect()
    };

    for app_config in apps_to_check {
        check_app_status(app_config, workspace_path);
    }
}

/// 检查应用的所有链接状态
fn check_app_status(
    app_config: &AppConfig,
    workspace_path: &Path,
) {
    println!("App: {}", app_config.name);

    for source in &app_config.sources {
        let (source_path, target_path) = resolve_paths(app_config, source, workspace_path);
        let source_display = source_path.to_string_lossy().to_string();
        let status = LinkOps::check_status(&source_path, &target_path);

        let status_icon = match status {
            LinkStatus::Linked => "✓",
            LinkStatus::Broken => "✗",
            _ => "?",
        };

        println!("  {} {} -> {}", status_icon, source_display, status.as_str());
    }
}