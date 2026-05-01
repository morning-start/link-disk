//! 列表命令处理

use crate::config::{AppConfig, Config};

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
