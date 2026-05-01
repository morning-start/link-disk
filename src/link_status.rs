//! 链接状态检查模块
//!
//! 提供链接状态的枚举定义和检查逻辑。

use std::path::Path;

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

    /// 检查链接是否有效（状态为 Linked）
    #[allow(dead_code)]
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Linked)
    }

    /// 检查链接是否损坏（状态为 Broken）
    #[allow(dead_code)]
    pub fn is_broken(&self) -> bool {
        matches!(self, Self::Broken)
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
    ///
    /// # 返回值
    /// 链接状态枚举值
    pub fn check(source: &Path, target: &Path) -> LinkStatus {
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
