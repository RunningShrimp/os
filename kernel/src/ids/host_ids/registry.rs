//! Registry Monitor
//!
//! 注册表监控器模块
//! 负责监控注册表变化和启动项

extern crate alloc;

// 重新导出类型（临时，后续会移动到这里）
#[allow(unused_imports)]
pub use super::host_ids::{
    MonitorOptions, RegistryChange, RegistryChangeType, RegistryMonitor, StartupChange,
    StartupItem, StartupItemType, StartupMonitor,
};
use crate::security::audit::AuditEvent;
