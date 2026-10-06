//! 列表命令处理

use anyhow::{Context, Result};

use crate::cli::{CliContext, ListArgs};
use crate::commands::{Command, load_config};
use crate::infra::{AppConfig, Config};

/// List 子命令实现
impl Command for ListArgs {
    fn execute(&self, ctx: &CliContext) -> Result<()> {
        let config = load_config(ctx)?;
        handle_list(&config, self.app.as_deref())
    }
}

/// 处理 list 命令：列出应用的链接配置
pub fn handle_list(config: &Config, app: Option<&str>) -> Result<()> {
    match app {
        Some(app_id) => {
            // 与其他命令一致：显式指定不存在的应用直接报错
            let app_config = config
                .get_app(app_id)
                .with_context(|| format!("App '{app_id}' not found in config"))?;
            print_app_links(app_config);
        }
        None => {
            for (_, app_config) in config.enabled_apps() {
                print_app_links(app_config);
                println!();
            }
        }
    }

    Ok(())
}

/// 打印单个应用的链接配置（仅展示配置的原始路径，不做解析）
fn print_app_links(app_config: &AppConfig) {
    println!("App: {}", app_config.name);

    for source in &app_config.sources {
        println!("  {} -> {}", source.source, source.target);
    }
}
