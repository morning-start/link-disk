//! CLI 命令解析模块
//!
//! 使用 clap 框架定义命令行接口，包括：
//! - init: 初始化工作区和配置文件
//! - link: 创建链接（转移文件夹并创建链接）
//! - unlink: 移除链接并恢复原文件位置
//! - list: 列出所有已配置的应用和链接
//! - status: 检查链接状态是否正常
//! - repair: 修复损坏的链接
//!
//! 每个子命令的参数各自定义为一个 [`clap::Args`] 结构体，
//! 由 [`Commands`] 枚举以元组变体的形式持有。
//! 命令实现直接对参数结构体实现 `Command` trait，
//! 无需在 execute 中二次 match 自己已知的变体。

use clap::{Args, Parser, Subcommand};

/// CLI 命令行参数结构体
#[derive(Parser)]
#[command(name = "link-disk")]
#[command(version)]
#[command(about = "Move folders and link them back", long_about = None)]
pub struct Cli {
    /// 详细输出模式 (-v, --verbose)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// 配置文件路径 (-c, --config)，默认 $LINK_DISK_CONFIG 或 ~/.link-disk/config.toml
    #[arg(short, long, global = true)]
    pub config: Option<String>,

    /// 子命令
    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    /// 提取全局选项，供各命令实现使用（避免命令实现再 match 整个 Cli 拿参数）
    #[must_use]
    pub fn context(&self) -> CliContext {
        CliContext {
            verbose: self.verbose,
            config: self.config.clone(),
        }
    }
}

/// 全局选项（跨子命令共享）
pub struct CliContext {
    /// 详细输出模式
    pub verbose: bool,
    /// 配置文件路径（未指定时按 `LINK_DISK_CONFIG` 环境变量 → 默认位置解析）
    pub config: Option<String>,
}

/// 支持的子命令枚举（参数定义见各 `*Args` 结构体）
#[derive(Subcommand)]
pub enum Commands {
    /// 初始化工作区和配置文件
    Init(InitArgs),
    /// 创建链接（转移文件夹并创建链接）
    Link(LinkArgs),
    /// 移除链接并恢复原文件位置
    Unlink(UnlinkArgs),
    /// 列出所有已配置的应用和链接
    List(ListArgs),
    /// 检查链接状态是否正常
    Status(StatusArgs),
    /// 修复损坏的链接
    Repair(RepairArgs),
}

/// init 子命令参数
#[derive(Args)]
pub struct InitArgs {
    /// 工作区路径 (-p, --path)
    #[arg(short, long, help = "工作区路径")]
    pub path: Option<String>,

    /// 强制重新初始化 (-f, --force)
    #[arg(short, long, help = "强制重新初始化（覆盖已有配置文件）")]
    pub force: bool,
}

/// link 子命令参数
#[derive(Args)]
pub struct LinkArgs {
    /// 应用名称列表（不指定则处理所有应用）
    #[arg(help = "应用名称（不指定则处理所有应用）")]
    pub apps: Vec<String>,

    /// 处理所有已配置的应用 (-a, --all)
    #[arg(short, long, help = "处理所有已配置的应用")]
    pub all: bool,

    /// 模拟运行，不实际执行操作 (-d, --dry-run)
    #[arg(short, long, help = "模拟运行，不实际执行操作")]
    pub dry_run: bool,

    /// 强制处理（删除已存在的软链接后重新链接）(-f, --force)
    #[arg(short, long, help = "强制处理（删除已存在的软链接后重新链接）")]
    pub force: bool,
}

/// unlink 子命令参数
#[derive(Args)]
pub struct UnlinkArgs {
    /// 应用名称列表（不指定则处理所有应用）
    #[arg(help = "应用名称（不指定则处理所有应用）")]
    pub apps: Vec<String>,

    /// 处理所有已配置的应用 (-a, --all)
    #[arg(short, long, help = "处理所有已配置的应用")]
    pub all: bool,

    /// 强制执行，不确认 (-f, --force)
    #[arg(short, long, help = "强制执行，不确认")]
    pub force: bool,

    /// 只删除链接，不移动文件 (-k, --keep-files)
    #[arg(short = 'k', long, help = "只删除链接，不移动文件")]
    pub keep_files: bool,
}

/// list 子命令参数
#[derive(Args)]
pub struct ListArgs {
    /// 只显示指定应用的链接 (-a, --app)
    #[arg(short, long, help = "只显示指定应用的链接")]
    pub app: Option<String>,
}

/// status 子命令参数
#[derive(Args)]
pub struct StatusArgs {
    /// 应用名称列表（不指定则检查所有应用）
    #[arg(help = "应用名称（不指定则检查所有应用）")]
    pub apps: Vec<String>,
}

/// repair 子命令参数
#[derive(Args)]
pub struct RepairArgs {
    /// 应用名称列表（不指定则修复所有应用）
    #[arg(help = "应用名称（不指定则修复所有应用）")]
    pub apps: Vec<String>,

    /// 处理所有已配置的应用 (-a, --all)
    #[arg(short, long, help = "处理所有已配置的应用")]
    pub all: bool,

    /// 强制修复（自动创建缺失的链接）(-f, --force)
    #[arg(short, long, help = "强制修复（自动创建缺失的链接）")]
    pub force: bool,
}
