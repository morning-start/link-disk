//! 配置文件解析模块
//!
//! 负责加载和解析 TOML 格式的配置文件，包括：
//! - 工作区路径配置
//! - 应用配置（名称、启用状态、链接策略）
//! - 源文件/目录配置（源路径、目标路径、链接类型）
//!
//! ## 强类型设计
//!
//! `link_type` 和 `on_exists` 直接反序列化为 `LinkType` / `OnExists` 枚举，
//! 把值域校验前移到反序列化阶段，无需在 `validate()` 中再字符串 match。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::warn;

use super::path_resolver::{is_known_placeholder, register_placeholder};
use crate::domain::{LinkType, OnExists};

/// 配置常量模块
pub mod constants {
    use crate::domain::LinkType;

    /// 默认链接类型
    pub const DEFAULT_LINK_TYPE: LinkType = LinkType::Symlink;
}

/// 顶层配置结构体
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// 工作区配置
    pub workspace: Workspace,
    /// 应用配置映射表 (key: 应用ID, value: 应用配置)
    ///
    /// 使用 `BTreeMap` 保证遍历顺序按应用 ID 字典序稳定，
    /// 使 link/list/status 等命令的输出可复现。
    #[serde(default)]
    pub apps: std::collections::BTreeMap<String, AppConfig>,
    /// 自定义占位符映射表
    /// 键为占位符名（不含尖括号），值为实际路径
    /// 例如: { "myapp": "C:/Program Files/MyApp" }
    #[serde(default)]
    pub custom_placeholders: std::collections::HashMap<String, String>,
}

/// 工作区配置
#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    /// 工作区根目录路径
    pub path: PathBuf,
}

/// 单个应用的配置
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    /// 显示名称
    pub name: String,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 目标已存在时的处理策略（缺省时由 `on_exists_or_default` 返回 Skip）
    #[serde(default)]
    pub on_exists: Option<OnExists>,
    /// 源文件/目录列表
    #[serde(default)]
    pub sources: Vec<Source>,
}

/// 默认启用状态为 true
fn default_enabled() -> bool {
    true
}

/// 源文件/目录配置
#[derive(Debug, Clone, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct Source {
    /// 源路径（支持占位符如 <home>、<appdata> 等）
    pub source: String,
    /// 相对于工作区的目标路径
    pub target: String,
    /// 链接类型：symlink 或 hardlink（缺省 symlink）
    #[serde(default = "default_link_type")]
    pub link_type: LinkType,
    /// 源级别 on_exists 策略覆盖（优先级高于应用级别）
    #[serde(default)]
    pub on_exists: Option<OnExists>,
}

/// 默认链接类型为符号链接
fn default_link_type() -> LinkType {
    constants::DEFAULT_LINK_TYPE
}

/// 检查配置文件权限是否安全
///
/// 在 Unix 上检查文件权限位（group/other 不应有读取权限）。
/// 在 Windows 上检查配置文件是否位于用户主目录或 AppData 下。
fn check_config_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(path) {
            let mode = metadata.permissions().mode();
            if mode & 0o077 != 0 {
                warn!(
                    "Config file '{}' has permissive permissions ({:#o}). \
                     Consider restricting access with: chmod 600 {}",
                    path.display(),
                    mode & 0o777,
                    path.display()
                );
            }
        }
    }

    #[cfg(windows)]
    {
        if let Some(parent) = path.parent() {
            let parent_str = parent.to_string_lossy().to_lowercase();
            let in_home = dirs::home_dir()
                .map(|h| parent_str.starts_with(&h.to_string_lossy().to_lowercase()))
                .unwrap_or(false);
            let in_appdata = dirs::data_dir()
                .or_else(dirs::data_local_dir)
                .map(|d| parent_str.starts_with(&d.to_string_lossy().to_lowercase()))
                .unwrap_or(false);
            if !in_home && !in_appdata {
                warn!(
                    "Config file '{}' is outside user home/AppData directory. \
                     Ensure file permissions are restricted to your user account only.",
                    path.display()
                );
            }
        }
    }
}

/// 检查将被拼接进工作区物理路径的分量（应用名、target）是否安全
///
/// 规则：相对路径、无根、不含 `..`、不以分隔符开头、不含
/// Windows 非法字符 `: * ? " < > |`。违反这些规则会在预期目录之外
/// 创建路径或直接失败，属于运行期才暴露的隐蔽错误，配置校验阶段拦截。
fn check_workspace_relative(app_id: &str, kind: &str, value: &str) -> Result<()> {
    let path = Path::new(value);
    let has_invalid_char = value
        .chars()
        .any(|c| matches!(c, ':' | '*' | '?' | '"' | '<' | '>' | '|'));

    if !value.is_empty()
        && path.is_relative()
        && !path.has_root()
        && !path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        && !has_invalid_char
    {
        return Ok(());
    }

    anyhow::bail!("App '{}' has unsafe {kind}: '{value}'", app_id)
}

impl Config {
    /// 从指定路径加载配置文件
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        let config: Config = toml::from_str(&content).context("Failed to parse config file")?;

        check_config_permissions(path);

        config.register_custom_placeholders()?;

        Ok(config)
    }

    /// 注册自定义占位符到运行时注册表
    fn register_custom_placeholders(&self) -> Result<()> {
        for (name, value) in &self.custom_placeholders {
            let key = format!("<{}>", name);
            let path = value.clone();
            register_placeholder(&key, Box::new(move || Some(path.clone()))).map_err(|e| {
                anyhow::anyhow!("Failed to register custom placeholder '{}': {}", key, e)
            })?;
        }
        Ok(())
    }

    /// 校验配置有效性
    ///
    /// 注意：`link_type` 和 `on_exists` 的值域校验已由 serde 反序列化完成。
    /// 双层 `on_exists`（源级 > 应用级，见 docs/config.md）是正常覆盖关系，
    /// 不存在冲突，因此不校验组合；这里只校验会导致运行期隐蔽错误的项：
    /// 空值、占位符合法性、路径分量安全性、同应用 target 冲突。
    pub fn validate(&self) -> Result<()> {
        if self.workspace.path.to_string_lossy().is_empty() {
            anyhow::bail!("Workspace path cannot be empty");
        }

        for (app_id, app_config) in &self.apps {
            if app_config.name.trim().is_empty() {
                anyhow::bail!("App '{}' has empty name", app_id);
            }

            check_workspace_relative(app_id, "name", &app_config.name)?;

            for source in &app_config.sources {
                if source.source.trim().is_empty() {
                    anyhow::bail!("App '{}' has empty source path", app_id);
                }

                if source.target.trim().is_empty() {
                    anyhow::bail!("App '{}' has empty target path", app_id);
                }

                check_placeholders(&source.source)
                    .map_err(|e| anyhow::anyhow!("App '{}': {}", app_id, e))?;
                check_workspace_relative(app_id, "target", &source.target)
                    .map_err(|e| anyhow::anyhow!("App '{}': {}", app_id, e))?;
            }

            check_target_conflicts(app_id, app_config)?;
        }

        Ok(())
    }

    /// 获取指定应用ID的配置
    pub fn get_app(&self, name: &str) -> Option<&AppConfig> {
        self.apps.get(name)
    }

    /// 获取所有启用的应用列表
    pub fn enabled_apps(&self) -> Vec<(&String, &AppConfig)> {
        self.apps.iter().filter(|(_, app)| app.enabled).collect()
    }
}

impl AppConfig {
    /// 获取生效策略：源级 `on_exists` > 应用级 `on_exists` > 默认 `Skip`
    #[must_use]
    pub fn effective_strategy(&self, source: &Source) -> OnExists {
        source.on_exists.or(self.on_exists).unwrap_or_default()
    }
}

/// 检查路径中的占位符是否都已注册（内置 + 自定义，注册表是唯一事实来源）
fn check_placeholders(path: &str) -> Result<()> {
    let mut start = 0;
    while let Some(open) = path[start..].find('<') {
        let abs_open = start + open;
        if let Some(close) = path[abs_open..].find('>') {
            let abs_close = abs_open + close + 1;
            let candidate = &path[abs_open..abs_close];

            if !is_known_placeholder(candidate) {
                anyhow::bail!("Unknown placeholder '{}' in path '{}'", candidate, path);
            }

            start = abs_close;
        } else {
            break;
        }
    }
    Ok(())
}

/// 检查应用内是否存在 target 冲突（多个 source 映射到相同 target）
fn check_target_conflicts(app_id: &str, app_config: &AppConfig) -> Result<()> {
    let mut seen = std::collections::HashMap::<&str, usize>::new();

    for (i, source) in app_config.sources.iter().enumerate() {
        if let Some(first) = seen.insert(source.target.as_str(), i) {
            anyhow::bail!(
                "App '{}' has target conflict: source[{}] and source[{}] both map to '{}'",
                app_id,
                first,
                i,
                source.target,
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_source(target: &str) -> Source {
        Source {
            source: "<home>/src".into(),
            target: target.into(),
            link_type: LinkType::Symlink,
            on_exists: None,
        }
    }

    fn sample_app(sources: Vec<Source>) -> AppConfig {
        AppConfig {
            name: "test-app".into(),
            enabled: true,
            on_exists: None,
            sources,
        }
    }

    // === check_placeholders 测试 ===

    #[test]
    fn test_known_placeholder_passes() {
        assert!(check_placeholders("<home>/AppData").is_ok());
        assert!(check_placeholders("<appdata>/test").is_ok());
        assert!(check_placeholders("<localappdata>/Data").is_ok());
        assert!(check_placeholders("<documents>/docs").is_ok());
        assert!(check_placeholders("<temp>/cache").is_ok());
    }

    #[test]
    fn test_no_placeholder_passes() {
        assert!(check_placeholders("C:/plain/path").is_ok());
        assert!(check_placeholders("D:/data/config").is_ok());
    }

    #[test]
    fn test_unknown_placeholder_fails() {
        let err = check_placeholders("<unknown>/test")
            .unwrap_err()
            .to_string();
        assert!(err.contains("Unknown placeholder"), "Got: {err}");
        assert!(err.contains("<unknown>"), "Got: {err}");
    }

    #[test]
    fn test_typo_placeholder_fails() {
        let err = check_placeholders("<hoome>/AppData")
            .unwrap_err()
            .to_string();
        assert!(err.contains("Unknown placeholder"));
    }

    #[test]
    fn test_mixed_known_and_unknown_fails() {
        let err = check_placeholders("<home>/<unkonwn>/data")
            .unwrap_err()
            .to_string();
        assert!(err.contains("Unknown placeholder"));
    }

    // === check_target_conflicts 测试 ===

    #[test]
    fn test_unique_targets_passes() {
        let app = sample_app(vec![sample_source("app/data"), sample_source("app/config")]);
        assert!(check_target_conflicts("test", &app).is_ok());
    }

    #[test]
    fn test_duplicate_target_fails() {
        let app = sample_app(vec![sample_source("app/data"), sample_source("app/data")]);
        let err = check_target_conflicts("test", &app)
            .unwrap_err()
            .to_string();
        assert!(err.contains("target conflict"));
    }

    #[test]
    fn test_three_sources_all_unique_passes() {
        let app = sample_app(vec![
            sample_source("app/data"),
            sample_source("app/config"),
            sample_source("app/cache"),
        ]);
        assert!(check_target_conflicts("test", &app).is_ok());
    }

    // === on_exists 策略覆盖语义测试 ===

    #[test]
    fn test_default_strategy_is_skip() {
        let app = sample_app(vec![]);
        assert_eq!(
            app.effective_strategy(&sample_source("dst")),
            OnExists::Skip
        );
    }

    #[test]
    fn test_app_level_strategy() {
        let app = AppConfig {
            name: "test".into(),
            enabled: true,
            on_exists: Some(OnExists::Replace),
            sources: vec![],
        };
        assert_eq!(
            app.effective_strategy(&sample_source("dst")),
            OnExists::Replace
        );
    }

    #[test]
    fn test_source_override_with_app_skip_is_valid() {
        // 应用级 skip + 源级覆盖是文档承诺的正常用法（源级 > 应用级），不应报错
        let app = sample_app(vec![Source {
            source: "<home>/src".into(),
            target: "dst".into(),
            link_type: LinkType::Symlink,
            on_exists: Some(OnExists::Merge),
        }]);
        let config = sample_config(app);
        assert!(
            config.validate().is_ok(),
            "source-level override must be accepted regardless of app-level strategy"
        );
    }

    #[test]
    fn test_mixed_app_and_source_strategies_are_valid() {
        let app = AppConfig {
            name: "test".into(),
            enabled: true,
            on_exists: Some(OnExists::Replace),
            sources: vec![
                sample_source("a"),
                Source {
                    source: "<home>/src".into(),
                    target: "b".into(),
                    link_type: LinkType::Symlink,
                    on_exists: Some(OnExists::Preserve),
                },
            ],
        };
        assert!(sample_config(app).validate().is_ok());
    }

    // === Config::validate 测试 ===

    /// 用单个应用构造最小可校验的 Config
    fn sample_config(app: AppConfig) -> Config {
        let mut apps = std::collections::BTreeMap::new();
        apps.insert("test".into(), app);
        Config {
            workspace: Workspace {
                path: PathBuf::from("D:/ws"),
            },
            apps,
            custom_placeholders: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_empty_workspace_path_fails() {
        let config = sample_config(sample_app(vec![]));
        let empty = Config {
            workspace: Workspace {
                path: PathBuf::new(),
            },
            ..config
        };
        assert!(empty.validate().is_err());
    }

    #[test]
    fn test_empty_app_name_fails() {
        let app = AppConfig {
            name: "".into(),
            ..sample_app(vec![])
        };
        assert!(sample_config(app).validate().is_err());
    }

    // === 工作区路径安全性测试 ===

    #[test]
    fn test_app_name_with_invalid_char_fails() {
        let app = AppConfig {
            name: "My:App".into(),
            ..sample_app(vec![])
        };
        let err = sample_config(app).validate().unwrap_err().to_string();
        assert!(err.contains("unsafe name"), "Got: {err}");
    }

    #[test]
    fn test_target_escaping_workspace_fails() {
        for bad_target in ["../escape", "C:/Windows", "/absolute"] {
            let app = sample_app(vec![sample_source(bad_target)]);
            let err = sample_config(app).validate().unwrap_err().to_string();
            assert!(
                err.contains("unsafe target"),
                "target '{bad_target}' should be rejected, got: {err}"
            );
        }
    }

    #[test]
    fn test_valid_multilevel_target_passes() {
        let app = sample_app(vec![sample_source("app/sub dir/data")]);
        assert!(sample_config(app).validate().is_ok());
    }

    // === serde 强类型反序列化测试 ===

    #[test]
    fn test_invalid_link_type_fails_at_deserialize() {
        let toml_str = r#"
[workspace]
path = "D:/ws"

[apps.bad]
name = "bad-link"

[[apps.bad.sources]]
source = "<home>/src"
target = "dst"
link_type = "invalid_link"
"#;
        let result: Result<Config, _> = toml::from_str(toml_str);
        assert!(
            result.is_err(),
            "Invalid link_type should fail deserialization"
        );
    }

    #[test]
    fn test_invalid_on_exists_fails_at_deserialize() {
        let toml_str = r#"
[workspace]
path = "D:/ws"

[apps.bad]
name = "bad-strategy"
on_exists = "bad_strategy"

[[apps.bad.sources]]
source = "<home>/src"
target = "dst"
"#;
        let result: Result<Config, _> = toml::from_str(toml_str);
        assert!(
            result.is_err(),
            "Invalid on_exists should fail deserialization"
        );
    }

    #[test]
    fn test_overwrite_alias_deserializes_to_preserve() {
        let toml_str = r#"
[workspace]
path = "D:/ws"

[apps.alias]
name = "alias-app"
on_exists = "overwrite"

[[apps.alias.sources]]
source = "<home>/src"
target = "dst"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.apps["alias"].on_exists, Some(OnExists::Preserve));
    }
}
