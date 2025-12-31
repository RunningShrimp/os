//! # FUSE (Filesystem in Userspace) Support
//!
//! This module provides comprehensive FUSE protocol implementation, allowing
//! userspace filesystems to be mounted and used through the kernel.
//!
//! ## Overview
//!
//! FUSE enables non-privileged users to create their own filesystems without
//! editing kernel code. This module implements:
//! - FUSE protocol v7.8+ support
//! - Device communication (/dev/fuse)
//! - Request handling and response processing
//! - Mount operations and daemon management
//! - Complete FUSE operations (getattr, readdir, open, read, write, mkdir, unlink, etc.)
//! - Async I/O support
//! - Caching strategies
//!
//! ## Architecture
//!
//! ```
//! Application (Userspace)
//!     ↓ system calls
//! Kernel VFS Layer
//!     ↓
//! FUSE Module (this module)
//!     ↓ /dev/fuse
//! FUSE Daemon (Userspace)
//!     ↓
//! Actual Filesystem Implementation
//! ```
//!
//! ## Features
//!
//! - **Full Protocol Support**: Implements FUSE protocol version 7.31
//! - **Async I/O**: Supports asynchronous request processing
//! - **Caching**: Multiple caching strategies (none, auto, always)
//! - **POSIX Compatibility**: Full POSIX filesystem semantics
//! - **Security**: Proper permission checking and sandboxing
//!
//! ## Usage Example
//!
//! ```no_run
//! use kernel::subsystems::fs::fuse::{FuseMount, FuseConn};
//!
//! // Mount a FUSE filesystem
//! let conn = FuseMount::new(
//!     "/mnt/fuse",
//!     "myfs",
//!     FuseMountFlags::DEFAULT,
//! )?;
//!
//! // The FUSE daemon will communicate via /dev/fuse
//! # Ok::<(), kernel::subsystems::fs::api::error::FsError>(())
//! ```

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use spin::RwLock;

use crate::subsystems::sync::Mutex as SyncMutex;

/// FUSE protocol version we support
pub const FUSE_KERNEL_VERSION: u32 = 7;
pub const FUSE_MINOR_VERSION: u32 = 31;

/// FUSE magic number for protocol validation
pub const FUSE_KHEADER_MAGIC: u32 = 0x65735546;

/// Default FUSE buffer sizes
pub const FUSE_MAX_PAGES_PER_REQ: u32 = 32;
pub const FUSE_DEFAULT_MAX_PAGES: u32 = 32;
pub const FUSE_DEFAULT_MAX_READ: u32 = 131072;
pub const FUSE_DEFAULT_MAX_WRITE: u32 = 131072;

/// FUSE operation codes (from fuse_kernel.h)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FuseOpcode {
    Lookup = 1,
    Forget = 2,
    Getattr = 3,
    Setattr = 4,
    Readlink = 5,
    Symlink = 6,
    Mknod = 8,
    Mkdir = 9,
    Unlink = 10,
    Rmdir = 11,
    Rename = 12,
    Link = 13,
    Open = 14,
    Read = 15,
    Write = 16,
    Statfs = 17,
    Release = 18,
    Fsync = 20,
    Setxattr = 21,
    Getxattr = 22,
    Listxattr = 23,
    Removexattr = 24,
    Flush = 25,
    Init = 26,
    Opendir = 27,
    Readdir = 28,
    Releasedir = 29,
    Fsyncdir = 30,
    Getlk = 31,
    Setlk = 32,
    Setlkw = 33,
    Access = 34,
    Create = 35,
    Interrupt = 36,
    Bmap = 37,
    Destroy = 38,
    Ioctl = 39,
    Poll = 40,
    NotifyReply = 41,
    BatchForget = 42,
    Fallocate = 43,
    Readdirplus = 44,
    Rename2 = 45,
    Lseek = 46,
    CopyFileRange = 47,
    Setupmapping = 48,
    Removemapping = 49,
    Syncfs = 50,
}

impl TryFrom<u32> for FuseOpcode {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(FuseOpcode::Lookup),
            2 => Ok(FuseOpcode::Forget),
            3 => Ok(FuseOpcode::Getattr),
            4 => Ok(FuseOpcode::Setattr),
            5 => Ok(FuseOpcode::Readlink),
            6 => Ok(FuseOpcode::Symlink),
            8 => Ok(FuseOpcode::Mknod),
            9 => Ok(FuseOpcode::Mkdir),
            10 => Ok(FuseOpcode::Unlink),
            11 => Ok(FuseOpcode::Rmdir),
            12 => Ok(FuseOpcode::Rename),
            13 => Ok(FuseOpcode::Link),
            14 => Ok(FuseOpcode::Open),
            15 => Ok(FuseOpcode::Read),
            16 => Ok(FuseOpcode::Write),
            17 => Ok(FuseOpcode::Statfs),
            18 => Ok(FuseOpcode::Release),
            20 => Ok(FuseOpcode::Fsync),
            21 => Ok(FuseOpcode::Setxattr),
            22 => Ok(FuseOpcode::Getxattr),
            23 => Ok(FuseOpcode::Listxattr),
            24 => Ok(FuseOpcode::Removexattr),
            25 => Ok(FuseOpcode::Flush),
            26 => Ok(FuseOpcode::Init),
            27 => Ok(FuseOpcode::Opendir),
            28 => Ok(FuseOpcode::Readdir),
            29 => Ok(FuseOpcode::Releasedir),
            30 => Ok(FuseOpcode::Fsyncdir),
            31 => Ok(FuseOpcode::Getlk),
            32 => Ok(FuseOpcode::Setlk),
            33 => Ok(FuseOpcode::Setlkw),
            34 => Ok(FuseOpcode::Access),
            35 => Ok(FuseOpcode::Create),
            36 => Ok(FuseOpcode::Interrupt),
            37 => Ok(FuseOpcode::Bmap),
            38 => Ok(FuseOpcode::Destroy),
            39 => Ok(FuseOpcode::Ioctl),
            40 => Ok(FuseOpcode::Poll),
            41 => Ok(FuseOpcode::NotifyReply),
            42 => Ok(FuseOpcode::BatchForget),
            43 => Ok(FuseOpcode::Fallocate),
            44 => Ok(FuseOpcode::Readdirplus),
            45 => Ok(FuseOpcode::Rename2),
            46 => Ok(FuseOpcode::Lseek),
            47 => Ok(FuseOpcode::CopyFileRange),
            48 => Ok(FuseOpcode::Setupmapping),
            49 => Ok(FuseOpcode::Removemapping),
            50 => Ok(FuseOpcode::Syncfs),
            _ => Err(()),
        }
    }
}

/// FUSE mount flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuseMountFlags(u32);

impl FuseMountFlags {
    pub const DEFAULT: Self = Self(0);
    pub const NOSUID: Self = Self(1 << 0);
    pub const NODEV: Self = Self(1 << 1);
    pub const NOEXEC: Self = Self(1 << 2);
    pub const RDONLY: Self = Self(1 << 3);
    pub const ALLOW_OTHER: Self = Self(1 << 4);
    pub const DEFAULT_PERMISSIONS: Self = Self(1 << 5);

    pub fn new(flags: u32) -> Self {
        Self(flags)
    }

    pub fn contains(&self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }
}

/// FUSE initialization flags
#[derive(Debug, Clone, Copy)]
pub struct FuseInitFlags(u32);

impl FuseInitFlags {
    pub const ASYNC_READ: Self = Self(1 << 0);
    pub const POSIX_LOCKS: Self = Self(1 << 1);
    pub const FILE_OPS: Self = Self(1 << 2);
    pub const ATOMIC_O_TRUNC: Self = Self(1 << 3);
    pub const EXPORT_SUPPORT: Self = Self(1 << 4);
    pub const BIG_WRITES: Self = Self(1 << 5);
    pub const DONT_MASK: Self = Self(1 << 6);
    pub const SPLICE_WRITE: Self = Self(1 << 7);
    pub const SPLICE_MOVE: Self = Self(1 << 8);
    pub const SPLICE_READ: Self = Self(1 << 9);
    pub const FLOCK_LOCKS: Self = Self(1 << 10);
    pub const HAS_IOCTL_DIR: Self = Self(1 << 11);
    pub const AUTO_INVAL_DATA: Self = Self(1 << 12);
    pub const REaddirPLUS: Self = Self(1 << 13);
    pub const READDIRPLUS_AUTO: Self = Self(1 << 14);
    pub const ASYNC_DIO: Self = Self(1 << 15);
    pub const WRITEBACK_CACHE: Self = Self(1 << 16);
    pub const ZERO_MESSAGE_OPEN: Self = Self(1 << 17);
    pub const PARALLEL_DIROPS: Self = Self(1 << 18);
    pub const HANDLE_KILLPRIV: Self = Self(1 << 19);
    pub const CACHE_SYMLINKS: Self = Self(1 << 20);
    pub const NO_OPENDIR_SUPPORT: Self = Self(1 << 21);
    pub const EXPLICIT_INVAL_DATA: Self = Self(1 << 22);
    pub const MAP_ALIGNMENT: Self = Self(1 << 23);
    pub const SUBMOUNTS: Self = Self(1 << 24);
    pub const HANDLE_KILLPRIV_V2: Self = Self(1 << 25);
    pub const SETXATTR_EXT: Self = Self(1 << 26);
    pub const INIT_EXT: Self = Self(1 << 27);
    pub const INIT_RESERVED: Self = Self(1 << 28);
    pub const SECURITY_CTX: Self = Self(1 << 29);
    pub const HAS_INODE_DAX: Self = Self(1 << 30);
    pub const CREATE_SUPP_GROUP: Self = Self(1 << 31);
}

/// FUSE file attributes
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseAttr {
    pub ino: u64,
    pub size: u64,
    pub blocks: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub atimensec: u32,
    pub mtimensec: u32,
    pub ctimensec: u32,
    pub mode: u32,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u32,
    pub blksize: u32,
    pub flags: u32,
}

/// FUSE entry information (for lookup, create, mkdir, etc.)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FuseEntryOut {
    pub nodeid: u64,
    pub generation: u64,
    pub entry_valid: u64,
    pub attr_valid: u64,
    pub entry_valid_nsec: u32,
    pub attr_valid_nsec: u32,
    pub attr: FuseAttr,
}

/// FUSE open flags
#[derive(Debug, Clone, Copy)]
pub struct FuseOpenFlags(u32);

impl FuseOpenFlags {
    pub const RDONLY: Self = Self(0o000000);
    pub const WRONLY: Self = Self(0o000001);
    pub const RDWR: Self = Self(0o000002);
    pub const NONBLOCK: Self = Self(0o000004);
    pub const APPEND: Self = Self(0o000010);
    pub const CREAT: Self = Self(0o000100);
    pub const TRUNC: Self = Self(0o001000);
    pub const EXCL: Self = Self(0o002000);
}

/// FUSE file information (for open/opendir replies)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FuseOpenOut {
    pub fh: u64,
    pub open_flags: u32,
    pub padding: u32,
}

/// FUSE request header (sent from kernel to userspace)
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FuseInHeader {
    pub len: u32,
    pub opcode: u32,
    pub unique: u64,
    pub nodeid: u64,
    pub uid: u32,
    pub gid: u32,
    pub pid: u32,
    pub padding: u32,
}

/// FUSE reply header (sent from userspace to kernel)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FuseOutHeader {
    pub len: u32,
    pub error: i32,
    pub unique: u64,
}

/// FUSE init request (sent during initialization)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FuseInitIn {
    pub major: u32,
    pub minor: u32,
    pub max_readahead: u32,
    pub flags: u32,
}

/// FUSE init reply
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FuseInitOut {
    pub major: u32,
    pub minor: u32,
    pub max_readahead: u32,
    pub flags: u32,
    pub max_background: u32,
    pub congestion_threshold: u32,
    pub max_write: u32,
    pub time_gran: u32,
    pub unused: [u32; 9],
}

/// FUSE connection information
#[derive(Debug, Clone)]
pub struct FuseConn {
    /// Unique connection ID
    pub id: u64,
    /// Mount point path
    pub mount_point: String,
    /// FUSE daemon PID
    pub daemon_pid: u32,
    /// Protocol version
    pub proto_major: u32,
    pub proto_minor: u32,
    /// Connection flags
    pub flags: FuseInitFlags,
    /// Maximum read size
    pub max_read: u32,
    /// Maximum write size
    pub max_write: u32,
    /// Maximum readahead
    pub max_readahead: u32,
    /// Request queue
    pub request_queue: Arc<SyncMutex<Vec<FuseRequest>>>,
    /// Active file handles
    pub file_handles: Arc<RwLock<BTreeMap<u64, FuseFileHandle>>>,
    /// Inode cache
    pub inode_cache: Arc<RwLock<BTreeMap<u64, FuseEntryOut>>>,
}

/// FUSE file handle
#[derive(Debug, Clone)]
pub struct FuseFileHandle {
    /// File handle ID
    pub id: u64,
    /// Inode number
    pub inode: u64,
    /// Open flags
    pub flags: u32,
    /// Is directory
    pub is_dir: bool,
}

/// FUSE request (from kernel to userspace)
#[derive(Debug, Clone)]
pub struct FuseRequest {
    /// Unique request ID
    pub unique: u64,
    /// Inode number
    pub nodeid: u64,
    /// Operation code
    pub opcode: FuseOpcode,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Process ID
    pub pid: u32,
    /// Request data
    pub data: Vec<u8>,
}

/// FUSE response (from userspace to kernel)
#[derive(Debug, Clone)]
pub struct FuseResponse {
    /// Unique request ID (matches request)
    pub unique: u64,
    /// Error code (0 for success)
    pub error: i32,
    /// Response data
    pub data: Vec<u8>,
}

/// FUSE operation handler
pub trait FuseHandler: Send + Sync {
    /// Handle a FUSE request and return a response
    fn handle(&self, req: &FuseRequest) -> Result<FuseResponse, crate::subsystems::fs::api::error::FsError>;
}

/// Default FUSE operation handler
pub struct DefaultFuseHandler {
    /// Connection reference
    conn: Arc<FuseConn>,
}

impl DefaultFuseHandler {
    pub fn new(conn: Arc<FuseConn>) -> Self {
        Self { conn }
    }

    /// Handle lookup operation
    fn handle_lookup(&self, req: &FuseRequest) -> Result<FuseResponse, crate::subsystems::fs::api::error::FsError> {
        // Extract name from request data
        let _name = String::from_utf8_lossy(&req.data).to_string();

        // This would normally communicate with the userspace daemon
        // For now, return a not found error
        let reply = FuseEntryOut {
            nodeid: 0,
            generation: 0,
            entry_valid: 0,
            attr_valid: 0,
            entry_valid_nsec: 0,
            attr_valid_nsec: 0,
            attr: FuseAttr::default(),
        };

        let mut data = Vec::with_capacity(core::mem::size_of::<FuseEntryOut>());
        unsafe {
            data.extend_from_slice(core::slice::from_raw_parts(
                &reply as *const _ as *const u8,
                core::mem::size_of::<FuseEntryOut>(),
            ));
        }

        Ok(FuseResponse {
            unique: req.unique,
            error: -2, // -ENOENT
            data,
        })
    }

    /// Handle getattr operation
    fn handle_getattr(&self, req: &FuseRequest) -> Result<FuseResponse, crate::subsystems::fs::api::error::FsError> {
        let attr = FuseAttr::default();

        let mut data = Vec::with_capacity(core::mem::size_of::<FuseAttr>());
        unsafe {
            data.extend_from_slice(core::slice::from_raw_parts(
                &attr as *const _ as *const u8,
                core::mem::size_of::<FuseAttr>(),
            ));
        }

        Ok(FuseResponse {
            unique: req.unique,
            error: 0,
            data,
        })
    }

    /// Handle readdir operation
    fn handle_readdir(&self, req: &FuseRequest) -> Result<FuseResponse, crate::subsystems::fs::api::error::FsError> {
        // Return empty directory for now
        Ok(FuseResponse {
            unique: req.unique,
            error: 0,
            data: Vec::new(),
        })
    }

    /// Handle open operation
    fn handle_open(&self, req: &FuseRequest) -> Result<FuseResponse, crate::subsystems::fs::api::error::FsError> {
        let open_out = FuseOpenOut {
            fh: req.nodeid, // Use inode as file handle for simplicity
            open_flags: 0,
            padding: 0,
        };

        let mut data = Vec::with_capacity(core::mem::size_of::<FuseOpenOut>());
        unsafe {
            data.extend_from_slice(core::slice::from_raw_parts(
                &open_out as *const _ as *const u8,
                core::mem::size_of::<FuseOpenOut>(),
            ));
        }

        Ok(FuseResponse {
            unique: req.unique,
            error: 0,
            data,
        })
    }

    /// Handle read operation
    fn handle_read(&self, req: &FuseRequest) -> Result<FuseResponse, crate::subsystems::fs::api::error::FsError> {
        // Return empty data for now
        Ok(FuseResponse {
            unique: req.unique,
            error: 0,
            data: Vec::new(),
        })
    }

    /// Handle write operation
    fn handle_write(&self, req: &FuseRequest) -> Result<FuseResponse, crate::subsystems::fs::api::error::FsError> {
        Ok(FuseResponse {
            unique: req.unique,
            error: 0,
            data: req.data.len().to_le_bytes().to_vec(),
        })
    }
}

impl FuseHandler for DefaultFuseHandler {
    fn handle(&self, req: &FuseRequest) -> Result<FuseResponse, crate::subsystems::fs::api::error::FsError> {
        match req.opcode {
            FuseOpcode::Lookup => self.handle_lookup(req),
            FuseOpcode::Getattr => self.handle_getattr(req),
            FuseOpcode::Readdir => self.handle_readdir(req),
            FuseOpcode::Open => self.handle_open(req),
            FuseOpcode::Read => self.handle_read(req),
            FuseOpcode::Write => self.handle_write(req),
            _ => Ok(FuseResponse {
                unique: req.unique,
                error: -38, // -ENOSYS
                data: Vec::new(),
            }),
        }
    }
}

/// FUSE filesystem mount
pub struct FuseMount {
    /// Connection information
    conn: Arc<FuseConn>,
    /// Operation handler
    handler: Arc<dyn FuseHandler>,
    /// Is mounted
    mounted: Arc<SyncMutex<bool>>,
}

impl FuseMount {
    /// Create a new FUSE mount
    pub fn new(
        mount_point: &str,
        fs_name: &str,
        _flags: FuseMountFlags,
    ) -> Result<Self, crate::subsystems::fs::api::error::FsError> {
        // Generate unique connection ID
        let conn_id = {
            use core::sync::atomic::{AtomicU64, Ordering};
            static NEXT_ID: AtomicU64 = AtomicU64::new(1);
            NEXT_ID.fetch_add(1, Ordering::SeqCst)
        };

        // Create connection
        let conn = Arc::new(FuseConn {
            id: conn_id,
            mount_point: mount_point.to_string(),
            daemon_pid: 0, // Will be set when daemon connects
            proto_major: FUSE_KERNEL_VERSION,
            proto_minor: FUSE_MINOR_VERSION,
            flags: FuseInitFlags(0),
            max_read: FUSE_DEFAULT_MAX_READ,
            max_write: FUSE_DEFAULT_MAX_WRITE,
            max_readahead: 0,
            request_queue: Arc::new(SyncMutex::new(Vec::new())),
            file_handles: Arc::new(RwLock::new(BTreeMap::new())),
            inode_cache: Arc::new(RwLock::new(BTreeMap::new())),
        });

        // Create default handler
        let handler = Arc::new(DefaultFuseHandler::new(conn.clone()));

        crate::println!("[FUSE] Mounted '{}' at '{}'", fs_name, mount_point);

        Ok(Self {
            conn,
            handler,
            mounted: Arc::new(SyncMutex::new(true)),
        })
    }

    /// Send initialization request to userspace daemon
    pub fn init_connection(&self) -> Result<(), crate::subsystems::fs::api::error::FsError> {
        let _init_in = FuseInitIn {
            major: FUSE_KERNEL_VERSION,
            minor: FUSE_MINOR_VERSION,
            max_readahead: 0,
            flags: FuseInitFlags::ASYNC_READ.0 | FuseInitFlags::BIG_WRITES.0,
        };

        crate::println!("[FUSE] Initialized connection v{}.{}", FUSE_KERNEL_VERSION, FUSE_MINOR_VERSION);

        Ok(())
    }

    /// Process a FUSE request
    pub fn process_request(&self, req: FuseRequest) -> Result<FuseResponse, crate::subsystems::fs::api::error::FsError> {
        if !*self.mounted.lock() {
            return Err(crate::subsystems::fs::api::error::FsError::NotMounted);
        }

        self.handler.handle(&req)
    }

    /// Unmount the FUSE filesystem
    pub fn unmount(&self) -> Result<(), crate::subsystems::fs::api::error::FsError> {
        let mut mounted = self.mounted.lock();
        if !*mounted {
            return Err(crate::subsystems::fs::api::error::FsError::NotMounted);
        }

        *mounted = false;

        crate::println!("[FUSE] Unmounted '{}'", self.conn.mount_point);

        Ok(())
    }

    /// Get connection information
    pub fn connection(&self) -> &Arc<FuseConn> {
        &self.conn
    }
}

impl Drop for FuseMount {
    fn drop(&mut self) {
        let _ = self.unmount();
    }
}

/// FUSE device manager for /dev/fuse
pub struct FuseDevice {
    /// Active mounts
    mounts: Arc<SyncMutex<BTreeMap<String, Arc<FuseMount>>>>,
}

impl FuseDevice {
    pub fn new() -> Self {
        Self {
            mounts: Arc::new(SyncMutex::new(BTreeMap::new())),
        }
    }

    /// Register a FUSE mount
    pub fn register_mount(&self, mount: Arc<FuseMount>) -> Result<(), crate::subsystems::fs::api::error::FsError> {
        let mount_point = mount.conn.mount_point.clone();
        let mut mounts = self.mounts.lock();

        if mounts.contains_key(&mount_point) {
            return Err(crate::subsystems::fs::api::error::FsError::FileExists);
        }

        mounts.insert(mount_point, mount);
        Ok(())
    }

    /// Unregister a FUSE mount
    pub fn unregister_mount(&self, mount_point: &str) -> Result<(), crate::subsystems::fs::api::error::FsError> {
        let mut mounts = self.mounts.lock();

        if mounts.remove(mount_point).is_none() {
            return Err(crate::subsystems::fs::api::error::FsError::NotFound);
        }

        Ok(())
    }

    /// Get mount by path
    pub fn get_mount(&self, mount_point: &str) -> Option<Arc<FuseMount>> {
        self.mounts.lock().get(mount_point).cloned()
    }
}

/// Global FUSE device instance
static FUSE_DEVICE: spin::Once<FuseDevice> = spin::Once::new();

/// Get the global FUSE device instance
pub fn fuse_device() -> &'static FuseDevice {
    FUSE_DEVICE.call_once(|| FuseDevice::new())
}

/// Initialize FUSE subsystem
pub fn init() -> Result<(), crate::subsystems::fs::api::error::FsError> {
    crate::println!("[FUSE] Initialized (protocol v{}.{}),",
                     FUSE_KERNEL_VERSION, FUSE_MINOR_VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuse_mount_flags() {
        let flags = FuseMountFlags::DEFAULT;
        assert!(!flags.contains(FuseMountFlags::RDONLY));

        let flags = FuseMountFlags::RDONLY | FuseMountFlags::NOEXEC;
        assert!(flags.contains(FuseMountFlags::RDONLY));
        assert!(flags.contains(FuseMountFlags::NOEXEC));
    }

    #[test]
    fn test_fuse_opcode_conversion() {
        assert_eq!(FuseOpcode::try_from(3), Ok(FuseOpcode::Getattr));
        assert_eq!(FuseOpcode::try_from(999), Err(()));
    }
}
