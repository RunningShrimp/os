//! File System Type Definitions
//!
//! Unified type definitions for file system operations

/// Block size for file system
pub const BSIZE: u32 = 1024;

/// Maximum number of inodes
pub const NINODE: usize = 200;

/// Number of direct block pointers
pub const NDIRECT: usize = 12;

/// Inode pointers per block
pub const IPB: usize = BSIZE as usize / 8;

/// Maximum directory entry size
pub const DIRSIZ: usize = 14;

/// Root inode number
pub const ROOTINO: usize = 1;

/// File system magic number
pub const FS_MAGIC: u32 = 0x10203040;

/// Inode type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeType {
    File,
    Directory,
    Device,
    Socket,
    Symlink,
    Unknown,
}

/// Disk inode structure
#[derive(Debug, Clone)]
pub struct DiskInode {
    pub file_type: InodeType,
    pub major: u16,
    pub minor: u16,
    pub nlink: u16,
    pub size: u32,
    pub blocks: [u32; NDIRECT + 1],
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct Dirent {
    pub inum: u16,
    pub name: [u8; DIRSIZ],
}

/// Buffer flags
#[derive(Debug, Clone, Copy)]
pub struct BufFlags;

/// Super block structure
#[derive(Debug, Clone)]
pub struct SuperBlock {
    pub size: u32,
    pub nblocks: u32,
    pub ninodes: u32,
    pub nlog: u32,
    pub logstart: u32,
    pub inodestart: u32,
    pub bmapstart: u32,
}

/// File system structure
pub struct Fs {
    pub super_block: SuperBlock,
}

/// Inode structure
pub struct Inode {
    pub inum: u32,
    pub disk_inode: DiskInode,
}

/// Buffer cache structure
pub struct BufCache;

/// Get file system instance
pub fn get_fs() -> &'static Fs {
    static FS_INSTANCE: Fs = Fs {
        super_block: SuperBlock {
            size: 0,
            nblocks: 0,
            ninodes: 0,
            nlog: 0,
            logstart: 0,
            inodestart: 0,
            bmapstart: 0,
        },
    };
    &FS_INSTANCE
}

/// Get JFS wrapper
pub fn get_jfs_wrapper() -> Option<&'static Fs> {
    Some(get_fs())
}

/// Journal statistics
#[derive(Debug, Clone)]
pub struct JournalStats {
    pub transactions: u64,
    pub commits: u64,
}
