//! 访问控制和权限管理模块
//!
//! 本模块提供系统的访问控制和权限管理功能，包括：
//! - 用户身份验证
//! - 权限检查
//! - 资源访问控制
//! - 能力管理
//! - 访问控制列表(ACL)

pub mod types;
pub mod config;
pub mod manager;

// Re-export all public types
pub use types::*;
pub use config::*;
pub use manager::AccessControlManager;
