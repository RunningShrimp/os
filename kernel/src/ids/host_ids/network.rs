//! Network Monitor
//!
//! 网络连接监控器模块
//! 负责监控网络连接和异常行为

extern crate alloc;

use crate::security::audit::AuditEvent;
use super::super::{IntrusionDetection, ThreatLevel};

// 重新导出类型（临时，后续会移动到这里）
pub use super::host_ids::{
    NetworkMonitor, NetworkConnection, ConnectionState, ConnectionStats,
    NetworkAnomalyDetector, NetworkAnomalyModel, NetworkModelType,
    NetworkAnomalyThresholds,
};

