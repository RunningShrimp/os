//! POSIX Basic Types
//!
//! Standard primitive types for POSIX compliance

// Allow POSIX-style type names (snake_case with _t suffix) which are standard in POSIX
#![allow(non_camel_case_types)]

/// Process ID type
pub type Pid = i32;

/// User ID type
pub type Uid = u32;

/// Group ID type
pub type Gid = u32;

/// File mode type
pub type Mode = u32;

/// POSIX mode_t type alias (for compatibility with system call APIs)
pub type mode_t = Mode;

/// Device ID type
pub type Dev = u64;

/// Inode number type
pub type Ino = u64;

/// Number of links type
pub type Nlink = u64;

/// File offset type
pub type Off = i64;

/// Block size type
pub type Blksize = i64;

/// Block count type
pub type Blkcnt = i64;

/// Time type (seconds since epoch)
pub type Time = i64;

/// Size type for syscalls
pub type SSize = isize;

/// Size type for shared memory and file sizes
pub type Size = usize;

/// Clock ID type
pub type ClockId = i32;

/// Clock tick type
pub type clock_t = i64;

/// File offset type (alternative name)
pub type off_t = Off;

/// AIO request priority type
pub type aio_reqprio_t = i32;

/// Size type for syscalls (alternative name)
pub type ssize_t = SSize;

/// Size type for shared memory (alternative name)
pub type size_t = Size;
