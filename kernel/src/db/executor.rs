//! SQL Query Executor
//!
//! Executes parsed SQL AST nodes and produces results.

use super::parser::{AstNode, SelectStatement, InsertStatement, UpdateStatement, DeleteStatement, CreateTableStatement};
use super::types::{Value, Row, RowSet, DataType};
use super::schema::Table;
use super::{DbError, DbResult, DbResult as Result};
use crate::sync::Mutex;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::RefCell;

/// Query executor
pub struct Executor {
    // In a real implementation, this would hold references to storage engines, catalog, etc.
    tables: Mutex<BTreeMap<String, Arc<Table>>>,
    data: Mutex<BTreeMap<String, Vec<Row>>>, // In-memory storage for demo
}

impl Executor {
    pub fn new() -> Self {
        Self {
            tables: Mutex::new(BTreeMap::new()),
            data: Mutex::new(BTreeMap::new()),
        }
    }

    /// Execute an AST node
    pub fn execute(&self, node: AstNode) -> DbResult<ExecutionResult> {
        match node {
            AstNode::SelectStmt(stmt) => self.execute_select(stmt),
            AstNode::InsertStmt(stmt) => self.execute_insert(stmt),
            AstNode::UpdateStmt(stmt) => self.execute_update(stmt),
            AstNode::DeleteStmt(stmt) => self.execute_delete(stmt),
            AstNode::CreateTableStmt(stmt) => self.execute_create_table(stmt),
            AstNode::DropTableStmt(stmt) => self.execute_drop_table(stmt),
            AstNode::CreateIndexStmt(stmt) => self.execute_create_index(stmt),
        }
    }

    fn execute_select(&self, stmt: SelectStatement) -> DbResult<ExecutionResult> {
        // Get table
        let table_name = &stmt.from.first().ok_or_else(|| DbError::Query("No table specified".into()))?.name;
        let table = self.get_table(table_name)?;

        // Get all rows
        let data = self.data.lock();
        let rows = data.get(table_name).cloned().unwrap_or_default();
        drop(data);

        // Filter rows
        let mut filtered_rows = Vec::new();
        for row in rows {
            if self.matches_where(&stmt.where_clause, &row, &table)? {
                filtered_rows.push(row);
            }
        }

        // Project columns
        let mut result_rows = Vec::new();
        for row in filtered_rows {
            let projected = self.project_columns(&stmt.columns, &row, &table)?;
            result_rows.push(projected);
        }

        Ok(ExecutionResult::Query {
            columns: self.get_column_names(&stmt.columns, &table),
            rows: result_rows,
            affected_rows: 0,
        })
    }

    fn execute_insert(&self, stmt: InsertStatement) -> DbResult<ExecutionResult> {
        let table = self.get_table(&stmt.table_name)?;

        let mut inserted = 0;
        let mut data = self.data.lock();
        let table_data = data.entry(stmt.table_name.clone()).or_insert_with(Vec::new);

        for value_row in &stmt.values {
            let mut row = Vec::new();

            // Map values to columns
            if stmt.columns.is_empty() {
                // Insert into all columns in order
                for (col, expr) in table.columns.iter().zip(value_row.iter()) {
                    let value = self.eval_expr(expr)?;
                    row.push(value);
                }
            } else {
                // Insert into specified columns
                for col in &table.columns {
                    let idx = stmt.columns.iter().position(|c| c == &col.name);
                    if let Some(idx) = idx {
                        row.push(self.eval_expr(&value_row[idx])?);
                    } else {
                        // Use default value
                        row.push(col.default_value.clone().unwrap_or(Value::Null));
                    }
                }
            }

            table.validate_row(&row)?;
            table_data.push(row);
            inserted += 1;
        }

        Ok(ExecutionResult::Modification {
            affected_rows: inserted,
            last_id: None,
        })
    }

    fn execute_update(&self, stmt: UpdateStatement) -> DbResult<ExecutionResult> {
        let table = self.get_table(&stmt.table_name)?;

        let mut updated = 0;
        let mut data = self.data.lock();

        if let Some(table_data) = data.get_mut(&stmt.table_name) {
            for row in table_data.iter_mut() {
                if self.matches_where(&stmt.where_clause, row, &table)? {
                    for assignment in &stmt.assignments {
                        let col_idx = table.get_column_index(&assignment.column)
                            .ok_or_else(|| DbError::NotFound(format!("Column {}", assignment.column)))?;

                        let value = self.eval_expr(&assignment.value)?;
                        row[col_idx] = value;
                    }
                    updated += 1;
                }
            }
        }

        Ok(ExecutionResult::Modification {
            affected_rows: updated,
            last_id: None,
        })
    }

    fn execute_delete(&self, stmt: DeleteStatement) -> DbResult<ExecutionResult> {
        let table = self.get_table(&stmt.table_name)?;

        let mut deleted = 0;
        let mut data = self.data.lock();

        if let Some(table_data) = data.get_mut(&stmt.table_name) {
            let mut new_data = Vec::new();

            for row in table_data.iter() {
                if self.matches_where(&stmt.where_clause, row, &table)? {
                    deleted += 1;
                } else {
                    new_data.push(row.clone());
                }
            }

            *table_data = new_data;
        }

        Ok(ExecutionResult::Modification {
            affected_rows: deleted,
            last_id: None,
        })
    }

    fn execute_create_table(&self, stmt: CreateTableStatement) -> DbResult<ExecutionResult> {
        let mut tables = self.tables.lock();

        if tables.contains_key(&stmt.table_name) {
            return Err(DbError::AlreadyExists(format!("Table {}", stmt.table_name)));
        }

        // Create table from column definitions
        let mut table = super::schema::Table::new(&stmt.table_name);
        for col_def in &stmt.columns {
            let mut col = super::schema::Column::new(&col_def.name, col_def.data_type);
            col.nullable = col_def.nullable;
            col.default_value = col_def.default_value.clone();
            col.primary_key = col_def.primary_key;
            col.unique = col_def.unique;
            col.auto_increment = col_def.auto_increment;
            table.add_column(col)?;
        }

        tables.insert(stmt.table_name.clone(), Arc::new(table));

        Ok(ExecutionResult::Schema {
            message: format!("Table {} created", stmt.table_name),
        })
    }

    fn execute_drop_table(&self, stmt: super::parser::DropTableStatement) -> DbResult<ExecutionResult> {
        let mut tables = self.tables.lock();
        let mut data = self.data.lock();

        if !stmt.if_exists && !tables.contains_key(&stmt.table_name) {
            return Err(DbError::NotFound(format!("Table {}", stmt.table_name)));
        }

        tables.remove(&stmt.table_name);
        data.remove(&stmt.table_name);

        Ok(ExecutionResult::Schema {
            message: format!("Table {} dropped", stmt.table_name),
        })
    }

    fn execute_create_index(&self, stmt: super::parser::CreateIndexStatement) -> DbResult<ExecutionResult> {
        // In a real implementation, this would create an index on the specified columns
        Ok(ExecutionResult::Schema {
            message: format!("Index {} created on table {}", stmt.index_name, stmt.table_name),
        })
    }

    // Helper methods

    fn get_table(&self, name: &str) -> DbResult<Arc<Table>> {
        let tables = self.tables.lock();
        tables.get(name).cloned()
            .ok_or_else(|| DbError::NotFound(format!("Table {}", name)))
    }

    fn matches_where(&self, where_clause: &Option<super::parser::Expr>, row: &Row, table: &Table) -> DbResult<bool> {
        match where_clause {
            Some(expr) => {
                let value = self.eval_expr_with_row(expr, row, table)?;
                match value {
                    Value::Boolean(b) => Ok(b),
                    _ => Err(DbError::TypeMismatch("WHERE clause must evaluate to boolean".into())),
                }
            }
            None => Ok(true),
        }
    }

    fn project_columns(&self, columns: &Vec<super::parser::SelectItem>, row: &Row, table: &Table) -> DbResult<Row> {
        let mut result = Vec::new();

        for item in columns {
            match item {
                super::parser::SelectItem::Wildcard => {
                    result.extend(row.clone());
                }
                super::parser::SelectItem::Expr(expr, _alias) => {
                    let value = self.eval_expr_with_row(expr, row, table)?;
                    result.push(value);
                }
            }
        }

        Ok(result)
    }

    fn get_column_names(&self, columns: &Vec<super::parser::SelectItem>, table: &Table) -> Vec<String> {
        let mut names = Vec::new();

        for item in columns {
            match item {
                super::parser::SelectItem::Wildcard => {
                    for col in &table.columns {
                        names.push(col.name.clone());
                    }
                }
                super::parser::SelectItem::Expr(_expr, alias) => {
                    if let Some(alias) = alias {
                        names.push(alias.clone());
                    } else {
                        names.push("expr".to_string()); // TODO: Get expression name
                    }
                }
            }
        }

        names
    }

    fn eval_expr(&self, expr: &super::parser::Expr) -> DbResult<Value> {
        // Simplified expression evaluation (without row context)
        match expr {
            super::parser::Expr::Literal(value) => Ok(value.clone()),
            super::parser::Expr::Column(_) => Err(DbError::Query("Column reference without row context".into())),
            _ => Err(DbError::Query("Complex expressions not yet supported".into())),
        }
    }

    fn eval_expr_with_row(&self, expr: &super::parser::Expr, row: &Row, table: &Table) -> DbResult<Value> {
        match expr {
            super::parser::Expr::Literal(value) => Ok(value.clone()),
            super::parser::Expr::Column(col_ref) => {
                let col_name = &col_ref.name;
                let idx = table.get_column_index(col_name)
                    .ok_or_else(|| DbError::NotFound(format!("Column {}", col_name)))?;
                Ok(row[idx].clone())
            }
            super::parser::Expr::BinaryOp(left, op, right) => {
                let left_val = self.eval_expr_with_row(left, row, table)?;
                let right_val = self.eval_expr_with_row(right, row, table)?;
                self.eval_binary_op(&left_val, *op, &right_val)
            }
            _ => Err(DbError::Query("Expression type not yet supported".into())),
        }
    }

    fn eval_binary_op(&self, left: &Value, op: super::parser::BinaryOperator, right: &Value) -> DbResult<Value> {
        match op {
            super::parser::BinaryOperator::Comparison(comp_op) => {
                self.eval_comparison(left, comp_op, right)
            }
            super::parser::BinaryOperator::Arithmetic(arith_op) => {
                self.eval_arithmetic(left, arith_op, right)
            }
            super::parser::BinaryOperator::And => {
                match (left, right) {
                    (Value::Boolean(l), Value::Boolean(r)) => Ok(Value::Boolean(*l && *r)),
                    _ => Err(DbError::TypeMismatch("AND requires boolean operands".into())),
                }
            }
            super::parser::BinaryOperator::Or => {
                match (left, right) {
                    (Value::Boolean(l), Value::Boolean(r)) => Ok(Value::Boolean(*l || *r)),
                    _ => Err(DbError::TypeMismatch("OR requires boolean operands".into())),
                }
            }
            _ => Err(DbError::Query("Binary operator not yet supported".into())),
        }
    }

    fn eval_comparison(&self, left: &Value, op: super::types::ComparisonOp, right: &Value) -> DbResult<Value> {
        use super::types::ComparisonOp;

        let result = match op {
            ComparisonOp::Equal => left == right,
            ComparisonOp::NotEqual => left != right,
            ComparisonOp::Less => self.compare_values(left, right)? == core::cmp::Ordering::Less,
            ComparisonOp::LessEqual => self.compare_values(left, right)? != core::cmp::Ordering::Greater,
            ComparisonOp::Greater => self.compare_values(left, right)? == core::cmp::Ordering::Greater,
            ComparisonOp::GreaterEqual => self.compare_values(left, right)? != core::cmp::Ordering::Less,
            _ => return Err(DbError::Query("Comparison operator not yet supported".into())),
        };

        Ok(Value::Boolean(result))
    }

    fn compare_values(&self, left: &Value, right: &Value) -> DbResult<core::cmp::Ordering> {
        use core::cmp::Ordering;

        match (left, right) {
            (Value::Int8(l), Value::Int8(r)) => Ok(l.cmp(r)),
            (Value::Int16(l), Value::Int16(r)) => Ok(l.cmp(r)),
            (Value::Int32(l), Value::Int32(r)) => Ok(l.cmp(r)),
            (Value::Int64(l), Value::Int64(r)) => Ok(l.cmp(r)),
            (Value::Float32(l), Value::Float32(r)) => l.partial_cmp(r).ok_or_else(|| DbError::Query("NaN comparison".into())),
            (Value::Float64(l), Value::Float64(r)) => l.partial_cmp(r).ok_or_else(|| DbError::Query("NaN comparison".into())),
            (Value::Text(l), Value::Text(r)) => Ok(l.cmp(r)),
            _ => Err(DbError::TypeMismatch("Cannot compare different types".into())),
        }
    }

    fn eval_arithmetic(&self, left: &Value, op: super::types::ArithmeticOp, right: &Value) -> DbResult<Value> {
        use super::types::ArithmeticOp;

        match (left, right, op) {
            (Value::Int32(l), Value::Int32(r), ArithmeticOp::Add) => Ok(Value::Int32(l + r)),
            (Value::Int32(l), Value::Int32(r), ArithmeticOp::Subtract) => Ok(Value::Int32(l - r)),
            (Value::Int32(l), Value::Int32(r), ArithmeticOp::Multiply) => Ok(Value::Int32(l * r)),
            (Value::Int32(l), Value::Int32(r), ArithmeticOp::Divide) => Ok(Value::Int32(l / r)),
            (Value::Int64(l), Value::Int64(r), ArithmeticOp::Add) => Ok(Value::Int64(l + r)),
            (Value::Int64(l), Value::Int64(r), ArithmeticOp::Subtract) => Ok(Value::Int64(l - r)),
            (Value::Float64(l), Value::Float64(r), ArithmeticOp::Add) => Ok(Value::Float64(l + r)),
            (Value::Float64(l), Value::Float64(r), ArithmeticOp::Subtract) => Ok(Value::Float64(l - r)),
            (Value::Float64(l), Value::Float64(r), ArithmeticOp::Multiply) => Ok(Value::Float64(l * r)),
            (Value::Float64(l), Value::Float64(r), ArithmeticOp::Divide) => Ok(Value::Float64(l / r)),
            _ => Err(DbError::TypeMismatch("Invalid arithmetic operation".into())),
        }
    }
}

/// Execution result
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    Query {
        columns: Vec<String>,
        rows: Vec<Row>,
        affected_rows: usize,
    },
    Modification {
        affected_rows: usize,
        last_id: Option<u64>,
    },
    Schema {
        message: String,
    },
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_table() {
        let executor = Executor::new();
        let sql = "CREATE TABLE users (id INT, name TEXT)";
        let ast = super::super::parser::parse_sql(sql).unwrap();
        assert!(executor.execute(ast).is_ok());
    }

    #[test]
    fn test_insert_select() {
        let executor = Executor::new();

        // Create table
        let create_sql = "CREATE TABLE users (id INT, name TEXT)";
        let create_ast = super::super::parser::parse_sql(create_sql).unwrap();
        executor.execute(create_ast).unwrap();

        // Insert data
        let insert_sql = "INSERT INTO users VALUES (1, 'Alice')";
        let insert_ast = super::super::parser::parse_sql(insert_sql).unwrap();
        executor.execute(insert_ast).unwrap();

        // Select data
        let select_sql = "SELECT * FROM users";
        let select_ast = super::super::parser::parse_sql(select_sql).unwrap();
        let result = executor.execute(select_ast).unwrap();

        match result {
            ExecutionResult::Query { rows, .. } => {
                assert_eq!(rows.len(), 1);
            }
            _ => panic!("Expected query result"),
        }
    }
}
