//! 列表命令处理

use crate::cli::{Cli, Commands};
use crate::commands::{load_config, Command};
use crate::infra::{AppConfig, Config};
use anyhow::Result;

/// List 命令实现
pub struct ListCommand;

impl Command for ListCommand {
    fn name(&self) -> &str {
        "list"
    }

    fn execute(&self, cli: &Cli) -> Result<()> {
        let app = match &cli.command {
            Commands::List { app } => app,
            _ => unreachable!(),
        };

        let config = load_config(&cli.config)?;
        handle_list(&config, app);
        Ok(())
    }
}

/// 处理 list 命令：列出应用的链接配置
pub fn handle_list(config: &Config, app: &Option<String>) {
    match app {
        Some(app_id) => {
            if let Some(app_config) = config.get_app(app_id) {
                print_app_links(app_config);
            } else {
                println!("App not found: {}", app_id);
            }
        }
        None => {
            for (_, app_config) in config.enabled_apps() {
                print_app_links(app_config);
                println!();
            }
        }
    }
}

/// 打印应用的链接配置信息
fn print_app_links(app_config: &AppConfig) {
    println!("App: {}", app_config.name);

    for source in &app_config.sources {
        println!("  {} -> {}", source.source, source.target);
    }
}