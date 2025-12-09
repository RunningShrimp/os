//! POSIX Types and Constants
//!
//! Standard types and constants for POSIX compliance

use alloc::string::ToString;


// ============================================================================
// Basic Types
// ============================================================================

/// Process ID type
pub type Pid = usize;

/// User ID type  
pub type Uid = u32;

/// Group ID type
pub type Gid = u32;

/// File mode type
pub type Mode = u32;

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

// ============================================================================
// File Mode Bits
// ============================================================================

/// Set-user-ID on execution
pub const S_ISUID: Mode = 0o4000;

/// Set-group-ID on execution
pub const S_ISGID: Mode = 0o2000;

/// Sticky bit
pub const S_ISVTX: Mode = 0o1000;

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

// ============================================================================
// Access Permission Constants
// ============================================================================

/// Test for read permission
pub const R_OK: i32 = 4;

/// Test for write permission
pub const W_OK: i32 = 2;

/// Test for execute permission
pub const X_OK: i32 = 1;

/// Test for existence of file
pub const F_OK: i32 = 0;

// File type bits
pub const S_IFMT: Mode = 0o170000;   // File type mask
pub const S_IFSOCK: Mode = 0o140000; // Socket
pub const S_IFLNK: Mode = 0o120000;  // Symbolic link
pub const S_IFREG: Mode = 0o100000;  // Regular file
pub const S_IFBLK: Mode = 0o060000;  // Block device
pub const S_IFDIR: Mode = 0o040000;  // Directory
pub const S_IFCHR: Mode = 0o020000;  // Character device
pub const S_IFIFO: Mode = 0o010000;  // FIFO

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

// ============================================================================
// Open Flags (fcntl.h)
// ============================================================================

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

/// Don't assign controlling terminal
pub const O_NOCTTY: i32 = 0o400;

/// Truncate file to zero length
pub const O_TRUNC: i32 = 0o1000;

/// Append on each write
pub const O_APPEND: i32 = 0o2000;

/// Non-blocking mode
pub const O_NONBLOCK: i32 = 0x0004;
pub const MSG_DONTWAIT: i32 = 0x40;
pub const SOL_SOCKET: i32 = 1;
pub const SO_REUSEADDR: i32 = 2;
pub const SO_TYPE: i32 = 3;
pub const SO_ERROR: i32 = 4;
pub const SO_KEEPALIVE: i32 = 9;
pub const SO_LINGER: i32 = 13;
pub const SO_REUSEPORT: i32 = 15;
pub const SO_SNDBUF: i32 = 0x1001;
pub const SO_RCVBUF: i32 = 0x1002;
pub const SO_RCVTIMEO: i32 = 20;
pub const SO_SNDTIMEO: i32 = 21;
pub const ECONNREFUSED: i32 = 111;
pub const O_NDELAY: i32 = O_NONBLOCK;

/// Synchronous writes
pub const O_SYNC: i32 = 0o10000;
pub const O_DSYNC: i32 = O_SYNC;

/// Open directory
pub const O_DIRECTORY: i32 = 0o200000;

/// Don't follow symbolic links
pub const O_NOFOLLOW: i32 = 0o400000;

/// Close on exec
pub const O_CLOEXEC: i32 = 0o2000000;

/// Direct I/O
pub const O_DIRECT: i32 = 0o40000;

/// Large file (for 32-bit systems)
pub const O_LARGEFILE: i32 = 0o100000;

/// Do not update access time
pub const O_NOATIME: i32 = 0o1000000;

/// Open path only (for linkat/fstatat etc)
pub const O_PATH: i32 = 0o10000000;

/// Create temporary file
pub const O_TMPFILE: i32 = 0o20000000;

// ============================================================================
// Seek Whence Values
// ============================================================================

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

// ============================================================================
// fcntl Commands
// ============================================================================

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

/// Get record locking info
pub const F_GETLK: i32 = 5;

/// Set record locking info (non-blocking)
pub const F_SETLK: i32 = 6;

/// Set record locking info (blocking)
pub const F_SETLKW: i32 = 7;

/// Get owner PID
pub const F_GETOWN: i32 = 9;

/// Set owner PID
pub const F_SETOWN: i32 = 8;

/// Duplicate file descriptor (close-on-exec)
pub const F_DUPFD_CLOEXEC: i32 = 1030;

/// File descriptor flags
pub const FD_CLOEXEC: i32 = 1;

// ============================================================================
// Stat Structure
// ============================================================================

/// POSIX stat structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Stat {
    /// Device ID
    pub st_dev: Dev,
    /// Inode number
    pub st_ino: Ino,
    /// File mode
    pub st_mode: Mode,
    /// Number of hard links
    pub st_nlink: Nlink,
    /// User ID of owner
    pub st_uid: Uid,
    /// Group ID of owner
    pub st_gid: Gid,
    /// Device ID (if special file)
    pub st_rdev: Dev,
    /// Total size in bytes
    pub st_size: Off,
    /// Block size for filesystem I/O
    pub st_blksize: Blksize,
    /// Number of 512B blocks allocated
    pub st_blocks: Blkcnt,
    /// Time of last access
    pub st_atime: Time,
    /// Nanoseconds of last access
    pub st_atime_nsec: i64,
    /// Time of last modification
    pub st_mtime: Time,
    /// Nanoseconds of last modification
    pub st_mtime_nsec: i64,
    /// Time of last status change
    pub st_ctime: Time,
    /// Nanoseconds of last status change
    pub st_ctime_nsec: i64,
}

// ============================================================================
// Timespec Structure
// ============================================================================

/// Time specification structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Timespec {
    /// Seconds
    pub tv_sec: Time,
    /// Nanoseconds
    pub tv_nsec: i64,
}

impl Timespec {
    pub const fn new(sec: Time, nsec: i64) -> Self {
        Self { tv_sec: sec, tv_nsec: nsec }
    }
    
    pub const fn zero() -> Self {
        Self { tv_sec: 0, tv_nsec: 0 }
    }
    
    /// Convert to nanoseconds
    pub const fn to_nanos(&self) -> i64 {
        self.tv_sec * 1_000_000_000 + self.tv_nsec
    }
    
    /// Create from nanoseconds
    pub const fn from_nanos(nanos: i64) -> Self {
        Self {
            tv_sec: nanos / 1_000_000_000,
            tv_nsec: nanos % 1_000_000_000,
        }
    }
}

/// Timeval structure (microseconds precision)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Timeval {
    /// Seconds
    pub tv_sec: Time,
    /// Microseconds
    pub tv_usec: i64,
}

// ============================================================================
// Clock IDs
// ============================================================================

/// System-wide real-time clock
pub const CLOCK_REALTIME: ClockId = 0;

/// Monotonic clock (cannot be set)
pub const CLOCK_MONOTONIC: ClockId = 1;

/// Per-process CPU time clock
pub const CLOCK_PROCESS_CPUTIME_ID: ClockId = 2;

/// Per-thread CPU time clock
pub const CLOCK_THREAD_CPUTIME_ID: ClockId = 3;

/// Monotonic raw (not adjusted by NTP)
pub const CLOCK_MONOTONIC_RAW: ClockId = 4;

/// Real-time clock (coarse resolution)
pub const CLOCK_REALTIME_COARSE: ClockId = 5;

/// Monotonic clock (coarse resolution)
pub const CLOCK_MONOTONIC_COARSE: ClockId = 6;

/// Boottime clock (includes suspend time)
pub const CLOCK_BOOTTIME: ClockId = 7;

// ============================================================================
// Wait Options
// ============================================================================

/// Return immediately if no child has exited
pub const WNOHANG: i32 = 1;

/// Also return if a child has stopped
pub const WUNTRACED: i32 = 2;

/// Also return if a stopped child has been resumed
pub const WCONTINUED: i32 = 8;

/// Wait for any child
pub const WAIT_ANY: usize = usize::MAX;

/// Wait for any child in process group
pub const WAIT_MYPGRP: Pid = 0;

/// Macros for wait status
#[inline]
pub const fn wifexited(status: i32) -> bool {
    (status & 0x7f) == 0
}

#[inline]
pub const fn wexitstatus(status: i32) -> i32 {
    (status >> 8) & 0xff
}

#[inline]
pub const fn wifsignaled(status: i32) -> bool {
    ((status & 0x7f) + 1) >> 1 > 0
}

#[inline]
pub const fn wtermsig(status: i32) -> i32 {
    status & 0x7f
}

#[inline]
pub const fn wifstopped(status: i32) -> bool {
    (status & 0xff) == 0x7f
}

#[inline]
pub const fn wstopsig(status: i32) -> i32 {
    wexitstatus(status)
}

#[inline]
pub const fn wifcontinued(status: i32) -> bool {
    status == 0xffff
}

/// Construct exit status
#[inline]
pub const fn w_exitcode(ret: i32, sig: i32) -> i32 {
    ((ret) << 8) | (sig)
}

// ============================================================================
// Dirent Structure
// ============================================================================

/// Maximum name length
pub const NAME_MAX: usize = 255;

/// Maximum path length
pub const PATH_MAX: usize = 4096;

/// Directory entry structure
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Dirent {
    /// Inode number
    pub d_ino: Ino,
    /// Offset to next dirent
    pub d_off: Off,
    /// Length of this dirent
    pub d_reclen: u16,
    /// File type
    pub d_type: u8,
    /// Filename (null-terminated)
    pub d_name: [u8; NAME_MAX + 1],
}

impl Default for Dirent {
    fn default() -> Self {
        Self {
            d_ino: 0,
            d_off: 0,
            d_reclen: 0,
            d_type: 0,
            d_name: [0; NAME_MAX + 1],
        }
    }
}

/// Directory entry types
pub const DT_UNKNOWN: u8 = 0;
pub const DT_FIFO: u8 = 1;
pub const DT_CHR: u8 = 2;
pub const DT_DIR: u8 = 4;
pub const DT_BLK: u8 = 6;
pub const DT_REG: u8 = 8;
pub const DT_LNK: u8 = 10;
pub const DT_SOCK: u8 = 12;
pub const DT_WHT: u8 = 14;

// ============================================================================
// Poll/Select Structures
// ============================================================================

/// Poll file descriptor structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PollFd {
    /// File descriptor
    pub fd: i32,
    /// Requested events
    pub events: i16,
    /// Returned events
    pub revents: i16,
}

/// Poll event flags
pub const POLLIN: i16 = 0x0001;     // There is data to read
pub const POLLPRI: i16 = 0x0002;    // There is urgent data to read
pub const POLLOUT: i16 = 0x0004;    // Writing now will not block
pub const POLLERR: i16 = 0x0008;    // Error condition
pub const POLLHUP: i16 = 0x0010;    // Hung up
pub const POLLNVAL: i16 = 0x0020;   // Invalid polling request
pub const POLLRDNORM: i16 = 0x0040; // Normal data may be read
pub const POLLRDBAND: i16 = 0x0080; // Priority data may be read
pub const POLLWRNORM: i16 = 0x0100; // Writing now will not block
pub const POLLWRBAND: i16 = 0x0200; // Priority data may be written

/// Maximum number of file descriptors in fd_set
pub const FD_SETSIZE: usize = 1024;

/// fd_set structure for select()
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FdSet {
    pub bits: [u64; (FD_SETSIZE + 63) / 64],
}

impl Default for FdSet {
    fn default() -> Self { Self { bits: [0; (FD_SETSIZE + 63) / 64] } }
}

#[inline]
pub fn fd_zero(set: &mut FdSet) { for b in set.bits.iter_mut() { *b = 0; } }

#[inline]
pub fn fd_set(set: &mut FdSet, fd: i32) {
    if fd >= 0 && (fd as usize) < FD_SETSIZE {
        let i = (fd as usize) / 64; let o = (fd as usize) % 64; set.bits[i] |= 1u64 << o;
    }
}

#[inline]
pub fn fd_clr(set: &mut FdSet, fd: i32) {
    if fd >= 0 && (fd as usize) < FD_SETSIZE {
        let i = (fd as usize) / 64; let o = (fd as usize) % 64; set.bits[i] &= !(1u64 << o);
    }
}

#[inline]
pub fn fd_isset(set: &FdSet, fd: i32) -> bool {
    if fd >= 0 && (fd as usize) < FD_SETSIZE {
        let i = (fd as usize) / 64; let o = (fd as usize) % 64; (set.bits[i] & (1u64 << o)) != 0
    } else { false }
}

// ============================================================================
// Resource Limits
// ============================================================================

/// Resource limit structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Rlimit {
    /// Soft limit
    pub rlim_cur: u64,
    /// Hard limit (ceiling for soft limit)
    pub rlim_max: u64,
}

/// Unlimited resource
pub const RLIM_INFINITY: u64 = !0;

/// Resource limit types
pub const RLIMIT_CPU: i32 = 0;      // CPU time in seconds
pub const RLIMIT_FSIZE: i32 = 1;    // Maximum file size
pub const RLIMIT_DATA: i32 = 2;     // Maximum data size
pub const RLIMIT_STACK: i32 = 3;    // Maximum stack size
pub const RLIMIT_CORE: i32 = 4;     // Maximum core file size
pub const RLIMIT_RSS: i32 = 5;      // Maximum resident set size
pub const RLIMIT_NPROC: i32 = 6;    // Maximum number of processes
pub const RLIMIT_NOFILE: i32 = 7;   // Maximum number of open files
pub const RLIMIT_MEMLOCK: i32 = 8;  // Maximum locked-in-memory address space
pub const RLIMIT_AS: i32 = 9;       // Maximum address space
pub const RLIMIT_LOCKS: i32 = 10;   // Maximum file locks held
pub const RLIMIT_SIGPENDING: i32 = 11; // Maximum pending signals
pub const RLIMIT_MSGQUEUE: i32 = 12;   // Maximum message queue bytes
pub const RLIMIT_NICE: i32 = 13;    // Maximum nice priority
pub const RLIMIT_RTPRIO: i32 = 14;  // Maximum real-time priority
pub const RLIMIT_RTTIME: i32 = 15;  // Timeout for RT tasks

// ============================================================================
// Mmap Flags
// ============================================================================

/// Page can be read
pub const PROT_READ: i32 = 0x1;

/// Page can be written
pub const PROT_WRITE: i32 = 0x2;

/// Page can be executed
pub const PROT_EXEC: i32 = 0x4;

/// Page cannot be accessed
pub const PROT_NONE: i32 = 0x0;

/// Share changes
pub const MAP_SHARED: i32 = 0x01;

/// Changes are private
pub const MAP_PRIVATE: i32 = 0x02;

/// Interpret addr exactly
pub const MAP_FIXED: i32 = 0x10;

/// Don't use a file
pub const MAP_ANONYMOUS: i32 = 0x20;
pub const MAP_ANON: i32 = MAP_ANONYMOUS;

/// Failed mmap return value
pub const MAP_FAILED: usize = !0;

// ============================================================================
// Advanced Memory Mapping Constants
// ============================================================================

/// Additional mmap flags
pub const MAP_LOCKED: i32 = 0x2000;     // Lock pages in memory
pub const MAP_NORESERVE: i32 = 0x4000;  // Don't reserve swap space
pub const MAP_POPULATE: i32 = 0x8000;   // Populate page tables
pub const MAP_NONBLOCK: i32 = 0x10000;  // Don't block on I/O
pub const MAP_STACK: i32 = 0x20000;      // Allocation for stack
pub const MAP_HUGETLB: i32 = 0x40000;    // Create huge page mapping
pub const MAP_GROWSDOWN: i32 = 0x100;   // Stack-like segment
pub const MAP_DENYWRITE: i32 = 0x800;    // Deny write access
pub const MAP_EXECUTABLE: i32 = 0x1000;   // Mark it as executable
pub const MAP_HUGE_SHIFT: i32 = 26;       // Huge page size shift
pub const MAP_HUGE_MASK: i32 = 0x3f << MAP_HUGE_SHIFT;

/// Huge page size constants
pub const MAP_HUGE_64KB: i32 = 16 << MAP_HUGE_SHIFT;
pub const MAP_HUGE_512KB: i32 = 19 << MAP_HUGE_SHIFT;
pub const MAP_HUGE_1MB: i32 = 20 << MAP_HUGE_SHIFT;
pub const MAP_HUGE_2MB: i32 = 21 << MAP_HUGE_SHIFT;
pub const MAP_HUGE_8MB: i32 = 23 << MAP_HUGE_SHIFT;
pub const MAP_HUGE_16MB: i32 = 24 << MAP_HUGE_SHIFT;
pub const MAP_HUGE_32MB: i32 = 25 << MAP_HUGE_SHIFT;
pub const MAP_HUGE_256MB: i32 = 26 << MAP_HUGE_SHIFT;
pub const MAP_HUGE_512MB: i32 = 27 << MAP_HUGE_SHIFT;
pub const MAP_HUGE_1GB: i32 = 28 << MAP_HUGE_SHIFT;
pub const MAP_HUGE_2GB: i32 = 29 << MAP_HUGE_SHIFT;
pub const MAP_HUGE_16GB: i32 = 34 << MAP_HUGE_SHIFT;

/// madvise advice values
pub const MADV_NORMAL: i32 = 0;     // No special treatment
pub const MADV_RANDOM: i32 = 1;    // Expect random page references
pub const MADV_SEQUENTIAL: i32 = 2; // Expect sequential page references
pub const MADV_WILLNEED: i32 = 3;  // Will need these pages
pub const MADV_DONTNEED: i32 = 4;  // Don't need these pages
pub const MADV_FREE: i32 = 8;       // Pages can be freed
pub const MADV_REMOVE: i32 = 9;     // Remove pages from memory
pub const MADV_DONTFORK: i32 = 10;  // Don't inherit across fork
pub const MADV_DOFORK: i32 = 11;    // Do inherit across fork
pub const MADV_MERGEABLE: i32 = 12; // KSM may merge pages
pub const MADV_UNMERGEABLE: i32 = 13; // KSM may not merge pages
pub const MADV_HUGEPAGE: i32 = 14;  // Use huge pages
pub const MADV_NOHUGEPAGE: i32 = 15; // Don't use huge pages
pub const MADV_DONTDUMP: i32 = 16;  // Exclude from core dump
pub const MADV_DODUMP: i32 = 17;    // Include in core dump
pub const MADV_HWPOISON: i32 = 100;  // Poison a page

/// mlockall flags
pub const MCL_CURRENT: i32 = 1;  // Lock currently mapped pages
pub const MCL_FUTURE: i32 = 2;   // Lock future mappings
pub const MCL_ONFAULT: i32 = 4;  // Lock pages on first fault

/// remap_file_pages flags
pub const MAP_FILE: i32 = 0;      // Mapped from file (default)
pub const MAP_RENAME: i32 = 0;    // Rename mapping (Linux-specific)

// ============================================================================
// Rusage Structure
// ============================================================================

/// Resource usage structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Rusage {
    /// User time used
    pub ru_utime: Timeval,
    /// System time used
    pub ru_stime: Timeval,
    /// Maximum resident set size
    pub ru_maxrss: i64,
    /// Integral shared memory size
    pub ru_ixrss: i64,
    /// Integral unshared data size
    pub ru_idrss: i64,
    /// Integral unshared stack size
    pub ru_isrss: i64,
    /// Page reclaims (soft page faults)
    pub ru_minflt: i64,
    /// Page faults (hard page faults)
    pub ru_majflt: i64,
    /// Swaps
    pub ru_nswap: i64,
    /// Block input operations
    pub ru_inblock: i64,
    /// Block output operations
    pub ru_oublock: i64,
    /// IPC messages sent
    pub ru_msgsnd: i64,
    /// IPC messages received
    pub ru_msgrcv: i64,
    /// Signals received
    pub ru_nsignals: i64,
    /// Voluntary context switches
    pub ru_nvcsw: i64,
    /// Involuntary context switches
    pub ru_nivcsw: i64,
}

/// Resource usage targets
pub const RUSAGE_SELF: i32 = 0;
pub const RUSAGE_CHILDREN: i32 = -1;
pub const RUSAGE_THREAD: i32 = 1;

// ============================================================================
// Utsname Structure
// ============================================================================

/// System name structure
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Utsname {
    /// Operating system name
    pub sysname: [u8; 65],
    /// Network node hostname
    pub nodename: [u8; 65],
    /// Operating system release
    pub release: [u8; 65],
    /// Operating system version
    pub version: [u8; 65],
    /// Hardware identifier
    pub machine: [u8; 65],
    /// NIS or YP domain name
    pub domainname: [u8; 65],
}

impl Default for Utsname {
    fn default() -> Self {
        Self {
            sysname: [0; 65],
            nodename: [0; 65],
            release: [0; 65],
            version: [0; 65],
            machine: [0; 65],
            domainname: [0; 65],
        }
    }
}

// ============================================================================
// I/O Vector
// ============================================================================

/// I/O vector for scatter/gather I/O
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IoVec {
    /// Base address
    pub iov_base: *mut u8,
    /// Length
    pub iov_len: usize,
}

impl Default for IoVec {
    fn default() -> Self {
        Self {
            iov_base: core::ptr::null_mut(),
            iov_len: 0,
        }
    }
}

/// Maximum number of I/O vectors
pub const IOV_MAX: usize = 1024;

// ============================================================================
// epoll Definitions (sys/epoll.h)
// ============================================================================

/// epoll control operations
pub const EPOLL_CTL_ADD: i32 = 1;
pub const EPOLL_CTL_DEL: i32 = 2;
pub const EPOLL_CTL_MOD: i32 = 3;

/// epoll event flags
pub const EPOLLIN: u32 = 0x001;
pub const EPOLLPRI: u32 = 0x002;
pub const EPOLLOUT: u32 = 0x004;
pub const EPOLLERR: u32 = 0x008;
pub const EPOLLHUP: u32 = 0x010;
pub const EPOLLRDHUP: u32 = 0x2000;
pub const EPOLLONESHOT: u32 = 0x40000000;
pub const EPOLLET: u32 = 0x80000000;

/// epoll create flags
pub const EPOLL_CLOEXEC: i32 = 0x00020000;

/// epoll event structure
#[repr(C)]
#[derive(Debug, Clone)]
pub struct EpollEvent {
    /// Events mask
    pub events: u32,
    /// User data
    pub data: EpollData,
}

impl Default for EpollEvent {
    fn default() -> Self {
        Self {
            events: 0,
            data: EpollData { u64: 0 },
        }
    }
}

/// epoll data union
#[repr(C)]
pub union EpollData {
    pub ptr: *mut u8,
    pub fd: i32,
    pub u32: u32,
    pub u64: u64,
}

// Safety: EpollData is used for epoll event data, which is typically
// accessed from a single thread context. The raw pointer is only used
// for user-space data passing and is not shared across threads.
unsafe impl Send for EpollData {}
unsafe impl Sync for EpollData {}

impl Clone for EpollData {
    fn clone(&self) -> Self {
        unsafe { Self { u64: self.u64 } }
    }
}

impl Default for EpollData {
    fn default() -> Self {
        Self { u64: 0 }
    }
}

impl core::fmt::Debug for EpollData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unsafe { f.debug_struct("EpollData").field("u64", &self.u64).finish() }
    }
}

// ============================================================================
// Socket Definitions (sys/socket.h)
// ============================================================================

/// Socket address structure (generic)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Sockaddr {
    /// Address family
    pub sa_family: u16,
    /// Address data
    pub sa_data: [u8; 14],
}

/// Socket address structure (IPv4)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SockaddrIn {
    /// Address family (AF_INET)
    pub sin_family: u16,
    /// Port number
    pub sin_port: u16,
    /// IPv4 address
    pub sin_addr: InAddr,
    /// Padding
    pub sin_zero: [u8; 8],
}

/// IPv4 address structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct InAddr {
    /// IPv4 address in network byte order
    pub s_addr: u32,
}

/// Address families
pub const AF_UNSPEC: i32 = 0;       // Unspecified
pub const AF_INET: i32 = 2;         // IPv4
pub const AF_INET6: i32 = 10;       // IPv6

/// Socket types
pub const SOCK_STREAM: i32 = 1;     // TCP
pub const SOCK_DGRAM: i32 = 2;      // UDP
pub const SOCK_RAW: i32 = 3;        // Raw socket
pub const SOCK_SEQPACKET: i32 = 5;  // Sequenced packet socket

/// Protocol families
pub const PF_UNSPEC: i32 = AF_UNSPEC;
pub const PF_INET: i32 = AF_INET;
pub const PF_INET6: i32 = AF_INET6;

/// Socket option levels
// Socket option level and options defined earlier in this file

/// Shutdown how
pub const SHUT_RD: i32 = 0;          // Shutdown read
pub const SHUT_WR: i32 = 1;          // Shutdown write
pub const SHUT_RDWR: i32 = 2;        // Shutdown both

// ============================================================================
// Clone Flags (sched.h)
// ============================================================================

/// Clone flags for clone() system call
/// These flags control what is shared between parent and child

/// Share virtual memory space
pub const CLONE_VM: i32 = 0x00000100;
/// Share file descriptor table
pub const CLONE_FILES: i32 = 0x00000400;
/// Share filesystem information
pub const CLONE_FS: i32 = 0x00000200;
/// Share signal handlers
pub const CLONE_SIGHAND: i32 = 0x00000800;
/// Share parent's PID (thread in same thread group)
pub const CLONE_THREAD: i32 = 0x00010000;
/// Share System V IPC semaphore undo lists
pub const CLONE_SYSVSEM: i32 = 0x00040000;
/// Set child's parent to caller's parent
pub const CLONE_PARENT: i32 = 0x00008000;
/// Set child's parent to caller
pub const CLONE_PARENT_SETTID: i32 = 0x00100000;
/// Clear child's TID in child memory
pub const CLONE_CHILD_CLEARTID: i32 = 0x00200000;
/// Set child's TID in child memory
pub const CLONE_CHILD_SETTID: i32 = 0x01000000;
/// New uts namespace
pub const CLONE_NEWUTS: i32 = 0x04000000;
/// New IPC namespace
pub const CLONE_NEWIPC: i32 = 0x08000000;
/// New user namespace
pub const CLONE_NEWUSER: i32 = 0x10000000;
/// New PID namespace
pub const CLONE_NEWPID: i32 = 0x20000000;
/// New network namespace
pub const CLONE_NEWNET: i32 = 0x40000000;
/// New mount namespace
pub const CLONE_NEWNS: i32 = 0x00020000;
/// Share I/O context
// pub const CLONE_IO: i32 = 0x80000000; // Commented out due to i32 overflow

// ============================================================================
// Signal Definitions (signal.h)
// ============================================================================

/// Signal set structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SigSet {
    /// Signal bits (up to 64 signals)
    pub bits: u64,
}

impl SigSet {
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn all() -> Self {
        Self { bits: !0 }
    }

    pub const fn has(&self, sig: i32) -> bool {
        if sig > 0 && sig <= 64 {
            (self.bits >> (sig - 1)) & 1 != 0
        } else {
            false
        }
    }

    pub const fn add(&mut self, sig: i32) {
        if sig > 0 && sig <= 64 {
            self.bits |= 1 << (sig - 1);
        }
    }

    pub const fn remove(&mut self, sig: i32) {
        if sig > 0 && sig <= 64 {
            self.bits &= !(1 << (sig - 1));
        }
    }

    pub const fn clear(&mut self) {
        self.bits = 0;
    }
}

/// Signal action structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SigAction {
    /// Signal handler or SIG_IGN/SIG_DFL
    pub sa_handler: usize,
    /// Signal mask to apply during handler execution
    pub sa_mask: SigSet,
    /// Flags for signal handling
    pub sa_flags: i32,
    /// Restorer function (obsolete)
    pub sa_restorer: usize,
}

impl Default for SigAction {
    fn default() -> Self {
        Self {
            sa_handler: SIG_DFL,
            sa_mask: SigSet::empty(),
            sa_flags: 0,
            sa_restorer: 0,
        }
    }
}

/// Special signal handler values
pub const SIG_DFL: usize = 0;      // Default handler
pub const SIG_IGN: usize = 1;      // Ignore signal
pub const SIG_ERR: isize = -1;     // Error return value

/// Signal handling flags
pub const SA_NOCLDSTOP: i32 = 0x00000001;  // Don't send SIGCHLD when child stops
pub const SA_NOCLDWAIT: i32 = 0x00000002;  // Don't create zombies on child termination
pub const SA_SIGINFO: i32 = 0x00000004;    // Use sa_sigaction instead of sa_handler
pub const SA_ONSTACK: i32 = 0x08000000;    // Use alternate signal stack
pub const SA_RESTART: i32 = 0x10000000;    // Restart syscall on signal
pub const SA_NODEFER: i32 = 0x40000000;    // Don't automatically block the signal
pub const SA_RESETHAND: i32 = -2147483648;  // Reset to SIG_DFL on signal delivery

/// Signal numbers (POSIX.1-2008)
pub const SIGHUP: i32 = 1;     // Hangup
pub const SIGINT: i32 = 2;     // Interrupt
pub const SIGQUIT: i32 = 3;    // Quit
pub const SIGILL: i32 = 4;     // Illegal instruction
pub const SIGTRAP: i32 = 5;    // Trace/breakpoint trap
pub const SIGABRT: i32 = 6;    // Abort
pub const SIGBUS: i32 = 7;     // Bus error
pub const SIGFPE: i32 = 8;     // Floating point exception
pub const SIGKILL: i32 = 9;    // Kill (cannot be caught or ignored)
pub const SIGUSR1: i32 = 10;   // User-defined signal 1
pub const SIGSEGV: i32 = 11;   // Segmentation violation
pub const SIGUSR2: i32 = 12;   // User-defined signal 2
pub const SIGPIPE: i32 = 13;   // Broken pipe
pub const SIGALRM: i32 = 14;   // Alarm clock
pub const SIGTERM: i32 = 15;   // Termination
pub const SIGSTKFLT: i32 = 16; // Stack fault on coprocessor
pub const SIGCHLD: i32 = 17;   // Child status has changed
pub const SIGCONT: i32 = 18;   // Continue (if stopped)
pub const SIGSTOP: i32 = 19;   // Stop (cannot be caught or ignored)
pub const SIGTSTP: i32 = 20;   // Stop signal generated from keyboard
pub const SIGTTIN: i32 = 21;   // Background read from tty
pub const SIGTTOU: i32 = 22;   // Background write to tty
pub const SIGURG: i32 = 23;    // Urgent condition on socket
pub const SIGXCPU: i32 = 24;   // CPU limit exceeded
pub const SIGXFSZ: i32 = 25;   // File size limit exceeded
pub const SIGVTALRM: i32 = 26; // Virtual timer expired
pub const SIGPROF: i32 = 27;   // Profiling timer expired
pub const SIGWINCH: i32 = 28;  // Window size change
pub const SIGIO: i32 = 29;     // I/O now possible
pub const SIGPWR: i32 = 30;    // Power failure
pub const SIGSYS: i32 = 31;    // Bad system call

/// Real-time signal range
pub const SIGRTMIN: i32 = 32;
pub const SIGRTMAX: i32 = 64;

/// Alternate signal stack structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StackT {
    /// Base address of stack
    pub ss_sp: *mut u8,
    /// Stack flags
    pub ss_flags: i32,
    /// Stack size
    pub ss_size: usize,
}

impl Default for StackT {
    fn default() -> Self {
        Self {
            ss_sp: core::ptr::null_mut(),
            ss_flags: SS_DISABLE,
            ss_size: 0,
        }
    }
}

/// Stack flags
pub const SS_ONSTACK: i32 = 1;   // Currently executing on alternate stack
pub const SS_DISABLE: i32 = 2;   // Alternate stack disabled

/// Signal information structure (for SA_SIGINFO)
#[repr(C)]
#[derive(Debug, Clone)]
pub struct SigInfoT {
    /// Signal number
    pub si_signo: i32,
    /// Signal code
    pub si_code: i32,
    /// Sending process ID
    pub si_pid: Pid,
    /// Sending process UID
    pub si_uid: Uid,
    /// Exit value or signal
    pub si_status: i32,
    /// User time consumed
    pub si_utime: Time,
    /// System time consumed
    pub si_stime: Time,
    /// Signal value
    pub si_value: SigVal,
    /// POSIX.1b timer ID
    pub si_timerid: i32,
    /// POSIX.1b timer overrun count
    pub si_overrun: i32,
    /// Faulting address
    pub si_addr: usize,
    /// Band event
    pub si_band: i64,
    /// File descriptor
    pub si_fd: i32,
}

/// Signal value union
#[repr(C)]
pub union SigVal {
    /// Integer value
    pub sival_int: i32,
    /// Pointer value
    pub sival_ptr: *mut u8,
}

// SAFETY: SigVal only contains integer and raw pointer values, which are Send + Sync
unsafe impl Send for SigVal {}
unsafe impl Sync for SigVal {}

impl Clone for SigVal {
    fn clone(&self) -> Self {
        unsafe { Self { sival_int: self.sival_int } }
    }
}

impl Copy for SigVal {}

impl Default for SigVal {
    fn default() -> Self {
        Self { sival_int: 0 }
    }
}

impl core::fmt::Debug for SigVal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unsafe { f.debug_struct("SigVal").field("sival_int", &self.sival_int).finish() }
    }
}

/// Signal codes
pub const SI_USER: i32 = 0;        // Signal sent by kill()
pub const SI_KERNEL: i32 = 0x80;   // Signal sent by kernel
pub const SI_QUEUE: i32 = -1;      // Signal sent by sigqueue()
pub const SI_TIMER: i32 = -2;      // Signal sent by timer expiration
pub const SI_MESGQ: i32 = -3;      // Signal sent by message arrival
pub const SI_ASYNCIO: i32 = -4;    // Signal sent by AIO completion
pub const SI_SIGIO: i32 = -5;      // Signal sent by queued SIGIO

/// Signal mask manipulation operations
pub const SIG_BLOCK: i32 = 0;      // Block signals
pub const SIG_UNBLOCK: i32 = 1;    // Unblock signals
pub const SIG_SETMASK: i32 = 2;    // Set the signal mask

/// Minimum stack size for alternate signal stack
pub const MINSIGSTKSZ: usize = 2048;
/// Recommended stack size for alternate signal stack
pub const SIGSTKSZ: usize = 8192;

/// Number of signals
pub const NSIG: i32 = 65;

// ============================================================================
// Semaphore Definitions (semaphore.h)
// ============================================================================

/// POSIX semaphore structure
#[repr(C)]
pub struct SemT {
    /// Internal semaphore implementation
    pub sem_internal: *mut u8,
}

impl SemT {
    /// Check if semaphore is null/uninitialized
    pub fn is_null(&self) -> bool {
        self.sem_internal.is_null()
    }
}

/// Semaphore permissions
pub const SEM_PERMISSIONS: Mode = 0o600;

// ============================================================================
// Message Queue Definitions (mqueue.h)
// ============================================================================

/// Message queue attributes
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MqAttr {
    /// Maximum number of messages
    pub mq_maxmsg: i64,
    /// Maximum message size
    pub mq_msgsize: i64,
    /// Number of messages currently queued
    pub mq_curmsgs: i64,
    /// Flags for the message queue
    pub mq_flags: i32,
}

impl Default for MqAttr {
    fn default() -> Self {
        Self {
            mq_maxmsg: 10,
            mq_msgsize: 8192,
            mq_curmsgs: 0,
            mq_flags: 0,
        }
    }
}

/// Message queue open flags specific to mqueue
pub const MQ_RDONLY: i32 = 0;       // Open for receiving only
pub const MQ_WRONLY: i32 = 1;       // Open for sending only
pub const MQ_RDWR: i32 = 2;         // Open for sending and receiving

/// Message queue notification
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MqNotify {
    /// Notification method
    pub notify_method: i32,
    /// Signal number or pipe fd
    pub notify_sig: i32,
}

/// Notification methods
pub const MQ_SIGNAL: i32 = 1;      // Notify via signal
pub const MQ_PIPE: i32 = 2;        // Notify via pipe

// ============================================================================
// Shared Memory Definitions (sys/mman.h extensions)
// ============================================================================

/// Shared memory open flags
pub const SHM_R: i32 = 0o400;      // Read permission
pub const SHM_W: i32 = 0o200;      // Write permission
pub const SHM_RDONLY: i32 = 0o400000; // Read-only attachment

/// Shared memory create flags
pub const SHM_HUGETLB: i32 = 0o4000; // Use huge pages
pub const SHM_HUGE_SHIFT: i32 = 26;
pub const SHM_HUGE_MASK: i32 = 0x3f << SHM_HUGE_SHIFT;

/// IPC commands
pub const IPC_CREAT: i32 = 0o01000000;  // Create entry if key doesn't exist
pub const IPC_EXCL: i32 = 0o02000000;    // Fail if key exists
pub const IPC_NOWAIT: i32 = 0o04000000;  // Return error on wait

/// Shared memory commands
pub const IPC_RMID: i32 = 0;       // Remove shared memory segment
pub const IPC_SET: i32 = 1;        // Set shared memory attributes
pub const IPC_STAT: i32 = 2;       // Get shared memory attributes

/// Shared memory flags
pub const SHM_RND: i32 = 0o200000;  // Round attach address to SHMLBA boundary
pub const SHMLBA: usize = 4096;     // Segment low boundary address multiple

/// Shared memory information
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ShmidDs {
    /// Operation permissions
    pub shm_perm: IpcPerm,
    /// Segment size
    pub shm_segsz: usize,
    /// Last attach time
    pub shm_atime: Time,
    /// Last detach time
    pub shm_dtime: Time,
    /// Last change time
    pub shm_ctime: Time,
    /// PID of creator
    pub shm_cpid: Pid,
    /// PID of last shmat()
    pub shm_lpid: Pid,
    /// Number of current attaches
    pub shm_nattch: u64,
}

/// IPC permissions structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IpcPerm {
    /// Owner user ID
    pub uid: Uid,
    /// Owner group ID
    pub gid: Gid,
    /// Creator user ID
    pub cuid: Uid,
    /// Creator group ID
    pub cgid: Gid,
    /// Read/write permission bits
    pub mode: Mode,
    /// Sequence number
    pub seq: u16,
    /// Key value
    pub key: i32,
}

// ============================================================================
// Timer Definitions (time.h extensions)
// ============================================================================

/// Timer ID type (opaque pointer)
pub type TimerT = *mut u8;

/// Timer creation flags
pub const TIMER_ABSTIME: i32 = 1;  // Absolute time

/// Timer clock types (already defined above in Clock IDs section)

/// Timer expiration notification
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SigEvent {
    /// Notification method
    pub sigev_notify: i32,
    /// Signal number
    pub sigev_signo: i32,
    /// Signal value
    pub sigev_value: SigVal,
    /// Notification function
    pub sigev_notify_function: usize,
    /// Notification attributes
    pub sigev_notify_attributes: usize,
}

/// Notification methods
pub const SIGEV_NONE: i32 = 0;     // No notification
pub const SIGEV_SIGNAL: i32 = 1;   // Signal notification
pub const SIGEV_THREAD: i32 = 2;   // Thread notification

/// Timer specifications
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Itimerspec {
    /// Timer initial expiration
    pub it_interval: Timespec,
    /// Timer interval
    pub it_value: Timespec,
}

impl Default for Itimerspec {
    fn default() -> Self {
        Self {
            it_interval: Timespec::zero(),
            it_value: Timespec::zero(),
        }
    }
}

// ============================================================================
// Thread support
// ============================================================================

pub mod thread;
pub mod sync;

// ============================================================================
// IPC and Synchronization modules
// ============================================================================

pub mod semaphore;
pub mod mqueue;
pub mod shm;
pub mod timer;
pub mod advanced_signal;
pub mod realtime;
pub mod advanced_thread;
pub mod security;
// pub mod advanced_tests;  // Temporarily disabled
// pub mod integration_tests;  // Temporarily disabled

pub use self::thread::*;
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Linger {
    pub l_onoff: i32,
    pub l_linger: i32,
}
pub use self::semaphore::*;
pub use self::mqueue::*;
pub use self::shm::*;
pub use self::timer::*;
// pub use self::advanced_signal::*;  // Disabled module
// pub use self::realtime::*;  // Disabled module
// pub use self::advanced_thread::*;  // Disabled module
// pub use self::security::*;  // Disabled module

// Import size types from libc
pub use crate::libc::interface::size_t;
pub use crate::libc::ssize_t;

// ============================================================================
// Network interface configuration
// ============================================================================

/// Network interface configuration structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IfConfig {
    pub name: [u8; 32],
    pub is_up: bool,
    pub ipv4_addr: [u8; 4],
    pub ipv4_netmask: [u8; 4],
    pub ipv4_gateway: [u8; 4],
    pub mtu: Option<u32>,
}

/// Network interface information structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IfInfo {
    pub id: u32,
    pub name: [u8; 32],
    pub is_up: bool,
    pub ipv4_addr: [u8; 4],
    pub ipv4_netmask: [u8; 4],
    pub ipv4_gateway: [u8; 4],
    pub mtu: u32,
}

// ============================================================================
// System resource limits and usage
// ============================================================================

/// Resource limit structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct rlimit {
    /// Soft limit
    pub rlim_cur: u64,
    /// Hard limit
    pub rlim_max: u64,
}

/// Resource usage structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct rusage {
    /// User CPU time used
    pub ru_utime: Timeval,
    /// System CPU time used
    pub ru_stime: Timeval,
    /// Maximum resident set size
    pub ru_maxrss: i64,
    /// Integral shared memory size
    pub ru_ixrss: i64,
    /// Integral unshared data size
    pub ru_idrss: i64,
    /// Integral unshared stack size
    pub ru_isrss: i64,
    /// Page reclaims (soft page faults)
    pub ru_minflt: i64,
    /// Page faults (hard page faults)
    pub ru_majflt: i64,
    /// Swaps
    pub ru_nswap: i64,
    /// Block input operations
    pub ru_inblock: i64,
    /// Block output operations
    pub ru_oublock: i64,
    /// IPC messages sent
    pub ru_msgsnd: i64,
    /// IPC messages received
    pub ru_msgrcv: i64,
    /// Signals received
    pub ru_nsignals: i64,
    /// Voluntary context switches
    pub ru_nvcsw: i64,
    /// Involuntary context switches
    pub ru_nivcsw: i64,
}

/// System information structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct sysinfo {
    /// Seconds since boot
    pub uptime: i64,
    /// Load averages (1, 5, 15 minutes)
    pub loads: [u64; 3],
    /// Total RAM
    pub totalram: u64,
    /// Free RAM
    pub freeram: u64,
    /// Shared RAM
    pub sharedram: u64,
    /// Buffer RAM
    pub bufferram: u64,
    /// Total swap
    pub totalswap: u64,
    /// Free swap
    pub freeswap: u64,
    /// Number of processes
    pub procs: u16,
    /// Total high memory
    pub totalhigh: u64,
    /// Free high memory
    pub freehigh: u64,
    /// Memory unit size
    pub mem_unit: u32,
}

/// Process times structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct tms {
    /// User time
    pub tms_utime: clock_t,
    /// System time
    pub tms_stime: clock_t,
    /// User time of children
    pub tms_cutime: clock_t,
    /// System time of children
    pub tms_cstime: clock_t,
}

/// Clock type (clock ticks)
pub type clock_t = i64;

/// User/group ID type
pub type id_t = u32;

/// Shared memory ID structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct shmid_ds {
    /// Owner's user ID
    pub shm_perm: IpcPerm,
    /// Size of segment in bytes
    pub shm_segsz: usize,
    /// Last attach time
    pub shm_atime: i64,
    /// Last detach time
    pub shm_dtime: i64,
    /// Last change time
    pub shm_ctime: i64,
    /// PID of creator
    pub shm_cpid: Pid,
    /// PID of last shmat/shmdt
    pub shm_lpid: Pid,
    /// Number of current attaches
    pub shm_nattch: u64,
}

/// UTS name structure (already defined as Utsname, add alias)
pub type utsname = Utsname;

/// Time specification structure
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct timespec {
    /// Seconds since epoch
    pub tv_sec: i64,
    /// Nanoseconds
    pub tv_nsec: i64,
}

/// Signal set type (C-compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct sigset_t {
    /// Signal set bits (64 signals)
    pub bits: [u64; 1],
}

/// Group ID type (C-compatible alias)
pub type gid_t = Gid;
pub type uid_t = Uid;
pub type socklen_t = u32;

// ============================================================================
// POSIX Thread Types
// ============================================================================

/// POSIX thread ID type
pub type pthread_t = usize;

/// POSIX mutex type
#[repr(C)]
pub struct pthread_mutex_t {
    _data: [u8; 40], // Opaque structure
}

/// POSIX condition variable type
#[repr(C)]
pub struct pthread_cond_t {
    _data: [u8; 48], // Opaque structure
}

/// POSIX read-write lock type
#[repr(C)]
pub struct pthread_rwlock_t {
    _data: [u8; 56], // Opaque structure
}

/// POSIX spinlock type
#[repr(C)]
pub struct pthread_spinlock_t {
    _data: [u8; 4], // Opaque structure
}

/// POSIX mutex attribute type
#[repr(C)]
pub struct pthread_mutexattr_t {
    _data: [u8; 4], // Opaque structure
}

/// POSIX read-write lock attribute type
#[repr(C)]
pub struct pthread_rwlockattr_t {
    _data: [u8; 8], // Opaque structure
}

/// POSIX thread-specific data key type
pub type pthread_key_t = u32;

/// Clock ID type
pub type clockid_t = i32;

/// Directory entry type (C-compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct dirent {
    pub d_ino: u64,
    pub d_off: i64,
    pub d_reclen: u16,
    pub d_type: u8,
    pub d_name: [u8; 256],
}

/// Stat structure (C-compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u64,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atime: i64,
    pub st_atime_nsec: i64,
    pub st_mtime: i64,
    pub st_mtime_nsec: i64,
    pub st_ctime: i64,
    pub st_ctime_nsec: i64,
}

// ============================================================================
// POSIX Semaphore and Message Queue Types
// ============================================================================

/// POSIX semaphore type
pub type sem_t = SemT;

/// POSIX message queue descriptor type
pub type mqd_t = i32;

// ============================================================================
// POSIX Socket Types
// ============================================================================

/// Socket address type (C-compatible alias)
pub type sockaddr = Sockaddr;

// ============================================================================
// POSIX File System Types
// ============================================================================

/// File offset type (C-compatible alias)
pub type off_t = Off;

/// File mode type (C-compatible alias)
pub type mode_t = Mode;

/// Directory type
#[repr(C)]
pub struct DIR {
    _data: [u8; 1], // Opaque structure
}

// ============================================================================
// POSIX Time Types
// ============================================================================

/// Time value structure (C-compatible alias)
pub type timeval = Timeval;

/// Time structure (C-compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct tm {
    pub tm_sec: i32,
    pub tm_min: i32,
    pub tm_hour: i32,
    pub tm_mday: i32,
    pub tm_mon: i32,
    pub tm_year: i32,
    pub tm_wday: i32,
    pub tm_yday: i32,
    pub tm_isdst: i32,
}

// ============================================================================
// POSIX File Descriptor Set Type
// ============================================================================

/// File descriptor set type (C-compatible alias)
pub type fd_set = FdSet;

// ============================================================================
// AIO Types (aio.h)
// ============================================================================

/// AIO operation codes
pub const LIO_READ: i32 = 0;    // Read operation
pub const LIO_WRITE: i32 = 1;   // Write operation
pub const LIO_NOP: i32 = 2;     // No operation
pub const LIO_WAIT: i32 = 1;    // Wait for all operations to complete
pub const LIO_NOWAIT: i32 = 0; // Don't wait for operations to complete

/// AIO return values
pub const AIO_CANCELED: i32 = -1;  // Operation was canceled
pub const AIO_NOTCANCELED: i32 = 0; // Operation could not be canceled
pub const AIO_ALLDONE: i32 = 1;    // All operations already completed

/// AIO control block structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct aiocb {
    /// File descriptor
    pub aio_fildes: i32,
    /// File offset
    pub aio_offset: off_t,
    /// Buffer address
    pub aio_buf: *mut u8,
    /// Number of bytes to transfer
    pub aio_nbytes: size_t,
    /// Return value (filled by kernel)
    pub __return_value: ssize_t,
    /// Error code (filled by kernel)
    pub __error_code: i32,
    /// Request priority
    pub aio_reqprio: aio_reqprio_t,
    /// Signal notification
    pub aio_sigevent: aio_sigevent_t,
    /// List I/O operation codes
    pub aio_lio_opcode: i32,
    /// File synchronization mode
    pub aio_fsync_mode: i32,
    /// List of aiocb pointers for lio_listio
    pub aio_listio: *mut *mut aiocb,
    /// Number of entries in list
    pub aio_nent: i32,
}

impl Default for aiocb {
    fn default() -> Self {
        Self {
            aio_fildes: -1,
            aio_offset: 0,
            aio_buf: core::ptr::null_mut(),
            aio_nbytes: 0,
            __return_value: 0,
            __error_code: 0,
            aio_reqprio: 0,
            aio_sigevent: aio_sigevent_t::default(),
            aio_lio_opcode: LIO_NOP,
            aio_fsync_mode: 0,
            aio_listio: core::ptr::null_mut(),
            aio_nent: 0,
        }
    }
}

/// AIO request priority type
pub type aio_reqprio_t = i32;

/// AIO signal event structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct aio_sigevent_t {
    /// Notification method
    pub sigev_notify: i32,
    /// Signal number
    pub sigev_signo: i32,
    /// Signal value
    pub sigev_value: SigVal,
    /// Notification function
    pub sigev_notify_function: usize,
    /// Notification attributes
    pub sigev_notify_attributes: usize,
}

impl Default for aio_sigevent_t {
    fn default() -> Self {
        Self {
            sigev_notify: 0, // SIGEV_NONE
            sigev_signo: 0,
            sigev_value: SigVal::default(),
            sigev_notify_function: 0,
            sigev_notify_attributes: 0,
        }
    }
}

/// File offset type for AIO
pub type aio_offset_t = off_t;
