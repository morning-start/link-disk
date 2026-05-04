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
//! ### 公开方法签名示例
//! ```rust,ignore
//! // ✅ 正确：输入参数使用 &Path
//! pub fn unlink(source: &Path, target: &Path, ...) -> Result<()>;
//! pub fn check_status(source: &Path, target: &Path) -> LinkStatus;
//!
//! // ✅ 正确：结构体字段使用 PathBuf（需要所有权）
//! pub struct LinkRequest {
//!     pub source: PathBuf,
//!     pub target: PathBuf,
//! }
//! ```

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tracing::{info, debug};

use crate::infra::FileSystem;
use super::link_status::{LinkStatus, LinkStatusChecker};
use super::strategies::{OnExists, OnExistsAction};

/// 链接类型枚举
#[derive(Debug, Clone, Copy, PartialEq)]
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

impl LinkType {
    /// 宽松解析：解析失败时默认为 Symlink
    pub fn from_str_lossy(s: &str) -> Self {
        <Self as FromStr>::from_str(s).unwrap_or(LinkType::Symlink)
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
    /// # 核心设计思路
    ///
    /// 所有情况的最终目标都是转化为 **"source 不存在 + target 存在"** 的标准状态，
    /// 然后直接执行 `create_link(source, target)`。
    ///
    /// # 流程
    /// 1. 检查符号链接：如果已正确链接则直接返回
    /// 2. 预处理：通过各种手段转化为标准状态
    ///    - source 存在 + target 不存在：移动 source → target
    ///    - source 存在 + target 存在：根据 on_exists 策略处理
    ///    - source 不存在 + target 不存在：创建 target 目录
    ///    - source 不存在 + target 存在：已是标准状态，无需操作
    /// 3. 创建链接：在 source 位置创建指向 target 的链接
    pub fn link_with_fs(request: &LinkRequest, fs: &dyn FileSystem, verbose: bool) -> Result<()> {
        let source = &request.source;
        let target = &request.target;

        Self::log_link_request(source, target, request);

        // 步骤1: 检查符号链接（如果已正确链接则直接返回）
        if Self::check_and_handle_symlink(source, target, request.force, fs, verbose)? {
            return Ok(());
        }

        // 步骤2: 预处理，转化为标准状态（source 不存在 + target 存在）
        Self::prepare_standard_state(source, target, request.on_exists, fs, verbose)?;

        // 步骤3: 创建链接
        Self::create_link(source, target, request.link_type, fs, verbose)
    }

    /// 预处理：将当前状态转化为标准状态（source 不存在 + target 存在）
    ///
    /// # 状态转化表
    /// | 当前状态 | 操作 | 转化结果 |
    /// |---------|------|---------|
    /// | source 存在 + target 不存在 | 移动 source → target | source 不存在 + target 存在 |
    /// | source 存在 + target 存在 + replace | 删除 target，移动 source → target | source 不存在 + target 存在 |
    /// | source 存在 + target 存在 + merge | 合并 source 到 target 后删除 source | source 不存在 + target 存在 |
    /// | source 存在 + target 存在 + overwrite | 删除 source | source 不存在 + target 存在 |
    /// | source 存在 + target 存在 + skip | 抛出错误（跳过） | 不继续 |
    /// | source 不存在 + target 不存在 | 创建 target 目录 | source 不存在 + target 存在 |
    /// | source 不存在 + target 存在 | 无需操作（已是标准状态） | source 不存在 + target 存在 |
    fn prepare_standard_state(
        source: &Path,
        target: &Path,
        on_exists: OnExists,
        fs: &dyn FileSystem,
        verbose: bool,
    ) -> Result<()> {
        if source.exists() && !source.is_symlink() {
            // source 存在：需要根据 target 是否存在进行不同处理
            if !target.exists() {
                // source 存在 + target 不存在：直接移动
                if verbose {
                    info!("Moving source to target (target doesn't exist)");
                }
                fs.ensure_parent_exists(target)?;
                fs.move_dir_cross_filesystem(source, target)?;
            } else {
                // source 存在 + target 存在：执行 on_exists 策略
                Self::apply_on_exists_strategy(source, target, on_exists, fs, verbose)?;
            }
        } else {
            // source 不存在：只需确保 target 存在
            if !target.exists() {
                if verbose {
                    info!("Creating target directory (source doesn't exist)");
                }
                std::fs::create_dir_all(target)
                    .with_context(|| format!("Failed to create target directory: {:?}", target))?;
            }
        }
        Ok(())
    }

    /// 应用 on_exists 策略处理 source 和 target 都存在的冲突
    ///
    /// # 策略行为
    /// - Replace: 删除 target → 移动 source → target
    /// - Merge: 合并 source 到 target → 删除 source
    /// - Overwrite: 删除 source
    /// - Skip: 返回错误，中断流程
    fn apply_on_exists_strategy(
        source: &Path,
        target: &Path,
        on_exists: OnExists,
        fs: &dyn FileSystem,
        verbose: bool,
    ) -> Result<()> {
        let strategy = on_exists.strategy();
        match strategy.execute(source, target, fs, verbose)? {
            OnExistsAction::Skip => {
                anyhow::bail!(
                    "Target already exists and on_exists strategy is 'skip'. \
                     Use a different strategy (replace/merge/overwrite) or remove the target manually."
                )
            }
            OnExistsAction::ContinueWithMove => {
                // Replace 策略：target 已被删除，移动 source → target
                fs.ensure_parent_exists(target)?;
                fs.move_dir_cross_filesystem(source, target)?;
            }
            OnExistsAction::ContinueWithoutMove => {
                // Merge/Overwrite 策略：source 已被删除或合并，无需移动
            }
        }
        Ok(())
    }

    fn log_link_request(source: &Path, target: &Path, request: &LinkRequest) {
        debug!("Linking: {} -> {}", source.display(), target.display());
        debug!("Source exists: {}", source.exists());
        debug!("Source is_symlink: {}", source.is_symlink());
        debug!("Target exists: {}", target.exists());
        debug!("Target is_symlink: {}", target.is_symlink());
        debug!("Force: {}", request.force);
        debug!("LinkType: {:?}", request.link_type);
    }

    /// 检查并处理已存在的符号链接
    ///
    /// 返回 `true` 表示已正确处理完成（应跳过后续步骤），
    /// 返回 `false` 表示需要继续执行后续步骤。
    fn check_and_handle_symlink(
        source: &Path,
        target: &Path,
        force: bool,
        fs: &dyn FileSystem,
        verbose: bool,
    ) -> Result<bool> {
        if !source.is_symlink() { return Ok(false); }

        if force {
            if verbose {
                info!("Force: removing existing symlink: {}", source.display());
            }
            fs.remove_if_exists(source)?;
            return Ok(false);
        }

        if let Some(target_path) = fs.read_link(source) {
            let normalized_linked = fs.normalize_path(&target_path);
            let normalized_target = fs.normalize_path(target);
            if normalized_linked == normalized_target {
                if verbose {
                    info!("Already linked: {} -> {}", source.display(), target_path.display());
                }
                return Ok(true);
            }
        }

        anyhow::bail!(
            "Source is already a symlink pointing to different target: {:?}",
            source
        )
    }

    /// 在源位置创建指向目标的链接
    fn create_link(
        source: &Path,
        target: &Path,
        link_type: LinkType,
        fs: &dyn FileSystem,
        verbose: bool,
    ) -> Result<()> {
        match link_type {
            LinkType::Symlink => {
                if verbose {
                    info!("Creating symlink: {} -> {}", source.display(), target.display());
                }
                fs.create_symlink(target, source)?;
            }
            LinkType::Hardlink => {
                if verbose {
                    info!("Creating hardlink: {} -> {}", source.display(), target.display());
                }
                fs.hard_link(target, source)?;
            }
        }

        if verbose {
            info!("Successfully linked: {} -> {}", source.display(), target.display());
        }

        Ok(())
    }

    /// 删除链接：移除源位置的链接，可选择将目标位置的文件移回源位置
    pub fn unlink_with_fs(source: &Path, target: &Path, keep_files: bool, fs: &dyn FileSystem) -> Result<()> {
        debug!("Unlinking: {} -> {}", source.display(), target.display());
        debug!("Keep files: {}", keep_files);

        if source.is_symlink() {
            fs.remove_if_exists(source)?;

            if !keep_files && target.exists() {
                Self::move_back(target, source, fs)?;
            }
        } else if source.exists() {
            anyhow::bail!("Source is not a symlink: {:?}", source);
        } else if target.exists() && !keep_files {
            Self::move_back(target, source, fs)?;
        }

        debug!("Successfully unlinked: {} -> {}", source.display(), target.display());
        Ok(())
    }

    /// 将目标位置的内容移回源位置（委托给 file_mover 模块）
    fn move_back(source: &Path, target: &Path, fs: &dyn FileSystem) -> Result<()> {
        super::file_mover::move_back(source, target, fs)
    }

    /// 检查链接状态（委托给 LinkStatusChecker）
    ///
    /// 返回值说明：
    /// - Linked: 链接正常，目标和源都存在
    /// - Broken: 链接损坏（源是符号链接但目标不存在）
    /// - BothExist: 源和目标都存在但不是链接
    /// - SourceOnly: 只有源存在
    /// - TargetOnly: 只有目标存在
    /// - None: 都不存在
    pub fn check_status(source: &Path, target: &Path) -> LinkStatus {
        LinkStatusChecker::check(source, target)
    }
}
