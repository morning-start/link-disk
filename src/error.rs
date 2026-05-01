//! 错误处理模块
//!
//! 定义 link-disk 专用的错误类型，包括：
//! - IO 错误：文件系统操作失败
//! - 配置错误：配置文件解析或验证失败
//! - 路径错误：路径无效或不存在
//! - 链接错误：链接创建/删除/验证失败

#![allow(dead_code)]

use std::path::Path;

/// link-disk 专用错误类型枚举
pub enum LinkDiskError {
    /// IO 操作错误（如文件读写、目录创建等）
    Io(std::io::Error),
    /// 配置相关错误（如解析失败、字段缺失等）
    Config(String),
    /// 路径相关错误（如路径格式不正确等）
    Path(String),
    /// 链接操作错误（如创建、删除、验证失败等）
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

/// 从 std::io::Error 自动转换为 LinkDiskError
impl From<std::io::Error> for LinkDiskError {
    fn from(err: std::io::Error) -> Self {
        LinkDiskError::Io(err)
    }
}

/// 模块级别的 Result 类型别名
pub type Result<T> = std::result::Result<T, LinkDiskError>;

/// 验证路径是否有效（不包含驱动器前缀）
pub fn validate_path(path: &Path) -> Result<()> {
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::Prefix(_)))
    {
        return Err(LinkDiskError::Path("Path contains invalid prefix".into()));
    }
    Ok(())
}
