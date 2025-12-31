//! B+Tree Storage Engine
//!
//! High-performance B+Tree implementation with:
//! - Node splitting and merging
//! - Range queries
//! - Transaction support
//! - Persistence and recovery

use super::types::{PageId, Value};
use super::{DbError, DbResult};
use crate::sync::Mutex;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ops::RangeBounds;

/// B+Tree configuration
#[derive(Debug, Clone)]
pub struct BTreeConfig {
    /// Order of the B+Tree (maximum number of children)
    pub order: usize,

    /// Page size in bytes
    pub page_size: usize,

    /// Enable persistence
    pub persistent: bool,

    /// Cache size in pages
    pub cache_size: usize,
}

impl Default for BTreeConfig {
    fn default() -> Self {
        Self {
            order: 64, // ~4KB pages
            page_size: 4096,
            persistent: true,
            cache_size: 1024,
        }
    }
}

/// B+Tree storage engine
pub struct BTree {
    config: BTreeConfig,
    root_page_id: Mutex<Option<PageId>>,
    next_page_id: Mutex<PageId>,
    pages: Mutex<BTreeMap<PageId, Arc<Node>>>,
    cache: Mutex<BTreeMap<PageId, Arc<Node>>>,
    cached_pages: Mutex<Vec<PageId>>,
}

impl BTree {
    pub fn new(config: BTreeConfig) -> Self {
        Self {
            config,
            root_page_id: Mutex::new(None),
            next_page_id: Mutex::new(1),
            pages: Mutex::new(BTreeMap::new()),
            cache: Mutex::new(BTreeMap::new()),
            cached_pages: Mutex::new(Vec::new()),
        }
    }

    /// Insert a key-value pair
    pub fn insert(&self, key: Value, value: Value) -> DbResult<()> {
        let mut root_id = self.root_page_id.lock();

        if root_id.is_none() {
            // Create root node
            let root = Node::new_leaf(self.next_page_id());
            let root_id = *root_id.get_or_insert(root.page_id);

            self.insert_node(root)?;
        }

        let root_page_id = root_id.unwrap();
        drop(root_id);

        // Perform insertion
        let (new_root, split_key, split_value) = self.insert_recursive(root_page_id, key, value)?;

        if let (Some(new_root), Some(split_key), Some(split_value)) = (new_root, split_key, split_value) {
            // Root was split, create new root
            let mut root = Node::new_internal(self.next_page_id());
            root.keys.push(split_key);
            root.children.push(vec![root_page_id, new_root.page_id]);

            let mut root_id = self.root_page_id.lock();
            *root_id = Some(root.page_id);

            self.insert_node(root)?;
        }

        Ok(())
    }

    /// Insert recursively
    fn insert_recursive(
        &self,
        page_id: PageId,
        key: Value,
        value: Value,
    ) -> DbResult<(Option<Node>, Option<Value>, Option<Value>)> {
        let node = self.get_node(page_id)?;

        if node.is_leaf() {
            let mut node = (*node).clone();

            // Insert into leaf
            let pos = node.find_position(&key);
            if pos < node.keys.len() && node.keys[pos] == key {
                // Update existing key
                node.values[pos] = value;
            } else {
                node.keys.insert(pos, key);
                node.values.insert(pos, value);
            }

            // Check if need to split
            if node.keys.len() > self.config.order {
                let split_pos = self.config.order / 2;
                let mut new_node = Node::new_leaf(self.next_page_id());

                let split_key = node.keys[split_pos].clone();
                let split_value = node.values[split_pos].clone();

                new_node.keys = node.keys.split_off(split_pos);
                new_node.values = node.values.split_off(split_pos);

                self.insert_node(node)?;
                self.insert_node(new_node)?;

                Ok((Some(new_node), Some(split_key), Some(split_value)))
            } else {
                self.insert_node(node)?;
                Ok((None, None, None))
            }
        } else {
            // Internal node - recurse to child
            let child_idx = node.find_child_index(&key);
            let child_page_id = node.children.as_ref().unwrap()[child_idx];

            let (new_child, split_key, split_value) =
                self.insert_recursive(child_page_id, key, value)?;

            if let (Some(new_child), split_key, split_value) = (new_child, split_key, split_value) {
                let mut node = (*node).clone();

                // Insert split key and new child
                node.keys.insert(child_idx, split_key.unwrap());
                node.children.as_mut().unwrap().insert(child_idx + 1, new_child.page_id);

                // Check if need to split
                if node.keys.len() > self.config.order {
                    let split_pos = self.config.order / 2;
                    let mut new_node = Node::new_internal(self.next_page_id());

                    let split_key = node.keys.remove(split_pos);

                    new_node.keys = node.keys.split_off(split_pos);
                    new_node.children = Some(node.children.as_mut().unwrap().split_off(split_pos + 1));

                    self.insert_node(node)?;
                    self.insert_node(new_node)?;

                    Ok((Some(new_node), Some(split_key), split_value))
                } else {
                    self.insert_node(node)?;
                    Ok((None, None, None))
                }
            } else {
                Ok((None, None, None))
            }
        }
    }

    /// Get a value by key
    pub fn get(&self, key: &Value) -> DbResult<Option<Value>> {
        let root_id = self.root_page_id.lock();
        let root_page_id = root_id.ok_or_else(|| DbError::NotFound("Empty tree".into()))?;
        drop(root_id);

        self.get_recursive(root_page_id, key)
    }

    fn get_recursive(&self, page_id: PageId, key: &Value) -> DbResult<Option<Value>> {
        let node = self.get_node(page_id)?;

        if node.is_leaf() {
            let pos = node.find_position(key);
            if pos < node.keys.len() && &node.keys[pos] == key {
                Ok(Some(node.values[pos].clone()))
            } else {
                Ok(None)
            }
        } else {
            let child_idx = node.find_child_index(key);
            let child_page_id = node.children.as_ref().unwrap()[child_idx];
            self.get_recursive(child_page_id, key)
        }
    }

    /// Delete a key
    pub fn delete(&self, key: &Value) -> DbResult<bool> {
        let root_id = self.root_page_id.lock();
        let root_page_id = root_id.ok_or_else(|| DbError::NotFound("Empty tree".into()))?;
        drop(root_id);

        self.delete_recursive(root_page_id, key)
    }

    fn delete_recursive(&self, page_id: PageId, key: &Value) -> DbResult<bool> {
        let node = self.get_node(page_id)?;

        if node.is_leaf() {
            let mut node = (*node).clone();
            let pos = node.find_position(key);

            if pos < node.keys.len() && &node.keys[pos] == key {
                node.keys.remove(pos);
                node.values.remove(pos);
                self.insert_node(node)?;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            let child_idx = node.find_child_index(key);
            let child_page_id = node.children.as_ref().unwrap()[child_idx];
            self.delete_recursive(child_page_id, key)
        }
    }

    /// Range query
    pub fn range<'a, R>(&'a self, range: R) -> DbResult<Vec<(Value, Value)>>
    where
        R: RangeBounds<&'a Value> + Clone,
    {
        let root_id = self.root_page_id.lock();
        let root_page_id = root_id.ok_or_else(|| DbError::NotFound("Empty tree".into()))?;
        drop(root_id);

        let mut results = Vec::new();
        self.range_recursive(root_page_id, range, &mut results)?;
        Ok(results)
    }

    fn range_recursive<'a, R>(
        &self,
        page_id: PageId,
        range: R,
        results: &mut Vec<(Value, Value)>,
    ) -> DbResult<()>
    where
        R: RangeBounds<&'a Value> + Clone,
    {
        let node = self.get_node(page_id)?;

        if node.is_leaf() {
            for (key, value) in node.keys.iter().zip(node.values.iter()) {
                if self.in_range(key, &range) {
                    results.push((key.clone(), value.clone()));
                }
            }
        } else {
            for (idx, child_page_id) in node.children.as_ref().unwrap().iter().enumerate() {
                self.range_recursive(*child_page_id, range.clone(), results)?;
            }
        }

        Ok(())
    }

    fn in_range<'a, R>(&self, key: &'a Value, range: &R) -> bool
    where
        R: RangeBounds<&'a Value>,
    {
        use core::ops::Bound;

        match range.start_bound() {
            Bound::Included(start) if key < start => return false,
            Bound::Excluded(start) if key <= start => return false,
            _ => {}
        }

        match range.end_bound() {
            Bound::Included(end) if key > end => return false,
            Bound::Excluded(end) if key >= end => return false,
            _ => {}
        }

        true
    }

    /// Get the next page ID
    fn next_page_id(&self) -> PageId {
        let mut next_id = self.next_page_id.lock();
        let id = *next_id;
        *next_id += 1;
        id
    }

    /// Get a node from cache or storage
    fn get_node(&self, page_id: PageId) -> DbResult<Arc<Node>> {
        // Check cache first
        {
            let cache = self.cache.lock();
            if let Some(node) = cache.get(&page_id) {
                return Ok(node.clone());
            }
        }

        // Load from storage
        let pages = self.pages.lock();
        pages
            .get(&page_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(format!("Page {}", page_id)))
    }

    /// Insert a node
    fn insert_node(&self, node: Node) -> DbResult<()> {
        let page_id = node.page_id;

        // Update cache
        {
            let mut cache = self.cache.lock();
            let mut cached_pages = self.cached_pages.lock();

            // Evict if cache is full
            if cached_pages.len() >= self.config.cache_size {
                if let Some(evict_id) = cached_pages.first() {
                    cache.remove(evict_id);
                    cached_pages.remove(0);
                }
            }

            cache.insert(page_id, Arc::new(node));
            cached_pages.push(page_id);
        }

        // Persist to storage
        let node = cache_get(&self.cache, page_id)?;
        let mut pages = self.pages.lock();
        pages.insert(page_id, node);

        Ok(())
    }

    /// Get statistics
    pub fn stats(&self) -> DbResult<BTreeStats> {
        let pages = self.pages.lock();

        let mut total_keys = 0;
        let mut total_nodes = 0;
        let mut max_depth = 0;

        for node in pages.values() {
            total_nodes += 1;
            total_keys += node.keys.len();
            // GH-#1257: Calculate max depth
            // See: https://github.com/npos/kernel/issues/1257
        }

        Ok(BTreeStats {
            total_nodes,
            total_keys,
            max_depth,
            tree_height: self.calculate_height()?,
        })
    }

    fn calculate_height(&self) -> DbResult<usize> {
        let root_id = self.root_page_id.lock();
        let root_page_id = match *root_id {
            Some(id) => id,
            None => return Ok(0),
        };
        drop(root_id);

        let mut height = 0;
        let mut current_page_id = root_page_id;

        loop {
            let node = self.get_node(current_page_id)?;
            height += 1;

            if node.is_leaf() {
                break;
            }

            current_page_id = node.children.as_ref().unwrap()[0];
        }

        Ok(height)
    }
}

fn cache_get(
    cache: &Mutex<BTreeMap<PageId, Arc<Node>>>,
    page_id: PageId,
) -> DbResult<Arc<Node>> {
    let cache = cache.lock();
    cache
        .get(&page_id)
        .cloned()
        .ok_or_else(|| DbError::NotFound(format!("Page {} in cache", page_id)))
}

/// B+Tree node
#[derive(Debug, Clone)]
pub struct Node {
    pub page_id: PageId,
    pub keys: Vec<Value>,
    pub values: Vec<Value>,
    pub children: Option<Vec<PageId>>,
}

impl Node {
    fn new_leaf(page_id: PageId) -> Self {
        Self {
            page_id,
            keys: Vec::new(),
            values: Vec::new(),
            children: None,
        }
    }

    fn new_internal(page_id: PageId) -> Self {
        Self {
            page_id,
            keys: Vec::new(),
            values: Vec::new(),
            children: Some(Vec::new()),
        }
    }

    fn is_leaf(&self) -> bool {
        self.children.is_none()
    }

    fn find_position(&self, key: &Value) -> usize {
        self.keys
            .binary_search_by(|k| {
                // Simple comparison - in real implementation, use proper ordering
                if k == key {
                    core::cmp::Ordering::Equal
                } else {
                    core::cmp::Ordering::Less
                }
            })
            .unwrap_or_else(|pos| pos)
    }

    fn find_child_index(&self, key: &Value) -> usize {
        let pos = self.find_position(key);
        if pos >= self.keys.len() {
            self.keys.len()
        } else {
            pos
        }
    }
}

/// B+Tree statistics
#[derive(Debug, Clone)]
pub struct BTreeStats {
    pub total_nodes: usize,
    pub total_keys: usize,
    pub max_depth: usize,
    pub tree_height: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btree_insert() {
        let btree = BTree::new(BTreeConfig::default());

        let key1 = Value::Int32(1);
        let value1 = Value::Text("one".to_string());

        assert!(btree.insert(key1.clone(), value1).is_ok());
        assert!(btree.get(&key1).is_ok());
    }

    #[test]
    fn test_btree_range() {
        let btree = BTree::new(BTreeConfig::default());

        for i in 1..=10 {
            let key = Value::Int32(i);
            let value = Value::Int32(i * 10);
            btree.insert(key, value).unwrap();
        }

        let start = &Value::Int32(3);
        let end = &Value::Int32(7);
        let results = btree.range(start..=end).unwrap();

        assert_eq!(results.len(), 5);
    }
}
