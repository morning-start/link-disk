//! 业务领域层
//!
//! 包含链接操作、状态检查、文件移动、策略定义、应用解析和请求构建等核心业务逻辑。

mod link_ops;
mod link_status;
mod file_mover;
mod strategies;
mod app_resolver;
mod request_builder;

#[allow(unused_imports)]
pub use link_ops::{LinkOps, LinkRequest, LinkType};
#[allow(unused_imports)]
pub use link_status::{LinkStatus, LinkStatusChecker};
#[allow(unused_imports)]
pub use strategies::{OnExists, OnExistsAction, OnExistsStrategy};
#[allow(unused_imports)]
pub use strategies::constants as strategy_constants;
#[allow(unused_imports)]
pub use app_resolver::resolve_apps;
#[allow(unused_imports)]
pub use request_builder::{build_link_request, resolve_paths};
