//! Process Monitor
//!
//! 进程监控器模块
//! 负责监控进程行为和特权提升

extern crate alloc;

// 重新导出类型（临时，后续会移动到这里）
#[allow(unused_imports)]
pub use super::host_ids::{
    ProcessMonitor, ProcessInfo, ProcessStatus, ProcessTree,
    ProcessBehaviorDetector, ProcessBehaviorModel, BehaviorPattern,
    BehaviorPatternType, BehaviorCondition, ConditionOperator,
    ProcessAnomalyThresholds, PrivilegeMonitor, PrivilegeEscalation,
    EscalationMethod, PrivilegeModel, UserPrivileges, GroupPrivileges,
    PrivilegeRule, PrivilegeCondition, PrivilegeUidCondition,
    PrivilegeGidCondition, PrivilegeAction,
};

