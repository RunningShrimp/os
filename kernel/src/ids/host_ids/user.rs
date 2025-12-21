//! User Activity Monitor
//!
//! 用户活动监控器模块
//! 负责监控用户会话和登录活动

extern crate alloc;

use crate::security::audit::AuditEvent;
use super::super::{IntrusionDetection, ThreatLevel};

// 重新导出类型（临时，后续会移动到这里）
pub use super::host_ids::{
    UserMonitor, UserSession, SessionType, SessionState,
    LoginEvent, LoginType, UserBehaviorAnalyzer,
};

