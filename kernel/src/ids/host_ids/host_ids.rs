/// Host Intrusion Detection System (HIDS) - Main Implementation
///
/// 主机入侵检测系统的主要实现

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

use crate::ids::{HostIdsConfig, IntrusionDetection};
use crate::security::audit::{AuditEvent, AuditEventType};

use super::detector::*;
use super::stats::HostIdsStats;

/// 主机入侵检测系统
///
/// 这是主机入侵检测系统的主要结构，协调各个监控器和检测器。
pub struct HostIds {
    /// 系统ID
    pub id: u64,
    /// 配置
    config: HostIdsConfig,
    /// 系统调用监控器
    syscall_monitor: Arc<Mutex<SyscallMonitor>>,
    /// 文件系统监控器
    file_monitor: Arc<Mutex<FileMonitor>>,
    /// 进程监控器
    process_monitor: Arc<Mutex<ProcessMonitor>>,
    /// 注册表监控器
    registry_monitor: Arc<Mutex<RegistryMonitor>>,
    /// 网络连接监控器
    network_monitor: Arc<Mutex<NetworkMonitor>>,
    /// 用户活动监控器
    user_monitor: Arc<Mutex<UserMonitor>>,
    /// 完整性检查器
    integrity_checker: Arc<Mutex<IntegrityChecker>>,
    /// 恶意软件扫描器
    malware_scanner: Arc<Mutex<MalwareScanner>>,
    /// 统计信息
    stats: Arc<Mutex<HostIdsStats>>,
}

impl HostIds {
    /// 创建新的主机入侵检测系统
    pub fn new() -> Self {
        Self {
            id: 1,
            config: HostIdsConfig::default(),
            syscall_monitor: Arc::new(Mutex::new(SyscallMonitor::new())),
            file_monitor: Arc::new(Mutex::new(FileMonitor::new())),
            process_monitor: Arc::new(Mutex::new(ProcessMonitor::new())),
            registry_monitor: Arc::new(Mutex::new(RegistryMonitor::new())),
            network_monitor: Arc::new(Mutex::new(NetworkMonitor::new())),
            user_monitor: Arc::new(Mutex::new(UserMonitor::new())),
            integrity_checker: Arc::new(Mutex::new(IntegrityChecker::new())),
            malware_scanner: Arc::new(Mutex::new(MalwareScanner::new())),
            stats: Arc::new(Mutex::new(HostIdsStats::default())),
        }
    }

    /// 初始化主机入侵检测系统
    pub fn init(&mut self, config: &HostIdsConfig) -> Result<(), &'static str> {
        self.config = config.clone();

        // 初始化各个监控器
        self.syscall_monitor
            .lock()
            .init(&config.monitored_syscalls)?;
        self.file_monitor.lock().init(&config.monitored_paths)?;
        self.process_monitor.lock().init()?;
        self.registry_monitor.lock().init()?;
        self.network_monitor.lock().init(config.monitor_network)?;
        self.user_monitor.lock().init()?;
        self.integrity_checker.lock().init()?;
        self.malware_scanner.lock().init()?;

        crate::println!("[HostIds] Host intrusion detection system initialized");
        Ok(())
    }

    /// 分析系统调用
    pub fn analyze_syscall(
        &mut self,
        event: &AuditEvent,
    ) -> Result<Vec<IntrusionDetection>, &'static str> {
        let start_time = crate::subsystems::time::timestamp_nanos();

        let detections = self.syscall_monitor.lock().analyze_syscall(event)?;

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.increment_syscall_count();
            stats.increment_event_count();

            let elapsed = crate::subsystems::time::timestamp_nanos() - start_time;
            stats.update_processing_time(elapsed / 1000);
        }

        Ok(detections)
    }

    /// 分析文件事件
    pub fn analyze_file_event(
        &mut self,
        event: &AuditEvent,
    ) -> Result<Vec<IntrusionDetection>, &'static str> {
        let start_time = crate::subsystems::time::timestamp_nanos();

        let detections = self.file_monitor.lock().analyze_file_event(event)?;

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.increment_file_event_count();
            stats.increment_event_count();

            let elapsed = crate::subsystems::time::timestamp_nanos() - start_time;
            stats.update_processing_time(elapsed / 1000);
        }

        Ok(detections)
    }

    /// 分析进程事件
    pub fn analyze_process_event(
        &mut self,
        event: &AuditEvent,
    ) -> Result<Vec<IntrusionDetection>, &'static str> {
        let start_time = crate::subsystems::time::timestamp_nanos();

        let detections = self.process_monitor.lock().analyze_process_event(event)?;

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.increment_process_monitor_count();
            stats.increment_event_count();

            let elapsed = crate::subsystems::time::timestamp_nanos() - start_time;
            stats.update_processing_time(elapsed / 1000);
        }

        Ok(detections)
    }

    /// 分析注册表变化
    pub fn analyze_registry_change(
        &mut self,
        event: &AuditEvent,
    ) -> Result<Vec<IntrusionDetection>, &'static str> {
        let start_time = crate::subsystems::time::timestamp_nanos();

        let detections = self
            .registry_monitor
            .lock()
            .analyze_registry_change(event)?;

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.increment_registry_change_count();
            stats.increment_event_count();

            let elapsed = crate::subsystems::time::timestamp_nanos() - start_time;
            stats.update_processing_time(elapsed / 1000);
        }

        Ok(detections)
    }

    /// 分析网络连接
    pub fn analyze_network_connection(
        &mut self,
        event: &AuditEvent,
    ) -> Result<Vec<IntrusionDetection>, &'static str> {
        let start_time = crate::subsystems::time::timestamp_nanos();

        let detections = self
            .network_monitor
            .lock()
            .analyze_network_connection(event)?;

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.increment_network_connection_count();
            stats.increment_event_count();

            let elapsed = crate::subsystems::time::timestamp_nanos() - start_time;
            stats.update_processing_time(elapsed / 1000);
        }

        Ok(detections)
    }

    /// 分析用户活动
    pub fn analyze_user_activity(
        &mut self,
        event: &AuditEvent,
    ) -> Result<Vec<IntrusionDetection>, &'static str> {
        let start_time = crate::subsystems::time::timestamp_nanos();

        let detections = self.user_monitor.lock().analyze_user_activity(event)?;

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.increment_user_activity_count();
            stats.increment_event_count();

            let elapsed = crate::subsystems::time::timestamp_nanos() - start_time;
            stats.update_processing_time(elapsed / 1000);
        }

        Ok(detections)
    }

    /// Analyze a generic audit event and dispatch to the correct analyzer.
    /// This makes HostIds usable from higher-level callers that only have an AuditEvent.
    pub fn analyze_event(
        &mut self,
        event: &AuditEvent,
    ) -> Result<Vec<IntrusionDetection>, &'static str> {
        match event.event_type {
            AuditEventType::Syscall => self.analyze_syscall(event),
            AuditEventType::FileAccess => self.analyze_file_event(event),
            AuditEventType::Process => self.analyze_process_event(event),
            AuditEventType::Network => self.analyze_network_connection(event),
            // Map less-common event types to either specific analyzers or fall back to user
            // activity
            AuditEventType::Authentication
            | AuditEventType::PermissionChange
            | AuditEventType::Configuration => self.analyze_user_activity(event),
            _ => Ok(Vec::new()),
        }
    }

    /// 执行完整性检查
    pub fn perform_integrity_check(&mut self) -> Result<Vec<IntrusionDetection>, &'static str> {
        let start_time = crate::subsystems::time::timestamp_nanos();

        let detections = self.integrity_checker.lock().perform_integrity_check()?;

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.increment_integrity_check_count();
            stats.increment_event_count();

            let elapsed = crate::subsystems::time::timestamp_nanos() - start_time;
            stats.update_processing_time(elapsed / 1000);
        }

        Ok(detections)
    }

    /// 执行恶意软件扫描
    pub fn perform_malware_scan(&mut self) -> Result<Vec<IntrusionDetection>, &'static str> {
        let start_time = crate::subsystems::time::timestamp_nanos();

        let detections = self.malware_scanner.lock().perform_scan()?;

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.increment_malware_detected_count(detections.len());
            stats.increment_event_count();

            let elapsed = crate::subsystems::time::timestamp_nanos() - start_time;
            stats.update_processing_time(elapsed / 1000);
        }

        Ok(detections)
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> HostIdsStats {
        self.stats.lock().clone()
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        self.stats.lock().reset();
    }

    /// 停止主机入侵检测系统
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        crate::println!("[HostIds] Host intrusion detection system shutdown");
        Ok(())
    }
}

impl Default for HostIds {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_ids_creation() {
        let hids = HostIds::new();
        assert_eq!(hids.id, 1);
        assert_eq!(hids.config.enabled, true);
    }

    #[test]
    fn test_host_ids_stats() {
        let hids = HostIds::new();
        let stats = hids.get_stats();
        assert_eq!(stats.total_monitored_events, 0);
        assert_eq!(stats.syscalls_analyzed, 0);
        assert_eq!(stats.file_events, 0);
    }

    #[test]
    fn test_host_ids_reset() {
        let hids = HostIds::new();
        hids.reset_stats();
        let stats = hids.get_stats();
        assert_eq!(stats.total_monitored_events, 0);
    }
}
