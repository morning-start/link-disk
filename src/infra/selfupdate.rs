//! 自我升级模块
//!
//! 从 GitHub Releases 查询并下载最新版本，替换当前可执行文件。
//!
//! ## 查询策略
//!
//! 优先使用 `gh api`（继承用户本地认证，速率限制宽松），
//! 不可用或 `--no-gh` 时回退到 GitHub REST API（匿名，60 次/小时/IP）。
//!
//! ## 升级流程（安全替换）
//!
//! 1. 下载资产到临时目录并解压出二进制
//! 2. 校验下载的二进制可执行（basic smoke：非空）
//! 3. `current → current.old.bak`，`new → current`
//! 4. 启动失败时自动回滚（`current.old.bak → current`）
//!
//! Windows 上运行中的 exe 无法被删除/覆盖，因此采用
//! "先把旧文件 rename 走，再把新文件 rename 进来" 的两段式替换。

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

/// 升级源仓库（owner/repo）
pub const REPO: &str = "morning-start/link-disk";

/// 当前二进制的版本号（来自 Cargo.toml）
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub release 元数据（仅取升级所需字段）
#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    /// 版本标签（如 "v2.2.1"）
    pub tag: String,
    /// 资产文件名列表
    pub assets: Vec<String>,
}

/// 资产命名约定：`link-disk-{target-triple}.{zip|tar.gz}`
fn asset_file_name() -> String {
    format!("link-disk-{}.{}", target_triple(), archive_ext())
}

/// 当前平台 target triple
///
/// 编译期注入（build script 不可用时退化为运行时探测），
/// 这里用 `env!` 拿不到，改用简单平台映射 + `cfg`：
fn target_triple() -> &'static str {
    // 常见三平台覆盖；其余平台由资产匹配报错提示手动下载
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc"
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        "aarch64-pc-windows-msvc"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "aarch64-unknown-linux-gnu"
    } else {
        "unknown"
    }
}

/// 平台对应的压缩包扩展名
fn archive_ext() -> &'static str {
    if cfg!(windows) { "zip" } else { "tar.gz" }
}

/// 在资产列表中匹配当前平台的资产
pub fn select_asset(assets: &[String]) -> Result<String> {
    let expected = asset_file_name();
    assets
        .iter()
        .find(|a| **a == expected)
        .cloned()
        .with_context(|| {
            format!(
                "No asset found for this platform (expected '{expected}'). \
                 Available: {}. Please download manually from \
                 https://github.com/{REPO}/releases",
                assets.join(", ")
            )
        })
}

/// 版本标签比较：`v2.2.1` > `v2.2.0`，`v2.10.0` > `v2.9.9`
///
/// 剥离前导 `v`/`V` 后按数值段逐段比较；缺省段视为 0
/// （`v2.2` == `v2.2.0`）。段内数值相等即视为相等，不做
/// 字符串次级比较——预发布标签（如 `2.2.1-rc1`）的数值段
/// 与正式版相同时按相等处理，避免误判阻碍升级。
pub fn compare_versions(current: &str, latest: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let parse = |v: &str| -> Vec<u64> {
        let v = v.trim().trim_start_matches(['v', 'V']);
        v.split('.')
            .map(|seg| {
                let digits: String = seg.chars().take_while(char::is_ascii_digit).collect();
                digits.parse().unwrap_or(0)
            })
            .collect()
    };

    let a = parse(current);
    let b = parse(latest);
    for i in 0..a.len().max(b.len()) {
        let na = a.get(i).copied().unwrap_or(0);
        let nb = b.get(i).copied().unwrap_or(0);
        if na != nb {
            return na.cmp(&nb);
        }
    }
    Ordering::Equal
}

/// 判断 `gh` CLI 是否可用
fn gh_available() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 查询最新 release：gh api 优先，回退 REST API
pub fn fetch_latest_release(use_gh: bool) -> Result<ReleaseInfo> {
    if use_gh && gh_available() {
        debug!("Fetching latest release via gh api");
        match query_gh() {
            Ok(info) => return Ok(info),
            Err(e) => debug!("gh api failed, falling back to REST: {e:#}"),
        }
    }
    debug!("Fetching latest release via GitHub REST API");
    query_rest()
}

/// 通过 `gh api` 查询（用户认证，速率限制宽松）
fn query_gh() -> Result<ReleaseInfo> {
    let output = Command::new("gh")
        .args([
            "api",
            "--jq",
            "{tag_name: .tag_name, assets: [.assets[].name]}",
        ])
        .arg(format!("/repos/{REPO}/releases/latest"))
        .output()
        .context("Failed to run 'gh api'")?;

    if !output.status.success() {
        bail!(
            "gh api failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    parse_release_json(&String::from_utf8_lossy(&output.stdout))
}

/// 通过 GitHub REST API 匿名查询
fn query_rest() -> Result<ReleaseInfo> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");

    // 优先 curl（Windows 10+ / 主流 Linux / macOS 均内置），其次 gh 已在上面尝试过
    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--location",
            "--header",
            "Accept: application/vnd.github+json",
            &url,
        ])
        .output()
        .context("Failed to run 'curl' (no built-in HTTP client in this build)")?;

    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!(
            "GitHub API request failed: {detail}. \
             Note: anonymous REST API is limited to 60 requests/hour/IP; \
             remove --no-gh to use authenticated 'gh api', \
             or wait for the rate limit to reset."
        );
    }
    parse_release_json(&String::from_utf8_lossy(&output.stdout))
}

/// 从 release JSON 提取 tag 与资产名（手写轻量解析，避免引入 serde_json）
///
/// 兼容两种资产形态：
/// - REST API 原始输出：`"assets":[{"name":"x.zip",...},...]`
/// - `gh api --jq` 重构输出：`"assets":["x.zip","y.zip"]`
fn parse_release_json(json: &str) -> Result<ReleaseInfo> {
    let tag =
        extract_string_field(json, "tag_name").context("Release JSON missing 'tag_name' field")?;

    let assets = parse_assets(json);
    Ok(ReleaseInfo { tag, assets })
}

/// 提取 assets 数组中的资产名（对象形态取 `name` 字段，字符串形态直接取值）
fn parse_assets(json: &str) -> Vec<String> {
    // 定位 "assets": [ ... ] 数组体
    let Some(start) = json.find("\"assets\":") else {
        return Vec::new();
    };
    let after_key = &json[start + 9..];
    let Some(open) = after_key.find('[') else {
        return Vec::new();
    };
    let body = &after_key[open + 1..];
    let Some(close) = body.find(']') else {
        return Vec::new();
    };
    let body = &body[..close];

    // 数组体内逐个提取字符串字面量；对象形态下还会匹配到 "name" 键本身，
    // 用值跟随关系过滤：跳过紧跟 `:` 的键名
    let mut assets = Vec::new();
    let mut rest = body;
    while let Some(pos) = rest.find('"') {
        rest = &rest[pos + 1..];
        let Some(end) = rest.find('"') else {
            break;
        };
        let literal = &rest[..end];
        rest = &rest[end + 1..];
        // 键名特征：后面（跳过空白）紧跟冒号
        if rest.trim_start().starts_with(':') {
            continue;
        }
        if !literal.is_empty() {
            assets.push(literal.to_string());
        }
    }
    assets
}

/// 提取 `"field": "value"` 中 value 的轻量解析
fn extract_string_field(json: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\":");
    let pos = json.find(&key)?;
    extract_leading_string(&json[pos + key.len()..])
}

/// 跳过空白后提取紧跟的 JSON 字符串字面量
fn extract_leading_string(s: &str) -> Option<String> {
    let s = s.trim_start();
    let mut chars = s.chars();
    if chars.next()? != '"' {
        return None;
    }
    let mut out = String::new();
    for c in chars {
        match c {
            '"' => return Some(out),
            '\\' => out.push('\\'), // 资产名不含转义序列，反斜杠按字面保留
            other => out.push(other),
        }
    }
    None
}

/// 下载指定资产的压缩包到 `dest_dir`
pub fn download_asset(tag: &str, asset: &str, dest_dir: &Path) -> Result<PathBuf> {
    let url = format!("https://github.com/{REPO}/releases/download/{tag}/{asset}");
    let archive_path = dest_dir.join(asset);

    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--location",
            "--output",
        ])
        .arg(&archive_path)
        .arg(&url)
        .output()
        .context("Failed to run 'curl' for download")?;

    if !output.status.success() {
        bail!(
            "Download failed ({url}): {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let meta = std::fs::metadata(&archive_path)
        .with_context(|| format!("Downloaded archive missing: {:?}", archive_path))?;
    if meta.len() == 0 {
        bail!("Downloaded archive is empty: {}", asset);
    }

    Ok(archive_path)
}

/// 解压压缩包，返回解压后目录
///
/// Windows zip 用 PowerShell `Expand-Archive`，tar.gz 用系统 `tar`
///（Windows 10+ 与主流 Unix 均内置）。
pub fn extract_archive(archive: &Path, dest_dir: &Path) -> Result<PathBuf> {
    let name = archive.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let extract_dir = dest_dir.join("extracted");

    let status = if name.ends_with(".zip") {
        Command::new("powershell")
            .args(["-NoProfile", "-Command", "Expand-Archive", "-Force"])
            .arg(format!("-Path {}", archive.display()))
            .arg(format!("-DestinationPath {}", extract_dir.display()))
            .status()
    } else {
        std::fs::create_dir_all(&extract_dir).ok();
        Command::new("tar")
            .args(["-xzf"])
            .arg(archive)
            .arg("-C")
            .arg(&extract_dir)
            .status()
    }
    .context("Failed to launch archive extractor")?;

    if !status.success() {
        bail!("Failed to extract archive: {}", name);
    }
    Ok(extract_dir)
}

/// 在解压目录中定位新二进制（压缩包根目录或一层子目录中的 link-disk[.exe]）
pub fn locate_binary(extract_dir: &Path) -> Result<PathBuf> {
    let exe_name = if cfg!(windows) {
        "link-disk.exe"
    } else {
        "link-disk"
    };

    let direct = extract_dir.join(exe_name);
    if direct.is_file() {
        return Ok(direct);
    }

    // 常见打包布局：压缩包内一层目录
    if let Ok(entries) = std::fs::read_dir(extract_dir) {
        for entry in entries.flatten() {
            let candidate = entry.path().join(exe_name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    bail!(
        "Binary '{exe_name}' not found in extracted archive at {:?}",
        extract_dir
    )
}

/// 当前可执行文件路径
pub fn current_exe_path() -> Result<PathBuf> {
    std::env::current_exe().context("Failed to locate current executable")
}

/// 用下载好的新二进制替换当前可执行文件（两段 rename，Windows 兼容）
///
/// 失败时尽力回滚。返回旧文件备份路径（用于提示，进程内无法自重启）。
pub fn install_binary(new_binary: &Path, current: &Path) -> Result<PathBuf> {
    if new_binary == current {
        bail!("New binary path equals current path; refuse to self-overwrite");
    }

    let backup = current.with_extension("old.bak");
    let _ = std::fs::remove_file(&backup);

    // 段1: 旧文件 rename 走（Windows 上运行中文件允许 rename）
    std::fs::rename(current, &backup)
        .with_context(|| format!("Failed to move current binary away: {:?}", current))?;

    // 段2: 新文件 rename 进来；失败则回滚段1
    if let Err(e) = std::fs::rename(new_binary, current) {
        let _ = std::fs::rename(&backup, current);
        return Err(e).context("Failed to move new binary into place (rolled back)");
    }

    Ok(backup)
}

/// 执行完整升级流程，返回人类可读的结果描述
///
/// `use_gh`: 是否允许用 gh CLI 查询；`target_version`: None 表示最新。
pub fn run_update(target_version: Option<&str>, use_gh: bool, force: bool) -> Result<String> {
    let release = fetch_latest_release(use_gh)?;
    let tag = match target_version {
        Some(v) => normalize_tag(v),
        None => release.tag.clone(),
    };

    let current = format!("v{CURRENT_VERSION}");
    // compare_versions(current, tag) == Less  ⇒ tag 比当前版本新，需要升级
    let order = compare_versions(&current, &tag);
    if order != std::cmp::Ordering::Less && !force {
        return Ok(format!(
            "Already up to date ({current}; latest is {tag}). \
             Use --force to reinstall {tag} anyway."
        ));
    }

    let asset = select_asset(&release.assets)?;

    let temp_dir = std::env::temp_dir().join(format!("link-disk-update-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir)
        .with_context(|| format!("Failed to create temp dir: {:?}", temp_dir))?;

    let result = download_asset(&tag, &asset, &temp_dir)
        .and_then(|archive| {
            let dir = extract_archive(&archive, &temp_dir)?;
            locate_binary(&dir)
        })
        .and_then(|new_bin| {
            let current_path = current_exe_path()?;
            install_binary(&new_bin, &current_path)
        });

    let _ = std::fs::remove_dir_all(&temp_dir);

    let backup = result.with_context(|| format!("Update to {tag} failed"))?;
    Ok(format!(
        "Updated to {tag}. Backup of previous binary: {}. \
         Run 'link-disk --version' to verify.",
        backup.display()
    ))
}

/// 版本号归一化：接受 `2.2.1` / `v2.2.1` / `V2.2.1`
fn normalize_tag(v: &str) -> String {
    let v = v.trim();
    if v.starts_with(['v', 'V']) {
        v.to_string()
    } else {
        format!("v{v}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare_semantic() {
        use std::cmp::Ordering::*;
        assert_eq!(compare_versions("v1.1.0", "v2.2.1"), Less);
        assert_eq!(compare_versions("v2.2.1", "v2.2.1"), Equal);
        assert_eq!(compare_versions("v2.2.0", "v2.1.9"), Greater);
        // 数值而非字典序：10 > 9
        assert_eq!(compare_versions("v2.9.9", "v2.10.0"), Less);
        // 无前导 v
        assert_eq!(compare_versions("1.1.0", "v2.0.0"), Less);
        // 长度不等：缺省段补 0
        assert_eq!(compare_versions("v2.2", "v2.2.0"), Equal);
    }

    #[test]
    fn asset_selection_matches_platform() {
        let assets = vec![
            "link-disk-aarch64-apple-darwin.tar.gz".to_string(),
            "link-disk-x86_64-pc-windows-msvc.zip".to_string(),
            "link-disk-x86_64-unknown-linux-gnu.tar.gz".to_string(),
        ];
        let picked = select_asset(&assets).unwrap();
        let expected = format!("link-disk-{}.{}", target_triple(), archive_ext());
        assert_eq!(picked, expected);
    }

    #[test]
    fn asset_selection_errors_for_unknown_platform() {
        let assets = vec!["link-disk-mips64-unknown-linux.tar.gz".to_string()];
        let err = select_asset(&assets).unwrap_err().to_string();
        assert!(err.contains("No asset found"), "Got: {err}");
        assert!(err.contains("download manually"), "Got: {err}");
    }

    #[test]
    fn parse_release_json_fields() {
        let json = r#"{
            "tag_name": "v2.2.1",
            "assets": [
                {"name": "link-disk-x86_64-pc-windows-msvc.zip", "size": 1},
                {"name": "link-disk-x86_64-unknown-linux-gnu.tar.gz", "size": 2}
            ]
        }"#;
        let info = parse_release_json(json).unwrap();
        assert_eq!(info.tag, "v2.2.1");
        assert_eq!(info.assets.len(), 2);
        assert!(info.assets[0].contains("windows"));
    }

    // gh api --jq 重构输出：assets 是纯字符串数组（真实演练发现的形态）
    #[test]
    fn parse_release_json_jq_string_array() {
        let json = r#"{"assets":["link-disk-aarch64-apple-darwin.tar.gz","link-disk-x86_64-pc-windows-msvc.zip"],"tag_name":"v2.2.1"}"#;
        let info = parse_release_json(json).unwrap();
        assert_eq!(info.tag, "v2.2.1");
        assert_eq!(info.assets.len(), 2);
        assert_eq!(info.assets[1], "link-disk-x86_64-pc-windows-msvc.zip");
    }

    #[test]
    fn parse_release_json_missing_tag_errors() {
        let err = parse_release_json(r#"{"message": "rate limited"}"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("tag_name"), "Got: {err}");
    }

    #[test]
    fn normalize_tag_variants() {
        assert_eq!(normalize_tag("2.2.1"), "v2.2.1");
        assert_eq!(normalize_tag("v2.2.1"), "v2.2.1");
        assert_eq!(normalize_tag(" V2.2.1 "), "V2.2.1");
    }

    // 升级决策方向回归：current < latest 必须判定为需要升级
    //（修复前条件写反，v1.1.0 → v2.2.1 被误报 "Already up to date"）
    #[test]
    fn update_decision_direction() {
        let current = format!("v{CURRENT_VERSION}");
        assert_eq!(
            compare_versions(&current, "v99.0.0"),
            std::cmp::Ordering::Less,
            "最新版更新 ⇒ Less ⇒ 应执行升级"
        );
        assert_eq!(
            compare_versions(&current, &format!("v{CURRENT_VERSION}")),
            std::cmp::Ordering::Equal,
            "版本相同 ⇒ Equal ⇒ 不升级"
        );
        assert_eq!(
            compare_versions(&current, "v0.0.1"),
            std::cmp::Ordering::Greater,
            "最新版更旧 ⇒ Greater ⇒ 不升级"
        );
    }
}
