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
