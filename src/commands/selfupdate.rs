//! 自我升级命令处理

use anyhow::Result;

use crate::cli::{CliContext, SelfupdateArgs};
use crate::commands::Command;
use crate::infra::selfupdate;

/// Selfupdate 子命令实现
impl Command for SelfupdateArgs {
    fn execute(&self, _ctx: &CliContext) -> Result<()> {
        if self.check {
            return self.check_only();
        }

        let use_gh = !self.no_gh;
        match self.version.as_deref() {
            Some(v) => println!("Updating link-disk to {v} ..."),
            None => println!("Checking for updates ..."),
        }

        let message = selfupdate::run_update(self.version.as_deref(), use_gh, self.force)?;
        println!("{message}");
        Ok(())
    }
}

impl SelfupdateArgs {
    /// `--check`：仅查询最新版本并给出升级建议，不做任何改动
    fn check_only(&self) -> Result<()> {
        let release = selfupdate::fetch_latest_release(!self.no_gh)?;
        let current = format!("v{}", selfupdate::CURRENT_VERSION);
        let latest = &release.tag;

        match selfupdate::compare_versions(&current, latest) {
            std::cmp::Ordering::Less => {
                println!("Update available: {current} -> {latest}");
                println!("Run 'link-disk selfupdate' to upgrade.");
            }
            _ => println!("Already up to date ({current}; latest is {latest})."),
        }
        Ok(())
    }
}
