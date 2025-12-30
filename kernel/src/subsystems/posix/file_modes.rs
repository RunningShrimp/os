//! POSIX File Mode Bits and Permissions

use super::types::Mode;

/// Set-user-ID on execution
pub const S_ISUID: Mode = 0o4000;

/// Set-group-ID on execution
pub const S_ISGID: Mode = 0o2000;

/// Sticky bit
pub const S_ISVTX: Mode = 0o1000;

/// File type mask
pub const S_IFMT: Mode = 0o170000;

/// Socket
pub const S_IFSOCK: Mode = 0o140000;

/// Symbolic link
pub const S_IFLNK: Mode = 0o120000;

/// Regular file
pub const S_IFREG: Mode = 0o100000;

/// Block device
pub const S_IFBLK: Mode = 0o060000;

/// Directory
pub const S_IFDIR: Mode = 0o040000;

/// Character device
pub const S_IFCHR: Mode = 0o020000;

/// FIFO
pub const S_IFIFO: Mode = 0o010000;

/// Read permission for owner
pub const S_IRUSR: Mode = 0o0400;

/// Write permission for owner
pub const S_IWUSR: Mode = 0o0200;

/// Execute permission for owner
pub const S_IXUSR: Mode = 0o0100;

/// Read, write, execute for owner
pub const S_IRWXU: Mode = S_IRUSR | S_IWUSR | S_IXUSR;

/// Read permission for group
pub const S_IRGRP: Mode = 0o0040;

/// Write permission for group
pub const S_IWGRP: Mode = 0o0020;

/// Execute permission for group
pub const S_IXGRP: Mode = 0o0010;

/// Read, write, execute for group
pub const S_IRWXG: Mode = S_IRGRP | S_IWGRP | S_IXGRP;

/// Read permission for others
pub const S_IROTH: Mode = 0o0004;

/// Write permission for others
pub const S_IWOTH: Mode = 0o0002;

/// Execute permission for others
pub const S_IXOTH: Mode = 0o0001;

/// Read, write, execute for others
pub const S_IRWXO: Mode = S_IROTH | S_IWOTH | S_IXOTH;

/// Test for read permission
pub const R_OK: i32 = 4;

/// Test for write permission
pub const W_OK: i32 = 2;

/// Test for execute permission
pub const X_OK: i32 = 1;

/// Test for existence of file
pub const F_OK: i32 = 0;

/// Check if mode indicates a regular file
#[inline]
pub const fn s_isreg(mode: Mode) -> bool {
    (mode & S_IFMT) == S_IFREG
}

/// Check if mode indicates a directory
#[inline]
pub const fn s_isdir(mode: Mode) -> bool {
    (mode & S_IFMT) == S_IFDIR
}

/// Check if mode indicates a character device
#[inline]
pub const fn s_ischr(mode: Mode) -> bool {
    (mode & S_IFMT) == S_IFCHR
}

/// Check if mode indicates a block device
#[inline]
pub const fn s_isblk(mode: Mode) -> bool {
    (mode & S_IFMT) == S_IFBLK
}

/// Check if mode indicates a FIFO
#[inline]
pub const fn s_isfifo(mode: Mode) -> bool {
    (mode & S_IFMT) == S_IFIFO
}

/// Check if mode indicates a symbolic link
#[inline]
pub const fn s_islnk(mode: Mode) -> bool {
    (mode & S_IFMT) == S_IFLNK
}

/// Check if mode indicates a socket
#[inline]
pub const fn s_issock(mode: Mode) -> bool {
    (mode & S_IFMT) == S_IFSOCK
}
