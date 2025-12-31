//! POSIX Memory Management Constants (mmap, mprotect, etc.)

/// Memory protection flags for mprotect and mmap
pub const PROT_NONE: i32 = 0x0;
pub const PROT_READ: i32 = 0x1;
pub const PROT_WRITE: i32 = 0x2;
pub const PROT_EXEC: i32 = 0x4;

/// Memory mapping flags for mmap
pub const MAP_SHARED: i32 = 0x01;
pub const MAP_PRIVATE: i32 = 0x02;
pub const MAP_FIXED: i32 = 0x10;
pub const MAP_ANONYMOUS: i32 = 0x20;
pub const MAP_GROWSDOWN: i32 = 0x00100;
pub const MAP_DENYWRITE: i32 = 0x00800;
pub const MAP_EXECUTABLE: i32 = 0x01000;
pub const MAP_LOCKED: i32 = 0x02000;
pub const MAP_NORESERVE: i32 = 0x04000;
pub const MAP_POPULATE: i32 = 0x08000;
pub const MAP_NONBLOCK: i32 = 0x10000;
pub const MAP_STACK: i32 = 0x20000;
pub const MAP_HUGETLB: i32 = 0x40000;

