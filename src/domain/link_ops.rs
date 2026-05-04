//! 链接操作模块
//!
//! 提供链接的核心功能，包括：
//! - 符号链接和硬链接的创建
//! - 链接的删除（unlink）
//! - 目标已存在时的处理策略
//! - 链接状态检查
//!
//! 目录操作（合并、回移）已合并为本模块的私有方法。
//!
//! ## API 参数约定
//!
//! 本模块遵循统一的参数风格：
//! - **输入参数**（只读访问）：使用 `&Path` 引用
//! - **返回值**（调用者需要所有权）：使用 `PathBuf`
//! - **结构体字段**（需要存储）：使用 `PathBuf`
//!
//! ### 公开方法签名示例
//! ```rust,ignore
//! // ✅ 正确：输入参数使用 &Path
//! pub fn unlink(source: &Path, target: &Path, ...) -> Result<()>;
//! pub fn check_status(source: &Path, target: &Path) -> LinkStatus;
//!
//! // ✅ 正确：结构体字段使用 PathBuf（需要所有权）
//! pub struct LinkRequest {
//!     pub source: PathBuf,
//!     pub target: PathBuf,
//! }
//! ```

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::LazyLock;
use tracing::{info, debug};

use crate::infra::{FileSystem, FsUtils};

// 重新导出 LinkStatus 和 LinkStatusChecker，保持向后兼容
pub use crate::domain::link_status::{LinkStatus, LinkStatusChecker};

/// 策略名称常量模块
///
/// 定义所有支持的策略名称常量，拼写错误可在编译时捕获。
pub mod strategies {
    /// 跳过策略
    pub const SKIP: &str = "skip";
    /// 替换策略
    pub const REPLACE: &str = "replace";
    /// 合并策略
    pub const MERGE: &str = "merge";
    /// 覆盖策略
    pub const OVERWRITE: &str = "overwrite";
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
    reg.insert(strategies::SKIP, skip_strategy_factory);
    reg.insert(strategies::REPLACE, replace_strategy_factory);
    reg.insert(strategies::MERGE, merge_strategy_factory);
    reg.insert(strategies::OVERWRITE, overwrite_strategy_factory);
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

/// 链接操作工具类
pub struct LinkOps;

/// 链接类型枚举
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinkType {
    /// 符号链接（软链接）
    Symlink,
    /// 硬链接
    Hardlink,
}

impl FromStr for LinkType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hardlink" => Ok(LinkType::Hardlink),
            "symlink" => Ok(LinkType::Symlink),
            _ => Err(format!("Unknown link type: {}", s)),
        }
    }
}

impl LinkType {
    /// 宽松解析：解析失败时默认为 Symlink
    pub fn from_str_lossy(s: &str) -> Self {
        <Self as FromStr>::from_str(s).unwrap_or(LinkType::Symlink)
    }

    #[deprecated(since = "1.2.0", note = "use FromStr trait instead")]
    #[allow(dead_code)]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "hardlink" | "Hardlink" | "HARDLINK" => LinkType::Hardlink,
            _ => LinkType::Symlink,
        }
    }
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
            Self::Skip => strategies::SKIP,
            Self::Replace => strategies::REPLACE,
            Self::Merge => strategies::MERGE,
            Self::Overwrite => strategies::OVERWRITE,
        };
        STRATEGY_REGISTRY
            .get(key)
            .map(|factory| factory())
            .unwrap_or_else(|| Box::new(SkipStrategy))
    }

    #[deprecated(since = "1.2.0", note = "use FromStr trait instead")]
    #[allow(dead_code)]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "merge" | "Merge" | "MERGE" => OnExists::Merge,
            "overwrite" | "Overwrite" | "OVERWRITE" => OnExists::Overwrite,
            "replace" | "Replace" | "REPLACE" => OnExists::Replace,
            _ => OnExists::Skip,
        }
    }
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

/// Skip 策略：跳过，不执行任何操作
struct SkipStrategy;

impl OnExistsStrategy for SkipStrategy {
    fn execute(&self, _source: &Path, target: &Path, _fs: &dyn FileSystem, verbose: bool) -> Result<OnExistsAction> {
        if verbose {
            info!("Target already exists, skipping: {:?}", target);
        }
        Ok(OnExistsAction::Skip)
    }
}

/// Replace 策略：删除目标后继续移动
struct ReplaceStrategy;

impl OnExistsStrategy for ReplaceStrategy {
    fn execute(&self, _source: &Path, target: &Path, fs: &dyn FileSystem, verbose: bool) -> Result<OnExistsAction> {
        if verbose {
            info!("Removing existing target: {:?}", target);
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
            info!("Merging directories: {:?} -> {:?}", source, target);
        }
        LinkOps::merge_dirs(source, target, fs)?;
        Ok(OnExistsAction::ContinueWithoutMove)
    }
}

/// Overwrite 策略：删除源文件后继续移动
struct OverwriteStrategy;

impl OnExistsStrategy for OverwriteStrategy {
    fn execute(&self, source: &Path, _target: &Path, fs: &dyn FileSystem, verbose: bool) -> Result<OnExistsAction> {
        if verbose {
            info!("Removing source for overwrite: {:?}", source);
        }
        fs.remove_if_exists(source)?;
        Ok(OnExistsAction::ContinueWithMove)
    }
}

/// 链接请求结构体
pub struct LinkRequest {
    /// 源路径（原位置）
    pub source: PathBuf,
    /// 目标路径（工作区中的位置）
    pub target: PathBuf,
    /// 链接类型
    pub link_type: LinkType,
    /// 目标已存在时的处理策略
    pub on_exists: OnExists,
    /// 是否强制覆盖已存在的符号链接
    pub force: bool,
}

impl LinkOps {
    /// 创建链接（便捷方法，使用默认的 FsUtils 实现）
    ///
    /// @deprecated 推荐使用 `link_with_fs()` 显式传入文件系统实现
    #[deprecated(note = "Use `link_with_fs()` with explicit FileSystem dependency")]
    #[allow(dead_code)]
    pub fn link(request: &LinkRequest, verbose: bool) -> Result<()> {
        let fs = FsUtils;
        Self::link_with_fs(request, &fs, verbose)
    }

    /// 创建链接：将源路径的内容转移到目标路径，然后在源位置创建链接指向目标
    ///
    /// 支持通过 FileSystem trait 注入不同的文件系统实现，便于测试。
    ///
    /// # 流程
    /// 1. 检查并处理已存在的符号链接（force 逻辑）
    /// 2. 根据策略处理目标已存在的情况
    /// 3. 移动源内容到目标（或准备目标目录结构）
    /// 4. 在源位置创建指向目标的链接
    pub fn link_with_fs(request: &LinkRequest, fs: &dyn FileSystem, verbose: bool) -> Result<()> {
        let source = &request.source;
        let target = &request.target;

        Self::log_link_request(source, target, request);

        // 步骤1: 符号链接检查与处理
        Self::check_and_handle_symlink(source, target, request.force, fs, verbose)?;

        // 步骤2-3: 处理源/目标存在性 + 移动文件或准备目录
        if source.exists() {
            let should_move = Self::handle_on_exists(source, target, request.on_exists, fs, verbose)?;
            if should_move {
                fs.ensure_parent_exists(target)?;
                fs.move_dir_cross_filesystem(source, target)?;
            }
        } else {
            Self::prepare_target_for_link(target, fs, verbose)?;
        }

        // 步骤4: 创建链接（始终执行）
        Self::create_link(source, target, request.link_type, fs, verbose)
    }

    fn log_link_request(source: &Path, target: &Path, request: &LinkRequest) {
        info!("Linking: {:?} -> {:?}", source, target);
        debug!("Source exists: {}", source.exists());
        debug!("Source is_symlink: {}", source.is_symlink());
        debug!("Target exists: {}", target.exists());
        debug!("Target is_symlink: {}", target.is_symlink());
        debug!("Force: {}", request.force);
        debug!("LinkType: {:?}", request.link_type);
    }

    /// 检查并处理已存在的符号链接
    fn check_and_handle_symlink(
        source: &Path,
        target: &Path,
        force: bool,
        fs: &dyn FileSystem,
        verbose: bool,
    ) -> Result<()> {
        if !source.is_symlink() { return Ok(()); }

        if force {
            if verbose {
                info!("Force: removing existing symlink: {:?}", source);
            }
            return fs.remove_if_exists(source);
        }

        if let Some(target_path) = fs.read_link(source) {
            let normalized_linked = fs.normalize_path(&target_path);
            let normalized_target = fs.normalize_path(target);
            if normalized_linked == normalized_target {
                if verbose {
                    info!("Already linked: {:?} -> {:?}", source, target_path);
                }
                return Ok(());
            }
        }

        anyhow::bail!(
            "Source is already a symlink pointing to different target: {:?}",
            source
        )
    }

    /// 处理 on_exists 策略（使用策略模式）
    ///
    /// 通过 OnExistsStrategy trait 实现开放封闭原则，
    /// 添加新策略无需修改此方法。
    ///
    /// 返回: 是否需要将源文件移动到目标位置
    fn handle_on_exists(
        source: &Path,
        target: &Path,
        on_exists: OnExists,
        fs: &dyn FileSystem,
        verbose: bool,
    ) -> Result<bool> {
        if !target.exists() { return Ok(true); }

        let strategy = on_exists.strategy();
        match strategy.execute(source, target, fs, verbose)? {
            OnExistsAction::Skip => Ok(false),
            OnExistsAction::ContinueWithMove => Ok(true),
            OnExistsAction::ContinueWithoutMove => Ok(false),
        }
    }

    /// 当源不存在时，准备目标目录结构用于创建链接
    fn prepare_target_for_link(target: &Path, fs: &dyn FileSystem, verbose: bool) -> Result<()> {
        if verbose {
            info!("Source does not exist, creating target directory structure...");
        }
        fs.ensure_parent_exists(target)?;
        if !target.exists() {
            std::fs::create_dir_all(target)
                .with_context(|| format!("Failed to create target directory: {:?}", target))?;
        }
        Ok(())
    }

    /// 在源位置创建指向目标的链接
    fn create_link(
        source: &Path,
        target: &Path,
        link_type: LinkType,
        fs: &dyn FileSystem,
        verbose: bool,
    ) -> Result<()> {
        match link_type {
            LinkType::Symlink => {
                if verbose {
                    info!("Creating symlink: {:?} -> {:?}", source, target);
                }
                fs.create_symlink(target, source)?;
            }
            LinkType::Hardlink => {
                if verbose {
                    info!("Creating hardlink: {:?} -> {:?}", source, target);
                }
                fs.hard_link(target, source)?;
            }
        }

        if verbose {
            info!("Successfully linked: {:?} -> {:?}", source, target);
        }

        Ok(())
    }

    /// 删除链接：移除源位置的链接，可选择将目标位置的文件移回源位置
    ///
    /// @deprecated 推荐使用 `unlink_with_fs()` 显式传入文件系统实现
    #[deprecated(note = "Use `unlink_with_fs()` with explicit FileSystem dependency")]
    #[allow(dead_code)]
    pub fn unlink(source: &Path, target: &Path, keep_files: bool, _verbose: bool) -> Result<()> {
        let fs = FsUtils;
        Self::unlink_with_fs(source, target, keep_files, &fs)
    }

    /// 删除链接（支持依赖注入版本）
    pub fn unlink_with_fs(source: &Path, target: &Path, keep_files: bool, fs: &dyn FileSystem) -> Result<()> {
        info!("Unlinking: {:?} -> {:?}", source, target);
        debug!("Keep files: {}", keep_files);

        if source.is_symlink() {
            fs.remove_if_exists(source)?;

            if !keep_files && target.exists() {
                Self::move_back(target, source, fs)?;
            }
        } else if source.exists() {
            anyhow::bail!("Source is not a symlink: {:?}", source);
        } else if target.exists() && !keep_files {
            Self::move_back(target, source, fs)?;
        }

        info!("Successfully unlinked: {:?} -> {:?}", source, target);
        Ok(())
    }

    /// 合并两个目录的内容（源目录合并到目标目录）
    ///
    /// 将源目录中的所有文件和子目录迭代合并到目标目录，
    /// 合并完成后删除源目录。
    ///
    /// 使用 BFS 迭代实现，避免深目录导致的栈溢出。
    ///
    /// # 参数
    /// - `source`: 源目录路径
    /// - `target`: 目标目录路径
    /// - `fs`: 文件系统操作接口
    fn merge_dirs(source: &Path, target: &Path, fs: &dyn FileSystem) -> Result<()> {
        if !source.is_dir() || !target.is_dir() {
            anyhow::bail!("Merge requires both paths to be directories");
        }

        info!("Merging directories: {:?} -> {:?}", source, target);

        let mut queue = VecDeque::new();
        queue.push_back((source.to_path_buf(), target.to_path_buf()));

        while let Some((src_dir, dst_dir)) = queue.pop_front() {
            if !dst_dir.exists() {
                std::fs::create_dir_all(&dst_dir)
                    .with_context(|| format!("Failed to create directory: {:?}", dst_dir))?;
            }

            for entry in std::fs::read_dir(&src_dir)
                .with_context(|| format!("Failed to read directory: {:?}", src_dir))?
            {
                let entry = entry?;
                let src_path = entry.path();
                let dst_path = dst_dir.join(entry.file_name());

                if src_path.is_dir() {
                    queue.push_back((src_path, dst_path));
                } else if !dst_path.exists() {
                    std::fs::copy(&src_path, &dst_path)
                        .with_context(|| format!("Failed to copy: {:?} to {:?}", src_path, dst_path))?;
                }
            }
        }

        fs.remove_if_exists(source)?;

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
    fn move_back(source: &Path, target: &Path, fs: &dyn FileSystem) -> Result<()> {
        if !source.exists() {
            anyhow::bail!("Target path does not exist: {:?}", source);
        }

        fs.ensure_parent_exists(target)?;

        if source.is_dir() {
            fs.copy_dir_recursive(source, target)?;
            fs.remove_if_exists(source)?;
        } else {
            fs.rename(source, target)?;
        }

        Ok(())
    }

    /// 检查链接状态（委托给 LinkStatusChecker）
    ///
    /// 返回值说明：
    /// - Linked: 链接正常，目标和源都存在
    /// - Broken: 链接损坏（源是符号链接但目标不存在）
    /// - BothExist: 源和目标都存在但不是链接
    /// - SourceOnly: 只有源存在
    /// - TargetOnly: 只有目标存在
    /// - None: 都不存在
    pub fn check_status(source: &Path, target: &Path) -> LinkStatus {
        LinkStatusChecker::check(source, target)
    }
}
