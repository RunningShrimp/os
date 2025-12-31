//! File system error types
//!
//! Comprehensive error handling for all file system operations.

#![allow(dead_code)]

use core::fmt;

/// Result type alias for file system operations
pub type FsResult<T> = Result<T, FsError>;

/// File system errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    /// No error
    Ok,
    /// Operation not permitted
    PermissionDenied,
    /// File does not exist
    NotFound,
    /// I/O error
    IoError,
    /// No space left on device
    NoSpace,
    /// File exists
    Exists,
    /// Not a directory
    NotADirectory,
    /// Is a directory
    IsADirectory,
    /// Invalid argument
    InvalidInput,
    /// File too large
    FileTooLarge,
    /// Too many open files
    TooManyOpenFiles,
    /// File or directory busy
    Busy,
    /// Interrupted system call
    Interrupted,
    /// Invalid file system operation
    InvalidOperation,
    /// Not supported
    NotSupported,
    /// Directory not empty
    NotEmpty,
    /// Read-only file system
    ReadOnly,
    /// File system not mounted
    NotMounted,
    /// File system corrupted
    Corrupted,
    /// File system full
    FileSystemFull,
    /// No such device
    NoDevice,
    /// Block device required
    BlockDeviceRequired,
    /// Text file busy
    TextFileBusy,
    /// Operation would block
    WouldBlock,
    /// Operation already in progress
    AlreadyInProgress,
    /// Connection reset
    ConnectionReset,
    /// Connection refused
    ConnectionRefused,
    /// Host unreachable
    HostUnreachable,
    /// Network unreachable
    NetworkUnreachable,
    /// Connection timed out
    TimedOut,
    /// Quota exceeded
    QuotaExceeded,
    /// Stale file handle
    StaleFileHandle,
    /// Bad file number
    BadFileNumber,
    /// No child process
    NoChildProcess,
    /// No data available
    NoData,
    /// No locks available
    NoLocksAvailable,
    /// Filename too long
    FilenameTooLong,
    /// Too many levels of symbolic links
    TooManyLinks,
    /// Invalid exchange
    InvalidExchange,
    /// Invalid seek
    InvalidSeek,
    /// No message of desired type
    NoMessage,
    /// Not a stream
    NotAStream,
    /// Out of memory
    OutOfMemory,
    /// Value too large
    ValueTooLarge,
    /// Wrong medium type
    WrongMediumType,
    /// Operation canceled
    OperationCanceled,
    /// Resource deadlock would occur
    Deadlock,
    /// File name too long
    FileNameTooLong,
    /// No such file or directory
    NoSuchFileOrDirectory,
    /// Not initialized
    NotInitialized,
    /// Already initialized
    AlreadyInitialized,
    /// Invalid state
    InvalidState,
    /// Invalid checksum
    InvalidChecksum,
    /// Checksum mismatch
    ChecksumMismatch,
    /// Invalid block
    InvalidBlock,
    /// Invalid inode
    InvalidInode,
    /// Invalid superblock
    InvalidSuperblock,
    /// Invalid segment
    InvalidSegment,
    /// Segment full
    SegmentFull,
    /// Journal full
    JournalFull,
    /// Journal corrupted
    JournalCorrupted,
    /// Snapshot not found
    SnapshotNotFound,
    /// Snapshot exists
    SnapshotExists,
    /// Clone failed
    CloneFailed,
    /// Replication failed
    ReplicationFailed,
    /// Placement failed
    PlacementFailed,
    /// Cache miss
    CacheMiss,
    /// Cache error
    CacheError,
    /// Metadata corrupted
    MetadataCorrupted,
    /// Data corrupted
    DataCorrupted,
    /// Unknown error
    Unknown(i32),
}

impl FsError {
    /// Get error name
    pub fn name(&self) -> &str {
        match self {
            FsError::Ok => "Ok",
            FsError::PermissionDenied => "PermissionDenied",
            FsError::NotFound => "NotFound",
            FsError::IoError => "IoError",
            FsError::NoSpace => "NoSpace",
            FsError::Exists => "Exists",
            FsError::NotADirectory => "NotADirectory",
            FsError::IsADirectory => "IsADirectory",
            FsError::InvalidInput => "InvalidInput",
            FsError::FileTooLarge => "FileTooLarge",
            FsError::TooManyOpenFiles => "TooManyOpenFiles",
            FsError::Busy => "Busy",
            FsError::Interrupted => "Interrupted",
            FsError::InvalidOperation => "InvalidOperation",
            FsError::NotSupported => "NotSupported",
            FsError::NotEmpty => "NotEmpty",
            FsError::ReadOnly => "ReadOnly",
            FsError::NotMounted => "NotMounted",
            FsError::Corrupted => "Corrupted",
            FsError::FileSystemFull => "FileSystemFull",
            FsError::NoDevice => "NoDevice",
            FsError::BlockDeviceRequired => "BlockDeviceRequired",
            FsError::TextFileBusy => "TextFileBusy",
            FsError::WouldBlock => "WouldBlock",
            FsError::AlreadyInProgress => "AlreadyInProgress",
            FsError::ConnectionReset => "ConnectionReset",
            FsError::ConnectionRefused => "ConnectionRefused",
            FsError::HostUnreachable => "HostUnreachable",
            FsError::NetworkUnreachable => "NetworkUnreachable",
            FsError::TimedOut => "TimedOut",
            FsError::QuotaExceeded => "QuotaExceeded",
            FsError::StaleFileHandle => "StaleFileHandle",
            FsError::BadFileNumber => "BadFileNumber",
            FsError::NoChildProcess => "NoChildProcess",
            FsError::NoData => "NoData",
            FsError::NoLocksAvailable => "NoLocksAvailable",
            FsError::FilenameTooLong => "FilenameTooLong",
            FsError::TooManyLinks => "TooManyLinks",
            FsError::InvalidExchange => "InvalidExchange",
            FsError::InvalidSeek => "InvalidSeek",
            FsError::NoMessage => "NoMessage",
            FsError::NotAStream => "NotAStream",
            FsError::OutOfMemory => "OutOfMemory",
            FsError::ValueTooLarge => "ValueTooLarge",
            FsError::WrongMediumType => "WrongMediumType",
            FsError::OperationCanceled => "OperationCanceled",
            FsError::Deadlock => "Deadlock",
            FsError::FileNameTooLong => "FileNameTooLong",
            FsError::NoSuchFileOrDirectory => "NoSuchFileOrDirectory",
            FsError::NotInitialized => "NotInitialized",
            FsError::AlreadyInitialized => "AlreadyInitialized",
            FsError::InvalidState => "InvalidState",
            FsError::InvalidChecksum => "InvalidChecksum",
            FsError::ChecksumMismatch => "ChecksumMismatch",
            FsError::InvalidBlock => "InvalidBlock",
            FsError::InvalidInode => "InvalidInode",
            FsError::InvalidSuperblock => "InvalidSuperblock",
            FsError::InvalidSegment => "InvalidSegment",
            FsError::SegmentFull => "SegmentFull",
            FsError::JournalFull => "JournalFull",
            FsError::JournalCorrupted => "JournalCorrupted",
            FsError::SnapshotNotFound => "SnapshotNotFound",
            FsError::SnapshotExists => "SnapshotExists",
            FsError::CloneFailed => "CloneFailed",
            FsError::ReplicationFailed => "ReplicationFailed",
            FsError::PlacementFailed => "PlacementFailed",
            FsError::CacheMiss => "CacheMiss",
            FsError::CacheError => "CacheError",
            FsError::MetadataCorrupted => "MetadataCorrupted",
            FsError::DataCorrupted => "DataCorrupted",
            FsError::Unknown(_) => "Unknown",
        }
    }

    /// Get error description
    pub fn description(&self) -> &str {
        match self {
            FsError::Ok => "No error",
            FsError::PermissionDenied => "Operation not permitted",
            FsError::NotFound => "No such file or directory",
            FsError::IoError => "I/O error",
            FsError::NoSpace => "No space left on device",
            FsError::Exists => "File exists",
            FsError::NotADirectory => "Not a directory",
            FsError::IsADirectory => "Is a directory",
            FsError::InvalidInput => "Invalid argument",
            FsError::FileTooLarge => "File too large",
            FsError::TooManyOpenFiles => "Too many open files",
            FsError::Busy => "Device or resource busy",
            FsError::Interrupted => "Interrupted system call",
            FsError::InvalidOperation => "Invalid file system operation",
            FsError::NotSupported => "Operation not supported",
            FsError::NotEmpty => "Directory not empty",
            FsError::ReadOnly => "Read-only file system",
            FsError::NotMounted => "File system not mounted",
            FsError::Corrupted => "File system corrupted",
            FsError::FileSystemFull => "File system full",
            FsError::NoDevice => "No such device",
            FsError::BlockDeviceRequired => "Block device required",
            FsError::TextFileBusy => "Text file busy",
            FsError::WouldBlock => "Operation would block",
            FsError::AlreadyInProgress => "Operation already in progress",
            FsError::ConnectionReset => "Connection reset",
            FsError::ConnectionRefused => "Connection refused",
            FsError::HostUnreachable => "Host unreachable",
            FsError::NetworkUnreachable => "Network unreachable",
            FsError::TimedOut => "Connection timed out",
            FsError::QuotaExceeded => "Disk quota exceeded",
            FsError::StaleFileHandle => "Stale file handle",
            FsError::BadFileNumber => "Bad file number",
            FsError::NoChildProcess => "No child process",
            FsError::NoData => "No data available",
            FsError::NoLocksAvailable => "No locks available",
            FsError::FilenameTooLong => "File name too long",
            FsError::TooManyLinks => "Too many levels of symbolic links",
            FsError::InvalidExchange => "Invalid exchange",
            FsError::InvalidSeek => "Invalid seek",
            FsError::NoMessage => "No message of desired type",
            FsError::NotAStream => "Not a stream",
            FsError::OutOfMemory => "Out of memory",
            FsError::ValueTooLarge => "Value too large",
            FsError::WrongMediumType => "Wrong medium type",
            FsError::OperationCanceled => "Operation canceled",
            FsError::Deadlock => "Resource deadlock would occur",
            FsError::FileNameTooLong => "File name too long",
            FsError::NoSuchFileOrDirectory => "No such file or directory",
            FsError::NotInitialized => "Component not initialized",
            FsError::AlreadyInitialized => "Component already initialized",
            FsError::InvalidState => "Invalid state",
            FsError::InvalidChecksum => "Invalid checksum",
            FsError::ChecksumMismatch => "Checksum mismatch",
            FsError::InvalidBlock => "Invalid block",
            FsError::InvalidInode => "Invalid inode",
            FsError::InvalidSuperblock => "Invalid superblock",
            FsError::InvalidSegment => "Invalid segment",
            FsError::SegmentFull => "Segment full",
            FsError::JournalFull => "Journal full",
            FsError::JournalCorrupted => "Journal corrupted",
            FsError::SnapshotNotFound => "Snapshot not found",
            FsError::SnapshotExists => "Snapshot already exists",
            FsError::CloneFailed => "Clone operation failed",
            FsError::ReplicationFailed => "Replication failed",
            FsError::PlacementFailed => "Placement failed",
            FsError::CacheMiss => "Cache miss",
            FsError::CacheError => "Cache error",
            FsError::MetadataCorrupted => "Metadata corrupted",
            FsError::DataCorrupted => "Data corrupted",
            FsError::Unknown(_) => "Unknown error",
        }
    }

    /// Convert to errno-style code
    pub fn as_errno(&self) -> i32 {
        match self {
            FsError::Ok => 0,
            FsError::PermissionDenied => 1,
            FsError::NotFound => 2,
            FsError::IoError => 5,
            FsError::NoSpace => 28,
            FsError::Exists => 17,
            FsError::NotADirectory => 20,
            FsError::IsADirectory => 21,
            FsError::InvalidInput => 22,
            FsError::FileTooLarge => 27,
            FsError::TooManyOpenFiles => 24,
            FsError::Busy => 16,
            FsError::Interrupted => 4,
            FsError::InvalidOperation => 38,
            FsError::NotSupported => 95,
            FsError::NotEmpty => 39,
            FsError::ReadOnly => 30,
            FsError::NotMounted => 21,
            FsError::Corrupted => 5,
            FsError::FileSystemFull => 28,
            FsError::NoDevice => 19,
            FsError::BlockDeviceRequired => 15,
            FsError::TextFileBusy => 26,
            FsError::WouldBlock => 11,
            FsError::AlreadyInProgress => 114,
            FsError::ConnectionReset => 104,
            FsError::ConnectionRefused => 111,
            FsError::HostUnreachable => 113,
            FsError::NetworkUnreachable => 101,
            FsError::TimedOut => 110,
            FsError::QuotaExceeded => 122,
            FsError::StaleFileHandle => 116,
            FsError::BadFileNumber => 9,
            FsError::NoChildProcess => 10,
            FsError::NoData => 61,
            FsError::NoLocksAvailable => 46,
            FsError::FilenameTooLong => 36,
            FsError::TooManyLinks => 40,
            FsError::InvalidExchange => 90,
            FsError::InvalidSeek => 29,
            FsError::NoMessage => 42,
            FsError::NotAStream => 88,
            FsError::OutOfMemory => 12,
            FsError::ValueTooLarge => 75,
            FsError::WrongMediumType => 124,
            FsError::OperationCanceled => 125,
            FsError::Deadlock => 35,
            FsError::FileNameTooLong => 36,
            FsError::NoSuchFileOrDirectory => 2,
            FsError::NotInitialized => 89,
            FsError::AlreadyInitialized => 114,
            FsError::InvalidState => 25,
            FsError::InvalidChecksum => 74,
            FsError::ChecksumMismatch => 74,
            FsError::InvalidBlock => 22,
            FsError::InvalidInode => 22,
            FsError::InvalidSuperblock => 22,
            FsError::InvalidSegment => 22,
            FsError::SegmentFull => 28,
            FsError::JournalFull => 28,
            FsError::JournalCorrupted => 5,
            FsError::SnapshotNotFound => 2,
            FsError::SnapshotExists => 17,
            FsError::CloneFailed => 5,
            FsError::ReplicationFailed => 5,
            FsError::PlacementFailed => 5,
            FsError::CacheMiss => 19,
            FsError::CacheError => 5,
            FsError::MetadataCorrupted => 5,
            FsError::DataCorrupted => 5,
            FsError::Unknown(code) => *code,
        }
    }
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name(), self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_names() {
        assert_eq!(FsError::NotFound.name(), "NotFound");
        assert_eq!(FsError::PermissionDenied.name(), "PermissionDenied");
    }

    #[test]
    fn test_error_descriptions() {
        assert!(!FsError::NotFound.description().is_empty());
        assert!(!FsError::IoError.description().is_empty());
    }

    #[test]
    fn test_errno_codes() {
        assert_eq!(FsError::Ok.as_errno(), 0);
        assert_eq!(FsError::NotFound.as_errno(), 2);
        assert_eq!(FsError::PermissionDenied.as_errno(), 1);
    }

    #[test]
    fn test_display() {
        let err = FsError::NotFound;
        let s = format!("{}", err);
        assert!(s.contains("NotFound"));
    }
}
