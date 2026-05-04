//! 业务领域层
//!
//! 包含链接操作、状态检查、应用解析和请求构建等核心业务逻辑。

mod link_ops;
mod link_status;
mod app_resolver;
mod request_builder;

pub use link_ops::{LinkOps, LinkRequest, LinkType, OnExists};
pub use link_ops::strategies;
pub use link_status::{LinkStatus, LinkStatusChecker};
pub use app_resolver::resolve_apps;
pub use request_builder::{build_link_request, resolve_paths};
