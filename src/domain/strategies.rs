//! 策略模式模块
//!
//! 提供 on_exists 策略的定义和实现，符合开放封闭原则（OCP）。
//! 添加新策略只需实现 OnExistsStrategy trait 并在注册表中注册。

use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::sync::LazyLock;

use anyhow::Result;

use crate::infra::FileSystem;
use crate::domain::file_mover;

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

/// on_exists 策略 trait（OCP: 开放封闭原则）
///
/// 实现此 trait 可以定义新的目标已存在处理策略，
/// 无需修改 LinkOps::link() 主流程。
pub trait OnExistsStrategy {
    /// 执行策略逻辑，返回行动指令
    fn execute(
        &self,
        source: &Path,
        target: &Path,
        fs: &dyn FileSystem,
        verbose: bool,
    ) -> Result<OnExistsAction>;
}

/// 目标已存在时的处理策略枚举
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OnExists {
    /// 跳过，不执行任何操作
    Skip,
    /// 合并目录内容
    Merge,
    /// 保留策略：保留 target 数据，删除 source 后创建链接
    Preserve,
    /// 替换策略：删除 target 后移动 source 到 target 位置
    Replace,
}

/// 策略工厂类型：返回 Box<dyn OnExistsStrategy>
///
/// 使用函数指针（fn）而非 trait object（dyn Fn），
/// 使类型自动实现 Send + Sync，可用于 LazyLock。
type StrategyFactory = fn() -> Box<dyn OnExistsStrategy>;

/// OnExists 策略注册表
///
/// 静态不可变映射，在首次访问时初始化。
/// 键为策略名称，值为策略工厂函数。
static STRATEGY_REGISTRY: LazyLock<HashMap<&'static str, StrategyFactory>> = LazyLock::new(|| {
    let mut reg: HashMap<&'static str, StrategyFactory> = HashMap::new();
    reg.insert(constants::SKIP, skip_strategy_factory);
    reg.insert(constants::REPLACE, replace_strategy_factory);
    reg.insert(constants::MERGE, merge_strategy_factory);
    reg.insert(constants::PRESERVE, preserve_strategy_factory);
    // 向后兼容：旧版 overwrite 别名
    reg.insert("overwrite", preserve_strategy_factory);
    reg
});

// 策略工厂函数（用于注册表，满足 fn 类型要求）
fn skip_strategy_factory() -> Box<dyn OnExistsStrategy> {
    Box::new(SkipStrategy)
}
fn replace_strategy_factory() -> Box<dyn OnExistsStrategy> {
    Box::new(ReplaceStrategy)
}
fn merge_strategy_factory() -> Box<dyn OnExistsStrategy> {
    Box::new(MergeStrategy)
}
fn preserve_strategy_factory() -> Box<dyn OnExistsStrategy> {
    Box::new(PreserveStrategy)
}

// === 策略实现 ===

/// Skip 策略：跳过，不执行任何操作
struct SkipStrategy;

impl OnExistsStrategy for SkipStrategy {
    fn execute(&self, _source: &Path, target: &Path, _fs: &dyn FileSystem, verbose: bool) -> Result<OnExistsAction> {
        if verbose {
            tracing::info!("Target already exists, skipping: {}", target.display());
        }
        Ok(OnExistsAction::Skip)
    }
}

/// Replace 策略：删除目标后继续移动
struct ReplaceStrategy;

impl OnExistsStrategy for ReplaceStrategy {
    fn execute(&self, _source: &Path, target: &Path, fs: &dyn FileSystem, verbose: bool) -> Result<OnExistsAction> {
        if verbose {
            tracing::info!("Removing existing target: {}", target.display());
        }
        fs.remove_if_exists(target)?;
        Ok(OnExistsAction::ContinueWithMove)
    }
}

/// Merge 策略：合并目录内容后不移动
struct MergeStrategy;

impl OnExistsStrategy for MergeStrategy {
    fn execute(&self, source: &Path, target: &Path, fs: &dyn FileSystem, verbose: bool) -> Result<OnExistsAction> {
        if verbose {
            tracing::info!("Merging directories: {} -> {}", source.display(), target.display());
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
}

/// Preserve 策略：保留 target 数据，删除 source 后创建链接
///
/// 当 target 已存在时，保留 target 内容不变，直接删除 source，
/// 然后在 source 位置创建一个指向 target 的链接。
/// 适用于 "target 上的数据是最新版本"的场景。
struct PreserveStrategy;

impl OnExistsStrategy for PreserveStrategy {
    fn execute(&self, source: &Path, _target: &Path, fs: &dyn FileSystem, verbose: bool) -> Result<OnExistsAction> {
        if verbose {
            tracing::info!("Preserving target, removing source: {}", source.display());
        }
        fs.remove_if_exists(source)?;
        Ok(OnExistsAction::ContinueWithoutMove)
    }
}

// === OnExists 枚举实现 ===

impl FromStr for OnExists {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "skip" => Ok(OnExists::Skip),
            "replace" => Ok(OnExists::Replace),
            "merge" => Ok(OnExists::Merge),
            "preserve" | "overwrite" => Ok(OnExists::Preserve),
            _ => Err(format!("Unknown on_exists strategy: {}", s)),
        }
    }
}

impl OnExists {
    /// 宽松解析：解析失败时默认为 Skip
    pub fn from_str_lossy(s: &str) -> Self {
        <Self as FromStr>::from_str(s).unwrap_or(OnExists::Skip)
    }

    /// 获取对应的策略实现（OCP: 开放封闭原则）
    ///
    /// 从策略注册表中获取对应的工厂函数并创建策略实例。
    /// 符合开放封闭原则：添加新策略只需在注册表中注册。
    pub fn strategy(&self) -> Box<dyn OnExistsStrategy> {
        let key = match self {
            Self::Skip => constants::SKIP,
            Self::Replace => constants::REPLACE,
            Self::Merge => constants::MERGE,
            Self::Preserve => constants::PRESERVE,
        };
        STRATEGY_REGISTRY
        .get(key)
        .map(|factory| factory())
        .unwrap_or_else(|| Box::new(SkipStrategy))
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

        let strategy = SkipStrategy;
        let result = strategy.execute(&source, &target, &FsUtils, false).unwrap();
        assert_eq!(result, OnExistsAction::Skip);
        assert!(target.exists());
    }

    // === Replace 策略测试 ===

    #[test]
    fn test_replace_strategy_removes_target() {
        let (_temp, source, target) = setup_test_env();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();

        let strategy = ReplaceStrategy;
        let result = strategy.execute(&source, &target, &FsUtils, false).unwrap();
        assert_eq!(result, OnExistsAction::ContinueWithMove);
        assert!(!target.exists());
    }

    #[test]
    fn test_replace_strategy_removes_file_target() {
        let (_temp, source, target) = setup_test_env();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(&target, "existing file").unwrap();

        let strategy = ReplaceStrategy;
        let result = strategy.execute(&source, &target, &FsUtils, false).unwrap();
        assert_eq!(result, OnExistsAction::ContinueWithMove);
        assert!(!target.exists());
    }

    // === Preserve 策略测试 ===

    #[test]
    fn test_preserve_strategy_removes_source() {
        let (_temp, source, target) = setup_test_env();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();

        let strategy = PreserveStrategy;
        let result = strategy.execute(&source, &target, &FsUtils, false).unwrap();
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

        let strategy = MergeStrategy;
        let result = strategy.execute(&source, &target, &FsUtils, false).unwrap();
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

        let strategy = MergeStrategy;
        strategy.execute(&source, &target, &FsUtils, false).unwrap();
        assert!(!source.exists());
    }

    #[test]
    fn test_merge_strategy_preserves_existing_target_files() {
        let (_temp, source, target) = setup_test_env();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("existing.txt"), "existing").unwrap();
        std::fs::write(source.join("new.txt"), "new").unwrap();

        let strategy = MergeStrategy;
        strategy.execute(&source, &target, &FsUtils, false).unwrap();

        assert!(target.join("existing.txt").exists());
        assert!(target.join("new.txt").exists());
        assert_eq!(
            std::fs::read_to_string(target.join("existing.txt")).unwrap(),
            "existing"
        );
    }

    // === OnExists::strategy 工厂方法测试 ===

    #[test]
    fn test_on_exists_skip_creates_skip_strategy() {
        let strategy = OnExists::Skip.strategy();
        let (_temp, source, target) = setup_test_env();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        let result = strategy.execute(&source, &target, &FsUtils, false).unwrap();
        assert_eq!(result, OnExistsAction::Skip);
    }

    #[test]
    fn test_on_exists_preserve_creates_preserve_strategy() {
        let strategy = OnExists::Preserve.strategy();
        let (_temp, source, target) = setup_test_env();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        let result = strategy.execute(&source, &target, &FsUtils, false).unwrap();
        assert_eq!(result, OnExistsAction::ContinueWithoutMove);
    }

    #[test]
    fn test_on_exists_strategy_defaults_to_skip_on_unknown() {
        let result = OnExists::from_str_lossy("unknown");
        assert_eq!(result, OnExists::Skip);
    }
}
