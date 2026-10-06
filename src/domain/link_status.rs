//! 链接状态检查模块
//!
//! 提供链接状态的枚举定义和检查逻辑。
//!
//! 状态查询经由 [`FileSystem`] trait，与领域层其余文件访问保持一致：
//! 真实文件系统用 `FsUtils`，测试用内存替身即可覆盖全部状态分支。

use std::path::Path;

use crate::infra::FileSystem;

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
    /// 获取状态的字符串表示
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Linked => "linked",
            Self::Broken => "broken",
            Self::BothExist => "both_exist",
            Self::SourceOnly => "source_only",
            Self::TargetOnly => "target_only",
            Self::None => "none",
        }
    }
}

/// 链接状态检查器
pub struct LinkStatusChecker;

impl LinkStatusChecker {
    /// 检查链接状态
    ///
    /// # 参数
    /// - `source`: 源路径（原位置）
    /// - `target`: 目标路径（工作区中的位置）
    /// - `fs`: 文件系统抽象（真实实现或测试替身）
    ///
    /// # 返回值
    /// 链接状态枚举值
    pub fn check(source: &Path, target: &Path, fs: &dyn FileSystem) -> LinkStatus {
        if fs.is_symlink(source) {
            if fs.exists(target) {
                LinkStatus::Linked
            } else {
                LinkStatus::Broken
            }
        } else if fs.exists(source) {
            if fs.exists(target) {
                LinkStatus::BothExist
            } else {
                LinkStatus::SourceOnly
            }
        } else if fs.exists(target) {
            LinkStatus::TargetOnly
        } else {
            LinkStatus::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::FsUtils;
    use tempfile::TempDir;

    #[test]
    fn status_none_when_both_missing() {
        let temp = TempDir::new().unwrap();
        let status =
            LinkStatusChecker::check(&temp.path().join("s"), &temp.path().join("t"), &FsUtils);
        assert_eq!(status, LinkStatus::None);
        assert_eq!(status.as_str(), "none");
    }

    #[test]
    fn status_source_only() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("s");
        std::fs::create_dir_all(&source).unwrap();
        let status = LinkStatusChecker::check(&source, &temp.path().join("t"), &FsUtils);
        assert_eq!(status, LinkStatus::SourceOnly);
    }

    #[test]
    fn status_target_only() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("t");
        std::fs::create_dir_all(&target).unwrap();
        let status = LinkStatusChecker::check(&temp.path().join("s"), &target, &FsUtils);
        assert_eq!(status, LinkStatus::TargetOnly);
    }

    #[test]
    fn status_both_exist() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("s");
        let target = temp.path().join("t");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        let status = LinkStatusChecker::check(&source, &target, &FsUtils);
        assert_eq!(status, LinkStatus::BothExist);
    }

    // 真实符号链接的两个状态（linked / broken）需要链接创建权限，
    // 在 link_ops.rs 的 MemoryFs 测试中已有无权限等价覆盖。
}
