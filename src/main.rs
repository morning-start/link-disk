//! 主程序入口模块
//!
//! 负责程序的整体流程控制，包括：
//! - 命令行参数解析
//! - 配置文件加载
//! - 各子命令的执行调度（委托给 commands 模块）
//! - 错误处理和用户提示

mod cli;
mod commands;
mod config;
mod error;
mod fs_utils;
mod link_ops;
mod path_resolver;
mod workspace;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use config::Config;

/// 程序入口点，捕获并处理所有错误
fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

/// 主运行函数：解析命令行参数并调度到对应的命令处理器
fn run() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init { path, force } => {
            commands::init::handle_init(path, *force, cli.verbose)?
        }

        Commands::Link {
            apps,
            all,
            dry_run,
            force,
        } => {
            let config = load_config(&cli.config)?;
            commands::link::handle_link(&config, apps, *all, *dry_run, *force, cli.verbose)?
        }

        Commands::Unlink {
            apps,
            all,
            force,
            keep_files,
        } => {
            if !*force {
                println!("This will remove links and move files back. Use --force to confirm.");
                return Ok(());
            }
            let config = load_config(&cli.config)?;
            commands::unlink::handle_unlink(&config, apps, *all, *keep_files, cli.verbose)?
        }

        Commands::List { app } => {
            let config = load_config(&cli.config)?;
            commands::list::handle_list(&config, app)
        }

        Commands::Status { apps } => {
            let config = load_config(&cli.config)?;
            commands::status::handle_status(&config, apps)
        }

        Commands::Repair { apps, force } => {
            let config = load_config(&cli.config)?;
            commands::repair::handle_repair(&config, apps, *force, cli.verbose)?
        }
    }

    Ok(())
}

/// 加载配置文件
fn load_config(config_path: &Option<String>) -> Result<Config> {
    let path = match config_path {
        Some(p) => workspace::Workspace::expand_path(p),
        None => workspace::Workspace::config_path()?,
    };

    if !path.exists() {
        anyhow::bail!(
            "Config file not found: {:?}. Run 'link-disk init' first.",
            path
        );
    }

    Config::load(&path)
}
