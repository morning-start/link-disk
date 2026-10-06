//! 列表命令处理

use anyhow::Result;

use crate::cli::{CliContext, ListArgs};
use crate::commands::{Command, load_config};
use crate::infra::{AppConfig, Config};

/// List 子命令实现
impl Command for ListArgs {
    fn execute(&self, ctx: &CliContext) -> Result<()> {
        let config = load_config(ctx)?;
        handle_list(&config, self.app.as_deref());
        Ok(())
    }
}

/// 处理 list 命令：列出应用的链接配置
pub fn handle_list(config: &Config, app: Option<&str>) {
    match app {
        Some(app_id) => match config.get_app(app_id) {
            Some(app_config) => print_app_links(app_config),
            None => println!("App not found: {app_id}"),
        },
        None => {
            for (_, app_config) in config.enabled_apps() {
                print_app_links(app_config);
                println!();
            }
        }
    }
}

/// 打印单个应用的链接配置（仅展示配置的原始路径，不做解析）
fn print_app_links(app_config: &AppConfig) {
    println!("App: {}", app_config.name);

    for source in &app_config.sources {
        println!("  {} -> {}", source.source, source.target);
    }
}
