//! 主程序入口模块
//!
//! 负责程序的整体流程控制，包括：
//! - 命令行参数解析
//! - 配置文件和子命令的调度（委托给 commands 模块）
//! - 错误处理和用户提示

mod cli;
mod commands;
mod domain;
mod infra;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use tracing_subscriber::EnvFilter;

/// 程序入口点，捕获并处理所有错误
fn main() {
    let cli = Cli::parse();
    setup_logging(cli.verbose);
    
    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

/// 初始化日志系统
fn setup_logging(verbose: bool) {
    let filter = if verbose {
        EnvFilter::new("link_disk=debug")
    } else {
        EnvFilter::new("link_disk=info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
}

/// 主运行函数：将命令分发给注册表中的 Command 实现
fn run(cli: Cli) -> Result<()> {
    commands::dispatch(cli)
}