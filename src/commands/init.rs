//! 初始化命令处理

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::cli::{CliContext, InitArgs};
use crate::commands::Command;
use crate::infra::Workspace;

/// 未显式指定路径时的默认工作区
const DEFAULT_WORKSPACE_PATH: &str = "D:/link-disk-workspace";

/// Init 子命令实现
impl Command for InitArgs {
    fn execute(&self, ctx: &CliContext) -> Result<()> {
        let workspace_path = match &self.path {
            Some(p) => PathBuf::from(p),
            None => {
                // 未指定路径时走默认位置；已有配置则要求显式 --force
                let config_path = Workspace::config_path()?;
                if config_path.exists() && !self.force {
                    anyhow::bail!("Config already exists. Use --force to reinitialize.");
                }
                PathBuf::from(DEFAULT_WORKSPACE_PATH)
            }
        };

        if ctx.verbose {
            println!("Initializing workspace at: {}", workspace_path.display());
        }

        Workspace::init(&workspace_path, self.force).context("Failed to initialize workspace")?;

        println!("Workspace initialized at: {}", workspace_path.display());
        println!("Config file: {}", Workspace::config_path()?.display());

        Ok(())
    }
}
