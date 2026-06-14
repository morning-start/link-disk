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
use std::sync::RwLock;

/// 占位符常量模块
///
/// 定义所有支持的占位符常量，拼写错误可在编译时捕获。
pub mod placeholders {
    pub const HOME: &str = "<home>";
    pub const APPDATA: &str = "<appdata>";
    pub const LOCALAPPDATA: &str = "<localappdata>";
    pub const DOCUMENTS: &str = "<documents>";
    pub const DESKTOP: &str = "<desktop>";
    pub const DOWNLOADS: &str = "<downloads>";
    pub const TEMP: &str = "<temp>";
    pub const PROGRAM_FILES: &str = "<programfiles>";
    pub const PROGRAM_FILES_X86: &str = "<programfilesx86>";

    /// 所有内置占位符列表（单一数据源，避免多处重复定义）
    ///
    /// `is_known_placeholder` 和 `register_placeholder` 共享此列表，
    /// 新增内置占位符时只需在此处加一行。
    pub const BUILT_IN: &[&str] = &[
        HOME,
        APPDATA,
        LOCALAPPDATA,
        DOCUMENTS,
        DESKTOP,
        DOWNLOADS,
        TEMP,
        PROGRAM_FILES,
        PROGRAM_FILES_X86,
    ];
}

/// 占位符解析器类型：返回 `Option<String>`
type PlaceholderResolver = Box<dyn Fn() -> Option<String> + Send + Sync>;

/// 占位符注册表
///
/// 使用 RwLock 支持运行时注册，读多写少场景性能优先。
/// 键为占位符字符串（如 `"<home>"`），值为解析函数。
static PLACEHOLDER_REGISTRY: LazyLock<RwLock<HashMap<String, PlaceholderResolver>>> =
    LazyLock::new(|| {
        let mut map: HashMap<String, PlaceholderResolver> = HashMap::new();

        map.insert(
            placeholders::HOME.to_string(),
            Box::new(|| dirs::home_dir().map(|p| p.to_string_lossy().into_owned())),
        );

        map.insert(
            placeholders::APPDATA.to_string(),
            Box::new(|| dirs::data_dir().map(|p| p.to_string_lossy().into_owned())),
        );

        map.insert(
            placeholders::LOCALAPPDATA.to_string(),
            Box::new(|| dirs::data_local_dir().map(|p| p.to_string_lossy().into_owned())),
        );

        map.insert(
            placeholders::DOCUMENTS.to_string(),
            Box::new(|| dirs::document_dir().map(|p| p.to_string_lossy().into_owned())),
        );

        map.insert(
            placeholders::DESKTOP.to_string(),
            Box::new(|| dirs::desktop_dir().map(|p| p.to_string_lossy().into_owned())),
        );

        map.insert(
            placeholders::DOWNLOADS.to_string(),
            Box::new(|| dirs::download_dir().map(|p| p.to_string_lossy().into_owned())),
        );

        map.insert(
            placeholders::TEMP.to_string(),
            Box::new(|| dirs::cache_dir().map(|p| p.to_string_lossy().into_owned())),
        );

        map.insert(
            placeholders::PROGRAM_FILES.to_string(),
            Box::new(|| std::env::var("ProgramFiles").ok()),
        );

        map.insert(
            placeholders::PROGRAM_FILES_X86.to_string(),
            Box::new(|| std::env::var("ProgramFiles(x86)").ok()),
        );

        RwLock::new(map)
    });

/// 检查占位符是否已注册（含内置和运行时注册）
pub fn is_known_placeholder(placeholder: &str) -> bool {
    if placeholders::BUILT_IN.contains(&placeholder) {
        return true;
    }

    let registry = PLACEHOLDER_REGISTRY
        .read()
        .expect("Placeholder registry lock poisoned");
    registry.contains_key(placeholder)
}

/// 运行时注册自定义占位符
///
/// 可以在程序启动后动态添加占位符，优先级低于内置占位符
/// （内置占位符不会被覆盖）。
///
/// # 参数
/// - `key`: 占位符字符串，应包含尖括号，如 `"<workspace>"`
/// - `resolver`: 解析函数，返回 `Option<String>`
///
/// # 返回值
/// 如果键名与内置占位符冲突则返回 `Err`，否则返回 `Ok`
pub fn register_placeholder(
    key: &str,
    resolver: Box<dyn Fn() -> Option<String> + Send + Sync>,
) -> Result<(), String> {
    if placeholders::BUILT_IN.contains(&key) {
        return Err(format!("Cannot override built-in placeholder '{}'", key));
    }

    let mut registry = PLACEHOLDER_REGISTRY
        .write()
        .expect("Placeholder registry lock poisoned");
    registry.insert(key.to_string(), resolver);
    Ok(())
}

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
    ///
    /// 不替换路径分隔符：`PathBuf`/`Path` 在各平台都能正确处理 `/` 和 `\`，
    /// 手动转 `\` 会破坏 Unix 兼容性，也违背"统一正斜杠"的路径处理原则。
    ///
    /// # 实现
    ///
    /// 不对整个注册表做 `O(N)` 全表 `contains` 扫描，而是先扫描输入字符串
    /// 提取其中实际出现的 `<...>` 占位符（去重），仅对命中的占位符查注册表。
    /// 路径中通常只含 0~2 个占位符，但注册表可能含数十项，此优化避免无谓的
    /// 反复字符串搜索与 resolver 闭包调用。
    fn replace_placeholders(input: &str) -> String {
        // 第一遍：提取输入中实际出现的占位符（按出现顺序去重）
        // 不对整个注册表做 O(N) 全表 contains 扫描，仅命中实际出现的 <...>。
        let mut hits: Vec<&str> = Vec::new();
        let mut rest = input;
        while let Some(open) = rest.find('<') {
            let after_open = &rest[open + 1..];
            match after_open.find('>') {
                Some(close) => {
                    let candidate = &rest[open..open + 1 + close + 1]; // 含 '<' 和 '>'
                    if !hits.contains(&candidate) {
                        hits.push(candidate);
                    }
                    rest = &after_open[close + 1..];
                }
                None => break,
            }
        }

        if hits.is_empty() {
            return input.to_string();
        }

        // 第二遍：仅对命中占位符查注册表并替换
        let registry = PLACEHOLDER_REGISTRY
            .read()
            .expect("Placeholder registry lock poisoned");

        let mut result = input.to_string();
        for placeholder in hits {
            if let Some(resolver) = registry.get(placeholder)
                && let Some(value) = resolver()
            {
                result = result.replace(placeholder, &value);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_home_placeholder() {
        let result = PathResolver::expand("<home>");
        assert!(!result.contains("<home>"));
    }

    #[test]
    fn test_register_custom_placeholder() {
        register_placeholder("<custom>", Box::new(|| Some("C:/custom/path".into()))).unwrap();
        let result = PathResolver::expand("<custom>/data");
        assert!(
            result.contains("C:\\custom\\path\\data") || result.contains("C:/custom/path/data")
        );
    }

    #[test]
    fn test_cannot_override_builtin() {
        let result = register_placeholder("<home>", Box::new(|| Some("override".into())));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("built-in"));
    }

    #[test]
    fn test_custom_placeholder_does_not_affect_builtin() {
        register_placeholder("<myapp>", Box::new(|| Some("D:/myapp".into()))).unwrap();
        let home = PathResolver::expand("<home>/test");
        assert!(!home.contains("<home>"));
    }

    #[test]
    fn test_multiple_custom_placeholders() {
        register_placeholder("<a>", Box::new(|| Some("A".into()))).unwrap();
        register_placeholder("<b>", Box::new(|| Some("B".into()))).unwrap();
        let result = PathResolver::expand("<a>/<b>");
        assert!(result.contains("A") && result.contains("B"));
    }

    #[test]
    fn test_register_duplicate_custom_ok() {
        register_placeholder("<dup>", Box::new(|| Some("first".into()))).unwrap();
        register_placeholder("<dup>", Box::new(|| Some("second".into()))).unwrap();
        let result = PathResolver::expand("<dup>");
        assert!(!result.contains("<dup>"));
    }

    #[test]
    fn test_is_known_placeholder_builtin() {
        assert!(is_known_placeholder("<home>"));
    }

    #[test]
    fn test_is_known_placeholder_custom() {
        register_placeholder("<mycust>", Box::new(|| Some("value".into()))).unwrap();
        assert!(is_known_placeholder("<mycust>"));
    }

    #[test]
    fn test_is_known_placeholder_unknown() {
        assert!(!is_known_placeholder("<nonexistent>"));
    }

    // === replace_placeholders 扫描优化回归测试 ===

    #[test]
    fn test_no_placeholder_returns_input_as_is() {
        let result = PathResolver::expand("C:/plain/path");
        assert_eq!(result, "C:/plain/path");
    }

    #[test]
    fn test_repeated_placeholder_all_replaced() {
        register_placeholder("<rep>", Box::new(|| Some("X".into()))).unwrap();
        let result = PathResolver::expand("<rep>/<rep>/<rep>");
        assert_eq!(result, "X/X/X");
    }

    #[test]
    fn test_unknown_placeholder_preserved() {
        let result = PathResolver::expand("<home>/<definitely_unknown_xyz>");
        // <home> 被展开，未注册的占位符保持原样
        assert!(!result.contains("<home>"));
        assert!(result.contains("<definitely_unknown_xyz>"));
    }
}
