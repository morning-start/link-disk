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
    /// 支持通过 FileSystem trait 注入不同的文件系统实现，便于测试。
    ///
    /// # 流程
    /// 1. 检查并处理已存在的符号链接（force 逻辑）
    /// 2. 根据策略处理目标已存在的情况
    /// 3. 移动源内容到目标（或准备目标目录结构）
    /// 4. 在源位置创建指向目标的链接
    pub fn link_with_fs(request: &LinkRequest, fs: &dyn FileSystem, verbose: bool) -> Result<()> {
        let source = &request.source;
        let target = &request.target;

        Self::log_link_request(source, target, request);

        // 步骤1: 符号链接检查与处理
        Self::check_and_handle_symlink(source, target, request.force, fs, verbose)?;

        // 步骤2-3: 处理源/目标存在性 + 移动文件或准备目录
        if source.exists() {
            let should_move = Self::handle_on_exists(source, target, request.on_exists, fs, verbose)?;
            if should_move {
                fs.ensure_parent_exists(target)?;
                fs.move_dir_cross_filesystem(source, target)?;
            }
        } else {
            Self::prepare_target_for_link(target, fs, verbose)?;
        }

        // 步骤4: 创建链接（始终执行）
        Self::create_link(source, target, request.link_type, fs, verbose)
    }

    fn log_link_request(source: &Path, target: &Path, request: &LinkRequest) {
        info!("Linking: {:?} -> {:?}", source, target);
        debug!("Source exists: {}", source.exists());
        debug!("Source is_symlink: {}", source.is_symlink());
        debug!("Target exists: {}", target.exists());
        debug!("Target is_symlink: {}", target.is_symlink());
        debug!("Force: {}", request.force);
        debug!("LinkType: {:?}", request.link_type);
    }

    /// 检查并处理已存在的符号链接
    fn check_and_handle_symlink(
        source: &Path,
        target: &Path,
        force: bool,
        fs: &dyn FileSystem,
        verbose: bool,
    ) -> Result<()> {
        if !source.is_symlink() { return Ok(()); }

        if force {
            if verbose {
                info!("Force: removing existing symlink: {:?}", source);
            }
            return fs.remove_if_exists(source);
        }

        if let Some(target_path) = fs.read_link(source) {
            let normalized_linked = fs.normalize_path(&target_path);
            let normalized_target = fs.normalize_path(target);
            if normalized_linked == normalized_target {
                if verbose {
                    info!("Already linked: {:?} -> {:?}", source, target_path);
                }
                return Ok(());
            }
        }

        anyhow::bail!(
            "Source is already a symlink pointing to different target: {:?}",
            source
        )
    }

    /// 处理 on_exists 策略（使用策略模式）
    ///
    /// 通过 OnExistsStrategy trait 实现开放封闭原则，
    /// 添加新策略无需修改此方法。
    ///
    /// 返回: 是否需要将源文件移动到目标位置
    fn handle_on_exists(
        source: &Path,
        target: &Path,
        on_exists: OnExists,
        fs: &dyn FileSystem,
        verbose: bool,
    ) -> Result<bool> {
        if !target.exists() { return Ok(true); }

        let strategy = on_exists.strategy();
        match strategy.execute(source, target, fs, verbose)? {
            OnExistsAction::Skip => Ok(false),
            OnExistsAction::ContinueWithMove => Ok(true),
            OnExistsAction::ContinueWithoutMove => Ok(false),
        }
    }

    /// 当源不存在时，准备目标目录结构用于创建链接
    fn prepare_target_for_link(target: &Path, fs: &dyn FileSystem, verbose: bool) -> Result<()> {
        if verbose {
            info!("Source does not exist, creating target directory structure...");
        }
        fs.ensure_parent_exists(target)?;
        if !target.exists() {
            std::fs::create_dir_all(target)
                .with_context(|| format!("Failed to create target directory: {:?}", target))?;
        }
        Ok(())
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
                    info!("Creating symlink: {:?} -> {:?}", source, target);
                }
                fs.create_symlink(target, source)?;
            }
            LinkType::Hardlink => {
                if verbose {
                    info!("Creating hardlink: {:?} -> {:?}", source, target);
                }
                fs.hard_link(target, source)?;
            }
        }

        if verbose {
            info!("Successfully linked: {:?} -> {:?}", source, target);
        }

        Ok(())
    }

    /// 删除链接：移除源位置的链接，可选择将目标位置的文件移回源位置
    pub fn unlink_with_fs(source: &Path, target: &Path, keep_files: bool, fs: &dyn FileSystem) -> Result<()> {
        info!("Unlinking: {:?} -> {:?}", source, target);
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

        info!("Successfully unlinked: {:?} -> {:?}", source, target);
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
