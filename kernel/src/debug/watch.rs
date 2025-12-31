//! Variable Watchpoints
//!
//! 变量监视点模块
//! 提供变量监视点、硬件断点、数据断点、实时监控等功能

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic {AtomicBool, AtomicU64, Ordering, Ordering};
use spin::Mutex;

use crate::error::unified::{UnifiedError, UnifiedResult};

/// 监视点管理器
#[derive(Debug)]
pub struct WatchpointManager {
    /// 监视点列表
    pub watchpoints: Mutex<BTreeMap<u64, Watchpoint>>,
    /// 下一个监视点 ID
    pub next_id: AtomicU64,
    /// 硬件断点数量（x86_64 限制为 4）
    pub hardware_count: AtomicU64,
    /// 启用的监视点数量
    pub enabled_count: AtomicU64,
    /// 监视点触发历史
    pub trigger_history: Mutex<Vec<WatchpointTrigger>>,
    /// 最大历史记录数
    pub max_history: usize,
}

/// 监视点
#[derive(Debug, Clone)]
pub struct Watchpoint {
    /// 监视点 ID
    pub id: u64,
    /// 监视点类型
    pub watch_type: WatchType,
    /// 监视地址
    pub address: u64,
    /// 监视大小
    pub size: usize,
    /// 访问类型
    pub access_type: AccessType,
    /// 条件
    pub condition: Option<WatchCondition>,
    /// 是否启用
    pub enabled: bool,
    /// 触发次数
    pub trigger_count: u64,
    /// 最大触发次数（None = 无限）
    pub max_triggers: Option<u64>,
    /// 命中时的操作
    pub actions: Vec<WatchAction>,
    /// 名称/描述
    pub name: Option<String>,
    /// 旧值（用于比较）
    pub old_value: Option<Vec<u8>>,
}

/// 监视点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchType {
    /// 软件监视点（通过内存保护实现）
    Software,
    /// 硬件监视点（使用调试寄存器）
    Hardware,
    /// 数据断点
    DataBreakpoint,
}

/// 访问类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    /// 读访问
    Read,
    /// 写访问
    Write,
    /// 读写访问
    ReadWrite,
    /// 执行（代码断点）
    Execute,
}

/// 监视条件
#[derive(Debug, Clone)]
pub enum WatchCondition {
    /// 等于
    Equal(u64),
    /// 不等于
    NotEqual(u64),
    /// 大于
    GreaterThan(u64),
    /// 小于
    LessThan(u64),
    /// 位掩码
    BitMask(u64, u64),
    /// 范围
    Range(u64, u64),
    /// 自定义表达式
    Custom(String),
}

/// 监视操作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchAction {
    /// 打印消息
    PrintMessage,
    /// 暂停执行
    BreakExecution,
    /// 记录日志
    LogTrace,
    /// 调用回调
    Callback,
    /// 忽略
    Ignore,
}

/// 监视点触发记录
#[derive(Debug, Clone)]
pub struct WatchpointTrigger {
    /// 监视点 ID
    pub watchpoint_id: u64,
    /// 触发时间
    pub timestamp: u64,
    /// 触发地址
    pub address: u64,
    /// 访问类型
    pub access_type: AccessType,
    /// 新值
    pub new_value: Option<Vec<u8>>,
    /// 触发线程 ID
    pub thread_id: Option<u64>,
    /// 触发进程 ID
    pub process_id: Option<u64>,
    /// 调用栈
    pub backtrace: Option<Vec<u64>>,
}

/// 硬件断点配置
#[derive(Debug, Clone)]
pub struct HardwareBreakpointConfig {
    /// 调试寄存器索引（0-3）
    pub dr_index: u8,
    /// 断点地址
    pub address: u64,
    /// 地址掩码
    pub address_mask: u8,
    /// 访问类型
    pub access_type: AccessType,
    /// 大小（1, 2, 4, 或 8 字节）
    pub size: u8,
}

/// 监视点统计
#[derive(Debug, Clone, Default)]
pub struct WatchpointStatistics {
    /// 总监视点数
    pub total_watchpoints: u64,
    /// 启用的监视点数
    pub enabled_watchpoints: u64,
    /// 硬件监视点数
    pub hardware_watchpoints: u64,
    /// 软件监视点数
    pub software_watchpoints: u64,
    /// 总触发次数
    pub total_triggers: u64,
    /// 按类型统计的触发次数
    pub triggers_by_type: BTreeMap<WatchType, u64>,
    /// 按访问类型统计的触发次数
    pub triggers_by_access: BTreeMap<AccessType, u64>,
}

impl WatchpointManager {
    /// 创建新的监视点管理器
    pub fn new() -> Self {
        Self {
            watchpoints: Mutex::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
            hardware_count: AtomicU64::new(0),
            enabled_count: AtomicU64::new(0),
            trigger_history: Mutex::new(Vec::new()),
            max_history: 1000,
        }
    }

    /// 设置监视点
    pub fn set_watchpoint(
        &self,
        address: u64,
        size: usize,
        access_type: AccessType,
        watch_type: WatchType,
        name: Option<String>,
    ) -> UnifiedResult<u64> {
        // 检查硬件断点限制
        if watch_type == WatchType::Hardware {
            if self.hardware_count.load(Ordering::SeqCst) >= 4 {
                return Err(UnifiedError::ResourceBusy);
            }
        }

        // 检查地址对齐
        if !self.is_aligned(address, size) {
            return Err(UnifiedError::InvalidAlignment);
        }

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        let watchpoint = Watchpoint {
            id,
            watch_type,
            address,
            size,
            access_type,
            condition: None,
            enabled: true,
            trigger_count: 0,
            max_triggers: None,
            actions: vec![WatchAction::PrintMessage, WatchAction::LogTrace],
            name,
            old_value: None,
        };

        let mut watchpoints = self.watchpoints.lock();
        watchpoints.insert(id, watchpoint);
        self.enabled_count.fetch_add(1, Ordering::SeqCst);

        if watch_type == WatchType::Hardware {
            self.hardware_count.fetch_add(1, Ordering::SeqCst);
        }

        Ok(id)
    }

    /// 移除监视点
    pub fn remove_watchpoint(&self, id: u64) -> UnifiedResult<()> {
        let mut watchpoints = self.watchpoints.lock();
        let watchpoint = watchpoints.get(&id)
            .ok_or(UnifiedError::NotFound)?;

        if watchpoint.enabled {
            self.enabled_count.fetch_sub(1, Ordering::SeqCst);
        }

        if watchpoint.watch_type == WatchType::Hardware {
            self.hardware_count.fetch_sub(1, Ordering::SeqCst);
        }

        watchpoints.remove(&id);
        Ok(())
    }

    /// 启用/禁用监视点
    pub fn toggle_watchpoint(&self, id: u64, enable: bool) -> UnifiedResult<()> {
        let mut watchpoints = self.watchpoints.lock();
        let watchpoint = watchpoints.get_mut(&id)
            .ok_or(UnifiedError::NotFound)?;

        if enable && !watchpoint.enabled {
            watchpoint.enabled = true;
            self.enabled_count.fetch_add(1, Ordering::SeqCst);
        } else if !enable && watchpoint.enabled {
            watchpoint.enabled = false;
            self.enabled_count.fetch_sub(1, Ordering::SeqCst);
        }

        Ok(())
    }

    /// 检查是否触发监视点
    pub fn check_watchpoint(
        &self,
        address: u64,
        access_type: AccessType,
        new_value: Option<Vec<u8>>,
        thread_id: Option<u64>,
        process_id: Option<u64>,
    ) -> Option<WatchpointTrigger> {
        let watchpoints = self.watchpoints.lock();

        for watchpoint in watchpoints.values() {
            if !watchpoint.enabled {
                continue;
            }

            // 检查地址范围
            if address < watchpoint.address || address >= watchpoint.address + watchpoint.size as u64 {
                continue;
            }

            // 检查访问类型
            if !self.matches_access_type(watchpoint.access_type, access_type) {
                continue;
            }

            // 检查条件
            if let Some(ref condition) = watchpoint.condition {
                if !self.evaluate_condition(condition, new_value.as_ref()) {
                    continue;
                }
            }

            // 创建触发记录
            let trigger = WatchpointTrigger {
                watchpoint_id: watchpoint.id,
                timestamp: self.get_timestamp(),
                address,
                access_type,
                new_value,
                thread_id,
                process_id,
                backtrace: None,
            };

            // 执行动作
            self.execute_actions(watchpoint, &trigger);

            return Some(trigger);
        }

        None
    }

    /// 匹配访问类型
    fn matches_access_type(&self, watch_access: AccessType, actual_access: AccessType) -> bool {
        match (watch_access, actual_access) {
            (AccessType::ReadWrite, _) => true,
            (AccessType::Read, AccessType::Read) => true,
            (AccessType::Read, AccessType::ReadWrite) => true,
            (AccessType::Write, AccessType::Write) => true,
            (AccessType::Write, AccessType::ReadWrite) => true,
            (AccessType::Execute, AccessType::Execute) => true,
            _ => false,
        }
    }

    /// 评估条件
    fn evaluate_condition(&self, condition: &WatchCondition, value: Option<&Vec<u8>>) -> bool {
        let value = if let Some(v) = value {
            if v.len() >= 8 {
                u64::from_le_bytes([v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]])
            } else {
                return false;
            }
        } else {
            return false;
        };

        match condition {
            WatchCondition::Equal(ref target) => value == *target,
            WatchCondition::NotEqual(ref target) => value != *target,
            WatchCondition::GreaterThan(ref target) => value > *target,
            WatchCondition::LessThan(ref target) => value < *target,
            WatchCondition::BitMask(mask, expected) => (value & mask) == *expected,
            WatchCondition::Range(min, max) => value >= *min && value <= *max,
            WatchCondition::Custom(_) => false, // 简化实现
        }
    }

    /// 执行动作
    fn execute_actions(&self, watchpoint: &Watchpoint, trigger: &WatchpointTrigger) {
        for action in &watchpoint.actions {
            match action {
                WatchAction::PrintMessage => {
                    if let Some(ref name) = watchpoint.name {
                        log::info!("Watchpoint '{}' triggered at 0x{:x}", name, trigger.address);
                    } else {
                        log::info!("Watchpoint {} triggered at 0x{:x}", watchpoint.id, trigger.address);
                    }
                }
                WatchAction::LogTrace => {
                    // 记录到历史
                    let mut history = self.trigger_history.lock();
                    if history.len() >= self.max_history {
                        history.remove(0);
                    }
                    history.push(trigger.clone());
                }
                WatchAction::BreakExecution => {
                    // 在实际实现中，这里会触发断点
                    log::warn!("Watchpoint triggered: breaking execution");
                }
                WatchAction::Callback => {
                    // 简化实现：实际应调用回调函数
                }
                WatchAction::Ignore => {}
            }
        }
    }

    /// 检查地址对齐
    fn is_aligned(&self, address: u64, size: usize) -> bool {
        match size {
            1 => true,
            2 => address % 2 == 0,
            4 => address % 4 == 0,
            8 => address % 8 == 0,
            _ => false,
        }
    }

    /// 获取时间戳
    fn get_timestamp(&self) -> u64 {
        0
    }

    /// 列出所有监视点
    pub fn list_watchpoints(&self) -> Vec<Watchpoint> {
        self.watchpoints.lock().values().cloned().collect()
    }

    /// 获取监视点
    pub fn get_watchpoint(&self, id: u64) -> Option<Watchpoint> {
        self.watchpoints.lock().get(&id).cloned()
    }

    /// 获取触发历史
    pub fn get_trigger_history(&self) -> Vec<WatchpointTrigger> {
        self.trigger_history.lock().clone()
    }

    /// 清空触发历史
    pub fn clear_history(&self) {
        self.trigger_history.lock().clear();
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> WatchpointStatistics {
        let watchpoints = self.watchpoints.lock();
        let mut stats = WatchpointStatistics::default();

        stats.total_watchpoints = watchpoints.len() as u64;
        stats.hardware_watchpoints = self.hardware_count.load(Ordering::SeqCst);
        stats.software_watchpoints = stats.total_watchpoints - stats.hardware_watchpoints;
        stats.enabled_watchpoints = self.enabled_count.load(Ordering::SeqCst);

        for wp in watchpoints.values() {
            stats.total_triggers += wp.trigger_count;

            *stats.triggers_by_type.entry(wp.watch_type).or_insert(0) += wp.trigger_count;
            *stats.triggers_by_access.entry(wp.access_type).or_insert(0) += wp.trigger_count;
        }

        stats
    }

    /// 设置硬件断点
    pub fn set_hardware_breakpoint(
        &self,
        address: u64,
        access_type: AccessType,
        size: u8,
    ) -> UnifiedResult<u64> {
        // 检查大小
        if ![1, 2, 4, 8].contains(&size) {
            return Err(UnifiedError::InvalidArgument);
        }

        self.set_watchpoint(address, size as usize, access_type, WatchType::Hardware, None)
    }

    /// 监视变量变化
    pub fn watch_variable(&self, address: u64, size: usize, name: String) -> UnifiedResult<u64> {
        self.set_watchpoint(address, size, AccessType::ReadWrite, WatchType::Software, Some(name))
    }

    /// 获取变量值
    pub fn read_variable(&self, address: u64, size: usize) -> UnifiedResult<Vec<u8>> {
        // 简化实现：实际应从内存读取
        Ok(vec![0; size])
    }

    /// 设置变量值
    pub fn write_variable(&self, address: u64, data: &[u8]) -> UnifiedResult<()> {
        // 简化实现：实际应写入内存
        Ok(())
    }
}

impl Default for WatchpointManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let manager = WatchpointManager::new();
        assert_eq!(manager.hardware_count.load(Ordering::SeqCst), 0);
        assert_eq!(manager.enabled_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_set_watchpoint() {
        let manager = WatchpointManager::new();

        let id = manager.set_watchpoint(
            0x1000,
            8,
            AccessType::Write,
            WatchType::Hardware,
            Some("test_watch".to_string()),
        );

        assert!(id.is_ok());
        assert_eq!(manager.hardware_count.load(Ordering::SeqCst), 1);

        let watchpoint = manager.get_watchpoint(id.unwrap());
        assert!(watchpoint.is_some());
        assert_eq!(watchpoint.unwrap().address, 0x1000);
    }

    #[test]
    fn test_remove_watchpoint() {
        let manager = WatchpointManager::new();

        let id = manager
            .set_watchpoint(0x1000, 8, AccessType::Write, WatchType::Hardware, None)
            .unwrap();

        assert!(manager.remove_watchpoint(id).is_ok());
        assert_eq!(manager.hardware_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_toggle_watchpoint() {
        let manager = WatchpointManager::new();

        let id = manager
            .set_watchpoint(0x1000, 8, AccessType::Write, WatchType::Hardware, None)
            .unwrap();

        assert!(manager.toggle_watchpoint(id, false).is_ok());
        assert_eq!(manager.enabled_count.load(Ordering::SeqCst), 0);

        assert!(manager.toggle_watchpoint(id, true).is_ok());
        assert_eq!(manager.enabled_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_check_watchpoint() {
        let manager = WatchpointManager::new();

        manager
            .set_watchpoint(0x1000, 8, AccessType::Write, WatchType::Hardware, None)
            .unwrap();

        // 测试写访问触发
        let trigger = manager.check_watchpoint(
            0x1000,
            AccessType::Write,
            Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            Some(1),
            Some(100),
        );

        assert!(trigger.is_some());
        assert_eq!(trigger.unwrap().address, 0x1000);
    }

    #[test]
    fn test_hardware_limit() {
        let manager = WatchpointManager::new();

        // 设置 4 个硬件断点（x86_64 限制）
        for i in 0..4 {
            let result = manager.set_watchpoint(
                0x1000 + i as u64 * 0x100,
                8,
                AccessType::Write,
                WatchType::Hardware,
                None,
            );
            assert!(result.is_ok());
        }

        // 第 5 个应该失败
        let result = manager.set_watchpoint(
            0x5000,
            8,
            AccessType::Write,
            WatchType::Hardware,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_alignment_check() {
        let manager = WatchpointManager::new();

        // 未对齐的地址应该失败
        let result = manager.set_watchpoint(
            0x1001, // 2 字节未对齐
            2,
            AccessType::Write,
            WatchType::Hardware,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_statistics() {
        let manager = WatchpointManager::new();

        manager
            .set_watchpoint(0x1000, 8, AccessType::Write, WatchType::Hardware, None)
            .unwrap();
        manager
            .set_watchpoint(
                0x2000,
                4,
                AccessType::Read,
                WatchType::Software,
                None,
            )
            .unwrap();

        let stats = manager.get_statistics();
        assert_eq!(stats.total_watchpoints, 2);
        assert_eq!(stats.hardware_watchpoints, 1);
        assert_eq!(stats.software_watchpoints, 1);
        assert_eq!(stats.enabled_watchpoints, 2);
    }
}
