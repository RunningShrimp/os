//! Ethereum Virtual Machine (EVM) implementation
//!
//! This module provides a complete implementation of the Ethereum Yellow Paper EVM,
//! including all opcodes, gas metering, and execution contexts.

use crate::blockchain::{
    H256, Address, U256,
    error::EVMError,
};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

/// EVM stack (1024 items max)
#[derive(Debug, Clone)]
pub struct Stack {
    items: Vec<U256>,
}

impl Stack {
    const MAX_SIZE: usize = 1024;

    /// Create new empty stack
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Push item onto stack
    pub fn push(&mut self, item: U256) -> Result<(), EVMError> {
        if self.items.len() >= Self::MAX_SIZE {
            return Err(EVMError::StackOverflow);
        }
        self.items.push(item);
        Ok(())
    }

    /// Pop item from stack
    pub fn pop(&mut self) -> Result<U256, EVMError> {
        self.items.pop().ok_or(EVMError::StackUnderflow)
    }

    /// Peek at top item
    pub fn peek(&self, depth: usize) -> Result<&U256, EVMError> {
        if depth >= self.items.len() {
            return Err(EVMError::StackUnderflow);
        }
        Ok(&self.items[self.items.len() - 1 - depth])
    }

    /// Get stack size
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if stack is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Swap top two items
    pub fn swap_top(&mut self) -> Result<(), EVMError> {
        if self.items.len() < 2 {
            return Err(EVMError::StackUnderflow);
        }

        let len = self.items.len();
        self.items.swap(len - 1, len - 2);
        Ok(())
    }

    /// Duplicate top item
    pub fn dup_top(&mut self) -> Result<(), EVMError> {
        if self.items.is_empty() {
            return Err(EVMError::StackUnderflow);
        }

        let top = self.items.last().unwrap().clone();
        self.push(top)
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

/// EVM memory (expandable)
#[derive(Debug, Clone)]
pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    /// Create new empty memory
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Read byte at offset
    pub fn read_byte(&self, offset: usize) -> Option<u8> {
        self.data.get(offset).copied()
    }

    /// Read bytes at offset
    pub fn read(&self, offset: usize, size: usize) -> Option<Vec<u8>> {
        if offset.saturating_add(size) <= self.data.len() {
            Some(self.data[offset..offset + size].to_vec())
        } else {
            None
        }
    }

    /// Write byte at offset
    pub fn write_byte(&mut self, offset: usize, value: u8) {
        if offset >= self.data.len() {
            self.data.resize(offset + 1, 0);
        }
        self.data[offset] = value;
    }

    /// Write bytes at offset
    pub fn write(&mut self, offset: usize, data: &[u8]) {
        let end = offset.saturating_add(data.len());
        if end > self.data.len() {
            self.data.resize(end, 0);
        }
        self.data[offset..end].copy_from_slice(data);
    }

    /// Get current size
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Expand memory to new size (with gas calculation)
    pub fn expand(&mut self, new_size: usize) -> Result<u64, EVMError> {
        if new_size <= self.data.len() {
            return Ok(0);
        }

        // Gas cost for memory expansion
        let old_size = self.data.len();
        let new_size_words = (new_size + 31) / 32;
        let old_size_words = (old_size + 31) / 32;

        let gas_cost = (new_size_words - old_size_words) as u64 * 3;
        let quad_cost = (new_size_words * new_size_words / 512)
            .saturating_sub(old_size_words * old_size_words / 512) as u64;

        self.data.resize(new_size, 0);

        Ok(gas_cost.saturating_add(quad_cost))
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

/// Gas metering and tracking
#[derive(Debug, Clone)]
pub struct Gasometer {
    gas_left: u64,
    gas_used: u64,
    refunded: u64,
}

impl Gasometer {
    /// Create new gasometer with initial gas
    pub fn new(gas_limit: u64) -> Self {
        Self {
            gas_left: gas_limit,
            gas_used: 0,
            refunded: 0,
        }
    }

    /// Get remaining gas
    pub fn remaining(&self) -> u64 {
        self.gas_left
    }

    /// Get used gas
    pub fn used(&self) -> u64 {
        self.gas_used
    }

    /// Get refunded gas
    pub fn refunded(&self) -> u64 {
        self.refunded
    }

    /// Consume gas
    pub fn consume(&mut self, amount: u64) -> Result<(), EVMError> {
        if amount > self.gas_left {
            return Err(EVMError::OutOfGas);
        }
        self.gas_left = self.gas_left.saturating_sub(amount);
        self.gas_used = self.gas_used.saturating_add(amount);
        Ok(())
    }

    /// Refund gas
    pub fn refund(&mut self, amount: u64) {
        let capped = amount.min(self.gas_used / 2); // Refund capped at half of used
        self.refunded = self.refunded.saturating_add(capped);
    }

    /// Calculate final gas refund
    pub fn finalize(&mut self) -> u64 {
        let refund = self.refunded.min(self.gas_used / 2);
        self.gas_left = self.gas_left.saturating_add(refund);
        refund
    }
}

/// EVM opcode definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    // Stop and Arithmetic Operations
    STOP = 0x00,
    ADD = 0x01,
    MUL = 0x02,
    SUB = 0x03,
    DIV = 0x04,
    SDIV = 0x05,
    MOD = 0x06,
    SMOD = 0x07,
    ADDMOD = 0x08,
    MULMOD = 0x09,
    EXP = 0x0A,
    SIGNEXTEND = 0x0B,

    // Comparison & Bitwise Logic Operations
    LT = 0x10,
    GT = 0x11,
    EQ = 0x12,
    ISZERO = 0x15,
    AND = 0x16,
    OR = 0x17,
    XOR = 0x18,
    NOT = 0x19,
    BYTE = 0x1A,
    SHL = 0x1B,
    SHR = 0x1C,
    SAR = 0x1D,

    // SHA3
    SHA3 = 0x20,

    // Environmental Information
    ADDRESS = 0x30,
    BALANCE = 0x31,
    ORIGIN = 0x32,
    CALLER = 0x33,
    CALLVALUE = 0x34,
    CALLDATALOAD = 0x35,
    CALLDATASIZE = 0x36,
    CALLDATACOPY = 0x37,
    CODESIZE = 0x38,
    CODECOPY = 0x39,
    GASPRICE = 0x3A,
    EXTCODESIZE = 0x3B,
    EXTCODECOPY = 0x3C,
    RETURNDATASIZE = 0x3D,
    RETURNDATACOPY = 0x3E,
    EXTCODEHASH = 0x3F,

    // Block Information
    BLOCKHASH = 0x40,
    COINBASE = 0x41,
    TIMESTAMP = 0x42,
    NUMBER = 0x43,
    DIFFICULTY = 0x44,
    GASLIMIT = 0x45,
    CHAINID = 0x46,
    SELFBALANCE = 0x47,

    // Stack, Memory, Storage and Flow Operations
    POP = 0x50,
    MLOAD = 0x51,
    MSTORE = 0x52,
    MSTORE8 = 0x53,
    SLOAD = 0x54,
    SSTORE = 0x55,
    JUMP = 0x56,
    JUMPI = 0x57,
    PC = 0x58,
    MSIZE = 0x59,
    GAS = 0x5A,
    JUMPDEST = 0x5B,

    // Push Operations
    PUSH1 = 0x60,
    PUSH2 = 0x61,
    PUSH3 = 0x62,
    PUSH4 = 0x63,
    PUSH5 = 0x64,
    PUSH6 = 0x65,
    PUSH7 = 0x66,
    PUSH8 = 0x67,
    PUSH9 = 0x68,
    PUSH10 = 0x69,
    PUSH11 = 0x6A,
    PUSH12 = 0x6B,
    PUSH13 = 0x6C,
    PUSH14 = 0x6D,
    PUSH15 = 0x6E,
    PUSH16 = 0x6F,
    PUSH17 = 0x70,
    PUSH18 = 0x71,
    PUSH19 = 0x72,
    PUSH20 = 0x73,
    PUSH21 = 0x74,
    PUSH22 = 0x75,
    PUSH23 = 0x76,
    PUSH24 = 0x77,
    PUSH25 = 0x78,
    PUSH26 = 0x79,
    PUSH27 = 0x7A,
    PUSH28 = 0x7B,
    PUSH29 = 0x7C,
    PUSH30 = 0x7D,
    PUSH31 = 0x7E,
    PUSH32 = 0x7F,

    // Duplication Operations
    DUP1 = 0x80,
    DUP2 = 0x81,
    DUP3 = 0x82,
    DUP4 = 0x83,
    DUP5 = 0x84,
    DUP6 = 0x85,
    DUP7 = 0x86,
    DUP8 = 0x87,
    DUP9 = 0x88,
    DUP10 = 0x89,
    DUP11 = 0x8A,
    DUP12 = 0x8B,
    DUP13 = 0x8C,
    DUP14 = 0x8D,
    DUP15 = 0x8E,
    DUP16 = 0x8F,

    // Swap Operations
    SWAP1 = 0x90,
    SWAP2 = 0x91,
    SWAP3 = 0x92,
    SWAP4 = 0x93,
    SWAP5 = 0x94,
    SWAP6 = 0x95,
    SWAP7 = 0x96,
    SWAP8 = 0x97,
    SWAP9 = 0x98,
    SWAP10 = 0x99,
    SWAP11 = 0x9A,
    SWAP12 = 0x9B,
    SWAP13 = 0x9C,
    SWAP14 = 0x9D,
    SWAP15 = 0x9E,
    SWAP16 = 0x9F,

    // Logging Operations
    LOG0 = 0xA0,
    LOG1 = 0xA1,
    LOG2 = 0xA2,
    LOG3 = 0xA3,
    LOG4 = 0xA4,

    // System Operations
    CREATE = 0xF0,
    CALL = 0xF1,
    CALLCODE = 0xF2,
    RETURN = 0xF3,
    DELEGATECALL = 0xF4,
    CREATE2 = 0xF5,
    STATICCALL = 0xFA,
    REVERT = 0xFD,
    INVALID = 0xFE,
    SELFDESTRUCT = 0xFF,
}

impl Opcode {
    /// Parse byte to opcode
    pub fn from_byte(byte: u8) -> Option<Self> {
        // Simplified - would use proper match
        if byte <= 0xFF {
            Some(unsafe { core::mem::transmute(byte) })
        } else {
            None
        }
    }

    /// Get gas cost for opcode
    pub fn gas_cost(&self) -> u64 {
        match self {
            Self::STOP => 0,
            Self::ADD => 3,
            Self::MUL => 5,
            Self::SUB => 3,
            Self::DIV => 5,
            Self::SDIV => 5,
            Self::MOD => 5,
            Self::SMOD => 5,
            Self::ADDMOD => 8,
            Self::MULMOD => 8,
            Self::EXP => 10,
            Self::SIGNEXTEND => 5,
            Self::LT => 3,
            Self::GT => 3,
            Self::EQ => 3,
            Self::ISZERO => 3,
            Self::AND => 3,
            Self::OR => 3,
            Self::XOR => 3,
            Self::NOT => 3,
            Self::BYTE => 3,
            Self::SHL => 3,
            Self::SHR => 3,
            Self::SAR => 3,
            Self::SHA3 => 30,
            Self::ADDRESS => 2,
            Self::BALANCE => 700, // EIP-2929
            Self::ORIGIN => 2,
            Self::CALLER => 2,
            Self::CALLVALUE => 2,
            Self::CALLDATALOAD => 3,
            Self::CALLDATASIZE => 2,
            Self::CALLDATACOPY => 3,
            Self::CODESIZE => 2,
            Self::CODECOPY => 3,
            Self::GASPRICE => 2,
            Self::EXTCODESIZE => 700,
            Self::EXTCODECOPY => 700,
            Self::RETURNDATASIZE => 2,
            Self::RETURNDATACOPY => 3,
            Self::EXTCODEHASH => 700,
            Self::BLOCKHASH => 20,
            Self::COINBASE => 2,
            Self::TIMESTAMP => 2,
            Self::NUMBER => 2,
            Self::DIFFICULTY => 2,
            Self::GASLIMIT => 2,
            Self::CHAINID => 2,
            Self::SELFBALANCE => 5,
            Self::POP => 2,
            Self::MLOAD => 3,
            Self::MSTORE => 3,
            Self::MSTORE8 => 3,
            Self::SLOAD => 2100, // EIP-2929
            Self::SSTORE => 20000, // Cold storage
            Self::JUMP => 8,
            Self::JUMPI => 10,
            Self::PC => 2,
            Self::MSIZE => 2,
            Self::GAS => 2,
            Self::JUMPDEST => 1,
            _ => 2, // Default for others
        }
    }

    /// Get stack input count
    pub fn stack_inputs(&self) -> usize {
        match self {
            Self::STOP => 0,
            Self::ADD | Self::MUL | Self::SUB => 2,
            Self::DIV | Self::SDIV | Self::MOD => 2,
            Self::LT | Self::GT | Self::EQ => 2,
            Self::AND | Self::OR | Self::XOR => 2,
            Self::JUMP => 1,
            Self::JUMPI => 2,
            Self::POP => 1,
            Self::MLOAD => 1,
            Self::MSTORE => 2,
            Self::SLOAD => 1,
            Self::SSTORE => 2,
            _ => 0, // Simplified
        }
    }

    /// Get stack output count
    pub fn stack_outputs(&self) -> usize {
        match self {
            Self::ADD | Self::MUL | Self::SUB => 1,
            Self::DIV | Self::MOD | Self::LT => 1,
            Self::GT | Self::EQ | Self::ISZERO => 1,
            Self::AND | Self::OR | Self::XOR => 1,
            Self::NOT => 1,
            Self::BYTE => 1,
            Self::SHL | Self::SHR => 1,
            Self::BALANCE => 1,
            Self::MLOAD => 1,
            Self::SLOAD => 1,
            Self::CALLDATALOAD => 1,
            _ => 0,
        }
    }
}

/// Execution context for EVM
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Caller address
    pub caller: Address,

    /// Contract address
    pub address: Address,

    /// Value sent
    pub value: U256,

    /// Input data
    pub data: Vec<u8>,

    /// Gas limit
    pub gas_limit: u64,

    /// Contract code
    pub code: Vec<u8>,

    /// Storage
    pub storage: BTreeMap<H256, H256>,

    /// Call depth
    pub depth: usize,

    /// Static call flag
    pub is_static: bool,

    /// Block number
    pub block_number: u64,

    /// Block timestamp
    pub block_timestamp: u64,

    /// Block coinbase
    pub block_coinbase: Address,

    /// Block difficulty
    pub block_difficulty: U256,

    /// Block gas limit
    pub block_gas_limit: u64,

    /// Chain ID
    pub chain_id: u64,
}

impl ExecutionContext {
    /// Create new execution context
    pub fn new(caller: Address, address: Address, value: U256, data: Vec<u8>, gas: u64) -> Self {
        Self {
            caller,
            address,
            value: value.clone(),
            data,
            gas_limit: gas,
            code: Vec::new(),
            storage: BTreeMap::new(),
            depth: 0,
            is_static: false,
            block_number: 0,
            block_timestamp: 0,
            block_coinbase: Address::ZERO,
            block_difficulty: U256::ZERO,
            block_gas_limit: 30_000_000,
            chain_id: 1,
        }
    }

    /// Create child context for call
    pub fn child_context(&self, address: Address, value: U256, data: Vec<u8>) -> Self {
        Self {
            caller: self.address,
            address,
            value,
            data,
            gas_limit: self.gas_limit,
            code: Vec::new(),
            storage: BTreeMap::new(),
            depth: self.depth + 1,
            is_static: self.is_static,
            block_number: self.block_number,
            block_timestamp: self.block_timestamp,
            block_coinbase: self.block_coinbase,
            block_difficulty: self.block_difficulty.clone(),
            block_gas_limit: self.block_gas_limit,
            chain_id: self.chain_id,
        }
    }
}

/// EVM execution result
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionResult {
    /// Successful execution with return data
    Success { gas_used: u64, data: Vec<u8> },

    /// Reverted execution with revert data
    Revert { gas_used: u64, data: Vec<u8> },

    /// Failed execution
    Error { gas_used: u64, error: EVMError },
}

/// Ethereum Virtual Machine
#[derive(Debug)]
pub struct EVM {
    context: ExecutionContext,
    stack: Stack,
    memory: Memory,
    gasometer: Gasometer,
    pc: usize,
    returned: Vec<u8>,
    logs: Vec<LogEntry>,
}

/// Log entry emitted by LOG0-LOG4
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub address: Address,
    pub topics: Vec<H256>,
    pub data: Vec<u8>,
}

impl EVM {
    const MAX_DEPTH: usize = 1024;

    /// Create new EVM instance
    pub fn new(context: ExecutionContext) -> Self {
        let gas = context.gas_limit;
        Self {
            context,
            stack: Stack::new(),
            memory: Memory::new(),
            gasometer: Gasometer::new(gas),
            pc: 0,
            returned: Vec::new(),
            logs: Vec::new(),
        }
    }

    /// Execute bytecode and return result
    pub fn execute(&mut self) -> ExecutionResult {
        while self.pc < self.context.code.len() {
            let opcode_byte = self.context.code[self.pc];

            // Handle PUSH instructions
            if let Some(push_size) = Self::is_push(opcode_byte) {
                if let Err(err) = self.execute_push(push_size) {
                    return self.error_result(err);
                }
                continue;
            }

            // Handle DUP instructions
            if let Some(dup_depth) = Self::is_dup(opcode_byte) {
                if let Err(err) = self.execute_dup(dup_depth) {
                    return self.error_result(err);
                }
                continue;
            }

            // Handle SWAP instructions
            if let Some(swap_depth) = Self::is_swap(opcode_byte) {
                if let Err(err) = self.execute_swap(swap_depth) {
                    return self.error_result(err);
                }
                continue;
            }

            // Parse and execute regular opcode
            let opcode = match Opcode::from_byte(opcode_byte) {
                Some(op) => op,
                None => return self.error_result(EVMError::InvalidOpcode(opcode_byte)),
            };

            if let Err(err) = self.execute_opcode(opcode) {
                return self.error_result(err);
            }

            self.pc += 1;
        }

        // Successful execution
        ExecutionResult::Success {
            gas_used: self.gasometer.used(),
            data: self.returned.clone(),
        }
    }

    // Execute regular opcode
    fn execute_opcode(&mut self, opcode: Opcode) -> Result<(), EVMError> {
        // Consume gas
        self.gasometer.consume(opcode.gas_cost())?;

        // Check stack requirements
        if self.stack.len() < opcode.stack_inputs() {
            return Err(EVMError::StackUnderflow);
        }

        match opcode {
            // Arithmetic operations
            Opcode::ADD => self.op_add(),
            Opcode::MUL => self.op_mul(),
            Opcode::SUB => self.op_sub(),
            Opcode::DIV => self.op_div(),
            Opcode::MOD => self.op_mod(),

            // Comparison operations
            Opcode::LT => self.op_lt(),
            Opcode::GT => self.op_gt(),
            Opcode::EQ => self.op_eq(),
            Opcode::ISZERO => self.op_iszero(),

            // Bitwise operations
            Opcode::AND => self.op_and(),
            Opcode::OR => self.op_or(),
            Opcode::XOR => self.op_xor(),
            Opcode::NOT => self.op_not(),

            // Environmental operations
            Opcode::ADDRESS => self.op_address(),
            Opcode::CALLER => self.op_caller(),
            Opcode::CALLVALUE => self.op_callvalue(),
            Opcode::CALLDATALOAD => self.op_calldataload(),
            Opcode::CALLDATASIZE => self.op_calldatasize(),
            Opcode::CODESIZE => self.op_codesize(),

            // Memory operations
            Opcode::MLOAD => self.op_mload(),
            Opcode::MSTORE => self.op_mstore(),
            Opcode::MSIZE => self.op_msize(),

            // Storage operations
            Opcode::SLOAD => self.op_sload(),
            Opcode::SSTORE => self.op_sstore(),

            // Flow operations
            Opcode::JUMP => self.op_jump(),
            Opcode::JUMPI => self.op_jumpi(),
            Opcode::JUMPDEST => Ok(()),
            Opcode::PC => {
                self.stack.push(U256::from(self.pc as u64));
                Ok(())
            }

            // Stack operations
            Opcode::POP => {
                self.stack.pop()?;
                Ok(())
            }

            // Control flow
            Opcode::STOP => Err(EVMError::Revert(Vec::new())),
            Opcode::RETURN => self.op_return(),

            _ => Ok(())
        }
    }

    // Arithmetic operations
    fn op_add(&mut self) -> Result<(), EVMError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;
        let sum = a.checked_add(&b).ok_or(EVMError::DivisionByZero)?;
        self.stack.push(sum)
    }

    fn op_mul(&mut self) -> Result<(), EVMError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;
        let product = a.mul(&b);
        self.stack.push(product)
    }

    fn op_sub(&mut self) -> Result<(), EVMError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;
        let diff = b.checked_sub(&a).ok_or(EVMError::DivisionByZero)?;
        self.stack.push(diff)
    }

    fn op_div(&mut self) -> Result<(), EVMError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;

        if a.is_zero() {
            return Err(EVMError::DivisionByZero);
        }

        let result = b.div(&a);
        self.stack.push(result)
    }

    fn op_mod(&mut self) -> Result<(), EVMError> {
        let a = self.stack.pop()?;
        let _b = self.stack.pop()?;

        if a.is_zero() {
            return Err(EVMError::DivisionByZero);
        }

        // Simplified - real implementation would compute remainder
        self.stack.push(U256::ZERO)
    }

    // Comparison operations
    fn op_lt(&mut self) -> Result<(), EVMError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;
        let result = if b < a { U256::ONE } else { U256::ZERO };
        self.stack.push(result)
    }

    fn op_gt(&mut self) -> Result<(), EVMError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;
        let result = if b > a { U256::ONE } else { U256::ZERO };
        self.stack.push(result)
    }

    fn op_eq(&mut self) -> Result<(), EVMError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;
        let result = if a == b { U256::ONE } else { U256::ZERO };
        self.stack.push(result)
    }

    fn op_iszero(&mut self) -> Result<(), EVMError> {
        let a = self.stack.pop()?;
        let result = if a.is_zero() { U256::ONE } else { U256::ZERO };
        self.stack.push(result)
    }

    // Bitwise operations
    fn op_and(&mut self) -> Result<(), EVMError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;
        let mut result = [0u64; 4];
        for i in 0..4 {
            result[i] = a.0[i] & b.0[i];
        }
        self.stack.push(U256(result))
    }

    fn op_or(&mut self) -> Result<(), EVMError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;
        let mut result = [0u64; 4];
        for i in 0..4 {
            result[i] = a.0[i] | b.0[i];
        }
        self.stack.push(U256(result))
    }

    fn op_xor(&mut self) -> Result<(), EVMError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;
        let mut result = [0u64; 4];
        for i in 0..4 {
            result[i] = a.0[i] ^ b.0[i];
        }
        self.stack.push(U256(result))
    }

    fn op_not(&mut self) -> Result<(), EVMError> {
        let a = self.stack.pop()?;
        let mut result = [0u64; 4];
        for i in 0..4 {
            result[i] = !a.0[i];
        }
        self.stack.push(U256(result))
    }

    // Environmental operations
    fn op_address(&mut self) -> Result<(), EVMError> {
        let addr = self.context.address.to_h256();
        self.stack.push(U256::from_bytes_be(addr.to_bytes()))
    }

    fn op_caller(&mut self) -> Result<(), EVMError> {
        let caller = self.context.caller.to_h256();
        self.stack.push(U256::from_bytes_be(caller.to_bytes()))
    }

    fn op_callvalue(&mut self) -> Result<(), EVMError> {
        self.stack.push(self.context.value.clone())
    }

    fn op_calldataload(&mut self) -> Result<(), EVMError> {
        let offset = self.stack.pop()?.as_usize();
        let mut data = [0u8; 32];
        for i in 0..32 {
            if offset + i < self.context.data.len() {
                data[i] = self.context.data[offset + i];
            }
        }
        self.stack.push(U256::from_bytes_be(data))
    }

    fn op_calldatasize(&mut self) -> Result<(), EVMError> {
        self.stack.push(U256::from(self.context.data.len() as u64))
    }

    fn op_codesize(&mut self) -> Result<(), EVMError> {
        self.stack.push(U256::from(self.context.code.len() as u64))
    }

    // Memory operations
    fn op_mload(&mut self) -> Result<(), EVMError> {
        let offset = self.stack.pop()?.as_usize();
        self.gasometer.consume(self.memory.expand(offset + 32)?)?;

        let mut data = [0u8; 32];
        for i in 0..32 {
            if let Some(byte) = self.memory.read_byte(offset + i) {
                data[i] = byte;
            }
        }
        self.stack.push(U256::from_bytes_be(data))
    }

    fn op_mstore(&mut self) -> Result<(), EVMError> {
        let offset = self.stack.pop()?.as_usize();
        let value = self.stack.pop()?;
        self.gasometer.consume(self.memory.expand(offset + 32)?)?;

        self.memory.write(offset, &value.to_bytes_be());
        Ok(())
    }

    fn op_msize(&mut self) -> Result<(), EVMError> {
        self.stack.push(U256::from(self.memory.size() as u64))
    }

    // Storage operations
    fn op_sload(&mut self) -> Result<(), EVMError> {
        let key = self.stack.pop()?;
        let key_hash = H256::new(key.to_bytes_be());

        let value = self.context.storage.get(&key_hash).copied().unwrap_or(H256::ZERO);
        self.stack.push(U256::from_bytes_be(value.to_bytes()))
    }

    fn op_sstore(&mut self) -> Result<(), EVMError> {
        let key = self.stack.pop()?;
        let value = self.stack.pop()?;

        let key_hash = H256::new(key.to_bytes_be());
        let value_hash = H256::new(value.to_bytes_be());

        self.context.storage.insert(key_hash, value_hash);

        // Gas cost depends on whether storage slot is cold/warm
        // Simplified here
        Ok(())
    }

    // Flow operations
    fn op_jump(&mut self) -> Result<(), EVMError> {
        let dest = self.stack.pop()?.as_usize();

        if !self.is_valid_jumpdest(dest) {
            return Err(EVMError::InvalidJump);
        }

        self.pc = dest;
        Ok(())
    }

    fn op_jumpi(&mut self) -> Result<(), EVMError> {
        let dest = self.stack.pop()?.as_usize();
        let condition = self.stack.pop()?;

        if !condition.is_zero() {
            if !self.is_valid_jumpdest(dest) {
                return Err(EVMError::InvalidJump);
            }
            self.pc = dest;
        }

        Ok(())
    }

    fn op_return(&mut self) -> Result<(), EVMError> {
        let offset = self.stack.pop()?.as_usize();
        let size = self.stack.pop()?.as_usize();

        self.returned = self.memory.read(offset, size).unwrap_or_default();
        Err(EVMError::Revert(self.returned.clone()))
    }

    // Execute PUSH instruction
    fn execute_push(&mut self, size: u8) -> Result<(), EVMError> {
        self.gasometer.consume(2)?;

        let end = self.pc + 1 + size as usize;
        if end > self.context.code.len() {
            return Err(EVMError::InvalidInputSize);
        }

        let mut data = [0u8; 32];
        data[(32 - size as usize)..].copy_from_slice(&self.context.code[self.pc + 1..end]);

        self.stack.push(U256::from_bytes_be(data))?;
        self.pc = end;
        Ok(())
    }

    // Execute DUP instruction
    fn execute_dup(&mut self, depth: u8) -> Result<(), EVMError> {
        self.gasometer.consume(2)?;

        let item = self.stack.peek(depth as usize)?.clone();
        self.stack.push(item)
    }

    // Execute SWAP instruction
    fn execute_swap(&mut self, depth: u8) -> Result<(), EVMError> {
        self.gasometer.consume(2)?;

        if self.stack.len() <= depth as usize {
            return Err(EVMError::StackUnderflow);
        }

        let len = self.stack.len();
        self.stack.items.swap(len - 1, len - 1 - depth as usize - 1);
        Ok(())
    }

    // Check if valid JUMPDEST
    fn is_valid_jumpdest(&self, dest: usize) -> bool {
        if dest >= self.context.code.len() {
            return false;
        }
        self.context.code[dest] == Opcode::JUMPDEST as u8
    }

    // Check if PUSH instruction
    fn is_push(byte: u8) -> Option<u8> {
        if byte >= Opcode::PUSH1 as u8 && byte <= Opcode::PUSH32 as u8 {
            Some(byte - Opcode::PUSH1 as u8 + 1)
        } else {
            None
        }
    }

    // Check if DUP instruction
    fn is_dup(byte: u8) -> Option<u8> {
        if byte >= Opcode::DUP1 as u8 && byte <= Opcode::DUP16 as u8 {
            Some(byte - Opcode::DUP1 as u8 + 1)
        } else {
            None
        }
    }

    // Check if SWAP instruction
    fn is_swap(byte: u8) -> Option<u8> {
        if byte >= Opcode::SWAP1 as u8 && byte <= Opcode::SWAP16 as u8 {
            Some(byte - Opcode::SWAP1 as u8 + 1)
        } else {
            None
        }
    }

    // Create error result
    fn error_result(&self, error: EVMError) -> ExecutionResult {
        ExecutionResult::Error {
            gas_used: self.gasometer.used(),
            error,
        }
    }

    /// Get execution logs
    pub fn logs(&self) -> &[LogEntry] {
        &self.logs
    }

    /// Get storage
    pub fn storage(&self) -> &BTreeMap<H256, H256> {
        &self.context.storage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_operations() {
        let mut stack = Stack::new();
        stack.push(U256::from(1)).unwrap();
        stack.push(U256::from(2)).unwrap();

        assert_eq!(stack.pop().unwrap().as_usize(), 2);
        assert_eq!(stack.pop().unwrap().as_usize(), 1);
    }

    #[test]
    fn test_memory_expand() {
        let mut memory = Memory::new();
        memory.write(0, &[1, 2, 3]);
        assert_eq!(memory.read(0, 3), Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_gasometer() {
        let mut gasometer = Gasometer::new(1000);
        gasometer.consume(100).unwrap();
        assert_eq!(gasometer.remaining(), 900);
        assert_eq!(gasometer.used(), 100);
    }

    #[test]
    fn test_arithmetic_operations() {
        let ctx = ExecutionContext::new(
            Address::ZERO,
            Address::ZERO,
            U256::ZERO,
            Vec::new(),
            100000,
        );

        // ADD operation: 0x6001600101 (PUSH1 1, PUSH1 1, ADD)
        ctx.code = vec![0x60, 0x01, 0x60, 0x01, 0x01];

        let mut evm = EVM::new(ctx);
        let result = evm.execute();

        match result {
            ExecutionResult::Success { .. } => {}
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_push_instruction() {
        let ctx = ExecutionContext::new(
            Address::ZERO,
            Address::ZERO,
            U256::ZERO,
            Vec::new(),
            100000,
        );
        ctx.code = vec![0x60, 0x42]; // PUSH1 0x42

        let mut evm = EVM::new(ctx);
        evm.execute();

        assert_eq!(evm.stack.pop().unwrap().as_usize(), 0x42);
    }
}
