//! 链接操作模块
//!
//! 提供链接的核心功能，包括：
//! - 符号链接和硬链接的创建
//! - 链接的删除（unlink）
//! - 链接状态检查
//!
//! ## API 参数约定
//!
//! 本模块遵循统一的参数风格：
//! - **输入参数**（只读访问）：使用 `&Path` 引用
//! - **返回值**（调用者需要所有权）：使用 `PathBuf`
//! - **结构体字段**（需要存储）：使用 `PathBuf`
//!
//! ## 日志约定
//!
//! 领域层不感知 CLI `--verbose`：过程信息统一用 `tracing::debug!` 记录，
//! 由 `main.rs` 的 EnvFilter 决定是否展示（verbose 开启时 filter 降到 debug）。

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tracing::debug;

use super::link_status::{LinkStatus, LinkStatusChecker};
use super::strategies::{OnExists, OnExistsAction};
use crate::infra::FileSystem;
use crate::infra::detect_symlink_cycle;

/// 链接类型枚举
///
/// 使用 `#[serde(rename_all = "lowercase")]` 让 TOML 中可直接用小写字符串
/// 反序列化为强类型枚举，把校验前移到反序列化阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    /// 符号链接（软链接）
    Symlink,
    /// 硬链接
    Hardlink,
}

impl FromStr for LinkType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hardlink" => Ok(LinkType::Hardlink),
            "symlink" => Ok(LinkType::Symlink),
            _ => Err(format!("Unknown link type: {}", s)),
        }
    }
}

/// 链接请求结构体
pub struct LinkRequest {
    /// 源路径（原位置）
    pub source: PathBuf,
    /// 目标路径（工作区中的位置）
    pub target: PathBuf,
    /// 链接类型
    pub link_type: LinkType,
    /// 目标已存在时的处理策略
    pub on_exists: OnExists,
    /// 是否强制覆盖已存在的符号链接
    pub force: bool,
}

/// 链接操作工具类
pub struct LinkOps;

impl LinkOps {
    /// 创建链接：将源路径的内容转移到目标路径，然后在源位置创建链接指向目标
    ///
    /// ## 状态机
    ///
    /// 所有分支先归一到 **"source 不存在 + target 存在"** 的标准状态，
    /// 然后统一执行 `create_link(source, target)`：
    ///
    /// | 当前状态 | 动作 |
    /// |---------|------|
    /// | source 是指向 target 的链接 | 幂等，直接返回 |
    /// | source 是指向其他位置的链接 | force：删除后重建；否则报错 |
    /// | source 真实存在 + target 不存在 | 移动 source → target |
    /// | source 真实存在 + target 存在 | 按 `on_exists` 策略归一（skip 报错） |
    /// | source 不存在 + target 不存在 | 创建空 target 目录 |
    /// | source 不存在 + target 存在 | 无需归一，直接建链 |
    pub fn link_with_fs(request: &LinkRequest, fs: &dyn FileSystem) -> Result<()> {
        let source = request.source.as_path();
        let target = request.target.as_path();

        debug!(
            "Linking: {} -> {} (type: {:?}, on_exists: {:?}, force: {})",
            source.display(),
            target.display(),
            request.link_type,
            request.on_exists,
            request.force,
        );

        // 步骤1: 已存在的符号链接 → 幂等返回 / force 重建 / 报错
        if Self::handle_existing_symlink(source, target, request.force, fs)? {
            return Ok(());
        }

        // 步骤2: 预处理，转化为标准状态（source 不存在 + target 存在）
        Self::prepare_standard_state(source, target, request.on_exists, fs)?;

        // 步骤3: 创建链接
        Self::create_link(source, target, request.link_type, fs)
    }

    /// 处理 source 位置已存在的符号链接
    ///
    /// - source 不是链接：直接返回（进入状态归一流程）
    /// - 已正确指向 target：幂等返回 `true`（link_with_fs 应跳过后续步骤）
    fn handle_existing_symlink(
        source: &Path,
        target: &Path,
        force: bool,
        fs: &dyn FileSystem,
    ) -> Result<bool> {
        if !source.is_symlink() {
            return Ok(false);
        }

        if force {
            debug!("Force: removing existing symlink: {}", source.display());
            fs.remove_if_exists(source)?;
            return Ok(false);
        }

        if Self::points_to(source, target, fs) {
            debug!("Already linked: {}", source.display());
            return Ok(true);
        }

        anyhow::bail!(
            "Source is already a symlink pointing to different target: {:?}",
            source
        )
    }

    /// 检查符号链接是否指向预期目标（按规范化路径比较）
    fn points_to(link: &Path, target: &Path, fs: &dyn FileSystem) -> bool {
        fs.read_link(link)
            .is_some_and(|linked| fs.normalize_path(&linked) == fs.normalize_path(target))
    }

    /// 预处理：将当前状态转化为标准状态（source 不存在 + target 存在）
    fn prepare_standard_state(
        source: &Path,
        target: &Path,
        on_exists: OnExists,
        fs: &dyn FileSystem,
    ) -> Result<()> {
        // force 重建时链接已被删除，这里不存在符号链接分支
        if source.exists() {
            if target.exists() {
                // 双方都存在：由 on_exists 策略裁决
                Self::resolve_conflict(source, target, on_exists, fs)?;
            } else {
                // 仅 source 存在：直接移动
                debug!("Moving source to target (target doesn't exist)");
                fs.ensure_parent_exists(target)?;
                fs.move_path(source, target)?;
            }
        } else if !target.exists() {
            // 双方都不存在：创建空 target，保证有链接可指
            debug!("Creating target directory (source doesn't exist)");
            std::fs::create_dir_all(target)
                .with_context(|| format!("Failed to create target directory: {:?}", target))?;
        }

        Ok(())
    }

    /// 执行 on_exists 策略，把 "source 和 target 都存在" 的冲突归一为标准状态
    fn resolve_conflict(
        source: &Path,
        target: &Path,
        on_exists: OnExists,
        fs: &dyn FileSystem,
    ) -> Result<()> {
        match on_exists.execute(source, target, fs)? {
            OnExistsAction::Skip => {
                anyhow::bail!(
                    "Target already exists and on_exists strategy is 'skip'. \
                     Use a different strategy (replace/merge/preserve) or remove the target manually."
                )
            }
            OnExistsAction::ContinueWithMove => {
                // Replace 策略：target 已被删除，移动 source → target
                fs.ensure_parent_exists(target)?;
                fs.move_path(source, target)?;
            }
            OnExistsAction::ContinueWithoutMove => {
                // Merge/Preserve 策略：source 已被删除或合并，无需移动
            }
        }
        Ok(())
    }

    /// 在源位置创建指向目标的链接
    fn create_link(
        source: &Path,
        target: &Path,
        link_type: LinkType,
        fs: &dyn FileSystem,
    ) -> Result<()> {
        match link_type {
            LinkType::Symlink => {
                // 创建前检测循环：如果 target 的符号链接链可回溯到 source，则拒绝
                if let Some(cycle_path) = detect_symlink_cycle(target) {
                    anyhow::bail!(
                        "Symlink cycle detected: '{}' resolves back to the chain at '{}'. \
                         Cannot create symlink {} -> {}",
                        target.display(),
                        cycle_path.display(),
                        source.display(),
                        target.display(),
                    );
                }

                debug!(
                    "Creating symlink: {} -> {}",
                    source.display(),
                    target.display()
                );
                fs.create_symlink(target, source)?;
            }
            LinkType::Hardlink => {
                debug!(
                    "Creating hardlink: {} -> {}",
                    source.display(),
                    target.display()
                );
                fs.hard_link(target, source)?;
            }
        }

        debug!(
            "Successfully linked: {} -> {}",
            source.display(),
            target.display()
        );
        Ok(())
    }

    /// 删除链接：移除源位置的链接，可选择将目标位置的文件移回源位置
    pub fn unlink_with_fs(
        source: &Path,
        target: &Path,
        keep_files: bool,
        fs: &dyn FileSystem,
    ) -> Result<()> {
        debug!(
            "Unlinking: {} -> {} (keep_files: {})",
            source.display(),
            target.display(),
            keep_files,
        );

        if source.is_symlink() {
            fs.remove_if_exists(source)?;

            if !keep_files && target.exists() {
                super::file_mover::move_back(target, source, fs)?;
            }
        } else if source.exists() {
            anyhow::bail!("Source is not a symlink: {:?}", source);
        } else if target.exists() && !keep_files {
            // 链接已丢失但目标还在：把数据移回，避免成为孤儿
            super::file_mover::move_back(target, source, fs)?;
        }

        debug!(
            "Successfully unlinked: {} -> {}",
            source.display(),
            target.display()
        );
        Ok(())
    }

    /// 检查链接状态（委托给 [`LinkStatusChecker`]）
    pub fn check_status(source: &Path, target: &Path) -> LinkStatus {
        LinkStatusChecker::check(source, target)
    }
}
