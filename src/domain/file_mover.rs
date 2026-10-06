//! 文件移动模块
//!
//! 提供文件/目录的移动和合并操作，包括：
//! - 目录合并（BFS 迭代实现，避免栈溢出）
//! - 文件移回（用于 unlink 操作恢复文件）

use std::collections::VecDeque;
use std::path::Path;

use anyhow::{Context, Result};

use crate::infra::FileSystem;

/// 合并两个目录的内容（源目录合并到目标目录）
///
/// 将源目录中的所有文件和子目录迭代合并到目标目录，
/// 合并完成后删除源目录。
///
/// 使用 BFS 迭代实现，避免深目录导致的栈溢出。
///
/// 注意：目录遍历与文件复制经由 `std::fs` 直接执行
/// （`DirEntry` 是具体类型，无法抽象进 [`FileSystem`] trait），
/// 因此 merge 路径的测试使用真实临时目录而非内存替身。
///
/// # 参数
/// - `source`: 源目录路径
/// - `target`: 目标目录路径
/// - `fs`: 文件系统操作接口
pub fn merge_dirs(source: &Path, target: &Path, fs: &dyn FileSystem) -> Result<()> {
    if !fs.is_dir(source) || !fs.is_dir(target) {
        anyhow::bail!("Merge requires both paths to be directories");
    }

    let mut queue = VecDeque::new();
    queue.push_back((source.to_path_buf(), target.to_path_buf()));

    while let Some((src_dir, dst_dir)) = queue.pop_front() {
        if !fs.exists(&dst_dir) {
            fs.create_dir_all(&dst_dir)?;
        }

        for entry in std::fs::read_dir(&src_dir)
            .with_context(|| format!("Failed to read directory: {:?}", src_dir))?
        {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst_dir.join(entry.file_name());

            if fs.is_dir(&src_path) {
                queue.push_back((src_path, dst_path));
            } else if !fs.exists(&dst_path) {
                std::fs::copy(&src_path, &dst_path)
                    .with_context(|| format!("Failed to copy: {:?} to {:?}", src_path, dst_path))?;
            } else {
                // dst 已存在：保留 target 现有版本，跳过 source 中的同名文件。
                // 这是 merge 策略的安全默认行为（不覆盖既有数据），
                // 但需明确提示，避免用户误以为 source 全部内容都已合并。
                tracing::warn!(
                    "Merge skipped conflicting file (target retained): {}",
                    src_path.display()
                );
            }
        }
    }

    fs.remove_if_exists(source)?;

    Ok(())
}

/// 将目标位置的内容移回源位置
///
/// 用于 unlink 操作恢复文件到原始位置。
/// 如果目标是目录，则递归复制后删除；如果是文件，则直接重命名。
///
/// # 参数
/// - `source`: 源位置（当前文件所在位置）
/// - `target`: 目标位置（要移动到的位置）
/// - `fs`: 文件系统操作接口
pub fn move_back(source: &Path, target: &Path, fs: &dyn FileSystem) -> Result<()> {
    if !fs.exists(source) {
        anyhow::bail!("Source path does not exist: {:?}", source);
    }

    fs.ensure_parent_exists(target)?;

    if fs.is_dir(source) {
        fs.move_path(source, target)?;
    } else {
        fs.rename(source, target)?;
    }

    Ok(())
}
