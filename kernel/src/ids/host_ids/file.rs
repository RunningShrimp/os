//! File System Monitor
//!
//! 文件系统监控器模块
//! 负责监控文件系统操作和变化

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use crate::security::audit::AuditEvent;
use super::super::{IntrusionDetection, ThreatLevel};

// 重新导出类型（临时，后续会移动到这里）
pub use super::host_ids::{
    FileMonitor, FileEvent, FileEventType, FileEventDetails,
    SensitivityLevel, ChangeDetector, ChangeDetectionMode,
    FileChange, FileChangeType,
};

