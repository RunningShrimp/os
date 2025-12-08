//! Integrity Checker
//!
//! 完整性检查器模块
//! 负责文件完整性检查和基线管理

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use super::super::{IntrusionDetection, ThreatLevel};

// 重新导出类型（临时，后续会移动到这里）
pub use super::host_ids::{
    IntegrityChecker, FileHash, IntegrityBaseline,
    CheckScheduler, ScheduleConfig, CheckWindow,
};

