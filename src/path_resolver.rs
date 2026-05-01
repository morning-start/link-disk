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

use std::path::PathBuf;

/// 路径解析工具类
pub struct PathResolver;

impl PathResolver {
    /// 展开路径中的所有占位符，返回展开后的字符串
    pub fn expand(path: &str) -> String {
        Self::replace_placeholders(path)
    }

    /// 展开路径并检查是否存在，存在则返回 Some(PathBuf)
    pub fn resolve_if_exists(path: &str) -> Option<PathBuf> {
        let expanded = Self::replace_placeholders(path);
        let path = PathBuf::from(expanded);
        if path.exists() { Some(path) } else { None }
    }

    /// 替换字符串中的所有占位符为实际路径
    fn replace_placeholders(input: &str) -> String {
        let mut result = input.to_string();

        if let Some(home) = dirs::home_dir() {
            let home_str = home.to_string_lossy();
            if result.contains("<home>") {
                result = result.replace("<home>", &home_str);
            }
        }

        if let Some(appdata) = dirs::data_dir() {
            let appdata_str = appdata.to_string_lossy();
            if result.contains("<appdata>") {
                result = result.replace("<appdata>", &appdata_str);
            }
        }

        if let Some(local) = dirs::data_local_dir() {
            let local_str = local.to_string_lossy();
            if result.contains("<localappdata>") {
                result = result.replace("<localappdata>", &local_str);
            }
        }

        if let Some(documents) = dirs::document_dir() {
            let documents_str = documents.to_string_lossy();
            if result.contains("<documents>") {
                result = result.replace("<documents>", &documents_str);
            }
        }

        if let Some(desktop) = dirs::desktop_dir() {
            let desktop_str = desktop.to_string_lossy();
            if result.contains("<desktop>") {
                result = result.replace("<desktop>", &desktop_str);
            }
        }

        if let Some(download) = dirs::download_dir() {
            let download_str = download.to_string_lossy();
            if result.contains("<downloads>") {
                result = result.replace("<downloads>", &download_str);
            }
        }

        if let Some(temp) = dirs::cache_dir() {
            let temp_str = temp.to_string_lossy();
            if result.contains("<temp>") {
                result = result.replace("<temp>", &temp_str);
            }
        }

        if let Ok(pf) = std::env::var("ProgramFiles")
            && result.contains("<programfiles>")
        {
            result = result.replace("<programfiles>", &pf);
        }

        if let Ok(pf86) = std::env::var("ProgramFiles(x86)")
            && result.contains("<programfilesx86>")
        {
            result = result.replace("<programfilesx86>", &pf86);
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
