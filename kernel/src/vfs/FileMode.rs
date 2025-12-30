//! File mode and permission bits
extern crate alloc;

use crate::prelude::*;

use alloc::fmt;

/// File mode bits
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FileMode {
    /// File type and permission bits
    pub bits: u32,
}

impl FileMode {
    /// Empty file mode
    pub const EMPTY: Self = Self { bits: 0 };

    /// Regular file
    pub const S_IFREG: u32 = 0o100000;
    /// Directory
    pub const S_IFDIR: u32 = 0o040000;
    /// Character device
    pub const S_IFCHR: u32 = 0o020000;
    /// Block device
    pub const S_IFBLK: u32 = 0o060000;
    /// Symbolic link
    pub const S_IFLNK: u32 = 0o120000;
    /// Named pipe (FIFO)
    pub const S_IFIFO: u32 = 0o010000;
    /// Socket
    pub const S_IFSOCK: u32 = 0o140000;

    /// Owner permissions
    pub const S_IRWXU: u32 = 0o0700;
    pub const S_IRUSR: u32 = 0o0400;
    pub const S_IWUSR: u32 = 0o0200;
    pub const S_IXUSR: u32 = 0o0100;

    /// Group permissions
    pub const S_IRWXG: u32 = 0o0070;
    pub const S_IRGRP: u32 = 0o0040;
    pub const S_IWGRP: u32 = 0o0020;
    pub const S_IXGRP: u32 = 0o0010;

    /// Other permissions
    pub const S_IRWXO: u32 = 0o0007;
    pub const S_IROTH: u32 = 0o0004;
    pub const S_IWOTH: u32 = 0o0002;
    pub const S_IXOTH: u32 = 0o0001;

    /// Create a new file mode
    pub const fn new(bits: u32) -> Self {
        Self { bits }
    }

    /// Check if this is a regular file
    pub fn is_reg(&self) -> bool {
        (self.bits & 0o170000) == Self::S_IFREG
    }

    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        (self.bits & 0o170000) == Self::S_IFDIR
    }

    /// Check if this is a symbolic link
    pub fn is_lnk(&self) -> bool {
        (self.bits & 0o170000) == Self::S_IFLNK
    }

    /// Check if this is a character device
    pub fn is_chr(&self) -> bool {
        (self.bits & 0o170000) == Self::S_IFCHR
    }

    /// Check if this is a block device
    pub fn is_blk(&self) -> bool {
        (self.bits & 0o170000) == Self::S_IFBLK
    }

    /// Check if this is a FIFO
    pub fn is_fifo(&self) -> bool {
        (self.bits & 0o170000) == Self::S_IFIFO
    }

    /// Check if this is a socket
    pub fn is_sock(&self) -> bool {
        (self.bits & 0o170000) == Self::S_IFSOCK
    }

    /// Get permission bits
    pub fn permissions(&self) -> u32 {
        self.bits & 0o777
    }

    /// Create regular file mode
    pub const fn reg() -> Self {
        Self { bits: Self::S_IFREG | 0o644 }
    }

    /// Create directory mode
    pub const fn dir() -> Self {
        Self { bits: Self::S_IFDIR | 0o755 }
    }

    /// Create symbolic link mode
    pub const fn lnk() -> Self {
        Self { bits: Self::S_IFLNK }
    }
}

impl fmt::Display for FileMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04o}", self.bits)
    }
}

impl From<u32> for FileMode {
    fn from(bits: u32) -> Self {
        Self { bits }
    }
}

impl From<FileMode> for u32 {
    fn from(mode: FileMode) -> Self {
        mode.bits
    }
}
