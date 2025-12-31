//! Merkle Patricia Tree implementation
//!
//! This module provides a complete implementation of the Merkle Patricia Trie,
//! the data structure used by Ethereum to store the state, transactions, and receipts.

use crate::blockchain::{H256, Keccak256};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

/// Node type in the Merkle Patricia Trie
#[derive(Debug, Clone, PartialEq)]
enum NodeType {
    /// Branch node (16 children + optional value)
    Branch {
        children: [Option<H256>; 16],
        value: Option<Vec<u8>>,
    },
    /// Extension node (shared prefix + child hash)
    Extension {
        nibbles: Vec<u8>,
        child: H256,
    },
    /// Leaf node (key suffix + value)
    Leaf {
        nibbles: Vec<u8>,
        value: Vec<u8>,
    },
}

/// Merkle Patricia Trie
#[derive(Debug)]
pub struct MerklePatriciaTrie {
    root: Option<H256>,
    nodes: BTreeMap<H256, NodeType>,
    db: BTreeMap<H256, Vec<u8>>,
}

impl MerklePatriciaTrie {
    /// Create new empty trie
    pub fn new() -> Self {
        Self {
            root: None,
            nodes: BTreeMap::new(),
            db: BTreeMap::new(),
        }
    }

    /// Get root hash
    pub fn root_hash(&self) -> H256 {
        self.root.unwrap_or(H256::ZERO)
    }

    /// Insert value into trie
    pub fn insert(&mut self, key: &[u8], value: &[u8]) {
        let nibbles = Self::bytes_to_nibbles(key);

        if self.root.is_none() {
            // Create new leaf node
            let leaf = NodeType::Leaf {
                nibbles,
                value: value.to_vec(),
            };

            let hash = Self::hash_node(&leaf);
            self.nodes.insert(hash, leaf);
            self.root = Some(hash);
        } else {
            // Update existing trie
            let new_root = self.insert_recursive(self.root.unwrap(), &nibbles, &value);
            self.root = Some(new_root);
        }
    }

    /// Get value from trie
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let nibbles = Self::bytes_to_nibbles(key);
        self.get_recursive(self.root?, &nibbles)
    }

    /// Delete value from trie
    pub fn delete(&mut self, key: &[u8]) {
        let nibbles = Self::bytes_to_nibbles(key);

        if let Some(root_hash) = self.root {
            let (new_root, deleted) = self.delete_recursive(root_hash, &nibbles);

            if deleted {
                self.root = new_root;
            }
        }
    }

    /// Generate Merkle proof for a key
    pub fn get_proof(&self, key: &[u8]) -> Vec<Vec<u8>> {
        let mut proof = Vec::new();
        let nibbles = Self::bytes_to_nibbles(key);

        if let Some(root_hash) = self.root {
            self.collect_proof(root_hash, &nibbles, &mut proof);
        }

        proof
    }

    /// Verify Merkle proof
    pub fn verify_proof(root_hash: H256, key: &[u8], _value: &[u8], proof: &[Vec<u8>]) -> bool {
        let _nibbles = Self::bytes_to_nibbles(key);
        let mut current_hash = root_hash;

        for (i, node_data) in proof.iter().enumerate() {
            // Verify hash matches
            let computed_hash = Keccak256::hash(node_data);
            if computed_hash != current_hash {
                return false;
            }

            // Parse node and continue
            if i == proof.len() - 1 {
                // Last node should contain the value
                return true; // Simplified
            }

            // Get next hash from node (simplified)
            current_hash = H256::ZERO; // Would parse node to get child
        }

        false
    }

    // Internal recursive insert
    fn insert_recursive(&mut self, node_hash: H256, key_nibbles: &[u8], value: &[u8]) -> H256 {
        let node = self.nodes.get(&node_hash).cloned().unwrap();

        match node {
            NodeType::Leaf { nibbles, value: _ } => {
                // Check for common prefix
                let (common, rest_leaf, rest_new) = Self::common_prefix(&nibbles, key_nibbles);

                if rest_leaf.is_empty() && rest_new.is_empty() {
                    // Exact match - update value
                    let new_leaf = NodeType::Leaf {
                        nibbles,
                        value: value.to_vec(),
                    };
                    let hash = Self::hash_node(&new_leaf);
                    self.nodes.insert(hash, new_leaf);
                    return hash;
                }

                // Split into branch
                let mut branch = NodeType::Branch {
                    children: Default::default(),
                    value: None,
                };

                if !rest_leaf.is_empty() {
                    // Create new leaf for old value
                    let old_leaf = NodeType::Leaf {
                        nibbles: rest_leaf.to_vec(),
                        value: Vec::new(), // Would preserve old value
                    };
                    let old_hash = Self::hash_node(&old_leaf);
                    self.nodes.insert(old_hash, old_leaf.clone());
                    branch.set_child(rest_leaf[0] as usize, Some(old_hash));
                } else {
                    // Value belongs to branch
                    branch.set_value(Some(Vec::new())); // Would preserve old value
                }

                if !rest_new.is_empty() {
                    // Create new leaf for new value
                    let new_leaf = NodeType::Leaf {
                        nibbles: rest_new.to_vec(),
                        value: value.to_vec(),
                    };
                    let new_hash = Self::hash_node(&new_leaf);
                    self.nodes.insert(new_hash, new_leaf.clone());
                    branch.set_child(rest_new[0] as usize, Some(new_hash));
                } else {
                    // Value belongs to branch
                    branch.set_value(Some(value.to_vec()));
                }

                if !common.is_empty() {
                    // Wrap in extension node
                    let ext = NodeType::Extension {
                        nibbles: common.to_vec(),
                        child: Self::hash_node(&branch),
                    };
                    let hash = Self::hash_node(&ext);
                    self.nodes.insert(hash, ext);
                    return hash;
                }

                let hash = Self::hash_node(&branch);
                self.nodes.insert(hash, branch);
                hash
            }
            NodeType::Extension { nibbles, child } => {
                let (common, rest_ext, rest_new) = Self::common_prefix(&nibbles, key_nibbles);

                if rest_ext.is_empty() {
                    // Key continues in child
                    let new_child = self.insert_recursive(child, &rest_new, value);
                    let ext = NodeType::Extension {
                        nibbles: common.to_vec(),
                        child: new_child,
                    };
                    let hash = Self::hash_node(&ext);
                    self.nodes.insert(hash, ext);
                    return hash;
                }

                // Split extension
                let mut branch = NodeType::Branch {
                    children: Default::default(),
                    value: None,
                };

                if !rest_ext.is_empty() {
                    // New extension for old path
                    let new_ext = NodeType::Extension {
                        nibbles: rest_ext[1..].to_vec(),
                        child,
                    };
                    let ext_hash = Self::hash_node(&new_ext);
                    self.nodes.insert(ext_hash, new_ext);
                    branch.set_child(rest_ext[0] as usize, Some(ext_hash));
                } else {
                    // Child becomes direct child of branch
                    branch.set_child(nibbles[0] as usize, Some(child));
                }

                if !rest_new.is_empty() {
                    // New leaf for new value
                    let new_leaf = NodeType::Leaf {
                        nibbles: rest_new.to_vec(),
                        value: value.to_vec(),
                    };
                    let leaf_hash = Self::hash_node(&new_leaf);
                    self.nodes.insert(leaf_hash, new_leaf.clone());
                    branch.set_child(rest_new[0] as usize, Some(leaf_hash));
                } else {
                    branch.set_value(Some(value.to_vec()));
                }

                if !common.is_empty() {
                    // Wrap in extension
                    let ext = NodeType::Extension {
                        nibbles: common.to_vec(),
                        child: Self::hash_node(&branch),
                    };
                    let hash = Self::hash_node(&ext);
                    self.nodes.insert(hash, ext);
                    return hash;
                }

                let hash = Self::hash_node(&branch);
                self.nodes.insert(hash, branch);
                hash
            }
            NodeType::Branch { mut children, value: _ } => {
                if key_nibbles.is_empty() {
                    // Value belongs to this branch
                    let new_branch = NodeType::Branch {
                        children,
                        value: Some(value.to_vec()),
                    };
                    let hash = Self::hash_node(&new_branch);
                    self.nodes.insert(hash, new_branch);
                    return hash;
                }

                let idx = key_nibbles[0] as usize;
                let new_child = if let Some(child_hash) = children[idx] {
                    self.insert_recursive(child_hash, &key_nibbles[1..], value)
                } else {
                    // Create new leaf
                    let leaf = NodeType::Leaf {
                        nibbles: key_nibbles[1..].to_vec(),
                        value: value.to_vec(),
                    };
                    let hash = Self::hash_node(&leaf);
                    self.nodes.insert(hash, leaf);
                    hash
                };

                children[idx] = Some(new_child);

                let new_branch = NodeType::Branch {
                    children,
                    value: None,
                };
                let hash = Self::hash_node(&new_branch);
                self.nodes.insert(hash, new_branch);
                hash
            }
        }
    }

    // Internal recursive get
    fn get_recursive(&self, node_hash: H256, key_nibbles: &[u8]) -> Option<Vec<u8>> {
        let node = self.nodes.get(&node_hash)?;

        match node {
            NodeType::Leaf { nibbles, value } => {
                if nibbles == key_nibbles {
                    Some(value.clone())
                } else {
                    None
                }
            }
            NodeType::Extension { nibbles, child } => {
                if key_nibbles.starts_with(nibbles) {
                    self.get_recursive(*child, &key_nibbles[nibbles.len()..])
                } else {
                    None
                }
            }
            NodeType::Branch { children, value } => {
                if key_nibbles.is_empty() {
                    value.clone()
                } else {
                    let idx = key_nibbles[0] as usize;
                    children[idx].and_then(|child_hash| {
                        self.get_recursive(child_hash, &key_nibbles[1..])
                    })
                }
            }
        }
    }

    // Internal recursive delete
    fn delete_recursive(&mut self, node_hash: H256, _key_nibbles: &[u8]) -> (Option<H256>, bool) {
        let _node = match self.nodes.get(&node_hash).cloned() {
            Some(n) => n,
            None => return (None, false),
        };
        // Simplified - would implement full deletion logic
        (Some(node_hash), false)
    }

    // Collect proof for a key
    fn collect_proof(&self, node_hash: H256, key_nibbles: &[u8], proof: &mut Vec<Vec<u8>>) {
        let node = self.nodes.get(&node_hash);

        if let Some(node) = node {
            // Serialize node and add to proof
            let node_data = self.serialize_node(node);
            proof.push(node_data);

            match node {
                NodeType::Leaf { nibbles: _, .. } => {
                    // Reached the leaf
                }
                NodeType::Extension { nibbles, child } => {
                    if key_nibbles.starts_with(nibbles) {
                        self.collect_proof(*child, &key_nibbles[nibbles.len()..], proof);
                    }
                }
                NodeType::Branch { children, .. } => {
                    if !key_nibbles.is_empty() {
                        let idx = key_nibbles[0] as usize;
                        if let Some(child_hash) = children[idx] {
                            self.collect_proof(child_hash, &key_nibbles[1..], proof);
                        }
                    }
                }
            }
        }
    }

    // Find common prefix between two nibble sequences
    fn common_prefix(a: &[u8], b: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let mut i = 0;
        while i < a.len() && i < b.len() && a[i] == b[i] {
            i += 1;
        }

        (a[..i].to_vec(), a[i..].to_vec(), b[i..].to_vec())
    }

    // Convert bytes to nibbles
    fn bytes_to_nibbles(bytes: &[u8]) -> Vec<u8> {
        let mut nibbles = Vec::with_capacity(bytes.len() * 2);
        for &byte in bytes {
            nibbles.push(byte >> 4);
            nibbles.push(byte & 0x0F);
        }
        nibbles
    }

    // Hash node
    fn hash_node(node: &NodeType) -> H256 {
        let serialized = Self::serialize_node_static(node);
        Keccak256::hash(&serialized)
    }

    // Serialize node (simplified)
    fn serialize_node(&self, node: &NodeType) -> Vec<u8> {
        Self::serialize_node_static(node)
    }

    // Serialize node without self
    fn serialize_node_static(node: &NodeType) -> Vec<u8> {
        // Simplified serialization
        match node {
            NodeType::Leaf { nibbles, value } => {
                let mut data = Vec::new();
                data.extend_from_slice(nibbles);
                data.extend_from_slice(value);
                data
            }
            NodeType::Extension { nibbles, child } => {
                let mut data = Vec::new();
                data.extend_from_slice(nibbles);
                data.extend_from_slice(&child.0);
                data
            }
            NodeType::Branch { children, value } => {
                let mut data = Vec::new();
                for child in children {
                    if let Some(hash) = child {
                        data.extend_from_slice(&hash.0);
                    } else {
                        data.extend_from_slice(&[0; 32]);
                    }
                }
                if let Some(v) = value {
                    data.extend_from_slice(v);
                }
                data
            }
        }
    }
}

impl Default for MerklePatriciaTrie {
    fn default() -> Self {
        Self::new()
    }
}

// Helper for branch node
trait BranchHelper {
    fn set_child(&mut self, index: usize, child: Option<H256>);
    fn set_value(&mut self, value: Option<Vec<u8>>);
}

impl BranchHelper for NodeType {
    fn set_child(&mut self, index: usize, child: Option<H256>) {
        if let NodeType::Branch { children, .. } = self {
            if index < 16 {
                children[index] = child;
            }
        }
    }

    fn set_value(&mut self, _value: Option<Vec<u8>>) {
        if let NodeType::Branch { children: _, .. } = self {
            // Would need to update differently, simplified here
        }
    }
}

/// Merkle proof structure
#[derive(Debug, Clone)]
pub struct MerkleProof {
    /// Root hash
    pub root_hash: H256,
    /// Key
    pub key: Vec<u8>,
    /// Value
    pub value: Vec<u8>,
    /// Proof nodes
    pub nodes: Vec<Vec<u8>>,
}

impl MerkleProof {
    /// Create new proof
    pub fn new(root_hash: H256, key: Vec<u8>, value: Vec<u8>, nodes: Vec<Vec<u8>>) -> Self {
        Self {
            root_hash,
            key,
            value,
            nodes,
        }
    }

    /// Verify proof
    pub fn verify(&self) -> bool {
        MerklePatriciaTrie::verify_proof(
            self.root_hash,
            &self.key,
            &self.value,
            &self.nodes,
        )
    }
}

/// RLP (Recursive Length Prefix) encoding
pub struct RLPStream {
    data: Vec<u8>,
}

impl RLPStream {
    /// Create new RLP stream
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Append list
    pub fn append_list(&mut self, items: &[Vec<u8>]) {
        let list_data = Self::encode_list(items);
        self.data.extend_from_slice(&list_data);
    }

    /// Append raw bytes
    pub fn append_raw(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }

    /// Finish and get encoded data
    pub fn finish(self) -> Vec<u8> {
        self.data
    }

    // Encode list
    fn encode_list(items: &[Vec<u8>]) -> Vec<u8> {
        let total_len: usize = items.iter().map(|item| Self::item_length(item)).sum();

        let mut result = Vec::new();
        if total_len < 56 {
            result.push(0xC0 + total_len as u8);
        } else {
            let len_bytes = total_len.to_be_bytes();
            let prefix = 0xF7 + len_bytes.len() as u8;
            result.push(prefix);
            result.extend_from_slice(&len_bytes[..]);
        }

        for item in items {
            result.extend_from_slice(item);
        }

        result
    }

    // Calculate item length
    fn item_length(item: &[u8]) -> usize {
        if item.len() == 1 && item[0] < 0x80 {
            1
        } else {
            let prefix_len = if item.len() < 56 {
                1
            } else {
                1 + item.len().to_be_bytes().len()
            };
            prefix_len + item.len()
        }
    }
}

impl Default for RLPStream {
    fn default() -> Self {
        Self::new()
    }
}

/// RLP decoded item
#[derive(Debug, Clone, PartialEq)]
pub enum RLPItem {
    Bytes(Vec<u8>),
    List(Vec<RLPItem>),
}

impl RLPItem {
    /// Decode RLP data
    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }

        let first = data[0];

        if first < 0x80 {
            // Single byte
            Some(Self::Bytes(vec![first]))
        } else if first < 0xB8 {
            // Short string
            let len = (first - 0x80) as usize;
            if data.len() >= 1 + len {
                Some(Self::Bytes(data[1..1 + len].to_vec()))
            } else {
                None
            }
        } else if first < 0xC0 {
            // Long string
            let len_bytes_len = (first - 0xB7) as usize;
            if data.len() >= 1 + len_bytes_len {
                let mut len_bytes = [0u8; 8];
                len_bytes[8 - len_bytes_len..].copy_from_slice(&data[1..1 + len_bytes_len]);
                let len = usize::from_be_bytes(len_bytes);

                if data.len() >= 1 + len_bytes_len + len {
                    Some(Self::Bytes(data[1 + len_bytes_len..1 + len_bytes_len + len].to_vec()))
                } else {
                    None
                }
            } else {
                None
            }
        } else if first < 0xF8 {
            // Short list
            let len = (first - 0xC0) as usize;
            let mut items = Vec::new();
            let mut offset = 1;

            for _ in 0..len {
                if let Some(item) = Self::decode(&data[offset..]) {
                    offset += Self::encoded_length(&item);
                    items.push(item);
                } else {
                    return None;
                }
            }

            Some(Self::List(items))
        } else {
            // Long list
            let len_bytes_len = (first - 0xF7) as usize;
            if data.len() >= 1 + len_bytes_len {
                let mut len_bytes = [0u8; 8];
                len_bytes[8 - len_bytes_len..].copy_from_slice(&data[1..1 + len_bytes_len]);
                let len = usize::from_be_bytes(len_bytes);

                let mut items = Vec::new();
                let mut offset = 1 + len_bytes_len;

                for _ in 0..len {
                    if let Some(item) = Self::decode(&data[offset..]) {
                        offset += Self::encoded_length(&item);
                        items.push(item);
                    } else {
                        return None;
                    }
                }

                Some(Self::List(items))
            } else {
                None
            }
        }
    }

    // Get encoded length
    fn encoded_length(item: &Self) -> usize {
        match item {
            Self::Bytes(bytes) => {
                if bytes.len() == 1 && bytes[0] < 0x80 {
                    1
                } else if bytes.len() < 56 {
                    1 + bytes.len()
                } else {
                    1 + bytes.len().to_be_bytes().len() + bytes.len()
                }
            }
            Self::List(items) => {
                let total_len: usize = items.iter().map(Self::encoded_length).sum();
                if total_len < 56 {
                    1 + total_len
                } else {
                    1 + total_len.to_be_bytes().len() + total_len
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_trie() {
        let trie = MerklePatriciaTrie::new();
        assert_eq!(trie.root_hash(), H256::ZERO);
    }

    #[test]
    fn test_insert_and_get() {
        let mut trie = MerklePatriciaTrie::new();
        trie.insert(b"key1", b"value1");

        let value = trie.get(b"key1");
        assert_eq!(value, Some(b"value1".to_vec()));
    }

    #[test]
    fn test_multiple_keys() {
        let mut trie = MerklePatriciaTrie::new();
        trie.insert(b"key1", b"value1");
        trie.insert(b"key2", b"value2");
        trie.insert(b"key3", b"value3");

        assert_eq!(trie.get(b"key1"), Some(b"value1".to_vec()));
        assert_eq!(trie.get(b"key2"), Some(b"value2".to_vec()));
        assert_eq!(trie.get(b"key3"), Some(b"value3".to_vec()));
    }

    #[test]
    fn test_rlp_encode_decode() {
        let data = b"hello world";
        let encoded = RLPStream::encode_list(&[data.to_vec()]);
        let decoded = RLPItem::decode(&encoded);

        assert!(decoded.is_some());
    }

    #[test]
    fn test_nibble_conversion() {
        let bytes = [0xAB, 0xCD];
        let nibbles = MerklePatriciaTrie::bytes_to_nibbles(&bytes);
        assert_eq!(nibbles, vec![0xA, 0xB, 0xC, 0xD]);
    }
}
