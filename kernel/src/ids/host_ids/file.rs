//! File System Monitor
//!
//! 文件系统监控器模块
//! 负责监控文件系统操作和变化

extern crate alloc;

// 重新导出类型（临时，后续会移动到这里）
#[allow(unused_imports)]
pub use super::host_ids::{
    ChangeDetectionMode, ChangeDetector, FileChange, FileChangeType, FileEvent, FileEventDetails,
    FileEventType, FileMonitor, SensitivityLevel,
};
use crate::security::audit::AuditEvent;
