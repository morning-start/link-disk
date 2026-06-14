//! 策略模式模块
//!
//! 提供 on_exists 策略的定义和实现。
//!
//! ## 设计说明
//!
//! 策略行为直接以 `OnExists::execute()` 方法实现，每个变体对应一个 match 分支。
//! 4 个策略变体的行为差异清晰、字段固定，无需引入 trait + 注册表的间接层。

use std::path::Path;
use std::str::FromStr;

use anyhow::Result;
use serde::Deserialize;

use crate::domain::file_mover;
use crate::infra::FileSystem;

/// 策略名称常量模块
///
/// 定义所有支持的策略名称常量，拼写错误可在编译时捕获。
pub mod constants {
    /// 跳过策略
    pub const SKIP: &str = "skip";
    /// 替换策略
    pub const REPLACE: &str = "replace";
    /// 合并策略
    pub const MERGE: &str = "merge";
    /// 保留策略（保留 target 数据，删除 source 后创建链接）
    pub const PRESERVE: &str = "preserve";
    /// 向后兼容别名：旧版 overwrite 等价于 preserve
    pub const OVERWRITE_ALIAS: &str = "overwrite";
}

/// 策略执行结果：指示主流程如何继续
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OnExistsAction {
    /// 跳过后续操作
    Skip,
    /// 继续移动源文件到目标
    ContinueWithMove,
    /// 继续但不移动文件（如 Merge 后直接创建链接）
    ContinueWithoutMove,
}

/// 目标已存在时的处理策略枚举
///
/// 使用 `#[serde(rename_all = "lowercase")]` 让 TOML 中可直接用小写字符串
/// 反序列化为强类型枚举，把校验前移到反序列化阶段。
///
/// 向后兼容：`Preserve` 额外接受 `overwrite` 别名（旧版配置可继续使用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnExists {
    /// 跳过，不执行任何操作
    Skip,
    /// 合并目录内容
    Merge,
    /// 保留策略：保留 target 数据，删除 source 后创建链接
    #[serde(alias = "overwrite")]
    Preserve,
    /// 替换策略：删除 target 后移动 source 到 target 位置
    Replace,
}

impl Default for OnExists {
    /// 默认策略为 Skip（与配置默认值一致）
    fn default() -> Self {
        OnExists::Skip
    }
}

impl FromStr for OnExists {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            constants::SKIP => Ok(OnExists::Skip),
            constants::REPLACE => Ok(OnExists::Replace),
            constants::MERGE => Ok(OnExists::Merge),
            constants::PRESERVE | constants::OVERWRITE_ALIAS => Ok(OnExists::Preserve),
            _ => Err(format!("Unknown on_exists strategy: {}", s)),
        }
    }
}

impl OnExists {
    /// 执行策略逻辑，返回行动指令
    ///
    /// 每个变体对应一个分支，行为差异清晰。
    /// - `Skip`: 直接返回 `Skip`，主流程据此中断
    /// - `Replace`: 删除 target，返回 `ContinueWithMove`
    /// - `Merge`: 合并 source 到 target 后删除 source，返回 `ContinueWithoutMove`
    /// - `Preserve`: 保留 target，删除 source，返回 `ContinueWithoutMove`
    pub fn execute(
        self,
        source: &Path,
        target: &Path,
        fs: &dyn FileSystem,
        verbose: bool,
    ) -> Result<OnExistsAction> {
        match self {
            OnExists::Skip => {
                if verbose {
                    tracing::info!("Target already exists, skipping: {}", target.display());
                }
                Ok(OnExistsAction::Skip)
            }
            OnExists::Replace => {
                if verbose {
                    tracing::info!("Removing existing target: {}", target.display());
                }
                fs.remove_if_exists(target)?;
                Ok(OnExistsAction::ContinueWithMove)
            }
            OnExists::Merge => {
                if verbose {
                    tracing::info!(
                        "Merging directories: {} -> {}",
                        source.display(),
                        target.display()
                    );
                }
                file_mover::merge_dirs(source, target, fs).map_err(|e| {
                    anyhow::anyhow!(
                        "Merge strategy failed for {:?} -> {:?}. \n\
                         This may be caused by:\n\
                         - Source is not a valid directory\n\
                         - Source directory is corrupted or inaccessible\n\
                         Please check your config.toml 'on_exists' setting or fix the source path.\n\
                         Original error: {}",
                        source, target, e
                    )
                })?;
                Ok(OnExistsAction::ContinueWithoutMove)
            }
            OnExists::Preserve => {
                if verbose {
                    tracing::info!("Preserving target, removing source: {}", source.display());
                }
                fs.remove_if_exists(source)?;
                Ok(OnExistsAction::ContinueWithoutMove)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::FsUtils;
    use tempfile::TempDir;

    fn setup_test_env() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        (temp, source, target)
    }

    // === Skip 策略测试 ===

    #[test]
    fn test_skip_strategy_returns_skip() {
        let (_temp, source, target) = setup_test_env();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();

        let result = OnExists::Skip
            .execute(&source, &target, &FsUtils, false)
            .unwrap();
        assert_eq!(result, OnExistsAction::Skip);
        assert!(target.exists());
    }

    // === Replace 策略测试 ===

    #[test]
    fn test_replace_strategy_removes_target() {
        let (_temp, source, target) = setup_test_env();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();

        let result = OnExists::Replace
            .execute(&source, &target, &FsUtils, false)
            .unwrap();
        assert_eq!(result, OnExistsAction::ContinueWithMove);
        assert!(!target.exists());
    }

    #[test]
    fn test_replace_strategy_removes_file_target() {
        let (_temp, source, target) = setup_test_env();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(&target, "existing file").unwrap();

        let result = OnExists::Replace
            .execute(&source, &target, &FsUtils, false)
            .unwrap();
        assert_eq!(result, OnExistsAction::ContinueWithMove);
        assert!(!target.exists());
    }

    // === Preserve 策略测试 ===

    #[test]
    fn test_preserve_strategy_removes_source() {
        let (_temp, source, target) = setup_test_env();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();

        let result = OnExists::Preserve
            .execute(&source, &target, &FsUtils, false)
            .unwrap();
        assert_eq!(result, OnExistsAction::ContinueWithoutMove);
        assert!(!source.exists());
    }

    // === Merge 策略测试 ===

    #[test]
    fn test_merge_strategy_merges_files() {
        let (_temp, source, target) = setup_test_env();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(source.join("file1.txt"), "content1").unwrap();
        std::fs::write(source.join("file2.txt"), "content2").unwrap();

        let result = OnExists::Merge
            .execute(&source, &target, &FsUtils, false)
            .unwrap();
        assert_eq!(result, OnExistsAction::ContinueWithoutMove);
        assert!(target.join("file1.txt").exists());
        assert!(target.join("file2.txt").exists());
        assert_eq!(
            std::fs::read_to_string(target.join("file1.txt")).unwrap(),
            "content1"
        );
    }

    #[test]
    fn test_merge_strategy_source_removed_after_merge() {
        let (_temp, source, target) = setup_test_env();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(source.join("data.txt"), "data").unwrap();

        OnExists::Merge
            .execute(&source, &target, &FsUtils, false)
            .unwrap();
        assert!(!source.exists());
    }

    #[test]
    fn test_merge_strategy_preserves_existing_target_files() {
        let (_temp, source, target) = setup_test_env();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("existing.txt"), "existing").unwrap();
        std::fs::write(source.join("new.txt"), "new").unwrap();

        OnExists::Merge
            .execute(&source, &target, &FsUtils, false)
            .unwrap();

        assert!(target.join("existing.txt").exists());
        assert!(target.join("new.txt").exists());
        assert_eq!(
            std::fs::read_to_string(target.join("existing.txt")).unwrap(),
            "existing"
        );
    }

    // === OnExists 解析与默认值测试 ===

    #[test]
    fn test_on_exists_unknown_strategy_returns_err() {
        let result = OnExists::from_str("unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_on_exists_default_is_skip() {
        assert_eq!(OnExists::default(), OnExists::Skip);
    }
}
