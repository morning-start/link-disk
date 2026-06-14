//! 基础设施层
//!
//! 提供底层服务支持，包括配置解析、文件系统操作、路径解析和工作区管理。

mod config;
mod fs_utils;
mod path_resolver;
mod request_builder;
mod workspace;

pub use config::{AppConfig, Config, Source};
pub use fs_utils::{FileSystem, FsUtils, detect_symlink_cycle};
pub use path_resolver::PathResolver;
pub use request_builder::build_link_request;
pub use workspace::Workspace;

// 应用解析和请求构建函数（供 commands 层使用）
pub use request_builder::{resolve_apps, resolve_paths};

// 以下导出仅供集成测试使用
#[doc(hidden)]
#[allow(unused_imports)]
pub use config::Workspace as ConfigWorkspace;
