//! System call types

use crate::core::types::*;

/// System call number
pub type SyscallNumber = usize;

/// System call arguments
#[derive(Debug, Clone)]
pub struct SyscallArgs {
    /// First argument
    pub arg0: usize,
    /// Second argument
    pub arg1: usize,
    /// Third argument
    pub arg2: usize,
    /// Third argument
    pub arg3: usize,
    /// Fourth argument
    pub arg4: usize,
    /// Fifth argument
    pub arg5: usize,
}

impl SyscallArgs {
    /// Creates new system call arguments
    pub fn new(
        arg0: usize,
        arg1: usize,
        arg2: usize,
        arg3: usize,
        arg4: usize,
        arg5: usize,
    ) -> Self {
        Self {
            arg0,
            arg1,
            arg2,
            arg3,
            arg4,
            arg5,
        }
    }
    
    /// Creates empty system call arguments
    pub fn empty() -> Self {
        Self::new(0, 0, 0, 0, 0, 0)
    }
    
    /// Creates system call arguments with one argument
    pub fn with1(arg0: usize) -> Self {
        Self::new(arg0, 0, 0, 0, 0, 0)
    }
    
    /// Creates system call arguments with two arguments
    pub fn with2(arg0: usize, arg1: usize) -> Self {
        Self::new(arg0, arg1, 0, 0, 0, 0)
    }
    
    /// Creates system call arguments with three arguments
    pub fn with3(arg0: usize, arg1: usize, arg2: usize) -> Self {
        Self::new(arg0, arg1, arg2, 0, 0, 0)
    }
    
    /// Creates system call arguments with four arguments
    pub fn with4(arg0: usize, arg1: usize, arg2: usize, arg3: usize) -> Self {
        Self::new(arg0, arg1, arg2, arg3, 0, 0)
    }
    
    /// Creates system call arguments with five arguments
    pub fn with5(arg0: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize) -> Self {
        Self::new(arg0, arg1, arg2, arg3, arg4, 0)
    }
}

/// System call result
#[derive(Debug, Clone)]
pub enum SyscallResult {
    /// Success with return value
    Success(isize),
    /// Error with error code
    Error(KernelError),
}

impl SyscallResult {
    /// Creates a successful result
    pub fn success(value: isize) -> Self {
        SyscallResult::Success(value)
    }
    
    /// Creates an error result
    pub fn error(error: KernelError) -> Self {
        SyscallResult::Error(error)
    }
    
    /// Returns true if result is success
    pub fn is_success(&self) -> bool {
        matches!(self, SyscallResult::Success(_))
    }
    
    /// Returns true if result is error
    pub fn is_error(&self) -> bool {
        matches!(self, SyscallResult::Error(_))
    }
    
    /// Returns success value if successful
    pub fn success_value(&self) -> Option<isize> {
        match self {
            SyscallResult::Success(value) => Some(*value),
            _ => None,
        }
    }
    
    /// Returns error if failed
    pub fn error_value(&self) -> Option<KernelError> {
        match self {
            SyscallResult::Error(error) => Some(*error),
            _ => None,
        }
    }
}

/// Common system call numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonSyscall {
    /// Read from file descriptor
    Read = 0,
    /// Write to file descriptor
    Write = 1,
    /// Open file
    Open = 2,
    /// Close file descriptor
    Close = 3,
    /// Stat file
    Stat = 4,
    /// Fstat file
    Fstat = 5,
    /// Lstat file
    Lstat = 6,
    /// Poll file descriptors
    Poll = 7,
    /// Lseek file
    Lseek = 8,
    /// Memory map
    Mmap = 9,
    /// Memory protection
    Mprotect = 10,
    /// Memory unmap
    Munmap = 11,
    /// Duplicate file descriptor
    Dup = 12,
    /// Duplicate file descriptor to specific fd
    Dup2 = 13,
    /// Pipe
    Pipe = 14,
    /// Create pipe with flags
    Pipe2 = 15,
    /// Pause process
    Pause = 16,
    /// Create directory
    Mkdir = 17,
    /// Remove directory
    Rmdir = 18,
    /// Create FIFO
    Mknod = 19,
    /// Change file permissions
    Chmod = 20,
    /// Change file owner
    Chown = 21,
    /// Get file system statistics
    Statfs = 22,
    /// Get process ID
    Getpid = 23,
    /// Get parent process ID
    Getppid = 24,
    /// Get process group ID
    Getpgid = 25,
    /// Set process group ID
    Setpgid = 26,
    /// Get user ID
    Getuid = 27,
    /// Set user ID
    Setuid = 28,
    /// Get group ID
    Getgid = 29,
    /// Set group ID
    Setgid = 30,
    /// Get session ID
    Getsid = 31,
    /// Set session ID
    Setsid = 32,
    /// Get process times
    Times = 33,
    /// Set signal handler
    Signal = 34,
    /// Send signal to process
    Kill = 35,
    /// Wait for process to change state
    Wait = 36,
    /// Create new process
    Fork = 37,
    /// Execute program
    Exec = 38,
    /// Exit process
    Exit = 39,
    /// Wait for process to change state (with options)
    Wait4 = 40,
    /// Get process name
    Getpname = 41,
    /// Set process name
    Setpname = 42,
    /// Get current working directory
    Getcwd = 43,
    /// Change current working directory
    Chdir = 44,
    /// Change root directory
    Chroot = 45,
    /// Create hard link
    Link = 46,
    /// Create symbolic link
    Symlink = 47,
    /// Read symbolic link
    Readlink = 48,
    /// Create mount point
    Mount = 49,
    /// Remove mount point
    Umount = 50,
    /// Get system information
    Sysinfo = 51,
    /// Get time of day
    Gettimeofday = 52,
    /// Set time of day
    Settimeofday = 53,
    /// Get resource limits
    Getrlimit = 54,
    /// Set resource limits
    Setrlimit = 55,
    /// Get process resource usage
    Getrusage = 56,
    /// Get system information
    Sysctl = 57,
    /// Create socket
    Socket = 58,
    /// Connect socket
    Connect = 59,
    /// Accept connection
    Accept = 60,
    /// Send data
    Send = 61,
    /// Receive data
    Recv = 62,
    /// Send to address
    Sendto = 63,
    /// Receive from address
    Recvfrom = 64,
    /// Shutdown socket
    Shutdown = 65,
    /// Bind socket
    Bind = 66,
    /// Listen for connections
    Listen = 67,
    /// Get socket name
    Getsockname = 68,
    /// Get peer name
    Getpeername = 69,
    /// Get socket options
    Getsockopt = 70,
    /// Set socket options
    Setsockopt = 71,
    /// Clone process
    Clone = 72,
    /// Create thread
    ThreadCreate = 73,
    /// Exit thread
    ThreadExit = 74,
    /// Join thread
    ThreadJoin = 75,
    /// Get thread ID
    Gettid = 76,
    /// Set thread affinity
    Setaffinity = 77,
    /// Get thread affinity
    Getaffinity = 78,
    /// Set scheduler parameters
    Setscheduler = 79,
    /// Get scheduler parameters
    Getscheduler = 80,
    /// Yield CPU
    Yield = 81,
    /// Sleep for specified time
    Sleep = 82,
    /// Nanosleep
    Nanosleep = 83,
    /// Get real-time clock
    ClockGettime = 84,
    /// Set real-time clock
    ClockSettime = 85,
    /// Get clock resolution
    ClockGetres = 86,
    /// Create timer
    TimerCreate = 87,
    /// Set timer
    TimerSettime = 88,
    /// Get timer
    TimerGettime = 89,
    /// Delete timer
    TimerDelete = 90,
    /// Wait for timer
    TimerGetoverrun = 91,
    /// Create event file descriptor
    Eventfd = 92,
    /// Create signal file descriptor
    Signalfd = 93,
    /// Create timer file descriptor
    Timerfd = 94,
    /// Create epoll instance
    EpollCreate = 95,
    /// Control epoll
    EpollCtl = 96,
    /// Wait for epoll events
    EpollWait = 97,
    /// Create inotify instance
    InotifyInit = 98,
    /// Add inotify watch
    InotifyAddWatch = 99,
    /// Remove inotify watch
    InotifyRmWatch = 100,
    /// Create message queue
    MqOpen = 101,
    /// Close message queue
    MqClose = 102,
    /// Get message queue attributes
    MqGetattr = 103,
    /// Set message queue attributes
    MqSetattr = 104,
    /// Send message to queue
    MqTimedsend = 105,
    /// Receive message from queue
    MqTimedreceive = 106,
    /// Notify message queue
    MqNotify = 107,
    /// Get message queue
    MqGetsetattr = 108,
    /// Create shared memory
    Shmget = 109,
    /// Attach shared memory
    Shmat = 110,
    /// Detach shared memory
    Shmdt = 111,
    /// Control shared memory
    Shmctl = 112,
    /// Get semaphore
    Semget = 113,
    /// Semaphore operation
    Semop = 114,
    /// Control semaphore
    Semctl = 115,
    /// Get semaphore array
    Semtimedop = 116,
    /// Create message queue
    Msgget = 117,
    /// Send message
    Msgsnd = 118,
    /// Receive message
    Msgrcv = 119,
    /// Control message queue
    Msgctl = 120,
    /// Create shared memory object
    ShmOpen = 121,
    /// Unlink shared memory object
    ShmUnlink = 122,
    /// Create semaphore
    SemOpen = 123,
    /// Close semaphore
    SemClose = 124,
    /// Post semaphore
    SemPost = 125,
    /// Wait on semaphore
    SemWait = 126,
    /// Try wait on semaphore
    SemTrywait = 127,
    /// Timed wait on semaphore
    SemTimedwait = 128,
    /// Get semaphore value
    SemGetValue = 129,
    /// Open file with extended attributes
    Openat = 130,
    /// Create directory with extended attributes
    Mkdirat = 131,
    /// Create FIFO with extended attributes
    Mknodat = 132,
    /// Create hard link with extended attributes
    Linkat = 133,
    /// Create symbolic link with extended attributes
    Symlinkat = 134,
    /// Read symbolic link with extended attributes
    Readlinkat = 135,
    /// Remove directory with extended attributes
    Unlinkat = 136,
    /// Rename with extended attributes
    Renameat = 137,
}