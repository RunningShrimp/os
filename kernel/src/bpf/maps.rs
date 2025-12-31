//! # eBPF Maps
//!
//! eBPF maps 用于在内核和用户空间之间共享数据。

use crate::prelude::*;
use alloc::vec::Vec;
use spin::RwLock;

/// eBPF Map 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfMapType {
    /// 哈希表
    HashTable,
    /// 数组
    Array,
}

/// eBPF Map
pub struct BpfMap {
    /// Map 类型
    map_type: BpfMapType,
    /// 键大小
    key_size: u32,
    /// 值大小
    value_size: u32,
    /// 最大条目数
    max_entries: u32,
    /// 数据存储
    data: RwLock<BpfMapData>,
}

/// Map 数据存储
enum BpfMapData {
    /// 哈希表存储
    Hash(BTreeMap<Vec<u8>, Vec<u8>>),
    /// 数组存储
    Array(Vec<Option<Vec<u8>>>),
}

impl BpfMap {
    /// 创建新的 Map
    pub fn new(
        map_type: BpfMapType,
        key_size: u32,
        value_size: u32,
        max_entries: u32,
        _name: &str,
    ) -> core::result::Result<Self, crate::bpf::BpfError> {
        // 验证参数
        if key_size == 0 || value_size == 0 || max_entries == 0 {
            return Err(crate::bpf::BpfError::InsufficientResources);
        }

        // 根据类型初始化存储
        let data = match map_type {
            BpfMapType::HashTable => BpfMapData::Hash(BTreeMap::new()),
            BpfMapType::Array => BpfMapData::Array(vec![None; max_entries as usize]),
        };

        Ok(Self {
            map_type,
            key_size,
            value_size,
            max_entries,
            data: RwLock::new(data),
        })
    }

    /// 获取 Map 类型
    pub fn map_type(&self) -> BpfMapType {
        self.map_type
    }

    /// 查找键
    pub fn lookup(&self, key: &[u8]) -> core::result::Result<Option<Vec<u8>>, crate::bpf::BpfError> {
        // 验证键大小
        if key.len() != self.key_size as usize {
            return Ok(None);
        }

        let data = self.data.read();

        match &*data {
            BpfMapData::Hash(map) => Ok(map.get(key).cloned()),
            BpfMapData::Array(arr) => {
                if key.len() != 4 {
                    return Ok(None);
                }
                let idx = u32::from_le_bytes([key[0], key[1], key[2], key[3]]) as usize;
                if idx < arr.len() {
                    Ok(arr[idx].clone())
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// 更新键值对
    pub fn update(&self, key: &[u8], value: &[u8], _flags: u64) -> core::result::Result<(), crate::bpf::BpfError> {
        // 验证大小
        if key.len() != self.key_size as usize || value.len() != self.value_size as usize {
            return Err(crate::bpf::BpfError::InsufficientResources);
        }

        let mut data = self.data.write();

        match &mut *data {
            BpfMapData::Hash(map) => {
                if !map.contains_key(key) && map.len() >= self.max_entries as usize {
                    return Err(crate::bpf::BpfError::InsufficientResources);
                }
                map.insert(key.to_vec(), value.to_vec());
                Ok(())
            }
            BpfMapData::Array(arr) => {
                if key.len() != 4 {
                    return Err(crate::bpf::BpfError::InsufficientResources);
                }
                let idx = u32::from_le_bytes([key[0], key[1], key[2], key[3]]) as usize;
                if idx < arr.len() {
                    arr[idx] = Some(value.to_vec());
                    Ok(())
                } else {
                    Err(crate::bpf::BpfError::InsufficientResources)
                }
            }
        }
    }

    /// 获取当前条目数
    pub fn len(&self) -> usize {
        let data = self.data.read();
        match &*data {
            BpfMapData::Hash(map) => map.len(),
            BpfMapData::Array(arr) => arr.iter().filter(|x| x.is_some()).count(),
        }
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_type() {
        assert_eq!(BpfMapType::HashTable, BpfMapType::HashTable);
    }

    #[test]
    fn test_bpf_map_hash_create() {
        let map = BpfMap::new(BpfMapType::HashTable, 4, 8, 1024, "test_hash");
        assert!(map.is_ok());

        let map = map.unwrap();
        assert_eq!(map.key_size, 4);
        assert_eq!(map.value_size, 8);
        assert!(map.is_empty());
    }

    #[test]
    fn test_bpf_map_hash_operations() {
        let map = BpfMap::new(BpfMapType::HashTable, 4, 8, 1024, "test_hash").unwrap();

        let key = vec![1u8, 2, 3, 4];
        let value = vec![42u8; 8];

        // 插入
        map.update(&key, &value, 0).unwrap();

        // 查找
        let found = map.lookup(&key).unwrap();
        assert_eq!(found, Some(value.clone()));

        // 长度
        assert_eq!(map.len(), 1);
    }
}
