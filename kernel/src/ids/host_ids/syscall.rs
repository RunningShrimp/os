//! System Call Monitor
//!
//! 系统调用监控器模块
//! 负责监控和分析系统调用行为

extern crate alloc;

use crate::security::audit::AuditEvent;
use super::super::{IntrusionDetection, ThreatLevel};

// 重新导出类型（临时，后续会移动到这里）
pub use super::host_ids::{
    SyscallMonitor, SyscallStats, SyscallAnomalyDetector, SyscallAnomalyModel,
    ArgPattern, ArgPatternType, SyscallThresholds, CallTracer, CallFrame,
    SyscallArg, SyscallArgType, CallChain, CallChainType,
};

