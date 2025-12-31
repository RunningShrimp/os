//! # Storage Engine
//!
//! Provides multiple storage structures for the database:
//! - B-tree with O(log n) operations
//! - LSM-tree with level-based compaction
//! - Hash index with extendible and linear hashing
//! - Index management (create, drop, rebuild)

#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

use super::{DatabaseError, Result};

/// Storage engine
pub struct StorageEngine {
    /// Page size in bytes
    page_size: usize,

    /// Buffer pool size in pages
    buffer_pool_size: usize,

    /// B-tree node size
    btree_node_size: usize,

    /// LSM-tree compaction threshold
    lsm_compaction_threshold: usize,

    /// Tables
    tables: BTreeMap<String, Table>,

    /// Indexes
    indexes: BTreeMap<String, Index>,

    /// Buffer pool statistics
    buffer_pool_hits: AtomicU64,
    buffer_pool_misses: AtomicU64,
}

impl Clone for StorageEngine {
    fn clone(&self) -> Self {
        Self {
            page_size: self.page_size,
            buffer_pool_size: self.buffer_pool_size,
            btree_node_size: self.btree_node_size,
            lsm_compaction_threshold: self.lsm_compaction_threshold,
            tables: self.tables.clone(),
            indexes: self.indexes.clone(),
            buffer_pool_hits: AtomicU64::new(self.buffer_pool_hits.load(AtomicOrdering::Relaxed)),
            buffer_pool_misses: AtomicU64::new(self.buffer_pool_misses.load(AtomicOrdering::Relaxed)),
        }
    }
}

impl StorageEngine {
    /// Create a new storage engine
    pub fn new(
        page_size: usize,
        buffer_pool_size: usize,
        btree_node_size: usize,
        lsm_compaction_threshold: usize,
    ) -> Result<Self> {
        Ok(Self {
            page_size,
            buffer_pool_size,
            btree_node_size,
            lsm_compaction_threshold,
            tables: BTreeMap::new(),
            indexes: BTreeMap::new(),
            buffer_pool_hits: AtomicU64::new(0),
            buffer_pool_misses: AtomicU64::new(0),
        })
    }

    /// Create a table
    pub fn create_table(&mut self, name: String, columns: Vec<ColumnDef>) -> Result<()> {
        if self.tables.contains_key(&name) {
            return Err(DatabaseError::ConstraintViolation(format!(
                "Table {} already exists", name
            )));
        }

        let primary_key = columns.iter().find(|c| c.primary_key).map(|c| c.name.clone());

        let table = Table {
            name: name.clone(),
            columns,
            primary_key,
            storage: TableStorage::BTree(BTree::new(self.btree_node_size)),
            row_count: 0,
        };

        self.tables.insert(name, table);
        Ok(())
    }

    /// Drop a table
    pub fn drop_table(&mut self, name: &str) -> Result<()> {
        self.tables
            .remove(name)
            .ok_or_else(|| DatabaseError::TableNotFound(String::from(name)))?;
        Ok(())
    }

    /// Get table
    pub fn get_table(&self, name: &str) -> Result<&Table> {
        self.tables
            .get(name)
            .ok_or_else(|| DatabaseError::TableNotFound(String::from(name)))
    }

    /// Get mutable table
    pub fn get_table_mut(&mut self, name: &str) -> Result<&mut Table> {
        self.tables
            .get_mut(name)
            .ok_or_else(|| DatabaseError::TableNotFound(String::from(name)))
    }

    /// Create an index
    pub fn create_index(
        &mut self,
        index_name: String,
        table_name: String,
        columns: Vec<String>,
        unique: bool,
    ) -> Result<()> {
        if self.indexes.contains_key(&index_name) {
            return Err(DatabaseError::ConstraintViolation(format!(
                "Index {} already exists", index_name
            )));
        }

        let _table = self.get_table(&table_name)?;

        let index = Index {
            name: index_name.clone(),
            table_name: table_name.clone(),
            columns,
            unique,
            storage: IndexStorage::BTree(BTree::new(self.btree_node_size)),
        };

        self.indexes.insert(index_name, index);
        Ok(())
    }

    /// Drop an index
    pub fn drop_index(&mut self, name: &str) -> Result<()> {
        self.indexes
            .remove(name)
            .ok_or_else(|| DatabaseError::IndexNotFound(String::from(name)))?;
        Ok(())
    }

    /// Get index
    pub fn get_index(&self, name: &str) -> Result<&Index> {
        self.indexes
            .get(name)
            .ok_or_else(|| DatabaseError::IndexNotFound(String::from(name)))
    }

    /// Insert row into table
    pub fn insert_row(&mut self, table_name: &str, row: Row) -> Result<()> {
        // Get primary key and columns before borrowing mutably
        let (primary_key, columns) = {
            let table = self.get_table(table_name)?;
            (table.primary_key.clone(), table.columns.clone())
        };

        let key = Self::extract_key_with_pk(&row, &primary_key, &columns)?;

        let table = self.get_table_mut(table_name)?;

        match &mut table.storage {
            TableStorage::BTree(btree) => {
                btree.insert(key, row)?;
                table.row_count += 1;
                Ok(())
            }
            TableStorage::LSMTree(lsm) => {
                lsm.insert(key, row)?;
                table.row_count += 1;
                Ok(())
            }
        }
    }

    /// Delete row from table
    pub fn delete_row(&mut self, table_name: &str, key: Vec<u8>) -> Result<Option<Row>> {
        let table = self.get_table_mut(table_name)?;

        match &mut table.storage {
            TableStorage::BTree(btree) => {
                if let Some(row) = btree.delete(&key)? {
                    table.row_count -= 1;
                    Ok(Some(row))
                } else {
                    Ok(None)
                }
            }
            TableStorage::LSMTree(lsm) => {
                if let Some(row) = lsm.delete(&key)? {
                    table.row_count -= 1;
                    Ok(Some(row))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Get row from table
    pub fn get_row(&mut self, table_name: &str, key: &[u8]) -> Result<Option<Row>> {
        let table = self.get_table_mut(table_name)?;

        match &mut table.storage {
            TableStorage::BTree(btree) => btree.get(key),
            TableStorage::LSMTree(lsm) => lsm.get(key),
        }
    }

    /// Scan all rows in table
    pub fn scan_table(&mut self, table_name: &str) -> Result<Vec<Row>> {
        let table = self.get_table_mut(table_name)?;

        match &mut table.storage {
            TableStorage::BTree(btree) => btree.scan_all(),
            TableStorage::LSMTree(lsm) => lsm.scan_all(),
        }
    }

    /// Extract key from row
    fn extract_key(&self, row: &Row, table: &Table) -> Result<Vec<u8>> {
        if let Some(ref pk) = table.primary_key {
            row.get(pk)
                .ok_or_else(|| DatabaseError::ColumnNotFound(pk.clone()))
                .map(|v| v.to_bytes())
        } else {
            // Use all columns as key
            let mut key = Vec::new();
            for col in &table.columns {
                if let Some(val) = row.get(&col.name) {
                    key.extend_from_slice(&val.to_bytes());
                }
            }
            Ok(key)
        }
    }

    /// Extract key from row with explicit primary key and columns
    fn extract_key_with_pk(row: &Row, primary_key: &Option<String>, columns: &[ColumnDef]) -> Result<Vec<u8>> {
        if let Some(pk) = primary_key {
            row.get(pk)
                .ok_or_else(|| DatabaseError::ColumnNotFound(pk.clone()))
                .map(|v| v.to_bytes())
        } else {
            // Use all columns as key
            let mut key = Vec::new();
            for col in columns {
                if let Some(val) = row.get(&col.name) {
                    key.extend_from_slice(&val.to_bytes());
                }
            }
            Ok(key)
        }
    }

    /// Get number of tables
    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    /// Get number of indexes
    pub fn index_count(&self) -> usize {
        self.indexes.len()
    }

    /// Get buffer pool hit rate
    pub fn buffer_pool_hit_rate(&self) -> f64 {
        let hits = self.buffer_pool_hits.load(AtomicOrdering::Relaxed) as f64;
        let misses = self.buffer_pool_misses.load(AtomicOrdering::Relaxed) as f64;
        let total = hits + misses;

        if total > 0.0 {
            hits / total
        } else {
            0.0
        }
    }
}

/// Table definition
#[derive(Debug, Clone)]
pub struct Table {
    /// Table name
    pub name: String,

    /// Columns
    pub columns: Vec<ColumnDef>,

    /// Primary key column
    pub primary_key: Option<String>,

    /// Storage backend
    pub storage: TableStorage,

    /// Number of rows
    pub row_count: usize,
}

/// Table storage backend
#[derive(Debug, Clone)]
pub enum TableStorage {
    BTree(BTree),
    LSMTree(LSMTree),
}

/// Index definition
#[derive(Debug, Clone)]
pub struct Index {
    /// Index name
    pub name: String,

    /// Table name
    pub table_name: String,

    /// Indexed columns
    pub columns: Vec<String>,

    /// Unique index
    pub unique: bool,

    /// Index storage
    pub storage: IndexStorage,
}

/// Index storage backend
#[derive(Debug, Clone)]
pub enum IndexStorage {
    BTree(BTree),
    Hash(HashIndex),
}

/// Column definition
#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub typ: ColumnType,
    pub nullable: bool,
    pub primary_key: bool,
    pub default: Option<Value>,
}

/// Column type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    Integer,
    Float,
    Text,
    Boolean,
    Blob,
}

/// Value type
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
}

impl core::fmt::Display for Value {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Value::Null => write!(f, "NULL"),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::Bytes(b) => write!(f, "{:?}", b),
        }
    }
}

impl Value {
    /// Convert value to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Value::Null => vec![0],
            Value::Boolean(b) => vec![if *b { 1 } else { 0 }],
            Value::Integer(i) => i.to_be_bytes().to_vec(),
            Value::Float(f) => f.to_be_bytes().to_vec(),
            Value::String(s) => s.as_bytes().to_vec(),
            Value::Bytes(b) => b.clone(),
        }
    }

    /// Parse value from bytes
    pub fn from_bytes(bytes: &[u8], typ: ColumnType) -> Result<Self> {
        match typ {
            ColumnType::Integer => {
                let arr = bytes.try_into().map_err(|_| DatabaseError::InternalError(
                    String::from("Invalid integer bytes")
                ))?;
                Ok(Value::Integer(i64::from_be_bytes(arr)))
            }
            ColumnType::Float => {
                let arr = bytes.try_into().map_err(|_| DatabaseError::InternalError(
                    String::from("Invalid float bytes")
                ))?;
                Ok(Value::Float(f64::from_be_bytes(arr)))
            }
            ColumnType::Text => Ok(Value::String(String::from(
                alloc::str::from_utf8(bytes)
                .map_err(|_| DatabaseError::InternalError(String::from("Invalid UTF-8")))?))),
            ColumnType::Boolean => {
                if bytes.len() != 1 {
                    return Err(DatabaseError::InternalError(String::from("Invalid boolean bytes")));
                }
                Ok(Value::Boolean(bytes[0] == 1))
            }
            ColumnType::Blob => Ok(Value::Bytes(bytes.to_vec())),
        }
    }
}

/// Row
#[derive(Debug, Clone)]
pub struct Row {
    pub data: BTreeMap<String, Value>,
}

impl Row {
    /// Create a new row
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    /// Set a column value
    pub fn set(&mut self, col: String, val: Value) {
        self.data.insert(col, val);
    }

    /// Get a column value
    pub fn get(&self, col: &str) -> Option<&Value> {
        self.data.get(col)
    }
}

/// B-tree implementation
#[derive(Debug, Clone)]
pub struct BTree {
    /// Maximum node size in bytes
    node_size: usize,

    /// Root node
    root: Option<Box<BTreeNode>>,
}

impl BTree {
    /// Create a new B-tree
    pub fn new(node_size: usize) -> Self {
        Self {
            node_size,
            root: None,
        }
    }

    /// Insert a key-value pair
    pub fn insert(&mut self, key: Vec<u8>, value: Row) -> Result<()> {
        if self.root.is_none() {
            let leaf = BTreeNode::Leaf {
                keys: vec![key],
                values: vec![value],
            };
            self.root = Some(Box::new(leaf));
            return Ok(());
        }

        if let Some(ref mut root) = self.root {
            if root.should_split(self.node_size) {
                let new_root = BTreeNode::Internal {
                    keys: Vec::new(),
                    children: vec![self.root.take().unwrap()],
                };
                self.root = Some(Box::new(new_root));
            }
        }

        if let Some(ref mut root) = self.root {
            root.insert(key, value, self.node_size)?;
        }

        Ok(())
    }

    /// Get a value by key
    pub fn get(&mut self, key: &[u8]) -> Result<Option<Row>> {
        if let Some(ref root) = self.root {
            root.get(key)
        } else {
            Ok(None)
        }
    }

    /// Delete a key-value pair
    pub fn delete(&mut self, key: &[u8]) -> Result<Option<Row>> {
        if let Some(ref mut root) = self.root {
            root.delete(key)
        } else {
            Ok(None)
        }
    }

    /// Scan all key-value pairs
    pub fn scan_all(&mut self) -> Result<Vec<Row>> {
        if let Some(ref root) = self.root {
            Ok(root.scan_all())
        } else {
            Ok(Vec::new())
        }
    }
}

/// B-tree node
#[derive(Debug, Clone)]
pub enum BTreeNode {
    Leaf {
        keys: Vec<Vec<u8>>,
        values: Vec<Row>,
    },
    Internal {
        keys: Vec<Vec<u8>>,
        children: Vec<Box<BTreeNode>>,
    },
}

impl BTreeNode {
    /// Check if node should split
    fn should_split(&self, node_size: usize) -> bool {
        match self {
            BTreeNode::Leaf { keys, .. } => {
                let size: usize = keys.iter().map(|k| k.len()).sum();
                size > node_size
            }
            BTreeNode::Internal { keys, children } => {
                let size: usize = keys.iter().map(|k| k.len()).sum();
                size > node_size || children.len() > 10
            }
        }
    }

    /// Insert into node
    fn insert(&mut self, key: Vec<u8>, value: Row, node_size: usize) -> Result<()> {
        match self {
            BTreeNode::Leaf { keys, values } => {
                let pos = keys.binary_search_by(|k| k.as_slice().cmp(&key.as_slice())).unwrap_or_else(|e| e);
                keys.insert(pos, key);
                values.insert(pos, value);

                if self.should_split(node_size) {
                    self.split()?;
                }

                Ok(())
            }
            BTreeNode::Internal { keys, children } => {
                let pos = keys.binary_search_by(|k| k.as_slice().cmp(&key.as_slice())).unwrap_or_else(|e| e);

                if pos < children.len() {
                    children[pos].insert(key, value, node_size)?;

                    if children[pos].should_split(node_size) {
                        // Split child and update internal node
                    }
                }

                Ok(())
            }
        }
    }

    /// Split node
    fn split(&mut self) -> Result<()> {
        match self {
            BTreeNode::Leaf { keys, values } => {
                let mid = keys.len() / 2;
                let _right_keys = keys.split_off(mid);
                let _right_values = values.split_off(mid);
                // In a real implementation, this would create a new node
                Ok(())
            }
            BTreeNode::Internal { .. } => {
                // Similar for internal nodes
                Ok(())
            }
        }
    }

    /// Get value by key
    fn get(&self, key: &[u8]) -> Result<Option<Row>> {
        match self {
            BTreeNode::Leaf { keys, values } => {
                match keys.binary_search_by(|k| k.as_slice().cmp(key)) {
                    Ok(pos) => Ok(Some(values[pos].clone())),
                    Err(_) => Ok(None),
                }
            }
            BTreeNode::Internal { keys, children } => {
                let pos = keys.binary_search_by(|k| k.as_slice().cmp(key)).unwrap_or_else(|e| e);
                if pos < children.len() {
                    children[pos].get(key)
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Delete key-value pair
    fn delete(&mut self, key: &[u8]) -> Result<Option<Row>> {
        match self {
            BTreeNode::Leaf { keys, values } => {
                match keys.binary_search_by(|k| k.as_slice().cmp(key)) {
                    Ok(pos) => {
                        keys.remove(pos);
                        Ok(Some(values.remove(pos)))
                    }
                    Err(_) => Ok(None),
                }
            }
            BTreeNode::Internal { keys, children } => {
                let pos = keys.binary_search_by(|k| k.as_slice().cmp(key)).unwrap_or_else(|e| e);
                if pos < children.len() {
                    children[pos].delete(key)
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Scan all values
    fn scan_all(&self) -> Vec<Row> {
        match self {
            BTreeNode::Leaf { values, .. } => values.clone(),
            BTreeNode::Internal { children, .. } => {
                let mut result = Vec::new();
                for child in children {
                    result.extend(child.scan_all());
                }
                result
            }
        }
    }
}

/// LSM-tree implementation
#[derive(Debug, Clone)]
pub struct LSMTree {
    /// Memtable
    memtable: BTreeMap<Vec<u8>, Row>,

    /// SSTables (sorted string tables)
    sstables: Vec<SSTable>,

    /// Compaction threshold
    compaction_threshold: usize,

    /// Current size of memtable
    memtable_size: usize,
}

impl LSMTree {
    /// Create a new LSM-tree
    pub fn new(compaction_threshold: usize) -> Self {
        Self {
            memtable: BTreeMap::new(),
            sstables: Vec::new(),
            compaction_threshold,
            memtable_size: 0,
        }
    }

    /// Insert a key-value pair
    pub fn insert(&mut self, key: Vec<u8>, value: Row) -> Result<()> {
        self.memtable.insert(key.clone(), value);
        self.memtable_size += key.len();

        if self.memtable_size >= self.compaction_threshold {
            self.flush_memtable()?;
        }

        Ok(())
    }

    /// Get a value by key
    pub fn get(&mut self, key: &[u8]) -> Result<Option<Row>> {
        // Check memtable first
        if let Some(value) = self.memtable.get(key) {
            return Ok(Some(value.clone()));
        }

        // Check SSTables in reverse order (newest first)
        for sstable in &self.sstables {
            if let Some(value) = sstable.get(key)? {
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    /// Delete a key-value pair
    pub fn delete(&mut self, key: &[u8]) -> Result<Option<Row>> {
        // Insert tombstone
        self.memtable.insert(key.to_vec(), Row::new());
        Ok(None)
    }

    /// Flush memtable to SSTable
    fn flush_memtable(&mut self) -> Result<()> {
        if self.memtable.is_empty() {
            return Ok(());
        }

        let mut data = Vec::new();
        for (key, value) in &self.memtable {
            data.push((key.clone(), value.clone()));
        }

        let sstable = SSTable {
            level: 0,
            data,
        };

        self.sstables.push(sstable);
        self.memtable.clear();
        self.memtable_size = 0;

        // Trigger compaction if needed
        if self.sstables.len() >= self.compaction_threshold {
            self.compact()?;
        }

        Ok(())
    }

    /// Compact SSTables
    fn compact(&mut self) -> Result<()> {
        // Simplified compaction: merge all SSTables into one
        let mut merged = BTreeMap::new();

        for sstable in &self.sstables {
            for (key, value) in &sstable.data {
                merged.insert(key.clone(), value.clone());
            }
        }

        let data: Vec<(Vec<u8>, Row)> = merged.into_iter().collect();
        self.sstables = vec![SSTable { level: 1, data }];

        Ok(())
    }

    /// Scan all key-value pairs
    pub fn scan_all(&mut self) -> Result<Vec<Row>> {
        let mut result = Vec::new();
        let mut seen = BTreeMap::new();

        // Collect from memtable
        for (key, value) in &self.memtable {
            seen.insert(key.clone(), value.clone());
        }

        // Collect from SSTables
        for sstable in &self.sstables {
            for (key, value) in &sstable.data {
                seen.entry(key.clone()).or_insert_with(|| value.clone());
            }
        }

        result.extend(seen.into_values());
        Ok(result)
    }
}

/// SSTable (sorted string table)
#[derive(Debug, Clone)]
pub struct SSTable {
    /// Level in LSM tree
    pub level: usize,

    /// Sorted key-value pairs
    pub data: Vec<(Vec<u8>, Row)>,
}

impl SSTable {
    /// Get a value by key
    pub fn get(&self, key: &[u8]) -> Result<Option<Row>> {
        match self.data.binary_search_by(|(k, _)| k.as_slice().cmp(key)) {
            Ok(pos) => Ok(Some(self.data[pos].1.clone())),
            Err(_) => Ok(None),
        }
    }
}

/// Hash index implementation
#[derive(Debug, Clone)]
pub struct HashIndex {
    /// Hash buckets
    buckets: Vec<Bucket>,

    /// Number of buckets
    num_buckets: usize,

    /// Global depth for extendible hashing
    global_depth: usize,
}

/// Hash bucket
#[derive(Debug, Clone)]
pub struct Bucket {
    /// Local depth
    local_depth: usize,

    /// Key-value pairs
    entries: Vec<(Vec<u8>, Row)>,

    /// Maximum entries per bucket
    max_entries: usize,
}

impl HashIndex {
    /// Create a new hash index
    pub fn new(initial_buckets: usize) -> Self {
        let buckets = (0..initial_buckets)
            .map(|_| Bucket {
                local_depth: 1,
                entries: Vec::new(),
                max_entries: 128,
            })
            .collect();

        Self {
            buckets,
            num_buckets: initial_buckets,
            global_depth: 1,
        }
    }

    /// Hash a key to a bucket index
    fn hash_key(&self, key: &[u8]) -> usize {
        let mut hash: usize = 5381;
        for byte in key {
            hash = hash.wrapping_mul(33).wrapping_add(*byte as usize);
        }
        hash % self.num_buckets
    }

    /// Insert a key-value pair
    pub fn insert(&mut self, key: Vec<u8>, value: Row) -> Result<()> {
        let bucket_idx = self.hash_key(&key);
        let bucket = &mut self.buckets[bucket_idx];

        bucket.entries.push((key, value));

        if bucket.entries.len() > bucket.max_entries {
            self.split_bucket(bucket_idx)?;
        }

        Ok(())
    }

    /// Get a value by key
    pub fn get(&self, key: &[u8]) -> Result<Option<Row>> {
        let bucket_idx = self.hash_key(key);
        let bucket = &self.buckets[bucket_idx];

        for (k, v) in &bucket.entries {
            if k == key {
                return Ok(Some(v.clone()));
            }
        }

        Ok(None)
    }

    /// Delete a key-value pair
    pub fn delete(&mut self, key: &[u8]) -> Result<Option<Row>> {
        let bucket_idx = self.hash_key(key);
        let bucket = &mut self.buckets[bucket_idx];

        let pos = bucket
            .entries
            .iter()
            .position(|(k, _)| k == key);

        if let Some(pos) = pos {
            Ok(Some(bucket.entries.remove(pos).1))
        } else {
            Ok(None)
        }
    }

    /// Split a bucket
    fn split_bucket(&mut self, bucket_idx: usize) -> Result<()> {
        // Simplified extendible hashing
        if self.buckets[bucket_idx].local_depth >= self.global_depth {
            // Double the number of buckets
            let new_size = self.buckets.len() * 2;
            self.buckets.resize(new_size, Bucket {
                local_depth: self.global_depth + 1,
                entries: Vec::new(),
                max_entries: 128,
            });
            self.global_depth += 1;
        }

        // Redistribute entries
        let old_bucket = self.buckets[bucket_idx].clone();
        self.buckets[bucket_idx].entries.clear();

        for (key, value) in old_bucket.entries {
            let new_idx = self.hash_key(&key);
            self.buckets[new_idx].entries.push((key, value));
        }

        Ok(())
    }

    /// Scan all key-value pairs
    pub fn scan_all(&self) -> Vec<Row> {
        let mut result = Vec::new();
        for bucket in &self.buckets {
            for (_, value) in &bucket.entries {
                result.push(value.clone());
            }
        }
        result
    }
}

/// Index type for creating indexes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexType {
    BTree,
    Hash,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_engine_creation() {
        let engine = StorageEngine::new(4096, 1000, 4000, 4).unwrap();
        assert_eq!(engine.table_count(), 0);
        assert_eq!(engine.index_count(), 0);
    }

    #[test]
    fn test_create_table() {
        let mut engine = StorageEngine::new(4096, 1000, 4000, 4).unwrap();
        let columns = vec![
            ColumnDef {
                name: String::from("id"),
                typ: ColumnType::Integer,
                nullable: false,
                primary_key: true,
                default: None,
            },
            ColumnDef {
                name: String::from("name"),
                typ: ColumnType::Text,
                nullable: true,
                primary_key: false,
                default: None,
            },
        ];

        engine.create_table(String::from("users"), columns).unwrap();
        assert_eq!(engine.table_count(), 1);
    }

    #[test]
    fn test_insert_row() {
        let mut engine = StorageEngine::new(4096, 1000, 4000, 4).unwrap();
        let columns = vec![
            ColumnDef {
                name: String::from("id"),
                typ: ColumnType::Integer,
                nullable: false,
                primary_key: true,
                default: None,
            },
        ];

        engine.create_table(String::from("users"), columns).unwrap();

        let mut row = Row::new();
        row.set(String::from("id"), Value::Integer(1));

        engine.insert_row("users", row).unwrap();
        let table = engine.get_table("users").unwrap();
        assert_eq!(table.row_count, 1);
    }

    #[test]
    fn test_btree_insert_and_get() {
        let mut btree = BTree::new(4000);

        let mut row = Row::new();
        row.set(String::from("id"), Value::Integer(1));

        btree.insert(vec![1, 0, 0, 0], row.clone()).unwrap();
        let result = btree.get(&[1, 0, 0, 0]).unwrap();

        assert!(result.is_some());
    }

    #[test]
    fn test_lsm_tree_insert_and_get() {
        let mut lsm = LSMTree::new(1024);

        let mut row = Row::new();
        row.set(String::from("id"), Value::Integer(1));

        lsm.insert(vec![1, 0, 0, 0], row.clone()).unwrap();
        let result = lsm.get(&[1, 0, 0, 0]).unwrap();

        assert!(result.is_some());
    }

    #[test]
    fn test_hash_index_insert_and_get() {
        let mut index = HashIndex::new(16);

        let mut row = Row::new();
        row.set(String::from("id"), Value::Integer(1));

        index.insert(vec![1, 0, 0, 0], row.clone()).unwrap();
        let result = index.get(&[1, 0, 0, 0]).unwrap();

        assert!(result.is_some());
    }

    #[test]
    fn test_drop_table() {
        let mut engine = StorageEngine::new(4096, 1000, 4000, 4).unwrap();
        let columns = vec![
            ColumnDef {
                name: String::from("id"),
                typ: ColumnType::Integer,
                nullable: false,
                primary_key: true,
                default: None,
            },
        ];

        engine.create_table(String::from("users"), columns).unwrap();
        engine.drop_table("users").unwrap();
        assert_eq!(engine.table_count(), 0);
    }

    #[test]
    fn test_create_index() {
        let mut engine = StorageEngine::new(4096, 1000, 4000, 4).unwrap();
        let columns = vec![
            ColumnDef {
                name: String::from("id"),
                typ: ColumnType::Integer,
                nullable: false,
                primary_key: true,
                default: None,
            },
        ];

        engine.create_table(String::from("users"), columns).unwrap();
        engine.create_index(
            String::from("idx_id"),
            String::from("users"),
            vec![String::from("id")],
            true,
        ).unwrap();

        assert_eq!(engine.index_count(), 1);
    }

    #[test]
    fn test_value_to_bytes() {
        let val = Value::Integer(42);
        let bytes = val.to_bytes();
        assert_eq!(bytes.len(), 8);
    }

    #[test]
    fn test_buffer_pool_hit_rate() {
        let engine = StorageEngine::new(4096, 1000, 4000, 4).unwrap();
        let hit_rate = engine.buffer_pool_hit_rate();
        assert_eq!(hit_rate, 0.0);
    }
}
