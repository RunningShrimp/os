//! Database Schema Management
//!
//! Defines tables, columns, indexes, and constraints.

use super::types::{DataType, Value};
use super::{DbError, DbResult};
use std::collections::HashSet;
use std::sync::Arc;

/// Column definition in a table
#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default_value: Option<Value>,
    pub primary_key: bool,
    pub unique: bool,
    pub auto_increment: bool,
}

impl Column {
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable: true,
            default_value: None,
            primary_key: false,
            unique: false,
            auto_increment: false,
        }
    }

    pub fn nullable(mut self, nullable: bool) -> Self {
        self.nullable = nullable;
        self
    }

    pub fn default(mut self, value: Value) -> Self {
        self.default_value = Some(value);
        self
    }

    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self.nullable = false;
        self.unique = true;
        self
    }

    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    pub fn auto_increment(mut self) -> Self {
        self.auto_increment = true;
        self
    }

    pub fn validate(&self, value: &Value) -> DbResult<()> {
        // Check NULL constraint
        if value.is_null() && !self.nullable {
            return Err(DbError::ConstraintViolation(format!(
                "Column '{}' cannot be NULL",
                self.name
            )));
        }

        // Check type compatibility
        if !value.is_null() {
            let value_type = value.data_type();
            if value_type != DataType::Null && value_type != self.data_type {
                // Allow some implicit conversions
                if !self.compatible_types(value_type, self.data_type) {
                    return Err(DbError::TypeMismatch(format!(
                        "Column '{}' expects {} but got {}",
                        self.name, self.data_type, value_type
                    )));
                }
            }
        }

        Ok(())
    }

    fn compatible_types(&self, from: DataType, to: DataType) -> bool {
        // Allow numeric type conversions
        if from.is_numeric() && to.is_numeric() {
            return true;
        }
        false
    }
}

/// Table definition
#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub primary_key: Vec<String>,
    pub indexes: Vec<Index>,
}

impl Table {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
            primary_key: Vec::new(),
            indexes: Vec::new(),
        }
    }

    pub fn with_columns(mut self, columns: Vec<Column>) -> Self {
        for col in &columns {
            if col.primary_key {
                self.primary_key.push(col.name.clone());
            }
        }
        self.columns = columns;
        self
    }

    pub fn add_column(&mut self, column: Column) -> DbResult<()> {
        // Check for duplicate column names
        if self.columns.iter().any(|c| c.name == column.name) {
            return Err(DbError::AlreadyExists(format!(
                "Column '{}' already exists in table '{}'",
                column.name, self.name
            )));
        }

        if column.primary_key {
            self.primary_key.push(column.name.clone());
        }

        self.columns.push(column);
        Ok(())
    }

    pub fn get_column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.name == name)
    }

    pub fn get_column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.name == name)
    }

    pub fn add_index(&mut self, index: Index) -> DbResult<()> {
        // Validate index columns exist
        for col in &index.columns {
            if self.get_column(col).is_none() {
                return Err(DbError::NotFound(format!(
                    "Column '{}' not found in table '{}'",
                    col, self.name
                )));
            }
        }

        self.indexes.push(index);
        Ok(())
    }

    pub fn validate_row(&self, row: &[Value]) -> DbResult<()> {
        if row.len() != self.columns.len() {
            return Err(DbError::InvalidArgument(format!(
                "Row has {} values but table '{}' expects {}",
                row.len(),
                self.name,
                self.columns.len()
            )));
        }

        for (col, value) in self.columns.iter().zip(row.iter()) {
            col.validate(value)?;
        }

        Ok(())
    }

    pub fn has_primary_key(&self) -> bool {
        !self.primary_key.is_empty()
    }
}

/// Index definition
#[derive(Debug, Clone)]
pub struct Index {
    pub name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub index_type: IndexType,
}

impl Index {
    pub fn new(name: impl Into<String>, table_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            table_name: table_name.into(),
            columns: Vec::new(),
            unique: false,
            index_type: IndexType::BTree,
        }
    }

    pub fn with_columns(mut self, columns: Vec<String>) -> Self {
        self.columns = columns;
        self
    }

    pub fn unique(mut self, unique: bool) -> Self {
        self.unique = unique;
        self
    }

    pub fn index_type(mut self, index_type: IndexType) -> Self {
        self.index_type = index_type;
        self
    }
}

/// Index type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexType {
    /// B-Tree index (default)
    BTree,

    /// Hash index
    Hash,

    /// Full-text index
    FullText,

    /// GiST index
    GiST,
}

/// Foreign key constraint
#[derive(Debug, Clone)]
pub struct ForeignKey {
    pub name: String,
    pub columns: Vec<String>,
    pub ref_table: String,
    pub ref_columns: Vec<String>,
    pub on_delete: ReferentialAction,
    pub on_update: ReferentialAction,
}

/// Action to take on reference update/delete
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferentialAction {
    NoAction,
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
}

/// Check constraint
#[derive(Debug, Clone)]
pub struct CheckConstraint {
    pub name: String,
    pub expression: String,
}

/// Schema containing multiple tables
#[derive(Debug, Clone)]
pub struct Schema {
    pub name: String,
    pub tables: Vec<Table>,
}

impl Schema {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tables: Vec::new(),
        }
    }

    pub fn add_table(&mut self, table: Table) -> DbResult<()> {
        // Check for duplicate table names
        if self.tables.iter().any(|t| t.name == table.name) {
            return Err(DbError::AlreadyExists(format!(
                "Table '{}' already exists in schema '{}'",
                table.name, self.name
            )));
        }

        self.tables.push(table);
        Ok(())
    }

    pub fn get_table(&self, name: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.name == name)
    }

    pub fn get_table_mut(&mut self, name: &str) -> Option<&mut Table> {
        self.tables.iter_mut().find(|t| t.name == name)
    }

    pub fn table_exists(&self, name: &str) -> bool {
        self.get_table(name).is_some()
    }
}

/// Database statistics for query optimization
#[derive(Debug, Clone)]
pub struct TableStats {
    pub table_name: String,
    pub row_count: u64,
    pub page_count: u64,
    pub avg_row_size: usize,
    pub column_stats: Vec<ColumnStats>,
}

#[derive(Debug, Clone)]
pub struct ColumnStats {
    pub column_name: String,
    pub null_count: u64,
    pub distinct_count: u64,
    pub min_value: Option<Value>,
    pub max_value: Option<Value>,
    pub histogram: Option<Vec<u64>>,
}

impl TableStats {
    pub fn estimate_selectivity(&self, column: &str) -> f64 {
        if let Some(col_stat) = self.column_stats.iter().find(|c| c.column_name == column) {
            if self.row_count > 0 {
                return col_stat.distinct_count as f64 / self.row_count as f64;
            }
        }
        0.1 // Default selectivity estimate
    }
}
