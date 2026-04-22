use anyhow::{Context, Result};
use std::path::Path;

pub struct FsUtils;

impl FsUtils {
    pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
        if !dst.exists() {
            std::fs::create_dir_all(dst)
                .with_context(|| format!("Failed to create directory: {:?}", dst))?;
        }

        for entry in std::fs::read_dir(src)
            .with_context(|| format!("Failed to read directory: {:?}", src))?
        {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                Self::copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)
                    .with_context(|| format!("Failed to copy {:?} to {:?}", src_path, dst_path))?;
            }
        }

        Ok(())
    }

    pub fn move_dir_cross_filesystem(src: &Path, dst: &Path) -> Result<()> {
        if src.is_file() {
            std::fs::copy(src, dst)
                .with_context(|| format!("Failed to copy file from {:?} to {:?}", src, dst))?;
            std::fs::remove_file(src)
                .with_context(|| format!("Failed to remove source file: {:?}", src))?;
        } else {
            Self::copy_dir_recursive(src, dst)?;
            std::fs::remove_dir_all(src)
                .with_context(|| format!("Failed to remove source directory: {:?}", src))?;
        }
        Ok(())
    }

    pub fn normalize_path(path: &Path) -> String {
        path.to_string_lossy().replace("\\", "/").to_lowercase()
    }

    pub fn ensure_parent_exists(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() && !parent.exists() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory: {:?}", parent))?;
        }
        Ok(())
    }

    pub fn remove_if_exists(path: &Path, verbose: bool) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        if path.is_dir() {
            if verbose {
                println!("Removing directory: {:?}", path);
            }
            std::fs::remove_dir_all(path)
                .with_context(|| format!("Failed to remove directory: {:?}", path))?;
        } else {
            if verbose {
                println!("Removing file: {:?}", path);
            }
            std::fs::remove_file(path)
                .with_context(|| format!("Failed to remove file: {:?}", path))?;
        }
        Ok(())
    }

    pub fn rename(src: &Path, dst: &Path) -> Result<()> {
        std::fs::rename(src, dst)
            .with_context(|| format!("Failed to rename {:?} to {:?}", src, dst))?;
        Ok(())
    }

    pub fn create_symlink(target: &Path, link: &Path) -> Result<()> {
        if link.is_symlink() {
            std::fs::remove_file(link)
                .with_context(|| format!("Failed to remove existing symlink: {:?}", link))?;
        }

        if link.exists() {
            Self::remove_if_exists(link, false)?;
        }

        if target.is_dir() {
            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(target, link).with_context(|| {
                format!("Failed to create directory symlink at {:?} pointing to {:?}", link, target)
            })?;

            #[cfg(not(windows))]
            std::os::unix::fs::symlink(target, link).with_context(|| {
                format!("Failed to create directory symlink at {:?} pointing to {:?}", link, target)
            })?;
        } else {
            #[cfg(windows)]
            std::os::windows::fs::symlink_file(target, link).with_context(|| {
                format!("Failed to create file symlink at {:?} pointing to {:?}", link, target)
            })?;

            #[cfg(not(windows))]
            std::os::unix::fs::symlink(target, link).with_context(|| {
                format!("Failed to create file symlink at {:?} pointing to {:?}", link, target)
            })?;
        }
        Ok(())
    }

    pub fn read_link(path: &Path) -> Option<std::path::PathBuf> {
        std::fs::read_link(path).ok()
    }

    pub fn hard_link(target: &Path, link: &Path) -> Result<()> {
        std::fs::hard_link(target, link)
            .with_context(|| format!("Failed to create hardlink at {:?} pointing to {:?}", link, target))?;
        Ok(())
    }
}
