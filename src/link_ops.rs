use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct LinkOps;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinkType {
    Symlink,
    Hardlink,
}

impl LinkType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "hardlink" => LinkType::Hardlink,
            _ => LinkType::Symlink,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OnExists {
    Skip,
    Merge,
    Replace,
}

impl OnExists {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "merge" => OnExists::Merge,
            "replace" => OnExists::Replace,
            _ => OnExists::Skip,
        }
    }
}

pub struct LinkRequest {
    pub source: PathBuf,
    pub target: PathBuf,
    pub link_type: LinkType,
    pub on_exists: OnExists,
}

impl LinkOps {
    pub fn link(request: &LinkRequest, verbose: bool) -> Result<()> {
        let source = &request.source;
        let target = &request.target;

        if verbose {
            println!("Linking: {:?} -> {:?}", source, target);
        }

        if source.is_symlink() {
            if let Ok(target_path) = std::fs::read_link(source) {
                let normalized_linked = Self::normalize_path(&target_path);
                let normalized_target = Self::normalize_path(target);
                if normalized_linked == normalized_target {
                    if verbose {
                        println!("Already linked: {:?} -> {:?}", source, target_path);
                    }
                    return Ok(());
                }
            }
            anyhow::bail!("Source is already a symlink pointing to different target: {:?}", source);
        }

        if source.exists() {
            if target.exists() {
                match request.on_exists {
                    OnExists::Skip => {
                        if verbose {
                            println!("Target already exists, skipping: {:?}", target);
                        }
                        return Ok(());
                    }
                    OnExists::Replace => {
                        if verbose {
                            println!("Removing existing target: {:?}", target);
                        }
                        if target.is_dir() {
                            std::fs::remove_dir_all(target)
                                .context("Failed to remove existing target directory")?;
                        } else {
                            std::fs::remove_file(target)
                                .context("Failed to remove existing target file")?;
                        }
                    }
                    OnExists::Merge => {
                        if verbose {
                            println!("Merging into existing target: {:?}", target);
                        }
                        return Self::merge_dirs(source, target, verbose);
                    }
                }
            }

            let parent = target.parent().unwrap_or(target);
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .context("Failed to create target parent directory")?;
            }

            Self::move_dir_cross_filesystem(source, target)?;
        } else {
            // source 不存在的情况
            if !target.exists() {
                // target 也不存在，创建 target 目录
                let parent = target.parent().unwrap_or(target);
                if !parent.exists() {
                    std::fs::create_dir_all(parent)
                        .context("Failed to create target parent directory")?;
                }
            }
            // target 存在则直接创建链接
        }

        match request.link_type {
            LinkType::Symlink => {
                #[cfg(windows)]
                std::os::windows::fs::symlink_dir(target, source)
                    .with_context(|| format!("Failed to create symlink at {:?} pointing to {:?}", source, target))?;

                #[cfg(not(windows))]
                std::os::unix::fs::symlink(target, source)
                    .with_context(|| format!("Failed to create symlink at {:?} pointing to {:?}", source, target))?;
            }
            LinkType::Hardlink => {
                std::fs::hard_link(target, source)
                    .with_context(|| format!("Failed to create hardlink at {:?} pointing to {:?}", source, target))?;
            }
        }

        if verbose {
            println!("Successfully linked: {:?} -> {:?}", source, target);
        }

        Ok(())
    }

    pub fn unlink(source: &PathBuf, target: &PathBuf, keep_files: bool, verbose: bool) -> Result<()> {
        if verbose {
            println!("Unlinking: {:?} -> {:?}", source, target);
        }

        if source.is_symlink() {
            let meta = std::fs::symlink_metadata(source)?;
            #[cfg(windows)]
            if meta.is_dir() {
                std::fs::remove_dir(source)?;
            } else {
                std::fs::remove_file(source)?;
            }

            #[cfg(not(windows))]
            std::fs::remove_file(source)?;

            if !keep_files && target.exists() {
                Self::move_back(target, source)?;
            }
        } else if source.exists() {
            anyhow::bail!("Source is not a symlink: {:?}", source);
        } else if target.exists() && !keep_files {
            Self::move_back(target, source)?;
        }

        if verbose {
            println!("Successfully unlinked: {:?} -> {:?}", source, target);
        }

        Ok(())
    }

    fn merge_dirs(source: &PathBuf, target: &PathBuf, verbose: bool) -> Result<()> {
        if !source.is_dir() || !target.is_dir() {
            anyhow::bail!("Merge requires both paths to be directories");
        }

        for entry in std::fs::read_dir(source)
            .with_context(|| format!("Failed to read directory: {:?}", source))?
        {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = target.join(entry.file_name());

            if src_path.is_dir() {
                Self::merge_dirs(&src_path, &dst_path, verbose)?;
            } else {
                if dst_path.exists() {
                    if verbose {
                        println!("Skipping existing file: {:?}", dst_path);
                    }
                } else {
                    std::fs::copy(&src_path, &dst_path)
                        .with_context(|| format!("Failed to copy: {:?} to {:?}", src_path, dst_path))?;
                }
            }
        }

        std::fs::remove_dir_all(source)
            .with_context(|| format!("Failed to remove merged source: {:?}", source))?;

        Ok(())
    }

    fn move_back(source: &PathBuf, target: &PathBuf) -> Result<()> {
        if !source.exists() {
            anyhow::bail!("Target path does not exist: {:?}", source);
        }

        let parent = target.parent().unwrap_or(target);
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .context("Failed to create parent directory")?;
        }

        if source.is_dir() {
            Self::copy_dir_recursive(source, target)?;
            std::fs::remove_dir_all(source)
                .context("Failed to remove source after move back")?;
        } else {
            std::fs::rename(source, target)
                .with_context(|| format!("Failed to move from {:?} to {:?}", source, target))?;
        }

        Ok(())
    }

    fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<()> {
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
                    .with_context(|| format!("Failed to copy: {:?} to {:?}", src_path, dst_path))?;
            }
        }

        Ok(())
    }

    pub fn check_status(source: &PathBuf, target: &PathBuf) -> &'static str {
        if source.is_symlink() {
            if target.exists() {
                "linked"
            } else {
                "broken"
            }
        } else if source.exists() {
            if target.exists() {
                "both_exist"
            } else {
                "source_only"
            }
        } else if target.exists() {
            "target_only"
        } else {
            "none"
        }
    }

    fn move_dir_cross_filesystem(src: &PathBuf, dst: &PathBuf) -> Result<()> {
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

    fn normalize_path(path: &PathBuf) -> String {
        path.to_string_lossy()
            .replace("\\", "/")
            .to_lowercase()
    }
}
