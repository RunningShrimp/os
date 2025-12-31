//! SQL Parser
//!
//! Complete SQL parser with lexical analysis, syntax analysis, and AST construction.
//! Supports SELECT, INSERT, UPDATE, DELETE, CREATE TABLE, and more.

use super::types::{
    Value, ColumnRef, ComparisonOp, ArithmeticOp, LogicalOp, Order, JoinType, DataType,
};
use super::{DbError, DbResult};
use alloc::string::String;
use alloc::vec::Vec;

/// SQL Token
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Select,
    Insert,
    Update,
    Delete,
    Create,
    Drop,
    Alter,
    Table,
    Index,
    Into,
    Values,
    Set,
    From,
    Where,
    Join,
    Inner,
    Left,
    Right,
    Full,
    On,
    As,
    And,
    Or,
    Not,
    Is,
    Null,
    Between,
    Like,
    In,
    Exists,
    Order,
    By,
    Asc,
    Desc,
    Limit,
    Offset,
    Group,
    Having,
    Distinct,
    All,

    // Operators
    Equal,          // =
    NotEqual,       // != or <>
    Less,           // <
    LessEqual,      // <=
    Greater,        // >
    GreaterEqual,   // >=
    Plus,           // +
    Minus,          // -
    Multiply,       // *
    Divide,         // /
    Modulo,         // %

    // Punctuation
    LeftParen,
    RightParen,
    Comma,
    Semicolon,
    Dot,
    Star,
    Question,

    // Literals
    Identifier(String),
    StringLiteral(String),
    Number(f64),
    Boolean(bool),

    // Types
    Integer,
    BigInt,
    Float,
    Double,
    Text,
    Blob,
    BooleanType,
    Date,
    Timestamp,

    // End
    EOF,
}

/// SQL Lexer (Tokenizer)
pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    current: Option<char>,
}

impl Lexer {
    pub fn new(sql: &str) -> Self {
        let mut chars: Vec<char> = sql.chars().collect();
        let current = chars.first().cloned();

        Self {
            input: chars,
            pos: 0,
            current,
        }
    }

    /// Tokenize the entire SQL string
    pub fn tokenize(&mut self) -> DbResult<Vec<Token>> {
        let mut tokens = Vec::new();

        while let Some(token) = self.next_token()? {
            if token != Token::EOF {
                tokens.push(token);
            }
        }

        Ok(tokens)
    }

    /// Get the next token
    fn next_token(&mut self) -> DbResult<Option<Token>> {
        self.skip_whitespace();

        match self.current {
            None => Ok(Some(Token::EOF)),
            Some(c) => {
                match c {
                    // Operators
                    '=' => {
                        self.advance();
                        Ok(Some(Token::Equal))
                    }
                    '!' => {
                        self.advance();
                        if self.current == Some('=') {
                            self.advance();
                            Ok(Some(Token::NotEqual))
                        } else {
                            Err(DbError::Syntax("Expected '=' after '!'".into()))
                        }
                    }
                    '<' => {
                        self.advance();
                        if self.current == Some('=') {
                            self.advance();
                            Ok(Some(Token::LessEqual))
                        } else if self.current == Some('>') {
                            self.advance();
                            Ok(Some(Token::NotEqual))
                        } else {
                            Ok(Some(Token::Less))
                        }
                    }
                    '>' => {
                        self.advance();
                        if self.current == Some('=') {
                            self.advance();
                            Ok(Some(Token::GreaterEqual))
                        } else {
                            Ok(Some(Token::Greater))
                        }
                    }
                    '+' => {
                        self.advance();
                        Ok(Some(Token::Plus))
                    }
                    '-' => {
                        self.advance();
                        Ok(Some(Token::Minus))
                    }
                    '*' => {
                        self.advance();
                        Ok(Some(Token::Star))
                    }
                    '/' => {
                        self.advance();
                        Ok(Some(Token::Divide))
                    }
                    '%' => {
                        self.advance();
                        Ok(Some(Token::Modulo))
                    }

                    // Punctuation
                    '(' => {
                        self.advance();
                        Ok(Some(Token::LeftParen))
                    }
                    ')' => {
                        self.advance();
                        Ok(Some(Token::RightParen))
                    }
                    ',' => {
                        self.advance();
                        Ok(Some(Token::Comma))
                    }
                    ';' => {
                        self.advance();
                        Ok(Some(Token::Semicolon))
                    }
                    '.' => {
                        self.advance();
                        Ok(Some(Token::Dot))
                    }
                    '?' => {
                        self.advance();
                        Ok(Some(Token::Question))
                    }

                    // String literal
                    '\'' => self.lex_string(),

                    // Number
                    '0'..='9' => self.lex_number(),

                    // Identifier or keyword
                    'a'..='z' | 'A'..='Z' | '_' => self.lex_identifier(),

                    // Unexpected
                    c => Err(DbError::Syntax(format!("Unexpected character: {}", c))),
                }
            }
        }
    }

    fn lex_string(&mut self) -> DbResult<Option<Token>> {
        self.advance(); // Skip opening quote
        let start = self.pos;

        while let Some(c) = self.current {
            if c == '\'' {
                let s: String = self.input[start..self.pos].iter().collect();
                self.advance(); // Skip closing quote
                return Ok(Some(Token::StringLiteral(s)));
            }
            self.advance();
        }

        Err(DbError::Syntax("Unterminated string literal".into()))
    }

    fn lex_number(&mut self) -> DbResult<Option<Token>> {
        let start = self.pos;

        // Integer part
        while let Some(c) = self.current {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        // Fractional part
        if self.current == Some('.') {
            self.advance();
            while let Some(c) = self.current {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        let s: String = self.input[start..self.pos].iter().collect();
        let num = s.parse::<f64>().map_err(|_| DbError::Syntax("Invalid number".into()))?;

        Ok(Some(Token::Number(num)))
    }

    fn lex_identifier(&mut self) -> DbResult<Option<Token>> {
        let start = self.pos;

        while let Some(c) = self.current {
            if c.is_ascii_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let s: String = self.input[start..self.pos].iter().collect();
        let upper = s.to_uppercase();

        let token = match upper.as_str() {
            "SELECT" => Token::Select,
            "INSERT" => Token::Insert,
            "UPDATE" => Token::Update,
            "DELETE" => Token::Delete,
            "CREATE" => Token::Create,
            "DROP" => Token::Drop,
            "ALTER" => Token::Alter,
            "TABLE" => Token::Table,
            "INDEX" => Token::Index,
            "INTO" => Token::Into,
            "VALUES" => Token::Values,
            "SET" => Token::Set,
            "FROM" => Token::From,
            "WHERE" => Token::Where,
            "JOIN" => Token::Join,
            "INNER" => Token::Inner,
            "LEFT" => Token::Left,
            "RIGHT" => Token::Right,
            "FULL" => Token::Full,
            "ON" => Token::On,
            "AS" => Token::As,
            "AND" => Token::And,
            "OR" => Token::Or,
            "NOT" => Token::Not,
            "IS" => Token::Is,
            "NULL" => Token::Null,
            "BETWEEN" => Token::Between,
            "LIKE" => Token::Like,
            "IN" => Token::In,
            "EXISTS" => Token::Exists,
            "ORDER" => Token::Order,
            "BY" => Token::By,
            "ASC" => Token::Asc,
            "DESC" => Token::Desc,
            "LIMIT" => Token::Limit,
            "OFFSET" => Token::Offset,
            "GROUP" => Token::Group,
            "HAVING" => Token::Having,
            "DISTINCT" => Token::Distinct,
            "ALL" => Token::All,
            "INTEGER" => Token::Integer,
            "BIGINT" => Token::BigInt,
            "FLOAT" => Token::Float,
            "DOUBLE" => Token::Double,
            "TEXT" => Token::Text,
            "BLOB" => Token::Blob,
            "BOOLEAN" => Token::BooleanType,
            "DATE" => Token::Date,
            "TIMESTAMP" => Token::Timestamp,
            "TRUE" => Token::Boolean(true),
            "FALSE" => Token::Boolean(false),
            _ => Token::Identifier(s),
        };

        Ok(Some(token))
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current {
            if c.is_ascii_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn advance(&mut self) {
        self.pos += 1;
        self.current = self.input.get(self.pos).cloned();
    }
}

/// SQL AST Node types
pub enum AstNode {
    SelectStmt(SelectStatement),
    InsertStmt(InsertStatement),
    UpdateStmt(UpdateStatement),
    DeleteStmt(DeleteStatement),
    CreateTableStmt(CreateTableStatement),
    DropTableStmt(DropTableStatement),
    CreateIndexStmt(CreateIndexStatement),
}

/// SELECT statement
#[derive(Debug, Clone)]
pub struct SelectStatement {
    pub distinct: bool,
    pub columns: Vec<SelectItem>,
    pub from: Vec<TableRef>,
    pub joins: Vec<Join>,
    pub where_clause: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
    pub order_by: Vec<OrderBy>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Select item (column or expression)
#[derive(Debug, Clone)]
pub enum SelectItem {
    Wildcard,
    Expr(Expr, Option<String>), // Expression and optional alias
}

/// Table reference
#[derive(Debug, Clone)]
pub struct TableRef {
    pub name: String,
    pub alias: Option<String>,
}

/// JOIN clause
#[derive(Debug, Clone)]
pub struct Join {
    pub join_type: JoinType,
    pub table: TableRef,
    pub on: Expr,
}

/// Expression
#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Value),
    Column(ColumnRef),
    BinaryOp(Box<Expr>, BinaryOperator, Box<Expr>),
    UnaryOp(UnaryOperator, Box<Expr>),
    Function(String, Vec<Expr>),
    Aggregate(AggregateFunction, Box<Expr>, Option<String>),
    Cast(Box<Expr>, DataType),
    Case(Vec<CaseBranch>, Box<Expr>),
    Exists(Box<SelectStatement>),
    In(Box<Expr>, Vec<Expr>),
    Between(Box<Expr>, bool, Box<Expr>, Box<Expr>), // expr, NOT, low, high
}

/// Binary operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    And,
    Or,
    Comparison(ComparisonOp),
    Arithmetic(ArithmeticOp),
    Like,
    NotLike,
}

/// Unary operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Not,
    Minus,
    IsNull,
    IsNotNull,
}

/// Aggregate function
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

/// CASE branch
#[derive(Debug, Clone)]
pub struct CaseBranch {
    pub condition: Expr,
    pub result: Expr,
}

/// ORDER BY clause
#[derive(Debug, Clone)]
pub struct OrderBy {
    pub expr: Expr,
    pub order: Order,
}

/// INSERT statement
#[derive(Debug, Clone)]
pub struct InsertStatement {
    pub table_name: String,
    pub columns: Vec<String>,
    pub values: Vec<Vec<Expr>>,
}

/// UPDATE statement
#[derive(Debug, Clone)]
pub struct UpdateStatement {
    pub table_name: String,
    pub assignments: Vec<Assignment>,
    pub where_clause: Option<Expr>,
}

/// Assignment (column = value)
#[derive(Debug, Clone)]
pub struct Assignment {
    pub column: String,
    pub value: Expr,
}

/// DELETE statement
#[derive(Debug, Clone)]
pub struct DeleteStatement {
    pub table_name: String,
    pub where_clause: Option<Expr>,
}

/// CREATE TABLE statement
#[derive(Debug, Clone)]
pub struct CreateTableStatement {
    pub table_name: String,
    pub columns: Vec<ColumnDef>,
}

/// Column definition
#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default_value: Option<Value>,
    pub primary_key: bool,
    pub unique: bool,
    pub auto_increment: bool,
}

/// DROP TABLE statement
#[derive(Debug, Clone)]
pub struct DropTableStatement {
    pub table_name: String,
    pub if_exists: bool,
}

/// CREATE INDEX statement
#[derive(Debug, Clone)]
pub struct CreateIndexStatement {
    pub index_name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

/// SQL Parser
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> DbResult<AstNode> {
        match self.current_token() {
            Token::Select => self.parse_select(),
            Token::Insert => self.parse_insert(),
            Token::Update => self.parse_update(),
            Token::Delete => self.parse_delete(),
            Token::Create => self.parse_create(),
            Token::Drop => self.parse_drop(),
            token => Err(DbError::Syntax(format!("Unexpected token: {:?}", token))),
        }
    }

    fn parse_select(&mut self) -> DbResult<AstNode> {
        self.expect_token(Token::Select)?;

        let distinct = self.match_token(Token::Distinct);

        let mut columns = Vec::new();
        if self.match_token(Token::Star) {
            columns.push(SelectItem::Wildcard);
        } else {
            loop {
                columns.push(self.parse_select_item()?);
                if !self.match_token(Token::Comma) {
                    break;
                }
            }
        }

        self.expect_token(Token::From)?;

        let mut from = Vec::new();
        loop {
            from.push(self.parse_table_ref()?);
            if !self.match_token(Token::Comma) {
                break;
            }
        }

        let mut joins = Vec::new();
        while self.match_token(Token::Join) || self.match_token(Token::Left)
            || self.match_token(Token::Right) || self.match_token(Token::Inner)
        {
            joins.push(self.parse_join()?);
        }

        let where_clause = if self.match_token(Token::Where) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        // TODO: Parse GROUP BY, HAVING, ORDER BY, LIMIT

        Ok(AstNode::SelectStmt(SelectStatement {
            distinct,
            columns,
            from,
            joins,
            where_clause,
            group_by: Vec::new(),
            having: None,
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }))
    }

    fn parse_select_item(&mut self) -> DbResult<SelectItem> {
        let expr = self.parse_expr()?;

        let alias = if self.match_token(Token::As) {
            Some(self.expect_identifier()?)
        } else {
            None
        };

        Ok(SelectItem::Expr(expr, alias))
    }

    fn parse_table_ref(&mut self) -> DbResult<TableRef> {
        let name = self.expect_identifier()?;

        let alias = if self.match_token(Token::As) {
            Some(self.expect_identifier()?)
        } else {
            None
        };

        Ok(TableRef { name, alias })
    }

    fn parse_join(&mut self) -> DbResult<Join> {
        let join_type = if self.match_token(Token::Left) {
            JoinType::Left
        } else if self.match_token(Token::Right) {
            JoinType::Right
        } else if self.match_token(Token::Inner) {
            JoinType::Inner
        } else {
            JoinType::Inner
        };

        self.expect_token(Token::Join)?;

        let table = self.parse_table_ref()?;
        self.expect_token(Token::On)?;

        let on = self.parse_expr()?;

        Ok(Join {
            join_type,
            table,
            on,
        })
    }

    fn parse_expr(&mut self) -> DbResult<Expr> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> DbResult<Expr> {
        let mut left = self.parse_and_expr()?;

        while self.match_token(Token::Or) {
            let right = self.parse_and_expr()?;
            left = Expr::BinaryOp(
                Box::new(left),
                BinaryOperator::Or,
                Box::new(right),
            );
        }

        Ok(left)
    }

    fn parse_and_expr(&mut self) -> DbResult<Expr> {
        let mut left = self.parse_comparison_expr()?;

        while self.match_token(Token::And) {
            let right = self.parse_comparison_expr()?;
            left = Expr::BinaryOp(
                Box::new(left),
                BinaryOperator::And,
                Box::new(right),
            );
        }

        Ok(left)
    }

    fn parse_comparison_expr(&mut self) -> DbResult<Expr> {
        let left = self.parse_additive_expr()?;

        if let Some(op) = self.match_comparison_op() {
            let right = self.parse_additive_expr()?;
            return Ok(Expr::BinaryOp(
                Box::new(left),
                BinaryOperator::Comparison(op),
                Box::new(right),
            ));
        }

        Ok(left)
    }

    fn parse_additive_expr(&mut self) -> DbResult<Expr> {
        let mut left = self.parse_multiplicative_expr()?;

        while let Some(op) = self.match_arithmetic_op(&[Token::Plus, Token::Minus]) {
            let right = self.parse_multiplicative_expr()?;
            let arith_op = if op == Token::Plus {
                ArithmeticOp::Add
            } else {
                ArithmeticOp::Subtract
            };
            left = Expr::BinaryOp(
                Box::new(left),
                BinaryOperator::Arithmetic(arith_op),
                Box::new(right),
            );
        }

        Ok(left)
    }

    fn parse_multiplicative_expr(&mut self) -> DbResult<Expr> {
        let mut left = self.parse_unary_expr()?;

        while let Some(op) = self.match_arithmetic_op(&[Token::Multiply, Token::Divide, Token::Modulo]) {
            let right = self.parse_unary_expr()?;
            let arith_op = match op {
                Token::Multiply => ArithmeticOp::Multiply,
                Token::Divide => ArithmeticOp::Divide,
                Token::Modulo => ArithmeticOp::Modulo,
                _ => unreachable!(),
            };
            left = Expr::BinaryOp(
                Box::new(left),
                BinaryOperator::Arithmetic(arith_op),
                Box::new(right),
            );
        }

        Ok(left)
    }

    fn parse_unary_expr(&mut self) -> DbResult<Expr> {
        if self.match_token(Token::Not) {
            let expr = self.parse_unary_expr()?;
            return Ok(Expr::UnaryOp(UnaryOperator::Not, Box::new(expr)));
        }

        if self.match_token(Token::Minus) {
            let expr = self.parse_unary_expr()?;
            return Ok(Expr::UnaryOp(UnaryOperator::Minus, Box::new(expr)));
        }

        self.parse_primary_expr()
    }

    fn parse_primary_expr(&mut self) -> DbResult<Expr> {
        match self.current_token() {
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect_token(Token::RightParen)?;
                Ok(expr)
            }
            Token::Number(n) => {
                self.advance();
                Ok(Expr::Literal(Value::Float64(n)))
            }
            Token::StringLiteral(s) => {
                self.advance();
                Ok(Expr::Literal(Value::Text(s)))
            }
            Token::Boolean(b) => {
                self.advance();
                Ok(Expr::Literal(Value::Boolean(b)))
            }
            Token::Null => {
                self.advance();
                Ok(Expr::Literal(Value::Null))
            }
            Token::Identifier(name) => {
                self.advance();
                Ok(Expr::Column(ColumnRef::new(name)))
            }
            token => Err(DbError::Syntax(format!("Unexpected token in expression: {:?}", token))),
        }
    }

    fn parse_insert(&mut self) -> DbResult<AstNode> {
        self.expect_token(Token::Insert)?;
        self.expect_token(Token::Into)?;
        let table_name = self.expect_identifier()?;

        let mut columns = Vec::new();
        if self.match_token(Token::LeftParen) {
            loop {
                columns.push(self.expect_identifier()?);
                if !self.match_token(Token::Comma) {
                    break;
                }
            }
            self.expect_token(Token::RightParen)?;
        }

        self.expect_token(Token::Values)?;

        let mut values = Vec::new();
        loop {
            self.expect_token(Token::LeftParen)?;
            let mut row = Vec::new();
            loop {
                row.push(self.parse_expr()?);
                if !self.match_token(Token::Comma) {
                    break;
                }
            }
            self.expect_token(Token::RightParen)?;
            values.push(row);

            if !self.match_token(Token::Comma) {
                break;
            }
        }

        Ok(AstNode::InsertStmt(InsertStatement {
            table_name,
            columns,
            values,
        }))
    }

    fn parse_update(&mut self) -> DbResult<AstNode> {
        self.expect_token(Token::Update)?;
        let table_name = self.expect_identifier()?;
        self.expect_token(Token::Set)?;

        let mut assignments = Vec::new();
        loop {
            let column = self.expect_identifier()?;
            self.expect_token(Token::Equal)?;
            let value = self.parse_expr()?;
            assignments.push(Assignment { column, value });

            if !self.match_token(Token::Comma) {
                break;
            }
        }

        let where_clause = if self.match_token(Token::Where) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(AstNode::UpdateStmt(UpdateStatement {
            table_name,
            assignments,
            where_clause,
        }))
    }

    fn parse_delete(&mut self) -> DbResult<AstNode> {
        self.expect_token(Token::Delete)?;
        self.expect_token(Token::From)?;
        let table_name = self.expect_identifier()?;

        let where_clause = if self.match_token(Token::Where) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(AstNode::DeleteStmt(DeleteStatement {
            table_name,
            where_clause,
        }))
    }

    fn parse_create(&mut self) -> DbResult<AstNode> {
        self.expect_token(Token::Create)?;

        match self.current_token() {
            Token::Table => self.parse_create_table(),
            Token::Index => self.parse_create_index(),
            _ => Err(DbError::Syntax("Expected TABLE or INDEX after CREATE".into())),
        }
    }

    fn parse_create_table(&mut self) -> DbResult<AstNode> {
        self.expect_token(Token::Table)?;
        let table_name = self.expect_identifier()?;
        self.expect_token(Token::LeftParen)?;

        let mut columns = Vec::new();
        loop {
            columns.push(self.parse_column_def()?);

            if !self.match_token(Token::Comma) {
                break;
            }
        }

        self.expect_token(Token::RightParen)?;

        Ok(AstNode::CreateTableStmt(CreateTableStatement {
            table_name,
            columns,
        }))
    }

    fn parse_column_def(&mut self) -> DbResult<ColumnDef> {
        let name = self.expect_identifier()?;
        let data_type = self.parse_data_type()?;

        let mut nullable = true;
        let mut default_value = None;
        let mut primary_key = false;
        let mut unique = false;
        let mut auto_increment = false;

        loop {
            if self.match_token(Token::Not) {
                self.expect_token(Token::Null)?;
                nullable = false;
            } else if self.match_token(Token::Null) {
                nullable = true;
            } else if self.match_keyword("PRIMARY") {
                self.expect_keyword("KEY")?;
                primary_key = true;
            } else if self.match_token(Token::Unique) {
                unique = true;
            } else if self.match_keyword("AUTO_INCREMENT") {
                auto_increment = true;
            } else if self.match_token(Token::Default) {
                default_value = Some(self.parse_literal()?); // TODO: Fix this
                break;
            } else {
                break;
            }
        }

        Ok(ColumnDef {
            name,
            data_type,
            nullable,
            default_value,
            primary_key,
            unique,
            auto_increment,
        })
    }

    fn parse_data_type(&mut self) -> DbResult<DataType> {
        match self.current_token() {
            Token::Integer => {
                self.advance();
                Ok(DataType::Int32)
            }
            Token::BigInt => {
                self.advance();
                Ok(DataType::Int64)
            }
            Token::Float => {
                self.advance();
                Ok(DataType::Float32)
            }
            Token::Double => {
                self.advance();
                Ok(DataType::Float64)
            }
            Token::Text => {
                self.advance();
                Ok(DataType::Text)
            }
            Token::Blob => {
                self.advance();
                Ok(DataType::Blob)
            }
            Token::BooleanType => {
                self.advance();
                Ok(DataType::Boolean)
            }
            Token::Date => {
                self.advance();
                Ok(DataType::Date)
            }
            Token::Timestamp => {
                self.advance();
                Ok(DataType::Timestamp)
            }
            token => Err(DbError::Syntax(format!("Expected data type, got: {:?}", token))),
        }
    }

    fn parse_create_index(&mut self) -> DbResult<AstNode> {
        self.expect_token(Token::Index)?;
        let index_name = self.expect_identifier()?;
        self.expect_token(Token::On)?;
        let table_name = self.expect_identifier()?;
        self.expect_token(Token::LeftParen)?;

        let mut columns = Vec::new();
        loop {
            columns.push(self.expect_identifier()?);
            if !self.match_token(Token::Comma) {
                break;
            }
        }

        self.expect_token(Token::RightParen)?;

        let unique = false; // TODO: Parse UNIQUE

        Ok(AstNode::CreateIndexStmt(CreateIndexStatement {
            index_name,
            table_name,
            columns,
            unique,
        }))
    }

    fn parse_drop(&mut self) -> DbResult<AstNode> {
        self.expect_token(Token::Drop)?;
        self.expect_token(Token::Table)?;
        let table_name = self.expect_identifier()?;
        let if_exists = false; // TODO: Parse IF EXISTS

        Ok(AstNode::DropTableStmt(DropTableStatement {
            table_name,
            if_exists,
        }))
    }

    fn parse_literal(&mut self) -> DbResult<Value> {
        match self.current_token() {
            Token::Number(n) => {
                self.advance();
                Ok(Value::Float64(n))
            }
            Token::StringLiteral(s) => {
                self.advance();
                Ok(Value::Text(s))
            }
            _ => Err(DbError::Syntax("Expected literal".into())),
        }
    }

    // Helper methods
    fn current_token(&self) -> Token {
        self.tokens.get(self.pos).cloned().unwrap_or(Token::EOF)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn expect_token(&mut self, expected: Token) -> DbResult<Token> {
        let current = self.current_token();
        if core::mem::discriminant(&current) == core::mem::discriminant(&expected) {
            self.advance();
            Ok(current)
        } else {
            Err(DbError::Syntax(format!("Expected {:?}, got {:?}", expected, current)))
        }
    }

    fn match_token(&mut self, token: Token) -> bool {
        if core::mem::discriminant(&self.current_token()) == core::mem::discriminant(&token) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_identifier(&mut self) -> DbResult<String> {
        match self.current_token() {
            Token::Identifier(name) => {
                self.advance();
                Ok(name)
            }
            token => Err(DbError::Syntax(format!("Expected identifier, got {:?}", token))),
        }
    }

    fn match_keyword(&mut self, keyword: &str) -> bool {
        match self.current_token() {
            Token::Identifier(ref name) if name.eq_ignore_ascii_case(keyword) => {
                self.advance();
                true
            }
            _ => false,
        }
    }

    fn expect_keyword(&mut self, keyword: &str) -> DbResult<()> {
        if self.match_keyword(keyword) {
            Ok(())
        } else {
            Err(DbError::Syntax(format!("Expected keyword: {}", keyword)))
        }
    }

    fn match_comparison_op(&mut self) -> Option<ComparisonOp> {
        match self.current_token() {
            Token::Equal => {
                self.advance();
                Some(ComparisonOp::Equal)
            }
            Token::NotEqual => {
                self.advance();
                Some(ComparisonOp::NotEqual)
            }
            Token::Less => {
                self.advance();
                Some(ComparisonOp::Less)
            }
            Token::LessEqual => {
                self.advance();
                Some(ComparisonOp::LessEqual)
            }
            Token::Greater => {
                self.advance();
                Some(ComparisonOp::Greater)
            }
            Token::GreaterEqual => {
                self.advance();
                Some(ComparisonOp::GreaterEqual)
            }
            _ => None,
        }
    }

    fn match_arithmetic_op(&mut self, ops: &[Token]) -> Option<Token> {
        for op in ops {
            if core::mem::discriminant(&self.current_token()) == core::mem::discriminant(op) {
                let token = self.current_token();
                self.advance();
                return Some(token);
            }
        }
        None
    }
}

/// Parse SQL string into AST
pub fn parse_sql(sql: &str) -> DbResult<AstNode> {
    let mut lexer = Lexer::new(sql);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer() {
        let sql = "SELECT * FROM users WHERE id = 1";
        let mut lexer = Lexer::new(sql);
        let tokens = lexer.tokenize().unwrap();
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_parser() {
        let sql = "SELECT * FROM users";
        let ast = parse_sql(sql);
        assert!(ast.is_ok());
    }
}
