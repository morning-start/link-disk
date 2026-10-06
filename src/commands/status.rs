//! 状态检查命令处理

use anyhow::Result;

use crate::cli::{CliContext, StatusArgs};
use crate::commands::{Command, load_config, select_apps};
use crate::domain::{LinkOps, LinkStatus, resolve_source_target};
use crate::infra::{Config, FsUtils};

/// Status 子命令实现
impl Command for StatusArgs {
    fn execute(&self, ctx: &CliContext) -> Result<()> {
        let config = load_config(ctx)?;
        handle_status(&config, &self.apps)
    }
}

/// 处理 status 命令：检查应用链接状态
pub fn handle_status(config: &Config, apps: &[String]) -> Result<()> {
    let fs = FsUtils;
    for (_, app_config) in select_apps(config, apps, false)? {
        println!("App: {}", app_config.name);

        for source in &app_config.sources {
            let (source_path, target_path) =
                resolve_source_target(app_config, source, &config.workspace.path);
            let status = LinkOps::check_status(&source_path, &target_path, &fs);

            let status_icon = match status {
                LinkStatus::Linked => "✓",
                LinkStatus::Broken => "✗",
                _ => "?",
            };

            println!(
                "  {} {} -> {}",
                status_icon,
                source_path.display(),
                status.as_str()
            );
        }
    }

    Ok(())
}
