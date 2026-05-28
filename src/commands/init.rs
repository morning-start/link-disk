//! 初始化命令处理

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::cli::{Cli, Commands};
use crate::commands::Command;
use crate::infra::Workspace;

/// Init 命令实现
pub struct InitCommand;

impl Command for InitCommand {
    fn name(&self) -> &str {
        "init"
    }

    fn execute(&self, cli: &Cli) -> Result<()> {
        let (path, force) = match &cli.command {
            Commands::Init { path, force } => (path, *force),
            _ => unreachable!(),
        };

        let workspace_path = match path {
            Some(p) => PathBuf::from(p),
            None => {
                let config_path = Workspace::config_path()?;
                if config_path.exists() && !force {
                    anyhow::bail!("Config already exists. Use --force to reinitialize.");
                }
                PathBuf::from("D:/link-disk-workspace")
            }
        };

        if cli.verbose {
            println!("Initializing workspace at: {}", workspace_path.display());
        }

        Workspace::init(&workspace_path).context("Failed to initialize workspace")?;

        println!("Workspace initialized at: {}", workspace_path.display());
        println!("Config file: {}", Workspace::config_path()?.display());

        Ok(())
    }
}