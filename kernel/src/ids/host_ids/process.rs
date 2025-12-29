//! Process Monitor
//!
//! 进程监控器模块
//! 负责监控进程行为和特权提升

extern crate alloc;

// 重新导出类型（临时，后续会移动到这里）
#[allow(unused_imports)]
pub use super::host_ids::{
    BehaviorCondition, BehaviorPattern, BehaviorPatternType, ConditionOperator, EscalationMethod,
    GroupPrivileges, PrivilegeAction, PrivilegeCondition, PrivilegeEscalation,
    PrivilegeGidCondition, PrivilegeModel, PrivilegeMonitor, PrivilegeRule, PrivilegeUidCondition,
    ProcessAnomalyThresholds, ProcessBehaviorDetector, ProcessBehaviorModel, ProcessInfo,
    ProcessMonitor, ProcessStatus, ProcessTree, UserPrivileges,
};
