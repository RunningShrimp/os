//! Error types for blockchain operations

use core::fmt;
use alloc::string::String;
use alloc::vec::Vec;

/// Blockchain error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockchainError {
    /// Invalid block header
    InvalidHeader(String),

    /// Invalid transaction
    InvalidTransaction(String),

    /// Invalid signature
    InvalidSignature,

    /// Invalid proof
    InvalidProof,

    /// State root mismatch
    StateRootMismatch,

    /// Gas limit exceeded
    GasLimitExceeded,

    /// Insufficient funds
    InsufficientFunds,

    /// Nonce mismatch
    NonceMismatch,

    /// Chain not found
    ChainNotFound,

    /// Block not found
    BlockNotFound,

    /// Database error
    DatabaseError(String),

    /// Internal error
    InternalError(String),
}

impl fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHeader(msg) => write!(f, "Invalid header: {}", msg),
            Self::InvalidTransaction(msg) => write!(f, "Invalid transaction: {}", msg),
            Self::InvalidSignature => write!(f, "Invalid signature"),
            Self::InvalidProof => write!(f, "Invalid proof"),
            Self::StateRootMismatch => write!(f, "State root mismatch"),
            Self::GasLimitExceeded => write!(f, "Gas limit exceeded"),
            Self::InsufficientFunds => write!(f, "Insufficient funds"),
            Self::NonceMismatch => write!(f, "Nonce mismatch"),
            Self::ChainNotFound => write!(f, "Chain not found"),
            Self::BlockNotFound => write!(f, "Block not found"),
            Self::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            Self::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// Consensus error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsensusError {
    /// Invalid proof of work
    InvalidProofOfWork,

    /// Invalid proof of stake
    InvalidProofOfStake,

    /// Difficulty mismatch
    DifficultyMismatch,

    /// Timestamp invalid
    InvalidTimestamp,

    /// Block number mismatch
    InvalidBlockNumber,

    /// Unknown uncle
    UnknownUncle,

    /// Too many uncles
    TooManyUncles,

    /// Uncles not sorted
    UnclesNotSorted,
}

impl fmt::Display for ConsensusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProofOfWork => write!(f, "Invalid proof of work"),
            Self::InvalidProofOfStake => write!(f, "Invalid proof of stake"),
            Self::DifficultyMismatch => write!(f, "Difficulty mismatch"),
            Self::InvalidTimestamp => write!(f, "Invalid timestamp"),
            Self::InvalidBlockNumber => write!(f, "Invalid block number"),
            Self::UnknownUncle => write!(f, "Unknown uncle"),
            Self::TooManyUncles => write!(f, "Too many uncles"),
            Self::UnclesNotSorted => write!(f, "Uncles not sorted"),
        }
    }
}

/// EVM error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EVMError {
    /// Stack overflow
    StackOverflow,

    /// Stack underflow
    StackUnderflow,

    /// Invalid jump destination
    InvalidJump,

    /// Out of gas
    OutOfGas,

    /// Invalid opcode
    InvalidOpcode(u8),

    /// Invalid memory access
    InvalidMemoryAccess,

    /// Revert instruction
    Revert(Vec<u8>),

    /// Invalid input size
    InvalidInputSize,

    /// Division by zero
    DivisionByZero,

    /// Invalid precompiled contract
    InvalidPrecompiledContract,

    /// Call depth exceeded
    CallDepthExceeded,

    /// Contract creation failed
    ContractCreationFailed,
}

impl fmt::Display for EVMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StackOverflow => write!(f, "Stack overflow"),
            Self::StackUnderflow => write!(f, "Stack underflow"),
            Self::InvalidJump => write!(f, "Invalid jump destination"),
            Self::OutOfGas => write!(f, "Out of gas"),
            Self::InvalidOpcode(op) => write!(f, "Invalid opcode: 0x{:02x}", op),
            Self::InvalidMemoryAccess => write!(f, "Invalid memory access"),
            Self::Revert(data) => write!(f, "Revert: {:?}", data),
            Self::InvalidInputSize => write!(f, "Invalid input size"),
            Self::DivisionByZero => write!(f, "Division by zero"),
            Self::InvalidPrecompiledContract => write!(f, "Invalid precompiled contract"),
            Self::CallDepthExceeded => write!(f, "Call depth exceeded"),
            Self::ContractCreationFailed => write!(f, "Contract creation failed"),
        }
    }
}

/// Contract error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractError {
    /// Contract not found
    ContractNotFound,

    /// Invalid ABI
    InvalidABI(String),

    /// ABI encoding error
    ABIEncodeError,

    /// ABI decoding error
    ABIDecodeError,

    /// Method not found
    MethodNotFound,

    /// Event not found
    EventNotFound,

    /// Invalid contract code
    InvalidContractCode,

    /// Deployment failed
    DeploymentFailed(String),

    /// Call failed
    CallFailed(String),

    /// Transfer failed
    TransferFailed,

    /// Insufficient funds
    InsufficientFunds,
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContractNotFound => write!(f, "Contract not found"),
            Self::InvalidABI(msg) => write!(f, "Invalid ABI: {}", msg),
            Self::ABIEncodeError => write!(f, "ABI encoding error"),
            Self::ABIDecodeError => write!(f, "ABI decoding error"),
            Self::MethodNotFound => write!(f, "Method not found"),
            Self::EventNotFound => write!(f, "Event not found"),
            Self::InvalidContractCode => write!(f, "Invalid contract code"),
            Self::DeploymentFailed(msg) => write!(f, "Deployment failed: {}", msg),
            Self::CallFailed(msg) => write!(f, "Call failed: {}", msg),
            Self::TransferFailed => write!(f, "Transfer failed"),
            Self::InsufficientFunds => write!(f, "Insufficient funds"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = BlockchainError::GasLimitExceeded;
        assert_eq!(format!("{}", err), "Gas limit exceeded");

        let err = EVMError::OutOfGas;
        assert_eq!(format!("{}", err), "Out of gas");
    }
}
