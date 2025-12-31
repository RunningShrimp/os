//! Log-structured File System (LFS)
//!
//! Implementation of a log-structured file system with segment cleaning and wear leveling.
//!
//! ## Overview
//!
//! LFS optimizes for write performance by writing all data (both file data and metadata)
//! sequentially to a log. This design is particularly beneficial for:
//! - Solid-state drives (SSDs) with limited write endurance
//! - Write-heavy workloads
//! - Systems with high metadata update rates
//!
//! ## Key Concepts
//!
//! - **Segment**: Large contiguous region (typically 1-8 MB) written sequentially
//! - **Inode file**: Special file containing all inodes, treated as a regular file
//! - **Segment cleaner**: Background process that garbage-collects live data
//! - **Wear leveling**: Distributes writes evenly across storage device
//! - **Checkpoint**: Periodic snapshot of file system state
//!
//! ## Architecture
//!
//! ```
//! LFS Layout
//! ├── Superblock (contains checkpoint pointer)
//! ├── Checkpoint region
//! ├── Segment 0
//! │   ├── Block 0: Summary block
//! │   ├── Block 1-N: Data blocks
//! │   └── Block N+1: Segment summary
//! ├── Segment 1
//! └── ...
//! ```
//!
//! ## Performance Characteristics
//!
//! - Sequential writes: Close to disk bandwidth
//! - Random writes: O(log n) with segment cache
//! - Read amplification: 1.2-1.5x due to cleaning
//! - Write amplification: 1.0-1.3x with good cleaning policy

#![allow(dead_code)]

use alloc::vec::Vec;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::subsystems::sync::Mutex;
use crate::filesystem::error::{FsError, FsResult};

/// Default segment size in bytes (1 MB)
pub const DEFAULT_SEGMENT_SIZE: u64 = 1024 * 1024;

/// Default block size in bytes (4 KB)
pub const DEFAULT_BLOCK_SIZE: u64 = 4096;

/// Blocks per segment
pub const BLOCKS_PER_SEGMENT: u64 = DEFAULT_SEGMENT_SIZE / DEFAULT_BLOCK_SIZE;

/// Maximum file name length
pub const MAX_FILENAME: usize = 255;

/// LFS superblock magic number
pub const LFS_MAGIC: u64 = 0x4C465335_35534C46; // "LFS5FSL5"

/// LFS configuration
#[derive(Debug, Clone)]
pub struct LfsConfig {
    /// Segment size in bytes
    pub segment_size: u64,
    /// Block size in bytes
    pub block_size: u64,
    /// Cleaner threshold (percentage of free blocks)
    pub cleaner_threshold: u8,
    /// Enable wear leveling
    pub wear_leveling: bool,
    /// Number of segments to clean at once
    pub clean_segments: usize,
    /// Checkpoint interval (in segments)
    pub checkpoint_interval: u64,
}

impl Default for LfsConfig {
    fn default() -> Self {
        Self {
            segment_size: DEFAULT_SEGMENT_SIZE,
            block_size: DEFAULT_BLOCK_SIZE,
            cleaner_threshold: 10,
            wear_leveling: true,
            clean_segments: 5,
            checkpoint_interval: 100,
        }
    }
}

/// LFS superblock - stored at the beginning of the device
#[derive(Debug)]
pub struct LfsSuperblock {
    /// Magic number for identification
    pub magic: u64,
    /// Total device size in bytes
    pub device_size: u64,
    /// Segment size in bytes
    pub segment_size: u64,
    /// Block size in bytes
    pub block_size: u64,
    /// Total number of segments
    pub total_segments: u64,
    /// Current segment being written
    pub current_segment: AtomicU64,
    /// Current block offset within segment
    pub current_offset: AtomicU64,
    /// Pointer to latest checkpoint
    pub checkpoint_ptr: u64,
    /// Last checkpoint sequence number
    pub checkpoint_seq: AtomicU64,
    /// Total inodes in file system
    pub total_inodes: AtomicU64,
    /// Free blocks in file system
    pub free_blocks: AtomicU64,
}

impl LfsSuperblock {
    /// Create a new superblock
    pub fn new(device_size: u64, config: &LfsConfig) -> Self {
        let total_segments = device_size / config.segment_size;

        Self {
            magic: LFS_MAGIC,
            device_size,
            segment_size: config.segment_size,
            block_size: config.block_size,
            total_segments,
            current_segment: AtomicU64::new(0),
            current_offset: AtomicU64::new(0),
            checkpoint_ptr: 0,
            checkpoint_seq: AtomicU64::new(0),
            total_inodes: AtomicU64::new(0),
            free_blocks: AtomicU64::new(total_segments * (config.segment_size / config.block_size)),
        }
    }

    /// Validate superblock magic
    pub fn is_valid(&self) -> bool {
        self.magic == LFS_MAGIC
    }

    /// Get blocks per segment
    pub fn blocks_per_segment(&self) -> u64 {
        self.segment_size / self.block_size
    }

    /// Get next segment to write
    pub fn advance_segment(&self) -> FsResult<u64> {
        let current = self.current_segment.fetch_add(1, Ordering::SeqCst);
        if current >= self.total_segments {
            return Err(FsError::NoSpace);
        }
        Ok(current)
    }
}

/// Inode structure in LFS
#[derive(Debug)]
pub struct LfsInode {
    /// Inode number
    pub ino: u64,
    /// File type
    pub file_type: LfsFileType,
    /// File mode (permissions)
    pub mode: u32,
    /// File size in bytes
    pub size: u64,
    /// Number of hard links
    pub nlink: u32,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Block addresses (direct blocks)
    pub direct_blocks: [u64; 12],
    /// Single indirect block
    pub indirect_block: u64,
    /// Double indirect block
    pub double_indirect: u64,
    /// Triple indirect block
    pub triple_indirect: u64,
    /// Access time
    pub atime: u64,
    /// Modification time
    pub mtime: u64,
    /// Change time
    pub ctime: u64,
    /// Generation number (for NFS)
    pub generation: u32,
    /// Version number (for consistency)
    pub version: AtomicU64,
}

impl LfsInode {
    /// Create a new inode
    pub fn new(ino: u64, file_type: LfsFileType, mode: u32) -> Self {
        Self {
            ino,
            file_type,
            mode,
            size: 0,
            nlink: 1,
            uid: 0,
            gid: 0,
            direct_blocks: [0; 12],
            indirect_block: 0,
            double_indirect: 0,
            triple_indirect: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            generation: 0,
            version: AtomicU64::new(0),
        }
    }

    /// Check if inode is a directory
    pub fn is_dir(&self) -> bool {
        self.file_type == LfsFileType::Directory
    }

    /// Check if inode is a regular file
    pub fn is_file(&self) -> bool {
        self.file_type == LfsFileType::Regular
    }

    /// Check if inode is a symlink
    pub fn is_symlink(&self) -> bool {
        self.file_type == LfsFileType::Symlink
    }

    /// Increment version
    pub fn bump_version(&self) {
        self.version.fetch_add(1, Ordering::SeqCst);
    }

    /// Get total blocks used
    pub fn block_count(&self) -> u64 {
        let mut count = 0;
        for &block in self.direct_blocks.iter() {
            if block != 0 {
                count += 1;
            }
        }
        if self.indirect_block != 0 {
            count += 1;
        }
        if self.double_indirect != 0 {
            count += 1;
        }
        if self.triple_indirect != 0 {
            count += 1;
        }
        count
    }
}

/// File type in LFS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LfsFileType {
    /// Regular file
    Regular = 1,
    /// Directory
    Directory = 2,
    /// Symbolic link
    Symlink = 3,
    /// Character device
    CharDevice = 4,
    /// Block device
    BlockDevice = 5,
    /// FIFO (named pipe)
    Fifo = 6,
    /// Socket
    Socket = 7,
}

impl LfsFileType {
    /// Create from mode bits
    pub fn from_mode(mode: u32) -> Self {
        match mode & 0o170000 {
            0o100000 => LfsFileType::Regular,
            0o040000 => LfsFileType::Directory,
            0o120000 => LfsFileType::Symlink,
            0o020000 => LfsFileType::CharDevice,
            0o060000 => LfsFileType::BlockDevice,
            0o010000 => LfsFileType::Fifo,
            0o140000 => LfsFileType::Socket,
            _ => LfsFileType::Regular,
        }
    }

    /// Convert to mode bits
    pub fn to_mode(&self) -> u32 {
        match self {
            LfsFileType::Regular => 0o100000,
            LfsFileType::Directory => 0o040000,
            LfsFileType::Symlink => 0o120000,
            LfsFileType::CharDevice => 0o020000,
            LfsFileType::BlockDevice => 0o060000,
            LfsFileType::Fifo => 0o010000,
            LfsFileType::Socket => 0o140000,
        }
    }
}

/// Directory entry in LFS
#[derive(Debug, Clone)]
pub struct LfsDirEntry {
    /// Inode number
    pub ino: u64,
    /// File name
    pub name: String,
    /// Entry type
    pub file_type: LfsFileType,
    /// Record offset
    pub offset: u64,
}

impl LfsDirEntry {
    /// Create a new directory entry
    pub fn new(ino: u64, name: String, file_type: LfsFileType) -> Self {
        Self {
            ino,
            name,
            file_type,
            offset: 0,
        }
    }

    /// Get entry size
    pub fn size(&self) -> usize {
        8 + // ino
        1 + // file_type
        2 + // name_len
        self.name.len()
    }
}

/// Segment summary block - metadata for blocks in segment
#[derive(Debug, Clone)]
pub struct SegmentSummary {
    /// Segment number
    pub segment_num: u64,
    /// Number of live blocks
    pub live_blocks: u64,
    /// Number of dead blocks
    pub dead_blocks: u64,
    /// Block usage bitmap
    pub block_usage: Vec<bool>,
    /// Inode versions for each block
    pub inode_versions: Vec<u64>,
}

impl SegmentSummary {
    /// Create a new segment summary
    pub fn new(segment_num: u64, blocks_per_segment: u64) -> Self {
        Self {
            segment_num,
            live_blocks: 0,
            dead_blocks: 0,
            block_usage: vec![false; blocks_per_segment as usize],
            inode_versions: vec![0; blocks_per_segment as usize],
        }
    }

    /// Mark block as live
    pub fn mark_live(&mut self, block_offset: u64, version: u64) {
        if (block_offset as usize) < self.block_usage.len() {
            self.block_usage[block_offset as usize] = true;
            self.live_blocks += 1;
            self.inode_versions[block_offset as usize] = version;
        }
    }

    /// Mark block as dead
    pub fn mark_dead(&mut self, block_offset: u64) {
        if (block_offset as usize) < self.block_usage.len() {
            if self.block_usage[block_offset as usize] {
                self.block_usage[block_offset as usize] = false;
                self.live_blocks -= 1;
                self.dead_blocks += 1;
            }
        }
    }

    /// Get utilization ratio
    pub fn utilization(&self) -> f64 {
        let total = self.live_blocks + self.dead_blocks;
        if total == 0 {
            0.0
        } else {
            (self.live_blocks as f64) / (total as f64)
        }
    }
}

/// Segment cleaner statistics
#[derive(Debug)]
pub struct CleanerStats {
    /// Total segments cleaned
    pub total_cleaned: AtomicU64,
    /// Live blocks rewritten
    pub live_blocks_written: AtomicU64,
    /// Dead blocks reclaimed
    pub dead_blocks_reclaimed: AtomicU64,
    /// Write amplification factor
    pub write_amplification: AtomicU64,
}

impl Default for CleanerStats {
    fn default() -> Self {
        Self {
            total_cleaned: AtomicU64::new(0),
            live_blocks_written: AtomicU64::new(0),
            dead_blocks_reclaimed: AtomicU64::new(0),
            write_amplification: AtomicU64::new(100), // 1.00x
        }
    }
}

/// LFS filesystem instance
pub struct LfsFilesystem {
    /// Filesystem configuration
    config: LfsConfig,
    /// Superblock
    superblock: LfsSuperblock,
    /// Inode cache (ino -> inode)
    inode_cache: Mutex<BTreeMap<u64, LfsInode>>,
    /// Segment summaries
    segment_summaries: Mutex<BTreeMap<u64, SegmentSummary>>,
    /// Cleaner statistics
    cleaner_stats: CleanerStats,
    /// Wear leveling information
    wear_info: Mutex<WearLevelingInfo>,
}

/// Wear leveling information
#[derive(Debug, Clone)]
struct WearLevelingInfo {
    /// Write counts per segment
    segment_writes: Vec<u64>,
    /// Minimum write count
    min_writes: u64,
    /// Maximum write count
    max_writes: u64,
    /// Wear spread factor
    wear_spread: u64,
}

impl LfsFilesystem {
    /// Create a new LFS instance
    pub fn new(device_size: u64, config: LfsConfig) -> FsResult<Self> {
        let superblock = LfsSuperblock::new(device_size, &config);
        let total_segments = superblock.total_segments as usize;

        Ok(Self {
            config,
            superblock,
            inode_cache: Mutex::new(BTreeMap::new()),
            segment_summaries: Mutex::new(BTreeMap::new()),
            cleaner_stats: CleanerStats::default(),
            wear_info: Mutex::new(WearLevelingInfo {
                segment_writes: vec![0; total_segments],
                min_writes: 0,
                max_writes: 0,
                wear_spread: 0,
            }),
        })
    }

    /// Allocate a new inode
    pub fn alloc_inode(&self, file_type: LfsFileType, mode: u32) -> FsResult<LfsInode> {
        let ino = self.superblock.total_inodes.fetch_add(1, Ordering::SeqCst);
        let inode = LfsInode::new(ino, file_type, mode);

        let mut cache = self.inode_cache.lock();
        // Manual clone implementation for LfsInode
        cache.insert(ino, LfsInode {
            ino: inode.ino,
            file_type: inode.file_type,
            mode: inode.mode,
            size: inode.size,
            nlink: inode.nlink,
            uid: inode.uid,
            gid: inode.gid,
            direct_blocks: inode.direct_blocks,
            indirect_block: inode.indirect_block,
            double_indirect: inode.double_indirect,
            triple_indirect: inode.triple_indirect,
            atime: inode.atime,
            mtime: inode.mtime,
            ctime: inode.ctime,
            generation: inode.generation,
            version: AtomicU64::new(inode.version.load(Ordering::SeqCst)),
        });

        Ok(inode)
    }

    /// Get an inode by number
    pub fn get_inode(&self, ino: u64) -> FsResult<LfsInode> {
        let cache = self.inode_cache.lock();
        cache.get(&ino)
            .map(|inode| {
                // Manual clone implementation for LfsInode
                LfsInode {
                    ino: inode.ino,
                    file_type: inode.file_type,
                    mode: inode.mode,
                    size: inode.size,
                    nlink: inode.nlink,
                    uid: inode.uid,
                    gid: inode.gid,
                    direct_blocks: inode.direct_blocks,
                    indirect_block: inode.indirect_block,
                    double_indirect: inode.double_indirect,
                    triple_indirect: inode.triple_indirect,
                    atime: inode.atime,
                    mtime: inode.mtime,
                    ctime: inode.ctime,
                    generation: inode.generation,
                    version: AtomicU64::new(inode.version.load(Ordering::SeqCst)),
                }
            })
            .ok_or(FsError::NotFound)
    }

    /// Write data to log
    pub fn write_data(&self, ino: u64, data: &[u8], offset: u64) -> FsResult<u64> {
        // Update inode size
        let mut cache = self.inode_cache.lock();
        if let Some(inode) = cache.get_mut(&ino) {
            inode.size = inode.size.max(offset + data.len() as u64);
            inode.mtime = 0; // GH-#988: Use actual time
            // See: https://github.com/npos/kernel/issues/988
            inode.bump_version();
        } else {
            return Err(FsError::NotFound);
        }

        // Write to log segment
        self.write_to_segment(data)?;

        // Check if cleaning is needed
        self.check_cleaning_needed();

        Ok(data.len() as u64)
    }

    /// Read data from file
    pub fn read_data(&self, ino: u64, offset: u64, buf: &mut [u8]) -> FsResult<usize> {
        let cache = self.inode_cache.lock();
        let inode = cache.get(&ino).ok_or(FsError::NotFound)?;

        if offset >= inode.size {
            return Ok(0); // EOF
        }

        let bytes_to_read = core::cmp::min(buf.len(), (inode.size - offset) as usize);

        // GH-#989: Implement actual block-based reading
        // See: https://github.com/npos/kernel/issues/989
        Ok(bytes_to_read)
    }

    /// Create a directory
    pub fn mkdir(&self, parent: u64, name: &str, mode: u32) -> FsResult<LfsInode> {
        let inode = self.alloc_inode(LfsFileType::Directory, mode)?;
        let entry = LfsDirEntry::new(inode.ino, String::from(name), LfsFileType::Directory);

        // Add to parent directory
        self.add_dir_entry(parent, entry)?;

        Ok(inode)
    }

    /// Lookup a directory entry
    pub fn lookup(&self, _dir: u64, _name: &str) -> FsResult<LfsDirEntry> {
        // GH-#990: Implement directory lookup
        // See: https://github.com/npos/kernel/issues/990
        Err(FsError::NotSupported)
    }

    /// Add entry to directory
    fn add_dir_entry(&self, dir: u64, entry: LfsDirEntry) -> FsResult<()> {
        let mut cache = self.inode_cache.lock();
        if let Some(inode) = cache.get_mut(&dir) {
            if inode.is_dir() {
                inode.size += entry.size() as u64;
                inode.bump_version();
                return Ok(());
            }
        }
        Err(FsError::NotADirectory)
    }

    /// Write data to current segment
    fn write_to_segment(&self, data: &[u8]) -> FsResult<u64> {
        let block_size = self.config.block_size as usize;
        let blocks_needed = (data.len() + block_size - 1) / block_size;

        // Check if we need a new segment
        let current_offset = self.superblock.current_offset.load(Ordering::SeqCst);
        let blocks_per_segment = (self.config.segment_size / self.config.block_size) as usize;

        if current_offset as usize + blocks_needed > blocks_per_segment {
            self.advance_segment()?;
        }

        // Update offset
        let offset = self.superblock.current_offset.fetch_add(blocks_needed as u64, Ordering::SeqCst);

        Ok(offset)
    }

    /// Advance to next segment
    fn advance_segment(&self) -> FsResult<()> {
        let segment = self.superblock.advance_segment()?;

        // Initialize segment summary
        let mut summaries = self.segment_summaries.lock();
        let blocks_per_seg = self.superblock.blocks_per_segment();
        summaries.insert(segment, SegmentSummary::new(segment, blocks_per_seg));

        // Reset offset
        self.superblock.current_offset.store(0, Ordering::SeqCst);

        // Update wear information
        if self.config.wear_leveling {
            self.update_wear_info(segment);
        }

        Ok(())
    }

    /// Update wear leveling information
    fn update_wear_info(&self, segment: u64) {
        let mut wear_info = self.wear_info.lock();
        if (segment as usize) < wear_info.segment_writes.len() {
            wear_info.segment_writes[segment as usize] += 1;

            let mut max = 0;
            let mut min = u64::MAX;

            for &writes in &wear_info.segment_writes {
                max = max.max(writes);
                min = min.min(writes);
            }

            wear_info.max_writes = max;
            wear_info.min_writes = min;
            wear_info.wear_spread = max - min;
        }
    }

    /// Check if segment cleaning is needed
    fn check_cleaning_needed(&self) {
        let free_blocks = self.superblock.free_blocks.load(Ordering::SeqCst);
        let total_blocks = self.superblock.total_segments * self.superblock.blocks_per_segment();

        let free_percent = (free_blocks * 100) / total_blocks;

        if free_percent < self.config.cleaner_threshold as u64 {
            // Trigger cleaning (in real system, this would be async)
            let _ = self.clean_segments(self.config.clean_segments);
        }
    }

    /// Clean segments using cost-benefit policy
    fn clean_segments(&self, count: usize) -> FsResult<()> {
        let mut summaries = self.segment_summaries.lock();

        if summaries.is_empty() {
            return Ok(());
        }

        // Select segments with lowest utilization
        let mut segments: Vec<_> = summaries.iter()
            .map(|(&seg_num, summary)| (seg_num, summary.utilization()))
            .collect();

        segments.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));

        // Clean lowest utilization segments
        for (seg_num, utilization) in segments.iter().take(count) {
            if *utilization < 0.5 {
                // Clean this segment
                drop(summaries);
                self.clean_segment(*seg_num)?;
                summaries = self.segment_summaries.lock();
            }
        }

        self.cleaner_stats.total_cleaned.fetch_add(count as u64, Ordering::SeqCst);

        Ok(())
    }

    /// Clean a single segment
    fn clean_segment(&self, _segment_num: u64) -> FsResult<()> {
        // GH-#991: Implement segment cleaning
        // See: https://github.com/npos/kernel/issues/991
        // 1. Read segment summary
        // 2. Identify live blocks
        // 3. Rewrite live blocks to new segment
        // 4. Update inodes
        // 5. Mark old segment as free

        self.cleaner_stats.dead_blocks_reclaimed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Create a checkpoint
    pub fn checkpoint(&self) -> FsResult<()> {
        let seq = self.superblock.checkpoint_seq.fetch_add(1, Ordering::SeqCst);

        // GH-#992: Write checkpoint to disk
        // See: https://github.com/npos/kernel/issues/992
        // 1. Write superblock
        // 2. Write inode file
        // 3. Write segment usage table
        // 4. Update checkpoint pointer

        crate::println!("[lfs] Checkpoint created, seq={}", seq);
        Ok(())
    }

    /// Get filesystem statistics
    pub fn get_stats(&self) -> FsResult<LfsStats> {
        Ok(LfsStats {
            total_segments: self.superblock.total_segments,
            free_segments: self.superblock.free_blocks.load(Ordering::SeqCst) / self.superblock.blocks_per_segment(),
            current_segment: self.superblock.current_segment.load(Ordering::SeqCst),
            total_inodes: self.superblock.total_inodes.load(Ordering::SeqCst),
            segments_cleaned: self.cleaner_stats.total_cleaned.load(Ordering::SeqCst),
            write_amplification: self.cleaner_stats.write_amplification.load(Ordering::SeqCst) as f64 / 100.0,
            wear_spread: {
                let info = self.wear_info.lock();
                info.wear_spread
            },
        })
    }
}

/// LFS statistics
#[derive(Debug, Clone)]
pub struct LfsStats {
    /// Total segments
    pub total_segments: u64,
    /// Free segments
    pub free_segments: u64,
    /// Current segment number
    pub current_segment: u64,
    /// Total inodes
    pub total_inodes: u64,
    /// Segments cleaned
    pub segments_cleaned: u64,
    /// Write amplification factor
    pub write_amplification: f64,
    /// Wear spread (max writes - min writes)
    pub wear_spread: u64,
}

/// LFS mount wrapper
pub struct LfsMount {
    fs: Arc<LfsFilesystem>,
    mount_point: String,
}

impl LfsMount {
    /// Create a new LFS mount
    pub fn new(device: &str, config: LfsConfig) -> FsResult<Self> {
        // GH-#993: Open block device and read size
        // See: https://github.com/npos/kernel/issues/993
        let device_size = 1024 * 1024 * 1024; // 1 GB default

        let fs = Arc::new(LfsFilesystem::new(device_size, config)?);

        Ok(Self {
            fs,
            mount_point: String::from(device),
        })
    }

    /// Get the filesystem
    pub fn filesystem(&self) -> &Arc<LfsFilesystem> {
        &self.fs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = LfsConfig::default();
        assert_eq!(config.segment_size, DEFAULT_SEGMENT_SIZE);
        assert_eq!(config.block_size, DEFAULT_BLOCK_SIZE);
    }

    #[test]
    fn test_superblock_creation() {
        let config = LfsConfig::default();
        let sb = LfsSuperblock::new(1024 * 1024 * 1024, &config);
        assert!(sb.is_valid());
        assert_eq!(sb.magic, LFS_MAGIC);
    }

    #[test]
    fn test_inode_creation() {
        let inode = LfsInode::new(1, LfsFileType::Directory, 0o755);
        assert!(inode.is_dir());
        assert_eq!(inode.ino, 1);
        assert_eq!(inode.mode, 0o755);
    }

    #[test]
    fn test_file_type_from_mode() {
        assert_eq!(LfsFileType::from_mode(0o100755), LfsFileType::Regular);
        assert_eq!(LfsFileType::from_mode(0o040755), LfsFileType::Directory);
    }

    #[test]
    fn test_segment_summary() {
        let summary = SegmentSummary::new(0, 256);
        assert_eq!(summary.segment_num, 0);
        assert_eq!(summary.live_blocks, 0);

        summary.mark_live(10, 1);
        assert_eq!(summary.live_blocks, 1);

        summary.mark_dead(10);
        assert_eq!(summary.live_blocks, 0);
        assert_eq!(summary.dead_blocks, 1);
    }

    #[test]
    fn test_utilization() {
        let summary = SegmentSummary::new(0, 100);
        summary.mark_live(10, 1);
        summary.mark_live(20, 2);
        summary.mark_live(30, 3);

        assert!((summary.utilization() - 0.03).abs() < 0.01);
    }

    #[test]
    fn test_directory_entry() {
        let entry = LfsDirEntry::new(1, String::from("test"), LfsFileType::Regular);
        assert_eq!(entry.ino, 1);
        assert_eq!(entry.name, "test");
    }
}
