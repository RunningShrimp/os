//! Ext4 File System Implementation
//!
//! This module implements the Ext4 file system core functionality for the NOS operating system.
//! Ext4 is a widely used journaling file system for Linux with features like:
//! - Large file system and file size support
//! - Extents for efficient block allocation
//! - Journaling for data integrity
//! - Flexible block allocation strategies
//! - Backward compatibility with Ext2/Ext3
//!
//! # Module Structure
//!
//! - `inode.rs`: Inode management and operations
//! - `superblock.rs`: Superblock management and operations
//! - `journal.rs`: Journaling implementation

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use alloc::boxed::Box;
use crate::drivers::BlockDevice;
use crate::subsystems::sync::Mutex;
use crate::subsystems::fs::fs_impl::BufCache;

// 导出子模块
pub mod inode;
pub mod superblock;
pub mod journal;

// 重新导出常用类型和函数
pub use inode::*;
pub use superblock::*;
pub use journal::*;

/// Ext4 magic number
pub const EXT4_MAGIC: u16 = 0xEF53;

/// Ext4 file system implementation
pub struct Ext4FileSystem {
    dev: Box<dyn BlockDevice>,
    sb: Ext4SuperBlock,
    block_size: u32,
    group_count: u32,
    group_descs: Vec<Ext4GroupDesc>,
    buf_cache: BufCache,
    inode_cache: Mutex<BTreeMap<u32, Ext4Inode>>,
    block_bitmap_cache: Mutex<BTreeMap<u32, Vec<bool>>>,
    inode_bitmap_cache: Mutex<BTreeMap<u32, Vec<bool>>>,
    #[allow(dead_code)]
    mount_options: Ext4MountOptions,
    #[allow(dead_code)]
    journal: Option<Box<dyn JournalingFileSystem>>,
    #[allow(dead_code)]
    xattr_cache: Mutex<BTreeMap<u32, BTreeMap<String, Vec<u8>>>>,
    #[allow(dead_code)]
    acl_cache: Mutex<BTreeMap<u32, Vec<u8>>>,
    #[allow(dead_code)]
    quota_info: Mutex<BTreeMap<u32, Ext4QuotaInfo>>,
    #[allow(dead_code)]
    project_quota: Mutex<BTreeMap<u32, Ext4ProjectQuota>>,
    #[allow(dead_code)]
    encryption_contexts: Mutex<BTreeMap<u32, Ext4EncryptionContext>>,
    #[allow(dead_code)]
    extent_status_trees: Mutex<BTreeMap<u32, Ext4ExtentStatusTree>>,
    #[allow(dead_code)]
    mmp: Option<Ext4MmpStruct>,
    #[allow(dead_code)]
    stats: Mutex<Ext4Stats>,
    #[allow(dead_code)]
    checksum_seed: Mutex<Ext4ChecksumSeed>,
    #[allow(dead_code)]
    flex_bg_descs: Mutex<BTreeMap<u32, Ext4FlexBgDesc>>,
    #[allow(dead_code)]
    dir_index_roots: Mutex<BTreeMap<u32, Ext4DirIndexRoot>>,
    #[allow(dead_code)]
    dir_index_tails: Mutex<BTreeMap<u32, Ext4DirIndexTail>>,
    #[allow(dead_code)]
    dir_index_nodes: Mutex<BTreeMap<u32, Ext4DirIndexNode>>,
    #[allow(dead_code)]
    xattr_headers: Mutex<BTreeMap<u32, Ext4XattrHeader>>,
    #[allow(dead_code)]
    xattr_entries: Mutex<BTreeMap<u32, Vec<Ext4XattrEntry>>>,
    #[allow(dead_code)]
    journal_entries: Mutex<BTreeMap<u32, Vec<JournalEntry>>>,
    #[allow(dead_code)]
    journal_transactions: Mutex<BTreeMap<u32, JournalTransaction>>,
    #[allow(dead_code)]
    journal_checkpoint: Mutex<u32>,
}

impl Ext4FileSystem {
    pub fn new(dev: Box<dyn BlockDevice>) -> Self {
        Self {
            dev,
            sb: Ext4SuperBlock::default(),
            block_size: 1024,
            group_count: 0,
            group_descs: Vec::new(),
            buf_cache: BufCache::new(),
            inode_cache: Mutex::new(BTreeMap::new()),
            block_bitmap_cache: Mutex::new(BTreeMap::new()),
            inode_bitmap_cache: Mutex::new(BTreeMap::new()),
            mount_options: Ext4MountOptions::default(),
            journal: None,
            xattr_cache: Mutex::new(BTreeMap::new()),
            acl_cache: Mutex::new(BTreeMap::new()),
            quota_info: Mutex::new(BTreeMap::new()),
            project_quota: Mutex::new(BTreeMap::new()),
            encryption_contexts: Mutex::new(BTreeMap::new()),
            extent_status_trees: Mutex::new(BTreeMap::new()),
            mmp: None,
            stats: Mutex::new(Ext4Stats::default()),
            checksum_seed: Mutex::new(Ext4ChecksumSeed { checksum_seed: 0 }),
            flex_bg_descs: Mutex::new(BTreeMap::new()),
            dir_index_roots: Mutex::new(BTreeMap::new()),
            dir_index_tails: Mutex::new(BTreeMap::new()),
            dir_index_nodes: Mutex::new(BTreeMap::new()),
            xattr_headers: Mutex::new(BTreeMap::new()),
            xattr_entries: Mutex::new(BTreeMap::new()),
            journal_entries: Mutex::new(BTreeMap::new()),
            journal_transactions: Mutex::new(BTreeMap::new()),
            journal_checkpoint: Mutex::new(0),
        }
    }

    /// Initialize the file system
    pub fn init(&mut self) -> Result<(), &'static str> {
        // Initialize buffer cache
        self.buf_cache.init();

        // Read superblock (at block 1)
        self.read_superblock()?;

        // Verify magic number
        if self.sb.s_magic != EXT4_MAGIC {
            return Err("Invalid Ext4 magic number");
        }

        // Calculate block size
        self.block_size = 1024 << self.sb.s_log_block_size;

        // Calculate block group count
        let blocks_per_group = self.sb.s_blocks_per_group as u64;
        let total_blocks = self.get_total_blocks();
        self.group_count = ((total_blocks + blocks_per_group - 1) / blocks_per_group) as u32;

        // Read block group descriptors
        self.read_group_descriptors()?;

        crate::println!(
            "ext4: {} blocks, {} inodes, {} groups, block size: {}",
            self.get_total_blocks(),
            self.get_total_inodes(),
            self.group_count,
            self.block_size
        );

        Ok(())
    }

    /// Get total number of blocks in the file system
    pub fn get_total_blocks(&self) -> u64 {
        ((self.sb.s_blocks_count_hi as u64) << 32) | (self.sb.s_blocks_count_lo as u64)
    }

    /// Get total number of inodes in the file system
    pub fn get_total_inodes(&self) -> u32 {
        ((self.sb.s_inodes_count_hi as u32) << 16) | self.sb.s_inodes_count
    }

    /// Read block bitmap for a group
    fn read_block_bitmap(&self, group: u32) -> Result<Vec<bool>, &'static str> {
        // Check cache first
        {
            let cache = self.block_bitmap_cache.lock();
            if let Some(bitmap) = cache.get(&group) {
                return Ok(bitmap.clone());
            }
        }

        if group >= self.group_count {
            return Err("Invalid group number");
        }

        // Get group descriptor
        let desc = &self.group_descs[group as usize];
        let bitmap_block = desc.bg_block_bitmap;

        // Read bitmap block
        let mut buf = vec![0u8; self.block_size as usize];
        self.dev.read(bitmap_block as usize, &mut buf);

        // Convert to boolean vector
        let mut bitmap = Vec::new();
        for byte in buf {
            for bit in 0..8 {
                bitmap.push((byte & (1 << bit)) != 0);
            }
        }

        // Cache bitmap
        {
            let mut cache = self.block_bitmap_cache.lock();
            cache.insert(group, bitmap.clone());
        }

        Ok(bitmap)
    }

    /// Write block bitmap for a group
    fn write_block_bitmap(&mut self, group: u32, bitmap: &[bool]) -> Result<(), &'static str> {
        if group >= self.group_count {
            return Err("Invalid group number");
        }

        // Get group descriptor
        let desc = &self.group_descs[group as usize];
        let bitmap_block = desc.bg_block_bitmap;

        // Convert boolean vector to bytes
        let mut buf = vec![0u8; self.block_size as usize];
        for (i, &is_set) in bitmap.iter().enumerate() {
            if i >= self.block_size as usize * 8 {
                break;
            }

            let byte_idx = i / 8;
            let bit_idx = i % 8;

            if is_set {
                buf[byte_idx] |= 1 << bit_idx;
            }
        }

        // Write bitmap block
        self.dev.write(bitmap_block as usize, &buf);

        // Update cache
        {
            let mut cache = self.block_bitmap_cache.lock();
            cache.insert(group, bitmap.to_vec());
        }

        Ok(())
    }

    /// Read inode bitmap for a group
    fn read_inode_bitmap(&self, group: u32) -> Result<Vec<bool>, &'static str> {
        // Check cache first
        {
            let cache = self.inode_bitmap_cache.lock();
            if let Some(bitmap) = cache.get(&group) {
                return Ok(bitmap.clone());
            }
        }

        if group >= self.group_count {
            return Err("Invalid group number");
        }

        // Get group descriptor
        let desc = &self.group_descs[group as usize];
        let bitmap_block = desc.bg_inode_bitmap;

        // Read bitmap block
        let mut buf = vec![0u8; self.block_size as usize];
        self.dev.read(bitmap_block as usize, &mut buf);

        // Convert to boolean vector
        let mut bitmap = Vec::new();
        for byte in buf {
            for bit in 0..8 {
                bitmap.push((byte & (1 << bit)) != 0);
            }
        }

        // Cache bitmap
        {
            let mut cache = self.inode_bitmap_cache.lock();
            cache.insert(group, bitmap.clone());
        }

        Ok(bitmap)
    }

    /// Write inode bitmap for a group
    fn write_inode_bitmap(&mut self, group: u32, bitmap: &[bool]) -> Result<(), &'static str> {
        if group >= self.group_count {
            return Err("Invalid group number");
        }

        // Get group descriptor
        let desc = &self.group_descs[group as usize];
        let bitmap_block = desc.bg_inode_bitmap;

        // Convert boolean vector to bytes
        let mut buf = vec![0u8; self.block_size as usize];
        for (i, &is_set) in bitmap.iter().enumerate() {
            if i >= self.block_size as usize * 8 {
                break;
            }

            let byte_idx = i / 8;
            let bit_idx = i % 8;

            if is_set {
                buf[byte_idx] |= 1 << bit_idx;
            }
        }

        // Write bitmap block
        self.dev.write(bitmap_block as usize, &buf);

        // Update cache
        {
            let mut cache = self.inode_bitmap_cache.lock();
            cache.insert(group, bitmap.to_vec());
        }

        Ok(())
    }

    /// Allocate a free block
    pub fn alloc_block(&mut self) -> Result<u32, &'static str> {
        // Search through groups for a free block
        for group in 0..self.group_count {
            let mut bitmap = self.read_block_bitmap(group)?;

            // Find first free block in this group
            for (i, &is_used) in bitmap.iter().enumerate() {
                if !is_used {
                    // Mark as used
                    bitmap[i] = true;
                    self.write_block_bitmap(group, &bitmap)?;

                    // Calculate block number
                    let blocks_per_group = self.sb.s_blocks_per_group;
                    let block_num = group * blocks_per_group + i as u32;

                    return Ok(block_num);
                }
            }
        }

        Err("No free blocks available")
    }

    /// Free a block
    pub fn free_block(&mut self, block_num: u32) -> Result<(), &'static str> {
        // Calculate group and index
        let blocks_per_group = self.sb.s_blocks_per_group;
        let group = block_num / blocks_per_group;
        let index = (block_num % blocks_per_group) as usize;

        if group >= self.group_count {
            return Err("Invalid block number");
        }

        // Read bitmap
        let mut bitmap = self.read_block_bitmap(group)?;

        // Mark as free
        if index < bitmap.len() {
            bitmap[index] = false;
            self.write_block_bitmap(group, &bitmap)?;
            return Ok(());
        }

        Err("Invalid block index")
    }

    /// Allocate a free inode
    pub fn alloc_inode(&mut self) -> Result<u32, &'static str> {
        // Search through groups for a free inode
        for group in 0..self.group_count {
            let mut bitmap = self.read_inode_bitmap(group)?;

            // Find first free inode in this group
            for (i, &is_used) in bitmap.iter().enumerate() {
                if !is_used {
                    // Mark as used
                    bitmap[i] = true;
                    self.write_inode_bitmap(group, &bitmap)?;

                    // Calculate inode number
                    let inodes_per_group = self.sb.s_inodes_per_group;
                    let inum = group * inodes_per_group + i as u32 + 1; // +1 because inode 0 is reserved

                    return Ok(inum);
                }
            }
        }

        Err("No free inodes available")
    }

    /// Free an inode
    pub fn free_inode(&mut self, inum: u32) -> Result<(), &'static str> {
        // Calculate group and index
        let inodes_per_group = self.sb.s_inodes_per_group;
        let group = (inum - 1) / inodes_per_group;
        let index = ((inum - 1) % inodes_per_group) as usize;

        if group >= self.group_count {
            return Err("Invalid inode number");
        }

        // Read bitmap
        let mut bitmap = self.read_inode_bitmap(group)?;

        // Mark as free
        if index < bitmap.len() {
            bitmap[index] = false;
            self.write_inode_bitmap(group, &bitmap)?;

            // Remove from cache
            {
                let mut cache = self.inode_cache.lock();
                cache.remove(&inum);
            }

            return Ok(());
        }

        Err("Invalid inode index")
    }
}

/// Initialize Ext4 file system
pub fn init() {
    crate::println!("ext4: initializing");
    // In a real implementation, this would initialize the Ext4 file system
    crate::println!("ext4: initialized");
}
