//! Network Monitor
//!
//! 网络连接监控器模块
//! 负责监控网络连接和异常行为

extern crate alloc;

// 重新导出类型（临时，后续会移动到这里）
#[allow(unused_imports)]
pub use super::host_ids::{
    ConnectionState, ConnectionStats, NetworkAnomalyDetector, NetworkAnomalyModel,
    NetworkAnomalyThresholds, NetworkConnection, NetworkModelType, NetworkMonitor,
};
use crate::security::audit::AuditEvent;
