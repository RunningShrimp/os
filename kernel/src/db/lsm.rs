//! LSM-Tree (Log-Structured Merge Tree) Storage Engine
//!
//! Write-optimized storage engine with:
//! - MemTable (in-memory write buffer)
//! - SSTable (Sorted String Table)
//! - Compaction (merging SSTables)
//! - Multi-level storage

use super::types::{Value, PageId};
use super::{DbError, DbResult};
use crate::sync::Mutex;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cmp::Ordering;

/// LSM-Tree configuration
#[derive(Debug, Clone)]
pub struct LsmConfig {
    /// MemTable size threshold in bytes
    pub memtable_threshold: usize,

    /// Number of levels
    pub num_levels: usize,

    /// Size multiplier between levels
    pub level_multiplier: usize,

    /// SSTable size in bytes
    pub sst_size: usize,

    /// Enable compaction
    pub enable_compaction: bool,

    /// Compaction strategy
    pub compaction_strategy: CompactionStrategy,
}

impl Default for LsmConfig {
    fn default() -> Self {
        Self {
            memtable_threshold: 64 * 1024 * 1024, // 64MB
            num_levels: 7,
            level_multiplier: 10,
            sst_size: 64 * 1024 * 1024, // 64MB
            enable_compaction: true,
            compaction_strategy: CompactionStrategy::Leveled,
        }
    }
}

/// Compaction strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionStrategy {
    /// Leveled compaction (like LevelDB)
    Leveled,

    /// Tiered compaction (like Cassandra)
    Tiered,

    /// Universal compaction (like RocksDB)
    Universal,
}

/// LSM-Tree storage engine
pub struct LsmTree {
    config: LsmConfig,
    memtable: Mutex<Arc<MemTable>>,
    immutable_memtables: Mutex<Vec<Arc<MemTable>>>,
    levels: Mutex<Vec<Level>>,
    next_sst_id: Mutex<u64>,
    wal_enabled: Mutex<bool>,
}

impl LsmTree {
    pub fn new(config: LsmConfig) -> Self {
        let mut levels = Vec::new();
        for _ in 0..config.num_levels {
            levels.push(Level::new());
        }

        Self {
            config,
            memtable: Mutex::new(Arc::new(MemTable::new())),
            immutable_memtables: Mutex::new(Vec::new()),
            levels: Mutex::new(levels),
            next_sst_id: Mutex::new(1),
            wal_enabled: Mutex::new(true),
        }
    }

    /// Write a key-value pair
    pub fn put(&self, key: Value, value: Value) -> DbResult<()> {
        // Write to WAL if enabled
        if *self.wal_enabled.lock() {
            // GH-#1260: Write to WAL
            // See: https://github.com/npos/kernel/issues/1260
        }

        // Write to memtable
        let mut memtable = self.memtable.lock();
        let mem_ref = Arc::make_mut(&mut memtable);

        mem_ref.put(key, value);

        // Check if need to flush
        if mem_ref.size() >= self.config.memtable_threshold {
            self.flush_memtable()?;
        }

        Ok(())
    }

    /// Get a value by key
    pub fn get(&self, key: &Value) -> DbResult<Option<Value>> {
        // Check memtable first
        {
            let memtable = self.memtable.lock();
            if let Some(value) = memtable.get(key) {
                return Ok(Some(value));
            }
        }

        // Check immutable memtables
        {
            let imm_tables = self.immutable_memtables.lock();
            for memtable in imm_tables.iter().rev() {
                if let Some(value) = memtable.get(key) {
                    return Ok(Some(value));
                }
            }
        }

        // Check levels (from newest to oldest)
        let levels = self.levels.lock();
        for level in levels.iter() {
            if let Some(value) = level.get(key)? {
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    /// Delete a key
    pub fn delete(&self, key: &Value) -> DbResult<()> {
        self.put(key.clone(), Value::Null)
    }

    /// Flush memtable to disk
    fn flush_memtable(&self) -> DbResult<()> {
        // Swap memtable with new one
        let mut memtable_guard = self.memtable.lock();
        let old_memtable = core::mem::replace(&mut *memtable_guard, Arc::new(MemTable::new()));
        drop(memtable_guard);

        // Add to immutable memtables
        let mut imm_tables = self.immutable_memtables.lock();
        imm_tables.push(old_memtable.clone());

        // Flush to level 0
        let sst = self.flush_to_sstable(&old_memtable)?;

        // Add to level 0
        let mut levels = self.levels.lock();
        if let Some(level) = levels.get_mut(0) {
            level.add_sstable(sst);
        }

        // Remove from immutable memtables
        imm_tables.retain(|m| !Arc::ptr_eq(m, &old_memtable));

        // Trigger compaction if needed
        if self.config.enable_compaction {
            self.maybe_compact(&mut levels)?;
        }

        Ok(())
    }

    /// Flush memtable to SSTable
    fn flush_to_sstable(&self, memtable: &Arc<MemTable>) -> DbResult<SsTable> {
        let sst_id = {
            let mut next_id = self.next_sst_id.lock();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let mut sst = SsTable::new(sst_id, 0);

        // Copy entries from memtable
        for (key, value) in memtable.entries.lock().iter() {
            sst.put(key.clone(), value.clone());
        }

        Ok(sst)
    }

    /// Check if compaction is needed and perform it
    fn maybe_compact(&self, levels: &mut Vec<Level>) -> DbResult<()> {
        // Check each level
        for level_idx in 0..levels.len() - 1 {
            let level = &levels[level_idx];

            if level.needs_compaction(level_idx, &self.config) {
                self.compact_level(levels, level_idx)?;
            }
        }

        Ok(())
    }

    /// Compact a level
    fn compact_level(&self, levels: &mut Vec<Level>, level_idx: usize) -> DbResult<()> {
        match self.config.compaction_strategy {
            CompactionStrategy::Leveled => self.leveled_compaction(levels, level_idx),
            CompactionStrategy::Tiered => self.tiered_compaction(levels, level_idx),
            CompactionStrategy::Universal => self.universal_compaction(levels, level_idx),
        }
    }

    /// Leveled compaction (like LevelDB)
    fn leveled_compaction(&self, levels: &mut Vec<Level>, level_idx: usize) -> DbResult<()> {
        let level = &levels[level_idx];
        let next_level = &mut levels[level_idx + 1];

        // Get SSTables to compact
        let ssts_to_compact: Vec<_> = level.sstables().iter().cloned().collect();

        if ssts_to_compact.is_empty() {
            return Ok(());
        }

        // Merge with overlapping SSTables in next level
        let overlapping = next_level.find_overlapping(&ssts_to_compact);

        // Merge and create new SSTables
        let merged = self.merge_sstables(&ssts_to_compact, &overlapping)?;

        // Remove old SSTables
        for sst in ssts_to_compact {
            level.remove_sstable(sst.id());
        }

        for sst in overlapping {
            next_level.remove_sstable(sst.id());
        }

        // Add merged SSTables to next level
        for sst in merged {
            next_level.add_sstable(sst);
        }

        Ok(())
    }

    /// Tiered compaction (like Cassandra)
    fn tiered_compaction(&self, levels: &mut Vec<Level>, level_idx: usize) -> DbResult<()> {
        // Similar to leveled but merges entire tiers
        self.leveled_compaction(levels, level_idx)
    }

    /// Universal compaction (like RocksDB)
    fn universal_compaction(&self, levels: &mut Vec<Level>, level_idx: usize) -> DbResult<()> {
        // Single-level compaction for all files
        self.leveled_compaction(levels, level_idx)
    }

    /// Merge multiple SSTables
    fn merge_sstables(&self, ssts1: &[SsTable], ssts2: &[SsTable]) -> DbResult<Vec<SsTable>> {
        let mut merged = Vec::new();
        let mut merged_sst = SsTable::new(self.next_sst_id_locked(), 0);

        // Collect all entries and sort
        let mut all_entries = BTreeMap::new();

        for sst in ssts1.iter().chain(ssts2.iter()) {
            for (key, value) in sst.entries() {
                // Keep the latest value (last write wins)
                all_entries.insert(key.clone(), value.clone());
            }
        }

        // Split into multiple SSTables if needed
        let mut current_size = 0;
        for (key, value) in all_entries {
            let entry_size = key.size() + value.size();

            if current_size + entry_size > self.config.sst_size && !merged_sst.is_empty() {
                merged.push(merged_sst);
                merged_sst = SsTable::new(self.next_sst_id_locked(), 0);
                current_size = 0;
            }

            merged_sst.put(key, value);
            current_size += entry_size;
        }

        if !merged_sst.is_empty() {
            merged.push(merged_sst);
        }

        Ok(merged)
    }

    fn next_sst_id_locked(&self) -> u64 {
        let mut id = self.next_sst_id.lock();
        let sst_id = *id;
        *id += 1;
        sst_id
    }

    /// Range scan
    pub fn scan(&self, range: impl Clone + core::ops::RangeBounds<Value>) -> DbResult<Vec<(Value, Value)>> {
        let mut results = Vec::new();
        let mut seen_keys = alloc::collections::BTreeSet::new();

        // Scan memtable
        {
            let memtable = self.memtable.lock();
            for (key, value) in memtable.range(range.clone()) {
                if seen_keys.insert(key.clone()) {
                    results.push((key, value));
                }
            }
        }

        // Scan immutable memtables
        {
            let imm_tables = self.immutable_memtables.lock();
            for memtable in imm_tables.iter().rev() {
                for (key, value) in memtable.range(range.clone()) {
                    if seen_keys.insert(key.clone()) {
                        results.push((key, value));
                    }
                }
            }
        }

        // Scan levels
        let levels = self.levels.lock();
        for level in levels.iter() {
            for (key, value) in level.range(range.clone())? {
                if seen_keys.insert(key.clone()) {
                    results.push((key, value));
                }
            }
        }

        Ok(results)
    }

    /// Get statistics
    pub fn stats(&self) -> DbResult<LsmStats> {
        let memtable = self.memtable.lock();
        let imm_tables = self.immutable_memtables.lock();
        let levels = self.levels.lock();

        let mut total_sstables = 0;
        let mut total_size = 0;

        for level in levels.iter() {
            total_sstables += level.sstables().len();
            for sst in level.sstables() {
                total_size += sst.size();
            }
        }

        Ok(LsmStats {
            memtable_size: memtable.size(),
            num_imm_memtables: imm_tables.len(),
            total_sstables,
            total_size,
            num_levels: self.config.num_levels,
        })
    }
}

/// In-memory write buffer
#[derive(Debug)]
pub struct MemTable {
    entries: Mutex<BTreeMap<Value, Value>>,
    size: Mutex<usize>,
}

impl MemTable {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(BTreeMap::new()),
            size: Mutex::new(0),
        }
    }

    pub fn put(&self, key: Value, value: Value) {
        let mut entries = self.entries.lock();
        let mut size = self.size.lock();

        // Remove old value if exists
        if let Some(old_value) = entries.get(&key) {
            *size -= key.size() + old_value.size();
        }

        entries.insert(key, value);
        *size += key.size() + value.size();
    }

    pub fn get(&self, key: &Value) -> Option<Value> {
        let entries = self.entries.lock();
        entries.get(key).cloned()
    }

    pub fn size(&self) -> usize {
        *self.size.lock()
    }

    pub fn range<'a, R>(&'a self, range: R) -> Vec<(Value, Value)>
    where
        R: core::ops::RangeBounds<Value> + Clone,
    {
        let entries = self.entries.lock();
        entries
            .range(range)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

/// Level in LSM tree
#[derive(Debug)]
pub struct Level {
    sstables: Vec<SsTable>,
    level_num: usize,
}

impl Level {
    pub fn new() -> Self {
        Self {
            sstables: Vec::new(),
            level_num: 0,
        }
    }

    pub fn with_level(level_num: usize) -> Self {
        Self {
            sstables: Vec::new(),
            level_num,
        }
    }

    pub fn add_sstable(&mut self, sst: SsTable) {
        self.sstables.push(sst);
    }

    pub fn remove_sstable(&mut self, sst_id: u64) {
        self.sstables.retain(|sst| sst.id() != sst_id);
    }

    pub fn sstables(&self) -> &[SsTable] {
        &self.sstables
    }

    pub fn get(&self, key: &Value) -> DbResult<Option<Value>> {
        for sst in self.sstables.iter().rev() {
            if let Some(value) = sst.get(key) {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }

    pub fn find_overlapping(&self, ssts: &[SsTable]) -> Vec<SsTable> {
        // Simple implementation: return all SSTables
        // In practice, would find overlapping ranges
        self.sstables.clone()
    }

    pub fn needs_compaction(&self, level_num: usize, config: &LsmConfig) -> bool {
        let total_size: usize = self.sstables.iter().map(|sst| sst.size()).sum();

        if level_num == 0 {
            self.sstables.len() > 4
        } else {
            total_size > config.sst_size * config.level_multiplier.pow(level_num as u32)
        }
    }

    pub fn range(&self, range: impl Clone + core::ops::RangeBounds<Value>) -> DbResult<Vec<(Value, Value)>> {
        let mut results = Vec::new();

        for sst in &self.sstables {
            for (key, value) in sst.entries() {
                // GH-#1261: Check if key is in range
                // See: https://github.com/npos/kernel/issues/1261
                results.push((key, value));
            }
        }

        Ok(results)
    }
}

/// Sorted String Table (SSTable)
#[derive(Debug, Clone)]
pub struct SsTable {
    id: u64,
    level: usize,
    entries: BTreeMap<Value, Value>,
}

impl SsTable {
    pub fn new(id: u64, level: usize) -> Self {
        Self {
            id,
            level,
            entries: BTreeMap::new(),
        }
    }

    pub fn put(&mut self, key: Value, value: Value) {
        self.entries.insert(key, value);
    }

    pub fn get(&self, key: &Value) -> Option<Value> {
        self.entries.get(key).cloned()
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn level(&self) -> usize {
        self.level
    }

    pub fn size(&self) -> usize {
        self.entries.iter().map(|(k, v)| k.size() + v.size()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> impl Iterator<Item = &(Value, Value)> {
        self.entries.iter()
    }
}

/// LSM-Tree statistics
#[derive(Debug, Clone)]
pub struct LsmStats {
    pub memtable_size: usize,
    pub num_imm_memtables: usize,
    pub total_sstables: usize,
    pub total_size: usize,
    pub num_levels: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsm_put_get() {
        let lsm = LsmTree::new(LsmConfig::default());

        let key = Value::Int32(1);
        let value = Value::Text("test".to_string());

        assert!(lsm.put(key.clone(), value.clone()).is_ok());
        assert_eq!(lsm.get(&key).unwrap(), Some(value));
    }

    #[test]
    fn test_lsm_delete() {
        let lsm = LsmTree::new(LsmConfig::default());

        let key = Value::Int32(1);
        let value = Value::Text("test".to_string());

        lsm.put(key.clone(), value.clone()).unwrap();
        lsm.delete(&key).unwrap();
        assert_eq!(lsm.get(&key).unwrap(), None);
    }
}
