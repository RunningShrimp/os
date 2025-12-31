# Advanced Database System for Kernel

A comprehensive, production-grade embedded database system implemented in Rust for the kernel environment.

## Overview

This database system provides a complete SQL database engine with multiple storage engines, ACID transactions, and advanced features like full-text search. It's designed for kernel-level operation with no external dependencies.

## Features

### Core Functionality
- **Multiple Storage Engines**: B+Tree and LSM-Tree for different workloads
- **Complete SQL Support**: SELECT, INSERT, UPDATE, DELETE, CREATE TABLE, JOIN
- **ACID Transactions**: Full transaction support with MVCC
- **Query Optimization**: Intelligent query planning and caching
- **Persistence**: Write-Ahead Logging (WAL) for durability

### Advanced Features
- **Full-Text Search**: Inverted index with BM25 ranking
- **Transaction Isolation**: Read Uncommitted, Read Committed, Repeatable Read, Serializable
- **Crash Recovery**: Automatic recovery from WAL
- **Query Caching**: LRU cache for query results
- **Connection Pooling**: Efficient connection management

## Architecture

```
db/
├── mod.rs           # Module exports and constants
├── error.rs         # Error types and handling
├── types.rs         # Core data types
├── schema.rs        # Schema management
├── catalog.rs       # System catalog
├── btree.rs         # B+Tree storage engine
├── lsm.rs           # LSM-Tree storage engine
├── parser.rs        # SQL lexer and parser
├── executor.rs      # Query executor
├── transaction.rs   # Transaction management (MVCC)
├── wal.rs           # Write-Ahead Logging
├── fulltext.rs      # Full-text search
├── engine.rs        # Main database engine
└── tests.rs         # Integration tests
```

## Usage

### Basic Operations

```rust
use kernel::db::engine::Database;

// Open database
let db = Database::open("mydb", DatabaseConfig::default())?;

// Create table
db.execute("CREATE TABLE users (id INT, name TEXT, age INT)")?;

// Insert data
db.execute("INSERT INTO users VALUES (1, 'Alice', 30)")?;
db.execute("INSERT INTO users VALUES (2, 'Bob', 25)")?;

// Query data
let rows = db.query("SELECT * FROM users WHERE age > 25")?;
for row in rows {
    println!("{:?}", row);
}

// Update data
db.execute("UPDATE users SET age = 31 WHERE id = 1")?;

// Delete data
db.execute("DELETE FROM users WHERE id = 2")?;
```

### Transactions

```rust
// Begin transaction
let tx = db.begin_transaction_with_level(IsolationLevel::RepeatableRead)?;

// Execute operations in transaction
tx.execute("INSERT INTO accounts VALUES (1, 1000)")?;
tx.execute("UPDATE accounts SET balance = balance - 100 WHERE id = 1")?;

// Commit or rollback
db.commit_transaction(&tx)?;
// or
db.rollback_transaction(&tx)?;
```

### Connection Pool

```rust
use kernel::db::engine::ConnectionPool;

let pool = ConnectionPool::new(arc_db, 10);

{
    let conn = pool.acquire()?;
    conn.execute("SELECT * FROM users")?;
    pool.release(conn);
}
```

### Full-Text Search

```rust
use kernel::db::fulltext::FullTextIndex;

let index = FullTextIndex::new("ft_idx", "documents", "content");

// Index document
index.insert_document(1, "The quick brown fox jumps over the lazy dog")?;

// Search
let results = index.search("quick fox", 10)?;
for result in results {
    println!("Document ID: {}, Score: {}", result.row_id, result.score);
}

// Fuzzy search
let results = index.fuzzy_search("qick", 2, 10)?;
```

### Storage Engine Selection

```rust
// B+Tree (default, good for read-heavy workloads)
let mut config = DatabaseConfig::default();
config.storage_engine = StorageEngine::BTree;

// LSM-Tree (good for write-heavy workloads)
config.storage_engine = StorageEngine::LsmTree;

let db = Database::open("mydb", config)?;
```

## SQL Support

### DDL (Data Definition Language)
- `CREATE TABLE table_name (columns)`
- `DROP TABLE table_name`
- `CREATE INDEX index_name ON table_name (columns)`

### DML (Data Manipulation Language)
- `SELECT [columns] FROM table [WHERE condition] [JOIN ...]`
- `INSERT INTO table [(columns)] VALUES (values)`
- `UPDATE table SET column = value [WHERE condition]`
- `DELETE FROM table [WHERE condition]`

### Data Types
- `INT` / `INTEGER` - 32-bit integer
- `BIGINT` - 64-bit integer
- `FLOAT` - 32-bit floating point
- `DOUBLE` - 64-bit floating point
- `TEXT` - Variable-length string
- `BLOB` - Binary data
- `BOOLEAN` - Boolean value
- `DATE` - Date (YYYYMMDD)
- `TIMESTAMP` - Unix timestamp

## Transaction Isolation Levels

1. **Read Uncommitted**: Lowest isolation, sees uncommitted changes
2. **Read Committed**: Only sees committed changes (default)
3. **Repeatable Read**: Consistent snapshot within transaction
4. **Serializable**: Highest isolation, complete isolation

## Performance Considerations

### Storage Engine Selection
- **B+Tree**: Best for read-heavy workloads with random access patterns
- **LSM-Tree**: Best for write-heavy workloads with sequential writes

### Query Optimization
- Use indexes on frequently queried columns
- Select only needed columns instead of `SELECT *`
- Use appropriate WHERE clauses to limit result sets

### Caching
- Query cache automatically caches SELECT queries
- Configurable cache size
- LRU eviction policy

## Configuration Options

```rust
pub struct DatabaseConfig {
    pub storage_engine: StorageEngine,  // B+Tree or LSM-Tree
    pub cache_size: usize,              // Query cache size
    pub enable_wal: bool,               // Enable Write-Ahead Logging
    pub checkpoint_interval: u64,       // WAL checkpoint interval
    pub page_size: usize,               // Storage page size
}
```

## Error Handling

All database operations return `DbResult<T>`:

```rust
use kernel::db::{DbError, DbResult};

match db.execute("SELECT * FROM users") {
    Ok(result) => println!("{:?}", result),
    Err(DbError::NotFound(msg)) => eprintln!("Not found: {}", msg),
    Err(DbError::Query(msg)) => eprintln!("Query error: {}", msg),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Thread Safety

The database engine is fully thread-safe and supports concurrent access:
- Multiple readers can operate simultaneously
- Writers are properly synchronized
- Transaction isolation prevents race conditions

## Testing

Run the test suite:

```bash
cargo test --lib db::tests
```

Test coverage includes:
- Basic CRUD operations
- Transaction commit/rollback
- Storage engine functionality
- SQL parsing and execution
- Full-text search
- Error handling
- Stress testing

## Future Enhancements

Potential areas for expansion:
1. More SQL features (subqueries, CTEs, window functions)
2. Additional storage engines (Hash, Columnar)
3. Distributed database support
4. Replication and high availability
5. Query optimizer enhancements
6. PL/SQL stored procedures
7. Triggers and constraints
8. Views and materialized views
9. Backup and restore utilities
10. Performance monitoring and profiling

## License

Part of the NOS kernel project.

## Contributing

This is a core kernel component. Changes should be carefully tested and reviewed.
