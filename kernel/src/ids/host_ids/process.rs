//! Process Monitor
//!
//! 进程监控器模块
//! 负责监控进程行为和特权提升

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use crate::security::audit::AuditEvent;
use super::super::{IntrusionDetection, ThreatLevel, Evidence};

// 重新导出类型（临时，后续会移动到这里）
pub use super::host_ids::{
    ProcessMonitor, ProcessInfo, ProcessStatus, ProcessTree,
    ProcessBehaviorDetector, ProcessBehaviorModel, BehaviorPattern,
    BehaviorPatternType, BehaviorCondition, ConditionOperator,
    ProcessAnomalyThresholds, PrivilegeMonitor, PrivilegeEscalation,
    EscalationMethod, PrivilegeModel, UserPrivileges, GroupPrivileges,
    PrivilegeRule, PrivilegeCondition, PrivilegeUidCondition,
    PrivilegeGidCondition, PrivilegeAction,
};
pub use super::super::network_ids::ComparisonOperator;

