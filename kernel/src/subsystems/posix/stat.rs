//! POSIX Stat and Time Structures

use super::types::{Blkcnt, Blksize, Dev, Ino, Mode, Nlink, Off, Time};
use crate::subsystems::process::rlimit::RLIM_INFINITY;

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

/// POSIX-compatible timespec type alias
#[allow(non_camel_case_types)]
pub type timespec = Timespec;

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

impl Rlimit {
    /// 创建新的资源限制
    pub const fn new(cur: u64, max: u64) -> Self {
        Self { rlim_cur: cur, rlim_max: max }
    }

    /// 检查资源限制值是否有效
    ///
    /// 有效性规则:
    /// - 软限制不能超过硬限制（除非硬限制为 RLIM_INFINITY）
    pub const fn is_valid(&self) -> bool {
        self.rlim_cur <= self.rlim_max || self.rlim_max == RLIM_INFINITY
    }
}
