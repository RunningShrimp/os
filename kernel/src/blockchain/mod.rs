//! Blockchain and Smart Contract Implementation
//!
//! This module provides a complete blockchain system with smart contract support,
//! implementing the Ethereum Virtual Machine (EVM), consensus algorithms, and P2P networking.
//!
//! ## Architecture
//!
//! The blockchain system is organized into several core components:
//!
//! - **EVM (Ethereum Virtual Machine)**: Bytecode execution engine for smart contracts
//! - **Contract Engine**: Smart contract deployment and execution management
//! - **Consensus Layer**: Proof of Work and Proof of Stake implementations
//! - **Merkle Patricia Tree**: Efficient verifiable data structure for state
//! - **Cryptography**: ECDSA signatures, Keccak-256 hashing, and address generation
//! - **P2P Network**: Decentralized peer-to-peer communication protocol
//!
//! ## Security Considerations
//!
//! This module prioritizes correctness and security above all else:
//! - Gas is measured precisely to prevent DoS attacks
//! - State consistency is guaranteed through Merkle proofs
//! - Cryptographic operations use audited primitives
//! - All financial operations are atomic and reversible
//!
//! ## Usage
//!
//! ```rust,no_run
//! use kernel::blockchain::{
//!     evm::EVM,
//!     contract::ContractEngine,
//!     consensus::ConsensusEngine,
//!     mpt::MerklePatriciaTrie,
//! };
//!
//! // Create EVM instance
//! let evm = EVM::new();
//!
//! // Deploy contract
//! let contract_address = evm.deploy(bytecode, caller, gas_limit);
//!
//! // Execute transaction
//! let result = evm.transact(contract_address, input, caller, gas_limit);
//! ```

pub mod crypto;
pub mod mpt;
pub mod evm;
pub mod contract;
pub mod consensus;
pub mod p2p;

pub use crypto::{
    Keccak256,
    ECDSASigner,
    ECDSAVerifier,
    Signature,
    PublicKey,
    PrivateKey,
};

pub use types::Address;

pub use mpt::{
    MerklePatriciaTrie,
    MerkleProof,
    RLPStream,
    RLPItem,
};

pub use evm::{
    EVM,
    ExecutionContext,
    Opcode,
    Gasometer,
    Stack,
    Memory,
};

pub use contract::{
    ContractEngine,
    Contract,
    ContractABI,
    EventLog,
    TransactionReceipt,
};

pub use consensus::{
    ConsensusEngine,
    ConsensusType,
    BlockValidator,
    PoWEngine,
    PoSEngine,
};

pub use p2p::{
    P2PNetwork,
    PeerInfo,
    ProtocolHandler,
    BlockSync,
};

mod types;
mod error;

pub use types::{
    Block,
    Transaction,
    Header,
    Receipt,
    Bloom,
    H256,
    U256,
    U512,
};

pub use error::{
    BlockchainError,
    ConsensusError,
    EVMError,
    ContractError,
};

/// Blockchain configuration parameters
#[derive(Debug, Clone)]
pub struct BlockchainConfig {
    /// Chain ID for EIP-155 replay protection
    pub chain_id: u64,

    /// Maximum block gas limit
    pub max_block_gas: u64,

    /// Minimum gas price for transactions
    pub min_gas_price: U256,

    /// Block time in seconds (for PoS)
    pub block_time: u64,

    /// Difficulty adjustment period (in blocks)
    pub difficulty_period: u64,

    /// Consensus type (PoW or PoS)
    pub consensus_type: ConsensusType,

    /// Maximum transaction size in bytes
    pub max_tx_size: usize,
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            chain_id: 1, // Ethereum mainnet
            max_block_gas: 30_000_000,
            min_gas_price: U256::from(1_000_000_000u64), // 1 Gwei
            block_time: 12,
            difficulty_period: 100000,
            consensus_type: ConsensusType::PoW,
            max_tx_size: 128 * 1024, // 128 KB
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = BlockchainConfig::default();
        assert_eq!(config.chain_id, 1);
        assert_eq!(config.max_block_gas, 30_000_000);
    }

    #[test]
    fn test_blockchain_module_integration() {
        // Test that all core components are accessible
        let _ = Keccak256::hash(b"test");
        let _ = MerklePatriciaTrie::new();
        let _ = Address::ZERO;
        let _ = U256::from(42);
    }
}

// Include comprehensive integration tests
#[path = "tests.rs"]
mod integration_tests;
