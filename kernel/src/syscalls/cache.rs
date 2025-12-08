//! System Call Result Cache Module
//!
//! This module provides caching functionality for system call results. It supports:
//! 1. Caching of pure function system calls (no side effects)
//! 2. Cache consistency guarantees
//! 3. Configurable cache size and eviction policies
//! 4. Integration with the system call dispatch mechanism

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    vec::Vec,
    string::{String, ToString},
};
use crate::syscalls::common::{SyscallError, SyscallResult};

/// Cache entry for system call results
pub struct SyscallCacheEntry {
    /// System call result
    result: SyscallResult,
    /// Timestamp when the entry was added to the cache
    timestamp: u64,
    /// Reference count (number of active users of this entry)
    ref_count: usize,
    /// Flags indicating entry properties
    flags: CacheEntryFlags,
}

/// Flags for cache entries
bitflags::bitflags! {
    pub struct CacheEntryFlags: u8 {
        /// Entry is for a pure syscall (no side effects)
        const PURE = 0b00000001;
        /// Entry should be kept in cache indefinitely (no eviction)
        const PINNED = 0b00000010;
        /// Entry has been invalidated but not yet removed
        const INVALID = 0b00000100;
    }
}

/// System call cache key (syscall number + arguments)
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SyscallCacheKey {
    /// System call number
    syscall_num: u32,
    /// System call arguments
    args: Vec<u64>,
}

impl SyscallCacheKey {
    /// Create a new cache key
    pub fn new(syscall_num: u32, args: &[u64]) -> Self {
        Self {
            syscall_num,
            args: args.to_vec(),
        }
    }
}

/// System call cache configuration
pub struct SyscallCacheConfig {
    /// Maximum number of entries in the cache
    max_entries: usize,
    /// Timeout for cache entries in milliseconds
    entry_timeout_ms: u64,
    /// Enable cache for pure syscalls only
    pure_only: bool,
    /// Eviction policy (LRU by default)
    eviction_policy: EvictionPolicy,
}

impl Default for SyscallCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1024,
            entry_timeout_ms: 30000, // 30 seconds
            pure_only: true,
            eviction_policy: EvictionPolicy::LRU,
        }
    }
}

/// Cache eviction policies
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvictionPolicy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// Time To Live
    TTL,
}

/// System call result cache
pub struct SyscallCache {
    /// Cache configuration
    config: SyscallCacheConfig,
    /// Cache storage (key -> entry)
    entries: BTreeMap<SyscallCacheKey, SyscallCacheEntry>,
    /// LRU tracking (most recently used first)
    lru_list: Vec<SyscallCacheKey>,
    /// Next entry ID (for internal use)
    next_id: usize,
    /// Pure syscall whitelist (syscall numbers that are pure functions)
    pure_syscalls: BTreeMap<u32, String>,
}

impl SyscallCache {
    /// Create a new system call cache with default configuration
    pub fn new() -> Self {
        Self::with_config(SyscallCacheConfig::default())
    }

    /// Create a new system call cache with custom configuration
    pub fn with_config(config: SyscallCacheConfig) -> Self {
        // Initialize pure syscall whitelist
        let mut pure_syscalls = BTreeMap::new();
        
        // Register known pure syscalls
        // Process management
        pure_syscalls.insert(crate::syscalls::SYS_GETPID, "getpid".to_string());
        
        // Time-related
        // Add more pure syscalls here as they are identified
        
        Self {
            config,
            entries: BTreeMap::new(),
            lru_list: Vec::new(),
            next_id: 0,
            pure_syscalls,
        }
    }

    /// Check if a syscall is pure and can be cached
    pub fn is_pure_syscall(&self, syscall_num: u32) -> bool {
        self.pure_syscalls.contains_key(&syscall_num)
    }

    /// Get a cached result for a syscall
    pub fn get(&mut self, key: &SyscallCacheKey) -> Option<SyscallResult> {
        // Check if the syscall is pure (if pure_only is enabled)
        if self.config.pure_only && !self.is_pure_syscall(key.syscall_num) {
            return None;
        }

        // Find the entry
        let entry = self.entries.get_mut(key)?;
        
        // Check if entry is invalid
        if entry.flags.contains(CacheEntryFlags::INVALID) {
            // Remove invalid entry
            self.entries.remove(key);
            self.lru_list.retain(|k| k != key);
            return None;
        }

        // Check TTL if enabled
        if self.config.eviction_policy == EvictionPolicy::TTL {
            let current_time = crate::time::timestamp_nanos() / 1000000;
            if current_time - entry.timestamp > self.config.entry_timeout_ms {
                self.entries.remove(key);
                self.lru_list.retain(|k| k != key);
                return None;
            }
        }

        // Update LRU list
        if let Some(pos) = self.lru_list.iter().position(|k| k == key) {
            // Move to front of LRU list
            self.lru_list.remove(pos);
            self.lru_list.push(key.clone());
        }

        // Increment reference count
        entry.ref_count += 1;
        
        // Return a copy of the result
        Some(entry.result.clone())
    }

    /// Add a result to the cache
    pub fn put(&mut self, key: SyscallCacheKey, result: SyscallResult) {
        // Check if the syscall is pure (if pure_only is enabled)
        if self.config.pure_only && !self.is_pure_syscall(key.syscall_num) {
            return;
        }

        // Check cache size limit
        if self.entries.len() >= self.config.max_entries {
            // Evict entry based on policy
            self.evict_entry();
        }

        // Remove existing entry if present
        if self.entries.contains_key(&key) {
            self.entries.remove(&key);
            self.lru_list.retain(|k| k != &key);
        }

        // Create new entry
        let entry = SyscallCacheEntry {
            result,
            timestamp: crate::time::timestamp_nanos() / 1000000,
            ref_count: 1,
            flags: CacheEntryFlags::PURE, // Assume pure since we checked earlier
        };

        // Add to cache
        self.entries.insert(key.clone(), entry);
        
        // Add to LRU list
        self.lru_list.push(key);
    }

    /// Evict an entry from the cache based on configured policy
    fn evict_entry(&mut self) {
        if self.lru_list.is_empty() {
            return;
        }

        // Get the entry to evict
        let key_to_evict = match self.config.eviction_policy {
            EvictionPolicy::LRU => {
                // Evict least recently used (first in list)
                self.lru_list.remove(0)
            },
            EvictionPolicy::TTL => {
                // Evict oldest entry
                // This is inefficient, but for simplicity we'll do it this way
                let mut oldest_key = None;
                let mut oldest_time = u64::MAX;
                
                for (key, entry) in &self.entries {
                    if entry.timestamp < oldest_time {
                        oldest_time = entry.timestamp;
                        oldest_key = Some(key.clone());
                    }
                }
                
                oldest_key.unwrap()
            },
            EvictionPolicy::LFU => {
                // Not implemented yet - fallback to LRU
                self.lru_list.remove(0)
            },
        };

        // Remove from cache
        self.entries.remove(&key_to_evict);
        self.lru_list.retain(|k| k != &key_to_evict);
    }

    /// Invalidate all entries for a specific syscall number
    pub fn invalidate_syscall(&mut self, syscall_num: u32) {
        // Collect keys to invalidate
        let keys_to_invalidate: Vec<_> = self.entries
            .keys()
            .filter(|key| key.syscall_num == syscall_num)
            .cloned()
            .collect();

        // Invalidate entries
        for key in keys_to_invalidate {
            if let Some(entry) = self.entries.get_mut(&key) {
                entry.flags.insert(CacheEntryFlags::INVALID);
            }
        }
    }

    /// Invalidate all cache entries
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
        self.lru_list.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            total_entries: self.entries.len(),
            max_entries: self.config.max_entries,
            pure_syscall_count: self.pure_syscalls.len(),
            lru_list_length: self.lru_list.len(),
        }
    }
}

/// Cache statistics
pub struct CacheStats {
    /// Current number of entries in the cache
    pub total_entries: usize,
    /// Maximum allowed entries in the cache
    pub max_entries: usize,
    /// Number of pure syscalls registered
    pub pure_syscall_count: usize,
    /// Length of LRU list
    pub lru_list_length: usize,
}

/// Global system call cache instance
use crate::sync::Mutex;
static GLOBAL_SYSCALL_CACHE: Mutex<SyscallCache> = Mutex::new(SyscallCache::new());

/// Initialize the system call cache
pub fn init_syscall_cache() {
    // Configure cache with appropriate settings
    let config = SyscallCacheConfig {
        max_entries: 2048,
        entry_timeout_ms: 60000, // 1 minute
        pure_only: true,
        eviction_policy: EvictionPolicy::LRU,
    };
    
    *GLOBAL_SYSCALL_CACHE.lock() = SyscallCache::with_config(config);
}

/// Get the global system call cache
pub fn get_global_cache() -> &'static Mutex<SyscallCache> {
    &GLOBAL_SYSCALL_CACHE
}