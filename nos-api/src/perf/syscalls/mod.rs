//! Minimal syscalls stubs for nos-perf

pub mod common {
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum SyscallError {
        InvalidSyscall,
        InvalidArgument,
        NotFound,
        IoError,
        OutOfMemory,
        TimedOut,
        ConnectionRefused,
        NotSupported,
        NoBufferSpace,
        BadFileDescriptor,
    }
}

pub const SYS_GETPID: u32 = 39;
