//! 基础设施层
//!
//! 提供底层服务支持，包括配置解析、文件系统操作、路径解析和工作区管理。

mod config;
mod fs_utils;
mod path_resolver;
mod workspace;
mod request_builder;

pub use config::{Config, AppConfig, Source};
pub use fs_utils::{FileSystem, FsUtils, FsWriter};
pub use path_resolver::PathResolver;
pub use workspace::Workspace;

// 应用解析和请求构建函数（供 commands 层使用）
pub use request_builder::{resolve_apps, build_link_request, resolve_paths};

// 以下导出仅供集成测试使用
#[doc(hidden)]
#[allow(unused_imports)]
pub use config::Workspace as ConfigWorkspace;
#[doc(hidden)]
#[allow(unused_imports)]
pub use config::constants as config_constants;
#[doc(hidden)]
#[allow(unused_imports)]
pub use config::strategy_constants;
#[doc(hidden)]
#[allow(unused_imports)]
pub use fs_utils::{FsReader, FsLinker, FsCopier};
