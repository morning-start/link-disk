//! CLI 命令解析模块
//!
//! 使用 clap 框架定义命令行接口，包括：
//! - init: 初始化工作区和配置文件
//! - link: 创建链接（转移文件夹并创建链接）
//! - unlink: 移除链接并恢复原文件位置
//! - list: 列出所有已配置的应用和链接
//! - status: 检查链接状态是否正常
//! - repair: 修复损坏的链接

use clap::{Parser, Subcommand};

/// CLI 命令行参数结构体
#[derive(Parser)]
#[command(name = "link-disk")]
#[command(author = "Your Name")]
#[command(version)]
#[command(about = "Move folders and link them back", long_about = None)]
pub struct Cli {
    /// 详细输出模式 (-v, --verbose)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// 配置文件路径 (-c, --config)
    #[arg(short, long, global = true, default_value = "~/.link-disk/config.toml")]
    pub config: Option<String>,

    /// 子命令
    #[command(subcommand)]
    pub command: Commands,
}

/// 支持的子命令枚举
#[derive(Subcommand)]
pub enum Commands {
    /// 初始化工作区和配置文件
    Init {
        /// 工作区路径 (-p, --path)
        #[arg(short, long, help = "工作区路径")]
        path: Option<String>,

        /// 强制重新初始化 (-f, --force)
        #[arg(short, long, help = "强制重新初始化")]
        force: bool,
    },
    /// 创建链接（转移文件夹并创建链接）
    Link {
        /// 应用名称列表（不指定则处理所有应用）
        #[arg(help = "应用名称（不指定则处理所有应用）")]
        apps: Vec<String>,

        /// 处理所有已配置的应用 (-a, --all)
        #[arg(short, long, help = "处理所有已配置的应用")]
        all: bool,

        /// 模拟运行，不实际执行操作 (-d, --dry-run)
        #[arg(short, long, help = "模拟运行，不实际执行操作")]
        dry_run: bool,

        /// 强制处理（删除已存在的软链接后重新链接）(-f, --force)
        #[arg(short, long, help = "强制处理（删除已存在的软链接后重新链接）")]
        force: bool,
    },
    /// 移除链接并恢复原文件位置
    Unlink {
        /// 应用名称列表（不指定则处理所有应用）
        #[arg(help = "应用名称（不指定则处理所有应用）")]
        apps: Vec<String>,

        /// 处理所有已配置的应用 (-a, --all)
        #[arg(short, long, help = "处理所有已配置的应用")]
        all: bool,

        /// 强制执行，不确认 (-f, --force)
        #[arg(short, long, help = "强制执行，不确认")]
        force: bool,

        /// 只删除链接，不移动文件 (-k, --keep-files)
        #[arg(short = 'k', long, help = "只删除链接，不移动文件")]
        keep_files: bool,
    },
    /// 列出所有已配置的应用和链接
    List {
        /// 只显示指定应用的链接 (-a, --app)
        #[arg(short, long, help = "只显示指定应用的链接")]
        app: Option<String>,
    },
    /// 检查链接状态是否正常
    Status {
        /// 应用名称列表（不指定则检查所有应用）
        #[arg(help = "应用名称（不指定则检查所有应用）")]
        apps: Vec<String>,
    },
    /// 修复损坏的链接
    Repair {
        /// 应用名称列表（不指定则修复所有应用）
        #[arg(help = "应用名称（不指定则修复所有应用）")]
        apps: Vec<String>,

        /// 强制修复（自动创建缺失的链接）(-f, --force)
        #[arg(short, long, help = "强制修复（自动创建缺失的链接）")]
        force: bool,
    },
}
