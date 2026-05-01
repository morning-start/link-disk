//! 链接操作模块
//!
//! 提供链接的核心功能，包括：
//! - 符号链接和硬链接的创建
//! - 链接的删除（unlink）
//! - 目标已存在时的处理策略
//! - 链接状态检查
//! - 目录合并操作

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::fs_utils::FsUtils;

/// 链接状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStatus {
    /// 链接正常，源是符号链接，目标是实际文件
    Linked,
    /// 链接存在但目标文件被删除
    Broken,
    /// 源和目标都存在（源不是链接）
    BothExist,
    /// 只有源位置存在文件
    SourceOnly,
    /// 只有目标位置存在文件
    TargetOnly,
    /// 源和目标都不存在
    None,
}

impl LinkStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Linked => "linked",
            Self::Broken => "broken",
            Self::BothExist => "both_exist",
            Self::SourceOnly => "source_only",
            Self::TargetOnly => "target_only",
            Self::None => "none",
        }
    }

    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Linked)
    }

    pub fn is_broken(&self) -> bool {
        matches!(self, Self::Broken)
    }
}

/// 链接操作工具类
pub struct LinkOps;

/// 链接类型枚举
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinkType {
    /// 符号链接（软链接）
    Symlink,
    /// 硬链接
    Hardlink,
}

impl LinkType {
    /// 从字符串解析链接类型，默认为 Symlink
    pub fn from_str(s: &str) -> Self {
        match s {
            "hardlink" | "Hardlink" | "HARDLINK" => LinkType::Hardlink,
            _ => LinkType::Symlink,
        }
    }
}

/// 目标已存在时的处理策略枚举
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OnExists {
    /// 跳过，不执行任何操作
    Skip,
    /// 合并目录内容
    Merge,
    /// 覆盖源文件后重新创建链接
    Overwrite,
    /// 删除目标后移动源到目标位置
    Replace,
}

impl OnExists {
    /// 从字符串解析策略，默认为 Skip
    pub fn from_str(s: &str) -> Self {
        match s {
            "merge" | "Merge" | "MERGE" => OnExists::Merge,
            "overwrite" | "Overwrite" | "OVERWRITE" => OnExists::Overwrite,
            "replace" | "Replace" | "REPLACE" => OnExists::Replace,
            _ => OnExists::Skip,
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

impl LinkOps {
    /// 创建链接：将源路径的内容转移到目标路径，然后在源位置创建链接指向目标
    ///
    /// # 流程
    /// 1. 检查并处理已存在的符号链接（force 逻辑）
    /// 2. 根据策略处理目标已存在的情况
    /// 3. 移动源内容到目标（或准备目标目录结构）
    /// 4. 在源位置创建指向目标的链接
    pub fn link(request: &LinkRequest, verbose: bool) -> Result<()> {
        let source = &request.source;
        let target = &request.target;

        Self::log_link_request(source, target, request);

        // 步骤1: 符号链接检查与处理
        Self::check_and_handle_symlink(source, target, request.force, verbose)?;

        // 步骤2-3: 处理源/目标存在性 + 移动文件或准备目录
        if source.exists() {
            let should_continue = Self::handle_on_exists(source, target, request.on_exists, verbose)?;
            if !should_continue { return Ok(()); }

            FsUtils::ensure_parent_exists(target)?;
            FsUtils::move_dir_cross_filesystem(source, target)?;
        } else {
            Self::prepare_target_for_link(target, verbose)?;
        }

        // 步骤4: 创建链接
        Self::create_link(source, target, request.link_type, verbose)
    }

    fn log_link_request(source: &Path, target: &Path, request: &LinkRequest) {
        println!("Linking: {:?} -> {:?}", source, target);
        println!("  Source exists: {}", source.exists());
        println!("  Source is_symlink: {}", source.is_symlink());
        println!("  Target exists: {}", target.exists());
        println!("  Target is_symlink: {}", target.is_symlink());
        println!("  Force: {}", request.force);
        println!("  LinkType: {:?}", request.link_type);
    }

    /// 检查并处理已存在的符号链接
    fn check_and_handle_symlink(
        source: &Path,
        target: &Path,
        force: bool,
        verbose: bool,
    ) -> Result<()> {
        if !source.is_symlink() { return Ok(()); }

        if force {
            if verbose {
                println!("Force: removing existing symlink: {:?}", source);
            }
            return FsUtils::remove_if_exists(source, false);
        }

        if let Some(target_path) = FsUtils::read_link(source) {
            let normalized_linked = FsUtils::normalize_path(&target_path);
            let normalized_target = FsUtils::normalize_path(target);
            if normalized_linked == normalized_target {
                if verbose {
                    println!("Already linked: {:?} -> {:?}", source, target_path);
                }
                return Ok(());
            }
        }

        anyhow::bail!(
            "Source is already a symlink pointing to different target: {:?}",
            source
        )
    }

    /// 处理 on_exists 策略，返回是否需要继续执行移动和链接操作
    ///
    /// 返回值：
    /// - `Ok(true)`: 继续执行（需要移动文件）
    /// - `Ok(false)`: 跳过后续操作（Skip 策略或 Merge 后直接结束）
    fn handle_on_exists(
        source: &Path,
        target: &Path,
        strategy: OnExists,
        verbose: bool,
    ) -> Result<bool> {
        if !target.exists() { return Ok(true); }

        match strategy {
            OnExists::Skip => {
                if verbose {
                    println!("Target already exists, skipping: {:?}", target);
                }
                Ok(false)
            }
            OnExists::Replace => {
                if verbose {
                    println!("Removing existing target: {:?}", target);
                }
                FsUtils::remove_if_exists(target, verbose)?;
                Ok(true)
            }
            OnExists::Merge => {
                if verbose {
                    println!("Merging directories: {:?} -> {:?}", source, target);
                }
                Self::merge_dirs(source, target, verbose)?;
                Ok(false)
            }
            OnExists::Overwrite => {
                if verbose {
                    println!("Removing source for overwrite: {:?}", source);
                }
                FsUtils::remove_if_exists(source, verbose)?;
                Ok(true)
            }
        }
    }

    /// 当源不存在时，准备目标目录结构用于创建链接
    fn prepare_target_for_link(target: &Path, verbose: bool) -> Result<()> {
        if verbose {
            println!("Source does not exist, creating target directory structure...");
        }
        FsUtils::ensure_parent_exists(target)?;
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
        verbose: bool,
    ) -> Result<()> {
        match link_type {
            LinkType::Symlink => {
                if verbose {
                    println!("Creating symlink: {:?} -> {:?}", source, target);
                }
                FsUtils::create_symlink(target, source)?;
            }
            LinkType::Hardlink => {
                if verbose {
                    println!("Creating hardlink: {:?} -> {:?}", source, target);
                }
                FsUtils::hard_link(target, source)?;
            }
        }

        if verbose {
            println!("Successfully linked: {:?} -> {:?}", source, target);
        }

        Ok(())
    }

    /// 删除链接：移除源位置的链接，可选择将目标位置的文件移回源位置
    pub fn unlink(source: &Path, target: &Path, keep_files: bool, verbose: bool) -> Result<()> {
        if verbose {
            println!("Unlinking: {:?} -> {:?}", source, target);
        }

        if source.is_symlink() {
            FsUtils::remove_if_exists(source, false)?;

            if !keep_files && target.exists() {
                Self::move_back(target, source)?;
            }
        } else if source.exists() {
            anyhow::bail!("Source is not a symlink: {:?}", source);
        } else if target.exists() && !keep_files {
            Self::move_back(target, source)?;
        }

        if verbose {
            println!("Successfully unlinked: {:?} -> {:?}", source, target);
        }

        Ok(())
    }

    /// 合并两个目录的内容（源目录合并到目标目录）
    fn merge_dirs(source: &Path, target: &Path, verbose: bool) -> Result<()> {
        if !source.is_dir() || !target.is_dir() {
            anyhow::bail!("Merge requires both paths to be directories");
        }

        for entry in std::fs::read_dir(source)
            .with_context(|| format!("Failed to read directory: {:?}", source))?
        {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = target.join(entry.file_name());

            if src_path.is_dir() {
                Self::merge_dirs(&src_path, &dst_path, verbose)?;
            } else if !dst_path.exists() {
                std::fs::copy(&src_path, &dst_path)
                    .with_context(|| format!("Failed to copy: {:?} to {:?}", src_path, dst_path))?;
            } else if verbose {
                println!("Skipping existing file: {:?}", dst_path);
            }
        }

        FsUtils::remove_if_exists(source, verbose)?;

        Ok(())
    }

    /// 将目标位置的内容移回源位置
    fn move_back(source: &Path, target: &Path) -> Result<()> {
        if !source.exists() {
            anyhow::bail!("Target path does not exist: {:?}", source);
        }

        FsUtils::ensure_parent_exists(target)?;

        if source.is_dir() {
            FsUtils::copy_dir_recursive(source, target)?;
            FsUtils::remove_if_exists(source, false)?;
        } else {
            FsUtils::rename(source, target)?;
        }

        Ok(())
    }

    /// 检查链接状态
    ///
    /// 返回值说明：
    /// - Linked: 链接正常，目标和源都存在
    /// - Broken: 链接损坏（源是符号链接但目标不存在）
    /// - BothExist: 源和目标都存在但不是链接
    /// - SourceOnly: 只有源存在
    /// - TargetOnly: 只有目标存在
    /// - None: 都不存在
    pub fn check_status(source: &Path, target: &Path) -> LinkStatus {
        if source.is_symlink() {
            if target.exists() { LinkStatus::Linked } else { LinkStatus::Broken }
        } else if source.exists() {
            if target.exists() {
                LinkStatus::BothExist
            } else {
                LinkStatus::SourceOnly
            }
        } else if target.exists() {
            LinkStatus::TargetOnly
        } else {
            LinkStatus::None
        }
    }
}
