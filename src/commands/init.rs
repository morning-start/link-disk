//! 初始化命令处理

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::infra::Workspace;

/// 处理 init 命令：初始化工作区
pub fn handle_init(path: &Option<String>, force: bool, verbose: bool) -> Result<()> {
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

    if verbose {
        println!("Initializing workspace at: {:?}", workspace_path);
    }

    Workspace::init(&workspace_path).context("Failed to initialize workspace")?;

    println!("Workspace initialized at: {:?}", workspace_path);
    println!("Config file: {:?}", Workspace::config_path()?);

    Ok(())
}
