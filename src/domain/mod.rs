//! 业务领域层
//!
//! 包含链接操作、状态检查、文件移动、策略定义等核心业务逻辑。

mod link_ops;
mod link_status;
mod file_mover;
mod strategies;

pub use link_ops::{LinkOps, LinkRequest, LinkType};
pub use link_status::LinkStatus;
pub use strategies::OnExists;

// 以下导出仅供集成测试使用
#[doc(hidden)]
#[allow(unused_imports)]
pub use link_status::LinkStatusChecker;
