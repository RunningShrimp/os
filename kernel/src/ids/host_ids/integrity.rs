//! Integrity Checker
//!
//! 完整性检查器模块
//! 负责文件完整性检查和基线管理

extern crate alloc;

// 重新导出类型（临时，后续会移动到这里）
#[allow(unused_imports)]
pub use super::host_ids::{
    IntegrityChecker, FileHash, IntegrityBaseline,
    CheckScheduler, ScheduleConfig, CheckWindow,
};

