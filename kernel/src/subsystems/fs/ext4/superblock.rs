//! Ext4 Superblock Management
//!
//! 提供Ext4超级块的读取、写入和管理功能

extern crate alloc;
use crate::subsystems::fs::ext4::Ext4FileSystem;

/// Ext4 file system state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Ext4State {
    Clean = 1,
    Errors = 2,
    OrphanRecovery = 3,
}

/// Ext4 error handling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Ext4Errors {
    Continue = 1,
    RemountRo = 2,
    Panic = 3,
}

/// Ext4 superblock structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4SuperBlock {
    /// Total number of inodes in file system
    pub s_inodes_count: u32,
    /// Total number of blocks in file system
    pub s_blocks_count_lo: u32,
    /// Number of reserved blocks for superuser
    pub s_r_blocks_count_lo: u32,
    /// Number of free blocks
    pub s_free_blocks_count_lo: u32,
    /// Number of free inodes
    pub s_free_inodes_count: u32,
    /// First data block
    pub s_first_data_block: u32,
    /// Block size (log2)
    pub s_log_block_size: u32,
    /// Fragment size (log2)
    pub s_log_frag_size: u32,
    /// Number of blocks per group
    pub s_blocks_per_group: u32,
    /// Number of fragments per group
    pub s_frags_per_group: u32,
    /// Number of inodes per group
    pub s_inodes_per_group: u32,
    /// Mount time
    pub s_mtime: u32,
    /// Write time
    pub s_wtime: u32,
    /// Number of mounts since last fsck
    pub s_mnt_count: u16,
    /// Maximum number of mounts before fsck
    pub s_max_mnt_count: u16,
    /// Magic number
    pub s_magic: u16,
    /// File system state
    pub s_state: u16,
    /// Error handling policy
    pub s_errors: u16,
    /// Minor revision level
    pub s_minor_rev_level: u16,
    /// Time of last fsck
    pub s_lastcheck: u32,
    /// Maximum time between fscks
    pub s_checkinterval: u32,
    /// Creator OS
    pub s_creator_os: u32,
    /// Revision level
    pub s_rev_level: u32,
    /// Default reserved UID
    pub s_def_resuid: u16,
    /// Default reserved GID
    pub s_def_resgid: u16,
    /// First non-reserved inode
    pub s_first_ino: u32,
    /// Size of inode structure
    pub s_inode_size: u16,
    /// Block group descriptor size
    pub s_desc_size: u16,
    /// Compatible feature flags
    pub s_feature_compat: u32,
    /// Incompatible feature flags
    pub s_feature_incompat: u32,
    /// Read-only compatible feature flags
    pub s_feature_ro_compat: u32,
    /// UUID of file system
    pub s_uuid: [u8; 16],
    /// Volume name
    pub s_volume_name: [u8; 16],
    /// Directory where last mounted
    pub s_last_mounted: [u8; 64],
    /// Algorithm usage bitmap
    pub s_algorithm_usage_bitmap: u32,
    /// Preallocation blocks
    pub s_prealloc_blocks: u8,
    /// Preallocation directory blocks
    pub s_prealloc_dir_blocks: u8,
    /// Reserved GDT blocks
    pub s_reserved_gdt_blocks: u16,
    /// Journal UUID
    pub s_journal_uuid: [u8; 16],
    /// Journal inode number
    pub s_journal_inum: u32,
    /// Journal device
    pub s_journal_dev: u32,
    /// Last orphan inode
    pub s_last_orphan: u32,
    /// Hash seed for directory indices
    pub s_hash_seed: [u32; 4],
    /// Default hash version
    pub s_def_hash_version: u8,
    /// Journal backup type
    pub s_jnl_backup_type: u8,
    /// Size of descriptor groups
    pub s_desc_size_backup: u16,
    /// Default mount options
    pub s_default_mount_opts: u32,
    /// First metablock block group
    pub s_first_meta_bg: u32,
    /// MKFS time
    pub s_mkfs_time: u32,
    /// Journal backup blocks
    pub s_jnl_blocks: [u32; 17],
    /// Total number of blocks (high 32 bits)
    pub s_blocks_count_hi: u32,
    /// Reserved blocks (high 32 bits)
    pub s_r_blocks_count_hi: u32,
    /// Free blocks (high 32 bits)
    pub s_free_blocks_count_hi: u32,
    /// Number of inodes (high 16 bits)
    pub s_inodes_count_hi: u16,
    /// Project quota enabled
    pub s_proj_quota: u16,
    /// Padding
    pub s_padding: [u32; 107],
}

impl Default for Ext4SuperBlock {
    fn default() -> Self {
        Self {
            s_inodes_count: 0,
            s_blocks_count_lo: 0,
            s_r_blocks_count_lo: 0,
            s_free_blocks_count_lo: 0,
            s_free_inodes_count: 0,
            s_first_data_block: 0,
            s_log_block_size: 0,
            s_log_frag_size: 0,
            s_blocks_per_group: 0,
            s_frags_per_group: 0,
            s_inodes_per_group: 0,
            s_mtime: 0,
            s_wtime: 0,
            s_mnt_count: 0,
            s_max_mnt_count: 0,
            s_magic: 0,
            s_state: 0,
            s_errors: 0,
            s_minor_rev_level: 0,
            s_lastcheck: 0,
            s_checkinterval: 0,
            s_creator_os: 0,
            s_rev_level: 0,
            s_def_resuid: 0,
            s_def_resgid: 0,
            s_first_ino: 11, // Default first non-reserved inode
            s_inode_size: 128, // Default inode size
            s_desc_size: 32, // Default descriptor size
            s_feature_compat: 0,
            s_feature_incompat: 0,
            s_feature_ro_compat: 0,
            s_uuid: [0; 16],
            s_volume_name: [0; 16],
            s_last_mounted: [0; 64],
            s_algorithm_usage_bitmap: 0,
            s_prealloc_blocks: 0,
            s_prealloc_dir_blocks: 0,
            s_reserved_gdt_blocks: 0,
            s_journal_uuid: [0; 16],
            s_journal_inum: 0,
            s_journal_dev: 0,
            s_last_orphan: 0,
            s_hash_seed: [0; 4],
            s_def_hash_version: 0,
            s_jnl_backup_type: 0,
            s_desc_size_backup: 0,
            s_default_mount_opts: 0,
            s_first_meta_bg: 0,
            s_mkfs_time: 0,
            s_jnl_blocks: [0; 17],
            s_blocks_count_hi: 0,
            s_r_blocks_count_hi: 0,
            s_free_blocks_count_hi: 0,
            s_inodes_count_hi: 0,
            s_proj_quota: 0,
            s_padding: [0; 107],
        }
    }
}

/// Ext4 block group descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4GroupDesc {
    /// Block bitmap block
    pub bg_block_bitmap: u32,
    /// Inode bitmap block
    pub bg_inode_bitmap: u32,
    /// Starting inode table block
    pub bg_inode_table: u32,
    /// Number of free blocks
    pub bg_free_blocks_count: u16,
    /// Number of free inodes
    pub bg_free_inodes_count: u16,
    /// Number of used directories
    pub bg_used_dirs_count: u16,
    /// Padding
    pub bg_pad: u16,
    /// Reserved for future use
    pub bg_reserved: [u32; 3],
}

impl Default for Ext4GroupDesc {
    fn default() -> Self {
        Self {
            bg_block_bitmap: 0,
            bg_inode_bitmap: 0,
            bg_inode_table: 0,
            bg_free_blocks_count: 0,
            bg_free_inodes_count: 0,
            bg_used_dirs_count: 0,
            bg_pad: 0,
            bg_reserved: [0; 3],
        }
    }
}

/// Ext4 file system mount options
#[derive(Debug, Clone)]
pub struct Ext4MountOptions {
    pub read_only: bool,
    pub noatime: bool,
    pub nodiratime: bool,
    pub relatime: bool,
    pub strictatime: bool,
    pub data_journaling: bool,
    pub data_ordered: bool,
    pub data_writeback: bool,
    pub user_xattr: bool,
    pub acl: bool,
    pub usrquota: bool,
    pub grpquota: bool,
    pub prjquota: bool,
    pub barrier: bool,
    pub nobarrier: bool,
    pub block_size: u32,
    pub inode_size: u32,
    pub journal_size: u32,
    pub checksum: bool,
    pub encrypt: bool,
    pub casefold: bool,
    pub project: bool,
    pub largedir: bool,
    pub inline_data: bool,
    pub metadata_csum: bool,
    pub _64bit: bool,
    pub flex_bg: bool,
    pub sparse_super: bool,
    pub huge_file: bool,
    pub bigalloc: bool,
    pub quota: bool,
    pub mmp: bool,
    pub dir_index: bool,
    pub ext_attr: bool,
    pub journal: bool,
    pub recover: bool,
    pub compression: bool,
    pub filetype: bool,
    pub meta_bg: bool,
    pub extents: bool,
    pub write_policy: Ext4WritePolicy,
}

/// Runtime write policy for balancing latency vs durability.
#[derive(Debug, Clone, Copy)]
pub enum Ext4WritePolicy {
    Balanced,
    LatencyOptimized,
    Durability,
}

impl Default for Ext4MountOptions {
    fn default() -> Self {
        Self {
            read_only: false,
            noatime: false,
            nodiratime: false,
            relatime: false,
            strictatime: false,
            data_journaling: false,
            data_ordered: true,
            data_writeback: false,
            user_xattr: true,
            acl: true,
            usrquota: false,
            grpquota: false,
            prjquota: false,
            barrier: true,
            nobarrier: false,
            block_size: 4096,
            inode_size: 256,
            journal_size: 0,
            checksum: true,
            encrypt: false,
            casefold: false,
            project: false,
            largedir: false,
            inline_data: false,
            metadata_csum: true,
            _64bit: true,
            flex_bg: true,
            sparse_super: true,
            huge_file: true,
            bigalloc: false,
            quota: false,
            mmp: false,
            dir_index: true,
            ext_attr: true,
            journal: true,
            recover: true,
            compression: false,
            filetype: true,
            meta_bg: true,
            extents: true,
            write_policy: Ext4WritePolicy::Balanced,
        }
    }
}

impl Ext4FileSystem {
    /// Read superblock from disk
    pub fn read_superblock(&mut self) -> Result<(), &'static str> {
        let mut buf = [0u8; 1024];
        self.dev.read(1, &mut buf); // Superblock is at block 1

        // Parse superblock
        self.sb.s_inodes_count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        self.sb.s_blocks_count_lo = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        self.sb.s_r_blocks_count_lo = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        self.sb.s_free_blocks_count_lo = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
        self.sb.s_free_inodes_count = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);
        self.sb.s_first_data_block = u32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]);
        self.sb.s_log_block_size = u32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]);
        self.sb.s_log_frag_size = u32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]);
        self.sb.s_blocks_per_group = u32::from_le_bytes([buf[32], buf[33], buf[34], buf[35]]);
        self.sb.s_frags_per_group = u32::from_le_bytes([buf[36], buf[37], buf[38], buf[39]]);
        self.sb.s_inodes_per_group = u32::from_le_bytes([buf[40], buf[41], buf[42], buf[43]]);
        self.sb.s_mtime = u32::from_le_bytes([buf[44], buf[45], buf[46], buf[47]]);
        self.sb.s_wtime = u32::from_le_bytes([buf[48], buf[49], buf[50], buf[51]]);
        self.sb.s_mnt_count = u16::from_le_bytes([buf[52], buf[53]]);
        self.sb.s_max_mnt_count = u16::from_le_bytes([buf[54], buf[55]]);
        self.sb.s_magic = u16::from_le_bytes([buf[56], buf[57]]);
        self.sb.s_state = u16::from_le_bytes([buf[58], buf[59]]);
        self.sb.s_errors = u16::from_le_bytes([buf[60], buf[61]]);
        self.sb.s_minor_rev_level = u16::from_le_bytes([buf[62], buf[63]]);
        self.sb.s_lastcheck = u32::from_le_bytes([buf[64], buf[65], buf[66], buf[67]]);
        self.sb.s_checkinterval = u32::from_le_bytes([buf[68], buf[69], buf[70], buf[71]]);
        self.sb.s_creator_os = u32::from_le_bytes([buf[72], buf[73], buf[74], buf[75]]);
        self.sb.s_rev_level = u32::from_le_bytes([buf[76], buf[77], buf[78], buf[79]]);
        self.sb.s_def_resuid = u16::from_le_bytes([buf[80], buf[81]]);
        self.sb.s_def_resgid = u16::from_le_bytes([buf[82], buf[83]]);
        self.sb.s_first_ino = u32::from_le_bytes([buf[84], buf[85], buf[86], buf[87]]);
        self.sb.s_inode_size = u16::from_le_bytes([buf[88], buf[89]]);
        self.sb.s_desc_size = u16::from_le_bytes([buf[90], buf[91]]);
        self.sb.s_feature_compat = u32::from_le_bytes([buf[92], buf[93], buf[94], buf[95]]);
        self.sb.s_feature_incompat = u32::from_le_bytes([buf[96], buf[97], buf[98], buf[99]]);
        self.sb.s_feature_ro_compat = u32::from_le_bytes([buf[100], buf[101], buf[102], buf[103]]);

        // Copy UUID
        self.sb.s_uuid.copy_from_slice(&buf[104..120]);

        // Copy volume name
        self.sb.s_volume_name.copy_from_slice(&buf[120..136]);

        // Copy last mounted path
        self.sb.s_last_mounted.copy_from_slice(&buf[136..200]);

        // Continue parsing remaining fields...
        self.sb.s_algorithm_usage_bitmap = u32::from_le_bytes([buf[200], buf[201], buf[202], buf[203]]);
        self.sb.s_prealloc_blocks = buf[204];
        self.sb.s_prealloc_dir_blocks = buf[205];
        self.sb.s_reserved_gdt_blocks = u16::from_le_bytes([buf[206], buf[207]]);

        // Copy journal UUID
        self.sb.s_journal_uuid.copy_from_slice(&buf[208..224]);

        self.sb.s_journal_inum = u32::from_le_bytes([buf[224], buf[225], buf[226], buf[227]]);
        self.sb.s_journal_dev = u32::from_le_bytes([buf[228], buf[229], buf[230], buf[231]]);
        self.sb.s_last_orphan = u32::from_le_bytes([buf[232], buf[233], buf[234], buf[235]]);

        // Copy hash seed
        for i in 0..4 {
            self.sb.s_hash_seed[i] = u32::from_le_bytes([
                buf[236 + i * 4],
                buf[237 + i * 4],
                buf[238 + i * 4],
                buf[239 + i * 4],
            ]);
        }

        self.sb.s_def_hash_version = buf[252];
        self.sb.s_jnl_backup_type = buf[253];
        self.sb.s_desc_size_backup = u16::from_le_bytes([buf[254], buf[255]]);

        self.sb.s_default_mount_opts = u32::from_le_bytes([buf[256], buf[257], buf[258], buf[259]]);
        self.sb.s_first_meta_bg = u32::from_le_bytes([buf[260], buf[261], buf[262], buf[263]]);
        self.sb.s_mkfs_time = u32::from_le_bytes([buf[264], buf[265], buf[266], buf[267]]);

        // Copy journal backup blocks
        for i in 0..17 {
            self.sb.s_jnl_blocks[i] = u32::from_le_bytes([
                buf[268 + i * 4],
                buf[269 + i * 4],
                buf[270 + i * 4],
                buf[271 + i * 4],
            ]);
        }

        self.sb.s_blocks_count_hi = u32::from_le_bytes([buf[340], buf[341], buf[342], buf[343]]);
        self.sb.s_r_blocks_count_hi = u32::from_le_bytes([buf[344], buf[345], buf[346], buf[347]]);
        self.sb.s_free_blocks_count_hi = u32::from_le_bytes([buf[348], buf[349], buf[350], buf[351]]);
        self.sb.s_inodes_count_hi = u16::from_le_bytes([buf[352], buf[353]]);
        self.sb.s_proj_quota = u16::from_le_bytes([buf[354], buf[355]]);

        Ok(())
    }

    /// Read block group descriptors from disk
    pub fn read_group_descriptors(&mut self) -> Result<(), &'static str> {
        // Calculate block group descriptor table location
        let desc_size = if self.sb.s_desc_size > 0 {
            self.sb.s_desc_size as usize
        } else {
            32 // Default size
        };

        let block_size = self.block_size as usize;
        let desc_per_block = block_size / desc_size;
        let desc_blocks = (self.group_count + desc_per_block as u32 - 1) / desc_per_block as u32;

        // Validate that we have enough block groups to read the descriptor table
        if desc_blocks == 0 {
            return Err("Invalid descriptor block count");
        }

        // Start block for group descriptor table
        let desc_start = if self.block_size == 1024 {
            2
        } else {
            1
        };

        // Calculate end block to prevent reading beyond the descriptor table
        let desc_end = desc_start + desc_blocks;

        // Read all group descriptors
        self.group_descs.clear();
        for group in 0..self.group_count {
            let desc_block = desc_start + (group / desc_per_block as u32);

            // Validate that we're reading within the descriptor table bounds
            if desc_block >= desc_end {
                return Err("Block group descriptor out of bounds");
            }

            let desc_offset = (group % desc_per_block as u32) * desc_size as u32;

            // Read block containing descriptor
            let mut buf = vec![0u8; block_size];
            self.dev.read(desc_block as usize, &mut buf);

            // Parse descriptor
            let offset = desc_offset as usize;
            let mut desc = Ext4GroupDesc::default();

            desc.bg_block_bitmap = u32::from_le_bytes([
                buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3],
            ]);
            desc.bg_inode_bitmap = u32::from_le_bytes([
                buf[offset + 4], buf[offset + 5], buf[offset + 6], buf[offset + 7],
            ]);
            desc.bg_inode_table = u32::from_le_bytes([
                buf[offset + 8], buf[offset + 9], buf[offset + 10], buf[offset + 11],
            ]);
            desc.bg_free_blocks_count = u16::from_le_bytes([
                buf[offset + 12], buf[offset + 13],
            ]);
            desc.bg_free_inodes_count = u16::from_le_bytes([
                buf[offset + 14], buf[offset + 15],
            ]);
            desc.bg_used_dirs_count = u16::from_le_bytes([
                buf[offset + 16], buf[offset + 17],
            ]);
            desc.bg_pad = u16::from_le_bytes([buf[offset + 18], buf[offset + 19]]);

            // Read reserved fields if available
            if desc_size >= 32 {
                for i in 0..3 {
                    desc.bg_reserved[i] = u32::from_le_bytes([
                        buf[offset + 20 + i * 4],
                        buf[offset + 21 + i * 4],
                        buf[offset + 22 + i * 4],
                        buf[offset + 23 + i * 4],
                    ]);
                }
            }

            self.group_descs.push(desc);
        }

        Ok(())
    }
}
