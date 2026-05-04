//! 基础设施层
//!
//! 提供底层服务支持，包括配置解析、文件系统操作、路径解析和工作区管理。

mod config;
mod fs_utils;
mod path_resolver;
mod workspace;

#[allow(unused_imports)]
pub use config::{Config, AppConfig, Source, Workspace as ConfigWorkspace};
#[allow(unused_imports)]
pub use config::constants as config_constants;
#[allow(unused_imports)]
pub use config::strategy_constants;
#[allow(unused_imports)]
pub use fs_utils::{FileSystem, FsUtils, FsReader, FsWriter, FsLinker, FsCopier};
#[allow(unused_imports)]
pub use path_resolver::PathResolver;
#[allow(unused_imports)]
pub use workspace::Workspace;
