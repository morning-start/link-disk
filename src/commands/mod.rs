//! 命令处理模块
//!
//! 将各子命令的处理逻辑拆分为独立模块，遵循单一职责原则（SRP）。
//! 使用命令注册表模式，通过 [`dispatch`] 函数统一调度。

pub mod init;
pub mod link;
pub mod unlink;
pub mod list;
pub mod status;
pub mod repair;

use crate::cli::{Cli, Commands};
use crate::infra::{Config, PathResolver, Workspace};
use anyhow::{Context, Result};

/// 命令接口 trait
///
/// 所有子命令统一实现此 trait，通过 [`dispatch`] 函数统一调度。
/// 添加新命令只需：
/// 1. 在 cli.rs 添加 Commands 枚举变体
/// 2. 新建模块实现 Command trait
/// 3. 在 dispatch 中添加匹配分支
pub trait Command {
    /// 命令名称
    #[allow(dead_code)]
    fn name(&self) -> &str;
    /// 执行命令
    fn execute(&self, cli: &Cli) -> Result<()>;
}

/// 命令调度入口
///
/// 根据 clap 解析的命令枚举分发到对应的 Command 实现。
/// 不在 run() 中直接 match，而是集中在此处调度，
/// 降低 main.rs 与命令实现的耦合。
pub fn dispatch(cli: Cli) -> Result<()> {
    match &cli.command {
        Commands::Init { .. } => init::InitCommand.execute(&cli),
        Commands::Link { .. } => link::LinkCommand.execute(&cli),
        Commands::Unlink { .. } => unlink::UnlinkCommand.execute(&cli),
        Commands::List { .. } => list::ListCommand.execute(&cli),
        Commands::Status { .. } => status::StatusCommand.execute(&cli),
        Commands::Repair { .. } => repair::RepairCommand.execute(&cli),
    }
}

/// 加载并校验配置文件（共享给各命令使用）
pub fn load_config(config_path: &Option<String>) -> Result<Config> {
    let path = match config_path {
        Some(p) => PathResolver::expand_home(p),
        None => Workspace::config_path()?,
    };

    if !path.exists() {
        anyhow::bail!(
            "Config file not found: {:?}. Run 'link-disk init' first.",
            path
        );
    }

    let config = Config::load(&path)?;
    config.validate().context("Invalid configuration")?;
    Ok(config)
}