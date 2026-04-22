use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::fs_utils::FsUtils;

pub struct LinkOps;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinkType {
    Symlink,
    Hardlink,
}

impl LinkType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "hardlink" | "Hardlink" | "HARDLINK" => LinkType::Hardlink,
            _ => LinkType::Symlink,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OnExists {
    Skip,
    Merge,
    Overwrite,
    Replace,
}

impl OnExists {
    pub fn from_str(s: &str) -> Self {
        match s {
            "merge" | "Merge" | "MERGE" => OnExists::Merge,
            "overwrite" | "Overwrite" | "OVERWRITE" => OnExists::Overwrite,
            "replace" | "Replace" | "REPLACE" => OnExists::Replace,
            _ => OnExists::Skip,
        }
    }
}

pub struct LinkRequest {
    pub source: PathBuf,
    pub target: PathBuf,
    pub link_type: LinkType,
    pub on_exists: OnExists,
    pub force: bool,
}

impl LinkOps {
    pub fn link(request: &LinkRequest, verbose: bool) -> Result<()> {
        let source = &request.source;
        let target = &request.target;

        if verbose {
            println!("Linking: {:?} -> {:?}", source, target);
            println!("  Source exists: {}", source.exists());
            println!("  Source is_symlink: {}", source.is_symlink());
            println!("  Target exists: {}", target.exists());
            println!("  Target is_symlink: {}", target.is_symlink());
            println!("  Force: {}", request.force);
            println!("  LinkType: {:?}", request.link_type);
        }

        if source.is_symlink() {
            if request.force {
                if verbose {
                    println!("Force: removing existing symlink: {:?}", source);
                }
                FsUtils::remove_if_exists(source, false)?;
            } else {
                if let Some(target_path) = FsUtils::read_link(source) {
                    let normalized_linked = FsUtils::normalize_path(&target_path);
                    let normalized_target = FsUtils::normalize_path(target);
                    if normalized_linked == normalized_target {
                        if verbose {
                            println!("Already linked: {:?} -> {:?}", source, target_path);
                        }
                        return Ok(());
                    }
                }
                anyhow::bail!(
                    "Source is already a symlink pointing to different target: {:?}",
                    source
                );
            }
        }

        if source.exists() {
            if verbose {
                println!("Source exists, moving to target...");
            }
            if target.exists() {
                match request.on_exists {
                    OnExists::Skip => {
                        if verbose {
                            println!("Target already exists, skipping: {:?}", target);
                        }
                        return Ok(());
                    }
                    OnExists::Replace => {
                        FsUtils::remove_if_exists(target, verbose)?;
                    }
                    OnExists::Merge => {
                        return Self::merge_dirs(source, target, verbose);
                    }
                    OnExists::Overwrite => {
                        FsUtils::remove_if_exists(source, verbose)?;
                        return Ok(());
                    }
                }
            }

            FsUtils::ensure_parent_exists(target)?;
            FsUtils::move_dir_cross_filesystem(source, target)?;
        } else {
            if verbose {
                println!("Source does not exist, creating target directory structure...");
            }
            FsUtils::ensure_parent_exists(target)?;
            if !target.exists() {
                std::fs::create_dir_all(target)
                    .with_context(|| format!("Failed to create target directory: {:?}", target))?;
            }
        }

        match request.link_type {
            LinkType::Symlink => {
                if verbose {
                    println!("Creating symlink: {:?} -> {:?}", source, target);
                }
                FsUtils::create_symlink(target, source)?;
            }
            LinkType::Hardlink => {
                if verbose {
                    println!("Creating hardlink: {:?} -> {:?}", source, target);
                }
                FsUtils::hard_link(target, source)?;
            }
        }

        if verbose {
            println!("Successfully linked: {:?} -> {:?}", source, target);
        }

        Ok(())
    }

    pub fn unlink(source: &Path, target: &Path, keep_files: bool, verbose: bool) -> Result<()> {
        if verbose {
            println!("Unlinking: {:?} -> {:?}", source, target);
        }

        if source.is_symlink() {
            FsUtils::remove_if_exists(source, false)?;

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

    fn merge_dirs(source: &Path, target: &Path, verbose: bool) -> Result<()> {
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
            } else if !dst_path.exists() {
                std::fs::copy(&src_path, &dst_path).with_context(|| {
                    format!("Failed to copy: {:?} to {:?}", src_path, dst_path)
                })?;
            } else if verbose {
                println!("Skipping existing file: {:?}", dst_path);
            }
        }

        FsUtils::remove_if_exists(source, verbose)?;

        Ok(())
    }

    fn move_back(source: &Path, target: &Path) -> Result<()> {
        if !source.exists() {
            anyhow::bail!("Target path does not exist: {:?}", source);
        }

        FsUtils::ensure_parent_exists(target)?;

        if source.is_dir() {
            FsUtils::copy_dir_recursive(source, target)?;
            FsUtils::remove_if_exists(source, false)?;
        } else {
            FsUtils::rename(source, target)?;
        }

        Ok(())
    }

    pub fn check_status(source: &Path, target: &Path) -> &'static str {
        if source.is_symlink() {
            if target.exists() { "linked" } else { "broken" }
        } else if source.exists() {
            if target.exists() { "both_exist" } else { "source_only" }
        } else if target.exists() {
            "target_only"
        } else {
            "none"
        }
    }
}
