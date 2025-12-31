//! Dynamic Debug Control
//!
//! 动态调试控制
//! 提供基于模式的调试启用、运行时调试级别调整、分类调试输出等功能

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic {AtomicBool, AtomicU32, Ordering, Ordering};
use spin::Mutex;

use crate::error::unified::{UnifiedError, UnifiedResult};

/// 动态调试管理器
#[derive(Debug)]
pub struct DynamicDebugManager {
    /// 调试模式
    pub debug_pattern: Mutex<DebugPattern>,
    /// 调试级别
    pub debug_level: AtomicU32,
    /// 启用的类别
    pub enabled_categories: Mutex<BTreeMap<String, DebugCategory>>,
    /// 调试输出缓冲区
    pub output_buffer: Mutex<DebugOutputBuffer>,
    /// 是否启用动态调试
    pub enabled: AtomicBool,
    /// 统计信息
    pub stats: Mutex<DebugStatistics>,
}

/// 调试模式
#[derive(Debug, Clone)]
pub struct DebugPattern {
    /// 包含模式（启用的文件/函数）
    pub includes: Vec<String>,
    /// 排除模式（禁用的文件/函数）
    pub excludes: Vec<String>,
    /// 通配符模式
    pub wildcards: Vec<String>,
    /// 精确匹配
    pub exact_matches: Vec<String>,
}

/// 调试类别
#[derive(Debug, Clone)]
pub struct DebugCategory {
    /// 类别名称
    pub name: String,
    /// 是否启用
    pub enabled: bool,
    /// 调试级别
    pub level: u32,
    /// 描述
    pub description: String,
    /// 输出计数
    pub output_count: u64,
    /// 最后输出时间
    pub last_output_timestamp: u64,
}

/// 调试输出缓冲区
#[derive(Debug)]
pub struct DebugOutputBuffer {
    /// 输出条目
    pub entries: Vec<DebugEntry>,
    /// 缓冲区大小
    pub max_size: usize,
    /// 当前索引（环形缓冲）
    pub current_index: usize,
    /// 是否启用环形缓冲
    pub circular: bool,
}

/// 调试条目
#[derive(Debug, Clone)]
pub struct DebugEntry {
    /// 时间戳
    pub timestamp: u64,
    /// 类别
    pub category: String,
    /// 级别
    pub level: u32,
    /// 文件名
    pub file: String,
    /// 行号
    pub line: u32,
    /// 函数名
    pub function: String,
    /// 消息
    pub message: String,
    /// 模块路径
    pub module_path: String,
}

/// 调试统计信息
#[derive(Debug, Clone, Default)]
pub struct DebugStatistics {
    /// 总输出数
    pub total_outputs: u64,
    /// 按类别统计
    pub outputs_by_category: BTreeMap<String, u64>,
    /// 按级别统计
    pub outputs_by_level: BTreeMap<u32, u64>,
    /// 按文件统计
    pub outputs_by_file: BTreeMap<String, u64>,
    /// 丢弃的输出数
    pub dropped_outputs: u64,
    /// 缓冲区溢出次数
    pub buffer_overflows: u64,
    /// 模式匹配次数
    pub pattern_matches: u64,
    /// 模式不匹配次数
    pub pattern_misses: u64,
}

/// 调试级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DebugLevel {
    /// 关闭
    Off = 0,
    /// 错误
    Error = 1,
    /// 警告
    Warn = 2,
    /// 信息
    Info = 3,
    /// 调试
    Debug = 4,
    /// 跟踪
    Trace = 5,
}

impl DynamicDebugManager {
    /// 创建新的动态调试管理器
    pub fn new() -> Self {
        Self {
            debug_pattern: Mutex::new(DebugPattern::default()),
            debug_level: AtomicU32::new(DebugLevel::Info as u32),
            enabled_categories: Mutex::new(BTreeMap::new()),
            output_buffer: Mutex::new(DebugOutputBuffer::new(4096)),
            enabled: AtomicBool::new(false),
            stats: Mutex::new(DebugStatistics::default()),
        }
    }

    /// 初始化动态调试
    pub fn init(&self) -> UnifiedResult<()> {
        self.enabled.store(true, Ordering::SeqCst);

        // 初始化默认类别
        let mut categories = self.enabled_categories.lock();
        categories.insert("general".to_string(), DebugCategory {
            name: "general".to_string(),
            enabled: true,
            level: DebugLevel::Info as u32,
            description: "General debug output".to_string(),
            output_count: 0,
            last_output_timestamp: 0,
        });

        categories.insert("memory".to_string(), DebugCategory {
            name: "memory".to_string(),
            enabled: true,
            level: DebugLevel::Debug as u32,
            description: "Memory management debug output".to_string(),
            output_count: 0,
            last_output_timestamp: 0,
        });

        categories.insert("syscall".to_string(), DebugCategory {
            name: "syscall".to_string(),
            enabled: true,
            level: DebugLevel::Debug as u32,
            description: "System call debug output".to_string(),
            output_count: 0,
            last_output_timestamp: 0,
        });

        categories.insert("process".to_string(), DebugCategory {
            name: "process".to_string(),
            enabled: true,
            level: DebugLevel::Debug as u32,
            description: "Process management debug output".to_string(),
            output_count: 0,
            last_output_timestamp: 0,
        });

        categories.insert("fs".to_string(), DebugCategory {
            name: "fs".to_string(),
            enabled: true,
            level: DebugLevel::Debug as u32,
            description: "Filesystem debug output".to_string(),
            output_count: 0,
            last_output_timestamp: 0,
        });

        categories.insert("network".to_string(), DebugCategory {
            name: "network".to_string(),
            enabled: true,
            level: DebugLevel::Debug as u32,
            description: "Network debug output".to_string(),
            output_count: 0,
            last_output_timestamp: 0,
        });

        categories.insert("driver".to_string(), DebugCategory {
            name: "driver".to_string(),
            enabled: true,
            level: DebugLevel::Debug as u32,
            description: "Driver debug output".to_string(),
            output_count: 0,
            last_output_timestamp: 0,
        });

        Ok(())
    }

    /// 设置调试级别
    pub fn set_debug_level(&self, level: DebugLevel) {
        self.debug_level.store(level as u32, Ordering::SeqCst);
    }

    /// 获取调试级别
    pub fn get_debug_level(&self) -> DebugLevel {
        match self.debug_level.load(Ordering::SeqCst) {
            0 => DebugLevel::Off,
            1 => DebugLevel::Error,
            2 => DebugLevel::Warn,
            3 => DebugLevel::Info,
            4 => DebugLevel::Debug,
            5 => DebugLevel::Trace,
            _ => DebugLevel::Info,
        }
    }

    /// 添加调试模式
    pub fn add_pattern(&self, pattern: &str) -> UnifiedResult<()> {
        let mut debug_pattern = self.debug_pattern.lock();

        if pattern.starts_with('-') {
            // 排除模式
            debug_pattern.excludes.push(pattern[1..].to_string());
        } else if pattern.contains('*') {
            // 通配符模式
            debug_pattern.wildcards.push(pattern.to_string());
        } else {
            // 精确匹配
            debug_pattern.exact_matches.push(pattern.to_string());
        }

        Ok(())
    }

    /// 移除调试模式
    pub fn remove_pattern(&self, pattern: &str) -> UnifiedResult<()> {
        let mut debug_pattern = self.debug_pattern.lock();

        if pattern.starts_with('-') {
            let exclude = pattern[1..].to_string();
            debug_pattern.excludes.retain(|p| p != &exclude);
        } else if pattern.contains('*') {
            debug_pattern.wildcards.retain(|p| p != pattern);
        } else {
            debug_pattern.exact_matches.retain(|p| p != pattern);
        }

        Ok(())
    }

    /// 启用调试类别
    pub fn enable_category(&self, category: &str, level: DebugLevel) -> UnifiedResult<()> {
        let mut categories = self.enabled_categories.lock();

        if let Some(cat) = categories.get_mut(category) {
            cat.enabled = true;
            cat.level = level as u32;
        } else {
            categories.insert(category.to_string(), DebugCategory {
                name: category.to_string(),
                enabled: true,
                level: level as u32,
                description: format!("{} debug output", category),
                output_count: 0,
                last_output_timestamp: 0,
            });
        }

        Ok(())
    }

    /// 禁用调试类别
    pub fn disable_category(&self, category: &str) -> UnifiedResult<()> {
        let mut categories = self.enabled_categories.lock();

        if let Some(cat) = categories.get_mut(category) {
            cat.enabled = false;
        } else {
            return Err(UnifiedError::NotFound);
        }

        Ok(())
    }

    /// 检查是否应该输出
    pub fn should_output(&self, category: &str, level: u32, file: &str, function: &str) -> bool {
        // 检查是否启用
        if !self.enabled.load(Ordering::SeqCst) {
            return false;
        }

        // 检查级别
        let current_level = self.debug_level.load(Ordering::SeqCst);
        if level > current_level {
            return false;
        }

        // 检查类别
        let categories = self.enabled_categories.lock();
        if let Some(cat) = categories.get(category) {
            if !cat.enabled || level > cat.level {
                return false;
            }
        } else {
            return false;
        }
        drop(categories);

        // 检查模式
        let debug_pattern = self.debug_pattern.lock();
        let mut should_output = false;

        // 检查排除模式
        for exclude in &debug_pattern.excludes {
            if file.contains(exclude) || function.contains(exclude) {
                return false;
            }
        }

        // 检查精确匹配
        for exact in &debug_pattern.exact_matches {
            if file == exact || function == exact {
                should_output = true;
                break;
            }
        }

        // 检查通配符模式
        if !should_output {
            for wildcard in &debug_pattern.wildcards {
                if self.match_wildcard(file, wildcard) || self.match_wildcard(function, wildcard) {
                    should_output = true;
                    break;
                }
            }
        }

        should_output
    }

    /// 输出调试信息
    pub fn debug_output(
        &self,
        category: &str,
        level: DebugLevel,
        file: &str,
        line: u32,
        function: &str,
        module_path: &str,
        message: &str,
    ) {
        if !self.should_output(category, level as u32, file, function) {
            return;
        }

        let entry = DebugEntry {
            timestamp: self.get_timestamp(),
            category: category.to_string(),
            level: level as u32,
            file: file.to_string(),
            line,
            function: function.to_string(),
            message: message.to_string(),
            module_path: module_path.to_string(),
        };

        // 添加到输出缓冲区
        let mut buffer = self.output_buffer.lock();
        if buffer.add_entry(entry.clone()).is_err() {
            let mut stats = self.stats.lock();
            stats.dropped_outputs += 1;
        }

        // 更新统计信息
        let mut stats = self.stats.lock();
        stats.total_outputs += 1;
        *stats.outputs_by_category.entry(category.to_string()).or_insert(0) += 1;
        *stats.outputs_by_level.entry(level as u32).or_insert(0) += 1;
        *stats.outputs_by_file.entry(file.to_string()).or_insert(0) += 1;

        // 更新类别统计
        let mut categories = self.enabled_categories.lock();
        if let Some(cat) = categories.get_mut(category) {
            cat.output_count += 1;
            cat.last_output_timestamp = entry.timestamp;
        }

        // 实际输出到日志
        self.log_entry(&entry);
    }

    /// 获取调试输出
    pub fn get_output(&self, category: Option<&str>, level: Option<DebugLevel>) -> Vec<DebugEntry> {
        let buffer = self.output_buffer.lock();
        let entries = buffer.get_entries();

        if category.is_none() && level.is_none() {
            return entries.clone();
        }

        entries
            .iter()
            .filter(|entry| {
                if let Some(cat) = category {
                    if entry.category != cat {
                        return false;
                    }
                }
                if let Some(lvl) = level {
                    if entry.level != lvl as u32 {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    /// 清空输出缓冲区
    pub fn clear_output(&self) {
        let mut buffer = self.output_buffer.lock();
        buffer.clear();
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> DebugStatistics {
        self.stats.lock().clone()
    }

    /// 重置统计信息
    pub fn reset_statistics(&self) {
        let mut stats = self.stats.lock();
        *stats = DebugStatistics::default();
    }

    /// 列出所有类别
    pub fn list_categories(&self) -> Vec<DebugCategory> {
        self.enabled_categories.lock().values().cloned().collect()
    }

    /// 匹配通配符模式
    fn match_wildcard(&self, text: &str, pattern: &str) -> bool {
        let pattern_chars: Vec<char> = pattern.chars().collect();
        let text_chars: Vec<char> = text.chars().collect();
        let mut p_idx = 0;
        let mut t_idx = 0;
        let mut p_star_idx = -1i32;
        let mut t_star_idx = -1i32;

        while t_idx < text_chars.len() {
            if p_idx < pattern_chars.len()
                && (pattern_chars[p_idx] == text_chars[t_idx] || pattern_chars[p_idx] == '?')
            {
                p_idx += 1;
                t_idx += 1;
            } else if p_idx < pattern_chars.len() && pattern_chars[p_idx] == '*' {
                p_star_idx = p_idx as i32;
                t_star_idx = t_idx as i32;
                p_idx += 1;
            } else if p_star_idx >= 0 {
                p_idx = (p_star_idx + 1) as usize;
                t_idx = (t_star_idx + 1) as usize;
                t_star_idx += 1;
            } else {
                return false;
            }
        }

        while p_idx < pattern_chars.len() && pattern_chars[p_idx] == '*' {
            p_idx += 1;
        }

        p_idx == pattern_chars.len()
    }

    /// 记录日志条目
    fn log_entry(&self, entry: &DebugEntry) {
        let level_str = match entry.level {
            0 => "OFF",
            1 => "ERROR",
            2 => "WARN",
            3 => "INFO",
            4 => "DEBUG",
            5 => "TRACE",
            _ => "UNKNOWN",
        };

        log::log!(
            match entry.level {
                1 => log::Level::Error,
                2 => log::Level::Warn,
                3 => log::Level::Info,
                4 => log::Level::Debug,
                5 => log::Level::Trace,
                _ => log::Level::Info,
            },
            "[{}:{}:{}] {} {}: {}",
            entry.file,
            entry.line,
            entry.function,
            level_str,
            entry.category,
            entry.message
        );
    }

    /// 获取时间戳
    fn get_timestamp(&self) -> u64 {
        // 在实际实现中，从系统时钟获取
        // 这里简化返回 0
        0
    }
}

impl DebugOutputBuffer {
    /// 创建新的调试输出缓冲区
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Vec::with_capacity(max_size),
            max_size,
            current_index: 0,
            circular: true,
        }
    }

    /// 添加条目
    pub fn add_entry(&mut self, entry: DebugEntry) -> UnifiedResult<()> {
        if self.entries.len() < self.max_size {
            self.entries.push(entry);
        } else if self.circular {
            self.entries[self.current_index] = entry;
            self.current_index = (self.current_index + 1) % self.max_size;
        } else {
            return Err(UnifiedError::OutOfSpace);
        }

        Ok(())
    }

    /// 获取所有条目
    pub fn get_entries(&self) -> &[DebugEntry] {
        &self.entries
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_index = 0;
    }

    /// 设置是否为环形缓冲
    pub fn set_circular(&mut self, circular: bool) {
        self.circular = circular;
    }
}

impl Default for DebugPattern {
    fn default() -> Self {
        Self {
            includes: Vec::new(),
            excludes: Vec::new(),
            wildcards: Vec::new(),
            exact_matches: Vec::new(),
        }
    }
}

/// 动态调试宏
#[macro_export]
macro_rules! dynamic_debug {
    ($category:expr, $level:expr, $($arg:tt)*) => {
        if let Some(manager) = $crate::debug::dynamic_debug::DYNAMIC_DEBUG_MANAGER.get() {
            manager.debug_output(
                $category,
                $level,
                file!(),
                line!(),
                function!(),
                module_path!(),
                &format!($($arg)*),
            );
        }
    };
}

/// 全局动态调试管理器（在运行时初始化)
pub static DYNAMIC_DEBUG_MANAGER: spin::Once<DynamicDebugManager> = spin::Once::new();

/// 初始化全局动态调试管理器
pub fn init_dynamic_debug() {
    DYNAMIC_DEBUG_MANAGER.call_once(|| {
        let manager = DynamicDebugManager::new();
        manager.init().expect("Failed to initialize dynamic debug");
        manager
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let manager = DynamicDebugManager::new();
        assert!(!manager.enabled.load(Ordering::SeqCst));
    }

    #[test]
    fn test_debug_levels() {
        let manager = DynamicDebugManager::new();
        manager.set_debug_level(DebugLevel::Debug);
        assert_eq!(manager.get_debug_level(), DebugLevel::Debug);
    }

    #[test]
    fn test_category_enable_disable() {
        let manager = DynamicDebugManager::new();
        manager.init().unwrap();

        manager.enable_category("test", DebugLevel::Info).unwrap();
        let categories = manager.list_categories();
        assert!(categories.iter().any(|c| c.name == "test"));

        manager.disable_category("test").unwrap();
        let categories = manager.list_categories();
        let test_cat = categories.iter().find(|c| c.name == "test").unwrap();
        assert!(!test_cat.enabled);
    }

    #[test]
    fn test_wildcard_match() {
        let manager = DynamicDebugManager::new();
        assert!(manager.match_wildcard("test_file.rs", "test_*"));
        assert!(manager.match_wildcard("kernel/src/test.rs", "kernel/src/*.rs"));
        assert!(!manager.match_wildcard("other.rs", "test_*"));
    }

    #[test]
    fn test_output_buffer() {
        let mut buffer = DebugOutputBuffer::new(10);
        let entry = DebugEntry {
            timestamp: 0,
            category: "test".to_string(),
            level: 3,
            file: "test.rs".to_string(),
            line: 10,
            function: "test_func".to_string(),
            message: "Test message".to_string(),
            module_path: "test".to_string(),
        };

        assert!(buffer.add_entry(entry).is_ok());
        assert_eq!(buffer.get_entries().len(), 1);

        buffer.clear();
        assert_eq!(buffer.get_entries().len(), 0);
    }
}
