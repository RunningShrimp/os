# FS (File System) Implementation Layer

## Overview

The FS subsystem provides concrete implementations of file system algorithms and structures that are used by the VFS abstraction layer.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                VFS Abstraction Layer                 │
│  - Provides common interfaces (FileSystemType,       │
│    SuperBlock, Inode)                           │
└────────────────────┬────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│           FS Implementation Layer                  │
│                                                   │
│  ┌─────────────────────────────────────────────┐    │
│  │   API Layer (api/)                      │    │
│  │   - FileOperations                       │    │
│  │   - DirectoryOperations                  │    │
│  │   - PathOperations                      │    │
│  └─────────────────────────────────────────────┘    │
│                                                   │
│  ┌─────────────────────────────────────────────┐    │
│  │   Concrete Implementations                │    │
│  │   - Ext2 (simple persistent)           │    │
│  │   - Ext4 (journaling)                 │    │
│  │   - XV6-style FS                      │    │
│  └─────────────────────────────────────────────┘    │
│                                                   │
│  ┌─────────────────────────────────────────────┐    │
│  │   Advanced Features                       │    │
│  │   - Multi-level cache                   │    │
│  │   - Journaling                          │    │
│  │   - Recovery                           │    │
│  │   - File permissions                   │    │
│  │   - File locking                       │    │
│  └─────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

## Module Structure

| Module | Description |
|--------|-------------|
| `mod.rs` | Main FS module, contains `VfsManager` |
| `api/traits.rs` | Core API traits (FileOperations, DirectoryOperations, PathOperations) |
| `api/error.rs` | FS-specific error types |
| `api/types.rs` | API data types (FileHandle, DirEntry, FileAttr) |
| `ext2.rs` | EXT2 file system implementation |
| `ext4.rs` | EXT4 file system implementation |
| `ext4_persistence.rs` | EXT4 on-disk structures |
| `fs_cache.rs` | Multi-level file system cache |
| `fs_impl.rs` | XV6-style file system |
| `file.rs` | File abstraction layer |
| `file_permissions.rs` | File permissions and ACLs |
| `file_locking.rs` | File locking mechanisms |
| `journaling_fs.rs` | Journaling file system support |
| `journaling_wrapper.rs` | Journaling wrapper layer |
| `journaling_enhanced.rs` | Enhanced journaling features |
| `recovery.rs` | Crash recovery mechanisms |

## Key Components

### VfsManager

Global VFS manager that coordinates filesystems:

```rust
pub struct VfsManager {
    fs_types: Mutex<BTreeMap<String, Arc<dyn FileSystemType>>>,
    mounts: Mutex<BTreeMap<String, Arc<Mount>>>,
    root_mounted: Mutex<Option<Arc<Mount>>>,
}

impl VfsManager {
    pub fn register_fs(&self, name: &str, fs_type: Arc<dyn FileSystemType>);
    pub fn mount(&self, fs_type: &str, mount_point: &str, dev: Option<String>, flags: u32) -> Result<(), VfsError>;
    pub fn unmount(&self, mount_point: &str) -> Result<(), VfsError>;
    pub fn verify_root(&self) -> Result<(), VfsError>;
}
```

### API Traits

#### FileOperations

Operations on file handles:

```rust
pub trait FileOperations {
    fn read(&mut self, offset: u64, buf: &mut [u8]) -> Result<usize, FsError>;
    fn write(&mut self, offset: u64, buf: &[u8]) -> Result<usize, FsError>;
    fn size(&self) -> u64;
    fn truncate(&mut self, size: u64) -> Result<(), FsError>;
    fn sync(&mut self) -> Result<(), FsError>;
}
```

#### DirectoryOperations

Operations on directories:

```rust
pub trait DirectoryOperations {
    fn create_subdir(&mut self, name: &str) -> Result<(), FsError>;
    fn remove_subdir(&mut self, name: &str) -> Result<(), FsError>;
    fn list_entries(&self) -> Result<Vec<DirEntry>, FsError>;
    fn find_entry(&self, name: &str) -> Result<Option<DirEntry>, FsError>;
}
```

#### PathOperations

Path manipulation and resolution:

```rust
pub trait PathOperations {
    fn parse(path: &str) -> Result<Vec<PathComponent>, FsError>;
    fn normalize(path: &str) -> Result<String, FsError>;
    fn validate(path: &str) -> Result<(), FsError>;
    fn join(base: &str, relative: &str) -> Result<String, FsError>;
}
```

## File System Implementations

### EXT2

Simple, reliable file system:

- Block size: 1024 or 4096 bytes
- Inode size: 128 bytes
- Max file size: 2TB (with 4KB blocks)
- Features: Directories, regular files, symbolic links
- Location: `ext2.rs`

### EXT4

Journaling file system with advanced features:

- Block size: 1024, 2048, or 4096 bytes
- Inode size: 256 bytes
- Max file size: 16TB (with extents)
- Features: Extents, journaling, large directories, timestamps
- Location: `ext4.rs`, `ext4_persistence.rs`

### XV6-style FS

Simple teaching file system:

- Block size: 1024 bytes
- Direct pointers: 12
- Single indirect: 256 blocks
- Double indirect: 256*256 blocks
- Max file size: 8MB (with 1KB blocks)
- Location: `fs_impl.rs`

## Advanced Features

### File System Cache (`fs_cache.rs`)

Multi-level cache with configurable eviction policies:

```rust
pub enum CacheLevel {
    L1(CacheConfig),
    L2(CacheConfig),
    L3(CacheConfig),
}

pub enum EvictionPolicy {
    LRU,
    LFU,
    FIFO,
    Random,
    Clock,
    ARC,
    TwoQueue,
}

pub struct FsCache {
    levels: Vec<CacheLevel>,
    stats: CacheStats,
}
```

Supported entry types:
- DataBlock
- MetadataBlock
- DirectoryEntry
- Inode
- IndirectBlock
- ExtendedAttribute
- JournalEntry
- BitmapBlock
- Superblock
- GroupDescriptor

### Journaling (`journaling_fs.rs`)

Crash recovery through write-ahead logging:

```rust
pub enum JournalEntryType {
    Begin,
    Update,
    Commit,
    Checkpoint,
    Revoke,
}

pub struct JournalTransaction {
    entries: Vec<JournalEntry>,
    state: TransactionState,
}

pub enum TransactionState {
    Inactive,
    Active,
    Prepared,
    Committed,
    Aborted,
}
```

### Recovery (`recovery.rs`)

System state recovery after crashes:

```rust
pub enum RecoveryEvent {
    SystemStartup,
    SystemShutdown,
    FsMount,
    FsUnmount,
    CheckpointStart,
    CheckpointComplete,
    SnapshotCreate,
    SnapshotRestore,
    RecoveryOperation,
}

pub struct RecoveryManager {
    events: Vec<RecoveryEvent>,
    snapshots: Vec<Snapshot>,
}
```

### File Permissions (`file_permissions.rs`)

Access control with ACLs:

```rust
pub struct AclEntry {
    acl_type: AclType,
    uid: Option<u32>,
    gid: Option<u32>,
    permissions: FileMode,
}

pub enum AclType {
    User,
    Group,
    Other,
    Mask,
}
```

### File Locking (`file_locking.rs``

Advisory and mandatory file locking:

```rust
pub enum LockType {
    ReadShared,
    WriteExclusive,
}

pub struct LockRange {
    offset: u64,
    length: u64,
    lock_type: LockType,
}

pub struct FileLockManager {
    locks: Mutex<Vec<LockRange>>,
    wait_queue: Mutex<Vec<LockRequest>>,
}
```

## Error Handling

All FS operations return `Result<T, FsError>`:

```rust
pub enum FsError {
    PathNotFound,
    PermissionDenied,
    FileExists,
    NotADirectory,
    IsADirectory,
    InvalidPath,
    IoError,
    NoSpace,
    ReadOnly,
    Busy,
    NotSupported,
}
```

## Integration

### VFS Integration

FS implementations register with VFS manager:

```rust
// Register filesystem types
vfs_manager.register_fs("ext2", Arc::new(Ext2FsType::new()));
vfs_manager.register_fs("ext4", Arc::new(Ext4FsType::new()));

// Mount filesystem
vfs_manager.mount("ext4", "/mnt/data", Some("/dev/sda1"), 0)?;
```

### System Call Integration

FS operations are called via system calls:

```rust
// File operations
sys_open(path, flags, mode)?;
sys_read(fd, buf)?;
sys_write(fd, buf)?;
sys_close(fd)?;

// Directory operations
sys_mkdir(path, mode)?;
sys_rmdir(path)?;
sys_readdir(fd, entries)?;

// Metadata operations
sys_stat(path, &stat)?;
sys_fstat(fd, &stat)?;
sys_chmod(path, mode)?;
```

## Future Improvements

- [ ] Implement path resolution in PathOperations
- [ ] Add NFS (Network File System) support
- [ ] Implement Btrfs (copy-on-write)
- [ ] Add ZFS support
- [ ] Implement FUSE (User-space filesystem)
- [ ] Add quota management
- [ ] Implement snapshotting for all FS types
- [ ] Add deduplication support
- [ ] Implement compression
- [ ] Add encryption at rest
