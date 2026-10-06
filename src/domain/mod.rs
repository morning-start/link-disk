//! 业务领域层
//!
//! 包含链接操作、状态检查、文件移动、策略定义、请求构建等核心业务逻辑。

mod file_mover;
mod link_ops;
mod link_status;
mod request_builder;
mod strategies;

pub use link_ops::{LinkOps, LinkRequest, LinkType};
pub use link_status::LinkStatus;
pub use request_builder::{build_link_request, resolve_source_target};
pub use strategies::OnExists;

// 仅供集成测试使用；bin 目标内部不引用，避免 dead_code 误报
#[doc(hidden)]
#[allow(unused_imports)]
pub use link_status::LinkStatusChecker;
