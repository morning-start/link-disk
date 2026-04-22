use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "link-disk")]
#[command(author = "Your Name")]
#[command(version = "0.2.0")]
#[command(about = "Move folders and link them back", long_about = None)]
pub struct Cli {
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[arg(short, long, global = true, default_value = "~/.link-disk/config.toml")]
    pub config: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "初始化工作区和配置文件")]
    Init {
        #[arg(short, long, help = "工作区路径")]
        path: Option<String>,

        #[arg(short, long, help = "强制重新初始化")]
        force: bool,
    },
    #[command(about = "创建链接（转移文件夹并创建链接）")]
    Link {
        #[arg(help = "应用名称（不指定则处理所有应用）")]
        apps: Vec<String>,

        #[arg(short, long, help = "处理所有已配置的应用")]
        all: bool,

        #[arg(short, long, help = "模拟运行，不实际执行操作")]
        dry_run: bool,

        #[arg(short, long, help = "强制处理（删除已存在的软链接后重新链接）")]
        force: bool,
    },
    #[command(about = "移除链接并恢复原文件位置")]
    Unlink {
        #[arg(help = "应用名称（不指定则处理所有应用）")]
        apps: Vec<String>,

        #[arg(short, long, help = "处理所有已配置的应用")]
        all: bool,

        #[arg(short, long, help = "强制执行，不确认")]
        force: bool,

        #[arg(short = 'k', long, help = "只删除链接，不移动文件")]
        keep_files: bool,
    },
    #[command(about = "列出所有已配置的应用和链接")]
    List {
        #[arg(short, long, help = "只显示指定应用的链接")]
        app: Option<String>,
    },
    #[command(about = "检查链接状态是否正常")]
    Status {
        #[arg(help = "应用名称（不指定则检查所有应用）")]
        apps: Vec<String>,
    },
    #[command(about = "修复损坏的链接")]
    Repair {
        #[arg(help = "应用名称（不指定则修复所有应用）")]
        apps: Vec<String>,

        #[arg(short, long, help = "强制修复（自动创建缺失的链接）")]
        force: bool,
    },
}
