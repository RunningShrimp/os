//! Ext4 File System Implementation
//!
//! This module implements the Ext4 file system core functionality for the NOS operating system.
//! Ext4 is a widely used journaling file system for Linux with features like:
//! - Large file system and file size support
//! - Extents for efficient block allocation
//! - Journaling for data integrity
//! - Flexible block allocation strategies
//! - Backward compatibility with Ext2/Ext3

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use crate::drivers::BlockDevice;
use crate::subsystems::sync::Mutex;
use crate::subsystems::fs::fs_impl::BufCache;
use core::hash::Hasher;

/// Placeholder journaling file system trait
pub trait JournalingFileSystem {
    fn begin_transaction(&self) -> u32;
    fn commit_transaction(&self, id: u32);
}

/// Placeholder journal entry
#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub block: u32,
    pub data: Vec<u8>,
}

/// Placeholder journal transaction
#[derive(Debug, Clone)]
pub struct JournalTransaction {
    pub id: u32,
    pub entries: Vec<JournalEntry>,
}

// ============================================================================
// Ext4 Constants and Structures
// ============================================================================

/// Ext4 magic number
pub const EXT4_MAGIC: u16 = 0xEF53;

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

/// Ext4 feature flags - compatible
pub const EXT4_FEATURE_COMPAT_DIR_PREALLOC: u32 = 0x0001;
pub const EXT4_FEATURE_COMPAT_IMAGIC_INODES: u32 = 0x0002;
pub const EXT4_FEATURE_COMPAT_HAS_JOURNAL: u32 = 0x0004;
pub const EXT4_FEATURE_COMPAT_EXT_ATTR: u32 = 0x0008;
pub const EXT4_FEATURE_COMPAT_RESIZE_INODE: u32 = 0x0010;
pub const EXT4_FEATURE_COMPAT_DIR_INDEX: u32 = 0x0020;
pub const EXT4_FEATURE_COMPAT_LAZY_BG: u32 = 0x0040;
pub const EXT4_FEATURE_COMPAT_EXCLUDE_INODE: u32 = 0x0080;
pub const EXT4_FEATURE_COMPAT_EXCLUDE_BITMAP: u32 = 0x0100;
pub const EXT4_FEATURE_COMPAT_SPARSE_SUPER2: u32 = 0x0200;

/// Ext4 feature flags - incompatible
pub const EXT4_FEATURE_INCOMPAT_COMPRESSION: u32 = 0x0001;
pub const EXT4_FEATURE_INCOMPAT_FILETYPE: u32 = 0x0002;
pub const EXT4_FEATURE_INCOMPAT_RECOVER: u32 = 0x0004;
pub const EXT4_FEATURE_INCOMPAT_JOURNAL_DEV: u32 = 0x0008;
pub const EXT4_FEATURE_INCOMPAT_META_BG: u32 = 0x0010;
pub const EXT4_FEATURE_INCOMPAT_EXTENTS: u32 = 0x0040;
pub const EXT4_FEATURE_INCOMPAT_64BIT: u32 = 0x0080;
pub const EXT4_FEATURE_INCOMPAT_MMP: u32 = 0x0100;
pub const EXT4_FEATURE_INCOMPAT_FLEX_BG: u32 = 0x0200;
pub const EXT4_FEATURE_INCOMPAT_EA_INODE: u32 = 0x0400;
pub const EXT4_FEATURE_INCOMPAT_DIRDATA: u32 = 0x1000;
pub const EXT4_FEATURE_INCOMPAT_CSUM_SEED: u32 = 0x2000;
pub const EXT4_FEATURE_INCOMPAT_LARGEDIR: u32 = 0x4000;
pub const EXT4_FEATURE_INCOMPAT_INLINE_DATA: u32 = 0x8000;
pub const EXT4_FEATURE_INCOMPAT_ENCRYPT: u32 = 0x10000;

/// Ext4 feature flags - read-only compatible
pub const EXT4_FEATURE_RO_COMPAT_SPARSE_SUPER: u32 = 0x0001;
pub const EXT4_FEATURE_RO_COMPAT_LARGE_FILE: u32 = 0x0002;
pub const EXT4_FEATURE_RO_COMPAT_BTREE_DIR: u32 = 0x0004;
pub const EXT4_FEATURE_RO_COMPAT_HUGE_FILE: u32 = 0x0008;
pub const EXT4_FEATURE_RO_COMPAT_GDT_CSUM: u32 = 0x0010;
pub const EXT4_FEATURE_RO_COMPAT_DIR_NLINK: u32 = 0x0020;
pub const EXT4_FEATURE_RO_COMPAT_EXTRA_ISIZE: u32 = 0x0040;
pub const EXT4_FEATURE_RO_COMPAT_HAS_SNAPSHOT: u32 = 0x0100;
pub const EXT4_FEATURE_RO_COMPAT_QUOTA: u32 = 0x0200;
pub const EXT4_FEATURE_RO_COMPAT_BIGALLOC: u32 = 0x0400;
pub const EXT4_FEATURE_RO_COMPAT_METADATA_CSUM: u32 = 0x0800;
pub const EXT4_FEATURE_RO_COMPAT_REPLICA: u32 = 0x1000;
pub const EXT4_FEATURE_RO_COMPAT_READONLY: u32 = 0x2000;
pub const EXT4_FEATURE_RO_COMPAT_PROJECT: u32 = 0x4000;

/// Ext4 encryption modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Ext4EncryptionMode {
    Invalid = 0,
    AES256XTS = 1,
    AES256GCM = 2,
    AES256CBC = 3,
    AES256CTS = 4,
}

/// Ext4 encryption context
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4EncryptionContext {
    pub mode: u8,
    pub flags: u8,
    pub master_key_descriptor: [u8; 8],
    pub nonce: [u8; 16],
}

/// Ext4 extended attribute entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4XattrEntry {
    pub e_name_len: u8,
    pub e_name_index: u8,
    pub e_value_offs: u16,
    pub e_value_block: u32,
    pub e_value_size: u32,
    pub e_hash: u32,
    pub e_name: [u8; 0],
}

/// Ext4 extended attribute header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4XattrHeader {
    pub h_magic: u32,
    pub h_refcount: u32,
    pub h_blocks: u32,
    pub h_hash: u32,
    pub h_checksum: u32,
    pub h_reserved: [u32; 3],
}

/// Ext4 quota information
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4QuotaInfo {
    pub dqb_bhardlimit: u64,
    pub dqb_bsoftlimit: u64,
    pub dqb_curspace: u64,
    pub dqb_ihardlimit: u64,
    pub dqb_isoftlimit: u64,
    pub dqb_curinodes: u64,
    pub dqb_btime: u64,
    pub dqb_itime: u64,
    pub dqb_valid: u32,
    pub dqb_pad: u32,
}

/// Ext4 project quota
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4ProjectQuota {
    pub prj_quota_id: u32,
    pub quota: Ext4QuotaInfo,
}

/// Ext4 directory hash versions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Ext4DirHashVersion {
    Legacy = 0,
    HalfMD4 = 1,
    Tea = 2,
    LegacyUnsigned = 3,
    HalfMD4Unsigned = 4,
    TeaUnsigned = 5,
}

/// Ext4 directory index entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4DirIndexEntry {
    pub hash: u32,
    pub block: u32,
}

/// Ext4 directory index root
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4DirIndexRoot {
    pub limit: u8,
    pub count: u8,
    pub current_index: u8,
    pub hash_version: u8,
    pub padding: [u8; 4],
    pub hash_seed: [u32; 4],
    pub tree_depth: u8,
    pub indirect_levels: u8,
    pub unused_flags: u16,
}

/// Ext4 directory index tail
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4DirIndexTail {
    pub dt_checksum: u32,
    pub dt_reserved: [u32; 3],
}

/// Ext4 directory index node
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4DirIndexNode {
    pub fake_inode: u32,
    pub limit: u16,
    pub count: u16,
    pub current_index: u8,
    pub hash_version: u8,
    pub padding: [u8; 6],
    pub entries: [Ext4DirIndexEntry; 0],
}

/// Ext4 extent status tree entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4ExtentStatus {
    pub es_lblk: u32,
    pub es_len: u32,
    pub es_pblk_hi: u16,
    pub es_status: u16,
    pub es_pblk_lo: u32,
}

/// Ext4 extent status flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Ext4ExtentStatusFlags {
    Written = 0x0001,
    Unwritten = 0x0002,
    Delayed = 0x0004,
    Hole = 0x0008,
}

/// Ext4 extent status tree
#[derive(Debug, Clone)]
pub struct Ext4ExtentStatusTree {
    pub root: Ext4ExtentStatus,
    pub depth: u32,
    pub count: u32,
    pub max_entries: u32,
    pub entries: Vec<Ext4ExtentStatus>,
}

/// Ext4 multi-mount protection (MMP) structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4MmpStruct {
    pub mmp_magic: u32,
    pub mmp_seq: u32,
    pub mmp_time: u64,
    pub mmp_nodename: [u8; 64],
    pub mmp_bdevname: [u8; 32],
    pub mmp_check: u32,
    pub mmp_interval: u16,
    pub mmp_pad: u16,
    pub mmp_generation: u32,
    pub mmp_reserved: [u32; 22],
}

/// Ext4 flexible block group descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4FlexBgDesc {
    pub block_bitmap: u32,
    pub inode_bitmap: u32,
    pub inode_table: u32,
    pub free_blocks: u16,
    pub free_inodes: u16,
    pub used_dirs: u16,
    pub flags: u16,
    pub exclude_bitmap: u64,
    pub block_bitmap_csum: u16,
    pub inode_bitmap_csum: u16,
    pub reserved: [u32; 3],
}

/// Ext4 checksum seed
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext4ChecksumSeed {
    pub checksum_seed: u32,
}

/// Ext4 file system statistics
#[derive(Debug, Clone)]
pub struct Ext4Stats {
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub total_inodes: u32,
    pub free_inodes: u32,
    pub directories: u32,
    pub files: u32,
    pub symlinks: u32,
    pub devices: u32,
    pub fifos: u32,
    pub sockets: u32,
    pub fragments: u64,
    pub free_fragments: u64,
    pub allocated_blocks: u64,
    pub allocated_inodes: u32,
    pub deleted_inodes: u32,
    pub orphan_inodes: u32,
    pub quota_inodes: u32,
    pub journal_inodes: u32,
    pub reserved_inodes: u32,
    pub used_blocks: u64,
    pub used_inodes: u32,
    pub reserved_blocks: u64,
    pub reserved_inodes_count: u32,
    pub system_blocks: u64,
    pub system_inodes: u32,
    pub user_blocks: u64,
    pub user_inodes: u32,
    pub group_descriptors: u32,
    pub block_groups: u32,
    pub flex_groups: u32,
    pub metadata_blocks: u64,
    pub metadata_inodes: u32,
    pub data_blocks: u64,
    pub data_inodes: u32,
    pub journal_blocks: u64,
    pub journal_inodes_count: u32,
    pub quota_blocks: u64,
    pub quota_inodes_count: u32,
    pub reserved_quota_blocks: u64,
    pub reserved_quota_inodes: u32,
    pub used_quota_blocks: u64,
    pub used_quota_inodes: u32,
    pub free_quota_blocks: u64,
    pub free_quota_inodes: u32,
    pub reserved_journal_blocks: u64,
    pub reserved_journal_inodes: u32,
    pub used_journal_blocks: u64,
    pub used_journal_inodes: u32,
    pub free_journal_blocks: u64,
    pub free_journal_inodes: u32,
    pub reserved_metadata_blocks: u64,
    pub reserved_metadata_inodes: u32,
    pub used_metadata_blocks: u64,
    pub used_metadata_inodes: u32,
    pub free_metadata_blocks: u64,
    pub free_metadata_inodes: u32,
    pub reserved_data_blocks: u64,
    pub reserved_data_inodes: u32,
    pub used_data_blocks: u64,
    pub used_data_inodes: u32,
    pub free_data_blocks: u64,
    pub free_data_inodes: u32,
    pub reserved_system_blocks: u64,
    pub reserved_system_inodes: u32,
    pub used_system_blocks: u64,
    pub used_system_inodes: u32,
    pub free_system_blocks: u64,
    pub free_system_inodes: u32,
    pub reserved_user_blocks: u64,
    pub reserved_user_inodes: u32,
    pub used_user_blocks: u64,
    pub used_user_inodes: u32,
    pub free_user_blocks: u64,
    pub free_user_inodes: u32,
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

impl Default for Ext4Stats {
    fn default() -> Self {
        Self {
            total_blocks: 0,
            free_blocks: 0,
            total_inodes: 0,
            free_inodes: 0,
            directories: 0,
            files: 0,
            symlinks: 0,
            devices: 0,
            fifos: 0,
            sockets: 0,
            fragments: 0,
            free_fragments: 0,
            allocated_blocks: 0,
            allocated_inodes: 0,
            deleted_inodes: 0,
            orphan_inodes: 0,
            quota_inodes: 0,
            journal_inodes: 0,
            reserved_inodes: 0,
            used_blocks: 0,
            used_inodes: 0,
            reserved_blocks: 0,
            reserved_inodes_count: 0,
            system_blocks: 0,
            system_inodes: 0,
            user_blocks: 0,
            user_inodes: 0,
            group_descriptors: 0,
            block_groups: 0,
            flex_groups: 0,
            metadata_blocks: 0,
            metadata_inodes: 0,
            data_blocks: 0,
            data_inodes: 0,
            journal_blocks: 0,
            journal_inodes_count: 0,
            quota_blocks: 0,
            quota_inodes_count: 0,
            reserved_quota_blocks: 0,
            reserved_quota_inodes: 0,
            used_quota_blocks: 0,
            used_quota_inodes: 0,
            free_quota_blocks: 0,
            free_quota_inodes: 0,
            reserved_journal_blocks: 0,
            reserved_journal_inodes: 0,
            used_journal_blocks: 0,
            used_journal_inodes: 0,
            free_journal_blocks: 0,
            free_journal_inodes: 0,
            reserved_metadata_blocks: 0,
            reserved_metadata_inodes: 0,
            used_metadata_blocks: 0,
            used_metadata_inodes: 0,
            free_metadata_blocks: 0,
            free_metadata_inodes: 0,
            reserved_data_blocks: 0,
            reserved_data_inodes: 0,
            used_data_blocks: 0,
            used_data_inodes: 0,
            free_data_blocks: 0,
            free_data_inodes: 0,
            reserved_system_blocks: 0,
            reserved_system_inodes: 0,
            used_system_blocks: 0,
            used_system_inodes: 0,
            free_system_blocks: 0,
            free_system_inodes: 0,
            reserved_user_blocks: 0,
            reserved_user_inodes: 0,
            used_user_blocks: 0,
            used_user_inodes: 0,
            free_user_blocks: 0,
            free_user_inodes: 0,
        }
    }
}

// ============================================================================
// Ext4 File System Implementation
// ============================================================================

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
    mount_options: Ext4MountOptions,
    journal: Option<Box<dyn JournalingFileSystem>>,
    xattr_cache: Mutex<BTreeMap<u32, BTreeMap<String, Vec<u8>>>>,
    acl_cache: Mutex<BTreeMap<u32, Vec<u8>>>,
    quota_info: Mutex<BTreeMap<u32, Ext4QuotaInfo>>,
    project_quota: Mutex<BTreeMap<u32, Ext4ProjectQuota>>,
    encryption_contexts: Mutex<BTreeMap<u32, Ext4EncryptionContext>>,
    extent_status_trees: Mutex<BTreeMap<u32, Ext4ExtentStatusTree>>,
    mmp: Option<Ext4MmpStruct>,
    stats: Mutex<Ext4Stats>,
    checksum_seed: Mutex<Ext4ChecksumSeed>,
    flex_bg_descs: Mutex<BTreeMap<u32, Ext4FlexBgDesc>>,
    dir_index_roots: Mutex<BTreeMap<u32, Ext4DirIndexRoot>>,
    dir_index_tails: Mutex<BTreeMap<u32, Ext4DirIndexTail>>,
    dir_index_nodes: Mutex<BTreeMap<u32, Ext4DirIndexNode>>,
    xattr_headers: Mutex<BTreeMap<u32, Ext4XattrHeader>>,
    xattr_entries: Mutex<BTreeMap<u32, Vec<Ext4XattrEntry>>>,
    journal_entries: Mutex<BTreeMap<u32, Vec<JournalEntry>>>,
    journal_transactions: Mutex<BTreeMap<u32, JournalTransaction>>,
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
        let blocks_per_group = self.sb.s_blocks_per_group;
        let total_blocks = self.get_total_blocks();
        self.group_count = (total_blocks + blocks_per_group - 1) / blocks_per_group;

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

    /// Read superblock from disk
    fn read_superblock(&mut self) -> Result<(), &'static str> {
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
    fn read_group_descriptors(&mut self) -> Result<(), &'static str> {
        // Calculate block group descriptor table location
        let desc_size = if self.sb.s_desc_size > 0 {
            self.sb.s_desc_size as usize
        } else {
            32 // Default size
        };

        let block_size = self.block_size as usize;
        let desc_per_block = block_size / desc_size;
        let desc_blocks = (self.group_count + desc_per_block as u32 - 1) / desc_per_block as u32;

        // Start block for group descriptor table
        let desc_start = if self.block_size == 1024 {
            2
        } else {
            1
        };

        // Read all group descriptors
        self.group_descs.clear();
        for group in 0..self.group_count {
            let desc_block = desc_start + (group / desc_per_block as u32);
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
        if (self.sb.s_feature_incompat & 0x0040) != 0 { // EXT4_FEATURE_INCOMPAT_EXTENTS
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
        if (self.sb.s_feature_incompat & 0x0040) != 0 { // EXT4_FEATURE_INCOMPAT_EXTENTS
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
    fn read_from_extents(&self, inode: &Ext4Inode, dst: &mut [u8], offset: u64, end_offset: u64, total_read: &mut usize) -> Result<(), &'static str> {
        // Parse extent header from i_block[0]
        let extent_header = unsafe {
            let ptr = inode.i_block.as_ptr() as *const Ext4ExtentHeader;
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
            let ptr = inode.i_block.as_ptr().add(1) as *const Ext4Extent;
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
    fn write_to_extents(&mut self, inode: &mut Ext4Inode, src: &[u8], offset: u64, end_offset: u64, total_written: &mut usize) -> Result<(), &'static str> {
        // For simplicity, we'll implement a basic version that allocates new blocks as needed
        // and creates a simple extent structure
        
        // Parse extent header from i_block[0]
        let extent_header = unsafe {
            let ptr = inode.i_block.as_ptr() as *const Ext4ExtentHeader;
            *ptr
        };
        
        if extent_header.eh_magic != 0xF30A {
            // Initialize extent header
            let mut header = Ext4ExtentHeader {
                eh_magic: 0xF30A,
                eh_entries: 0,
                eh_max: 4, // Max 4 extents in inode
                eh_depth: 0,
                eh_generation: 0,
            };
            
            // Write header to inode
            let header_ptr = inode.i_block.as_mut_ptr() as *mut Ext4ExtentHeader;
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
            let extent = Ext4Extent {
                ee_block: (extent_start / self.block_size as u64) as u32,
                ee_len: extent_len as u16,
                ee_start_hi: (block_num >> 16) as u16,
                ee_start_lo: (block_num & 0xFFFF) as u32,
            };
            
            // Write extent to inode
            let extents_ptr = unsafe {
                let ptr = inode.i_block.as_mut_ptr().add(1) as *mut Ext4Extent;
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
            let ptr = inode.i_block.as_mut_ptr() as *mut Ext4ExtentHeader;
            ptr
        };
        
        unsafe {
            (*header_ptr).eh_entries = extent_count;
        }
        
        Ok(())
    }

    /// Read data using direct/indirect block mapping
    fn read_from_blocks(&self, inode: &Ext4Inode, dst: &mut [u8], offset: u64, end_offset: u64, total_read: &mut usize) -> Result<(), &'static str> {
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
    fn write_to_blocks(&mut self, inode: &mut Ext4Inode, src: &[u8], offset: u64, end_offset: u64, total_written: &mut usize) -> Result<(), &'static str> {
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
}

/// Initialize Ext4 file system
pub fn init() {
    crate::println!("ext4: initializing");
    // In a real implementation, this would initialize the Ext4 file system
    crate::println!("ext4: initialized");
}