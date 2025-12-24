//! POSIX Open Flags (fcntl.h)

/// Open for reading only
pub const O_RDONLY: i32 = 0;

/// Open for writing only
pub const O_WRONLY: i32 = 1;

/// Open for reading and writing
pub const O_RDWR: i32 = 2;

/// Access mode mask
pub const O_ACCMODE: i32 = 3;

/// Create file if it doesn't exist
pub const O_CREAT: i32 = 0o100;

/// Exclusive use flag
pub const O_EXCL: i32 = 0o200;

/// No delay for the data to be written
pub const O_NOCTTY: i32 = 0o400;

/// Append data to the end of file
pub const O_APPEND: i32 = 0o2000;

/// Non-blocking mode
pub const O_NONBLOCK: i32 = 0o4000;

/// Synchronous writes
pub const O_SYNC: i32 = 0o10000;

/// Truncate file to zero length
pub const O_TRUNC: i32 = 0o1000;

/// Close on exec
pub const O_CLOEXEC: i32 = 0o200000;

/// Direct I/O access
pub const O_DIRECT: i32 = 0o40000;

/// Don't update file access time
pub const O_NOATIME: i32 = 0o100000;

/// Path is a symbolic link
pub const O_NOFOLLOW: i32 = 0o200000;

/// Message flags
pub const MSG_DONTWAIT: i32 = 0o40;

/// Socket level
pub const SOL_SOCKET: i32 = 1;

/// Reuse address
pub const SO_REUSEADDR: i32 = 2;

/// Reuse port
pub const SO_REUSEPORT: i32 = 15;

/// Keepalive
pub const SO_KEEPALIVE: i32 = 9;
