//! POSIX Stat and Time Structures

use super::types::{Mode, Dev, Ino, Nlink, Off, Blksize, Blkcnt, Time};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Stat {
    pub st_dev: Dev,
    pub st_ino: Ino,
    pub st_nlink: Nlink,
    pub st_mode: Mode,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: Dev,
    pub st_size: Off,
    pub st_blksize: Blksize,
    pub st_blocks: Blkcnt,
    pub st_atime: Time,
    pub st_mtime: Time,
    pub st_ctime: Time,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Timeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Rlimit {
    pub rlim_cur: u64,
    pub rlim_max: u64,
}
