//! System Call Monitor
//!
//! 系统调用监控器模块
//! 负责监控和分析系统调用行为

extern crate alloc;

// 重新导出类型（临时，后续会移动到这里）
#[allow(unused_imports)]
pub use super::host_ids::{
    ArgPattern, ArgPatternType, CallChain, CallChainType, CallFrame, CallTracer,
    SyscallAnomalyDetector, SyscallAnomalyModel, SyscallArg, SyscallArgType, SyscallMonitor,
    SyscallStats, SyscallThresholds,
};
use crate::security::audit::AuditEvent;
