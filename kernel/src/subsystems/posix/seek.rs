//! POSIX Seek Constants (unistd.h)

/// Seek from beginning of file
pub const SEEK_SET: i32 = 0;

/// Seek from current position
pub const SEEK_CUR: i32 = 1;

/// Seek from end of file
pub const SEEK_END: i32 = 2;

/// Seek to next data
pub const SEEK_DATA: i32 = 3;

/// Seek to next hole
pub const SEEK_HOLE: i32 = 4;
