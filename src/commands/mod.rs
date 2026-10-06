//! 命令处理模块
//!
//! 将各子命令的处理逻辑拆分为独立模块，遵循单一职责原则（SRP）。
//! 使用命令注册表模式，通过 [`dispatch`] 函数统一调度。

pub mod init;
pub mod link;
pub mod list;
pub mod repair;
pub mod status;
pub mod unlink;

use anyhow::{Context, Result};

use crate::cli::{Cli, CliContext, Commands};
use crate::infra::{AppConfig, Config, PathResolver, Workspace};

/// 命令接口 trait
///
/// 所有子命令的参数结构体（见 `cli.rs`）统一实现此 trait，
/// 通过 [`dispatch`] 函数统一调度。添加新命令只需：
/// 1. 在 cli.rs 添加 `*Args` 参数结构体和 `Commands` 枚举变体
/// 2. 新建模块为该参数结构体实现 Command trait
/// 3. 在 dispatch 中添加匹配分支
pub trait Command {
    /// 执行命令（`ctx` 提供全局选项，参数结构体自身提供子命令选项）
    fn execute(&self, ctx: &CliContext) -> Result<()>;
}

/// 命令调度入口
///
/// 根据 clap 解析的命令枚举分发到对应的 Command 实现。
/// 不在 run() 中直接 match，而是集中在此处调度，
/// 降低 main.rs 与命令实现的耦合。
pub fn dispatch(cli: Cli) -> Result<()> {
    let ctx = cli.context();
    match &cli.command {
        Commands::Init(args) => args.execute(&ctx),
        Commands::Link(args) => args.execute(&ctx),
        Commands::Unlink(args) => args.execute(&ctx),
        Commands::List(args) => args.execute(&ctx),
        Commands::Status(args) => args.execute(&ctx),
        Commands::Repair(args) => args.execute(&ctx),
    }
}

/// 按优先级解析配置文件路径并加载、校验配置。
///
/// 优先级（与 AGENTS.md 约定一致）：
/// 1. 命令行 `--config <path>`
/// 2. 环境变量 `LINK_DISK_CONFIG`
/// 3. 默认位置 `~/.link-disk/config.toml`
pub fn load_config(ctx: &CliContext) -> Result<Config> {
    let path = resolve_config_path(ctx)?;

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

/// 解析配置文件路径（命令行 > `LINK_DISK_CONFIG` 环境变量 > 默认位置）
fn resolve_config_path(ctx: &CliContext) -> Result<std::path::PathBuf> {
    if let Some(p) = &ctx.config {
        return Ok(PathResolver::expand_home(p));
    }

    let env_path = std::env::var("LINK_DISK_CONFIG")
        .ok()
        .filter(|p| !p.trim().is_empty());
    if let Some(p) = env_path {
        return Ok(PathResolver::expand_home(&p));
    }

    Workspace::config_path()
}

/// 解析本次要处理的应用列表（link / unlink / status / repair 共用）
///
/// 统一行为：
/// - `all` 或 `ids` 为空 → 所有启用的应用（按 ID 字典序，输出稳定）
/// - 显式指定了不存在的应用 ID → 直接报错（而不是静默跳过）
pub fn select_apps<'a>(
    config: &'a Config,
    ids: &'a [String],
    all: bool,
) -> Result<Vec<(&'a String, &'a AppConfig)>> {
    if all || ids.is_empty() {
        return Ok(config.enabled_apps());
    }

    ids.iter()
        .map(|id| {
            config
                .get_app(id)
                .with_context(|| format!("App '{}' not found in config", id))
                .map(|app| (id, app))
        })
        .collect()
}
