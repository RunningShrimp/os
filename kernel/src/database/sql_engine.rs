//! # SQL Engine
//!
//! Provides SQL parsing, query planning, and optimization capabilities.
//!
//! ## Components
//!
//! - **Lexer**: Tokenizes SQL statements
//! - **Parser**: Parses tokens into abstract syntax tree (AST)
//! - **Query Planner**: Creates logical and physical query plans
//! - **Query Optimizer**: Optimizes query plans using cost-based and rule-based approaches
//! - **Expression Evaluator**: Evaluates arithmetic, logical, and function expressions

#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use super::{DatabaseError, Result};

/// SQL token type
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Keywords
    Select,
    Insert,
    Update,
    Delete,
    Create,
    Drop,
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
    On,
    Group,
    By,
    Having,
    Order,
    Asc,
    Desc,
    Limit,
    Offset,

    // Operators
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    And,
    Or,
    Not,
    Like,
    In,
    Is,
    Null,

    // Literals
    Identifier(String),
    StringLiteral(String),
    Number(f64),
    Boolean(bool),

    // Punctuation
    Comma,
    Semicolon,
    LeftParen,
    RightParen,
    Dot,
    Star,

    // Special
    EOF,
    Whitespace,
}

/// SQL token with position information
#[derive(Debug, Clone)]
pub struct Token {
    /// Token type
    pub typ: TokenType,

    /// Line number
    pub line: usize,

    /// Column number
    pub column: usize,
}

impl Token {
    /// Create a new token
    pub fn new(typ: TokenType, line: usize, column: usize) -> Self {
        Self { typ, line, column }
    }
}

/// SQL lexer
pub struct SqlLexer {
    /// Input string
    input: Vec<char>,

    /// Current position
    pos: usize,

    /// Current line
    line: usize,

    /// Current column
    column: usize,
}

impl SqlLexer {
    /// Create a new lexer
    pub fn new(sql: &str) -> Self {
        Self {
            input: sql.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    /// Tokenize the input
    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        while self.pos < self.input.len() {
            let token = self.next_token()?;
            if token.typ != TokenType::Whitespace {
                tokens.push(token);
            }
        }

        tokens.push(Token::new(TokenType::EOF, self.line, self.column));
        Ok(tokens)
    }

    /// Get the next token
    fn next_token(&mut self) -> Result<Token> {
        if self.pos >= self.input.len() {
            return Ok(Token::new(TokenType::EOF, self.line, self.column));
        }

        let ch = self.input[self.pos];

        match ch {
            ' ' | '\t' | '\r' => {
                self.advance();
                Ok(Token::new(TokenType::Whitespace, self.line, self.column))
            }
            '\n' => {
                self.advance();
                self.line += 1;
                self.column = 1;
                Ok(Token::new(TokenType::Whitespace, self.line, self.column))
            }
            ',' => {
                self.advance();
                Ok(Token::new(TokenType::Comma, self.line, self.column))
            }
            ';' => {
                self.advance();
                Ok(Token::new(TokenType::Semicolon, self.line, self.column))
            }
            '(' => {
                self.advance();
                Ok(Token::new(TokenType::LeftParen, self.line, self.column))
            }
            ')' => {
                self.advance();
                Ok(Token::new(TokenType::RightParen, self.line, self.column))
            }
            '.' => {
                self.advance();
                Ok(Token::new(TokenType::Dot, self.line, self.column))
            }
            '*' => {
                self.advance();
                Ok(Token::new(TokenType::Star, self.line, self.column))
            }
            '\'' => self.string_literal(),
            '0'..='9' => self.number(),
            'a'..='z' | 'A'..='Z' | '_' => self.identifier_or_keyword(),
            '=' => {
                self.advance();
                Ok(Token::new(TokenType::Equal, self.line, self.column))
            }
            '!' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenType::NotEqual, self.line, self.column))
                } else {
                    Err(DatabaseError::SyntaxError(
                        String::from("Expected '=' after '!'")
                    ))
                }
            }
            '<' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenType::LessEqual, self.line, self.column))
                } else {
                    Ok(Token::new(TokenType::Less, self.line, self.column))
                }
            }
            '>' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenType::GreaterEqual, self.line, self.column))
                } else {
                    Ok(Token::new(TokenType::Greater, self.line, self.column))
                }
            }
            _ => Err(DatabaseError::SyntaxError(
                format!("Unexpected character: {}", ch)
            )),
        }
    }

    /// Advance to the next character
    fn advance(&mut self) {
        self.pos += 1;
        self.column += 1;
    }

    /// Peek at the next character without consuming it
    fn peek(&self) -> Option<char> {
        if self.pos < self.input.len() {
            Some(self.input[self.pos])
        } else {
            None
        }
    }

    /// Read a string literal
    fn string_literal(&mut self) -> Result<Token> {
        self.advance(); // Skip opening quote
        let start_line = self.line;
        let start_column = self.column;

        let mut value = String::new();
        while self.pos < self.input.len() {
            let ch = self.input[self.pos];
            if ch == '\'' {
                self.advance();
                return Ok(Token::new(TokenType::StringLiteral(value), start_line, start_column));
            } else {
                value.push(ch);
                self.advance();
            }
        }

        Err(DatabaseError::SyntaxError(String::from("Unterminated string literal")))
    }

    /// Read a number
    fn number(&mut self) -> Result<Token> {
        let start_line = self.line;
        let start_column = self.column;

        let mut num_str = String::new();
        while self.pos < self.input.len() {
            let ch = self.peek().unwrap();
            if ch.is_ascii_digit() || ch == '.' {
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let value = num_str.parse::<f64>().map_err(|_| {
            DatabaseError::SyntaxError(format!("Invalid number: {}", num_str))
        })?;

        Ok(Token::new(TokenType::Number(value), start_line, start_column))
    }

    /// Read an identifier or keyword
    fn identifier_or_keyword(&mut self) -> Result<Token> {
        let start_line = self.line;
        let start_column = self.column;

        let mut ident = String::new();
        while self.pos < self.input.len() {
            let ch = self.peek().unwrap();
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let typ = match ident.to_uppercase().as_str() {
            "SELECT" => TokenType::Select,
            "INSERT" => TokenType::Insert,
            "UPDATE" => TokenType::Update,
            "DELETE" => TokenType::Delete,
            "CREATE" => TokenType::Create,
            "DROP" => TokenType::Drop,
            "TABLE" => TokenType::Table,
            "INDEX" => TokenType::Index,
            "INTO" => TokenType::Into,
            "VALUES" => TokenType::Values,
            "SET" => TokenType::Set,
            "FROM" => TokenType::From,
            "WHERE" => TokenType::Where,
            "JOIN" => TokenType::Join,
            "INNER" => TokenType::Inner,
            "LEFT" => TokenType::Left,
            "RIGHT" => TokenType::Right,
            "ON" => TokenType::On,
            "GROUP" => TokenType::Group,
            "BY" => TokenType::By,
            "HAVING" => TokenType::Having,
            "ORDER" => TokenType::Order,
            "ASC" => TokenType::Asc,
            "DESC" => TokenType::Desc,
            "LIMIT" => TokenType::Limit,
            "OFFSET" => TokenType::Offset,
            "AND" => TokenType::And,
            "OR" => TokenType::Or,
            "NOT" => TokenType::Not,
            "NULL" => TokenType::Null,
            "IS" => TokenType::Is,
            "LIKE" => TokenType::Like,
            "IN" => TokenType::In,
            "TRUE" => TokenType::Boolean(true),
            "FALSE" => TokenType::Boolean(false),
            _ => TokenType::Identifier(ident),
        };

        Ok(Token::new(typ, start_line, start_column))
    }
}

/// SQL expression
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Column reference
    Column(String),

    /// Literal value
    Literal(Value),

    /// Binary operation
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Unary operation
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expression>,
    },

    /// Function call
    Function {
        name: String,
        args: Vec<Expression>,
    },

    /// Aggregate function
    Aggregate {
        func: AggregateFunction,
        arg: Box<Expression>,
        distinct: bool,
    },

    /// Subquery
    Subquery(Box<Query>),

    /// List of values (for IN clause)
    ValueList(Vec<Value>),

    /// NULL
    Null,
}

/// Binary operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    And,
    Or,
    Like,
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

/// SQL value
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
}

/// SQL query
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// SELECT query
    Select(Query),

    /// INSERT statement
    Insert {
        table_name: String,
        columns: Vec<String>,
        values: Vec<Vec<Expression>>,
    },

    /// UPDATE statement
    Update {
        table_name: String,
        assignments: Vec<(String, Expression)>,
        where_clause: Option<Expression>,
    },

    /// DELETE statement
    Delete {
        table_name: String,
        where_clause: Option<Expression>,
    },

    /// CREATE TABLE statement
    CreateTable {
        table_name: String,
        columns: Vec<ColumnDef>,
    },

    /// CREATE INDEX statement
    CreateIndex {
        index_name: String,
        table_name: String,
        columns: Vec<String>,
        unique: bool,
    },

    /// DROP TABLE statement
    DropTable {
        table_name: String,
    },

    /// DROP INDEX statement
    DropIndex {
        index_name: String,
    },
}

/// Column definition
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDef {
    /// Column name
    pub name: String,

    /// Column type
    pub typ: ColumnType,

    /// Whether column is nullable
    pub nullable: bool,

    /// Whether column is primary key
    pub primary_key: bool,

    /// Default value
    pub default: Option<Value>,
}

/// Column type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    Integer,
    Float,
    Text,
    Boolean,
    Blob,
}

/// SQL SELECT query
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    /// Selected columns
    pub columns: Vec<SelectItem>,

    /// FROM clause
    pub from: Vec<TableRef>,

    /// WHERE clause
    pub where_clause: Option<Expression>,

    /// GROUP BY clause
    pub group_by: Vec<Expression>,

    /// HAVING clause
    pub having: Option<Expression>,

    /// ORDER BY clause
    pub order_by: Vec<OrderBy>,

    /// LIMIT clause
    pub limit: Option<usize>,

    /// OFFSET clause
    pub offset: Option<usize>,
}

/// Select item (column or expression)
#[derive(Debug, Clone, PartialEq)]
pub enum SelectItem {
    /// Wildcard (*)
    Wildcard,

    /// Column reference
    Column(String),

    /// Expression with optional alias
    Expression(Expression, Option<String>),
}

/// Table reference
#[derive(Debug, Clone, PartialEq)]
pub enum TableRef {
    /// Simple table name
    Table(String),

    /// Table with alias
    AliasedTable(String, String),

    /// Join
    Join {
        left: Box<TableRef>,
        right: Box<TableRef>,
        join_type: JoinType,
        condition: Expression,
    },
}

/// Join type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
}

/// ORDER BY item
#[derive(Debug, Clone, PartialEq)]
pub struct OrderBy {
    /// Expression to order by
    pub expr: Expression,

    /// Ascending or descending
    pub ascending: bool,
}

/// SQL parser
pub struct SqlParser {
    /// Tokens
    tokens: Vec<Token>,

    /// Current position
    pos: usize,
}

impl SqlParser {
    /// Create a new parser
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
        }
    }

    /// Parse the tokens into a statement
    pub fn parse(&mut self) -> Result<Statement> {
        if self.pos >= self.tokens.len() {
            return Err(DatabaseError::SyntaxError(String::from("Empty statement")));
        }

        match self.current().typ {
            TokenType::Select => self.parse_select(),
            TokenType::Insert => self.parse_insert(),
            TokenType::Update => self.parse_update(),
            TokenType::Delete => self.parse_delete(),
            TokenType::Create => self.parse_create(),
            TokenType::Drop => self.parse_drop(),
            _ => Err(DatabaseError::SyntaxError(
                format!("Unexpected token: {:?}", self.current().typ)
            )),
        }
    }

    /// Get current token
    fn current(&self) -> &Token {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos]
        } else {
            &self.tokens[self.tokens.len() - 1]
        }
    }

    /// Advance to next token
    fn advance(&mut self) -> &Token {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        self.current()
    }

    /// Check if current token is of a specific type
    fn check(&self, typ: TokenType) -> bool {
        self.current().typ == typ
    }

    /// Consume token if it matches expected type
    fn consume(&mut self, typ: TokenType) -> Result<&Token> {
        if self.check(typ.clone()) {
            Ok(self.advance())
        } else {
            Err(DatabaseError::SyntaxError(
                format!("Expected {:?}, found {:?}", typ, self.current().typ)
            ))
        }
    }

    /// Parse SELECT statement
    fn parse_select(&mut self) -> Result<Statement> {
        self.consume(TokenType::Select)?;

        let columns = self.parse_select_list()?;
        self.consume(TokenType::From)?;
        let from = self.parse_from()?;

        let mut where_clause = None;
        if self.check(TokenType::Where) {
            self.advance();
            where_clause = Some(self.parse_expression()?);
        }

        let mut group_by = Vec::new();
        if self.check(TokenType::Group) {
            self.advance();
            self.consume(TokenType::By)?;
            group_by = self.parse_expression_list()?;
        }

        let mut having = None;
        if self.check(TokenType::Having) {
            self.advance();
            having = Some(self.parse_expression()?);
        }

        let mut order_by = Vec::new();
        if self.check(TokenType::Order) {
            self.advance();
            self.consume(TokenType::By)?;
            order_by = self.parse_order_by()?;
        }

        let mut limit = None;
        if self.check(TokenType::Limit) {
            self.advance();
            limit = Some(self.parse_number()?);
        }

        let mut offset = None;
        if self.check(TokenType::Offset) {
            self.advance();
            offset = Some(self.parse_number()?);
        }

        Ok(Statement::Select(Query {
            columns,
            from,
            where_clause,
            group_by,
            having,
            order_by,
            limit,
            offset,
        }))
    }

    /// Parse select list
    fn parse_select_list(&mut self) -> Result<Vec<SelectItem>> {
        let mut items = Vec::new();

        loop {
            if self.check(TokenType::Star) {
                self.advance();
                items.push(SelectItem::Wildcard);
            } else {
                let expr = self.parse_expression()?;
                let alias = if let TokenType::Identifier(name) = self.current().typ.clone() {
                    self.advance();
                    Some(name)
                } else {
                    None
                };
                items.push(SelectItem::Expression(expr, alias));
            }

            if !self.check(TokenType::Comma) {
                break;
            }
            self.advance();
        }

        Ok(items)
    }

    /// Parse FROM clause
    fn parse_from(&mut self) -> Result<Vec<TableRef>> {
        let mut tables = Vec::new();
        tables.push(self.parse_table_ref()?);

        while self.check(TokenType::Comma) {
            self.advance();
            tables.push(self.parse_table_ref()?);
        }

        Ok(tables)
    }

    /// Parse table reference
    fn parse_table_ref(&mut self) -> Result<TableRef> {
        let name = match self.current().typ.clone() {
            TokenType::Identifier(n) => {
                self.advance();
                n
            }
            _ => return Err(DatabaseError::SyntaxError(
                String::from("Expected table name")
            )),
        };

        // Check for join
        if self.check(TokenType::Join) || self.check(TokenType::Inner) || self.check(TokenType::Left) || self.check(TokenType::Right) {
            return self.parse_join(TableRef::Table(name));
        }

        // Check for alias
        if let TokenType::Identifier(alias) = self.current().typ.clone() {
            self.advance();
            Ok(TableRef::AliasedTable(name, alias))
        } else {
            Ok(TableRef::Table(name))
        }
    }

    /// Parse join
    fn parse_join(&mut self, left: TableRef) -> Result<TableRef> {
        let mut join_type = JoinType::Inner;

        if self.check(TokenType::Inner) {
            self.advance();
            self.consume(TokenType::Join)?;
        } else if self.check(TokenType::Left) {
            self.advance();
            self.consume(TokenType::Join)?;
            join_type = JoinType::Left;
        } else if self.check(TokenType::Right) {
            self.advance();
            self.consume(TokenType::Join)?;
            join_type = JoinType::Right;
        } else if self.check(TokenType::Join) {
            self.advance();
        } else {
            return Ok(left);
        }

        let right_name = match self.current().typ.clone() {
            TokenType::Identifier(n) => {
                self.advance();
                n
            }
            _ => return Err(DatabaseError::SyntaxError(
                String::from("Expected table name after JOIN")
            )),
        };

        let right = TableRef::Table(right_name);

        self.consume(TokenType::On)?;
        let condition = self.parse_expression()?;

        Ok(TableRef::Join {
            left: Box::new(left),
            right: Box::new(right),
            join_type,
            condition,
        })
    }

    /// Parse expression
    fn parse_expression(&mut self) -> Result<Expression> {
        self.parse_or()
    }

    /// Parse OR expression
    fn parse_or(&mut self) -> Result<Expression> {
        let mut left = self.parse_and()?;

        while self.check(TokenType::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expression::BinaryOp {
                op: BinaryOperator::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse AND expression
    fn parse_and(&mut self) -> Result<Expression> {
        let mut left = self.parse_comparison()?;

        while self.check(TokenType::And) {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expression::BinaryOp {
                op: BinaryOperator::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse comparison expression
    fn parse_comparison(&mut self) -> Result<Expression> {
        let left = self.parse_additive()?;

        let op = match self.current().typ {
            TokenType::Equal => { self.advance(); Some(BinaryOperator::Equal) }
            TokenType::NotEqual => { self.advance(); Some(BinaryOperator::NotEqual) }
            TokenType::Less => { self.advance(); Some(BinaryOperator::Less) }
            TokenType::Greater => { self.advance(); Some(BinaryOperator::Greater) }
            TokenType::LessEqual => { self.advance(); Some(BinaryOperator::LessEqual) }
            TokenType::GreaterEqual => { self.advance(); Some(BinaryOperator::GreaterEqual) }
            TokenType::Like => { self.advance(); Some(BinaryOperator::Like) }
            _ => None,
        };

        if let Some(op) = op {
            let right = self.parse_additive()?;
            Ok(Expression::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    /// Parse additive expression
    fn parse_additive(&mut self) -> Result<Expression> {
        let mut left = self.parse_multiplicative()?;

        while let TokenType::Identifier(ref op) = self.current().typ {
            match op.as_str() {
                "+" => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = Expression::BinaryOp {
                        op: BinaryOperator::Add,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                }
                "-" => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = Expression::BinaryOp {
                        op: BinaryOperator::Subtract,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }

        Ok(left)
    }

    /// Parse multiplicative expression
    fn parse_multiplicative(&mut self) -> Result<Expression> {
        let mut left = self.parse_unary()?;

        while let TokenType::Identifier(ref op) = self.current().typ {
            match op.as_str() {
                "*" => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = Expression::BinaryOp {
                        op: BinaryOperator::Multiply,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                }
                "/" => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = Expression::BinaryOp {
                        op: BinaryOperator::Divide,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                }
                "%" => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = Expression::BinaryOp {
                        op: BinaryOperator::Modulo,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }

        Ok(left)
    }

    /// Parse unary expression
    fn parse_unary(&mut self) -> Result<Expression> {
        if self.check(TokenType::Not) {
            self.advance();
            let operand = self.parse_unary()?;
            Ok(Expression::UnaryOp {
                op: UnaryOperator::Not,
                operand: Box::new(operand),
            })
        } else if let TokenType::Identifier(ref op) = self.current().typ {
            if op == "-" {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expression::UnaryOp {
                    op: UnaryOperator::Minus,
                    operand: Box::new(operand),
                })
            } else {
                self.parse_primary()
            }
        } else {
            self.parse_primary()
        }
    }

    /// Parse primary expression
    fn parse_primary(&mut self) -> Result<Expression> {
        match self.current().typ.clone() {
            TokenType::Identifier(name) => {
                self.advance();

                // Check if it's a function call
                if self.check(TokenType::LeftParen) {
                    self.advance();
                    let args = self.parse_expression_list()?;
                    self.consume(TokenType::RightParen)?;

                    Ok(Expression::Function { name, args })
                } else {
                    Ok(Expression::Column(name))
                }
            }
            TokenType::StringLiteral(s) => {
                self.advance();
                Ok(Expression::Literal(Value::String(s)))
            }
            TokenType::Number(n) => {
                self.advance();
                Ok(Expression::Literal(Value::Float(n)))
            }
            TokenType::Boolean(b) => {
                self.advance();
                Ok(Expression::Literal(Value::Boolean(b)))
            }
            TokenType::Null => {
                self.advance();
                Ok(Expression::Null)
            }
            TokenType::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.consume(TokenType::RightParen)?;
                Ok(expr)
            }
            _ => Err(DatabaseError::SyntaxError(
                format!("Unexpected token in expression: {:?}", self.current().typ)
            )),
        }
    }

    /// Parse expression list
    fn parse_expression_list(&mut self) -> Result<Vec<Expression>> {
        let mut exprs = Vec::new();

        if !self.check(TokenType::RightParen) {
            loop {
                exprs.push(self.parse_expression()?);
                if !self.check(TokenType::Comma) {
                    break;
                }
                self.advance();
            }
        }

        Ok(exprs)
    }

    /// Parse ORDER BY clause
    fn parse_order_by(&mut self) -> Result<Vec<OrderBy>> {
        let mut items = Vec::new();

        loop {
            let expr = self.parse_expression()?;
            let ascending = if self.check(TokenType::Desc) {
                self.advance();
                false
            } else {
                if self.check(TokenType::Asc) {
                    self.advance();
                }
                true
            };
            items.push(OrderBy { expr, ascending });

            if !self.check(TokenType::Comma) {
                break;
            }
            self.advance();
        }

        Ok(items)
    }

    /// Parse number
    fn parse_number(&mut self) -> Result<usize> {
        match self.current().typ.clone() {
            TokenType::Number(n) => {
                self.advance();
                Ok(n as usize)
            }
            _ => Err(DatabaseError::SyntaxError(
                String::from("Expected number")
            )),
        }
    }

    /// Parse INSERT statement
    fn parse_insert(&mut self) -> Result<Statement> {
        self.consume(TokenType::Insert)?;
        self.consume(TokenType::Into)?;

        let table_name = match self.current().typ.clone() {
            TokenType::Identifier(n) => {
                self.advance();
                n
            }
            _ => return Err(DatabaseError::SyntaxError(
                String::from("Expected table name")
            )),
        };

        let mut columns = Vec::new();
        if self.check(TokenType::LeftParen) {
            self.advance();
            while !self.check(TokenType::RightParen) {
                let col = match self.current().typ.clone() {
                    TokenType::Identifier(n) => {
                        self.advance();
                        n
                    }
                    _ => return Err(DatabaseError::SyntaxError(
                        String::from("Expected column name")
                    )),
                };
                columns.push(col);

                if !self.check(TokenType::Comma) {
                    break;
                }
                self.advance();
            }
            self.consume(TokenType::RightParen)?;
        }

        self.consume(TokenType::Values)?;

        let mut values = Vec::new();
        loop {
            self.consume(TokenType::LeftParen)?;
            let mut row = Vec::new();
            while !self.check(TokenType::RightParen) {
                row.push(self.parse_expression()?);
                if !self.check(TokenType::Comma) {
                    break;
                }
                self.advance();
            }
            self.consume(TokenType::RightParen)?;
            values.push(row);

            if !self.check(TokenType::Comma) {
                break;
            }
            self.advance();
        }

        Ok(Statement::Insert {
            table_name,
            columns,
            values,
        })
    }

    /// Parse UPDATE statement
    fn parse_update(&mut self) -> Result<Statement> {
        self.consume(TokenType::Update)?;

        let table_name = match self.current().typ.clone() {
            TokenType::Identifier(n) => {
                self.advance();
                n
            }
            _ => return Err(DatabaseError::SyntaxError(
                String::from("Expected table name")
            )),
        };

        self.consume(TokenType::Set)?;

        let mut assignments = Vec::new();
        loop {
            let col = match self.current().typ.clone() {
                TokenType::Identifier(n) => {
                    self.advance();
                    n
                }
                _ => return Err(DatabaseError::SyntaxError(
                    String::from("Expected column name")
                )),
            };

            self.consume(TokenType::Equal)?;
            let expr = self.parse_expression()?;

            assignments.push((col, expr));

            if !self.check(TokenType::Comma) {
                break;
            }
            self.advance();
        }

        let mut where_clause = None;
        if self.check(TokenType::Where) {
            self.advance();
            where_clause = Some(self.parse_expression()?);
        }

        Ok(Statement::Update {
            table_name,
            assignments,
            where_clause,
        })
    }

    /// Parse DELETE statement
    fn parse_delete(&mut self) -> Result<Statement> {
        self.consume(TokenType::Delete)?;
        self.consume(TokenType::From)?;

        let table_name = match self.current().typ.clone() {
            TokenType::Identifier(n) => {
                self.advance();
                n
            }
            _ => return Err(DatabaseError::SyntaxError(
                String::from("Expected table name")
            )),
        };

        let mut where_clause = None;
        if self.check(TokenType::Where) {
            self.advance();
            where_clause = Some(self.parse_expression()?);
        }

        Ok(Statement::Delete {
            table_name,
            where_clause,
        })
    }

    /// Parse CREATE statement
    fn parse_create(&mut self) -> Result<Statement> {
        self.consume(TokenType::Create)?;

        match self.current().typ {
            TokenType::Table => self.parse_create_table(),
            TokenType::Index => self.parse_create_index(),
            _ => Err(DatabaseError::SyntaxError(
                String::from("Expected TABLE or INDEX")
            )),
        }
    }

    /// Parse CREATE TABLE statement
    fn parse_create_table(&mut self) -> Result<Statement> {
        self.consume(TokenType::Table)?;

        let table_name = match self.current().typ.clone() {
            TokenType::Identifier(n) => {
                self.advance();
                n
            }
            _ => return Err(DatabaseError::SyntaxError(
                String::from("Expected table name")
            )),
        };

        self.consume(TokenType::LeftParen)?;

        let mut columns = Vec::new();
        loop {
            let name = match self.current().typ.clone() {
                TokenType::Identifier(n) => {
                    self.advance();
                    n
                }
                _ => return Err(DatabaseError::SyntaxError(
                    String::from("Expected column name")
                )),
            };

            let typ = match self.current().typ.clone() {
                TokenType::Identifier(t) => {
                    self.advance();
                    match t.to_uppercase().as_str() {
                        "INT" | "INTEGER" => ColumnType::Integer,
                        "FLOAT" | "DOUBLE" => ColumnType::Float,
                        "TEXT" | "VARCHAR" => ColumnType::Text,
                        "BOOL" | "BOOLEAN" => ColumnType::Boolean,
                        "BLOB" => ColumnType::Blob,
                        _ => return Err(DatabaseError::SyntaxError(
                            format!("Unknown type: {}", t)
                        )),
                    }
                }
                _ => return Err(DatabaseError::SyntaxError(
                    String::from("Expected type")
                )),
            };

            let mut nullable = true;
            let mut primary_key = false;
            let mut default = None;

            loop {
                if let TokenType::Identifier(ref kw) = self.current().typ {
                    match kw.to_uppercase().as_str() {
                        "PRIMARY" => {
                            self.advance();
                            if let TokenType::Identifier(ref k) = self.current().typ {
                                if k.to_uppercase() == "KEY" {
                                    self.advance();
                                    primary_key = true;
                                    nullable = false;
                                }
                            }
                        }
                        "NOT" => {
                            self.advance();
                            if let TokenType::Identifier(ref k) = self.current().typ {
                                if k.to_uppercase() == "NULL" {
                                    self.advance();
                                    nullable = false;
                                }
                            }
                        }
                        "NULL" => {
                            self.advance();
                            nullable = true;
                        }
                        "DEFAULT" => {
                            self.advance();
                            default = Some(self.parse_default_value()?);
                        }
                        _ => break,
                    }
                } else {
                    break;
                }
            }

            columns.push(ColumnDef {
                name,
                typ,
                nullable,
                primary_key,
                default,
            });

            if !self.check(TokenType::Comma) {
                break;
            }
            self.advance();
        }

        self.consume(TokenType::RightParen)?;

        Ok(Statement::CreateTable {
            table_name,
            columns,
        })
    }

    /// Parse default value
    fn parse_default_value(&mut self) -> Result<Value> {
        match self.current().typ.clone() {
            TokenType::StringLiteral(s) => {
                self.advance();
                Ok(Value::String(s))
            }
            TokenType::Number(n) => {
                self.advance();
                if n.fract() == 0.0 {
                    Ok(Value::Integer(n as i64))
                } else {
                    Ok(Value::Float(n))
                }
            }
            TokenType::Boolean(b) => {
                self.advance();
                Ok(Value::Boolean(b))
            }
            TokenType::Null => {
                self.advance();
                Ok(Value::Null)
            }
            _ => Err(DatabaseError::SyntaxError(
                String::from("Expected default value")
            )),
        }
    }

    /// Parse CREATE INDEX statement
    fn parse_create_index(&mut self) -> Result<Statement> {
        self.consume(TokenType::Index)?;

        let index_name = match self.current().typ.clone() {
            TokenType::Identifier(n) => {
                self.advance();
                n
            }
            _ => return Err(DatabaseError::SyntaxError(
                String::from("Expected index name")
            )),
        };

        self.consume(TokenType::On)?;

        let table_name = match self.current().typ.clone() {
            TokenType::Identifier(n) => {
                self.advance();
                n
            }
            _ => return Err(DatabaseError::SyntaxError(
                String::from("Expected table name")
            )),
        };

        self.consume(TokenType::LeftParen)?;

        let mut columns = Vec::new();
        loop {
            let col = match self.current().typ.clone() {
                TokenType::Identifier(n) => {
                    self.advance();
                    n
                }
                _ => return Err(DatabaseError::SyntaxError(
                    String::from("Expected column name")
                )),
            };
            columns.push(col);

            if !self.check(TokenType::Comma) {
                break;
            }
            self.advance();
        }

        self.consume(TokenType::RightParen)?;

        Ok(Statement::CreateIndex {
            index_name,
            table_name,
            columns,
            unique: false,
        })
    }

    /// Parse DROP statement
    fn parse_drop(&mut self) -> Result<Statement> {
        self.consume(TokenType::Drop)?;

        match self.current().typ {
            TokenType::Table => self.parse_drop_table(),
            TokenType::Index => self.parse_drop_index(),
            _ => Err(DatabaseError::SyntaxError(
                String::from("Expected TABLE or INDEX")
            )),
        }
    }

    /// Parse DROP TABLE statement
    fn parse_drop_table(&mut self) -> Result<Statement> {
        self.consume(TokenType::Table)?;

        let table_name = match self.current().typ.clone() {
            TokenType::Identifier(n) => {
                self.advance();
                n
            }
            _ => return Err(DatabaseError::SyntaxError(
                String::from("Expected table name")
            )),
        };

        Ok(Statement::DropTable { table_name })
    }

    /// Parse DROP INDEX statement
    fn parse_drop_index(&mut self) -> Result<Statement> {
        self.consume(TokenType::Index)?;

        let index_name = match self.current().typ.clone() {
            TokenType::Identifier(n) => {
                self.advance();
                n
            }
            _ => return Err(DatabaseError::SyntaxError(
                String::from("Expected index name")
            )),
        };

        Ok(Statement::DropIndex { index_name })
    }
}

/// SQL engine that combines lexer, parser, and planner
pub struct SqlEngine {
    /// Enable query optimizer
    enable_optimizer: bool,
}

impl SqlEngine {
    /// Create a new SQL engine
    pub fn new(enable_optimizer: bool) -> Result<Self> {
        Ok(Self {
            enable_optimizer,
        })
    }

    /// Parse a SQL statement
    pub fn parse(&self, sql: &str) -> Result<Statement> {
        let mut lexer = SqlLexer::new(sql);
        let tokens = lexer.tokenize()?;
        let mut parser = SqlParser::new(tokens);
        parser.parse()
    }

    /// Create a query plan from a statement
    pub fn plan(&self, stmt: Statement) -> Result<QueryPlan> {
        let logical_plan = self.create_logical_plan(stmt)?;
        let physical_plan = self.create_physical_plan(logical_plan.clone())?;
        Ok(QueryPlan {
            logical: logical_plan,
            physical: physical_plan,
        })
    }

    /// Create logical plan
    fn create_logical_plan(&self, stmt: Statement) -> Result<LogicalPlan> {
        match stmt {
            Statement::Select(query) => Ok(LogicalPlan::Select(query)),
            Statement::Insert { table_name, columns, values } => {
                Ok(LogicalPlan::Insert { table_name, columns, values })
            }
            Statement::Update { table_name, assignments, where_clause } => {
                Ok(LogicalPlan::Update { table_name, assignments, where_clause })
            }
            Statement::Delete { table_name, where_clause } => {
                Ok(LogicalPlan::Delete { table_name, where_clause })
            }
            Statement::CreateTable { table_name, columns } => {
                Ok(LogicalPlan::CreateTable { table_name, columns })
            }
            Statement::CreateIndex { index_name, table_name, columns, unique } => {
                Ok(LogicalPlan::CreateIndex { index_name, table_name, columns, unique })
            }
            Statement::DropTable { table_name } => {
                Ok(LogicalPlan::DropTable { table_name })
            }
            Statement::DropIndex { index_name } => {
                Ok(LogicalPlan::DropIndex { index_name })
            }
        }
    }

    /// Create physical plan
    fn create_physical_plan(&self, logical: LogicalPlan) -> Result<PhysicalPlan> {
        // Simple optimization: if optimizer is enabled, apply some rules
        if self.enable_optimizer {
            self.optimize_physical_plan(logical)
        } else {
            Ok(self.create_default_physical_plan(logical)?)
        }
    }

    /// Optimize physical plan
    fn optimize_physical_plan(&self, logical: LogicalPlan) -> Result<PhysicalPlan> {
        // Apply optimization rules
        let optimized = self.push_down_predicates(logical)?;
        Ok(self.create_default_physical_plan(optimized)?)
    }

    /// Push down predicates
    fn push_down_predicates(&self, plan: LogicalPlan) -> Result<LogicalPlan> {
        // Simplified predicate pushdown
        Ok(plan)
    }

    /// Create default physical plan
    fn create_default_physical_plan(&self, logical: LogicalPlan) -> Result<PhysicalPlan> {
        match logical {
            LogicalPlan::Select(query) => {
                Ok(PhysicalPlan::SeqScan { query })
            }
            LogicalPlan::Insert { table_name, columns, values } => {
                Ok(PhysicalPlan::Insert { table_name, columns, values })
            }
            LogicalPlan::Update { table_name, assignments, where_clause } => {
                Ok(PhysicalPlan::Update { table_name, assignments, where_clause })
            }
            LogicalPlan::Delete { table_name, where_clause } => {
                Ok(PhysicalPlan::Delete { table_name, where_clause })
            }
            LogicalPlan::CreateTable { table_name, columns } => {
                Ok(PhysicalPlan::CreateTable { table_name, columns })
            }
            LogicalPlan::CreateIndex { index_name, table_name, columns, unique } => {
                Ok(PhysicalPlan::CreateIndex { index_name, table_name, columns, unique })
            }
            LogicalPlan::DropTable { table_name } => {
                Ok(PhysicalPlan::DropTable { table_name })
            }
            LogicalPlan::DropIndex { index_name } => {
                Ok(PhysicalPlan::DropIndex { index_name })
            }
        }
    }
}

/// Logical query plan
#[derive(Debug, Clone)]
pub enum LogicalPlan {
    Select(Query),
    Insert { table_name: String, columns: Vec<String>, values: Vec<Vec<Expression>> },
    Update { table_name: String, assignments: Vec<(String, Expression)>, where_clause: Option<Expression> },
    Delete { table_name: String, where_clause: Option<Expression> },
    CreateTable { table_name: String, columns: Vec<ColumnDef> },
    CreateIndex { index_name: String, table_name: String, columns: Vec<String>, unique: bool },
    DropTable { table_name: String },
    DropIndex { index_name: String },
}

/// Physical query plan
#[derive(Debug, Clone)]
pub enum PhysicalPlan {
    SeqScan { query: Query },
    IndexScan { table_name: String, index_name: String, query: Query },
    NestedLoopJoin { left: Box<PhysicalPlan>, right: Box<PhysicalPlan>, condition: Expression },
    HashJoin { left: Box<PhysicalPlan>, right: Box<PhysicalPlan>, join_key: String },
    MergeJoin { left: Box<PhysicalPlan>, right: Box<PhysicalPlan>, join_key: String },
    Aggregate { input: Box<PhysicalPlan>, group_by: Vec<Expression>, aggregates: Vec<Expression> },
    Sort { input: Box<PhysicalPlan>, order_by: Vec<OrderBy> },
    Limit { input: Box<PhysicalPlan>, limit: usize, offset: usize },
    Insert { table_name: String, columns: Vec<String>, values: Vec<Vec<Expression>> },
    Update { table_name: String, assignments: Vec<(String, Expression)>, where_clause: Option<Expression> },
    Delete { table_name: String, where_clause: Option<Expression> },
    CreateTable { table_name: String, columns: Vec<ColumnDef> },
    CreateIndex { index_name: String, table_name: String, columns: Vec<String>, unique: bool },
    DropTable { table_name: String },
    DropIndex { index_name: String },
}

/// Query plan containing both logical and physical plans
#[derive(Debug, Clone)]
pub struct QueryPlan {
    /// Logical plan
    pub logical: LogicalPlan,

    /// Physical plan
    pub physical: PhysicalPlan,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_select() {
        let sql = "SELECT id, name FROM users";
        let mut lexer = SqlLexer::new(sql);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].typ, TokenType::Select);
        assert_eq!(tokens[1].typ, TokenType::Identifier(String::from("id")));
        assert_eq!(tokens[2].typ, TokenType::Comma);
        assert_eq!(tokens[3].typ, TokenType::Identifier(String::from("name")));
    }

    #[test]
    fn test_lexer_string_literal() {
        let sql = "'hello world'";
        let mut lexer = SqlLexer::new(sql);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].typ, TokenType::StringLiteral(String::from("hello world")));
    }

    #[test]
    fn test_parser_select_simple() {
        let sql = "SELECT id FROM users";
        let engine = SqlEngine::new(false).unwrap();
        let stmt = engine.parse(sql).unwrap();

        match stmt {
            Statement::Select(query) => {
                assert_eq!(query.columns.len(), 1);
                assert_eq!(query.from.len(), 1);
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parser_select_with_where() {
        let sql = "SELECT * FROM users WHERE id = 1";
        let engine = SqlEngine::new(false).unwrap();
        let stmt = engine.parse(sql).unwrap();

        match stmt {
            Statement::Select(query) => {
                assert!(query.where_clause.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parser_insert() {
        let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice')";
        let engine = SqlEngine::new(false).unwrap();
        let stmt = engine.parse(sql).unwrap();

        match stmt {
            Statement::Insert { table_name, columns, values } => {
                assert_eq!(table_name, "users");
                assert_eq!(columns.len(), 2);
                assert_eq!(values.len(), 1);
            }
            _ => panic!("Expected INSERT statement"),
        }
    }

    #[test]
    fn test_parser_create_table() {
        let sql = "CREATE TABLE users (id INT PRIMARY KEY, name TEXT NOT NULL)";
        let engine = SqlEngine::new(false).unwrap();
        let stmt = engine.parse(sql).unwrap();

        match stmt {
            Statement::CreateTable { table_name, columns } => {
                assert_eq!(table_name, "users");
                assert_eq!(columns.len(), 2);
            }
            _ => panic!("Expected CREATE TABLE statement"),
        }
    }

    #[test]
    fn test_query_plan() {
        let sql = "SELECT * FROM users";
        let engine = SqlEngine::new(true).unwrap();
        let stmt = engine.parse(sql).unwrap();
        let plan = engine.plan(stmt).unwrap();

        match plan.physical {
            PhysicalPlan::SeqScan { .. } => {},
            _ => panic!("Expected SeqScan"),
        }
    }

    #[test]
    fn test_error_unterminated_string() {
        let sql = "'unterminated";
        let mut lexer = SqlLexer::new(sql);
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn test_expression_arithmetic() {
        let sql = "SELECT a + b * c FROM t";
        let engine = SqlEngine::new(false).unwrap();
        let stmt = engine.parse(sql).unwrap();

        match stmt {
            Statement::Select(query) => {
                assert_eq!(query.columns.len(), 1);
            }
            _ => panic!("Expected SELECT statement"),
        }
    }
}
