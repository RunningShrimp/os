/// Host Intrusion Detection System (HIDS) - Statistics
///
/// 统计信息相关功能

extern crate alloc;

use alloc::vec::Vec;

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
    /// 注册表变化数
    pub registry_changes: u64,
    /// 网络连接监控数
    pub network_connections_monitored: u64,
    /// 用户活动监控数
    pub user_activities_monitored: u64,
    /// 完整性检查数
    pub integrity_checks: u64,
    /// 恶意软件检测数
    pub malware_detected: u64,
    /// 平均处理时间（微秒）
    pub avg_processing_time_us: u64,
}

impl HostIdsStats {
    /// 创建新的统计信息
    pub fn new() -> Self {
        Self::default()
    }

    /// 更新处理时间
    pub fn update_processing_time(&mut self, elapsed_us: u64) {
        self.avg_processing_time_us = (self.avg_processing_time_us + elapsed_us) / 2;
    }

    /// 增加事件计数
    pub fn increment_event_count(&mut self) {
        self.total_monitored_events += 1;
    }

    /// 增加系统调用计数
    pub fn increment_syscall_count(&mut self) {
        self.syscalls_analyzed += 1;
    }

    /// 增加文件事件计数
    pub fn increment_file_event_count(&mut self) {
        self.file_events += 1;
    }

    /// 增加进程监控计数
    pub fn increment_process_monitor_count(&mut self) {
        self.processes_monitored += 1;
    }

    /// 增加注册表变化计数
    pub fn increment_registry_change_count(&mut self) {
        self.registry_changes += 1;
    }

    /// 增加网络连接监控计数
    pub fn increment_network_connection_count(&mut self) {
        self.network_connections_monitored += 1;
    }

    /// 增加用户活动监控计数
    pub fn increment_user_activity_count(&mut self) {
        self.user_activities_monitored += 1;
    }

    /// 增加完整性检查计数
    pub fn increment_integrity_check_count(&mut self) {
        self.integrity_checks += 1;
    }

    /// 增加恶意软件检测计数
    pub fn increment_malware_detected_count(&mut self, count: usize) {
        self.malware_detected += count as u64;
    }

    /// 重置所有统计信息
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// 获取统计摘要
    pub fn summary(&self) -> String {
        alloc::format!(
            "HostIdsStats {{ \
             total_events: {}, \
             syscalls: {}, \
             file_events: {}, \
             processes: {}, \
             registry: {}, \
             network: {}, \
             user: {}, \
             integrity: {}, \
             malware: {}, \
             avg_time_us: {} \
             }}",
            self.total_monitored_events,
            self.syscalls_analyzed,
            self.file_events,
            self.processes_monitored,
            self.registry_changes,
            self.network_connections_monitored,
            self.user_activities_monitored,
            self.integrity_checks,
            self.malware_detected,
            self.avg_processing_time_us
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_increment() {
        let mut stats = HostIdsStats::new();
        stats.increment_syscall_count();
        stats.increment_file_event_count();
        stats.increment_process_monitor_count();

        assert_eq!(stats.syscalls_analyzed, 1);
        assert_eq!(stats.file_events, 1);
        assert_eq!(stats.processes_monitored, 1);
        assert_eq!(stats.total_monitored_events, 0); // Not incremented by specific counters
    }

    #[test]
    fn test_stats_reset() {
        let mut stats = HostIdsStats::new();
        stats.syscalls_analyzed = 100;
        stats.file_events = 50;
        stats.reset();

        assert_eq!(stats.syscalls_analyzed, 0);
        assert_eq!(stats.file_events, 0);
    }

    #[test]
    fn test_processing_time_update() {
        let mut stats = HostIdsStats::new();
        stats.update_processing_time(100);
        stats.update_processing_time(200);

        assert_eq!(stats.avg_processing_time_us, 150);
    }
}
