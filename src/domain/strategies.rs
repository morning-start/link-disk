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
    /// 覆盖策略
    pub const OVERWRITE: &str = "overwrite";
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
    /// 覆盖源文件后重新创建链接
    Overwrite,
    /// 删除目标后移动源到目标位置
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
    reg.insert(constants::OVERWRITE, overwrite_strategy_factory);
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
fn overwrite_strategy_factory() -> Box<dyn OnExistsStrategy> {
    Box::new(OverwriteStrategy)
}

// === 策略实现 ===

/// Skip 策略：跳过，不执行任何操作
struct SkipStrategy;

impl OnExistsStrategy for SkipStrategy {
    fn execute(&self, _source: &Path, target: &Path, _fs: &dyn FileSystem, verbose: bool) -> Result<OnExistsAction> {
        if verbose {
            tracing::info!("Target already exists, skipping: {:?}", target);
        }
        Ok(OnExistsAction::Skip)
    }
}

/// Replace 策略：删除目标后继续移动
struct ReplaceStrategy;

impl OnExistsStrategy for ReplaceStrategy {
    fn execute(&self, _source: &Path, target: &Path, fs: &dyn FileSystem, verbose: bool) -> Result<OnExistsAction> {
        if verbose {
            tracing::info!("Removing existing target: {:?}", target);
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
            tracing::info!("Merging directories: {:?} -> {:?}", source, target);
        }
        file_mover::merge_dirs(source, target, fs)?;
        Ok(OnExistsAction::ContinueWithoutMove)
    }
}

/// Overwrite 策略：删除源文件后继续移动
struct OverwriteStrategy;

impl OnExistsStrategy for OverwriteStrategy {
    fn execute(&self, source: &Path, _target: &Path, fs: &dyn FileSystem, verbose: bool) -> Result<OnExistsAction> {
        if verbose {
            tracing::info!("Removing source for overwrite: {:?}", source);
        }
        fs.remove_if_exists(source)?;
        Ok(OnExistsAction::ContinueWithMove)
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
            "overwrite" => Ok(OnExists::Overwrite),
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
            Self::Overwrite => constants::OVERWRITE,
        };
        STRATEGY_REGISTRY
            .get(key)
            .map(|factory| factory())
            .unwrap_or_else(|| Box::new(SkipStrategy))
    }
}
