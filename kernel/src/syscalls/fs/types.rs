//! Filesystem System Call Types
//!
//! This module defines types, enums, and structs used by the filesystem
//! syscall implementation.

use alloc::{string::String, vec::Vec};

/// Filesystem operation types for statistics tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemOperation {
    /// Change directory (chdir, fchdir)
    ChangeDirectory,
    /// Get current working directory (getcwd)
    GetCurrentDirectory,
    /// Make directory (mkdir)
    MakeDirectory,
    /// Remove directory (rmdir)
    RemoveDirectory,
    /// Unlink file (unlink)
    Unlink,
    /// Rename file (rename)
    Rename,
    /// Create hard link (link)
    Link,
    /// Create symbolic link (symlink)
    Readlink,
    /// Read symbolic link (readlink)
    Symlink,
    /// Change file mode (chmod, fchmod)
    ChangeMode,
    /// Change file owner (chown, fchown)
    ChangeOwner,
    /// Set file creation mask (umask)
    SetUmask,
    /// Get file status (stat)
    Stat,
    /// Get file status without following links (lstat)
    Lstat,
    /// Check file accessibility (access)
    Access,
    /// Mount filesystem (mount)
    Mount,
    /// Unmount filesystem (umount)
    Umount,
    /// Other filesystem operations
    Other,
}

/// Filesystem error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilesystemError {
    /// Invalid argument provided
    InvalidArgument,
    /// File name too long
    FileNameTooLong,
    /// File or directory not found
    NotFound,
    /// Permission denied
    PermissionDenied,
    /// File already exists
    AlreadyExists,
    /// Input/output error
    IoError,
    /// No space left on device
    NoSpaceLeft,
    /// Read-only filesystem
    ReadOnly,
    /// Operation not supported
    NotSupported,
    /// Not a directory
    NotDirectory,
    /// Is a directory
    IsDirectory,
    /// Directory not empty
    DirectoryNotEmpty,
    /// Filesystem is busy
    FilesystemBusy,
    /// Cross-device link
    CrossDeviceLink,
    /// Stale file handle
    StaleFileHandle,
}

/// Filesystem request types
#[derive(Debug, Clone)]
pub enum FilesystemRequest {
    /// Change directory request
    ChangeDirectory { path: String },
    /// Get current working directory request
    GetCurrentDirectory { buffer: Vec<u8>, size: usize },
    /// Make directory request
    MakeDirectory { path: String, mode: u32 },
    /// Remove directory request
    RemoveDirectory { path: String },
    /// Unlink file request
    Unlink { path: String },
    /// Rename file request
    Rename { old_path: String, new_path: String },
    /// Create hard link request
    Link { old_path: String, new_path: String },
    /// Create symbolic link request
    Symlink { target: String, link_path: String },
    /// Read symbolic link request
    Readlink { path: String, buffer: Vec<u8>, size: usize },
    /// Change file mode request
    ChangeMode { path: String, mode: u32 },
    /// Change file owner request
    ChangeOwner { path: String, uid: u32, gid: u32 },
    /// Set file creation mask request
    SetUmask { mask: u32 },
    /// Get file status request
    Stat { path: String },
    /// Get file status without following links request
    Lstat { path: String },
    /// Check file accessibility request
    Access { path: String, mode: u32 },
    /// Mount filesystem request
    Mount {
        source: String,
        target: String,
        fs_type: String,
        flags: u32,
        options: String,
    },
    /// Unmount filesystem request
    Umount { target: String, flags: u32 },
}

/// Filesystem response types
#[derive(Debug, Clone)]
pub enum FilesystemResponse {
    /// Success with no data
    Success,
    /// Success with string data
    SuccessString { data: String },
    /// Success with boolean data
    SuccessBool { result: bool },
    /// Success with numeric data
    SuccessU64 { value: u64 },
    /// Success with file statistics
    SuccessFileStats { stats: FileStats },
    /// Error response
    Error { error: FilesystemError },
}

/// File statistics structure
#[derive(Debug, Clone, Default)]
pub struct FileStats {
    /// File inode number
    pub ino: u64,
    /// File mode
    pub mode: u32,
    /// Number of hard links
    pub nlink: u32,
    /// Owner user ID
    pub uid: u32,
    /// Owner group ID
    pub gid: u32,
    /// Device ID
    pub rdev: u64,
    /// File size in bytes
    pub size: u64,
    /// Block size
    pub blksize: u64,
    /// Number of blocks allocated
    pub blocks: u64,
    /// Last access time
    pub atime: u64,
    /// Last modification time
    pub mtime: u64,
    /// Last status change time
    pub ctime: u64,
}

/// Mount options structure
#[derive(Debug, Clone, Default)]
pub struct MountOptions {
    /// Read-only flag
    pub read_only: bool,
    /// No-suid flag
    pub nosuid: bool,
    /// No-dev flag
    pub nodev: bool,
    /// No-exec flag
    pub noexec: bool,
    /// Synchronous flag
    pub sync: bool,
    /// No-atime flag
    pub noatime: bool,
    /// Custom mount options
    pub custom_options: Vec<String>,
}

/// Filesystem metadata structure
#[derive(Debug, Clone, Default)]
pub struct FilesystemMetadata {
    /// Total size in bytes
    pub total_size: u64,
    /// Used size in bytes
    pub used_size: u64,
    /// Available size in bytes
    pub available_size: u64,
    /// Total number of inodes
    pub total_inodes: u64,
    /// Number of used inodes
    pub used_inodes: u64,
    /// Filesystem type
    pub fs_type: String,
    /// Mount point
    pub mount_point: String,
}

/// Path resolution result
#[derive(Debug, Clone)]
pub enum PathResolutionResult {
    /// Successfully resolved path
    Resolved { path: String, exists: bool },
    /// Path not found
    NotFound,
    /// Permission denied
    PermissionDenied,
    /// Invalid path format
    InvalidPath,
}

/// File operation permission types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilePermission {
    /// Read permission
    Read = 1 << 0,
    /// Write permission
    Write = 1 << 1,
    /// Execute permission
    Execute = 1 << 2,
}

impl FilePermission {
    /// Check if permission includes read access
    pub fn can_read(self) -> bool {
        (self as u32) & Self::Read as u32 != 0
    }

    /// Check if permission includes write access
    pub fn can_write(self) -> bool {
        (self as u32) & Self::Write as u32 != 0
    }

    /// Check if permission includes execute access
    pub fn can_execute(self) -> bool {
        (self as u32) & Self::Execute as u32 != 0
    }

    /// Convert from bitmask
    pub fn from_mask(mask: u32) -> Self {
        let mut perm = Self::Execute;
        if mask & Self::Read as u32 != 0 {
            perm = Self::Read;
        }
        if mask & Self::Write as u32 != 0 {
            perm = Self::Write;
        }
        if mask & Self::Execute as u32 != 0 {
            perm = Self::Execute;
        }
        perm
    }
}

/// File seek origin types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSeekOrigin {
    /// From beginning of file
    Set = 0,
    /// From current position
    Current = 1,
    /// From end of file
    End = 2,
}

/// File open flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOpenFlag {
    /// Read-only access
    ReadOnly = 0,
    /// Write-only access
    WriteOnly = 1,
    /// Read-write access
    ReadWrite = 2,
    /// Create file if it doesn't exist
    Create = 64,
    /// Exclusive access
    Exclusive = 128,
    /// Append mode
    Append = 1024,
    /// Truncate file on open
    Truncate = 512,
    /// Non-blocking mode
    NonBlocking = 2048,
}

impl FileOpenFlag {
    /// Check if flag includes write access
    pub fn is_writable(self) -> bool {
        matches!(self, Self::WriteOnly | Self::ReadWrite)
    }

    /// Check if flag includes read access
    pub fn is_readable(self) -> bool {
        matches!(self, Self::ReadOnly | Self::ReadWrite)
    }

    /// Check if flag should create file if it doesn't exist
    pub fn should_create(self) -> bool {
        matches!(self, Self::Create)
    }
}

/// Default conversion for FilesystemOperation
impl Default for FilesystemOperation {
    fn default() -> Self {
        FilesystemOperation::Other
    }
}
