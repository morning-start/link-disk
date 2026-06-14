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
    /// 已知占位符列表（用于配置校验）
    pub const KNOWN_PLACEHOLDERS: &[&str] = &[
        "<home>",
        "<appdata>",
        "<localappdata>",
        "<documents>",
        "<desktop>",
        "<downloads>",
        "<temp>",
        "<programfiles>",
        "<programfilesx86>",
    ];
}

/// 顶层配置结构体
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// 工作区配置
    pub workspace: Workspace,
    /// 应用配置映射表 (key: 应用ID, value: 应用配置)
    #[serde(default)]
    pub apps: std::collections::HashMap<String, AppConfig>,
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

/// 检查应用级策略与源级策略之间是否存在冲突
///
/// 冲突模式：
/// - skip + (replace/merge/preserve): 应用级 skip 导致所有 source 策略不会生效
/// - replace/preserve + (merge/skip): 应用级销毁与源级保护矛盾
fn check_strategy_conflicts(app_id: &str, app_config: &AppConfig) -> Result<()> {
    let app_strategy = app_config.on_exists_or_default();

    for (i, source) in app_config.sources.iter().enumerate() {
        let Some(src_strategy) = source.on_exists else {
            continue;
        };

        // 冲突模式 1: app 为 skip 但 source 为其他策略
        if app_strategy == OnExists::Skip && src_strategy != OnExists::Skip {
            anyhow::bail!(
                "App '{}' strategy is 'skip' but source[{}] strategy is '{:?}'. \
                 All source strategies are ignored when app strategy is 'skip'. \
                 Either set app strategy to '{:?}' or remove the source-level override.",
                app_id,
                i,
                src_strategy,
                src_strategy,
            );
        }

        // 冲突模式 2: app 为 replace/preserve 但 source 为 merge/skip
        if matches!(app_strategy, OnExists::Replace | OnExists::Preserve)
            && matches!(src_strategy, OnExists::Merge | OnExists::Skip)
        {
            anyhow::bail!(
                "App '{}' strategy is '{:?}' (destructive) but source[{}] strategy is '{:?}' (preserving). \
                 This creates conflicting behaviors: the app-level strategy will override the source-level. \
                 Consider aligning the strategies.",
                app_id,
                app_strategy,
                i,
                src_strategy,
            );
        }
    }

    Ok(())
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
    /// 注意：`link_type` 和 `on_exists` 的值域校验已由 serde 反序列化完成，
    /// 此处仅校验结构关系（空值、占位符合法性、target 冲突、策略冲突）。
    pub fn validate(&self) -> Result<()> {
        if self.workspace.path.to_string_lossy().is_empty() {
            anyhow::bail!("Workspace path cannot be empty");
        }

        for (app_id, app_config) in &self.apps {
            if app_config.name.trim().is_empty() {
                anyhow::bail!("App '{}' has empty name", app_id);
            }

            for source in &app_config.sources {
                if source.source.trim().is_empty() {
                    anyhow::bail!("App '{}' has empty source path", app_id);
                }

                if source.target.trim().is_empty() {
                    anyhow::bail!("App '{}' has empty target path", app_id);
                }

                check_placeholders(&source.source)
                    .map_err(|e| anyhow::anyhow!("App '{}': {}", app_id, e))?;
            }

            check_target_conflicts(app_id, app_config)?;
            check_strategy_conflicts(app_id, app_config)?;
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
    /// 获取目标已存在时的处理策略，未配置则返回默认 Skip
    pub fn on_exists_or_default(&self) -> OnExists {
        self.on_exists.unwrap_or_default()
    }
}

/// 检查路径中的占位符是否都是已知的（含运行时注册的自定义占位符）
fn check_placeholders(path: &str) -> Result<()> {
    let mut start = 0;
    while let Some(open) = path[start..].find('<') {
        let abs_open = start + open;
        if let Some(close) = path[abs_open..].find('>') {
            let abs_close = abs_open + close + 1;
            let candidate = &path[abs_open..abs_close];
            let is_known = constants::KNOWN_PLACEHOLDERS.contains(&candidate)
                || is_known_placeholder(candidate);

            if !is_known {
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

    for source in &app_config.sources {
        if let Some(index) = seen.get(source.target.as_str()) {
            anyhow::bail!(
                "App '{}' has target conflict: source[{}] and source[{}] both map to '{}'",
                app_id,
                index,
                seen.len(),
                source.target,
            );
        }
        seen.insert(source.target.as_str(), seen.len());
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

    // === on_exists 策略优先级测试 ===

    #[test]
    fn test_default_strategy_is_skip() {
        let app = AppConfig {
            name: "test".into(),
            enabled: true,
            on_exists: None,
            sources: vec![],
        };
        assert_eq!(app.on_exists_or_default(), OnExists::Skip);
    }

    #[test]
    fn test_app_level_strategy() {
        let app = AppConfig {
            name: "test".into(),
            enabled: true,
            on_exists: Some(OnExists::Replace),
            sources: vec![],
        };
        assert_eq!(app.on_exists_or_default(), OnExists::Replace);
    }

    // === Config::validate 测试 ===

    #[test]
    fn test_empty_workspace_path_fails() {
        use std::collections::HashMap;
        let config = Config {
            workspace: Workspace {
                path: PathBuf::new(),
            },
            apps: HashMap::new(),
            custom_placeholders: HashMap::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_empty_app_name_fails() {
        let app = AppConfig {
            name: "".into(),
            enabled: true,
            on_exists: None,
            sources: vec![],
        };
        let mut apps = std::collections::HashMap::new();
        apps.insert("empty".into(), app);
        let config = Config {
            workspace: Workspace {
                path: PathBuf::from("D:/ws"),
            },
            apps,
            custom_placeholders: std::collections::HashMap::new(),
        };
        assert!(config.validate().is_err());
    }

    // === 策略冲突测试 ===

    #[test]
    fn test_app_skip_with_source_replace_fails() {
        let app = AppConfig {
            name: "test".into(),
            enabled: true,
            on_exists: Some(OnExists::Skip),
            sources: vec![Source {
                source: "<home>/src".into(),
                target: "dst".into(),
                link_type: LinkType::Symlink,
                on_exists: Some(OnExists::Replace),
            }],
        };
        let mut apps = std::collections::HashMap::new();
        apps.insert("test".into(), app);
        let config = Config {
            workspace: Workspace {
                path: PathBuf::from("D:/ws"),
            },
            apps,
            custom_placeholders: std::collections::HashMap::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_app_skip_with_source_merge_fails() {
        let app = AppConfig {
            name: "test".into(),
            enabled: true,
            on_exists: Some(OnExists::Skip),
            sources: vec![Source {
                source: "<home>/src".into(),
                target: "dst".into(),
                link_type: LinkType::Symlink,
                on_exists: Some(OnExists::Merge),
            }],
        };
        let mut apps = std::collections::HashMap::new();
        apps.insert("test".into(), app);
        let config = Config {
            workspace: Workspace {
                path: PathBuf::from("D:/ws"),
            },
            apps,
            custom_placeholders: std::collections::HashMap::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_app_replace_with_source_merge_fails() {
        let app = AppConfig {
            name: "test".into(),
            enabled: true,
            on_exists: Some(OnExists::Replace),
            sources: vec![Source {
                source: "<home>/src".into(),
                target: "dst".into(),
                link_type: LinkType::Symlink,
                on_exists: Some(OnExists::Merge),
            }],
        };
        let mut apps = std::collections::HashMap::new();
        apps.insert("test".into(), app);
        let config = Config {
            workspace: Workspace {
                path: PathBuf::from("D:/ws"),
            },
            apps,
            custom_placeholders: std::collections::HashMap::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_app_preserve_with_source_skip_fails() {
        let app = AppConfig {
            name: "test".into(),
            enabled: true,
            on_exists: Some(OnExists::Preserve),
            sources: vec![Source {
                source: "<home>/src".into(),
                target: "dst".into(),
                link_type: LinkType::Symlink,
                on_exists: Some(OnExists::Skip),
            }],
        };
        let mut apps = std::collections::HashMap::new();
        apps.insert("test".into(), app);
        let config = Config {
            workspace: Workspace {
                path: PathBuf::from("D:/ws"),
            },
            apps,
            custom_placeholders: std::collections::HashMap::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_no_conflict_without_source_override() {
        let app = AppConfig {
            name: "test".into(),
            enabled: true,
            on_exists: Some(OnExists::Replace),
            sources: vec![Source {
                source: "<home>/src".into(),
                target: "dst".into(),
                link_type: LinkType::Symlink,
                on_exists: None,
            }],
        };
        let mut apps = std::collections::HashMap::new();
        apps.insert("test".into(), app);
        let config = Config {
            workspace: Workspace {
                path: PathBuf::from("D:/ws"),
            },
            apps,
            custom_placeholders: std::collections::HashMap::new(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_no_conflict_consistent_strategies() {
        let app = AppConfig {
            name: "test".into(),
            enabled: true,
            on_exists: Some(OnExists::Replace),
            sources: vec![Source {
                source: "<home>/src".into(),
                target: "dst".into(),
                link_type: LinkType::Symlink,
                on_exists: Some(OnExists::Replace),
            }],
        };
        let mut apps = std::collections::HashMap::new();
        apps.insert("test".into(), app);
        let config = Config {
            workspace: Workspace {
                path: PathBuf::from("D:/ws"),
            },
            apps,
            custom_placeholders: std::collections::HashMap::new(),
        };
        assert!(config.validate().is_ok());
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
