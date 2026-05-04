//! 文件系统工具模块
//!
//! 提供文件系统底层操作的封装和抽象接口，包括：
//! - 目录递归复制
//! - 跨文件系统移动（复制+删除）
//! - 路径规范化处理
//! - 父目录自动创建
//! - 文件/目录/符号链接的安全删除
//! - 符号链接和硬链接的创建
//!
//! ## 接口设计（ISP: 接口隔离原则）
//!
//! 为遵循接口隔离原则，文件系统操作被拆分为 4 个子 trait：
//! - [`FsReader`] - 只读查询操作
//! - [`FsCopier`] - 目录复制操作
//! - [`FsWriter`] - 文件/目录写操作
//! - [`FsLinker`] - 链接创建操作
//!
//! 同时保留 [`FileSystem`] 组合 trait 以保持向后兼容。

use anyhow::{Context, Result};
use std::path::Path;
use tracing::debug;

/// 只读查询操作 trait（ISP: 接口隔离原则）
///
/// 提供文件系统的只读查询功能，适用于状态检查、路径比较等场景。
pub trait FsReader {
    /// 规范化路径（统一使用正斜杠并转为小写）
    fn normalize_path(&self, path: &Path) -> String;

    /// 读取符号链接指向的目标路径
    fn read_link(&self, path: &Path) -> Option<std::path::PathBuf>;
}

/// 目录复制操作 trait（ISP: 接口隔离原则）
///
/// 提供目录级别的复制功能，适用于目录合并、备份等场景。
pub trait FsCopier {
    /// 递归复制目录及其所有内容
    fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> Result<()>;
}

/// 文件/目录写操作 trait（ISP: 接口隔离原则）
///
/// 提供文件系统的写操作功能，包括创建、删除、移动等。
pub trait FsWriter {
    /// 跨文件系统移动（先复制再删除原位置）
    fn move_dir_cross_filesystem(&self, src: &Path, dst: &Path) -> Result<()>;

    /// 确保路径的父目录存在，不存在则创建
    fn ensure_parent_exists(&self, path: &Path) -> Result<()>;

    /// 安全删除文件、目录或符号链接
    fn remove_if_exists(&self, path: &Path) -> Result<()>;

    /// 重命名文件或目录
    fn rename(&self, src: &Path, dst: &Path) -> Result<()>;
}

/// 链接创建操作 trait（ISP: 接口隔离原则）
///
/// 提供符号链接和硬链接的创建功能。
pub trait FsLinker {
    /// 创建符号链接（自动检测目标类型选择正确的方法）
    fn create_symlink(&self, target: &Path, link: &Path) -> Result<()>;

    /// 创建硬链接
    fn hard_link(&self, target: &Path, link: &Path) -> Result<()>;
}

/// 文件系统操作组合 trait（向后兼容）
///
/// 组合了所有细粒度 trait，方便不需要精细控制的场景使用。
/// 推荐新功能优先使用子 trait 以实现更好的接口隔离。
pub trait FileSystem: FsReader + FsCopier + FsWriter + FsLinker {}

// 自动实现 FileSystem trait 给所有满足条件的类型
impl<T: FsReader + FsCopier + FsWriter + FsLinker> FileSystem for T {}

/// 文件系统操作工具类（默认实现）
pub struct FsUtils;

impl FsUtils {
    /// 删除符号链接（Windows 上区分目录/文件符号链接）
    ///
    /// 这是一个内部辅助方法，不在 FileSystem trait 中暴露。
    fn remove_symlink(path: &Path) -> Result<()> {
        #[cfg(windows)]
        {
            if std::fs::remove_dir(path).is_err() {
                std::fs::remove_file(path)?;
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            std::fs::remove_file(path)
                .with_context(|| format!("Failed to remove symlink: {:?}", path))
        }
    }
}

// === FsReader 实现 ===

impl FsReader for FsUtils {
    fn normalize_path(&self, path: &Path) -> String {
        path.to_string_lossy().replace("\\", "/").to_lowercase()
    }

    fn read_link(&self, path: &Path) -> Option<std::path::PathBuf> {
        std::fs::read_link(path).ok()
    }
}

// === FsCopier 实现 ===

impl FsCopier for FsUtils {
    fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> Result<()> {
        if !src.is_dir() {
            anyhow::bail!(
                "Source path is not a valid directory: {:?}. \n\
                 Please check your config.toml 'source' path is correct.",
                src
            );
        }

        if !dst.exists() {
            std::fs::create_dir_all(dst)
                .with_context(|| format!("Failed to create directory: {:?}", dst))?;
        }

        for entry in std::fs::read_dir(src)
            .with_context(|| format!("Failed to read directory: {:?}", src))?
        {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                self.copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)
                    .with_context(|| format!("Failed to copy {:?} to {:?}", src_path, dst_path))?;
            }
        }

        Ok(())
    }
}

// === FsWriter 实现 ===

impl FsWriter for FsUtils {
    fn move_dir_cross_filesystem(&self, src: &Path, dst: &Path) -> Result<()> {
        if src.is_file() {
            std::fs::copy(src, dst)
                .with_context(|| format!("Failed to copy file from {:?} to {:?}", src, dst))?;
            std::fs::remove_file(src)
                .with_context(|| format!("Failed to remove source file: {:?}", src))?;
        } else {
            self.copy_dir_recursive(src, dst)?;
            std::fs::remove_dir_all(src)
                .with_context(|| format!("Failed to remove source directory: {:?}", src))?;
        }
        Ok(())
    }

    fn ensure_parent_exists(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory: {:?}", parent))?;
        }
        Ok(())
    }

    fn remove_if_exists(&self, path: &Path) -> Result<()> {
        if path.is_symlink() {
            debug!("Removing symlink: {}", path.display());
            return Self::remove_symlink(path);
        }

        if !path.exists() {
            return Ok(());
        }

        if path.is_dir() {
            debug!("Removing directory: {}", path.display());
            std::fs::remove_dir_all(path)
                .with_context(|| format!("Failed to remove directory: {:?}", path))?;
        } else {
            debug!("Removing file: {}", path.display());
            std::fs::remove_file(path)
                .with_context(|| format!("Failed to remove file: {:?}", path))?;
        }
        Ok(())
    }

    fn rename(&self, src: &Path, dst: &Path) -> Result<()> {
        std::fs::rename(src, dst)
            .with_context(|| format!("Failed to rename {:?} to {:?}", src, dst))?;
        Ok(())
    }
}

// === FsLinker 实现 ===

impl FsLinker for FsUtils {
    fn create_symlink(&self, target: &Path, link: &Path) -> Result<()> {
        if link.is_symlink() {
            std::fs::remove_file(link)
                .with_context(|| format!("Failed to remove existing symlink: {:?}", link))?;
        }

        if link.exists() {
            self.remove_if_exists(link)?;
        }

        if target.is_dir() {
            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(target, link).with_context(|| {
                format!(
                    "Failed to create directory symlink at {:?} pointing to {:?}",
                    link, target
                )
            })?;

            #[cfg(not(windows))]
            std::os::unix::fs::symlink(target, link).with_context(|| {
                format!(
                    "Failed to create directory symlink at {:?} pointing to {:?}",
                    link, target
                )
            })?;
        } else {
            #[cfg(windows)]
            std::os::windows::fs::symlink_file(target, link).with_context(|| {
                format!(
                    "Failed to create file symlink at {:?} pointing to {:?}",
                    link, target
                )
            })?;

            #[cfg(not(windows))]
            std::os::unix::fs::symlink(target, link).with_context(|| {
                format!(
                    "Failed to create file symlink at {:?} pointing to {:?}",
                    link, target
                )
            })?;
        }
        Ok(())
    }

    fn hard_link(&self, target: &Path, link: &Path) -> Result<()> {
        std::fs::hard_link(target, link).with_context(|| {
            format!(
                "Failed to create hardlink at {:?} pointing to {:?}",
                link, target
            )
        })?;
        Ok(())
    }
}
