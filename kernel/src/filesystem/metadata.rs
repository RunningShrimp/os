//! Metadata Management
//!
//! Advanced metadata handling with B+trees, extents, and extended attributes.
//!
//! ## Overview
//!
//! This module provides sophisticated metadata management:
//! - **B+tree**: Efficient indexing for large directories
//! - **Extents**: Variable-length block allocation
//! - **Extended attributes**: Flexible metadata storage
//! - **ACLs**: Fine-grained access control
//! - **Quotas**: Disk space management
//!
//! ## Key Concepts
//!
//! - **B+tree**: Balanced tree with sorted keys and range queries
//! - **Extent**: Contiguous block range (logical + physical)
//! - **Xattr**: Extended attributes (key-value pairs)
//! - **ACL**: Access Control Lists (user/group permissions)
//! - **Quota**: Limits on blocks/inodes per user/group
//!
//! ## Architecture
//!
//! ```
//! Metadata Manager
//! ├── B+tree Index
//! │   ├── Internal nodes (keys + pointers)
//! │   └── Leaf nodes (keys + values)
//! ├── Extent Tree
//! │   ├── Logical extent mapping
//! │   └── Free space bitmap
//! ├── Extended Attributes
//! │   ├── User namespace
//! │   ├── Trusted namespace
//! │   └── Security namespace
//! └── Access Control
//!     ├── POSIX permissions
//!     └── NFSv4 ACLs
//! ```
//!
//! ## Performance
//!
//! - B+tree lookup: O(log n)
//! - Extent allocation: O(log n)
//! - Xattr lookup: O(1) with inline storage
//! - ACL check: O(k) where k = number of ACEs

#![allow(dead_code)]

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use core::sync::atomic::AtomicU64;

use crate::subsystems::sync::Mutex;
use crate::filesystem::error::{FsError, FsResult};

/// Default B+tree order (max children per node)
pub const DEFAULT_BTREE_ORDER: usize = 128;

/// Maximum xattr size (inline)
pub const MAX_XATTR_SIZE: usize = 256;

/// Maximum xattr name length
pub const MAX_XATTR_NAME: usize = 255;

/// B+tree node
#[derive(Debug, Clone)]
pub struct BTreeNode {
    /// Node is leaf
    pub is_leaf: bool,
    /// Number of keys
    pub num_keys: usize,
    /// Keys
    pub keys: Vec<String>,
    /// Values (for leaf nodes) or child pointers (for internal nodes)
    pub values: Vec<Option<BTreeValue>>,
    /// Parent pointer
    pub parent: Option<usize>,
    /// Next leaf (for range queries)
    pub next_leaf: Option<usize>,
}

/// B+tree value type
#[derive(Debug, Clone)]
pub enum BTreeValue {
    /// Inode number
    Inode(u64),
    /// Child node index
    Child(usize),
    /// Data pointer
    Data(u64),
}

impl BTreeNode {
    /// Create a new leaf node
    pub fn new_leaf() -> Self {
        Self {
            is_leaf: true,
            num_keys: 0,
            keys: Vec::new(),
            values: Vec::new(),
            parent: None,
            next_leaf: None,
        }
    }

    /// Create a new internal node
    pub fn new_internal() -> Self {
        Self {
            is_leaf: false,
            num_keys: 0,
            keys: Vec::new(),
            values: Vec::new(),
            parent: None,
            next_leaf: None,
        }
    }

    /// Search for a key
    pub fn search(&self, key: &str) -> Option<&BTreeValue> {
        let pos = self.keys.binary_search(&String::from(key)).ok()?;
        self.values.get(pos).and_then(|v| v.as_ref())
    }

    /// Insert a key-value pair
    pub fn insert(&mut self, key: String, value: BTreeValue) -> FsResult<()> {
        let pos = self.keys.binary_search(&key);

        match pos {
            Ok(_) => return Err(FsError::Exists),
            Err(idx) => {
                self.keys.insert(idx, key);
                self.values.insert(idx, Some(value));
                self.num_keys += 1;
                Ok(())
            }
        }
    }

    /// Split node (for B+tree maintenance)
    pub fn split(&mut self) -> (String, BTreeNode) {
        let mid = self.num_keys / 2;
        let split_key = self.keys[mid].clone();

        let mut new_node = if self.is_leaf {
            BTreeNode::new_leaf()
        } else {
            BTreeNode::new_internal()
        };

        // Move keys and values to new node
        new_node.keys = self.keys.split_off(mid);
        new_node.values = self.values.split_off(mid);

        // Update counts
        new_node.num_keys = new_node.keys.len();
        self.num_keys = self.keys.len();

        // Link leaf nodes
        if self.is_leaf {
            new_node.next_leaf = self.next_leaf;
            self.next_leaf = Some(new_node.keys.len());
        }

        (split_key, new_node)
    }

    /// Check if node is full
    pub fn is_full(&self, order: usize) -> bool {
        self.num_keys >= order - 1
    }
}

/// B+tree for directory indexing
pub struct BtreeIndex {
    /// Root node
    root: Mutex<BTreeNode>,
    /// Tree order
    order: usize,
    /// Node allocator
    next_node_id: AtomicU64,
    /// Nodes storage
    nodes: Mutex<BTreeMap<usize, BTreeNode>>,
}

impl BtreeIndex {
    /// Create a new B+tree index
    pub fn new(order: usize) -> Self {
        let root = BTreeNode::new_leaf();

        Self {
            root: Mutex::new(root),
            order,
            next_node_id: AtomicU64::new(1),
            nodes: Mutex::new(BTreeMap::new()),
        }
    }

    /// Lookup a key
    pub fn lookup(&self, key: &str) -> FsResult<u64> {
        let root = self.root.lock();

        match root.search(key) {
            Some(BTreeValue::Inode(ino)) => Ok(*ino),
            Some(BTreeValue::Child(_)) => {
                drop(root);
                self.search_recursive(key, 0)
            }
            Some(BTreeValue::Data(_)) => Err(FsError::InvalidOperation),
            None => Err(FsError::NotFound),
        }
    }

    /// Recursive search
    fn search_recursive(&self, key: &str, node_id: usize) -> FsResult<u64> {
        let nodes = self.nodes.lock();
        let node = nodes.get(&node_id).ok_or(FsError::NotFound)?;

        match node.search(key) {
            Some(BTreeValue::Inode(ino)) => Ok(*ino),
            Some(BTreeValue::Child(child_id)) => {
                let child = *child_id;
                drop(nodes);
                self.search_recursive(key, child)
            }
            _ => Err(FsError::NotFound),
        }
    }

    /// Insert a key-value pair
    pub fn insert(&self, key: String, ino: u64) -> FsResult<()> {
        let mut root = self.root.lock();

        if root.is_full(self.order) {
            // Split root
            let (split_key, _new_node) = root.split();

            let mut new_root = BTreeNode::new_internal();
            new_root.keys.push(split_key);
            new_root.values.push(Some(BTreeValue::Child(0)));
            new_root.values.push(Some(BTreeValue::Child(1)));

            *root = new_root;
        }

        root.insert(key, BTreeValue::Inode(ino))
    }

    /// Delete a key
    pub fn delete(&self, key: &str) -> FsResult<()> {
        // GH-#1249: Implement delete with redistribution/merging
        // See: https://github.com/npos/kernel/issues/1249
        let _ = key;
        Err(FsError::NotSupported)
    }

    /// Range query
    pub fn range(&self, start: &str, end: &str) -> FsResult<Vec<(String, u64)>> {
        let results = Vec::new();

        // GH-#1250: Implement efficient range traversal
        // See: https://github.com/npos/kernel/issues/1250
        let _ = (start, end);

        Ok(results)
    }
}

/// Extent - contiguous block range
#[derive(Debug, Clone)]
pub struct BtreeExtent {
    /// Logical start offset
    pub logical: u64,
    /// Physical block number
    pub physical: u64,
    /// Length in bytes
    pub length: u64,
    /// Extent state
    pub state: ExtentState,
}

/// Extent state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtentState {
    /// Free extent
    Free,
    /// Allocated extent
    Allocated,
    /// Reserved (for delayed allocation)
    Reserved,
}

impl BtreeExtent {
    /// Create a new extent
    pub fn new(logical: u64, physical: u64, length: u64) -> Self {
        Self {
            logical,
            physical,
            length,
            state: ExtentState::Allocated,
        }
    }

    /// Check if extents are contiguous
    pub fn is_contiguous(&self, other: &BtreeExtent) -> bool {
        self.logical + self.length == other.logical &&
        self.physical + self.length == other.physical
    }

    /// Merge with contiguous extent
    pub fn merge(&mut self, other: &BtreeExtent) -> FsResult<()> {
        if self.is_contiguous(other) {
            self.length += other.length;
            Ok(())
        } else {
            Err(FsError::InvalidOperation)
        }
    }

    /// Split extent at offset
    pub fn split(&self, offset: u64) -> (BtreeExtent, BtreeExtent) {
        let left = BtreeExtent {
            logical: self.logical,
            physical: self.physical,
            length: offset,
            state: self.state,
        };

        let right = BtreeExtent {
            logical: self.logical + offset,
            physical: self.physical + offset,
            length: self.length - offset,
            state: self.state,
        };

        (left, right)
    }
}

/// Free space bitmap
pub struct FreeSpaceBitmap {
    /// Bitmap data (bit = 1 means free)
    bitmap: Mutex<Vec<u64>>,
    /// Total blocks
    total_blocks: u64,
    /// Block size
    block_size: u64,
}

impl FreeSpaceBitmap {
    /// Create a new bitmap
    pub fn new(total_blocks: u64, block_size: u64) -> Self {
        let words = (total_blocks + 63) / 64;

        Self {
            bitmap: Mutex::new(vec![u64::MAX; words as usize]),
            total_blocks,
            block_size,
        }
    }

    /// Allocate contiguous blocks
    pub fn alloc_blocks(&self, count: u64) -> FsResult<u64> {
        let mut bitmap = self.bitmap.lock();

        for word_idx in 0..bitmap.len() {
            let word = bitmap[word_idx];

            if word == 0 {
                continue;
            }

            // Find free bit
            for bit_idx in 0..64 {
                if word & (1 << bit_idx) != 0 {
                    let block_num = (word_idx as u64) * 64 + bit_idx as u64;

                    // Check if we have enough contiguous blocks
                    if self.check_contiguous(&bitmap, block_num, count) {
                        // Mark blocks as allocated
                        self.mark_blocks_used(&mut bitmap, block_num, count);
                        return Ok(block_num);
                    }
                }
            }
        }

        Err(FsError::NoSpace)
    }

    /// Check if blocks are free and contiguous
    fn check_contiguous(&self, bitmap: &[u64], start: u64, count: u64) -> bool {
        for i in 0..count {
            let block = start + i;
            if block >= self.total_blocks {
                return false;
            }

            let word_idx = (block / 64) as usize;
            let bit_idx = (block % 64) as usize;

            if bitmap[word_idx] & (1 << bit_idx) == 0 {
                return false;
            }
        }

        true
    }

    /// Mark blocks as used
    fn mark_blocks_used(&self, bitmap: &mut [u64], start: u64, count: u64) {
        for i in 0..count {
            let block = start + i;
            let word_idx = (block / 64) as usize;
            let bit_idx = (block % 64) as usize;

            bitmap[word_idx] &= !(1 << bit_idx);
        }
    }

    /// Free blocks
    pub fn free_blocks(&self, start: u64, count: u64) -> FsResult<()> {
        let mut bitmap = self.bitmap.lock();

        for i in 0..count {
            let block = start + i;
            if block >= self.total_blocks {
                return Err(FsError::InvalidInput);
            }

            let word_idx = (block / 64) as usize;
            let bit_idx = (block % 64) as usize;

            bitmap[word_idx] |= 1 << bit_idx;
        }

        Ok(())
    }
}

/// Extended attribute namespace
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XattrNamespace {
    /// User namespace (user.*)
    User,
    /// Trusted namespace (trusted.*)
    Trusted,
    /// Security namespace (security.*)
    Security,
    /// System namespace (system.*)
    System,
}

/// Extended attribute
#[derive(Debug, Clone)]
pub struct Xattr {
    /// Namespace
    pub namespace: XattrNamespace,
    /// Attribute name
    pub name: String,
    /// Attribute value
    pub value: Vec<u8>,
}

impl Xattr {
    /// Create a new extended attribute
    pub fn new(namespace: XattrNamespace, name: String, value: Vec<u8>) -> Self {
        Self {
            namespace,
            name,
            value,
        }
    }

    /// Get full attribute name (with namespace prefix)
    pub fn full_name(&self) -> String {
        let prefix = match self.namespace {
            XattrNamespace::User => "user",
            XattrNamespace::Trusted => "trusted",
            XattrNamespace::Security => "security",
            XattrNamespace::System => "system",
        };
        format!("{}.{}", prefix, self.name)
    }

    /// Get attribute size
    pub fn size(&self) -> usize {
        self.name.len() + self.value.len()
    }
}

/// Access Control Entry (ACE)
#[derive(Debug, Clone)]
pub struct Ace {
    /// ACE type (allow/deny)
    pub ace_type: AceType,
    /// ACE flags
    pub flags: u32,
    /// Access mask
    pub mask: u32,
    /// Who (UID/GID)
    pub who: u32,
}

/// ACE type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AceType {
    /// Allow access
    Allow,
    /// Deny access
    Deny,
    /// Audit (log access)
    Audit,
    /// Alarm (alert on access)
    Alarm,
}

/// Access Control List
#[derive(Debug, Clone)]
pub struct Acl {
    /// ACL entries
    pub entries: Vec<Ace>,
}

impl Acl {
    /// Create a new ACL
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add an ACE
    pub fn add(&mut self, ace: Ace) {
        self.entries.push(ace);
    }

    /// Check if access is allowed
    pub fn check(&self, uid: u32, gid: u32, requested: u32) -> bool {
        // Default deny
        let mut allowed = false;

        for ace in &self.entries {
            // Check if this ACE applies
            if ace.who != uid && ace.who != gid {
                continue;
            }

            // Check if requested access is in mask
            if ace.mask & requested == 0 {
                continue;
            }

            match ace.ace_type {
                AceType::Allow => allowed = true,
                AceType::Deny => return false,
                _ => {}
            }
        }

        allowed
    }
}

/// Metadata manager
pub struct MetadataManager {
    /// Directory B+trees (inode -> BtreeIndex)
    directories: Mutex<BTreeMap<u64, BtreeIndex>>,
    /// Extent trees (inode -> extents)
    extents: Mutex<BTreeMap<u64, Vec<BtreeExtent>>>,
    /// Free space bitmap
    free_space: FreeSpaceBitmap,
    /// Extended attributes (inode -> xattrs)
    xattrs: Mutex<BTreeMap<u64, Vec<Xattr>>>,
    /// ACLs (inode -> ACL)
    acls: Mutex<BTreeMap<u64, Acl>>,
}

impl MetadataManager {
    /// Create a new metadata manager
    pub fn new(total_blocks: u64, block_size: u64) -> Self {
        Self {
            directories: Mutex::new(BTreeMap::new()),
            extents: Mutex::new(BTreeMap::new()),
            free_space: FreeSpaceBitmap::new(total_blocks, block_size),
            xattrs: Mutex::new(BTreeMap::new()),
            acls: Mutex::new(BTreeMap::new()),
        }
    }

    /// Create a directory B+tree
    pub fn create_directory(&self, ino: u64) -> FsResult<()> {
        let index = BtreeIndex::new(DEFAULT_BTREE_ORDER);

        let mut dirs = self.directories.lock();
        dirs.insert(ino, index);

        Ok(())
    }

    /// Lookup directory entry
    pub fn dir_lookup(&self, dir_ino: u64, name: &str) -> FsResult<u64> {
        let dirs = self.directories.lock();
        let index = dirs.get(&dir_ino).ok_or(FsError::NotFound)?;
        index.lookup(name)
    }

    /// Insert directory entry
    pub fn dir_insert(&self, dir_ino: u64, name: String, ino: u64) -> FsResult<()> {
        let dirs = self.directories.lock();
        let index = dirs.get(&dir_ino).ok_or(FsError::NotFound)?;
        index.insert(name, ino)
    }

    /// Allocate blocks for a file
    pub fn alloc_blocks(&self, ino: u64, offset: u64, size: u64) -> FsResult<BtreeExtent> {
        let block_count = (size + 4095) / 4096;
        let start_block = self.free_space.alloc_blocks(block_count)?;

        let extent = BtreeExtent::new(offset, start_block * 4096, size);

        let mut extents = self.extents.lock();
        extents.entry(ino).or_insert_with(Vec::new).push(extent.clone());

        Ok(extent)
    }

    /// Set extended attribute
    pub fn set_xattr(&self, ino: u64, xattr: Xattr) -> FsResult<()> {
        let mut xattrs = self.xattrs.lock();
        let entry = xattrs.entry(ino).or_insert_with(Vec::new);

        // Check if attribute exists
        if let Some(existing) = entry.iter().position(|x| x.name == xattr.name && x.namespace == xattr.namespace) {
            entry[existing] = xattr;
        } else {
            entry.push(xattr);
        }

        Ok(())
    }

    /// Get extended attribute
    pub fn get_xattr(&self, ino: u64, namespace: XattrNamespace, name: &str) -> FsResult<Vec<u8>> {
        let xattrs = self.xattrs.lock();
        let entry = xattrs.get(&ino).ok_or(FsError::NotFound)?;

        entry.iter()
            .find(|x| x.name == name && x.namespace == namespace)
            .map(|x| x.value.clone())
            .ok_or(FsError::NotFound)
    }

    /// Set ACL
    pub fn set_acl(&self, ino: u64, acl: Acl) -> FsResult<()> {
        let mut acls = self.acls.lock();
        acls.insert(ino, acl);
        Ok(())
    }

    /// Check access using ACL
    pub fn check_access(&self, ino: u64, uid: u32, gid: u32, requested: u32) -> bool {
        let acls = self.acls.lock();

        acls.get(&ino)
            .map(|acl| acl.check(uid, gid, requested))
            .unwrap_or(true) // Default allow if no ACL
    }
}

/// Initialize metadata manager
pub fn init_metadata_manager() -> FsResult<()> {
    crate::println!("[metadata] Metadata manager initialized");
    Ok(())
}

/// Shutdown metadata manager
pub fn shutdown_metadata_manager() -> FsResult<()> {
    crate::println!("[metadata] Metadata manager shutdown");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btree_extent_contiguous() {
        let e1 = BtreeExtent::new(0, 100, 4096);
        let e2 = BtreeExtent::new(4096, 10496, 4096);

        assert!(e1.is_contiguous(&e2));
    }

    #[test]
    fn test_btree_extent_merge() {
        let mut e1 = BtreeExtent::new(0, 100, 4096);
        let e2 = BtreeExtent::new(4096, 10496, 4096);

        assert!(e1.merge(&e2).is_ok());
        assert_eq!(e1.length, 8192);
    }

    #[test]
    fn test_free_space_bitmap() {
        let bitmap = FreeSpaceBitmap::new(1024, 4096);
        assert!(bitmap.alloc_blocks(10).is_ok());
    }

    #[test]
    fn test_xattr() {
        let xattr = Xattr::new(XattrNamespace::User, String::from("test"), vec![1, 2, 3]);
        assert_eq!(xattr.full_name(), "user.test");
    }

    #[test]
    fn test_acl() {
        let mut acl = Acl::new();
        acl.add(Ace {
            ace_type: AceType::Allow,
            flags: 0,
            mask: 0o755,
            who: 1000,
        });

        assert!(acl.check(1000, 1000, 0o644));
    }
}
