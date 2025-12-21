//! Host IDs Shared Types
//!
//! 主机入侵检测系统共享类型定义

use super::super::{ThreatLevel, Evidence};

/// 主机入侵检测统计
#[derive(Debug, Clone, Default)]
pub struct HostIdsStats {
    /// 总监控事件数
    pub total_monitored_events: u64,
    /// 系统调用分析数
    pub syscalls_analyzed: u64,
    /// 文件事件数
    pub file_events: u64,
    /// 进程监控数
    pub processes_monitored: u64,
    /// 检测到的异常数
    pub anomalies_detected: u64,
    /// 恶意软件检测数
    pub malware_detected: u64,
    /// 特权提升检测数
    pub privilege_escalations: u64,
    /// 注册表变化数
    pub registry_changes: u64,
    /// 网络连接监控数
    pub network_connections_monitored: u64,
    /// 用户活动监控数
    pub user_activities_monitored: u64,
    /// 完整性检查数
    pub integrity_checks: u64,
    /// 平均处理时间（微秒）
    pub avg_processing_time_us: u64,
    /// 内存使用量
    pub memory_usage_bytes: usize,
}

