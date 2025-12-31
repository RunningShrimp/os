//! Database Error Types
//!
//! Comprehensive error handling for the database system.

use crate::error::KernelError;

/// Database-specific error type
#[derive(Debug, Clone, PartialEq)]
pub enum DbError {
    /// I/O error occurred
    Io(String),

    /// Corrupted data detected
    Corrupted(String),

    /// Item not found
    NotFound(String),

    /// Item already exists
    AlreadyExists(String),

    /// Invalid argument provided
    InvalidArgument(String),

    /// Database is in inconsistent state
    Inconsistent(String),

    /// Transaction error
    Transaction(String),

    /// Lock acquisition failed
    Lock(String),

    /// Query execution error
    Query(String),

    /// Parse error
    Parse(String),

    /// Syntax error
    Syntax(String),

    /// Type mismatch
    TypeMismatch(String),

    /// Constraint violation
    ConstraintViolation(String),

    /// Out of memory
    OutOfMemory,

    /// Disk full
    DiskFull,

    /// WAL error
    Wal(String),

    /// Checksum mismatch
    ChecksumMismatch,

    /// Version mismatch
    VersionMismatch,

    /// Timeout
    Timeout,

    /// Internal error
    Internal(String),
}

impl DbError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self, DbError::Lock(_) | DbError::Timeout)
    }

    /// Check if error is permanent (non-retryable)
    pub fn is_permanent(&self) -> bool {
        matches!(
            self,
            DbError::InvalidArgument(_)
                | DbError::TypeMismatch(_)
                | DbError::ConstraintViolation(_)
                | DbError::NotFound(_)
        )
    }
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::Io(msg) => write!(f, "I/O error: {}", msg),
            DbError::Corrupted(msg) => write!(f, "Corrupted data: {}", msg),
            DbError::NotFound(msg) => write!(f, "Not found: {}", msg),
            DbError::AlreadyExists(msg) => write!(f, "Already exists: {}", msg),
            DbError::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            DbError::Inconsistent(msg) => write!(f, "Inconsistent state: {}", msg),
            DbError::Transaction(msg) => write!(f, "Transaction error: {}", msg),
            DbError::Lock(msg) => write!(f, "Lock error: {}", msg),
            DbError::Query(msg) => write!(f, "Query error: {}", msg),
            DbError::Parse(msg) => write!(f, "Parse error: {}", msg),
            DbError::Syntax(msg) => write!(f, "Syntax error: {}", msg),
            DbError::TypeMismatch(msg) => write!(f, "Type mismatch: {}", msg),
            DbError::ConstraintViolation(msg) => write!(f, "Constraint violation: {}", msg),
            DbError::OutOfMemory => write!(f, "Out of memory"),
            DbError::DiskFull => write!(f, "Disk full"),
            DbError::Wal(msg) => write!(f, "WAL error: {}", msg),
            DbError::ChecksumMismatch => write!(f, "Checksum mismatch"),
            DbError::VersionMismatch => write!(f, "Version mismatch"),
            DbError::Timeout => write!(f, "Operation timeout"),
            DbError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for DbError {}

/// Result type for database operations
pub type DbResult<T> = Result<T, DbError>;

/// Convert from std::io::Error
impl From<std::io::Error> for DbError {
    fn from(err: std::io::Error) -> Self {
        DbError::Io(err.to_string())
    }
}

/// Convert from KernelError
impl From<DbError> for KernelError {
    fn from(err: DbError) -> Self {
        KernelError::Database(err.to_string())
    }
}

/// Macro for creating database errors
#[macro_export]
macro_rules! db_error {
    ($variant:ident, $msg:expr) => {
        $crate::db::error::DbError::$variant($msg.to_string())
    };
    ($variant:ident, $fmt:expr, $($arg:tt)*) => {
        $crate::db::error::DbError::$variant(format!($fmt, $($arg)*))
    };
}

/// Macro for database results
#[macro_export]
macro_rules! db_result {
    ($expr:expr) => {
        $expr.map_err(|e| $crate::db::error::DbError::Internal(e.to_string()))
    };
}
