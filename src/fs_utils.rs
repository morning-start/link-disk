//! 文件系统工具模块
//!
//! 提供文件系统底层操作的封装，包括：
//! - 目录递归复制
//! - 跨文件系统移动（复制+删除）
//! - 路径规范化处理
//! - 父目录自动创建
//! - 文件/目录/符号链接的安全删除
//! - 符号链接和硬链接的创建

use anyhow::{Context, Result};
use std::path::Path;

/// 文件系统操作工具类
pub struct FsUtils;

impl FsUtils {
    /// 递归复制目录及其所有内容
    pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
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
                Self::copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)
                    .with_context(|| format!("Failed to copy {:?} to {:?}", src_path, dst_path))?;
            }
        }

        Ok(())
    }

    /// 跨文件系统移动（先复制再删除原位置）
    pub fn move_dir_cross_filesystem(src: &Path, dst: &Path) -> Result<()> {
        if src.is_file() {
            std::fs::copy(src, dst)
                .with_context(|| format!("Failed to copy file from {:?} to {:?}", src, dst))?;
            std::fs::remove_file(src)
                .with_context(|| format!("Failed to remove source file: {:?}", src))?;
        } else {
            Self::copy_dir_recursive(src, dst)?;
            std::fs::remove_dir_all(src)
                .with_context(|| format!("Failed to remove source directory: {:?}", src))?;
        }
        Ok(())
    }

    /// 规范化路径（统一使用正斜杠并转为小写）
    pub fn normalize_path(path: &Path) -> String {
        path.to_string_lossy().replace("\\", "/").to_lowercase()
    }

    /// 确保路径的父目录存在，不存在则创建
    pub fn ensure_parent_exists(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory: {:?}", parent))?;
        }
        Ok(())
    }

    /// 安全删除文件、目录或符号链接
    pub fn remove_if_exists(path: &Path, verbose: bool) -> Result<()> {
        if path.is_symlink() {
            if verbose {
                println!("Removing symlink: {:?}", path);
            }
            return Self::remove_symlink(path);
        }

        if !path.exists() {
            return Ok(());
        }

        if path.is_dir() {
            if verbose {
                println!("Removing directory: {:?}", path);
            }
            std::fs::remove_dir_all(path)
                .with_context(|| format!("Failed to remove directory: {:?}", path))?;
        } else {
            if verbose {
                println!("Removing file: {:?}", path);
            }
            std::fs::remove_file(path)
                .with_context(|| format!("Failed to remove file: {:?}", path))?;
        }
        Ok(())
    }

    /// 删除符号链接（Windows 上区分目录/文件符号链接）
    fn remove_symlink(path: &Path) -> Result<()> {
        #[cfg(windows)]
        {
            // Windows 上：先尝试 remove_dir（目录符号链接），失败则尝试 remove_file（文件符号链接）
            if std::fs::remove_dir(path).is_err() {
                std::fs::remove_file(path)?;
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            // Unix 系统上统一使用 remove_file
            std::fs::remove_file(path)
                .with_context(|| format!("Failed to remove symlink: {:?}", path))
        }
    }

    /// 重命名文件或目录
    pub fn rename(src: &Path, dst: &Path) -> Result<()> {
        std::fs::rename(src, dst)
            .with_context(|| format!("Failed to rename {:?} to {:?}", src, dst))?;
        Ok(())
    }

    /// 创建符号链接（自动检测目标类型选择正确的方法）
    pub fn create_symlink(target: &Path, link: &Path) -> Result<()> {
        if link.is_symlink() {
            std::fs::remove_file(link)
                .with_context(|| format!("Failed to remove existing symlink: {:?}", link))?;
        }

        if link.exists() {
            Self::remove_if_exists(link, false)?;
        }

        if target.is_dir() {
            // 创建目录符号链接
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
            // 创建文件符号链接
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

    /// 读取符号链接指向的目标路径
    pub fn read_link(path: &Path) -> Option<std::path::PathBuf> {
        std::fs::read_link(path).ok()
    }

    /// 创建硬链接
    pub fn hard_link(target: &Path, link: &Path) -> Result<()> {
        std::fs::hard_link(target, link).with_context(|| {
            format!(
                "Failed to create hardlink at {:?} pointing to {:?}",
                link, target
            )
        })?;
        Ok(())
    }
}
