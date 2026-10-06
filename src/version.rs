//! 版本信息
//!
//! 单一版本来源：build.rs 在编译期从 git tags 推导并注入
//! [`VERSION`]，`--version` 输出与 `selfupdate` 的版本比较共用此值，
//! 消除 "git tag 是 v2.2.1 而二进制自报 1.1.0" 的脱节问题。

/// 完整版本号（如 `"2.3.0"`），与仓库最新 git tag 一致
pub const VERSION: &str = env!("LINK_DISK_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver_like() {
        assert!(
            VERSION.split('.').count() >= 2
                && VERSION.chars().next().is_some_and(|c| c.is_ascii_digit()),
            "VERSION should look like a semver, got: {VERSION}"
        );
    }
}
