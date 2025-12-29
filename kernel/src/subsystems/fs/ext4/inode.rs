//! Ext4 Inode Management
//!
//! 提供Ext4 inode的读取、写入和管理功能

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use crate::subsystems::sync::Mutex;
use crate::subsystems::fs::ext4::{Ext4FileSystem, Ext4SuperBlock};

/// Ext4 inode structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4Inode {
    /// File mode
    pub i_mode: u16,
    /// Owner UID
    pub i_uid: u16,
    /// File size (low 32 bits)
    pub i_size_lo: u32,
    /// Access time
    pub i_atime: u32,
    /// Creation time
    pub i_ctime: u32,
    /// Modification time
    pub i_mtime: u32,
    /// Deletion time
    pub i_dtime: u32,
    /// Owner GID
    pub i_gid: u16,
    /// Number of links
    pub i_links_count: u16,
    /// Number of blocks (low 32 bits)
    pub i_blocks_lo: u32,
    /// File flags
    pub i_flags: u32,
    /// OS-specific value 1
    pub osd1: u32,
    /// Direct block pointers or extent header
    pub i_block: [u32; 15],
    /// File version
    pub i_generation: u32,
    /// File ACL
    pub i_file_acl: u32,
    /// Directory ACL
    pub i_dir_acl: u32,
    /// Fragment address
    pub i_faddr: u32,
    /// OS-specific value 2
    pub osd2: [u32; 3],
    /// File size (high 32 bits)
    pub i_size_hi: u32,
    /// Number of blocks (high 16 bits)
    pub i_blocks_hi: u16,
    /// Padding
    pub i_pad: u16,
    /// Project ID
    pub i_projid: u16,
    /// Reserved
    pub reserved: [u32; 4],
}

impl Default for Ext4Inode {
    fn default() -> Self {
        Self {
            i_mode: 0,
            i_uid: 0,
            i_size_lo: 0,
            i_atime: 0,
            i_ctime: 0,
            i_mtime: 0,
            i_dtime: 0,
            i_gid: 0,
            i_links_count: 0,
            i_blocks_lo: 0,
            i_flags: 0,
            osd1: 0,
            i_block: [0; 15],
            i_generation: 0,
            i_file_acl: 0,
            i_dir_acl: 0,
            i_faddr: 0,
            osd2: [0; 3],
            i_size_hi: 0,
            i_blocks_hi: 0,
            i_pad: 0,
            i_projid: 0,
            reserved: [0; 4],
        }
    }
}

/// Ext4 extent header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4ExtentHeader {
    /// Magic number (0xF30A)
    pub eh_magic: u16,
    /// Number of valid entries
    pub eh_entries: u16,
    /// Maximum number of entries
    pub eh_max: u16,
    /// Depth of this extent node
    pub eh_depth: u16,
    /// Generation
    pub eh_generation: u32,
}

/// Ext4 extent
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4Extent {
    /// First logical block covered by this extent
    pub ee_block: u32,
    /// Length of this extent in blocks
    pub ee_len: u16,
    /// Starting physical block
    pub ee_start_hi: u16,
    pub ee_start_lo: u32,
}

/// Ext4 extent index
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4ExtentIdx {
    /// Index covers logical blocks from this block
    pub ei_block: u32,
    /// Leaf node following this index has this logical block
    pub ei_leaf_lo: u32,
    /// High 16 bits of leaf block
    pub ei_leaf_hi: u16,
    /// Should be zero
    pub ei_unused: u16,
}

/// Ext4 directory entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4DirEntry {
    /// Inode number
    pub inode: u32,
    /// Record length
    pub rec_len: u16,
    /// Name length
    pub name_len: u8,
    /// File type
    pub file_type: u8,
    /// Name (variable length)
    pub name: [u8; 0], // Flexible array member
}

/// Ext4 file types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Ext4FileType {
    Unknown = 0,
    Regular = 1,
    Directory = 2,
    CharDev = 3,
    BlockDev = 4,
    Fifo = 5,
    Socket = 6,
    Symlink = 7,
}

impl Ext4FileSystem {
    /// Read an inode from disk
    pub fn read_inode(&self, inum: u32) -> Result<Ext4Inode, &'static str> {
        // Check cache first
        {
            let cache = self.inode_cache.lock();
            if let Some(inode) = cache.get(&inum) {
                return Ok(*inode);
            }
        }

        // Calculate group and index
        let inodes_per_group = self.sb.s_inodes_per_group;
        let group = (inum - 1) / inodes_per_group;
        let index = (inum - 1) % inodes_per_group;

        if group >= self.group_count {
            return Err("Invalid inode number");
        }

        // Get group descriptor
        let desc = &self.group_descs[group as usize];

        // Calculate inode table block and offset
        let inode_size = self.sb.s_inode_size as u32;
        let inode_table_block = desc.bg_inode_table;
        let inode_offset = index * inode_size;
        let block_offset = inode_offset / self.block_size;
        let offset_in_block = inode_offset % self.block_size;

        // Read block containing inode
        let mut buf = vec![0u8; self.block_size as usize];
        self.dev.read((inode_table_block + block_offset) as usize, &mut buf);

        // Parse inode
        let offset = offset_in_block as usize;
        let mut inode = Ext4Inode::default();

        inode.i_mode = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        inode.i_uid = u16::from_le_bytes([buf[offset + 2], buf[offset + 3]]);
        inode.i_size_lo = u32::from_le_bytes([
            buf[offset + 4], buf[offset + 5], buf[offset + 6], buf[offset + 7],
        ]);
        inode.i_atime = u32::from_le_bytes([
            buf[offset + 8], buf[offset + 9], buf[offset + 10], buf[offset + 11],
        ]);
        inode.i_ctime = u32::from_le_bytes([
            buf[offset + 12], buf[offset + 13], buf[offset + 14], buf[offset + 15],
        ]);
        inode.i_mtime = u32::from_le_bytes([
            buf[offset + 16], buf[offset + 17], buf[offset + 18], buf[offset + 19],
        ]);
        inode.i_dtime = u32::from_le_bytes([
            buf[offset + 20], buf[offset + 21], buf[offset + 22], buf[offset + 23],
        ]);
        inode.i_gid = u16::from_le_bytes([buf[offset + 24], buf[offset + 25]]);
        inode.i_links_count = u16::from_le_bytes([buf[offset + 26], buf[offset + 27]]);
        inode.i_blocks_lo = u32::from_le_bytes([
            buf[offset + 28], buf[offset + 29], buf[offset + 30], buf[offset + 31],
        ]);
        inode.i_flags = u32::from_le_bytes([
            buf[offset + 32], buf[offset + 33], buf[offset + 34], buf[offset + 35],
        ]);
        inode.osd1 = u32::from_le_bytes([
            buf[offset + 36], buf[offset + 37], buf[offset + 38], buf[offset + 39],
        ]);

        // Read block pointers or extent header
        for i in 0..15 {
            inode.i_block[i] = u32::from_le_bytes([
                buf[offset + 40 + i * 4],
                buf[offset + 41 + i * 4],
                buf[offset + 42 + i * 4],
                buf[offset + 43 + i * 4],
            ]);
        }

        inode.i_generation = u32::from_le_bytes([
            buf[offset + 100], buf[offset + 101], buf[offset + 102], buf[offset + 103],
        ]);
        inode.i_file_acl = u32::from_le_bytes([
            buf[offset + 104], buf[offset + 105], buf[offset + 106], buf[offset + 107],
        ]);
        inode.i_dir_acl = u32::from_le_bytes([
            buf[offset + 108], buf[offset + 109], buf[offset + 110], buf[offset + 111],
        ]);
        inode.i_faddr = u32::from_le_bytes([
            buf[offset + 112], buf[offset + 113], buf[offset + 114], buf[offset + 115],
        ]);

        // Read OS-specific fields
        for i in 0..3 {
            inode.osd2[i] = u32::from_le_bytes([
                buf[offset + 116 + i * 4],
                buf[offset + 117 + i * 4],
                buf[offset + 118 + i * 4],
                buf[offset + 119 + i * 4],
            ]);
        }

        // Read additional fields if inode size is large enough
        if self.sb.s_inode_size >= 160 {
            inode.i_size_hi = u32::from_le_bytes([
                buf[offset + 120], buf[offset + 121], buf[offset + 122], buf[offset + 123],
            ]);
            inode.i_blocks_hi = u16::from_le_bytes([buf[offset + 124], buf[offset + 125]]);
            inode.i_pad = u16::from_le_bytes([buf[offset + 126], buf[offset + 127]]);
            inode.i_projid = u16::from_le_bytes([buf[offset + 128], buf[offset + 129]]);

            // Read reserved fields
            for i in 0..4 {
                inode.reserved[i] = u32::from_le_bytes([
                    buf[offset + 132 + i * 4],
                    buf[offset + 133 + i * 4],
                    buf[offset + 134 + i * 4],
                    buf[offset + 135 + i * 4],
                ]);
            }
        }

        // Cache the inode
        {
            let mut cache = self.inode_cache.lock();
            cache.insert(inum, inode);
        }

        Ok(inode)
    }

    /// Write an inode to disk
    pub fn write_inode(&mut self, inum: u32, inode: &Ext4Inode) -> Result<(), &'static str> {
        // Calculate group and index
        let inodes_per_group = self.sb.s_inodes_per_group;
        let group = (inum - 1) / inodes_per_group;
        let index = (inum - 1) % inodes_per_group;

        if group >= self.group_count {
            return Err("Invalid inode number");
        }

        // Get group descriptor
        let desc = &self.group_descs[group as usize];

        // Calculate inode table block and offset
        let inode_size = self.sb.s_inode_size as u32;
        let inode_table_block = desc.bg_inode_table;
        let inode_offset = index * inode_size;
        let block_offset = inode_offset / self.block_size;
        let offset_in_block = inode_offset % self.block_size;

        // Read block containing inode
        let mut buf = vec![0u8; self.block_size as usize];
        self.dev.read((inode_table_block + block_offset) as usize, &mut buf);

        // Update inode in buffer
        let offset = offset_in_block as usize;

        buf[offset..offset + 2].copy_from_slice(&inode.i_mode.to_le_bytes());
        buf[offset + 2..offset + 4].copy_from_slice(&inode.i_uid.to_le_bytes());
        buf[offset + 4..offset + 8].copy_from_slice(&inode.i_size_lo.to_le_bytes());
        buf[offset + 8..offset + 12].copy_from_slice(&inode.i_atime.to_le_bytes());
        buf[offset + 12..offset + 16].copy_from_slice(&inode.i_ctime.to_le_bytes());
        buf[offset + 16..offset + 20].copy_from_slice(&inode.i_mtime.to_le_bytes());
        buf[offset + 20..offset + 24].copy_from_slice(&inode.i_dtime.to_le_bytes());
        buf[offset + 24..offset + 26].copy_from_slice(&inode.i_gid.to_le_bytes());
        buf[offset + 26..offset + 28].copy_from_slice(&inode.i_links_count.to_le_bytes());
        buf[offset + 28..offset + 32].copy_from_slice(&inode.i_blocks_lo.to_le_bytes());
        buf[offset + 32..offset + 36].copy_from_slice(&inode.i_flags.to_le_bytes());
        buf[offset + 36..offset + 40].copy_from_slice(&inode.osd1.to_le_bytes());

        // Write block pointers or extent header
        for i in 0..15 {
            buf[offset + 40 + i * 4..offset + 44 + i * 4]
                .copy_from_slice(&inode.i_block[i].to_le_bytes());
        }

        buf[offset + 100..offset + 104].copy_from_slice(&inode.i_generation.to_le_bytes());
        buf[offset + 104..offset + 108].copy_from_slice(&inode.i_file_acl.to_le_bytes());
        buf[offset + 108..offset + 112].copy_from_slice(&inode.i_dir_acl.to_le_bytes());
        buf[offset + 112..offset + 116].copy_from_slice(&inode.i_faddr.to_le_bytes());

        // Write OS-specific fields
        for i in 0..3 {
            buf[offset + 116 + i * 4..offset + 120 + i * 4]
                .copy_from_slice(&inode.osd2[i].to_le_bytes());
        }

        // Write additional fields if inode size is large enough
        if self.sb.s_inode_size >= 160 {
            buf[offset + 120..offset + 124].copy_from_slice(&inode.i_size_hi.to_le_bytes());
            buf[offset + 124..offset + 126].copy_from_slice(&inode.i_blocks_hi.to_le_bytes());
            buf[offset + 126..offset + 128].copy_from_slice(&inode.i_pad.to_le_bytes());
            buf[offset + 128..offset + 130].copy_from_slice(&inode.i_projid.to_le_bytes());

            // Write reserved fields
            for i in 0..4 {
                buf[offset + 132 + i * 4..offset + 136 + i * 4]
                    .copy_from_slice(&inode.reserved[i].to_le_bytes());
            }
        }

        // Write block back to disk
        self.dev.write((inode_table_block + block_offset) as usize, &buf);

        // Update cache
        {
            let mut cache = self.inode_cache.lock();
            cache.insert(inum, *inode);
        }

        Ok(())
    }
}
