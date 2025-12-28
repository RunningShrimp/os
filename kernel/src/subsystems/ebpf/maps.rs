//! eBPF Maps
//!
//! This module implements eBPF maps for kernel-space data sharing:
//! - Hash maps
//! - Array maps
//! - LRU maps
//! - Per-CPU maps
//! - Map sharing between eBPF programs
//!
//! Features:
//! - Multiple map types
//! - Atomic map operations
//! - LRU eviction for hash maps
//! - Per-CPU map support
//! - Map file descriptor management

use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::collections::BTreeSet;
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;

use super::vm::*;
use core::sync::atomic;

// ============================================================================
// eBPF Map Constants
// ============================================================================

/// Maximum number of map entries
pub const MAX_MAP_ENTRIES: usize = 1024;

/// Maximum map key size (in bytes)
pub const MAX_KEY_SIZE: usize = 256;

/// Map file descriptor type
pub const MAP_FD_TYPE: u32 = 0x1B4;

// ============================================================================
// eBPF Map Types
// ============================================================================

/// eBPF map types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EbpfMapType {
    /// Hash map
    Hash,
    
    /// Array map
    Array,
    
    /// Per-CPU hash map
    PerCpuHash,
    
    /// Per-CPU array map
    PerCpuArray,
    
    /// LRU hash map
    LruHash,
    
    /// LRU per-CPU hash map
    LruPerCpuHash,
}

/// Map key type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfKeyType {
    /// Unsigned integer (32-bit)
    U32,
    
    /// Unsigned integer (64-bit)
    U64,
    
    /// String key
    String,
    
    /// Binary key (max 16 bytes)
    Binary16,
    
    /// Binary key (max 32 bytes)
    Binary32,
}

/// Map value size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfValueSize {
    /// 8 bytes
    Size8,
    
    /// 16 bytes
    Size16,
    
    /// 32 bytes
    Size32,
    
    /// 64 bytes
    Size64,
    
    /// 128 bytes
    Size128,
    
    /// Variable size
    Dynamic,
}

// ============================================================================
// eBPF Map Metadata
// ============================================================================

/// eBPF map metadata
#[derive(Debug, Clone)]
pub struct EbpfMapMeta {
    /// Map ID
    pub map_id: u32,
    
    /// Map name
    pub name: String,
    
    /// Map type
    pub map_type: EbpfMapType,
    
    /// Key type
    pub key_type: EbpfKeyType,
    
    /// Value size
    pub value_size: EbpfValueSize,
    
    /// Maximum entries
    pub max_entries: usize,
    
    /// Number of entries (current)
    pub num_entries: AtomicUsize,
    
    /// Map flags
    pub flags: u32,
}

/// Map flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EbpfMapFlags {
    /// Map is read-only
    pub read_only: bool,
    
    /// Map is write-only
    pub write_only: bool,
    
    /// Map can be created by user-space
    pub user_creatable: bool,
    
    /// Map is per-CPU
    pub per_cpu: bool,
    
    /// Map supports preallocation
    pub supports_preallocation: bool,
}

// ============================================================================
// eBPF Maps
// ============================================================================

/// eBPF hash map entry
#[derive(Debug, Clone)]
pub struct EbpfHashEntry {
    /// Map key
    pub key: [u8; MAX_KEY_SIZE],
    
    /// Key length
    pub key_len: usize,
    
    /// Map value
    pub value: Vec<u8>,
    
    /// Value length
    pub value_len: usize,
    
    /// Last access timestamp (for LRU)
    pub last_access: AtomicU64,
}

impl EbpfHashEntry {
    /// Create new hash entry
    pub fn new(key: &[u8], value: &[u8]) -> Self {
        let mut key_array = [0u8; MAX_KEY_SIZE];
        key_array[..key.len()].copy_from_slice(key);
        
        Self {
            key: key_array,
            key_len: key.len(),
            value: value.to_vec(),
            value_len: value.len(),
            last_access: AtomicU64::new(crate::subsystems::time::timestamp_nanos()),
        }
    }
    
    /// Update access time
    pub fn update_access(&self) {
        self.last_access.store(crate::subsystems::time::timestamp_nanos(), Ordering::Relaxed);
    }
}

/// eBPF hash map
pub struct EbpfHashMap {
    /// Map metadata
    pub meta: EbpfMapMeta,
    
    /// Hash buckets (simplified: single BTreeMap)
    pub entries: Mutex<BTreeMap<Vec<u8>, EbpfHashEntry>>,
    
    /// LRU tracking
    pub lru_enabled: bool,
}

impl EbpfHashMap {
    /// Create new hash map
    pub fn new(name: String, key_type: EbpfKeyType, value_size: EbpfValueSize,
               max_entries: usize, flags: EbpfMapFlags) -> Self {
        let map_id = Self::generate_map_id();
        
        let meta = EbpfMapMeta {
            map_id,
            name,
            map_type: EbpfMapType::Hash,
            key_type,
            value_size,
            max_entries,
            num_entries: AtomicUsize::new(0),
            flags: flags.read_only as u32 | flags.write_only as u32 | 
                     flags.user_creatable as u32 | flags.per_cpu as u32 | 
                     flags.supports_preallocation as u32,
        };
        
        Self {
            meta,
            entries: Mutex::new(BTreeMap::new()),
            lru_enabled: flags.supports_preallocation,
        }
    }
    
    /// Insert into hash map
    pub fn insert(&self, key: &[u8], value: &[u8]) -> Result<(), EbpfError> {
        let key_vec = key.to_vec();
        
        let mut entries = self.entries.lock();
        
        // Check if map is full
        if entries.len() >= self.meta.max_entries {
            drop(entries);
            
            if self.lru_enabled {
                self.evict_lru()?;
                let mut entries = self.entries.lock();
                entries.insert(key_vec, EbpfHashEntry::new(key, value));
            } else {
                return Err(EbpfError::MapFull {
                    map_id: self.meta.map_id,
                });
            }
        } else {
            entries.insert(key_vec, EbpfHashEntry::new(key, value));
            self.meta.num_entries.fetch_add(1, Ordering::Relaxed);
        }
        
        Ok(())
    }
    
    /// Lookup in hash map
    pub fn lookup(&self, key: &[u8]) -> Option<EbpfHashEntry> {
        let entries = self.entries.lock();
        let key_vec = key.to_vec();
        
        entries.get(&key_vec).cloned()
    }
    
    /// Delete from hash map
    pub fn delete(&self, key: &[u8]) -> Result<(), EbpfError> {
        let key_vec = key.to_vec();
        let mut entries = self.entries.lock();
        
        if let Some(entry) = entries.remove(&key_vec) {
            self.meta.num_entries.fetch_sub(1, Ordering::Relaxed);
            
            if self.lru_enabled {
                // Clean up LRU tracking
            }
            
            crate::println!("[ebpf] Deleted key from map {}",
                            self.meta.map_id);
            
            Ok(())
        } else {
            Err(EbpfError::KeyNotFound {
                map_id: self.meta.map_id,
            })
        }
    }
    
    /// Evict LRU entry
    fn evict_lru(&self) -> Result<(), EbpfError> {
        let mut entries = self.entries.lock();
        
        if let Some((lru_key, lru_entry)) = entries.iter()
            .min_by_key(|a, b| {
                let time_a = a.1.last_access.load(Ordering::Relaxed);
                let time_b = b.1.last_access.load(Ordering::Relaxed);
                time_a.cmp(&time_b)
            }) {
            
            entries.remove(lru_key);
            self.meta.num_entries.fetch_sub(1, Ordering::Relaxed);
            
            crate::println!("[ebpf] Evicted LRU entry from map {}",
                            self.meta.map_id);
            
            Ok(())
        }
        
        Err(EbpfError::MapEmpty {
            map_id: self.meta.map_id,
        })
    }
    
    /// Get all entries
    pub fn get_all_entries(&self) -> Vec<EbpfHashEntry> {
        let entries = self.entries.lock();
        entries.values().cloned().collect()
    }
    
    /// Generate unique map ID
    fn generate_map_id() -> u32 {
        static MAP_ID_COUNTER: AtomicU32 = AtomicU32::new(1);
        MAP_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
    }
}

/// eBPF array map
pub struct EbpfArrayMap {
    /// Map metadata
    pub meta: EbpfMapMeta,
    
    /// Array entries
    pub entries: Mutex<Vec<Vec<u8>>>,
}

impl EbpfArrayMap {
    /// Create new array map
    pub fn new(name: String, key_type: EbpfKeyType, value_size: EbpfValueSize,
               max_entries: usize, flags: EbpfMapFlags) -> Self {
        let map_id = Self::generate_map_id();
        
        let meta = EbpfMapMeta {
            map_id,
            name,
            map_type: EbpfMapType::Array,
            key_type,
            value_size,
            max_entries,
            num_entries: AtomicUsize::new(0),
            flags: flags.read_only as u32 | flags.write_only as u32 | 
                     flags.user_creatable as u32 | flags.per_cpu as u32 | 
                     flags.supports_preallocation as u32,
        };
        
        let mut entries = Vec::new();
        entries.resize(max_entries, Vec::new());
        
        Self {
            meta,
            entries: Mutex::new(entries),
        }
    }
    
    /// Insert into array map
    pub fn insert(&self, key: u32, value: &[u8]) -> Result<(), EbpfError> {
        let key_usize = key as usize;
        
        let mut entries = self.entries.lock();
        
        // Check bounds
        if key_usize >= entries.len() {
            drop(entries);
            return Err(EbpfError::IndexOutOfBounds {
                map_id: self.meta.map_id,
                index: key_usize,
                max_index: entries.len(),
            });
        }
        
        // Store value
        entries[key_usize] = value.to_vec();
        self.meta.num_entries.fetch_add(1, Ordering::Relaxed);
        
        Ok(())
    }
    
    /// Lookup in array map
    pub fn lookup(&self, key: u32) -> Option<Vec<u8>> {
        let entries = self.entries.lock();
        
        if let Some(entry) = entries.get(key as usize) {
            Some(entry.clone())
        } else {
            None
        }
    }
    
    /// Delete from array map
    pub fn delete(&self, key: u32) -> Result<(), EbpfError> {
        let mut entries = self.entries.lock();
        let key_usize = key as usize;
        
        if key_usize >= entries.len() {
            return Err(EbpfError::IndexOutOfBounds {
                map_id: self.meta.map_id,
                index: key_usize,
                max_index: entries.len(),
            });
        }
        
        if entries.get(key_usize).is_some() {
            entries[key_usize] = Vec::new();
            self.meta.num_entries.fetch_sub(1, Ordering::Relaxed);
            
            Ok(())
        } else {
            Err(EbpfError::KeyNotFound {
                map_id: self.meta.map_id,
            })
        }
    }
    
    /// Get all entries
    pub fn get_all_entries(&self) -> Vec<Vec<u8>> {
        let entries = self.entries.lock();
        entries.iter().filter(|v| !v.is_empty()).cloned().collect()
    }
    
    /// Generate unique map ID
    fn generate_map_id() -> u32 {
        static MAP_ID_COUNTER: AtomicU32 = AtomicU32::new(1);
        MAP_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
    }
}

/// Map manager
pub struct EbpfMapManager {
    /// All maps
    pub maps: Mutex<BTreeMap<u32, Arc<dyn EbpfMap + Send + Sync>>>,
    
    /// Next map FD
    pub next_map_fd: AtomicU32,
    
    /// Total maps created
    pub total_maps: AtomicUsize,
}

/// eBPF map trait
pub trait EbpfMap: Send + Sync {
    /// Get map metadata
    fn get_meta(&self) -> &EbpfMapMeta;
    
    /// Get map ID
    fn get_id(&self) -> u32;
}

impl EbpfMap for EbpfHashMap {
    fn get_meta(&self) -> &EbpfMapMeta {
        &self.meta
    }
    
    fn get_id(&self) -> u32 {
        self.meta.map_id
    }
}

impl EbpfMap for EbpfArrayMap {
    fn get_meta(&self) -> &EbpfMapMeta {
        &self.meta
    }
    
    fn get_id(&self) -> u32 {
        self.meta.map_id
    }
}

impl EbpfMapManager {
    /// Create new map manager
    pub fn new() -> Self {
        Self {
            maps: Mutex::new(BTreeMap::new()),
            next_map_fd: AtomicU32::new(1),
            total_maps: AtomicUsize::new(0),
        }
    }
    
    /// Create hash map
    pub fn create_hash_map(&self, name: String, key_type: EbpfKeyType, 
                       value_size: EbpfValueSize, max_entries: usize, 
                       flags: EbpfMapFlags) -> Result<u32, EbpfError> {
        
        let map = Arc::new(EbpfHashMap::new(name, key_type, value_size, max_entries, flags) as Arc<dyn EbpfMap + Send + Sync>);
        let map_id = map.get_id();
        
        let mut maps = self.maps.lock();
        
        // Check for duplicate names
        for existing_map in maps.values() {
            if existing_map.get_meta().name == name {
                return Err(EbpfError::DuplicateName {
                    name,
                });
            }
        }
        
        let map_fd = self.next_map_fd.fetch_add(1, Ordering::Relaxed);
        maps.insert(map_id, map);
        self.total_maps.fetch_add(1, Ordering::Relaxed);
        
        crate::println!("[ebpf] Created hash map {} with FD {}", name, map_fd);
        
        Ok(map_fd)
    }
    
    /// Create array map
    pub fn create_array_map(&self, name: String, key_type: EbpfKeyType, 
                       value_size: EbpfValueSize, max_entries: usize, 
                       flags: EbpfMapFlags) -> Result<u32, EbpfError> {
        
        let map = Arc::new(EbpfArrayMap::new(name, key_type, value_size, max_entries, flags) as Arc<dyn EbpfMap + Send + Sync>);
        let map_id = map.get_id();
        
        let mut maps = self.maps.lock();
        maps.insert(map_id, map);
        self.total_maps.fetch_add(1, Ordering::Relaxed);
        
        crate::println!("[ebpf] Created array map {} with ID {}", name, map_id);
        
        Ok(map_id)
    }
    
    /// Get map by FD
    pub fn get_map(&self, map_fd: u32) -> Option<Arc<dyn EbpfMap + Send + Sync>> {
        let maps = self.maps.lock();
        maps.get(&map_fd).cloned()
    }
    
    /// Delete map
    pub fn delete_map(&self, map_fd: u32) -> Result<(), EbpfError> {
        let mut maps = self.maps.lock();
        
        if maps.remove(&map_fd).is_some() {
            crate::println!("[ebpf] Deleted map with FD {}", map_fd);
            Ok(())
        } else {
            Err(EbpfError::MapNotFound { map_fd })
        }
    }
    
    /// Get all maps
    pub fn get_all_maps(&self) -> Vec<(u32, Arc<dyn EbpfMap + Send + Sync>)> {
        let maps = self.maps.lock();
        maps.iter().map(|(&fd, map)| (fd, map.clone())).collect()
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> EbpfMapStats {
        let maps = self.maps.lock();
        
        let mut total_entries = 0usize;
        let mut total_hash_maps = 0usize;
        let mut total_array_maps = 0usize;
        
        for map in maps.values() {
            total_entries += map.get_meta().num_entries.load(Ordering::Relaxed);
            
            match map.get_meta().map_type {
                EbpfMapType::Hash | EbpfMapType::LruHash => {
                    total_hash_maps += 1;
                }
                EbpfMapType::Array => {
                    total_array_maps += 1;
                }
                _ => {}
            }
        }
        
        EbpfMapStats {
            total_maps: self.total_maps.load(Ordering::Relaxed),
            total_hash_maps,
            total_array_maps,
            total_entries,
        }
    }
}

/// eBPF map statistics
#[derive(Debug, Clone, Copy)]
pub struct EbpfMapStats {
    pub total_maps: usize,
    pub total_hash_maps: usize,
    pub total_array_maps: usize,
    pub total_entries: usize,
}

/// eBPF map errors
#[derive(Debug, Clone)]
pub enum EbpfError {
    /// Map is full
    MapFull {
        map_id: u32,
    },
    
    /// Key not found
    KeyNotFound {
        map_id: u32,
    },
    
    /// Index out of bounds (array map)
    IndexOutOfBounds {
        map_id: u32,
        index: usize,
        max_index: usize,
    },
    
    /// Map is empty (LRU eviction)
    MapEmpty {
        map_id: u32,
    },
    
    /// Map not found
    MapNotFound {
        map_fd: u32,
    },
    
    /// Duplicate map name
    DuplicateName {
        name: String,
    },
    
    /// Invalid key size
    InvalidKeySize,
    
    /// Invalid value size
    InvalidValueSize,
    
    /// Map creation failed
    CreationFailed {
        reason: String,
    },
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebpf_hash_entry() {
        let key = {
    let mut v = alloc::vec::Vec::new();
    v.push(0x01);
    v.push(0x02);
    v.push(0x03);
    v
};
        let value = {
    let mut v = alloc::vec::Vec::new();
    v.push(0xDE);
    v.push(0xAD);
    v.push(0xBE);
    v.push(0xEF);
    v
};
        
        let entry = EbpfHashEntry::new(&key, &value);
        
        assert_eq!(entry.key_len, 3);
        assert_eq!(entry.value_len, 4);
        assert!(entry.key[..key.len()] == key[..]);
        assert!(entry.value == value);
    }

    #[test]
    fn test_ebpf_hash_map() {
        let flags = EbpfMapFlags {
            read_only: false,
            write_only: false,
            user_creatable: true,
            per_cpu: false,
            supports_preallocation: true,
        };
        
        let map = EbpfHashMap::new(
            String::from("test_map"),
            EbpfKeyType::U32,
            EbpfValueSize::Size32,
            16,
            flags
        );
        
        let key = {
    let mut v = alloc::vec::Vec::new();
    v.push(0x01);
    v.push(0x02);
    v.push(0x03);
    v.push(0x04);
    v
};
        let value = {
    let mut v = alloc::vec::Vec::new();
    v.push(0xDE);
    v.push(0xAD);
    v.push(0xBE);
    v.push(0xEF);
    v
};
        
        map.insert(&key, &value).unwrap();
        
        let entry = map.lookup(&key).unwrap();
        assert_eq!(entry.value, value);
    }

    #[test]
    fn test_ebpf_array_map() {
        let flags = EbpfMapFlags {
            read_only: false,
            write_only: false,
            user_creatable: true,
            per_cpu: false,
            supports_preallocation: false,
        };
        
        let map = EbpfArrayMap::new(
            String::from("test_array"),
            EbpfKeyType::U32,
            EbpfValueSize::Size32,
            8,
            flags
        );
        
        let value = {
    let mut v = alloc::vec::Vec::new();
    v.push(0x01);
    v.push(0x02);
    v.push(0x03);
    v.push(0x04);
    v
};
        map.insert(0, &value).unwrap();
        
        let entry = map.lookup(0).unwrap();
        assert_eq!(entry, value);
    }

    #[test]
    fn test_ebpf_map_manager() {
        let manager = EbpfMapManager::new();
        
        let flags = EbpfMapFlags {
            read_only: false,
            write_only: false,
            user_creatable: true,
            per_cpu: false,
            supports_preallocation: true,
        };
        
        let map_fd = manager.create_hash_map(
            String::from("test_map"),
            EbpfKeyType::U32,
            EbpfValueSize::Size32,
            16,
            flags
        ).unwrap();
        
        let map = manager.get_map(map_fd).unwrap();
        assert_eq!(map.get_meta().name, "test_map");
        assert_eq!(map.get_id(), map_fd);
    }
}
