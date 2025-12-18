// Copyright (c) 2024 NOS Community
// SPDX-License-Identifier: Apache-2.0

//! Cache management module for performance optimization.
//!
//! This module provides caching functionality for system call results.
//! It supports multiple eviction policies and cache configurations.

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::{
    collections::BTreeMap,
    string::String,
    vec::Vec,
};

pub use super::core::{
    UnifiedSyscallStats, CacheConfig, CacheEntry,
    EvictionPolicy, UnifiedCache,
    get_current_timestamp_ns
};


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
    pub syscall_num: u32,
    /// System call arguments
    pub args: Vec<u64>,
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
    pub max_entries: usize,
    /// Timeout for cache entries in milliseconds
    pub entry_timeout_ms: u64,
    /// Enable cache for pure syscalls only
    pub pure_only: bool,
    /// Eviction policy (LRU by default)
    pub eviction_policy: EvictionPolicy,
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

/// System call cache
pub struct SyscallCache {
    /// Cache configuration
    config: SyscallCacheConfig,
    /// Cache storage (key -> entry)
    entries: BTreeMap<SyscallCacheKey, Vec<u8>>,
    /// LRU tracking (most recently used first)
    lru_list: Vec<SyscallCacheKey>,
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
        let pure_syscalls = BTreeMap::new();
        
        // Register known pure syscalls (will be provided by using crate)
        
        Self {
            config,
            entries: BTreeMap::new(),
            lru_list: Vec::new(),
            pure_syscalls,
        }
    }

    /// Check if a syscall is pure and can be cached
    pub fn is_pure_syscall(&self, syscall_num: u32) -> bool {
        self.pure_syscalls.contains_key(&syscall_num)
    }

    /// Get a cached result for a syscall
    pub fn get(&mut self, key: &SyscallCacheKey) -> Option<Vec<u8>> {
        // Check if the syscall is pure (if pure_only is enabled)
        if self.config.pure_only && !self.is_pure_syscall(key.syscall_num) {
            return None;
        }

        // Find the entry
        let entry = self.entries.get_mut(key)?;
        
        // Update LRU list
        if let Some(pos) = self.lru_list.iter().position(|k| k == key) {
            // Move to front of LRU list
            let _ = self.lru_list.remove(pos);
            self.lru_list.push(key.clone());
        }

        Some(entry.clone())
    }

    /// Add a result to the cache
    pub fn put(&mut self, key: SyscallCacheKey, result: Vec<u8>) {
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

        // Add to cache
        self.entries.insert(key.clone(), result);
        
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
                // Not implemented yet - fallback to LRU
                self.lru_list.remove(0)
            },
            EvictionPolicy::LFU => {
                // Not implemented yet - fallback to LRU
                self.lru_list.remove(0)
            },
            EvictionPolicy::FIFO => {
                // Evict first in list
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
            self.entries.remove(&key);
            self.lru_list.retain(|k| k != &key);
        }
    }

    /// Invalidate all cache entries
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
        self.lru_list.clear();
    }
}