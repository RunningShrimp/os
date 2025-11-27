//! File system implementation for xv6-rust
//! Implements xv6-compatible simple file system

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use crate::drivers::{BlockDevice, RamDisk};
use crate::sync::{Sleeplock, Mutex};

/// Block size in bytes
pub const BSIZE: usize = 1024;

/// Inodes per block
pub const IPB: usize = BSIZE / core::mem::size_of::<DiskInode>();

/// Directory entries per block
pub const DPB: usize = BSIZE / core::mem::size_of::<Dirent>();

/// Direct block pointers in inode
pub const NDIRECT: usize = 12;

/// Indirect block pointer
pub const NINDIRECT: usize = BSIZE / core::mem::size_of::<u32>();

/// Maximum file size (in blocks)
pub const MAXFILE: usize = NDIRECT + NINDIRECT;

/// Maximum path length
pub const MAXPATH: usize = 128;

/// Directory entry name length
pub const DIRSIZ: usize = 14;

/// Root inode number
pub const ROOTINO: u32 = 1;

/// File system magic number
pub const FS_MAGIC: u32 = 0x10203040;

// ============================================================================
// On-disk structures
// ============================================================================

/// Superblock structure
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct SuperBlock {
    pub magic: u32,       // Must be FS_MAGIC
    pub size: u32,        // Size of file system image (blocks)
    pub nblocks: u32,     // Number of data blocks
    pub ninodes: u32,     // Number of inodes
    pub nlog: u32,        // Number of log blocks
    pub logstart: u32,    // Block number of first log block
    pub inodestart: u32,  // Block number of first inode block
    pub bmapstart: u32,   // Block number of first free map block
}

/// Inode type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum InodeType {
    Free = 0,
    Dir = 1,
    File = 2,
    Device = 3,
}

impl Default for InodeType {
    fn default() -> Self {
        Self::Free
    }
}

impl From<u16> for InodeType {
    fn from(v: u16) -> Self {
        match v {
            1 => Self::Dir,
            2 => Self::File,
            3 => Self::Device,
            _ => Self::Free,
        }
    }
}

/// On-disk inode structure
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct DiskInode {
    pub itype: u16,              // File type
    pub major: i16,              // Major device number (for T_DEVICE)
    pub minor: i16,              // Minor device number (for T_DEVICE)
    pub nlink: i16,              // Number of links to inode
    pub size: u32,               // Size of file (bytes)
    pub addrs: [u32; NDIRECT + 1], // Data block addresses
}

/// Directory entry
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Dirent {
    pub inum: u16,               // Inode number
    pub name: [u8; DIRSIZ],      // File name
}

impl Dirent {
    pub fn name_str(&self) -> &str {
        let len = self.name.iter().position(|&c| c == 0).unwrap_or(DIRSIZ);
        core::str::from_utf8(&self.name[..len]).unwrap_or("")
    }

    pub fn set_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let len = bytes.len().min(DIRSIZ);
        self.name[..len].copy_from_slice(&bytes[..len]);
        if len < DIRSIZ {
            self.name[len..].fill(0);
        }
    }
}

// ============================================================================
// Buffer cache
// ============================================================================

/// Number of buffer cache entries
pub const NBUF: usize = 30;

/// Buffer flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufFlags(u8);

impl BufFlags {
    pub const VALID: Self = Self(1 << 0);
    pub const DIRTY: Self = Self(1 << 1);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn set(&mut self, flag: Self) {
        self.0 |= flag.0;
    }

    pub fn clear(&mut self, flag: Self) {
        self.0 &= !flag.0;
    }
}

/// Buffer cache entry
pub struct Buf {
    pub flags: BufFlags,
    pub dev: u32,
    pub blockno: u32,
    pub refcnt: u32,
    pub data: [u8; BSIZE],
}

impl Default for Buf {
    fn default() -> Self {
        Self {
            flags: BufFlags::empty(),
            dev: 0,
            blockno: 0,
            refcnt: 0,
            data: [0; BSIZE],
        }
    }
}

/// Buffer cache
pub struct BufCache {
    bufs: [Sleeplock<Buf>; NBUF],
}

impl BufCache {
    pub const fn new() -> Self {
        const EMPTY_BUF: Sleeplock<Buf> = Sleeplock::new(Buf {
            flags: BufFlags::empty(),
            dev: 0,
            blockno: 0,
            refcnt: 0,
            data: [0; BSIZE],
        });
        Self {
            bufs: [EMPTY_BUF; NBUF],
        }
    }

    /// Get a buffer for the given block, reading from disk if necessary
    pub fn bread(&self, dev: &impl BlockDevice, blockno: u32) -> Option<usize> {
        // First, try to find the block in cache
        for (i, buf_lock) in self.bufs.iter().enumerate() {
            let buf = buf_lock.lock();
            if buf.dev == 0 && buf.blockno == blockno && buf.flags.contains(BufFlags::VALID) {
                drop(buf);
                return Some(i);
            }
        }

        // Not found, allocate a buffer
        for (i, buf_lock) in self.bufs.iter().enumerate() {
            let mut buf = buf_lock.lock();
            if buf.refcnt == 0 {
                buf.dev = 0;
                buf.blockno = blockno;
                buf.flags = BufFlags::empty();
                buf.refcnt = 1;

                // Read from disk
                let offset = (blockno as usize) * BSIZE / 512;
                for j in 0..(BSIZE / 512) {
                    dev.read(offset + j, &mut buf.data[j * 512..(j + 1) * 512]);
                }
                buf.flags.set(BufFlags::VALID);
                
                drop(buf);
                return Some(i);
            }
        }

        None // No buffers available
    }

    /// Write buffer to disk
    pub fn bwrite(&self, dev: &impl BlockDevice, idx: usize) {
        let buf = self.bufs[idx].lock();
        let offset = (buf.blockno as usize) * BSIZE / 512;
        for j in 0..(BSIZE / 512) {
            dev.write(offset + j, &buf.data[j * 512..(j + 1) * 512]);
        }
    }

    /// Release a buffer
    pub fn brelse(&self, idx: usize) {
        let mut buf = self.bufs[idx].lock();
        buf.refcnt = buf.refcnt.saturating_sub(1);
    }
}

// ============================================================================
// In-memory inode
// ============================================================================

/// In-memory inode
pub struct Inode {
    pub dev: u32,        // Device number
    pub inum: u32,       // Inode number
    pub ref_count: i32,  // Reference count
    pub valid: bool,     // Has been read from disk?
    
    // Copy of disk inode
    pub itype: InodeType,
    pub major: i16,
    pub minor: i16,
    pub nlink: i16,
    pub size: u32,
    pub addrs: [u32; NDIRECT + 1],
}

impl Default for Inode {
    fn default() -> Self {
        Self {
            dev: 0,
            inum: 0,
            ref_count: 0,
            valid: false,
            itype: InodeType::Free,
            major: 0,
            minor: 0,
            nlink: 0,
            size: 0,
            addrs: [0; NDIRECT + 1],
        }
    }
}

impl Inode {
    /// Read data from inode
    pub fn read(&self, _dev: &impl BlockDevice, _dst: &mut [u8], _off: usize) -> usize {
        // TODO: Implement inode read
        0
    }

    /// Write data to inode
    pub fn write(&mut self, _dev: &impl BlockDevice, _src: &[u8], _off: usize) -> usize {
        // TODO: Implement inode write
        0
    }
}

// ============================================================================
// File system
// ============================================================================

/// Inode cache size
pub const NINODE: usize = 50;

/// File system state
pub struct Fs {
    dev: RamDisk,
    sb: SuperBlock,
    buf_cache: BufCache,
    inodes: Mutex<[Inode; NINODE]>,
}

impl Fs {
    pub fn new() -> Self {
        Self {
            dev: RamDisk,
            sb: SuperBlock::default(),
            buf_cache: BufCache::new(),
            inodes: Mutex::new([const { Inode {
                dev: 0,
                inum: 0,
                ref_count: 0,
                valid: false,
                itype: InodeType::Free,
                major: 0,
                minor: 0,
                nlink: 0,
                size: 0,
                addrs: [0; NDIRECT + 1],
            } }; NINODE]),
        }
    }

    /// Read superblock from disk
    pub fn read_super(&self) -> SuperBlock {
        let mut buf = [0u8; 512];
        self.dev.read(1, &mut buf); // Superblock is at block 1

        SuperBlock {
            magic: u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
            size: u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
            nblocks: u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
            ninodes: u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]),
            nlog: u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]),
            logstart: u32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]),
            inodestart: u32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]),
            bmapstart: u32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]),
        }
    }

    /// Write superblock to disk
    pub fn write_super(&self, sb: &SuperBlock) {
        let mut buf = [0u8; 512];
        buf[0..4].copy_from_slice(&sb.magic.to_le_bytes());
        buf[4..8].copy_from_slice(&sb.size.to_le_bytes());
        buf[8..12].copy_from_slice(&sb.nblocks.to_le_bytes());
        buf[12..16].copy_from_slice(&sb.ninodes.to_le_bytes());
        buf[16..20].copy_from_slice(&sb.nlog.to_le_bytes());
        buf[20..24].copy_from_slice(&sb.logstart.to_le_bytes());
        buf[24..28].copy_from_slice(&sb.inodestart.to_le_bytes());
        buf[28..32].copy_from_slice(&sb.bmapstart.to_le_bytes());
        self.dev.write(1, &buf);
    }

    /// Initialize file system
    pub fn init(&mut self) -> bool {
        self.sb = self.read_super();
        if self.sb.magic != FS_MAGIC {
            crate::println!("fs: invalid filesystem (bad magic)");
            return false;
        }
        crate::println!("fs: {} blocks, {} inodes", self.sb.nblocks, self.sb.ninodes);
        true
    }

    /// Allocate an inode
    pub fn ialloc(&self, itype: InodeType) -> Option<u32> {
        for inum in 1..self.sb.ninodes {
            let block = self.sb.inodestart + inum / (IPB as u32);
            let offset = (inum % (IPB as u32)) as usize * core::mem::size_of::<DiskInode>();
            
            // Read block containing inode
            let buf_idx = self.buf_cache.bread(&self.dev, block)?;
            let buf = self.buf_cache.bufs[buf_idx].lock();
            
            // Check if inode is free
            let disk_inode_type = u16::from_le_bytes([buf.data[offset], buf.data[offset + 1]]);
            
            if disk_inode_type == 0 {
                // Found free inode, initialize it
                drop(buf);
                
                let mut buf = self.buf_cache.bufs[buf_idx].lock();
                buf.data[offset..offset + 2].copy_from_slice(&(itype as u16).to_le_bytes());
                buf.flags.set(BufFlags::DIRTY);
                drop(buf);
                
                self.buf_cache.bwrite(&self.dev, buf_idx);
                self.buf_cache.brelse(buf_idx);
                
                return Some(inum);
            }
            
            drop(buf);
            self.buf_cache.brelse(buf_idx);
        }
        None
    }

    /// Get inode by number
    pub fn iget(&self, inum: u32) -> Option<usize> {
        let mut inodes = self.inodes.lock();
        
        // First, look for cached inode
        for (i, inode) in inodes.iter_mut().enumerate() {
            if inode.ref_count > 0 && inode.inum == inum {
                inode.ref_count += 1;
                return Some(i);
            }
        }
        
        // Not found, allocate new entry
        for (i, inode) in inodes.iter_mut().enumerate() {
            if inode.ref_count == 0 {
                inode.inum = inum;
                inode.ref_count = 1;
                inode.valid = false;
                return Some(i);
            }
        }
        
        None
    }

    /// Put (release) inode
    pub fn iput(&self, idx: usize) {
        let mut inodes = self.inodes.lock();
        if let Some(inode) = inodes.get_mut(idx) {
            inode.ref_count -= 1;
            if inode.ref_count == 0 && inode.nlink == 0 {
                // Truncate and free inode
                // TODO: Implement truncate
                inode.itype = InodeType::Free;
            }
        }
    }

    /// Look up directory entry
    pub fn dirlookup(&self, _dir_inum: u32, _name: &str) -> Option<u32> {
        // TODO: Implement directory lookup
        None
    }

    /// Create a new directory entry
    pub fn dirlink(&self, _dir_inum: u32, _name: &str, _inum: u32) -> bool {
        // TODO: Implement directory link
        false
    }

    /// List directory contents
    pub fn list_dir(&self, dir_inum: u32) -> Vec<(String, u32)> {
        let mut entries = Vec::new();
        
        // TODO: Read directory entries
        let _ = dir_inum;
        
        entries
    }

    /// List root directory
    pub fn list_root(&self) -> Vec<Inode> {
        Vec::new()
    }

    /// Create file system on device
    pub fn mkfs(&self) {
        // Initialize superblock
        let sb = SuperBlock {
            magic: FS_MAGIC,
            size: 1000,
            nblocks: 900,
            ninodes: 200,
            nlog: 30,
            logstart: 2,
            inodestart: 32,
            bmapstart: 58,
        };
        self.write_super(&sb);
        
        // Zero out all blocks
        let zero_block = [0u8; 512];
        for i in 0..100 {
            self.dev.write(i, &zero_block);
        }
        
        // Create root directory inode
        // TODO: Initialize root directory
        
        crate::println!("fs: created new filesystem");
    }
}

/// Global file system instance
static mut FS: Option<Fs> = None;

/// Initialize file system
pub fn init() {
    unsafe {
        let mut fs = Fs::new();
        if !fs.init() {
            crate::println!("fs: creating new filesystem");
            fs.mkfs();
            fs.init();
        }
        FS = Some(fs);
    }
    crate::println!("fs: initialized");
}

/// Get file system instance
pub fn get_fs() -> Option<&'static Fs> {
    unsafe { (*core::ptr::addr_of!(FS)).as_ref() }
}
