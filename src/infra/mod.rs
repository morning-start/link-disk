//! 基础设施层
//!
//! 提供底层服务支持，包括配置解析、文件系统操作、路径解析和工作区管理。

mod config;
mod fs_utils;
mod path_resolver;
mod workspace;

pub use config::{Config, AppConfig, Source};
pub use config::Workspace as ConfigWorkspace;
pub use fs_utils::{FileSystem, FsUtils, FsWriter, FsLinker};
pub use path_resolver::PathResolver;
pub use workspace::Workspace;
