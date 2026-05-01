//! 目录操作模块
//!
//! 提供目录级别的原子操作，包括：
//! - 目录合并（递归合并两个目录的内容）
//! - 文件回移（将目标位置的内容移回源位置）

use anyhow::{Context, Result};
use std::path::Path;

use crate::fs_utils::FileSystem;

/// 目录操作工具类
pub struct DirOps;

impl DirOps {
    /// 合并两个目录的内容（源目录合并到目标目录）
    ///
    /// 将源目录中的所有文件和子目录递归合并到目标目录，
    /// 合并完成后删除源目录。
    ///
    /// # 参数
    /// - `source`: 源目录路径
    /// - `target`: 目标目录路径
    /// - `fs`: 文件系统操作接口
    /// - `verbose`: 是否输出详细日志
    pub fn merge_dirs(source: &Path, target: &Path, fs: &dyn FileSystem, verbose: bool) -> Result<()> {
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
                Self::merge_dirs(&src_path, &dst_path, fs, verbose)?;
            } else if !dst_path.exists() {
                std::fs::copy(&src_path, &dst_path)
                    .with_context(|| format!("Failed to copy: {:?} to {:?}", src_path, dst_path))?;
            } else if verbose {
                println!("Skipping existing file: {:?}", dst_path);
            }
        }

        fs.remove_if_exists(source, verbose)?;

        Ok(())
    }

    /// 将目标位置的内容移回源位置
    ///
    /// 用于 unlink 操作中恢复文件到原始位置。
    /// 如果目标是目录，则递归复制后删除；如果是文件，则直接重命名。
    ///
    /// # 参数
    /// - `source`: 源位置（当前文件所在位置）
    /// - `target`: 目标位置（要移动到的位置）
    /// - `fs`: 文件系统操作接口
    pub fn move_back(source: &Path, target: &Path, fs: &dyn FileSystem) -> Result<()> {
        if !source.exists() {
            anyhow::bail!("Target path does not exist: {:?}", source);
        }

        fs.ensure_parent_exists(target)?;

        if source.is_dir() {
            fs.copy_dir_recursive(source, target)?;
            fs.remove_if_exists(source, false)?;
        } else {
            fs.rename(source, target)?;
        }

        Ok(())
    }
}
