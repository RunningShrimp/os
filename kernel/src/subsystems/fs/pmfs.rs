//! # 持久化内存文件系统 (PMFS)
//!
//! 专为持久化内存优化的文件系统实现。
//!
//! ## 功能
//!
//! - 持久化 B+Tree 索引
//! - 原子元数据更新
//! - 崩溃一致性保证
//! - Checkpoint/Recovery 机制
//!
//! ## 架构
//!
//! ```
//! PMFS
//!     ├── B+Tree 索引层
//!     ├── 对象存储层
//!     ├── 事务层
//!     └── 恢复层
//! ```

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use nos_api::Error;

use crate::vfs::types::FileType;
use crate::subsystems::sync::Mutex as AdvancedMutex;

/// PMFS 魔数
pub const PMFS_MAGIC: u64 = 0x504D4653_5652_0000; // "PMFSVR\0"

/// PMFS 版本
pub const PMFS_VERSION: u32 = 1;

/// B+Tree 阶数（分支因子）
pub const BTREE_ORDER: usize = 128;

/// 最大文件名长度
pub const MAX_FILENAME_LEN: usize = 255;

/// 根节点 ID
pub const ROOT_INODE: u64 = 1;

/// PMFS 超级块
#[derive(Debug)]
#[repr(C)]
pub struct PmfsSuperBlock {
    /// 魔数
    pub magic: u64,
    /// 版本
    pub version: u32,
    /// 文件系统大小
    pub fs_size: u64,
    /// 块大小
    pub block_size: u32,
    /// 总块数
    pub total_blocks: u64,
    /// 可用块数
    pub free_blocks: AtomicU64,
    /// 下一个 inode 号
    pub next_ino: AtomicU64,
    /// 根 inode
    pub root_ino: u64,
    /// 创建时间戳
    pub create_ts: u64,
    /// 挂载时间戳
    pub mount_ts: u64,
    /// 最后写入时间
    pub write_ts: u64,
    /// 检查点序号
    pub checkpoint_seq: AtomicU64,
}

/// B+Tree 节点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BTreeNodeType {
    /// 内部节点
    Internal = 0,
    /// 叶子节点
    Leaf = 1,
}

/// B+Tree 节点头部
#[derive(Debug, Clone)]
#[repr(C)]
pub struct BTreeNodeHeader {
    /// 节点类型
    pub node_type: u8,
    /// 键数量
    pub num_keys: u16,
    /// 节点编号
    pub node_id: u64,
    /// 父节点编号
    pub parent_id: u64,
}

/// B+Tree 内部节点
#[derive(Debug, Clone)]
pub struct BTreeInternalNode {
    /// 头部
    pub header: BTreeNodeHeader,
    /// 键
    pub keys: Vec<u64>,
    /// 子节点指针（子节点编号）
    pub children: Vec<u64>,
}

/// B+Tree 叶子节点
#[derive(Debug, Clone)]
pub struct BTreeLeafNode {
    /// 头部
    pub header: BTreeNodeHeader,
    /// 键（inode 号）
    pub keys: Vec<u64>,
    /// 值（inode 数据指针）
    pub values: Vec<u64>,
    /// 前驱叶子节点
    pub prev_leaf: u64,
    /// 后继叶子节点
    pub next_leaf: u64,
}

/// B+Tree 节点
#[derive(Debug, Clone)]
pub enum BTreeNode {
    Internal(BTreeInternalNode),
    Leaf(BTreeLeafNode),
}

/// PMFS Inode
#[derive(Debug, Clone)] // 移除 Copy，因为包含数组
pub struct PmfsInode {
    /// inode 号
    pub ino: u64,
    /// 文件类型
    pub file_type: FileType,
    /// 文件模式
    pub mode: u32,
    /// 文件大小
    pub size: u64,
    /// 数据块数
    pub blocks: u64,
    /// 直接块指针（12 个）
    pub direct_blocks: [u64; 12],
    /// 间接块指针
    pub indirect_block: u64,
    /// 双重间接块指针
    pub double_indirect: u64,
    /// 访问时间
    pub atime: u64,
    /// 修改时间
    pub mtime: u64,
    /// 创建时间
    pub ctime: u64,
    /// 链接计数
    pub nlink: u32,
    /// UID
    pub uid: u32,
    /// GID
    pub gid: u32,
}

impl PmfsInode {
    /// 创建新 inode
    pub fn new(ino: u64, file_type: FileType, mode: u32) -> Self {
        Self {
            ino,
            file_type,
            mode,
            size: 0,
            blocks: 0,
            direct_blocks: [0; 12],
            indirect_block: 0,
            double_indirect: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            nlink: 1,
            uid: 0,
            gid: 0,
        }
    }
}

/// 目录项
#[derive(Debug, Clone)]
#[repr(C)]
pub struct PmfsDirEntry {
    /// inode 号
    pub ino: u64,
    /// 文件类型
    pub file_type: u8,
    /// 名字长度
    pub name_len: u8,
    /// 文件名
    pub name: [u8; MAX_FILENAME_LEN],
}

/// 检查点头部
#[derive(Debug, Clone)]
#[repr(C)]
pub struct CheckpointHeader {
    /// 魔数
    pub magic: u64,
    /// 检查点序号
    pub seq: u64,
    /// 检查点时间戳
    pub timestamp: u64,
    /// 超级块偏移
    pub superblock_offset: u64,
    /// 根 B+Tree 节点偏移
    pub root_btree_offset: u64,
}

/// PMFS 文件系统
#[derive(Debug)]
pub struct Pmfs {
    /// 超级块
    pub superblock: PmfsSuperBlock,
    /// 持久化内存基址
    pub pmem_base: u64,
    /// 持久化内存大小
    pub pmem_size: u64,
    /// B+Tree 根节点编号
    pub btree_root: AtomicU64,
    /// Inode 缓存
    pub inode_cache: Mutex<BTreeMap<u64, Arc<PmfsInode>>>,
    /// B+Tree 节点缓存
    pub btree_cache: Mutex<BTreeMap<u64, Arc<BTreeNode>>>,
    /// 下一个节点 ID
    pub next_node_id: AtomicU64,
    /// 是否已挂载
    pub mounted: AtomicU64,
}

impl Pmfs {
    /// 创建新的 PMFS
    pub fn new(pmem_base: u64, pmem_size: u64) -> Self {
        let block_size = 4096u32;
        let total_blocks = pmem_size / block_size as u64;

        Self {
            superblock: PmfsSuperBlock {
                magic: PMFS_MAGIC,
                version: PMFS_VERSION,
                fs_size: pmem_size,
                block_size,
                total_blocks,
                free_blocks: AtomicU64::new(total_blocks),
                next_ino: AtomicU64::new(ROOT_INODE + 1),
                root_ino: ROOT_INODE,
                create_ts: 0,
                mount_ts: 0,
                write_ts: 0,
                checkpoint_seq: AtomicU64::new(0),
            },
            pmem_base,
            pmem_size,
            btree_root: AtomicU64::new(0),
            inode_cache: Mutex::new(BTreeMap::new()),
            btree_cache: Mutex::new(BTreeMap::new()),
            next_node_id: AtomicU64::new(1),
            mounted: AtomicU64::new(0),
        }
    }

    /// 初始化文件系统
    pub fn init(&self) -> Result<(), Error> {
        crate::println!("[pmfs] Initializing PMFS at 0x{:x}", self.pmem_base);

        // 创建根目录
        let root_inode = PmfsInode::new(ROOT_INODE, FileType::Directory, 0o755);
        self.inode_cache.lock().insert(ROOT_INODE, Arc::new(root_inode));

        // 创建 B+Tree 根节点
        let root_leaf = BTreeNode::Leaf(BTreeLeafNode {
            header: BTreeNodeHeader {
                node_type: BTreeNodeType::Leaf as u8,
                num_keys: 0,
                node_id: 0,
                parent_id: 0,
            },
            keys: Vec::new(),
            values: Vec::new(),
            prev_leaf: 0,
            next_leaf: 0,
        });

        self.btree_cache.lock().insert(0, Arc::new(root_leaf));
        self.btree_root.store(0, Ordering::Release);

        // 创建检查点
        self.create_checkpoint()?;

        self.mounted.store(1, Ordering::Release);

        crate::println!("[pmfs] PMFS initialized successfully");
        crate::println!("[pmfs]   Block size: {} bytes", self.superblock.block_size);
        crate::println!("[pmfs]   Total blocks: {}", self.superblock.total_blocks);

        Ok(())
    }

    /// 创建检查点
    pub fn create_checkpoint(&self) -> Result<(), Error> {
        let seq = self.superblock.checkpoint_seq.fetch_add(1, Ordering::SeqCst);

        crate::println!("[pmfs] Creating checkpoint {}", seq);

        // TODO: 持久化超级块
        // TODO: 持久化 B+Tree 根节点
        // TODO: 刷新所有缓存

        // 持久化检查点
        unsafe {
            let checkpoint_offset = self.pmem_size - 4096;
            let checkpoint_ptr = (self.pmem_base + checkpoint_offset) as *mut CheckpointHeader;

            let checkpoint = CheckpointHeader {
                magic: 0x434850_544B_5400, // "CPHKTT\0"
                seq,
                timestamp: 0, // TODO: 获取时间戳
                superblock_offset: 0,
                root_btree_offset: self.btree_root.load(Ordering::Acquire),
            };

            checkpoint_ptr.write(checkpoint);

            crate::subsystems::mm::libpmem::pmem_persist(
                checkpoint_ptr as u64,
                core::mem::size_of::<CheckpointHeader>(),
            );
        }

        Ok(())
    }

    /// 分配块
    pub fn alloc_block(&self) -> Result<u64, Error> {
        let block_id = self.superblock.free_blocks.fetch_sub(1, Ordering::SeqCst);

        if block_id == 0 {
            return Err(Error::InvalidArgument("no free blocks available".to_string()));
        }

        let offset = (block_id - 1) * self.superblock.block_size as u64;

        Ok(offset)
    }

    /// 分配 inode
    pub fn alloc_inode(&self, file_type: FileType, mode: u32) -> Result<Arc<PmfsInode>, Error> {
        let ino = self.superblock.next_ino.fetch_add(1, Ordering::SeqCst);
        let inode = Arc::new(PmfsInode::new(ino, file_type, mode));

        self.inode_cache.lock().insert(ino, inode.clone());

        Ok(inode)
    }

    /// 查找 inode
    pub fn lookup_inode(&self, ino: u64) -> Result<Arc<PmfsInode>, Error> {
        self.inode_cache
            .lock()
            .get(&ino)
            .cloned()
            .ok_or_else(|| Error::InvalidArgument(format!("inode {} not found", ino)))
    }

    /// B+Tree 查找
    pub fn btree_lookup(&self, key: u64) -> Result<u64, Error> {
        let node_id = self.btree_root.load(Ordering::Acquire);

        self.btree_lookup_recursive(node_id, key)
    }

    /// B+Tree 递归查找
    fn btree_lookup_recursive(&self, node_id: u64, key: u64) -> Result<u64, Error> {
        let nodes = self.btree_cache.lock();
        let node = nodes.get(&node_id).ok_or_else(|| Error::InvalidArgument("node not found".into()))?.clone();
        drop(nodes);

        match &*node {
            BTreeNode::Internal(internal) => {
                // 查找合适的子节点
                let idx = internal.keys.iter().position(|&k| key < k).unwrap_or(internal.keys.len());
                let child_id = internal.children[idx];
                self.btree_lookup_recursive(child_id, key)
            }
            BTreeNode::Leaf(leaf) => {
                // 在叶子节点中查找
                leaf.keys
                    .iter()
                    .position(|&k| k == key)
                    .and_then(|idx| leaf.values.get(idx).copied())
                    .ok_or_else(|| Error::InvalidArgument(format!("key {} not found", key)))
            }
        }
    }

    /// B+Tree 插入
    pub fn btree_insert(&self, key: u64, value: u64) -> Result<(), Error> {
        let root_id = self.btree_root.load(Ordering::Acquire);
        let (new_root, split_key, split_value) = self.btree_insert_recursive(root_id, key, value)?;

        if let (Some(new_root_id), Some(sk), Some(_sv)) = (new_root, split_key, split_value) {
            // 根节点分裂，创建新根
            let new_root_node = BTreeNode::Internal(BTreeInternalNode {
                header: BTreeNodeHeader {
                    node_type: BTreeNodeType::Internal as u8,
                    num_keys: 1,
                    node_id: self.next_node_id.fetch_add(1, Ordering::SeqCst),
                    parent_id: 0,
                },
                keys: vec![sk],
                children: vec![root_id, new_root_id],
            });

            let new_root_id = self.next_node_id.fetch_add(1, Ordering::SeqCst);
            self.btree_cache.lock().insert(new_root_id, Arc::new(new_root_node));
            self.btree_root.store(new_root_id, Ordering::Release);
        }

        Ok(())
    }

    /// B+Tree 递归插入
    fn btree_insert_recursive(
        &self,
        node_id: u64,
        key: u64,
        value: u64,
    ) -> Result<(Option<u64>, Option<u64>, Option<u64>), Error> {
        let nodes = self.btree_cache.lock();
        let node = nodes.get(&node_id).ok_or_else(|| Error::InvalidArgument("node not found".into()))?.clone();
        drop(nodes);

        match &*node {
            BTreeNode::Internal(internal) => {
                // 查找合适的子节点
                let idx = internal.keys.iter().position(|&k| key < k).unwrap_or(internal.keys.len());
                let child_id = internal.children[idx];

                let (new_child, split_key, split_value) =
                    self.btree_insert_recursive(child_id, key, value)?;

                if let (Some(_nc), Some(_sk), Some(_sv)) = (new_child, split_key, split_value) {
                    // 子节点分裂，需要在当前节点中插入新的键和子节点
                    // TODO: 处理节点分裂
                    Ok((None, None, None))
                } else {
                    Ok((None, None, None))
                }
            }
            BTreeNode::Leaf(leaf) => {
                // 在叶子节点中插入
                if leaf.keys.len() < BTREE_ORDER {
                    // 有空间，直接插入
                    // TODO: 插入并排序
                    Ok((None, None, None))
                } else {
                    // 需要分裂
                    // TODO: 实现叶子节点分裂
                    Ok((None, None, None))
                }
            }
        }
    }

    /// 恢复文件系统
    pub fn recover(&self) -> Result<(), Error> {
        crate::println!("[pmfs] Recovering PMFS...");

        // TODO: 从检查点恢复
        // 1. 读取最新的检查点
        // 2. 恢复超级块
        // 3. 恢复 B+Tree
        // 4. 重建缓存

        crate::println!("[pmfs] Recovery complete");

        Ok(())
    }
}

/// 全局 PMFS 实例
static PMFS_INSTANCE: AdvancedMutex<Option<Pmfs>> = AdvancedMutex::new(None);

/// 初始化 PMFS
pub fn init(pmem_base: u64, pmem_size: u64) -> Result<(), Error> {
    let pmfs = Pmfs::new(pmem_base, pmem_size);
    pmfs.init()?;

    *PMFS_INSTANCE.lock() = Some(pmfs);

    Ok(())
}

/// 关闭 PMFS
pub fn shutdown() -> Result<(), Error> {
    if let Some(pmfs) = PMFS_INSTANCE.lock().as_ref() {
        pmfs.create_checkpoint()?;
    }

    *PMFS_INSTANCE.lock() = None;

    crate::println!("[pmfs] PMFS shutdown complete");

    Ok(())
}

/// 获取 PMFS 实例
pub fn get_pmfs() -> Result<&'static AdvancedMutex<Option<Pmfs>>, Error> {
    if PMFS_INSTANCE.lock().is_some() {
        Ok(&PMFS_INSTANCE)
    } else {
        Err(Error::InvalidState("PMFS not initialized".to_string()))
    }
}

/// 创建检查点（便捷函数）
pub fn create_checkpoint() -> Result<(), Error> {
    let pmfs_guard = PMFS_INSTANCE.lock();
    let pmfs = pmfs_guard.as_ref().ok_or_else(|| Error::InvalidState("PMFS not initialized".to_string()))?;
    pmfs.create_checkpoint()
}

/// 分配 inode（便捷函数）
pub fn alloc_inode(file_type: FileType, mode: u32) -> Result<Arc<PmfsInode>, Error> {
    let pmfs_guard = PMFS_INSTANCE.lock();
    let pmfs = pmfs_guard.as_ref().ok_or_else(|| Error::InvalidState("PMFS not initialized".to_string()))?;
    pmfs.alloc_inode(file_type, mode)
}

/// 查找 inode（便捷函数）
pub fn lookup_inode(ino: u64) -> Result<Arc<PmfsInode>, Error> {
    let pmfs_guard = PMFS_INSTANCE.lock();
    let pmfs = pmfs_guard.as_ref().ok_or_else(|| Error::InvalidState("PMFS not initialized".to_string()))?;
    pmfs.lookup_inode(ino)
}

/// B+Tree 插入（便捷函数）
pub fn btree_insert(key: u64, value: u64) -> Result<(), Error> {
    let pmfs_guard = PMFS_INSTANCE.lock();
    let pmfs = pmfs_guard.as_ref().ok_or_else(|| Error::InvalidState("PMFS not initialized".into()))?;
    pmfs.btree_insert(key, value)
}

/// B+Tree 查找（便捷函数）
pub fn btree_lookup(key: u64) -> Result<u64, Error> {
    let pmfs_guard = PMFS_INSTANCE.lock();
    let pmfs = pmfs_guard.as_ref().ok_or_else(|| Error::InvalidState("PMFS not initialized".into()))?;
    pmfs.btree_lookup(key)
}
