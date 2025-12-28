//! Inotify file descriptor API

use alloc::vec::Vec;
use crate::api::KernelError;

pub struct InotifyInstance {
    events: Vec<InotifyEvent>,
}

#[derive(Debug, Clone)]
pub struct InotifyEvent {
    pub wd: i32,
    pub mask: u32,
    pub cookie: u32,
    pub name: Vec<u8>,
}

impl InotifyInstance {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
        }
    }

    pub fn has_events(&self) -> bool {
        !self.events.is_empty()
    }

    pub fn read_events(&mut self, _buf: &mut [u8]) -> usize {
        // Return number of events available
        self.events.len()
    }
}

pub fn init() -> Result<(), KernelError> {
    Ok(())
}

/// Inotify event mask flags (Linux compatible)
pub mod mask {
    /// File was accessed
    pub const IN_ACCESS: u32 = 0x00000001;
    /// Metadata changed
    pub const IN_MODIFY: u32 = 0x00000002;
    /// Attributes changed
    pub const IN_ATTRIB: u32 = 0x00000004;
    /// Writtable file was closed
    pub const IN_CLOSE_WRITE: u32 = 0x00000008;
    /// Unwrittable file was closed
    pub const IN_CLOSE_NOWRITE: u32 = 0x00000010;
    /// File was opened
    pub const IN_OPEN: u32 = 0x00000020;
    /// File was moved from X
    pub const IN_MOVED_FROM: u32 = 0x00000040;
    /// File was moved to Y
    pub const IN_MOVED_TO: u32 = 0x00000080;
    /// Subfile was created
    pub const IN_CREATE: u32 = 0x00000100;
    /// Subfile was deleted
    pub const IN_DELETE: u32 = 0x00000200;
    /// Self was deleted
    pub const IN_DELETE_SELF: u32 = 0x00000400;
    /// Self was moved
    pub const IN_MOVE_SELF: u32 = 0x00000800;
    /// Backing fs was unmounted
    pub const IN_UNMOUNT: u32 = 0x00002000;
    /// Event queued overflowed
    pub const IN_Q_OVERFLOW: u32 = 0x00004000;
    /// File was ignored
    pub const IN_IGNORED: u32 = 0x00008000;
    /// Only watch the path if it is a directory
    pub const IN_ONLYDIR: u32 = 0x01000000;
    /// Don't follow a sym link
    pub const IN_DONT_FOLLOW: u32 = 0x02000000;
    /// Exclude events on unlinked objects
    pub const IN_EXCL_UNLINK: u32 = 0x04000000;
    /// Add to the mask of an already existing watch
    pub const IN_MASK_ADD: u32 = 0x20000000;
    /// Event occurred against dir
    pub const IN_ISDIR: u32 = 0x40000000;
    /// Only send event once
    pub const IN_ONESHOT: u32 = 0x80000000;
}
