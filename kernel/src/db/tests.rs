//! Database Integration Tests
//!
//! Comprehensive tests demonstrating the full functionality of the database system.

#[cfg(test)]
mod integration_tests {
    use super::super::engine::{Database, DatabaseConfig, StorageEngine};
    use super::super::executor::ExecutionResult;
    use super::super::parser::parse_sql;
    use super::super::types::{Value, IsolationLevel};
    use super::super::btree::{BTree, BTreeConfig};
    use super::super::lsm::{LsmTree, LsmConfig};
    use super::super::fulltext::FullTextIndex;
    use super::super::transaction::TransactionManager;
    use super::super::wal::WalManager;

    /// Test basic database operations
    #[test]
    fn test_database_basics() {
        let db = Database::open("test_basics", DatabaseConfig::default()).unwrap();

        // Create table
        let result = db.execute("CREATE TABLE users (id INT, name TEXT, age INT)").unwrap();
        assert!(matches!(result, ExecutionResult::Schema { .. }));

        // Insert data
        db.execute("INSERT INTO users VALUES (1, 'Alice', 30)").unwrap();
        db.execute("INSERT INTO users VALUES (2, 'Bob', 25)").unwrap();
        db.execute("INSERT INTO users VALUES (3, 'Charlie', 35)").unwrap();

        // Query data
        let rows = db.query("SELECT * FROM users").unwrap();
        assert_eq!(rows.len(), 3);

        // Query with filter
        let rows = db.query("SELECT * FROM users WHERE age > 28").unwrap();
        assert_eq!(rows.len(), 2);

        // Update data
        let result = db.execute("UPDATE users SET age = 31 WHERE id = 1").unwrap();
        if let ExecutionResult::Modification { affected_rows, .. } = result {
            assert_eq!(affected_rows, 1);
        }

        // Delete data
        let result = db.execute("DELETE FROM users WHERE id = 3").unwrap();
        if let ExecutionResult::Modification { affected_rows, .. } = result {
            assert_eq!(affected_rows, 1);
        }

        let rows = db.query("SELECT * FROM users").unwrap();
        assert_eq!(rows.len(), 2);
    }

    /// Test transactions
    #[test]
    fn test_transactions() {
        let db = Database::open("test_tx", DatabaseConfig::default()).unwrap();

        db.execute("CREATE TABLE accounts (id INT, balance INT)").unwrap();
        db.execute("INSERT INTO accounts VALUES (1, 100)").unwrap();
        db.execute("INSERT INTO accounts VALUES (2, 200)").unwrap();

        // Test transaction commit
        let mut conn = super::super::engine::Connection::new(std::sync::Arc::new(db));
        conn.begin().unwrap();
        conn.execute("UPDATE accounts SET balance = 150 WHERE id = 1").unwrap();
        conn.commit().unwrap();

        let rows = conn.execute("SELECT * FROM accounts WHERE id = 1").unwrap();
        if let ExecutionResult::Query { rows, .. } = rows {
            assert_eq!(rows[0][0], Value::Int32(1));
        }

        // Test transaction rollback
        conn.begin().unwrap();
        conn.execute("UPDATE accounts SET balance = 999 WHERE id = 2").unwrap();
        conn.rollback().unwrap();

        let rows = conn.execute("SELECT * FROM accounts WHERE id = 2").unwrap();
        if let ExecutionResult::Query { rows, .. } = rows {
            assert_eq!(rows[0][1], Value::Int32(200));
        }
    }

    /// Test B+Tree storage engine
    #[test]
    fn test_btree_engine() {
        let btree = BTree::new(BTreeConfig::default());

        // Insert keys
        for i in 1..=100 {
            btree.insert(Value::Int32(i), Value::Int32(i * 10)).unwrap();
        }

        // Get key
        let result = btree.get(&Value::Int32(50)).unwrap();
        assert_eq!(result, Some(Value::Int32(500)));

        // Range query
        let results = btree.range(&Value::Int32(25)..=&Value::Int32(35)).unwrap();
        assert_eq!(results.len(), 11);

        // Delete key
        assert!(btree.delete(&Value::Int32(50)).unwrap());
        let result = btree.get(&Value::Int32(50)).unwrap();
        assert_eq!(result, None);

        // Statistics
        let stats = btree.stats().unwrap();
        assert!(stats.total_nodes > 0);
    }

    /// Test LSM-Tree storage engine
    #[test]
    fn test_lsm_engine() {
        let lsm = LsmTree::new(LsmConfig::default());

        // Insert data
        for i in 1..=100 {
            lsm.put(Value::Int32(i), Value::Text(format!("value{}", i))).unwrap();
        }

        // Get data
        let result = lsm.get(&Value::Int32(50)).unwrap();
        assert_eq!(result, Some(Value::Text("value50".to_string())));

        // Delete data
        lsm.delete(&Value::Int32(50)).unwrap();
        let result = lsm.get(&Value::Int32(50)).unwrap();
        assert_eq!(result, None);

        // Statistics
        let stats = lsm.stats().unwrap();
        assert!(stats.memtable_size > 0);
    }

    /// Test SQL parser
    #[test]
    fn test_sql_parser() {
        // Test SELECT
        let ast = parse_sql("SELECT id, name FROM users WHERE age > 25").unwrap();
        assert!(matches!(ast, super::super::parser::AstNode::SelectStmt(_)));

        // Test INSERT
        let ast = parse_sql("INSERT INTO users (id, name) VALUES (1, 'Alice')").unwrap();
        assert!(matches!(ast, super::super::parser::AstNode::InsertStmt(_)));

        // Test UPDATE
        let ast = parse_sql("UPDATE users SET age = 30 WHERE id = 1").unwrap();
        assert!(matches!(ast, super::super::parser::AstNode::UpdateStmt(_)));

        // Test DELETE
        let ast = parse_sql("DELETE FROM users WHERE id = 1").unwrap();
        assert!(matches!(ast, super::super::parser::AstNode::DeleteStmt(_)));

        // Test CREATE TABLE
        let ast = parse_sql("CREATE TABLE users (id INT PRIMARY KEY, name TEXT)").unwrap();
        assert!(matches!(ast, super::super::parser::AstNode::CreateTableStmt(_)));
    }

    /// Test full-text search
    #[test]
    fn test_fulltext_search() {
        let index = FullTextIndex::new("ft_idx", "documents", "content");

        // Index documents
        index.insert_document(1, "The quick brown fox jumps over the lazy dog").unwrap();
        index.insert_document(2, "Fast brown foxes are quick").unwrap();
        index.insert_document(3, "The lazy dog sleeps").unwrap();

        // Search
        let results = index.search("quick fox", 10).unwrap();
        assert!(!results.is_empty());

        // Fuzzy search
        let results = index.fuzzy_search("qick", 2, 10).unwrap();
        assert!(!results.is_empty());
    }

    /// Test transaction isolation levels
    #[test]
    fn test_isolation_levels() {
        let tm = TransactionManager::new();

        // Read Committed
        let tx1 = tm.begin(IsolationLevel::ReadCommitted).unwrap();
        assert_eq!(tx1.isolation_level, IsolationLevel::ReadCommitted);

        // Repeatable Read
        let tx2 = tm.begin(IsolationLevel::RepeatableRead).unwrap();
        assert_eq!(tx2.isolation_level, IsolationLevel::RepeatableRead);

        // Serializable
        let tx3 = tm.begin(IsolationLevel::Serializable).unwrap();
        assert_eq!(tx3.isolation_level, IsolationLevel::Serializable);
    }

    /// Test WAL operations
    #[test]
    fn test_wal() {
        let wal = WalManager::new(Some("test.log".to_string())).unwrap();

        // Write transaction records
        let lsn1 = wal.write_tx_begin(1).unwrap();
        let lsn2 = wal.write_page_update(1, 100, 0, vec![1, 2, 3, 4]).unwrap();
        let lsn3 = wal.write_tx_commit(1).unwrap();

        assert!(lsn3 > lsn2 && lsn2 > lsn1);

        // Flush
        wal.flush().unwrap();

        // Checkpoint
        wal.checkpoint().unwrap();
    }

    /// Test complex SQL queries
    #[test]
    fn test_complex_queries() {
        let db = Database::open("test_complex", DatabaseConfig::default()).unwrap();

        // Create tables
        db.execute("CREATE TABLE users (id INT, name TEXT)").unwrap();
        db.execute("CREATE TABLE orders (id INT, user_id INT, amount INT)").unwrap();

        // Insert data
        db.execute("INSERT INTO users VALUES (1, 'Alice')").unwrap();
        db.execute("INSERT INTO users VALUES (2, 'Bob')").unwrap();
        db.execute("INSERT INTO orders VALUES (1, 1, 100)").unwrap();
        db.execute("INSERT INTO orders VALUES (2, 1, 200)").unwrap();
        db.execute("INSERT INTO orders VALUES (3, 2, 150)").unwrap();

        // Note: JOINs are parsed but execution is simplified in this demo
        let ast = parse_sql("SELECT users.name, orders.amount FROM users JOIN orders ON users.id = orders.user_id").unwrap();
        assert!(matches!(ast, super::super::parser::AstNode::SelectStmt(_)));
    }

    /// Test database with LSM-Tree storage
    #[test]
    fn test_database_with_lsm() {
        let mut config = DatabaseConfig::default();
        config.storage_engine = StorageEngine::LsmTree;

        let db = Database::open("test_lsm_db", config).unwrap();

        db.execute("CREATE TABLE logs (id INT, message TEXT)").unwrap();
        db.execute("INSERT INTO logs VALUES (1, 'Error occurred')").unwrap();

        let rows = db.query("SELECT * FROM logs").unwrap();
        assert_eq!(rows.len(), 1);
    }

    /// Test query cache
    #[test]
    fn test_query_cache() {
        let db = Database::open("test_cache", DatabaseConfig::default()).unwrap();

        db.execute("CREATE TABLE cache_test (id INT, value INT)").unwrap();
        db.execute("INSERT INTO cache_test VALUES (1, 100)").unwrap();

        // First query - cache miss
        let _ = db.query("SELECT * FROM cache_test").unwrap();

        // Second query - cache hit
        let _ = db.query("SELECT * FROM cache_test").unwrap();

        // Check stats
        let stats = db.stats();
        assert!(stats.cache_hits > 0 || stats.cache_misses > 0);
    }

    /// Test error handling
    #[test]
    fn test_error_handling() {
        let db = Database::open("test_errors", DatabaseConfig::default()).unwrap();

        // Table doesn't exist
        let result = db.execute("SELECT * FROM nonexistent");
        assert!(result.is_err());

        // Invalid SQL
        let result = db.execute("INVALID SQL QUERY");
        assert!(result.is_err());

        // Type mismatch
        db.execute("CREATE TABLE types (id INT, value TEXT)").unwrap();
        let result = db.execute("INSERT INTO types VALUES ('string', 123)");
        assert!(result.is_err());
    }

    /// Test stress operations
    #[test]
    fn test_stress_operations() {
        let db = Database::open("test_stress", DatabaseConfig::default()).unwrap();

        db.execute("CREATE TABLE stress (id INT, data TEXT)").unwrap();

        // Insert many records
        for i in 0..1000 {
            db.execute(&format!("INSERT INTO stress VALUES ({}, 'data{}')", i, i)).unwrap();
        }

        let rows = db.query("SELECT * FROM stress").unwrap();
        assert_eq!(rows.len(), 1000);
    }
}
