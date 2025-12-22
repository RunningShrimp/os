//! 优化核心模块
//!
//! **NOTE**: This module provides optimization infrastructure that is being integrated
//! into the unified dispatcher. Some functionality may be deprecated.
//!
//! 提供系统调用优化的核心数据结构和通用功能，用于打破循环依赖

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

/// 通用系统调用统计信息
/// 
/// 统一所有模块的统计信息收集，避免重复实现
#[derive(Debug, Default)]
pub struct UnifiedSyscallStats {
    /// 调用次数
    pub call_count: AtomicU64,
    /// 总执行时间（纳秒）
    pub total_time_ns: AtomicU64,
    /// 错误次数
    pub error_count: AtomicU64,
    /// 成功次数
    pub success_count: AtomicU64,
    /// 缓存命中次数
    pub cache_hits: AtomicU64,
    /// 缓存未命中次数
    pub cache_misses: AtomicU64,
    /// 最后调用时间戳
    pub last_call_timestamp: AtomicU64,
}

impl UnifiedSyscallStats {
    /// 创建新的统计信息
    pub const fn new() -> Self {
        Self {
            call_count: AtomicU64::new(0),
            total_time_ns: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            last_call_timestamp: AtomicU64::new(0),
        }
    }

    /// 记录系统调用执行
    pub fn record_call(&self, duration_ns: u64, success: bool, cache_hit: bool) {
        self.call_count.fetch_add(1, Ordering::Relaxed);
        self.total_time_ns.fetch_add(duration_ns, Ordering::Relaxed);
        self.last_call_timestamp.store(get_current_timestamp_ns(), Ordering::Relaxed);
        
        if success {
            self.success_count.fetch_add(1, Ordering::Relaxed);
        } else {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }
        
        if cache_hit {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// 获取平均执行时间（纳秒）
    pub fn get_average_time_ns(&self) -> f64 {
        let count = self.call_count.load(Ordering::Relaxed);
        let total = self.total_time_ns.load(Ordering::Relaxed);
        
        if count == 0 {
            0.0
        } else {
            total as f64 / count as f64
        }
    }

    /// 获取错误率
    pub fn get_error_rate(&self) -> f64 {
        let total = self.call_count.load(Ordering::Relaxed);
        let errors = self.error_count.load(Ordering::Relaxed);
        
        if total == 0 {
            0.0
        } else {
            errors as f64 / total as f64
        }
    }

    /// 获取缓存命中率
    pub fn get_cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// 重置统计信息
    pub fn reset(&self) {
        self.call_count.store(0, Ordering::Relaxed);
        self.total_time_ns.store(0, Ordering::Relaxed);
        self.error_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.last_call_timestamp.store(0, Ordering::Relaxed);
    }

    /// 获取统计快照
    pub fn get_snapshot(&self) -> SyscallStatsSnapshot {
        SyscallStatsSnapshot {
            call_count: self.call_count.load(Ordering::Relaxed),
            total_time_ns: self.total_time_ns.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
            success_count: self.success_count.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            last_call_timestamp: self.last_call_timestamp.load(Ordering::Relaxed),
            average_time_ns: self.get_average_time_ns(),
            error_rate: self.get_error_rate(),
            cache_hit_rate: self.get_cache_hit_rate(),
        }
    }
}

impl Clone for UnifiedSyscallStats {
    fn clone(&self) -> Self {
        Self {
            call_count: AtomicU64::new(self.call_count.load(Ordering::Relaxed)),
            total_time_ns: AtomicU64::new(self.total_time_ns.load(Ordering::Relaxed)),
            error_count: AtomicU64::new(self.error_count.load(Ordering::Relaxed)),
            success_count: AtomicU64::new(self.success_count.load(Ordering::Relaxed)),
            cache_hits: AtomicU64::new(self.cache_hits.load(Ordering::Relaxed)),
            cache_misses: AtomicU64::new(self.cache_misses.load(Ordering::Relaxed)),
            last_call_timestamp: AtomicU64::new(self.last_call_timestamp.load(Ordering::Relaxed)),
        }
    }
}

/// 统计信息快照
#[derive(Debug, Clone)]
pub struct SyscallStatsSnapshot {
    pub call_count: u64,
    pub total_time_ns: u64,
    pub error_count: u64,
    pub success_count: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub last_call_timestamp: u64,
    pub average_time_ns: f64,
    pub error_rate: f64,
    pub cache_hit_rate: f64,
}

/// 通用缓存条目
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    /// 缓存的数据
    pub data: T,
    /// 创建时间戳
    pub timestamp: u64,
    /// 生存时间（TTL，纳秒）
    pub ttl_ns: u64,
    /// 访问次数
    pub access_count: AtomicU64,
    /// 最后访问时间
    pub last_access: AtomicU64,
}

impl<T: Clone> CacheEntry<T> {
    /// 创建新的缓存条目
    pub fn new(data: T, ttl_ns: u64) -> Self {
        let now = get_current_timestamp_ns();
        Self {
            data,
            timestamp: now,
            ttl_ns,
            access_count: AtomicU64::new(1),
            last_access: AtomicU64::new(now),
        }
    }

    /// 检查条目是否已过期
    pub fn is_expired(&self) -> bool {
        let now = get_current_timestamp_ns();
        now > self.timestamp + self.ttl_ns
    }

    /// 记录访问
    pub fn record_access(&self) {
        self.access_count.fetch_add(1, Ordering::Relaxed);
        self.last_access.store(get_current_timestamp_ns(), Ordering::Relaxed);
    }

    /// 获取访问次数
    pub fn get_access_count(&self) -> u64 {
        self.access_count.load(Ordering::Relaxed)
    }

    /// 获取最后访问时间
    pub fn get_last_access(&self) -> u64 {
        self.last_access.load(Ordering::Relaxed)
    }
}

/// 通用缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// 最大条目数
    pub max_entries: usize,
    /// 默认TTL（纳秒）
    pub default_ttl_ns: u64,
    /// 清理间隔（纳秒）
    pub cleanup_interval_ns: u64,
    /// 淘汰策略
    pub eviction_policy: EvictionPolicy,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1024,
            default_ttl_ns: 1_000_000_000, // 1秒
            cleanup_interval_ns: 100_000_000, // 100ms
            eviction_policy: EvictionPolicy::LRU,
        }
    }
}

/// 缓存淘汰策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionPolicy {
    /// 最近最少使用
    LRU,
    /// 最近最少使用（基于访问次数）
    LFU,
    /// 先进先出
    FIFO,
    /// 基于TTL
    TTL,
}

/// 通用性能优化配置
#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    /// 是否启用缓存
    pub enable_cache: bool,
    /// 是否启用批处理
    pub enable_batch: bool,
    /// 是否启用零拷贝
    pub enable_zero_copy: bool,
    /// 快速路径阈值（纳秒）
    pub fast_path_threshold_ns: u64,
    /// 批处理大小阈值
    pub batch_threshold: usize,
    /// 缓存TTL（纳秒）
    pub cache_ttl_ns: u64,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            enable_batch: true,
            enable_zero_copy: true,
            fast_path_threshold_ns: 1000, // 1微秒
            batch_threshold: 8,
            cache_ttl_ns: 1_000_000_000, // 1秒
        }
    }
}

/// 获取当前时间戳（纳秒）
pub fn get_current_timestamp_ns() -> u64 {
    crate::subsystems::time::timestamp_nanos()
}

/// 通用缓存实现
pub struct UnifiedCache<K, V> 
where 
    K: Clone + core::hash::Hash + Eq + Ord,
    V: Clone,
{
    /// 缓存条目
    entries: Arc<Mutex<BTreeMap<K, CacheEntry<V>>>>,
    /// 访问顺序（用于LRU）
    access_order: Arc<Mutex<Vec<K>>>,
    /// 配置
    config: CacheConfig,
    /// 统计信息
    stats: UnifiedSyscallStats,
}

impl<K, V> UnifiedCache<K, V>
where
    K: Clone + core::hash::Hash + Eq + Ord,
    V: Clone,
{
    /// 创建新的缓存
    pub fn new(config: CacheConfig) -> Self {
        Self {
            entries: Arc::new(Mutex::new(BTreeMap::new())),
            access_order: Arc::new(Mutex::new(Vec::new())),
            config,
            stats: UnifiedSyscallStats::new(),
        }
    }

    /// 获取缓存值
    pub fn get(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.lock();
        
        if let Some(entry) = entries.get_mut(key) {
            if entry.is_expired() {
                // 移除过期条目
                entries.remove(key);
                self.update_access_order(key, true);
                self.stats.record_call(0, false, false);
                return None;
            }
            
            entry.record_access();
            let value = entry.data.clone();
            drop(entries);
            
            self.update_access_order(key, false);
            self.stats.record_call(0, true, true);
            Some(value)
        } else {
            self.stats.record_call(0, false, false);
            None
        }
    }

    /// 设置缓存值
    pub fn set(&self, key: K, value: V, ttl_ns: Option<u64>) {
        let ttl = ttl_ns.unwrap_or(self.config.default_ttl_ns);
        let entry = CacheEntry::new(value, ttl);
        
        let mut entries = self.entries.lock();
        
        // 检查是否需要淘汰条目
        if entries.len() >= self.config.max_entries && !entries.contains_key(&key) {
            self.evict_entries(&mut entries);
        }
        
        entries.insert(key.clone(), entry);
        drop(entries);
        
        self.update_access_order(&key, false);
    }

    /// 移除缓存条目
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.lock();
        if let Some(entry) = entries.remove(key) {
            drop(entries);
            self.update_access_order(key, true);
            Some(entry.data)
        } else {
            None
        }
    }

    /// 清空缓存
    pub fn clear(&self) {
        let mut entries = self.entries.lock();
        entries.clear();
        drop(entries);
        
        let mut access_order = self.access_order.lock();
        access_order.clear();
    }

    /// 获取缓存统计信息
    pub fn get_stats(&self) -> SyscallStatsSnapshot {
        self.stats.get_snapshot()
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        self.stats.reset();
    }

    /// 获取缓存大小
    pub fn len(&self) -> usize {
        self.entries.lock().len()
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.lock().is_empty()
    }

    /// 更新访问顺序
    fn update_access_order(&self, key: &K, remove: bool) {
        let mut access_order = self.access_order.lock();
        
        if remove {
            access_order.retain(|k| k != key);
        } else {
            // 移除旧位置并添加到末尾（LRU）
            access_order.retain(|k| k != key);
            access_order.push(key.clone());
        }
    }

    /// 淘汰条目
    fn evict_entries(&self, entries: &mut BTreeMap<K, CacheEntry<V>>) {
        let mut access_order = self.access_order.lock();
        
        match self.config.eviction_policy {
            EvictionPolicy::LRU => {
                // 移除最旧的条目
                if let Some(oldest_key) = access_order.first() {
                    entries.remove(oldest_key);
                    access_order.remove(0);
                }
            }
            EvictionPolicy::LFU => {
                // 移除访问次数最少的条目
                let mut min_access = u64::MAX;
                let mut min_key = None;
                
                for (key, entry) in entries.iter() {
                    let access_count = entry.get_access_count();
                    if access_count < min_access {
                        min_access = access_count;
                        min_key = Some(key.clone());
                    }
                }
                
                if let Some(key) = min_key {
                    entries.remove(&key);
                    access_order.retain(|k| k != &key);
                }
            }
            EvictionPolicy::FIFO => {
                // 移除最早的条目
                if let Some(oldest_key) = access_order.first() {
                    entries.remove(oldest_key);
                    access_order.remove(0);
                }
            }
            EvictionPolicy::TTL => {
                // 移除所有过期条目
                let mut expired_keys = Vec::new();
                for (key, entry) in entries.iter() {
                    if entry.is_expired() {
                        expired_keys.push(key.clone());
                    }
                }
                
                for key in expired_keys {
                    entries.remove(&key);
                    access_order.retain(|k| k != &key);
                }
            }
        }
    }
}







