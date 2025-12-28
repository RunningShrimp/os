//! 扩展的文件描述符缓存系统
//!
//! 提供高效的文件描述符缓存机制，支持：
//! - 扩展缓存大小从8到16个文件描述符
//! - O(1)时间复杂度的缓存查找
//! - 智能缓存失效和更新机制
//! - 缓存统计和性能监控

extern crate alloc;
use core::sync::atomic::{AtomicU64, Ordering};

/// 缓存的文件描述符信息
#[derive(Debug, Clone, Copy)]
pub struct CachedFdInfo {
    /// 文件在全局文件表中的索引
    pub file_idx: Option<usize>,
    /// 文件类型（缓存以避免文件表查找）
    pub file_type: crate::fs::file::FileType,
    /// 缓存有效性标志
    pub valid: bool,
    /// 最后访问时间戳（用于LRU淘汰）
    pub last_access: u64,
    /// 访问频率（用于智能缓存）
    pub access_count: u32,
}

impl CachedFdInfo {
    #[inline]
    pub const fn new() -> Self {
        Self {
            file_idx: None,
            file_type: crate::fs::file::FileType::None,
            valid: false,
            last_access: 0,
            access_count: 0,
        }
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.valid && self.file_idx.is_some()
    }

    #[inline]
    pub fn invalidate(&mut self) {
        self.valid = false;
        self.file_idx = None;
        self.file_type = crate::fs::file::FileType::None;
        self.last_access = 0;
        self.access_count = 0;
    }

    #[inline]
    pub fn update(&mut self, file_idx: usize, file_type: crate::fs::file::FileType, timestamp: u64) {
        self.file_idx = Some(file_idx);
        self.file_type = file_type;
        self.valid = true;
        self.last_access = timestamp;
        self.access_count += 1;
    }
}

impl Default for CachedFdInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// 扩展的文件描述符缓存
/// 支持最多16个文件描述符的高效缓存
pub struct ExtendedFdCache {
    /// 缓存条目（扩展到16个FD）
    cache: [CachedFdInfo; 16],
    /// 缓存命中统计
    hits: AtomicU64,
    /// 缓存未命中统计
    misses: AtomicU64,
    /// 缓存更新统计
    updates: AtomicU64,
    /// 当前时间戳计数器
    timestamp_counter: AtomicU64,
}

impl ExtendedFdCache {
    /// 创建新的扩展文件描述符缓存
    #[inline]
    pub const fn new() -> Self {
        Self {
            cache: [CachedFdInfo::new(); 16],
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            updates: AtomicU64::new(0),
            timestamp_counter: AtomicU64::new(0),
        }
    }

    /// 获取缓存的文件描述符信息（O(1)查找）
    /// 
    /// # 参数
    /// * `fd` - 文件描述符编号（0-15）
    /// 
    /// # 返回值
    /// * `Some(file_idx)` 如果文件描述符在缓存中且有效
    /// * `None` 如果文件描述符不在缓存中或无效
    #[inline(always)]
    pub fn get(&self, fd: i32) -> Option<usize> {
        if fd >= 0 && fd < 16 {
            let cached = &self.cache[fd as usize];
            if cached.is_valid() {
                // 原子性地增加命中计数
                self.hits.fetch_add(1, Ordering::Relaxed);
                return cached.file_idx;
            }
        }
        
        // 原子性地增加未命中计数
        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// 更新文件描述符缓存
    /// 
    /// # 参数
    /// * `fd` - 文件描述符编号（0-15）
    /// * `file_idx` - 文件在全局文件表中的索引
    /// * `file_type` - 文件类型
    #[inline(always)]
    pub fn update(&mut self, fd: i32, file_idx: usize, file_type: crate::fs::file::FileType) {
        if fd >= 0 && fd < 16 {
            let timestamp = self.timestamp_counter.fetch_add(1, Ordering::Relaxed) + 1;
            self.cache[fd as usize].update(file_idx, file_type, timestamp);
            self.updates.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// 使文件描述符缓存失效
    /// 
    /// # 参数
    /// * `fd` - 文件描述符编号（0-15）
    #[inline(always)]
    pub fn invalidate(&mut self, fd: i32) {
        if fd >= 0 && fd < 16 {
            self.cache[fd as usize].invalidate();
        }
    }

    /// 使所有文件描述符缓存失效
    #[inline]
    pub fn invalidate_all(&mut self) {
        for cached in &mut self.cache {
            cached.invalidate();
        }
    }

    /// 智能缓存预热
    /// 根据访问模式预加载常用的文件描述符
    pub fn warmup(&mut self, common_fds: &[(i32, usize, crate::fs::file::FileType)]) {
        let timestamp = self.timestamp_counter.load(Ordering::Relaxed);
        
        for &(fd, file_idx, file_type) in common_fds {
            if fd >= 0 && fd < 16 {
                self.cache[fd as usize].update(file_idx, file_type, timestamp);
            }
        }
    }

    /// 获取缓存统计信息
    pub fn get_stats(&self) -> FdCacheStats {
        FdCacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            updates: self.updates.load(Ordering::Relaxed),
            hit_rate: self.calculate_hit_rate(),
        }
    }

    /// 重置缓存统计
    pub fn reset_stats(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.updates.store(0, Ordering::Relaxed);
    }

    /// 计算缓存命中率
    #[inline]
    fn calculate_hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// 获取最不常用的文件描述符（用于缓存替换）
    pub fn get_lru_fd(&self) -> Option<i32> {
        let mut lru_fd = None;
        let mut min_timestamp = u64::MAX;
        
        for (i, cached) in self.cache.iter().enumerate() {
            if cached.is_valid() {
                if cached.last_access < min_timestamp {
                    min_timestamp = cached.last_access;
                    lru_fd = Some(i as i32);
                }
            }
        }
        
        lru_fd
    }

    /// 获取访问频率最低的有效文件描述符
    pub fn get_lfu_fd(&self) -> Option<i32> {
        let mut lfu_fd = None;
        let mut min_count = u32::MAX;
        
        for (i, cached) in self.cache.iter().enumerate() {
            if cached.is_valid() {
                if cached.access_count < min_count {
                    min_count = cached.access_count;
                    lfu_fd = Some(i as i32);
                }
            }
        }
        
        lfu_fd
    }
}

impl Default for ExtendedFdCache {
    fn default() -> Self {
        Self::new()
    }
}

/// 文件描述符缓存统计信息
#[derive(Debug, Clone)]
pub struct FdCacheStats {
    /// 缓存命中次数
    pub hits: u64,
    /// 缓存未命中次数
    pub misses: u64,
    /// 缓存更新次数
    pub updates: u64,
    /// 缓存命中率（0.0-1.0）
    pub hit_rate: f64,
}

impl FdCacheStats {
    /// 获取总访问次数
    #[inline]
    pub fn total_accesses(&self) -> u64 {
        self.hits + self.misses
    }

    /// 获取缓存效率（每100次访问的更新次数）
    #[inline]
    pub fn efficiency(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            (self.updates as f64 / (self.hits + self.misses) as f64) * 100.0
        }
    }
}

/// 文件描述符缓存配置
#[derive(Debug, Clone)]
pub struct FdCacheConfig {
    /// 是否启用智能预热
    pub enable_warmup: bool,
    /// 是否启用统计收集
    pub enable_stats: bool,
    /// 缓存替换策略
    pub replacement_policy: CacheReplacementPolicy,
}

/// 缓存替换策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheReplacementPolicy {
    /// 最近最少使用（LRU）
    Lru,
    /// 最少使用频率（LFU）
    Lfu,
    /// 无替换（手动管理）
    None,
}

impl Default for FdCacheConfig {
    fn default() -> Self {
        Self {
            enable_warmup: true,
            enable_stats: true,
            replacement_policy: CacheReplacementPolicy::Lru,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fd_cache_basic_operations() {
        let mut cache = ExtendedFdCache::new();
        
        // 测试初始状态
        assert_eq!(cache.get(0), None);
        assert_eq!(cache.get(15), None);
        
        // 测试更新和获取
        cache.update(0, 100, crate::fs::file::FileType::Vfs);
        assert_eq!(cache.get(0), Some(100));
        
        // 测试失效
        cache.invalidate(0);
        assert_eq!(cache.get(0), None);
    }

    #[test]
    fn test_fd_cache_stats() {
        let mut cache = ExtendedFdCache::new();
        
        // 初始统计应该为0
        let stats = cache.get_stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.updates, 0);
        assert_eq!(stats.hit_rate, 0.0);
        
        // 进行一些操作
        cache.update(0, 100, crate::fs::file::FileType::Vfs);
        let _ = cache.get(0); // 命中
        let _ = cache.get(1); // 未命中
        
        // 检查统计
        let stats = cache.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.updates, 1);
        assert!((stats.hit_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_fd_cache_warmup() {
        let mut cache = ExtendedFdCache::new();
        
        let common_fds = [
            (0, 100, crate::fs::file::FileType::Vfs),
            (1, 101, crate::fs::file::FileType::Pipe),
            (2, 102, crate::fs::file::FileType::Socket),
        ];
        
        cache.warmup(&common_fds);
        
        assert_eq!(cache.get(0), Some(100));
        assert_eq!(cache.get(1), Some(101));
        assert_eq!(cache.get(2), Some(102));
    }

    #[test]
    fn test_lru_replacement() {
        let mut cache = ExtendedFdCache::new();
        
        // 添加多个缓存条目
        for i in 0..16 {
            cache.update(i, i * 10, crate::fs::file::FileType::Vfs);
        }
        
        // 获取LRU文件描述符
        let lru_fd = cache.get_lru_fd();
        assert!(lru_fd.is_some());
        
        // 验证LRU逻辑
        // 第一个添加的应该是LRU
        assert_eq!(lru_fd, Some(0));
    }
}
