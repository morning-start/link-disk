//! 基础设施层
//!
//! 提供底层服务支持，包括配置解析、文件系统操作、路径解析和工作区管理。
//!
//! 分层约束：infra 不依赖 domain（链接请求构建属于领域层，见 `domain/request_builder.rs`）。

mod config;
mod fs_utils;
mod path_resolver;
mod workspace;

pub use config::{AppConfig, Config, Source};
pub use fs_utils::{FileSystem, FsUtils, detect_symlink_cycle};
pub use path_resolver::PathResolver;
pub use workspace::Workspace;

// 仅供集成测试使用；与 `Workspace`（工作区管理器）区分，
// 这里导出的是 config 中的工作区路径配置结构体
#[doc(hidden)]
#[allow(unused_imports)]
pub use config::Workspace as ConfigWorkspace;
