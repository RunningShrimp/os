//! Database Catalog Management
//!
//! Central registry of all database metadata including schemas, tables,
 indexes, and statistics.

use super::schema::{Schema, Table, Index, TableStats};
use super::types::DataType;
use super::{DbError, DbResult};
use crate::sync::Mutex;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;

/// Catalog manager for database metadata
pub struct CatalogManager {
    schemas: Mutex<BTreeMap<String, Arc<Schema>>>,
    stats: Mutex<BTreeMap<String, TableStats>>,
}

impl CatalogManager {
    pub fn new() -> Self {
        Self {
            schemas: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(BTreeMap::new()),
        }
    }

    /// Create a new schema
    pub fn create_schema(&self, name: &str) -> DbResult<()> {
        let mut schemas = self.schemas.lock();

        if schemas.contains_key(name) {
            return Err(DbError::AlreadyExists(format!("Schema '{}'", name)));
        }

        let schema = Arc::new(Schema::new(name));
        schemas.insert(name.to_string(), schema);

        Ok(())
    }

    /// Drop a schema
    pub fn drop_schema(&self, name: &str) -> DbResult<()> {
        let mut schemas = self.schemas.lock();

        if !schemas.contains_key(name) {
            return Err(DbError::NotFound(format!("Schema '{}'", name)));
        }

        schemas.remove(name);
        Ok(())
    }

    /// Get a schema by name
    pub fn get_schema(&self, name: &str) -> DbResult<Arc<Schema>> {
        let schemas = self.schemas.lock();

        schemas
            .get(name)
            .cloned()
            .ok_or_else(|| DbError::NotFound(format!("Schema '{}'", name)))
    }

    /// List all schemas
    pub fn list_schemas(&self) -> Vec<String> {
        let schemas = self.schemas.lock();
        schemas.keys().cloned().collect()
    }

    /// Create a new table in a schema
    pub fn create_table(&self, schema_name: &str, table: Table) -> DbResult<()> {
        let schema = self.get_schema(schema_name)?;
        let mut schema_ref = Arc::try_unwrap(schema)
            .map_err(|_| DbError::Internal("Cannot modify locked schema".into()))?;

        schema_ref.add_table(table)?;

        // Re-wrap in Arc and update
        let mut schemas = self.schemas.lock();
        schemas.insert(schema_name.to_string(), Arc::new(schema_ref));

        Ok(())
    }

    /// Drop a table
    pub fn drop_table(&self, schema_name: &str, table_name: &str) -> DbResult<()> {
        let schema = self.get_schema(schema_name)?;

        let mut schema_ref = Arc::try_unwrap(schema)
            .map_err(|_| DbError::Internal("Cannot modify locked schema".into()))?;

        let index = schema_ref
            .tables
            .iter()
            .position(|t| t.name == table_name)
            .ok_or_else(|| DbError::NotFound(format!("Table '{}'", table_name)))?;

        schema_ref.tables.remove(index);

        // Update stats
        let mut stats = self.stats.lock();
        let key = format!("{}.{}", schema_name, table_name);
        stats.remove(&key);

        let mut schemas = self.schemas.lock();
        schemas.insert(schema_name.to_string(), Arc::new(schema_ref));

        Ok(())
    }

    /// Get a table
    pub fn get_table(&self, schema_name: &str, table_name: &str) -> DbResult<Arc<Table>> {
        let schema = self.get_schema(schema_name)?;

        schema
            .get_table(table_name)
            .map(|t| Arc::new(t.clone()))
            .ok_or_else(|| DbError::NotFound(format!("Table '{}.{}'", schema_name, table_name)))
    }

    /// List all tables in a schema
    pub fn list_tables(&self, schema_name: &str) -> DbResult<Vec<String>> {
        let schema = self.get_schema(schema_name)?;
        Ok(schema.tables.iter().map(|t| t.name.clone()).collect())
    }

    /// Create an index
    pub fn create_index(&self, schema_name: &str, table_name: &str, index: Index) -> DbResult<()> {
        let table = self.get_table(schema_name, table_name)?;
        let mut table_ref = Arc::try_unwrap(table)
            .map_err(|_| DbError::Internal("Cannot modify locked table".into()))?;

        table_ref.add_index(index)?;

        let schema = self.get_schema(schema_name)?;
        let mut schema_ref = Arc::try_unwrap(schema)
            .map_err(|_| DbError::Internal("Cannot modify locked schema".into()))?;

        if let Some(t) = schema_ref.get_table_mut(table_name) {
            *t = table_ref;
        }

        let mut schemas = self.schemas.lock();
        schemas.insert(schema_name.to_string(), Arc::new(schema_ref));

        Ok(())
    }

    /// Drop an index
    pub fn drop_index(&self, _schema_name: &str, _index_name: &str) -> DbResult<()> {
        // TODO: Implement index dropping
        Ok(())
    }

    /// Update table statistics
    pub fn update_stats(&self, schema_name: &str, table_name: &str, stats: TableStats) -> DbResult<()> {
        // Verify table exists
        self.get_table(schema_name, table_name)?;

        let key = format!("{}.{}", schema_name, table_name);
        let mut stats_map = self.stats.lock();
        stats_map.insert(key, stats);

        Ok(())
    }

    /// Get table statistics
    pub fn get_stats(&self, schema_name: &str, table_name: &str) -> DbResult<TableStats> {
        let key = format!("{}.{}", schema_name, table_name);
        let stats_map = self.stats.lock();

        stats_map
            .get(&key)
            .cloned()
            .ok_or_else(|| DbError::NotFound(format!("Statistics for '{}.{}'", schema_name, table_name)))
    }
}

impl Default for CatalogManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick lookup for table information
pub struct TableLookup {
    schema: String,
    table: String,
    pub table_ref: Arc<Table>,
}

impl TableLookup {
    pub fn new(schema: String, table: String, table_ref: Arc<Table>) -> Self {
        Self {
            schema,
            table,
            table_ref,
        }
    }

    pub fn qualified_name(&self) -> String {
        format!("{}.{}", self.schema, self.table)
    }
}
