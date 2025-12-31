//! Advanced Database System for Kernel
//!
//! This module provides a comprehensive, production-grade embedded database system
//! with multiple storage engines, SQL support, ACID transactions, and full-text search.
//!
//! # Architecture
//!
//! The database system is organized into several key components:
//!
//! - **Storage Engines**: B+Tree and LSM-Tree for different use cases
//! - **SQL Processing**: Parser, optimizer, and executor
//! - **Transaction Management**: ACID guarantees with MVCC
//! - **Persistence**: WAL-based durability and crash recovery
//! - **Search**: Full-text search with inverted indices
//!
//! # Features
//!
//! - Multiple storage engines (B+Tree, LSM-Tree)
//! - Complete SQL subset (SELECT, INSERT, UPDATE, DELETE, JOIN)
//! - ACID transactions with MVCC isolation
//! - Write-Ahead Logging (WAL) for durability
//! - Full-text search with BM25 ranking
//! - Query optimization and caching
//!
//! # Example
//!
//! ```rust,ignore
//! use kernel::db::engine::Database;
//!
//! // Create or open a database
//! let db = Database::open("/path/to/db")?;
//!
//! // Execute SQL queries
//! let result = db.execute("CREATE TABLE users (id INT, name TEXT)")?;
//! let result = db.execute("INSERT INTO users VALUES (1, 'Alice')")?;
//! let rows = db.query("SELECT * FROM users WHERE id = 1")?;
//!
//! // Transactions
//! let tx = db.begin_transaction()?;
//! tx.execute("INSERT INTO users VALUES (2, 'Bob')")?;
//! tx.commit()?;
//! ```

pub mod engine;
pub mod btree;
pub mod lsm;
pub mod parser;
pub mod executor;
pub mod transaction;
pub mod wal;
pub mod fulltext;

mod types;
mod error;
mod schema;
mod catalog;

pub use types::*;
pub use error::{DbError, DbResult};
pub use schema::{Schema, Column, Table, Index};
pub use catalog::{CatalogManager};

/// Database version
pub const DB_VERSION: &str = "1.0.0";

/// Default page size for storage engines (4KB)
pub const DEFAULT_PAGE_SIZE: usize = 4096;

/// Maximum key size
pub const MAX_KEY_SIZE: usize = 255;

/// Maximum value size
pub const MAX_VALUE_SIZE: usize = 65536;

/// Default cache size in pages
pub const DEFAULT_CACHE_SIZE: usize = 1024;

/// WAL segment size
pub const WAL_SEGMENT_SIZE: usize = 16 * 1024 * 1024; // 16MB

/// Maximum transaction duration in seconds
pub const MAX_TX_DURATION: u64 = 300;
