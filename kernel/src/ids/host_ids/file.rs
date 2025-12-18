//! File System Monitor
//!
//! 文件系统监控器模块
//! 负责监控文件系统操作和变化

extern crate alloc;

use crate::security::audit::AuditEvent;

// 重新导出类型（临时，后续会移动到这里）
#[allow(unused_imports)]
pub use super::host_ids::{
    FileMonitor, FileEvent, FileEventType, FileEventDetails,
    SensitivityLevel, ChangeDetector, ChangeDetectionMode,
    FileChange, FileChangeType,
};

