use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "link-disk")]
#[command(author = "Your Name")]
#[command(version = "0.1.0")]
#[command(about = "Move folders and link them back", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[arg(short, long, global = true, default_value = "~/.link-disk/config.toml")]
    pub config: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    Init {
        #[arg(short, long)]
        path: Option<String>,

        #[arg(short, long)]
        force: bool,
    },
    Link {
        apps: Vec<String>,

        #[arg(short, long)]
        all: bool,

        #[arg(short, long)]
        dry_run: bool,
    },
    Unlink {
        apps: Vec<String>,

        #[arg(short, long)]
        all: bool,

        #[arg(short, long)]
        force: bool,
    },
    List {
        #[arg(short, long)]
        app: Option<String>,
    },
    Status {
        apps: Vec<String>,
    },
    Repair {
        apps: Vec<String>,

        #[arg(short, long)]
        force: bool,
    },
}
