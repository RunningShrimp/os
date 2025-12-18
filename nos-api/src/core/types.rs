//! Core types used throughout NOS operating system

use core::fmt;

/// Process identifier type
pub type Pid = u32;

/// User identifier type
pub type Uid = u32;

/// Group identifier type
pub type Gid = u32;

/// File descriptor type
pub type Fd = i32;

/// Physical address type
pub type PhysAddr = usize;

/// Virtual address type
pub type VirtAddr = usize;

/// Page number type
pub type PageNum = usize;

/// Size type
pub type Size = usize;

/// Count type
pub type Count = usize;

/// Result type for operations that can fail
pub type Result<T> = core::result::Result<T, crate::error::Error>;

/// Time in nanoseconds
pub type Nanoseconds = u64;

/// Time in microseconds
pub type Microseconds = u64;

/// Time in milliseconds
pub type Milliseconds = u64;

/// Time in seconds
pub type Seconds = u64;

/// Represents a kernel error code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelError {
    /// Operation not permitted
    PermissionDenied,
    /// No such file or directory
    NotFound,
    /// Input/output error
    IoError,
    /// No such device or address
    NoDevice,
    /// Invalid argument
    InvalidArgument,
    /// Not enough memory
    OutOfMemory,
    /// Resource busy
    Busy,
    /// Operation would block
    WouldBlock,
    /// Operation already in progress
    AlreadyInProgress,
    /// Connection reset
    ConnectionReset,
    /// Connection aborted
    ConnectionAborted,
    /// No such process
    NoProcess,
    /// Interrupted system call
    Interrupted,
    /// Invalid file descriptor
    BadFileDescriptor,
    /// Operation not supported
    NotSupported,
    /// Operation timed out
    TimedOut,
    /// Out of memory
    OutOfSpace,
    /// Quota exceeded
    QuotaExceeded,
    /// Unknown error
    Unknown(i32),
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::PermissionDenied => write!(f, "Operation not permitted"),
            KernelError::NotFound => write!(f, "No such file or directory"),
            KernelError::IoError => write!(f, "Input/output error"),
            KernelError::NoDevice => write!(f, "No such device or address"),
            KernelError::InvalidArgument => write!(f, "Invalid argument"),
            KernelError::OutOfMemory => write!(f, "Not enough memory"),
            KernelError::Busy => write!(f, "Resource busy"),
            KernelError::WouldBlock => write!(f, "Operation would block"),
            KernelError::AlreadyInProgress => write!(f, "Operation already in progress"),
            KernelError::ConnectionReset => write!(f, "Connection reset"),
            KernelError::ConnectionAborted => write!(f, "Connection aborted"),
            KernelError::NoProcess => write!(f, "No such process"),
            KernelError::Interrupted => write!(f, "Interrupted system call"),
            KernelError::BadFileDescriptor => write!(f, "Invalid file descriptor"),
            KernelError::NotSupported => write!(f, "Operation not supported"),
            KernelError::TimedOut => write!(f, "Operation timed out"),
            KernelError::OutOfSpace => write!(f, "Out of space"),
            KernelError::QuotaExceeded => write!(f, "Quota exceeded"),
            KernelError::Unknown(code) => write!(f, "Unknown error: {}", code),
        }
    }
}

/// Represents a kernel capability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// No capabilities
    None,
    /// Basic capabilities
    Basic,
    /// Extended capabilities
    Extended,
    /// Full capabilities
    Full,
}

/// Represents a memory protection flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryProtection {
    /// No access
    None = 0x0,
    /// Read access
    Read = 0x1,
    /// Write access
    /// Write access implies read access
    Write = 0x3,
    /// Execute access
    Execute = 0x4,
    /// Read and execute access
    ReadExecute = 0x5,
    /// Read, write, and execute access
    ReadWriteExecute = 0x7,
}

/// Represents a memory mapping type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryMappingType {
    /// Private mapping
    Private,
    /// Shared mapping
    Shared,
    /// Copy-on-write mapping
    CopyOnWrite,
}

/// Represents a file access mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
    /// Read only
    ReadOnly = 0o0,
    /// Write only
    WriteOnly = 0o2,
    /// Read and write
    ReadWrite = 0o6,
}

/// Represents a file type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Regular file
    Regular,
    /// Directory
    Directory,
    /// Character device
    CharacterDevice,
    /// Block device
    BlockDevice,
    /// Symbolic link
    SymbolicLink,
    /// Named pipe
    NamedPipe,
    /// Socket
    Socket,
}

/// Represents a process state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// Process is running
    Running,
    /// Process is ready to run
    Ready,
    /// Process is blocked
    Blocked,
    /// Process has terminated
    Terminated,
    /// Process is zombie
    Zombie,
}

/// Represents a thread state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    /// Thread is running
    Running,
    /// Thread is ready to run
    Ready,
    /// Thread is blocked
    Blocked,
    /// Thread has terminated
    Terminated,
}

/// Represents a scheduling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    /// Normal scheduling
    Normal,
    /// Real-time scheduling
    RealTime,
    /// Batch scheduling
    Batch,
    /// Idle scheduling
    Idle,
}

/// Represents a network address family
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressFamily {
    /// IPv4
    IPv4,
    /// IPv6
    IPv6,
    /// Unix domain
    Unix,
    /// Packet socket
    Packet,
}

/// Represents a socket type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    /// Stream socket
    Stream,
    /// Datagram socket
    Datagram,
    /// Raw socket
    Raw,
    /// Sequential packet socket
    SeqPacket,
}

/// Represents a network protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// TCP
    Tcp,
    /// UDP
    Udp,
    /// ICMP
    Icmp,
    /// Raw IP
    RawIp,
}

/// Represents a signal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// Hangup
    Hup = 1,
    /// Interrupt
    Int = 2,
    /// Quit
    Quit = 3,
    /// Illegal instruction
    Ill = 4,
    /// Trace trap
    Trap = 5,
    /// Abort
    Abort = 6,
    /// Bus error
    Bus = 7,
    /// Floating point exception
    Fpe = 8,
    /// Kill
    Kill = 9,
    /// User-defined signal 1
    User1 = 10,
    /// Segmentation violation
    Segv = 11,
    /// User-defined signal 2
    User2 = 12,
    /// Broken pipe
    Pipe = 13,
    /// Alarm clock
    Alrm = 14,
    /// Software termination signal
    Term = 15,
    /// Child status has changed
    Chld = 17,
    /// Continue
    Cont = 18,
    /// Stop, unblockable
    Stop = 19,
    /// Keyboard stop
    Tstp = 20,
    /// Background read from tty
    Ttin = 21,
    /// Background write to tty
    Ttou = 22,
    /// Urgent condition on socket
    Urg = 23,
    /// CPU time limit exceeded
    Xcpu = 24,
    /// File size limit exceeded
    Xfsz = 25,
    /// Virtual alarm clock
    Vtalrm = 26,
    /// Profiling alarm clock
    Prof = 27,
    /// Window size change
    Winch = 28,
    /// I/O now possible
    Io = 29,
    /// Power failure restart
    Pwr = 30,
    /// Bad system call
    Sys = 31,
}