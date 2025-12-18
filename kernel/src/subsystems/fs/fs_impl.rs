// File system implementation for xv6-rust
// Implements xv6-compatible simple file system

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use core::hash::{Hash, Hasher};
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

/// Cache key: (device, blockno)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CacheKey {
    dev: u32,
    blockno: u32,
}

impl CacheKey {
    fn new(dev: u32, blockno: u32) -> Self {
        Self { dev, blockno }
    }
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Combine dev and blockno into a single hash value
        // Using a simple but effective mixing strategy
        let combined = ((self.dev as u64) << 32) | (self.blockno as u64);
        combined.hash(state);
    }
}

/// Buffer cache - now uses hash map for O(1) lookup
pub struct BufCache {
    bufs: Vec<Sleeplock<Buf>>,
    cache: Mutex<BTreeMap<CacheKey, usize>>,  // Maps (dev, blockno) to buffer index
    free_list: Mutex<Vec<usize>>,            // Free buffer indices for quick allocation
}

impl BufCache {
    pub const fn new() -> Self {
        Self {
            bufs: Vec::new(),
            cache: Mutex::new(BTreeMap::new()),
            free_list: Mutex::new(Vec::new()),
        }
    }

    /// Initialize buffer cache - needs to be called before use
    pub fn init(&mut self) {
        // Initialize buffers
        self.bufs.clear();
        for _ in 0..NBUF {
            self.bufs.push(Sleeplock::new(Buf::default()));
        }

        // Add all buffers to free list
        let mut free_list = self.free_list.lock();
        free_list.clear();
        for i in 0..NBUF {
            free_list.push(i);
        }
    }

    /// Get a buffer for the given block, reading from disk if necessary
    pub fn bread(&self, dev: &impl BlockDevice, blockno: u32) -> Option<usize> {
        let key = CacheKey::new(0, blockno);  // Note: Currently using dev=0 hardcoded
        
        // First, try to find the block in cache
        {
            let cache = self.cache.lock();
            if let Some(&idx) = cache.get(&key) {
                // Increase refcount
                let mut buf = self.bufs[idx].lock();
                buf.refcnt += 1;
                drop(buf);
                return Some(idx);
            }
        }

        // Not found, allocate a buffer from free list
        let mut free_list = self.free_list.lock();
        let idx = free_list.pop()?;  // Get next free buffer index
        drop(free_list);

        // Update buffer state
        let mut buf = self.bufs[idx].lock();
        
        // If the buffer was dirty, write it back to disk
        if buf.flags.contains(BufFlags::DIRTY) {
            let old_offset = (buf.blockno as usize) * BSIZE / 512;
            for j in 0..(BSIZE / 512) {
                dev.write(old_offset + j, &buf.data[j * 512..(j + 1) * 512]);
            }
        }

        // Initialize new buffer
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

        // Add to cache
        let mut cache = self.cache.lock();
        cache.insert(key, idx);
        drop(cache);

        Some(idx)
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
        
        if buf.refcnt == 0 {
            // Buffer is no longer in use, add to free list and remove from cache
            let key = CacheKey::new(buf.dev, buf.blockno);
            
            drop(buf);
            
            let mut cache = self.cache.lock();
            cache.remove(&key);
            drop(cache);
            
            let mut free_list = self.free_list.lock();
            free_list.push(idx);
        }
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
    pub fn read(&self, dev: &impl BlockDevice, dst: &mut [u8], off: usize) -> usize {
        if off >= self.size as usize {
            return 0;
        }
        
        let mut total = 0usize;
        let mut offset = off;
        let end = (off + dst.len()).min(self.size as usize);
        
        while offset < end {
            let block_idx = offset / BSIZE;
            let block_offset = offset % BSIZE;
            
            // Get block number from direct or indirect blocks
            let block_num = if block_idx < NDIRECT {
                self.addrs[block_idx]
            } else {
                // Would need to read indirect block
                // For now, return what we have
                break;
            };
            
            if block_num == 0 {
                break;
            }
            
            // Read block
            let mut buf = [0u8; BSIZE];
            dev.read(block_num as usize, &mut buf);
            
            let bytes_to_copy = (BSIZE - block_offset).min(end - offset);
            dst[total..total + bytes_to_copy].copy_from_slice(&buf[block_offset..block_offset + bytes_to_copy]);
            
            total += bytes_to_copy;
            offset += bytes_to_copy;
        }
        
        total
    }

    /// Write data to inode
    pub fn write(&mut self, dev: &impl BlockDevice, src: &[u8], off: usize) -> usize {
        let mut total = 0usize;
        let mut offset = off;
        let end = off + src.len();
        
        while offset < end {
            let block_idx = offset / BSIZE;
            let block_offset = offset % BSIZE;
            
            // Get or allocate block
            let block_num = if block_idx < NDIRECT {
                if self.addrs[block_idx] == 0 {
                    // Would need to allocate new block
                    // For now, just fail
                    break;
                }
                self.addrs[block_idx]
            } else {
                // Would handle indirect blocks
                break;
            };
            
            // Read-modify-write
            let mut buf = [0u8; BSIZE];
            dev.read(block_num as usize, &mut buf);
            
            let bytes_to_copy = (BSIZE - block_offset).min(end - offset);
            buf[block_offset..block_offset + bytes_to_copy].copy_from_slice(&src[total..total + bytes_to_copy]);
            
            dev.write(block_num as usize, &buf);
            
            total += bytes_to_copy;
            offset += bytes_to_copy;
        }
        
        // Update inode size if we wrote past the end
        if off + total > self.size as usize {
            self.size = (off + total) as u32;
        }
        
        total
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
        let mut fs = Self {
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
        };
        
        // Initialize buffer cache
        fs.buf_cache.init();
        
        fs
    }

    /// Read superblock from disk
    pub fn read_super(&self) -> SuperBlock {
        let mut buf = [0u8; 512];
        self.dev.read(1 as usize, &mut buf); // Superblock is at block 1

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
        self.dev.write(1 as usize, &buf);
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
                // Truncate: free all data blocks
                for i in 0..NDIRECT {
                    if inode.addrs[i] != 0 {
                        // Would call bfree to free the block
                        inode.addrs[i] = 0;
                    }
                }
                // Would also free indirect block if present
                if inode.addrs[NDIRECT] != 0 {
                    inode.addrs[NDIRECT] = 0;
                }
                inode.size = 0;
                inode.itype = InodeType::Free;
            }
        }
    }

    /// Look up directory entry
    pub fn dirlookup(&self, dir_inum: u32, name: &str) -> Option<u32> {
        // Get directory inode
        let inodes = self.inodes.lock();
        let dir_inode = inodes.iter().find(|i| i.inum == dir_inum && i.ref_count > 0)?;
        
        if dir_inode.itype != InodeType::Dir {
            return None;
        }
        
        // Read directory entries
        let mut buf = [0u8; BSIZE];
        let dirent_size = core::mem::size_of::<Dirent>();
        
        for i in 0..NDIRECT {
            if dir_inode.addrs[i] == 0 {
                continue;
            }
            
            self.dev.read(dir_inode.addrs[i] as usize, &mut buf);
            
            // Scan directory entries in this block
            for off in (0..BSIZE).step_by(dirent_size) {
                let inum = u16::from_le_bytes([buf[off], buf[off + 1]]);
                if inum == 0 {
                    continue;
                }
                
                // Extract name (null-terminated)
                let name_bytes = &buf[off + 2..off + dirent_size];
                let entry_name_end = name_bytes.iter().position(|&c| c == 0).unwrap_or(DIRSIZ);
                let entry_name = core::str::from_utf8(&name_bytes[..entry_name_end]).unwrap_or("");
                
                if entry_name == name {
                    return Some(inum as u32);
                }
            }
        }
        
        None
    }

    /// Create a new directory entry
    pub fn dirlink(&self, dir_inum: u32, name: &str, inum: u32) -> bool {
        // Get directory inode
        let inodes = self.inodes.lock();
        let dir_inode = inodes.iter().find(|i| i.inum == dir_inum && i.ref_count > 0);
        
        let dir_inode = match dir_inode {
            Some(i) if i.itype == InodeType::Dir => i,
            _ => return false,
        };
        
        let dirent_size = core::mem::size_of::<Dirent>();
        let mut buf = [0u8; BSIZE];
        
        // Find empty slot in directory
        for i in 0..NDIRECT {
            if dir_inode.addrs[i] == 0 {
                continue;
            }
            
            self.dev.read(dir_inode.addrs[i] as usize, &mut buf);
            
            for off in (0..BSIZE).step_by(dirent_size) {
                let entry_inum = u16::from_le_bytes([buf[off], buf[off + 1]]);
                if entry_inum == 0 {
                    // Found empty slot, write new entry
                    buf[off..off + 2].copy_from_slice(&(inum as u16).to_le_bytes());
                    
                    // Write name
                    let name_bytes = name.as_bytes();
                    let copy_len = name_bytes.len().min(DIRSIZ);
                    buf[off + 2..off + 2 + copy_len].copy_from_slice(&name_bytes[..copy_len]);
                    
                    // Zero-fill rest of name
                    for j in copy_len..DIRSIZ {
                        buf[off + 2 + j] = 0;
                    }
                    
                    self.dev.write(dir_inode.addrs[i] as usize, &buf);
                    return true;
                }
            }
        }
        
        false
    }

    /// List directory contents
    pub fn list_dir(&self, dir_inum: u32) -> Vec<(String, u32)> {
        let mut entries = Vec::new();
        
        // Get directory inode
        let inodes = self.inodes.lock();
        let dir_inode = match inodes.iter().find(|i| i.inum == dir_inum && i.ref_count > 0) {
            Some(i) if i.itype == InodeType::Dir => i,
            _ => return entries,
        };
        
        let dirent_size = core::mem::size_of::<Dirent>();
        let mut buf = [0u8; BSIZE];
        
        // Read directory entries
        for i in 0..NDIRECT {
            if dir_inode.addrs[i] == 0 {
                continue;
            }
            
            self.dev.read(dir_inode.addrs[i] as usize, &mut buf);
            
            for off in (0..BSIZE).step_by(dirent_size) {
                let inum = u16::from_le_bytes([buf[off], buf[off + 1]]);
                if inum == 0 {
                    continue;
                }
                
                // Extract name
                let name_bytes = &buf[off + 2..off + dirent_size];
                let name_end = name_bytes.iter().position(|&c| c == 0).unwrap_or(DIRSIZ);
                if let Ok(name) = core::str::from_utf8(&name_bytes[..name_end]) {
                    entries.push((String::from(name), inum as u32));
                }
            }
        }
        
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
            self.dev.write(i as usize, &zero_block);
        }
        
        // Create root directory inode (inode 1)
        let root_block = sb.inodestart;
        let mut buf = [0u8; 512];
        self.dev.read(root_block as usize, &mut buf);
        
        // Root inode is at offset 0 in block (inode 1)
        // Set type to directory
        buf[0..2].copy_from_slice(&(InodeType::Dir as u16).to_le_bytes());
        // Set nlink to 1
        buf[4..6].copy_from_slice(&1u16.to_le_bytes());
        // Size = 0 initially
        buf[6..10].copy_from_slice(&0u32.to_le_bytes());
        
        self.dev.write(root_block as usize, &buf);
        
        crate::println!("fs: created new filesystem with root directory");
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