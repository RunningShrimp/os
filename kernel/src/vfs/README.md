# VFS (Virtual File System) Module

## Overview

The VFS layer provides unified file system abstractions for the kernel, allowing multiple file system implementations to coexist and be accessed through a common interface.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              System Call Layer (syscalls)            │
└────────────────────┬────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│                  VFS Abstraction Layer              │
│  - Common file/directory/inode interfaces          │
│  - Mount management                              │
│  - Path resolution                              │
│  - Dentry cache (directory entry cache)           │
└────────────────────┬────────────────────────────────┘
                     │
        ┌────────────┴────────────┐
        ▼                         ▼
┌──────────────┐      ┌──────────────┐
│   RamFS      │      │    TmpFS     │
│  (in-memory) │      │ (limited)    │
└──────────────┘      └──────────────┘
        │                     │
        ▼                     ▼
┌──────────────────────────────────────┐
│         FS Implementation Layer      │
│  - Ext4 (journaling)           │
│  - Ext2 (simple)              │
│  - SysFS (kernel info)         │
│  - ProcFS (process info)       │
└──────────────────────────────────────┘
```

## Module Structure

| Module | Description |
|--------|-------------|
| `mod.rs` | Main VFS module, exports public API |
| `types.rs` | Core VFS types (FileType, FileMode, FileAttr, SeekWhence) |
| `error.rs` | VFS-specific error types |
| `mount.rs` | Mount point management |
| `dentry.rs` | Directory entry cache |
| `dir.rs` | Directory operations |
| `file.rs` | File handle operations |
| `ramfs.rs` | In-memory file system |
| `tmpfs.rs` | Size-limited in-memory file system |
| `ext4.rs` | EXT4 file system implementation |
| `fs.rs` | SysFS (kernel information filesystem) |
| `journal.rs` | Journaling support |

## Key Interfaces

### FileSystemType

All file systems must implement the `FileSystemType` trait:

```rust
pub trait FileSystemType {
    fn name(&self) -> &str;
    fn mount(&self, dev: Option<String>, flags: u32) -> Result<Arc<dyn SuperBlock>, VfsError>;
    fn detect(&self, data: &[u8]) -> bool;
}
```

### SuperBlock

Each mounted filesystem provides a `SuperBlock`:

```rust
pub trait SuperBlock: Send + Sync {
    fn fs_type(&self) -> &dyn FileSystemType;
    fn root_inode(&self) -> Arc<dyn Inode>;
    fn statfs(&self) -> Result<StatFs, VfsError>;
    fn sync(&self) -> Result<(), VfsError>;
}
```

### Inode

File/directory/symlink abstraction:

```rust
pub trait Inode: Send + Sync {
    fn inode_number(&self) -> u64;
    fn file_type(&self) -> FileType;
    fn mode(&self) -> FileMode;
    fn size(&self) -> u64;
    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, VfsError>;
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, VfsError>;
    fn write(&self, offset: u64, buf: &[u8]) -> Result<usize, VfsError>;
}
```

## Mount Points

Filesystems are mounted at specific paths in the VFS namespace:

```rust
// Mount ext4 filesystem
vfs.mount("ext4", "/mnt/data", Some("/dev/sda1"), 0)?;

// Mount tmpfs
vfs.mount("tmpfs", "/tmp", None, 0)?;

// Unmount
vfs.unmount("/mnt/data")?;
```

## Path Resolution

The VFS handles path resolution across mount points:

```
/usr/bin/bash
└── /usr (ext4 root)
    └── bin (ext4 directory)
        └── bash (ext4 file)

/mnt/data/file.txt
└── /mnt (ext4 root)
    └── data (mounted ext4 at /mnt/data)
        └── file.txt
```

## Dentry Cache

The dentry (directory entry) cache improves performance by caching directory lookups:

```rust
struct Dentry {
    name: String,
    parent: Option<Arc<Dentry>>,
    inode: Arc<dyn Inode>,
    children: Mutex<BTreeMap<String, Arc<Dentry>>>,
    mount: Option<Arc<Mount>>,
    refcount: AtomicUsize,
}
```

## Error Handling

All VFS operations return `Result<T, VfsError>`:

```rust
pub enum VfsError {
    NotFound,
    PermissionDenied,
    NotDirectory,
    IsDirectory,
    NotEmpty,
    Exists,
    NoSpace,
    InvalidPath,
    NotMounted,
    Busy,
    ReadOnly,
    IoError,
    NotSupported,
    InvalidOperation,
}
```

## Integration

### System Calls

The VFS layer is accessed via system calls in `subsystems/syscalls/fs/`:

```rust
// Open a file
let fd = sys_open("/path/to/file", O_RDONLY, 0)?;

// Read from file
let bytes = sys_read(fd, buf)?;

// Write to file
let written = sys_write(fd, buf)?;

// Close file
sys_close(fd)?;
```

### Initialization

From `core/init.rs`:

```rust
// Initialize filesystems
ramfs::init();
ext4::init();
procfs::init();
sysfs::init();

// Mount root filesystem
vfs.mount("ramfs", "/", None, 0)?;

// Verify root is accessible
vfs.verify_root()?;
```

## Future Improvements

- [ ] Implement path resolution module
- [ ] Add symbolic link resolution
- [ ] Implement proper VFS trait definitions
- [ ] Complete file operations (mkdir, create, write, unlink)
- [ ] Add NFS support
- [ ] Implement FUSE support
- [ ] Add quota management
- [ ] Implement extended attributes
- [ ] Add access control list (ACL) support
