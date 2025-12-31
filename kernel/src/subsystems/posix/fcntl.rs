//! POSIX fcntl Commands (fcntl.h)

/// Duplicate file descriptor
pub const F_DUPFD: i32 = 0;

/// Get file descriptor flags
pub const F_GETFD: i32 = 1;

/// Set file descriptor flags
pub const F_SETFD: i32 = 2;

/// Get file status flags
pub const F_GETFL: i32 = 3;

/// Set file status flags
pub const F_SETFL: i32 = 4;

/// Get lock
pub const F_GETLK: i32 = 5;

/// Set lock
pub const F_SETLK: i32 = 6;

/// Set lock with wait
pub const F_SETLKW: i32 = 7;

/// Get process group ID
pub const F_GETOWN: i32 = 9;

/// Set process group ID
pub const F_SETOWN: i32 = 8;
