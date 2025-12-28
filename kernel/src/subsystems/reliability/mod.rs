//! 可靠性模块
//! 
//! 本模块提供系统的可靠性功能，包括：
//! - 故障检测和恢复
//! - 系统状态检查点
//! - 错误日志和诊断

pub mod fault_manager;
pub mod checkpoint_manager;
pub mod error_log;

pub use fault_manager::*;
pub use checkpoint_manager::*;
pub use error_log::*;