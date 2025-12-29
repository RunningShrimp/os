
use crate::prelude::*;

//! Kernel Constants
//!
//! This module provides system-wide constants used throughout the kernel.

#![allow(dead_code)]

// ============================================================================
// Error Number Constants (errno-style)
// ============================================================================

/// No error
pub const EOK: i32 = 0;

/// Operation not permitted
pub const EPERM: i32 = 1;

/// No such file or directory
pub const ENOENT: i32 = 2;

/// No such process
pub const ESRCH: i32 = 3;

/// Interrupted system call
pub const EINTR: i32 = 4;

/// I/O error
pub const EIO: i32 = 5;

/// No such device or address
pub const ENXIO: i32 = 6;

/// Argument list too long
pub const E2BIG: i32 = 7;

/// Exec format error
pub const ENOEXEC: i32 = 8;

/// Bad file number
pub const EBADF: i32 = 9;

/// No child processes
pub const ECHILD: i32 = 10;

/// Try again
pub const EAGAIN: i32 = 11;

/// Out of memory
pub const ENOMEM: i32 = 12;

/// Permission denied
pub const EACCES: i32 = 13;

/// Bad address
pub const EFAULT: i32 = 14;

/// Block device required
pub const ENOTBLK: i32 = 15;

/// Device or resource busy
pub const EBUSY: i32 = 16;

/// File exists
pub const EEXIST: i32 = 17;

/// Cross-device link
pub const EXDEV: i32 = 18;

/// No such device
pub const ENODEV: i32 = 19;

/// Not a directory
pub const ENOTDIR: i32 = 20;

/// Is a directory
pub const EISDIR: i32 = 21;

/// Invalid argument
pub const EINVAL: i32 = 22;

/// File table overflow
pub const ENFILE: i32 = 23;

/// Too many open files
pub const EMFILE: i32 = 24;

/// Not a typewriter
pub const ENOTTY: i32 = 25;

/// Text file busy
pub const ETXTBSY: i32 = 26;

/// File too large
pub const EFBIG: i32 = 27;

/// No space left on device
pub const ENOSPC: i32 = 28;

/// Illegal seek
pub const ESPIPE: i32 = 29;

/// Read-only file system
pub const EROFS: i32 = 30;

/// Too many links
pub const EMLINK: i32 = 31;

/// Broken pipe
pub const EPIPE: i32 = 32;

/// Math argument out of domain of func
pub const EDOM: i32 = 33;

/// Math result not representable
pub const ERANGE: i32 = 34;

/// Resource deadlock would occur
pub const EDEADLK: i32 = 35;

/// File name too long
pub const ENAMETOOLONG: i32 = 36;

/// No record locks available
pub const ENOLCK: i32 = 37;

/// Function not implemented
pub const ENOSYS: i32 = 38;

/// Directory not empty
pub const ENOTEMPTY: i32 = 39;

/// Too many symbolic links encountered
pub const ELOOP: i32 = 40;

/// Operation would block
pub const EWOULDBLOCK: i32 = EAGAIN;

/// No message of desired type
pub const ENOMSG: i32 = 42;

/// Identifier removed
pub const EIDRM: i32 = 43;

/// Channel number out of range
pub const ECHRNG: i32 = 44;

/// Level 2 not synchronized
pub const EL2NSYNC: i32 = 45;

/// Level 3 halted
pub const EL3HLT: i32 = 46;

/// Level 3 reset
pub const EL3RST: i32 = 47;

/// Link number out of range
pub const ELNRNG: i32 = 48;

/// Protocol driver not attached
pub const EUNATCH: i32 = 49;

/// No CSI structure available
pub const ENOCSI: i32 = 50;

/// Level 2 halted
pub const EL2HLT: i32 = 51;

/// Invalid exchange
pub const EBADE: i32 = 52;

/// Invalid request descriptor
pub const EBADR: i32 = 53;

/// Exchange full
pub const EXFULL: i32 = 54;

/// No anode
pub const ENOANO: i32 = 55;

/// Invalid request code
pub const EBADRQC: i32 = 56;

/// Invalid slot
pub const EBADSLT: i32 = 57;

/// Bad font file format
pub const EDEADLOCK: i32 = EDEADLK;

/// Device not a stream
pub const ENOSTR: i32 = 60;

/// No data available
pub const ENODATA: i32 = 61;

/// Timer expired
pub const ETIME: i32 = 62;

/// Out of streams resources
pub const ENOSR: i32 = 63;

/// Machine is not on the network
pub const ENONET: i32 = 64;

/// Package not installed
pub const ENOPKG: i32 = 65;

/// Object is remote
pub const EREMOTE: i32 = 66;

/// Link has been severed
pub const ENOLINK: i32 = 67;

/// Advertise error
pub const EADV: i32 = 68;

/// Srmount error
pub const ESRMNT: i32 = 69;

/// Communication error on send
pub const ECOMM: i32 = 70;

/// Protocol error
pub const EPROTO: i32 = 71;

/// Multihop attempted
pub const EMULTIHOP: i32 = 72;

/// RFS specific error
pub const EDOTDOT: i32 = 73;

/// Not a data message
pub const EBADMSG: i32 = 74;

/// Value too large for defined data type
pub const EOVERFLOW: i32 = 75;

/// Name not unique on network
pub const ENOTUNIQ: i32 = 76;

/// File descriptor in bad state
pub const EBADFD: i32 = 77;

/// Remote address changed
pub const EREMCHG: i32 = 78;

/// Can not access a needed shared library
pub const ELIBACC: i32 = 79;

/// Accessing a corrupted shared library
pub const ELIBBAD: i32 = 80;

/// .lib section in a.out corrupted
pub const ELIBSCN: i32 = 81;

/// Attempting to link in too many shared libraries
pub const ELIBMAX: i32 = 82;

/// Cannot exec a shared library directly
pub const ELIBEXEC: i32 = 83;

/// Illegal byte sequence
pub const EILSEQ: i32 = 84;

/// Interrupted system call should be restarted
pub const ERESTART: i32 = 85;

/// Streams pipe error
pub const ESTRPIPE: i32 = 86;

/// Too many users
pub const EUSERS: i32 = 87;

/// Socket operation on non-socket
pub const ENOTSOCK: i32 = 88;

/// Destination address required
pub const EDESTADDRREQ: i32 = 89;

/// Message too long
pub const EMSGSIZE: i32 = 90;

/// Protocol wrong type for socket
pub const EPROTOTYPE: i32 = 91;

/// Protocol not available
pub const ENOPROTOOPT: i32 = 92;

/// Protocol not supported
pub const EPROTONOSUPPORT: i32 = 93;

/// Socket type not supported
pub const ESOCKTNOSUPPORT: i32 = 94;

/// Operation not supported on transport endpoint
pub const EOPNOTSUPP: i32 = 95;

/// Protocol family not supported
pub const EPFNOSUPPORT: i32 = 96;

/// Address family not supported by protocol
pub const EAFNOSUPPORT: i32 = 97;

/// Address already in use
pub const EADDRINUSE: i32 = 98;

/// Cannot assign requested address
pub const EADDRNOTAVAIL: i32 = 99;

/// Network is down
pub const ENETDOWN: i32 = 100;

/// Network is unreachable
pub const ENETUNREACH: i32 = 101;

/// Network dropped connection because of reset
pub const ENETRESET: i32 = 102;

/// Software caused connection abort
pub const ECONNABORTED: i32 = 103;

/// Connection reset by peer
pub const ECONNRESET: i32 = 104;

/// No buffer space available
pub const ENOBUFS: i32 = 105;

/// Transport endpoint is already connected
pub const EISCONN: i32 = 106;

/// Transport endpoint is not connected
pub const ENOTCONN: i32 = 107;

/// Cannot send after transport endpoint shutdown
pub ESHUTDOWN: i32 = 108;

/// Too many references: cannot splice
pub const ETOOMANYREFS: i32 = 109;

/// Connection timed out
pub const ETIMEDOUT: i32 = 110;

/// Connection refused
pub const ECONNREFUSED: i32 = 111;

/// Host is down
pub const EHOSTDOWN: i32 = 112;

/// No route to host
pub const EHOSTUNREACH: i32 = 113;

/// Operation already in progress
pub const EALREADY: i32 = 114;

/// Operation now in progress
pub const EINPROGRESS: i32 = 115;

/// Stale NFS file handle
pub const ESTALE: i32 = 116;

/// Structure needs cleaning
pub const EUCLEAN: i32 = 117;

/// Not a XENIX named type file
pub const ENOTNAM: i32 = 118;

/// No XENIX semaphores available
pub const ENAVAIL: i32 = 119;

/// Is a named type file
pub const EISNAM: i32 = 120;

/// Remote I/O error
pub const EREMOTEIO: i32 = 121;

/// Quota exceeded
pub const EDQUOT: i32 = 122;

/// No medium found
pub const ENOMEDIUM: i32 = 123;

/// Wrong medium type
pub const EMEDIUMTYPE: i32 = 124;

/// Operation canceled
pub const ECANCELED: i32 = 125;

/// Required key not available
pub const ENOKEY: i32 = 126;

/// Key has expired
pub const EKEYEXPIRED: i32 = 127;

/// Key has been revoked
pub const EKEYREVOKED: i32 = 128;

/// Key was rejected by service
pub const EKEYREJECTED: i32 = 129;

/// Owner died
pub const EOWNERDEAD: i32 = 130;

/// State not recoverable
pub const ENOTRECOVERABLE: i32 = 131;

// ============================================================================
// Memory Constants
// ============================================================================

/// Standard page size (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Page shift (log2 of PAGE_SIZE)
pub const PAGE_SHIFT: usize = 12;

/// Page mask
pub const PAGE_MASK: usize = PAGE_SIZE - 1;

/// Huge page size (2MB)
pub const HUGE_PAGE_SIZE: usize = 2 * 1024 * 1024;

/// Kernel virtual address space start (typical for x86_64)
pub const KERNEL_VMA_BASE: usize = 0xFFFF_8000_0000_0000;

/// User address space limit
pub const USER_LIMIT: usize = 0x0000_7FFF_FFFF_FFFF;

// ============================================================================
// Process Constants
// ============================================================================

/// Maximum number of processes
pub const MAX_PROCESSES: usize = 4096;

/// Maximum number of threads per process
pub const MAX_THREADS_PER_PROCESS: usize = 1024;

/// Maximum process name length
pub const MAX_PROCESS_NAME_LEN: usize = 16;

/// Root process ID
pub const ROOT_PID: u64 = 1;

/// Maximum number of open files per process
pub const MAX_OPEN_FILES: usize = 1024;

// ============================================================================
// Signal Constants
// ============================================================================

/// Signal set size (in words)
pub const SIG_SET_SIZE: usize = 8;

/// Number of signals
pub const NSIG: usize = 64;

/// Invalid signal number
pub const SIG_ERR: i32 = -1;

/// Default signal handling
pub const SIG_DFL: usize = 0;

/// Ignore signal
pub const SIG_IGN: usize = 1;

/// Hold signal
pub const SIG_HOLD: usize = 2;

// ============================================================================
// File System Constants
// ============================================================================

/// Maximum path length
pub const PATH_MAX: usize = 4096;

/// Maximum file name length
pub const NAME_MAX: usize = 255;

/// Maximum number of symbolic links
pub const MAXSYMLINKS: usize = 40;

/// Default file mode (rwxr-xr-x)
pub const DEFAULT_FILE_MODE: u32 = 0o755;

// ============================================================================
// Time Constants
// ============================================================================

/// Number of nanoseconds per second
pub const NSEC_PER_SEC: u64 = 1_000_000_000;

/// Number of microseconds per second
pub const USEC_PER_SEC: u64 = 1_000_000;

/// Number of milliseconds per second
pub const MSEC_PER_SEC: u64 = 1_000;

/// Number of nanoseconds per microsecond
pub const NSEC_PER_USEC: u64 = 1_000;

/// Number of nanoseconds per millisecond
pub const NSEC_PER_MSEC: u64 = 1_000_000;

// ============================================================================
// Network Constants
// ============================================================================

/// Maximum socket queue length
pub const SOMAXCONN: usize = 128;

/// Maximum segment size for TCP
pub const TCP_MSS_DEFAULT: u32 = 1460;

/// Default TCP window size
pub const TCP_WINDOW_DEFAULT: u32 = 65535;

// ============================================================================
// Architecture Constants
// ============================================================================

/// Cache line size (typical)
pub const CACHE_LINE_SIZE: usize = 64;

/// Cache line alignment mask
pub const CACHE_LINE_MASK: usize = CACHE_LINE_SIZE - 1;

/// Stack size (default)
pub const DEFAULT_STACK_SIZE: usize = 8 * 1024 * 1024; // 8MB

/// Guard page size
pub const GUARD_PAGE_SIZE: usize = PAGE_SIZE;

// ============================================================================
// Scheduler Constants
// ============================================================================

/// Maximum scheduler priority
pub const MAX_PRIO: usize = 140;

/// Minimum scheduler priority
pub const MIN_PRIO: usize = 0;

/// Default scheduler priority
pub const DEFAULT_PRIO: usize = 120;

/// Real-time priority start
pub const RT_PRIO_START: usize = 0;

/// Real-time priority end
pub const RT_PRIO_END: usize = 99;

/// Nice value start
pub const NICE_PRIO_START: usize = 100;

/// Nice value end
pub const NICE_PRIO_END: usize = 139;

// ============================================================================
// Buffer Constants
// ============================================================================

/// Default buffer size for I/O operations
pub const DEFAULT_BUFFER_SIZE: usize = 8 * 1024;

/// Minimum buffer size
pub const MIN_BUFFER_SIZE: usize = 256;

/// Maximum buffer size
pub const MAX_BUFFER_SIZE: usize = 1024 * 1024;
