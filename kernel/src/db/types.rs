//! Core Database Types
//!
//! Fundamental data types used throughout the database system.

use super::{DbError, DbResult};
use crate::db::error;
use std::collections::HashMap;

/// SQL data types supported by the database
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    /// NULL value
    Null,

    /// Boolean (true/false)
    Boolean,

    /// 8-bit signed integer
    Int8,

    /// 16-bit signed integer
    Int16,

    /// 32-bit signed integer
    Int32,

    /// 64-bit signed integer
    Int64,

    /// 32-bit floating point
    Float32,

    /// 64-bit floating point
    Float64,

    /// Text/string
    Text,

    /// Binary data
    Blob,

    /// Date (year, month, day)
    Date,

    /// Timestamp (date + time)
    Timestamp,
}

impl DataType {
    /// Get the size of the type in bytes (for fixed-size types)
    pub fn size(&self) -> Option<usize> {
        match self {
            DataType::Null | DataType::Boolean => Some(1),
            DataType::Int8 => Some(1),
            DataType::Int16 => Some(2),
            DataType::Int32 | DataType::Float32 => Some(4),
            DataType::Int64 | DataType::Float64 | DataType::Timestamp => Some(8),
            DataType::Date => Some(4),
            DataType::Text | DataType::Blob => None, // Variable size
        }
    }

    /// Check if type is numeric
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            DataType::Int8
                | DataType::Int16
                | DataType::Int32
                | DataType::Int64
                | DataType::Float32
                | DataType::Float64
        )
    }

    /// Check if type is variable-length
    pub fn is_variable(&self) -> bool {
        matches!(self, DataType::Text | DataType::Blob)
    }
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Null => write!(f, "NULL"),
            DataType::Boolean => write!(f, "BOOLEAN"),
            DataType::Int8 => write!(f, "INT8"),
            DataType::Int16 => write!(f, "INT16"),
            DataType::Int32 => write!(f, "INT"),
            DataType::Int64 => write!(f, "BIGINT"),
            DataType::Float32 => write!(f, "FLOAT"),
            DataType::Float64 => write!(f, "DOUBLE"),
            DataType::Text => write!(f, "TEXT"),
            DataType::Blob => write!(f, "BLOB"),
            DataType::Date => write!(f, "DATE"),
            DataType::Timestamp => write!(f, "TIMESTAMP"),
        }
    }
}

/// Database value that can be stored in columns
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Boolean(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Text(String),
    Blob(Vec<u8>),
    Date(i32), // YYYYMMDD as integer
    Timestamp(i64), // Unix timestamp
}

impl Value {
    /// Get the data type of this value
    pub fn data_type(&self) -> DataType {
        match self {
            Value::Null => DataType::Null,
            Value::Boolean(_) => DataType::Boolean,
            Value::Int8(_) => DataType::Int8,
            Value::Int16(_) => DataType::Int16,
            Value::Int32(_) => DataType::Int32,
            Value::Int64(_) => DataType::Int64,
            Value::Float32(_) => DataType::Float32,
            Value::Float64(_) => DataType::Float64,
            Value::Text(_) => DataType::Text,
            Value::Blob(_) => DataType::Blob,
            Value::Date(_) => DataType::Date,
            Value::Timestamp(_) => DataType::Timestamp,
        }
    }

    /// Get size in bytes
    pub fn size(&self) -> usize {
        match self {
            Value::Null => 0,
            Value::Boolean(_) => 1,
            Value::Int8(_) => 1,
            Value::Int16(_) => 2,
            Value::Int32(_) | Value::Float32(_) | Value::Date(_) => 4,
            Value::Int64(_) | Value::Float64(_) | Value::Timestamp(_) => 8,
            Value::Text(s) => s.len(),
            Value::Blob(b) => b.len(),
        }
    }

    /// Convert to i64 if numeric
    pub fn as_i64(&self) -> DbResult<i64> {
        match self {
            Value::Int8(v) => Ok(*v as i64),
            Value::Int16(v) => Ok(*v as i64),
            Value::Int32(v) => Ok(*v as i64),
            Value::Int64(v) => Ok(*v),
            Value::Float32(v) => Ok(*v as i64),
            Value::Float64(v) => Ok(*v as i64),
            _ => Err(DbError::TypeMismatch(format!("Cannot convert {:?} to i64", self))),
        }
    }

    /// Convert to f64 if numeric
    pub fn as_f64(&self) -> DbResult<f64> {
        match self {
            Value::Int8(v) => Ok(*v as f64),
            Value::Int16(v) => Ok(*v as f64),
            Value::Int32(v) => Ok(*v as f64),
            Value::Int64(v) => Ok(*v as f64),
            Value::Float32(v) => Ok(*v as f64),
            Value::Float64(v) => Ok(*v),
            _ => Err(DbError::TypeMismatch(format!("Cannot convert {:?} to f64", self))),
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> DbResult<&str> {
        match self {
            Value::Text(s) => Ok(s),
            _ => Err(DbError::TypeMismatch(format!("Cannot convert {:?} to string", self))),
        }
    }

    /// Convert to bytes
    pub fn as_bytes(&self) -> DbResult<&[u8]> {
        match self {
            Value::Blob(b) => Ok(b),
            _ => Err(DbError::TypeMismatch(format!("Cannot convert {:?} to bytes", self))),
        }
    }

    /// Check if value is NULL
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "NULL"),
            Value::Boolean(v) => write!(f, "{}", v),
            Value::Int8(v) => write!(f, "{}", v),
            Value::Int16(v) => write!(f, "{}", v),
            Value::Int32(v) => write!(f, "{}", v),
            Value::Int64(v) => write!(f, "{}", v),
            Value::Float32(v) => write!(f, "{}", v),
            Value::Float64(v) => write!(f, "{}", v),
            Value::Text(v) => write!(f, "'{}'", v),
            Value::Blob(_) => write!(f, "<BLOB>"),
            Value::Date(v) => write!(f, "{}", v),
            Value::Timestamp(v) => write!(f, "{}", v),
        }
    }
}

/// A row of values
pub type Row = Vec<Value>;

/// Multiple rows
pub type RowSet = Vec<Row>;

/// Column reference (table.column or just column)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ColumnRef {
    pub table: Option<String>,
    pub name: String,
}

impl ColumnRef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            table: None,
            name: name.into(),
        }
    }

    pub fn qualified(table: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            table: Some(table.into()),
            name: name.into(),
        }
    }
}

impl std::fmt::Display for ColumnRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(table) = &self.table {
            write!(f, "{}.{}", table, self.name)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Ascending,
    Descending,
}

/// Comparison operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Like,
    NotLike,
    In,
    NotIn,
    IsNull,
    IsNotNull,
}

impl std::fmt::Display for ComparisonOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComparisonOp::Equal => write!(f, "="),
            ComparisonOp::NotEqual => write!(f, "!="),
            ComparisonOp::Less => write!(f, "<"),
            ComparisonOp::LessEqual => write!(f, "<="),
            ComparisonOp::Greater => write!(f, ">"),
            ComparisonOp::GreaterEqual => write!(f, ">="),
            ComparisonOp::Like => write!(f, "LIKE"),
            ComparisonOp::NotLike => write!(f, "NOT LIKE"),
            ComparisonOp::In => write!(f, "IN"),
            ComparisonOp::NotIn => write!(f, "NOT IN"),
            ComparisonOp::IsNull => write!(f, "IS NULL"),
            ComparisonOp::IsNotNull => write!(f, "IS NOT NULL"),
        }
    }
}

/// Logical operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
    Not,
}

/// Arithmetic operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}

/// Join type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// Transaction isolation level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// Read uncommitted (lowest isolation)
    ReadUncommitted,

    /// Read committed
    ReadCommitted,

    /// Repeatable read
    RepeatableRead,

    /// Serializable (highest isolation)
    Serializable,
}

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxStatus {
    /// Transaction is active
    Active,

    /// Transaction committed
    Committed,

    /// Transaction rolled back
    RolledBack,
}

/// Unique identifier for a transaction
pub type TxId = u64;

/// Unique identifier for a page
pub type PageId = u64;

/// Unique identifier for a row
pub type RowId = u64;
