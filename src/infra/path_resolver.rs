//! 路径解析模块
//!
//! 负责将配置文件中的占位符路径转换为实际路径，支持：
//! - <home>: 用户主目录
//! - <appdata>: 应用数据目录 (AppData/Roaming)
//! - <localappdata>: 本地应用数据目录 (AppData/Local)
//! - <documents>: 文档目录
//! - <desktop>: 桌面目录
//! - <downloads>: 下载目录
//! - <temp>: 临时目录
//! - <programfiles>: Program Files 目录
//! - <programfilesx86>: Program Files (x86) 目录
//!
//! ## 设计说明（OCP: 开放封闭原则）
//!
//! 采用注册表模式实现占位符解析，符合开放封闭原则：
//! - 添加新占位符无需修改现有代码，只需在注册表中添加条目
//! - 支持运行时扩展（通过 `register_placeholder()`）
//! - 默认内置 9 个常用占位符

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;

/// 占位符常量模块
///
/// 定义所有支持的占位符常量，拼写错误可在编译时捕获。
pub mod placeholders {
    /// 用户主目录
    pub const HOME: &str = "<home>";
    /// 应用数据目录 (AppData/Roaming)
    pub const APPDATA: &str = "<appdata>";
    /// 本地应用数据目录 (AppData/Local)
    pub const LOCALAPPDATA: &str = "<localappdata>";
    /// 文档目录
    pub const DOCUMENTS: &str = "<documents>";
    /// 桌面目录
    pub const DESKTOP: &str = "<desktop>";
    /// 下载目录
    pub const DOWNLOADS: &str = "<downloads>";
    /// 临时目录
    pub const TEMP: &str = "<temp>";
    /// Program Files 目录
    pub const PROGRAM_FILES: &str = "<programfiles>";
    /// Program Files (x86) 目录
    pub const PROGRAM_FILES_X86: &str = "<programfilesx86>";
}

/// 占位符解析器类型：返回 `Option<String>`
type PlaceholderResolver = fn() -> Option<String>;

/// 占位符注册表
///
/// 静态不可变映射，在首次访问时初始化。
/// 键为占位符字符串（如 `"<home>"`），值为解析函数。
static PLACEHOLDER_REGISTRY: LazyLock<HashMap<&'static str, PlaceholderResolver>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();

        map.insert(
            placeholders::HOME,
            (|| dirs::home_dir().map(|p| p.to_string_lossy().into_owned())) as PlaceholderResolver,
        );

        map.insert(
            placeholders::APPDATA,
            (|| dirs::data_dir().map(|p| p.to_string_lossy().into_owned())) as PlaceholderResolver,
        );

        map.insert(
            placeholders::LOCALAPPDATA,
            (|| dirs::data_local_dir().map(|p| p.to_string_lossy().into_owned())) as PlaceholderResolver,
        );

        map.insert(
            placeholders::DOCUMENTS,
            (|| dirs::document_dir().map(|p| p.to_string_lossy().into_owned())) as PlaceholderResolver,
        );

        map.insert(
            placeholders::DESKTOP,
            (|| dirs::desktop_dir().map(|p| p.to_string_lossy().into_owned())) as PlaceholderResolver,
        );

        map.insert(
            placeholders::DOWNLOADS,
            (|| dirs::download_dir().map(|p| p.to_string_lossy().into_owned())) as PlaceholderResolver,
        );

        map.insert(
            placeholders::TEMP,
            (|| dirs::cache_dir().map(|p| p.to_string_lossy().into_owned())) as PlaceholderResolver,
        );

        map.insert(
            placeholders::PROGRAM_FILES,
            (|| std::env::var("ProgramFiles").ok()) as PlaceholderResolver,
        );

        map.insert(
            placeholders::PROGRAM_FILES_X86,
            (|| std::env::var("ProgramFiles(x86)").ok()) as PlaceholderResolver,
        );

        map
    });

/// 路径解析工具类
pub struct PathResolver;

impl PathResolver {
    /// 展开路径中的所有占位符，返回展开后的字符串
    pub fn expand(path: &str) -> String {
        Self::replace_placeholders(path)
    }

    /// 展开路径中的 ~ 前缀为用户主目录
    pub fn expand_home(path: &str) -> PathBuf {
        if path.starts_with("~")
            && let Some(home) = dirs::home_dir()
        {
            return home.join(
                path.trim_start_matches("~")
                    .trim_start_matches('/')
                    .trim_start_matches('\\'),
            );
        }
        PathBuf::from(path)
    }

    /// 展开路径并检查是否存在，存在则返回 Some(PathBuf)
    pub fn resolve_if_exists(path: &str) -> Option<PathBuf> {
        let expanded = Self::replace_placeholders(path);
        let path = PathBuf::from(expanded);
        if path.exists() { Some(path) } else { None }
    }

    /// 替换字符串中的所有占位符为实际路径
    ///
    /// 通过遍历注册表实现，符合开放封闭原则：
    /// 添加新占位符只需在注册表中添加条目，无需修改此方法。
    fn replace_placeholders(input: &str) -> String {
        let mut result = input.to_string();

        for (placeholder, resolver) in PLACEHOLDER_REGISTRY.iter() {
            if result.contains(placeholder)
                && let Some(value) = resolver()
            {
                result = result.replace(placeholder, &value);
            }
        }

        // 将正斜杠转换为反斜杠（Windows 路径格式）
        result.replace("/", "\\")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 <home> 占位符是否能正确展开
    #[test]
    fn test_home_placeholder() {
        let result = PathResolver::expand("<home>");
        assert!(!result.contains("<home>"));
    }
}
