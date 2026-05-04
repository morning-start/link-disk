use std::path::PathBuf;
use tempfile::TempDir;
use link_disk::infra::{FsUtils, FsWriter, FsLinker};

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
    assert_eq!(std::fs::read_to_string(&target_file).unwrap(), "test content");
}

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
    assert!(!result_str.contains("~"));
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
    };

    assert_eq!(config.workspace.path, PathBuf::from("D:/test-workspace"));
    assert!(config.apps.is_empty());
}
