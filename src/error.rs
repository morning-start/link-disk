#![allow(dead_code)]

use std::path::Path;

pub enum LinkDiskError {
    Io(std::io::Error),
    Config(String),
    Path(String),
    Link(String),
}

impl std::fmt::Display for LinkDiskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkDiskError::Io(e) => write!(f, "IO error: {}", e),
            LinkDiskError::Config(msg) => write!(f, "Config error: {}", msg),
            LinkDiskError::Path(msg) => write!(f, "Path error: {}", msg),
            LinkDiskError::Link(msg) => write!(f, "Link error: {}", msg),
        }
    }
}

impl std::fmt::Debug for LinkDiskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkDiskError::Io(e) => write!(f, "Io({:?})", e),
            LinkDiskError::Config(msg) => write!(f, "Config({:?})", msg),
            LinkDiskError::Path(msg) => write!(f, "Path({:?})", msg),
            LinkDiskError::Link(msg) => write!(f, "Link({:?})", msg),
        }
    }
}

impl From<std::io::Error> for LinkDiskError {
    fn from(err: std::io::Error) -> Self {
        LinkDiskError::Io(err)
    }
}

pub type Result<T> = std::result::Result<T, LinkDiskError>;

pub fn validate_path(path: &Path) -> Result<()> {
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::Prefix(_)))
    {
        return Err(LinkDiskError::Path("Path contains invalid prefix".into()));
    }
    Ok(())
}
