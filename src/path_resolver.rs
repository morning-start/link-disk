use std::path::PathBuf;

pub struct PathResolver;

impl PathResolver {
    pub fn expand(path: &str) -> String {
        Self::replace_placeholders(path)
    }

    pub fn resolve_if_exists(path: &str) -> Option<PathBuf> {
        let expanded = Self::replace_placeholders(path);
        let path = PathBuf::from(expanded);
        if path.exists() { Some(path) } else { None }
    }

    fn replace_placeholders(input: &str) -> String {
        let mut result = input.to_string();

        if let Some(home) = dirs::home_dir() {
            result = result.replace("<home>", &home.to_string_lossy());
        }

        if let Some(appdata) = dirs::data_dir() {
            result = result.replace("<appdata>", &appdata.to_string_lossy());
        }

        if let Some(local) = dirs::data_local_dir() {
            result = result.replace("<localappdata>", &local.to_string_lossy());
        }

        if let Some(documents) = dirs::document_dir() {
            result = result.replace("<documents>", &documents.to_string_lossy());
        }

        if let Some(desktop) = dirs::desktop_dir() {
            result = result.replace("<desktop>", &desktop.to_string_lossy());
        }

        if let Some(download) = dirs::download_dir() {
            result = result.replace("<downloads>", &download.to_string_lossy());
        }

        if let Some(temp) = dirs::cache_dir() {
            result = result.replace("<temp>", &temp.to_string_lossy());
        }

        if let Some(pf) = std::env::var_os("ProgramFiles") {
            result = result.replace("<programfiles>", &pf.to_string_lossy());
        }

        if let Some(pf86) = std::env::var_os("ProgramFiles(x86)") {
            result = result.replace("<programfilesx86>", &pf86.to_string_lossy());
        }

        result.replace("/", "\\")
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
}
