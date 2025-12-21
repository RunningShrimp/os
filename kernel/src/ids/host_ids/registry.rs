//! Registry Monitor
//!
//! 注册表监控器模块
//! 负责监控注册表变化和启动项

extern crate alloc;

use crate::security::audit::AuditEvent;
use super::super::{IntrusionDetection, ThreatLevel};

// 重新导出类型（临时，后续会移动到这里）
pub use super::host_ids::{
    RegistryMonitor, RegistryChange, RegistryChangeType,
    StartupMonitor, StartupItem, StartupItemType, MonitorOptions,
    StartupChange,
};

