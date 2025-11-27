//! POSIX Types and Constants
//! 
//! Standard types and constants for POSIX compliance

// ============================================================================
// Basic Types
// ============================================================================

/// Process ID type
pub type Pid = i32;

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
pub const O_NONBLOCK: i32 = 0o4000;
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
pub const WAIT_ANY: Pid = -1;

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
