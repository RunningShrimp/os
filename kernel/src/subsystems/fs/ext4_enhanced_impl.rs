//! Enhanced Ext4 File System Implementation - Additional Functionality
//!
//! This module implements additional functionality of enhanced Ext4 file system.

extern crate alloc;
use alloc::vec::Vec;
use crate::drivers::BlockDevice;

// 移除了未使用的导入: JournalingFileSystem, JournalEntry, JournalTransaction
use super::{
    Ext4FileSystemEnhanced, Ext4MountOptions, Ext4Stats, Ext4QuotaInfo, Ext4ProjectQuota,
    Ext4EncryptionContext, Ext4ExtentStatusTree, Ext4MmpStruct, Ext4ChecksumSeed,
    Ext4FlexBgDesc, Ext4DirIndexRoot, Ext4DirIndexTail, Ext4DirIndexNode,
    Ext4XattrHeader, Ext4XattrEntry, EXT4_FEATURE_INCOMPAT_EXTENTS,
    EXT4_FEATURE_COMPAT_HAS_JOURNAL, EXT4_FEATURE_RO_COMPAT_METADATA_CSUM,
    EXT4_FEATURE_INCOMPAT_64BIT, EXT4_FEATURE_RO_COMPAT_HUGE_FILE,
    EXT4_FEATURE_INCOMPAT_FLEX_BG, EXT4_FEATURE_INCOMPAT_ENCRYPT,
    EXT4_FEATURE_RO_COMPAT_QUOTA, EXT4_FEATURE_INCOMPAT_MMP,
    EXT4_FEATURE_INCOMPAT_LARGEDIR, EXT4_FEATURE_INCOMPAT_INLINE_DATA,
    EXT4_FEATURE_INCOMPAT_CSUM_SEED, EXT4_FEATURE_RO_COMPAT_PROJECT,
    EXT4_FEATURE_RO_COMPAT_BIGALLOC, EXT4_FEATURE_INCOMPAT_EA_INODE,
    EXT4_FEATURE_INCOMPAT_DIRDATA, EXT4_FEATURE_RO_COMPAT_REPLICA,
    EXT4_FEATURE_RO_COMPAT_READONLY, EXT4_FEATURE_COMPAT_DIR_PREALLOC,
    EXT4_FEATURE_COMPAT_IMAGIC_INODES, EXT4_FEATURE_COMPAT_EXT_ATTR,
    EXT4_FEATURE_COMPAT_RESIZE_INODE, EXT4_FEATURE_COMPAT_DIR_INDEX,
    EXT4_FEATURE_COMPAT_LAZY_BG, EXT4_FEATURE_COMPAT_EXCLUDE_INODE,
    EXT4_FEATURE_COMPAT_EXCLUDE_BITMAP, EXT4_FEATURE_COMPAT_SPARSE_SUPER2,
    EXT4_FEATURE_INCOMPAT_COMPRESSION, EXT4_FEATURE_INCOMPAT_FILETYPE,
    EXT4_FEATURE_INCOMPAT_RECOVER, EXT4_FEATURE_INCOMPAT_JOURNAL_DEV,
    EXT4_FEATURE_INCOMPAT_META_BG, EXT4_FEATURE_INCOMPAT_64BIT,
    EXT4_FEATURE_INCOMPAT_MMP, EXT4_FEATURE_INCOMPAT_FLEX_BG,
    EXT4_FEATURE_INCOMPAT_EA_INODE, EXT4_FEATURE_INCOMPAT_DIRDATA,
    EXT4_FEATURE_INCOMPAT_CSUM_SEED, EXT4_FEATURE_INCOMPAT_LARGEDIR,
    EXT4_FEATURE_INCOMPAT_INLINE_DATA, EXT4_FEATURE_INCOMPAT_ENCRYPT,
    EXT4_FEATURE_RO_COMPAT_SPARSE_SUPER, EXT4_FEATURE_RO_COMPAT_LARGE_FILE,
    EXT4_FEATURE_RO_COMPAT_BTREE_DIR, EXT4_FEATURE_RO_COMPAT_HUGE_FILE,
    EXT4_FEATURE_RO_COMPAT_GDT_CSUM, EXT4_FEATURE_RO_COMPAT_DIR_NLINK,
    EXT4_FEATURE_RO_COMPAT_EXTRA_ISIZE, EXT4_FEATURE_RO_COMPAT_HAS_SNAPSHOT,
    EXT4_FEATURE_RO_COMPAT_QUOTA, EXT4_FEATURE_RO_COMPAT_BIGALLOC,
    EXT4_FEATURE_RO_COMPAT_METADATA_CSUM, EXT4_FEATURE_RO_COMPAT_REPLICA,
    EXT4_FEATURE_RO_COMPAT_READONLY, EXT4_FEATURE_RO_COMPAT_PROJECT,
    Ext4EncryptionMode, Ext4ExtentStatusFlags, Ext4DirHashVersion,
};

impl Ext4FileSystemEnhanced {
    /// Initialize inline data
    fn init_inline_data(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize inline data
        crate::println!("ext4: initializing inline data");
        Ok(())
    }

    /// Initialize checksum seed
    fn init_csum_seed(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize checksum seed
        crate::println!("ext4: initializing checksum seed");
        Ok(())
    }

    /// Initialize EA inode
    fn init_ea_inode(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize EA inode
        crate::println!("ext4: initializing EA inode");
        Ok(())
    }

    /// Initialize directory data
    fn init_dirdata(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize directory data
        crate::println!("ext4: initializing directory data");
        Ok(())
    }

    /// Initialize replica
    fn init_replica(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize replica
        crate::println!("ext4: initializing replica");
        Ok(())
    }

    /// Initialize read-only
    fn init_readonly(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize read-only
        crate::println!("ext4: initializing read-only");
        Ok(())
    }

    /// Initialize directory preallocation
    fn init_dir_prealloc(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize directory preallocation
        crate::println!("ext4: initializing directory preallocation");
        Ok(())
    }

    /// Initialize imagic inodes
    fn init_imagic_inodes(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize imagic inodes
        crate::println!("ext4: initializing imagic inodes");
        Ok(())
    }

    /// Initialize resize inode
    fn init_resize_inode(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize resize inode
        crate::println!("ext4: initializing resize inode");
        Ok(())
    }

    /// Initialize lazy block groups
    fn init_lazy_bg(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize lazy block groups
        crate::println!("ext4: initializing lazy block groups");
        Ok(())
    }

    /// Initialize exclude inode
    fn init_exclude_inode(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize exclude inode
        crate::println!("ext4: initializing exclude inode");
        Ok(())
    }

    /// Initialize exclude bitmap
    fn init_exclude_bitmap(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize exclude bitmap
        crate::println!("ext4: initializing exclude bitmap");
        Ok(())
    }

    /// Initialize sparse super2
    fn init_sparse_super2(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize sparse super2
        crate::println!("ext4: initializing sparse super2");
        Ok(())
    }

    /// Initialize compression
    fn init_compression(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize compression
        crate::println!("ext4: initializing compression");
        Ok(())
    }

    /// Initialize file type
    fn init_filetype(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize file type
        crate::println!("ext4: initializing file type");
        Ok(())
    }

    /// Initialize recover
    fn init_recover(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize recover
        crate::println!("ext4: initializing recover");
        Ok(())
    }

    /// Initialize journal device
    fn init_journal_dev(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize journal device
        crate::println!("ext4: initializing journal device");
        Ok(())
    }

    /// Initialize meta block groups
    fn init_meta_bg(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize meta block groups
        crate::println!("ext4: initializing meta block groups");
        Ok(())
    }

    /// Initialize sparse super
    fn init_sparse_super(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize sparse super
        crate::println!("ext4: initializing sparse super");
        Ok(())
    }

    /// Initialize large file
    fn init_large_file(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize large file
        crate::println!("ext4: initializing large file");
        Ok(())
    }

    /// Initialize btree directory
    fn init_btree_dir(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize btree directory
        crate::println!("ext4: initializing btree directory");
        Ok(())
    }

    /// Initialize GDT checksum
    fn init_gdt_csum(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize GDT checksum
        crate::println!("ext4: initializing GDT checksum");
        Ok(())
    }

    /// Initialize directory nlink
    fn init_dir_nlink(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize directory nlink
        crate::println!("ext4: initializing directory nlink");
        Ok(())
    }

    /// Initialize extra isize
    fn init_extra_isize(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize extra isize
        crate::println!("ext4: initializing extra isize");
        Ok(())
    }

    /// Initialize snapshot
    fn init_snapshot(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would initialize snapshot
        crate::println!("ext4: initializing snapshot");
        Ok(())
    }

    /// Update file system statistics
    fn update_stats(&mut self) -> Result<(), &'static str> {
        // In a real implementation, this would update file system statistics
        crate::println!("ext4: updating statistics");
        Ok(())
    }

    /// Read an inode from disk
    pub fn read_inode(&self, inum: u32) -> Result<crate::subsystems::fs::ext4::Ext4Inode, &'static str> {
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
        let mut inode = crate::subsystems::fs::ext4::Ext4Inode::default();

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
    pub fn write_inode(&mut self, inum: u32, inode: &crate::subsystems::fs::ext4::Ext4Inode) -> Result<(), &'static str> {
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

    /// Read data from an inode
    pub fn read_inode_data(&self, inum: u32, dst: &mut [u8], offset: u64) -> Result<usize, &'static str> {
        let inode = self.read_inode(inum)?;
        let file_size = ((inode.i_size_hi as u64) << 32) | (inode.i_size_lo as u64);
        
        if offset >= file_size {
            return Ok(0);
        }
        
        let mut total_read = 0usize;
        let mut current_offset = offset;
        let end_offset = core::cmp::min(offset + dst.len() as u64, file_size);
        
        // Check if using extents or direct/indirect blocks
        if (self.sb.s_feature_incompat & EXT4_FEATURE_INCOMPAT_EXTENTS) != 0 {
            // Using extents
            self.read_from_extents(&inode, dst, current_offset, end_offset, &mut total_read)?;
        } else {
            // Using direct/indirect blocks
            self.read_from_blocks(&inode, dst, current_offset, end_offset, &mut total_read)?;
        }
        
        Ok(total_read)
    }

    /// Write data to an inode
    pub fn write_inode_data(&mut self, inum: u32, src: &[u8], offset: u64) -> Result<usize, &'static str> {
        let mut inode = self.read_inode(inum)?;
        let file_size = ((inode.i_size_hi as u64) << 32) | (inode.i_size_lo as u64);
        
        let mut total_written = 0usize;
        let mut current_offset = offset;
        let end_offset = offset + src.len() as u64;
        
        // Check if using extents or direct/indirect blocks
        if (self.sb.s_feature_incompat & EXT4_FEATURE_INCOMPAT_EXTENTS) != 0 {
            // Using extents
            self.write_to_extents(&mut inode, src, current_offset, end_offset, &mut total_written)?;
        } else {
            // Using direct/indirect blocks
            self.write_to_blocks(&mut inode, src, current_offset, end_offset, &mut total_written)?;
        }
        
        // Update file size if we wrote past the end
        if end_offset > file_size {
            inode.i_size_lo = (end_offset & 0xFFFFFFFF) as u32;
            inode.i_size_hi = (end_offset >> 32) as u32;
        }
        
        // Write back inode
        self.write_inode(inum, &inode)?;
        
        Ok(total_written)
    }

    /// Read data using extent mapping
    fn read_from_extents(&self, inode: &crate::subsystems::fs::ext4::Ext4Inode, dst: &mut [u8], offset: u64, end_offset: u64, total_read: &mut usize) -> Result<(), &'static str> {
        // Parse extent header from i_block[0]
        let extent_header = unsafe {
            let ptr = inode.i_block.as_ptr() as *const crate::subsystems::fs::ext4::Ext4ExtentHeader;
            *ptr
        };
        
        if extent_header.eh_magic != 0xF30A {
            return Err("Invalid extent magic");
        }
        
        // For simplicity, we'll only handle leaf extents (depth = 0)
        if extent_header.eh_depth != 0 {
            return Err("Extent tree traversal not implemented");
        }
        
        // Read extents
        let extents_ptr = unsafe {
            let ptr = inode.i_block.as_ptr().add(1) as *const crate::subsystems::fs::ext4::Ext4Extent;
            ptr
        };
        
        let mut bytes_remaining = (end_offset - offset) as usize;
        let mut dst_offset = 0;
        let mut current_offset = offset;
        
        for i in 0..extent_header.eh_entries {
            if bytes_remaining == 0 {
                break;
            }
            
            let extent = unsafe { *extents_ptr.add(i as usize) };
            let extent_start = ((extent.ee_start_hi as u64) << 32) | (extent.ee_start_lo as u64);
            let extent_len = extent.ee_len as u64;
            let extent_end = extent_start + extent_len;
            
            // Check if this extent contains our offset
            if current_offset >= extent_start && current_offset < extent_end {
                let extent_offset = current_offset - extent_start;
                let bytes_to_read = core::cmp::min(
                    bytes_remaining,
                    (extent_end - current_offset) as usize
                );
                
                // Read data from extent
                let mut buf = vec![0u8; self.block_size as usize];
                let mut block_offset = extent_offset as usize;
                let mut bytes_read = 0;
                
                while bytes_read < bytes_to_read {
                    let block_idx = (extent_start + block_offset as u64) / self.block_size as u64;
                    let offset_in_block = (extent_start + block_offset as u64) % self.block_size as u64;
                    let bytes_in_block = core::cmp::min(
                        bytes_to_read - bytes_read,
                        self.block_size as usize - offset_in_block as usize
                    );
                    
                    self.dev.read(block_idx as usize, &mut buf);
                    dst[dst_offset..dst_offset + bytes_in_block].copy_from_slice(
                        &buf[offset_in_block as usize..offset_in_block as usize + bytes_in_block]
                    );
                    
                    dst_offset += bytes_in_block;
                    block_offset += bytes_in_block as u64;
                    bytes_read += bytes_in_block;
                }
                
                *total_read += bytes_read;
                bytes_remaining -= bytes_read;
                current_offset += bytes_read as u64;
            }
        }
        
        Ok(())
    }

    /// Write data using extent mapping
    fn write_to_extents(&mut self, inode: &mut crate::subsystems::fs::ext4::Ext4Inode, src: &[u8], offset: u64, end_offset: u64, total_written: &mut usize) -> Result<(), &'static str> {
        // For simplicity, we'll implement a basic version that allocates new blocks as needed
        // and creates a simple extent structure
        
        // Parse extent header from i_block[0]
        let extent_header = unsafe {
            let ptr = inode.i_block.as_ptr() as *const crate::subsystems::fs::ext4::Ext4ExtentHeader;
            *ptr
        };
        
        if extent_header.eh_magic != 0xF30A {
            // Initialize extent header
            let mut header = crate::subsystems::fs::ext4::Ext4ExtentHeader {
                eh_magic: 0xF30A,
                eh_entries: 0,
                eh_max: 4, // Max 4 extents in inode
                eh_depth: 0,
                eh_generation: 0,
            };
            
            // Write header to inode
            let header_ptr = inode.i_block.as_mut_ptr() as *mut crate::subsystems::fs::ext4::Ext4ExtentHeader;
            unsafe { *header_ptr = header; }
        }
        
        let mut bytes_remaining = (end_offset - offset) as usize;
        let mut src_offset = 0;
        let mut current_offset = offset;
        let mut extent_count = 0;
        
        while bytes_remaining > 0 && extent_count < 4 {
            // Allocate a new block
            let block_num = self.alloc_block()?;
            
            // Calculate extent parameters
            let extent_start = current_offset / self.block_size as u64 * self.block_size as u64;
            let extent_len = core::cmp::min(
                bytes_remaining as u64 / self.block_size as u64,
                32768 as u64 // Max extent length
            );
            
            // Write data to block
            let mut buf = vec![0u8; self.block_size as usize];
            let bytes_to_write = core::cmp::min(
                bytes_remaining,
                extent_len as usize * self.block_size as usize
            );
            
            // Write data in block-sized chunks
            let mut written = 0;
            while written < bytes_to_write {
                let chunk_size = core::cmp::min(bytes_to_write - written, self.block_size as usize);
                buf[..chunk_size].copy_from_slice(&src[src_offset..src_offset + chunk_size]);
                
                let block_idx = (extent_start / self.block_size as u64 + written as u64 / self.block_size as u64) as usize;
                self.dev.write(block_idx, &buf);
                
                written += chunk_size;
                src_offset += chunk_size;
            }
            
            // Create extent entry
            let extent = crate::subsystems::fs::ext4::Ext4Extent {
                ee_block: (extent_start / self.block_size as u64) as u32,
                ee_len: extent_len as u16,
                ee_start_hi: (block_num >> 16) as u16,
                ee_start_lo: (block_num & 0xFFFF) as u32,
            };
            
            // Write extent to inode
            let extents_ptr = unsafe {
                let ptr = inode.i_block.as_mut_ptr().add(1) as *mut crate::subsystems::fs::ext4::Ext4Extent;
                ptr
            };
            
            unsafe { *extents_ptr.add(extent_count as usize) = extent; }
            
            // Update counters
            extent_count += 1;
            *total_written += written;
            bytes_remaining -= written;
            current_offset += written as u64;
        }
        
        // Update extent header
        let header_ptr = unsafe {
            let ptr = inode.i_block.as_mut_ptr() as *mut crate::subsystems::fs::ext4::Ext4ExtentHeader;
            ptr
        };
        
        unsafe {
            (*header_ptr).eh_entries = extent_count;
        }
        
        Ok(())
    }

    /// Read data using direct/indirect block mapping
    fn read_from_blocks(&self, inode: &crate::subsystems::fs::ext4::Ext4Inode, dst: &mut [u8], offset: u64, end_offset: u64, total_read: &mut usize) -> Result<(), &'static str> {
        // For simplicity, we'll only implement direct blocks
        const NDIRECT: usize = 12;
        
        let mut bytes_remaining = (end_offset - offset) as usize;
        let mut dst_offset = 0;
        let mut current_offset = offset;
        
        while bytes_remaining > 0 {
            let block_idx = (current_offset / self.block_size as u64) as usize;
            
            if block_idx >= NDIRECT {
                return Err("Indirect blocks not implemented");
            }
            
            let block_num = inode.i_block[block_idx];
            if block_num == 0 {
                // Hole in file
                let hole_size = core::cmp::min(
                    bytes_remaining,
                    self.block_size as usize - (current_offset % self.block_size as u64) as usize
                );
                
                // Fill with zeros
                for i in 0..hole_size {
                    dst[dst_offset + i] = 0;
                }
                
                dst_offset += hole_size;
                bytes_remaining -= hole_size;
                current_offset += hole_size as u64;
                *total_read += hole_size;
            } else {
                // Read block
                let mut buf = vec![0u8; self.block_size as usize];
                self.dev.read(block_num as usize, &mut buf);
                
                let offset_in_block = (current_offset % self.block_size as u64) as usize;
                let bytes_to_read = core::cmp::min(
                    bytes_remaining,
                    self.block_size as usize - offset_in_block
                );
                
                dst[dst_offset..dst_offset + bytes_to_read].copy_from_slice(
                    &buf[offset_in_block..offset_in_block + bytes_to_read]
                );
                
                dst_offset += bytes_to_read;
                bytes_remaining -= bytes_to_read;
                current_offset += bytes_to_read as u64;
                *total_read += bytes_to_read;
            }
        }
        
        Ok(())
    }

    /// Write data using direct/indirect block mapping
    fn write_to_blocks(&mut self, inode: &mut crate::subsystems::fs::ext4::Ext4Inode, src: &[u8], offset: u64, end_offset: u64, total_written: &mut usize) -> Result<(), &'static str> {
        // For simplicity, we'll only implement direct blocks
        const NDIRECT: usize = 12;
        
        let mut bytes_remaining = (end_offset - offset) as usize;
        let mut src_offset = 0;
        let mut current_offset = offset;
        
        while bytes_remaining > 0 {
            let block_idx = (current_offset / self.block_size as u64) as usize;
            
            if block_idx >= NDIRECT {
                return Err("Indirect blocks not implemented");
            }
            
            // Allocate block if needed
            if inode.i_block[block_idx] == 0 {
                inode.i_block[block_idx] = self.alloc_block()?;
            }
            
            let block_num = inode.i_block[block_idx];
            
            // Write data to block
            let mut buf = vec![0u8; self.block_size as usize];
            let offset_in_block = (current_offset % self.block_size as u64) as usize;
            let bytes_to_write = core::cmp::min(
                bytes_remaining,
                self.block_size as usize - offset_in_block
            );
            
            // Read existing block if not writing full block
            if bytes_to_write < self.block_size as usize {
                self.dev.read(block_num as usize, &mut buf);
            }
            
            buf[offset_in_block..offset_in_block + bytes_to_write].copy_from_slice(
                &src[src_offset..src_offset + bytes_to_write]
            );
            
            self.dev.write(block_num as usize, &buf);
            
            src_offset += bytes_to_write;
            bytes_remaining -= bytes_to_write;
            current_offset += bytes_to_write as u64;
            *total_written += bytes_to_write;
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
    fn write_block_bitmap(&mut self, group: u32, bitmap: &[bool]) -> Result<(), crate::subsystems::fs::ext4::Ext4Error> {
        // In a real implementation, this would write the block bitmap to disk
        crate::println!("ext4: writing block bitmap for group {}", group);
        Ok(())
    }
}