//! 构建脚本：从 git tags 推导版本号
//!
//! 版本来源优先级：
//! 1. `git describe --tags` 的最新标签（取首个 `-` 前的基本段，剥离前导 `v`）
//!    —— HEAD 停在 tag 上得到 `2.3.0`；开发提交得到最近一次发布的版本
//! 2. 回退 `Cargo.toml` 的 `version` 字段（无 `.git` 或无任何 tag 的场景，
//!    如从源码包构建）
//!
//! 产出 `LINK_DISK_VERSION` 环境变量，供 `crate::version::VERSION` 读取，
//! 保证 `--version`、`selfupdate` 的版本判断与 git tags 保持一致。

use std::process::Command;

fn main() {
    let version = git_tag_version().unwrap_or_else(|| {
        std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string())
    });

    println!("cargo:rustc-env=LINK_DISK_VERSION={version}");

    // git 状态变化时重新计算版本（HEAD 变动 / 新增 tag / fetch 更新 packed-refs）
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
    println!("cargo:rerun-if-changed=.git/packed-refs");
}

/// 从 `git describe --tags` 提取基本版本号
///
/// - `v2.3.0`                → `2.3.0`
/// - `v2.3.0-3-gacf823b`     → `2.3.0`（取 tag 基本段，忽略提交距离）
/// - `v2.3.0-dirty`          → `2.3.0`
/// - 无 tag / 无 git         → `None`（调用方回退 Cargo.toml）
fn git_tag_version() -> Option<String> {
    let output = Command::new("git")
        .args(["describe", "--tags"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let describe = String::from_utf8_lossy(&output.stdout);
    let describe = describe.trim();
    if describe.is_empty() {
        return None;
    }

    let base = describe.split('-').next()?;
    let version = base.strip_prefix('v').unwrap_or(base);
    if version.is_empty() || !version.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(version.to_string())
}
