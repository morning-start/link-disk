use link_disk::infra::{FileSystem, FsUtils};
use std::path::PathBuf;
use tempfile::TempDir;

fn setup_test_env_with_source() -> (TempDir, PathBuf, PathBuf) {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    std::fs::create_dir_all(&source).unwrap();
    (temp, source, target)
}

fn setup_test_env_empty() -> (TempDir, PathBuf, PathBuf) {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    (temp, source, target)
}

// 以下测试依赖符号链接创建权限。
// Windows 上默认需要管理员或开发者模式（错误码 1314），无特权的 CI 上标记为 ignored。
// 拥有权限时执行 `cargo test -- --ignored` 可跑完整链路。
#[cfg_attr(
    windows,
    ignore = "requires admin or developer mode to create symlinks"
)]
#[test]
fn test_symlink_directory_creation() {
    let (_temp, source, target) = setup_test_env_with_source();

    let fs = FsUtils;
    fs.ensure_parent_exists(&target).unwrap();
    fs.remove_if_exists(&target).unwrap();
    fs.create_symlink(&source, &target).unwrap();

    assert!(target.is_symlink());
    assert_eq!(std::fs::read_link(&target).unwrap(), source);
}

#[test]
fn test_symlink_file_creation() {
    let (_temp, source, target) = setup_test_env_with_source();
    let source_file = source.join("test.txt");
    std::fs::write(&source_file, "test content").unwrap();

    let fs = FsUtils;
    let target_file = target.join("test_link.txt");
    fs.ensure_parent_exists(&target_file).unwrap();
    fs.hard_link(&source_file, &target_file).unwrap();

    assert!(target_file.exists());
    assert!(!target_file.is_symlink());
    assert_eq!(
        std::fs::read_to_string(&target_file).unwrap(),
        "test content"
    );
}

#[cfg_attr(
    windows,
    ignore = "requires admin or developer mode to create symlinks"
)]
#[test]
fn test_symlink_removal() {
    let (_temp, source, target) = setup_test_env_with_source();

    let fs = FsUtils;
    fs.ensure_parent_exists(&target).unwrap();
    fs.remove_if_exists(&target).unwrap();
    fs.create_symlink(&source, &target).unwrap();
    assert!(target.is_symlink());

    fs.remove_if_exists(&target).unwrap();
    assert!(!target.exists());
}

#[test]
fn test_hardlink_creation() {
    let (_temp, source, target) = setup_test_env_with_source();
    let source_file = source.join("test.txt");
    std::fs::write(&source_file, "test content").unwrap();

    let fs = FsUtils;
    fs.hard_link(&source_file, &target).unwrap();

    assert!(target.exists());
    assert!(!target.is_symlink());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "test content");
}

#[test]
fn test_link_status_none() {
    let (temp, _, _) = setup_test_env_empty();
    let source = temp.path().join("nonexistent_src");
    let target = temp.path().join("nonexistent_tgt");

    let status = link_disk::domain::LinkStatusChecker::check(&source, &target);
    assert_eq!(status, link_disk::domain::LinkStatus::None);
}

#[test]
fn test_link_status_source_only() {
    let (_temp, source, target) = setup_test_env_with_source();

    let status = link_disk::domain::LinkStatusChecker::check(&source, &target);
    assert_eq!(status, link_disk::domain::LinkStatus::SourceOnly);
}

#[test]
fn test_link_status_target_only() {
    let (_temp, source, target) = setup_test_env_empty();
    std::fs::create_dir_all(&target).unwrap();

    let status = link_disk::domain::LinkStatusChecker::check(&source, &target);
    assert_eq!(status, link_disk::domain::LinkStatus::TargetOnly);
}

#[cfg_attr(
    windows,
    ignore = "requires admin or developer mode to create symlinks"
)]
#[test]
fn test_link_status_linked() {
    let (temp, _, _) = setup_test_env_empty();
    let target = temp.path().join("target");
    let source = temp.path().join("source");

    std::fs::create_dir_all(&target).unwrap();

    let fs = FsUtils;
    fs.remove_if_exists(&source).unwrap();
    fs.create_symlink(&target, &source).unwrap();

    let status = link_disk::domain::LinkStatusChecker::check(&source, &target);
    assert_eq!(status, link_disk::domain::LinkStatus::Linked);
}

#[test]
fn test_path_resolver_expand_home() {
    let result = link_disk::infra::PathResolver::expand_home("~/test");
    let result_str = result.to_string_lossy();
    assert!(!result_str.contains('~'));
    assert!(result_str.contains("Users") || result_str.contains("home"));
}

#[test]
fn test_path_resolver_expand_appdata() {
    let result = link_disk::infra::PathResolver::expand("<appdata>/test");
    assert!(!result.contains("<appdata>"));
    assert!(result.contains("AppData"));
    assert!(result.ends_with("/test") || result.ends_with("\\test"));
}

#[test]
fn test_path_resolver_expand_localappdata() {
    let result = link_disk::infra::PathResolver::expand("<localappdata>/test");
    assert!(!result.contains("<localappdata>"));
    assert!(result.contains("AppData"));
    assert!(result.ends_with("/test") || result.ends_with("\\test"));
}

#[test]
fn test_config_workspace() {
    use link_disk::infra::{Config, ConfigWorkspace};
    use std::collections::HashMap;

    let config = Config {
        workspace: ConfigWorkspace {
            path: PathBuf::from("D:/test-workspace"),
        },
        apps: HashMap::new(),
        custom_placeholders: HashMap::new(),
    };

    assert_eq!(config.workspace.path, PathBuf::from("D:/test-workspace"));
    assert!(config.apps.is_empty());
}

// === 配置校验集成测试 ===

#[test]
fn test_config_validate_valid_toml() {
    use link_disk::infra::Config;
    let toml_str = r#"
[workspace]
path = "D:/workspace"

[apps.test]
name = "Test App"
on_exists = "skip"

[[apps.test.sources]]
source = "<home>/AppData/Test"
target = "test/data"
link_type = "symlink"
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_validate_unknown_placeholder() {
    use link_disk::infra::Config;
    let toml_str = r#"
[workspace]
path = "D:/workspace"

[apps.test]
name = "Test App"

[[apps.test.sources]]
source = "<unknown>/path"
target = "test/data"
link_type = "symlink"
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.validate().is_err());
}

#[test]
fn test_config_validate_target_conflict() {
    use link_disk::infra::Config;
    let toml_str = r#"
[workspace]
path = "D:/workspace"

[apps.test]
name = "Test App"

[[apps.test.sources]]
source = "<home>/A"
target = "app/data"
link_type = "symlink"

[[apps.test.sources]]
source = "<home>/B"
target = "app/data"
link_type = "symlink"
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.validate().is_err());
}

#[test]
fn test_config_validate_app_toml_file() {
    use link_disk::infra::Config;
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");

    let toml_str = r#"
[workspace]
path = "D:/workspace"

[apps.test]
name = "Test App"

[[apps.test.sources]]
source = "<home>/Data"
target = "test/data"
link_type = "symlink"
"#;
    std::fs::write(&config_path, toml_str).unwrap();

    let config = Config::load(&config_path).unwrap();
    assert!(config.validate().is_ok());
}

// === LinkOps 完整流程测试 ===

#[cfg_attr(
    windows,
    ignore = "requires admin or developer mode to create symlinks"
)]
#[test]
fn test_link_ops_full_link_and_unlink() {
    use link_disk::domain::{LinkOps, LinkRequest, LinkType, OnExists};

    let (_temp, source, target) = setup_test_env_with_source();
    std::fs::write(source.join("config.txt"), "config data").unwrap();

    let request = LinkRequest {
        source: source.clone(),
        target: target.clone(),
        link_type: LinkType::Symlink,
        on_exists: OnExists::Replace,
        force: false,
    };

    let fs = FsUtils;
    let link_result = LinkOps::link_with_fs(&request, &fs, false);
    assert!(link_result.is_ok(), "Link failed: {:?}", link_result.err());

    // source 成为指向 target 的符号链接
    assert!(source.is_symlink());
    assert!(target.is_dir());
    assert!(target.join("config.txt").exists());

    let unlink_result = LinkOps::unlink_with_fs(&source, &target, false, &fs);
    assert!(
        unlink_result.is_ok(),
        "Unlink failed: {:?}",
        unlink_result.err()
    );
    // keep_files=false: 文件移回 source, target 被删除
    assert!(!target.exists());
    assert!(source.join("config.txt").exists());
}

#[cfg_attr(
    windows,
    ignore = "requires admin or developer mode to create symlinks"
)]
#[test]
fn test_link_ops_with_replace_strategy() {
    use link_disk::domain::{LinkOps, LinkRequest, LinkType, OnExists};

    let (_temp, source, target) = setup_test_env_with_source();
    std::fs::write(source.join("data.txt"), "data").unwrap();
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("old.txt"), "old").unwrap();

    let request = LinkRequest {
        source: source.clone(),
        target: target.clone(),
        link_type: LinkType::Symlink,
        on_exists: OnExists::Replace,
        force: false,
    };

    let fs = FsUtils;
    let link_result = LinkOps::link_with_fs(&request, &fs, false);
    assert!(link_result.is_ok());

    // source 成为 symlink，target 包含合并后的数据
    assert!(target.is_dir());
    assert!(target.join("data.txt").exists());
    assert!(!target.join("old.txt").exists());
}

// === 请求构建优先级测试 ===

#[test]
fn test_build_link_request_source_level_on_exists_priority() {
    use link_disk::domain::OnExists;
    use link_disk::infra::build_link_request;

    let temp = TempDir::new().unwrap();
    let workspace_path = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace_path).unwrap();

    let app = link_disk::infra::AppConfig {
        name: "test-app".into(),
        enabled: true,
        on_exists: Some(OnExists::Skip),
        sources: vec![link_disk::infra::Source {
            source: "<home>/Test".into(),
            target: "app/data".into(),
            link_type: link_disk::domain::LinkType::Symlink,
            on_exists: Some(OnExists::Replace),
        }],
    };

    let (request, _, _) = build_link_request(&app, &app.sources[0], &workspace_path, false);
    assert_eq!(request.on_exists, OnExists::Replace);
}

#[test]
fn test_build_link_request_falls_back_to_app_level() {
    use link_disk::domain::OnExists;
    use link_disk::infra::build_link_request;

    let temp = TempDir::new().unwrap();
    let workspace_path = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace_path).unwrap();

    let app = link_disk::infra::AppConfig {
        name: "test-app".into(),
        enabled: true,
        on_exists: Some(OnExists::Merge),
        sources: vec![link_disk::infra::Source {
            source: "<home>/Test".into(),
            target: "app/data".into(),
            link_type: link_disk::domain::LinkType::Symlink,
            on_exists: None,
        }],
    };

    let (request, _, _) = build_link_request(&app, &app.sources[0], &workspace_path, false);
    assert_eq!(request.on_exists, OnExists::Merge);
}
