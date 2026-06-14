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
//! ## 接口设计（单一 trait）
//!
//! 使用统一的 [`FileSystem`] trait 包含所有文件系统操作，
//! 由 [`FsUtils`] 提供默认实现。

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;
use tracing::debug;

/// 符号链接循环检测的最大深度
pub const MAX_SYMLINK_DEPTH: usize = 64;

/// 检测路径是否存在符号链接循环
///
/// 通过追踪符号链接链，检测是否形成循环。
/// 如果超过 `MAX_SYMLINK_DEPTH` 层，视为循环。
///
/// # 返回值
/// - `None`: 无循环且路径正常
/// - `Some(cycle_path)`: 发现循环，返回形成循环的路径
pub fn detect_symlink_cycle(path: &Path) -> Option<std::path::PathBuf> {
    let mut visited = HashSet::new();
    let mut current = path.to_path_buf();

    for _ in 0..MAX_SYMLINK_DEPTH {
        if !current.is_symlink() {
            return None;
        }

        // 使用 fs::read_link 获取原始目标，避免 canonicalize 的跨平台问题
        let target = match std::fs::read_link(&current) {
            Ok(t) => t,
            Err(_) => return None,
        };

        // 解析为目标物理路径（非 canonicalize，避免 Windows 限制）
        let resolved = if target.is_absolute() {
            target
        } else {
            match current.parent() {
                Some(parent) => parent.join(&target),
                None => target,
            }
        };

        // 用 resolved 路径（尽量规范化）检测访问过的路径
        if !visited.insert(resolved.clone()) {
            return Some(current);
        }

        current = resolved;
    }

    Some(current)
}

/// 文件系统操作 trait
///
/// 包含所有文件系统操作，由 [`FsUtils`] 提供默认实现。
pub trait FileSystem {
    /// 规范化路径（统一使用正斜杠并转为小写）
    fn normalize_path(&self, path: &Path) -> String;

    /// 读取符号链接指向的目标路径
    fn read_link(&self, path: &Path) -> Option<std::path::PathBuf>;

    /// 递归复制目录及其所有内容
    fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> Result<()>;

    /// 跨文件系统移动（先复制再删除原位置）
    fn move_dir_cross_filesystem(&self, src: &Path, dst: &Path) -> Result<()>;

    /// 确保路径的父目录存在，不存在则创建
    fn ensure_parent_exists(&self, path: &Path) -> Result<()>;

    /// 安全删除文件、目录或符号链接
    fn remove_if_exists(&self, path: &Path) -> Result<()>;

    /// 重命名文件或目录
    fn rename(&self, src: &Path, dst: &Path) -> Result<()>;

    /// 创建符号链接（自动检测目标类型选择正确的方法）
    fn create_symlink(&self, target: &Path, link: &Path) -> Result<()>;

    /// 创建硬链接
    fn hard_link(&self, target: &Path, link: &Path) -> Result<()>;
}

/// 文件系统操作工具类（默认实现）
pub struct FsUtils;

impl FsUtils {
    /// 删除符号链接（Windows 上区分目录/文件符号链接）
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

impl FileSystem for FsUtils {
    fn normalize_path(&self, path: &Path) -> String {
        let normalized = path.to_string_lossy().replace("\\", "/");
        #[cfg(windows)]
        {
            normalized.to_lowercase()
        }
        #[cfg(not(windows))]
        {
            normalized
        }
    }

    fn read_link(&self, path: &Path) -> Option<std::path::PathBuf> {
        std::fs::read_link(path).ok()
    }

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

    fn create_symlink(&self, target: &Path, link: &Path) -> Result<()> {
        if link.exists() || link.is_symlink() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_no_cycle_on_regular_path() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("realdir");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(detect_symlink_cycle(&dir).is_none());
    }

    // Windows 上创建符号链接默认需要管理员或开发者模式
    // （错误码 1314："客户端没有所需的特权"），无特权的 CI 上标记为 ignored，
    // 拥有权限时执行 `cargo test -- --ignored` 可跑完整链路。
    #[cfg_attr(
        windows,
        ignore = "requires admin or developer mode to create symlinks"
    )]
    #[test]
    fn test_detect_no_cycle_on_valid_symlink() {
        let temp = TempDir::new().unwrap();
        let real = temp.path().join("real");
        let link = temp.path().join("link");
        std::fs::create_dir_all(&real).unwrap();
        std::os::windows::fs::symlink_dir(&real, &link).unwrap();
        assert!(detect_symlink_cycle(&link).is_none());
    }

    #[cfg_attr(
        windows,
        ignore = "requires admin or developer mode to create symlinks"
    )]
    #[test]
    fn test_detect_simple_cycle() {
        let temp = TempDir::new().unwrap();
        let a = temp.path().join("a");
        let b = temp.path().join("b");
        std::os::windows::fs::symlink_dir(&b, &a).unwrap();
        std::os::windows::fs::symlink_dir(&a, &b).unwrap();
        assert!(detect_symlink_cycle(&a).is_some());
    }

    #[cfg_attr(
        windows,
        ignore = "requires admin or developer mode to create symlinks"
    )]
    #[test]
    fn test_detect_three_link_cycle() {
        let temp = TempDir::new().unwrap();
        let a = temp.path().join("a");
        let b = temp.path().join("b");
        let c = temp.path().join("c");
        std::os::windows::fs::symlink_dir(&b, &a).unwrap();
        std::os::windows::fs::symlink_dir(&c, &b).unwrap();
        std::os::windows::fs::symlink_dir(&a, &c).unwrap();
        assert!(detect_symlink_cycle(&a).is_some());
    }

    #[cfg_attr(
        windows,
        ignore = "requires admin or developer mode to create symlinks"
    )]
    #[test]
    fn test_detect_no_cycle_on_broken_symlink() {
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("nonexistent");
        let link = temp.path().join("link");
        std::os::windows::fs::symlink_dir(&nonexistent, &link).unwrap();
        assert!(detect_symlink_cycle(&link).is_none());
    }
}
