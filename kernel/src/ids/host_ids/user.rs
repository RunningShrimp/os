//! User Activity Monitor
//!
//! 用户活动监控器模块
//! 负责监控用户会话和登录活动

extern crate alloc;

// 重新导出类型（临时，后续会移动到这里）
#[allow(unused_imports)]
pub use super::host_ids::{
    LoginEvent, LoginType, SessionState, SessionType, UserBehaviorAnalyzer, UserMonitor,
    UserSession,
};
use crate::security::audit::AuditEvent;
