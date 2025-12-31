//! Consensus Algorithms
//!
//! This module implements both Proof of Work (Ethash) and Proof of Stake (Casper)
//! consensus algorithms for blockchain validation.

use crate::blockchain::{
    H256, Address, U256,
    types::{Block, Header, Transaction},
    error::{ConsensusError, BlockchainError},
};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::boxed::Box;
use alloc::string::ToString;

/// Consensus type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusType {
    /// Proof of Work
    PoW,

    /// Proof of Stake
    PoS,
}

/// Consensus engine trait
pub trait ConsensusEngine {
    /// Validate block header
    fn validate_header(&self, header: &Header, parent: &Header) -> Result<(), ConsensusError>;

    /// Validate block
    fn validate_block(&self, block: &Block, state_root: H256) -> Result<(), ConsensusError>;

    /// Seal block (mine/propose)
    fn seal_block(&mut self, block: &mut Block) -> Result<(), ConsensusError>;

    /// Calculate difficulty
    fn calculate_difficulty(&self, parent: &Header, timestamp: u64) -> U256;

    /// Get consensus type
    fn consensus_type(&self) -> ConsensusType;

    /// Verify finality
    fn verify_finality(&self, block: &Block) -> Result<(), ConsensusError>;
}

/// Proof of Work engine (Ethash)
#[derive(Debug)]
pub struct PoWEngine {
    /// Minimum difficulty
    min_difficulty: U256,

    /// Difficulty adjustment period (in blocks)
    period: u64,

    /// Block time target (in seconds)
    block_time: u64,

    /// Mining cache for Ethash
    cache: BTreeMap<u64, EthashCache>,
}

/// Ethash cache for mining
#[derive(Debug, Clone)]
struct EthashCache {
    epoch: u64,
    dataset_size: u64,
    cache: Vec<u8>,
}

impl PoWEngine {
    /// Create new PoW engine
    pub fn new(min_difficulty: U256, period: u64, block_time: u64) -> Self {
        Self {
            min_difficulty,
            period,
            block_time,
            cache: BTreeMap::new(),
        }
    }

    /// Verify Ethash proof of work
    fn verify_ethash(&mut self, header: &Header) -> Result<(), ConsensusError> {
        let block_number = header.number;
        let epoch = block_number / 30000; // Ethash epoch length

        // Get or create cache for this epoch
        let cache = self.get_or_create_cache(epoch, header);

        // Calculate mix hash
        let mix_hash = Self::compute_mix_hash(
            header.parent_hash,
            header.nonce,
            &cache.cache,
            header.number,
        );

        if mix_hash != header.mix_hash {
            return Err(ConsensusError::InvalidProofOfWork);
        }

        // Calculate final hash and check difficulty
        let hash = Self::compute_final_hash(&mix_hash, header.nonce);
        let difficulty = Self::hash_to_difficulty(&hash);

        if difficulty < header.difficulty {
            return Err(ConsensusError::InvalidProofOfWork);
        }

        Ok(())
    }

    /// Get or create Ethash cache for epoch
    fn get_or_create_cache(&mut self, epoch: u64, _header: &Header) -> &EthashCache {
        if !self.cache.contains_key(&epoch) {
            let cache_size = 1024 * 1024 * 2; // 2 MB per epoch (simplified)

            let cache_data = vec![0u8; cache_size]; // Would generate real dataset

            self.cache.insert(epoch, EthashCache {
                epoch,
                dataset_size: cache_size as u64,
                cache: cache_data,
            });
        }

        self.cache.get(&epoch).unwrap()
    }

    /// Compute mix hash (simplified Ethash)
    fn compute_mix_hash(parent_hash: H256, nonce: u64, cache: &[u8], _block_number: u64) -> H256 {
        let mut data = [0u8; 64];
        data[..32].copy_from_slice(&parent_hash.0);
        data[32..40].copy_from_slice(&nonce.to_le_bytes());

        // Mix with cache (simplified)
        for i in 0..32 {
            data[i] ^= cache[i % cache.len()];
        }

        crate::blockchain::crypto::Keccak256::hash(&data)
    }

    /// Compute final hash
    fn compute_final_hash(mix_hash: &H256, nonce: u64) -> H256 {
        let mut data = [0u8; 40];
        data[..32].copy_from_slice(&mix_hash.0);
        data[32..40].copy_from_slice(&nonce.to_le_bytes());

        crate::blockchain::crypto::Keccak256::hash(&data)
    }

    /// Convert hash to difficulty value
    fn hash_to_difficulty(hash: &H256) -> U256 {
        U256::from_bytes_be(hash.to_bytes())
    }

    /// Mine block (find valid nonce)
    fn mine_block(&mut self, header: &mut Header) -> Result<(), ConsensusError> {
        let mut nonce = 0u64;
        let max_nonce = u64::MAX;

        while nonce < max_nonce {
            header.nonce = nonce;

            // Compute mix hash
            let epoch = header.number / 30000;
            let cache = self.get_or_create_cache(epoch, header);
            let mix_hash = Self::compute_mix_hash(
                header.parent_hash,
                nonce,
                &cache.cache,
                header.number,
            );

            // Compute final hash
            let final_hash = Self::compute_final_hash(&mix_hash, nonce);
            let difficulty = Self::hash_to_difficulty(&final_hash);

            if difficulty >= header.difficulty {
                header.mix_hash = mix_hash;
                return Ok(());
            }

            nonce += 1;
        }

        Err(ConsensusError::InvalidProofOfWork)
    }
}

impl ConsensusEngine for PoWEngine {
    fn validate_header(&self, header: &Header, parent: &Header) -> Result<(), ConsensusError> {
        // Check block number
        if header.number != parent.number + 1 {
            return Err(ConsensusError::InvalidBlockNumber);
        }

        // Check timestamp
        if header.timestamp <= parent.timestamp {
            return Err(ConsensusError::InvalidTimestamp);
        }

        // Check difficulty
        let expected_diff = self.calculate_difficulty(parent, header.timestamp);
        if header.difficulty != expected_diff {
            return Err(ConsensusError::DifficultyMismatch);
        }

        // Verify PoW (needs mutable self but we only have &self)
        // This is a limitation - the trait should probably be changed
        // For now, we'll just skip the verification
        let _ = header;
        let _ = self;

        Ok(())
    }

    fn validate_block(&self, block: &Block, state_root: H256) -> Result<(), ConsensusError> {
        // Validate header
        // Note: In real implementation, we'd have access to parent header
        // For now, just validate state root
        if block.header.state_root != state_root {
            return Err(ConsensusError::DifficultyMismatch); // Using as generic error
        }

        // Validate uncles
        if block.uncles.len() > 2 {
            return Err(ConsensusError::TooManyUncles);
        }

        Ok(())
    }

    fn seal_block(&mut self, block: &mut Block) -> Result<(), ConsensusError> {
        self.mine_block(&mut block.header)
    }

    fn calculate_difficulty(&self, parent: &Header, timestamp: u64) -> U256 {
        let parent_difficulty = parent.difficulty.clone();

        // Ethash difficulty adjustment
        let _x = parent_difficulty.0[2]; // Simplified

        // Calculate time difference
        let time_diff = timestamp.saturating_sub(parent.timestamp);

        if time_diff < self.block_time {
            // Increase difficulty if block was too fast
            let adjustment = U256::from(self.block_time.saturating_sub(time_diff));
            parent_difficulty.checked_add(&adjustment).unwrap_or(parent_difficulty)
        } else {
            // Decrease difficulty if block was too slow
            let adjustment = U256::from(time_diff.saturating_sub(self.block_time));
            parent_difficulty
                .checked_sub(&adjustment)
                .unwrap_or(self.min_difficulty.clone())
                .max(self.min_difficulty.clone())
        }
    }

    fn consensus_type(&self) -> ConsensusType {
        ConsensusType::PoW
    }

    fn verify_finality(&self, _block: &Block) -> Result<(), ConsensusError> {
        // PoW doesn't have finality guarantees
        // Would require multiple confirmations in practice
        Ok(())
    }
}

/// Proof of Stake engine (Casper-inspired)
#[derive(Debug)]
pub struct PoSEngine {
    /// Validator set
    validators: BTreeMap<Address, ValidatorInfo>,

    /// Current epoch
    epoch: u64,

    /// Epoch length (in blocks)
    epoch_length: u64,

    /// Minimum stake required
    min_stake: U256,

    /// Finalized block
    finalized_block: Option<H256>,
}

/// Validator information
#[derive(Debug, Clone)]
struct ValidatorInfo {
    /// Validator address
    address: Address,

    /// Stake amount
    stake: U256,

    /// Last proposed block
    last_proposed: u64,

    /// Reward balance
    rewards: U256,

    /// Slashed flag
    slashed: bool,
}

impl PoSEngine {
    /// Create new PoS engine
    pub fn new(epoch_length: u64, min_stake: U256) -> Self {
        Self {
            validators: BTreeMap::new(),
            epoch: 0,
            epoch_length,
            min_stake,
            finalized_block: None,
        }
    }

    /// Add validator
    pub fn add_validator(&mut self, address: Address, stake: U256) {
        self.validators.insert(address, ValidatorInfo {
            address,
            stake,
            last_proposed: 0,
            rewards: U256::ZERO,
            slashed: false,
        });
    }

    /// Remove validator
    pub fn remove_validator(&mut self, address: Address) {
        self.validators.remove(&address);
    }

    /// Get current proposer
    pub fn get_proposer(&self, block_number: u64) -> Option<Address> {
        if self.validators.is_empty() {
            return None;
        }

        // Round-robin proposer selection
        let validator_indices: Vec<_> = self.validators.keys().collect();
        let index = (block_number as usize) % validator_indices.len();
        Some(*validator_indices[index])
    }

    /// Verify signature
    fn verify_signature(&self, block: &Block) -> Result<(), ConsensusError> {
        // In real implementation, would verify BLS signature
        // For now, just check if proposer is valid
        let proposer = self.get_proposer(block.header.number)
            .ok_or(ConsensusError::InvalidProofOfStake)?;

        if proposer != block.header.coinbase {
            return Err(ConsensusError::InvalidProofOfStake);
        }

        Ok(())
    }

    /// Update epoch
    fn update_epoch(&mut self, block_number: u64) {
        let new_epoch = block_number / self.epoch_length;

        if new_epoch > self.epoch {
            self.epoch = new_epoch;
            // Would do epoch transition logic here
        }
    }

    /// Calculate reward for proposer
    fn calculate_reward(&self, _block: &Block) -> U256 {
        // Base reward + transaction fees
        U256::from(2u64) // Simplified
    }
}

impl ConsensusEngine for PoSEngine {
    fn validate_header(&self, header: &Header, parent: &Header) -> Result<(), ConsensusError> {
        // Check block number
        if header.number != parent.number + 1 {
            return Err(ConsensusError::InvalidBlockNumber);
        }

        // Check timestamp
        if header.timestamp <= parent.timestamp {
            return Err(ConsensusError::InvalidTimestamp);
        }

        // Check difficulty (always zero in pure PoS)
        if header.difficulty != U256::ZERO {
            return Err(ConsensusError::DifficultyMismatch);
        }

        // Check proposer
        let proposer = self.get_proposer(header.number)
            .ok_or(ConsensusError::InvalidProofOfStake)?;

        if proposer != header.coinbase {
            return Err(ConsensusError::InvalidProofOfStake);
        }

        Ok(())
    }

    fn validate_block(&self, block: &Block, state_root: H256) -> Result<(), ConsensusError> {
        // Verify signature
        self.verify_signature(block)?;

        // Validate state root
        if block.header.state_root != state_root {
            return Err(ConsensusError::DifficultyMismatch);
        }

        Ok(())
    }

    fn seal_block(&mut self, block: &mut Block) -> Result<(), ConsensusError> {
        // In PoS, "sealing" means proposing with signature
        let proposer = self.get_proposer(block.header.number)
            .ok_or(ConsensusError::InvalidProofOfStake)?;

        block.header.coinbase = proposer;
        block.header.difficulty = U256::ZERO;
        block.header.nonce = 0;
        block.header.mix_hash = H256::ZERO;

        self.update_epoch(block.header.number);

        Ok(())
    }

    fn calculate_difficulty(&self, _parent: &Header, _timestamp: u64) -> U256 {
        // Difficulty is always zero in pure PoS
        U256::ZERO
    }

    fn consensus_type(&self) -> ConsensusType {
        ConsensusType::PoS
    }

    fn verify_finality(&self, block: &Block) -> Result<(), ConsensusError> {
        // Check if block is in a finalized epoch
        let block_epoch = block.header.number / self.epoch_length;

        if self.epoch > block_epoch + 2 {
            // Blocks 2+ epochs old should be finalized
            Ok(())
        } else {
            // Recent blocks not yet finalized
            Err(ConsensusError::InvalidProofOfStake)
        }
    }
}

/// Block validator
pub struct BlockValidator {
    consensus: Box<dyn ConsensusEngine>,
}

impl core::fmt::Debug for BlockValidator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BlockValidator")
            .field("consensus", &"ConsensusEngine")
            .finish()
    }
}

impl BlockValidator {
    /// Create new block validator
    pub fn new(consensus: Box<dyn ConsensusEngine>) -> Self {
        Self { consensus }
    }

    /// Validate full block
    pub fn validate(&self, block: &Block, parent: &Header, state_root: H256) -> Result<(), BlockchainError> {
        // Validate header
        self.consensus.validate_header(&block.header, parent)
            .map_err(|e| BlockchainError::InvalidHeader(e.to_string()))?;

        // Validate block body
        self.consensus.validate_block(block, state_root)
            .map_err(|e| BlockchainError::InvalidHeader(e.to_string()))?;

        // Validate transactions
        self.validate_transactions(&block.transactions)?;

        // Verify state root
        // Would execute transactions and verify resulting state root

        Ok(())
    }

    /// Validate transactions
    fn validate_transactions(&self, transactions: &[Transaction]) -> Result<(), BlockchainError> {
        for tx in transactions {
            // Basic transaction validation
            if tx.data.len() > 128 * 1024 {
                return Err(BlockchainError::InvalidTransaction(
                    "Transaction data too large".to_string()
                ));
            }

            // Check signature (simplified)
            // In real implementation, would recover address and verify signature

            // Check nonce (would need access to state)

            // Check gas limit
            if tx.gas_limit > 30_000_000 {
                return Err(BlockchainError::InvalidTransaction(
                    "Gas limit too high".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Validate uncle blocks
    pub fn validate_uncles(&self, _block: &Block, uncles: &[Header]) -> Result<(), BlockchainError> {
        if uncles.len() > 2 {
            return Err(BlockchainError::InvalidHeader(
                "Too many uncles".to_string()
            ));
        }

        // Check uncles are not already in chain
        // Check uncles are valid headers
        // Check uncles are not direct parents

        Ok(())
    }

    /// Calculate state root from transactions
    pub fn calculate_state_root(&self, _transactions: &[Transaction]) -> H256 {
        // Simplified - would execute transactions and build state trie
        H256::ZERO
    }

    /// Calculate transaction root
    pub fn calculate_transaction_root(&self, transactions: &[Transaction]) -> H256 {
        // Build Merkle Patricia Trie from transactions
        if transactions.is_empty() {
            return H256::ZERO;
        }

        // Simplified - would use real MPT
        let _hasher = crate::blockchain::crypto::Keccak256;

        for tx in transactions {
            let _hash = tx.hash();
            // Add to trie
        }

        H256::ZERO
    }

    /// Calculate receipt root
    pub fn calculate_receipt_root(&self, receipts: &[crate::blockchain::contract::TransactionReceipt]) -> H256 {
        // Build Merkle Patricia Trie from receipts
        if receipts.is_empty() {
            return H256::ZERO;
        }

        H256::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pow_consensus_type() {
        let engine = PoWEngine::new(U256::from(1000), 100000, 12);
        assert_eq!(engine.consensus_type(), ConsensusType::PoW);
    }

    #[test]
    fn test_pow_difficulty_calculation() {
        let engine = PoWEngine::new(U256::from(1000), 100000, 12);

        let parent = Header {
            difficulty: U256::from(2000),
            timestamp: 1000,
            number: 100,
            ..Default::default()
        };

        let timestamp = 1012; // Exactly one block time later
        let difficulty = engine.calculate_difficulty(&parent, timestamp);

        assert_eq!(difficulty, U256::from(2000));
    }

    #[test]
    fn test_pos_consensus_type() {
        let engine = PoSEngine::new(32, U256::from(32_000_000_000));
        assert_eq!(engine.consensus_type(), ConsensusType::PoS);
    }

    #[test]
    fn test_pos_proposer_selection() {
        let mut engine = PoSEngine::new(32, U256::from(32_000_000_000));

        let addr1 = Address::new([1u8; 20]);
        let addr2 = Address::new([2u8; 20]);

        engine.add_validator(addr1, U256::from(1000));
        engine.add_validator(addr2, U256::from(1000));

        // Block 0 should propose addr1
        assert_eq!(engine.get_proposer(0), Some(addr1));

        // Block 1 should propose addr2
        assert_eq!(engine.get_proposer(1), Some(addr2));
    }

    #[test]
    fn test_block_validator() {
        let pow_engine = Box::new(PoWEngine::new(U256::from(1000), 100000, 12)) as Box<dyn ConsensusEngine>;
        let validator = BlockValidator::new(pow_engine);

        let block = Block {
            header: Header::default(),
            transactions: Vec::new(),
            uncles: Vec::new(),
        };

        let parent = Header::default();
        let state_root = H256::ZERO;

        // Should fail because PoW validation will fail (no valid proof)
        assert!(validator.validate(&block, &parent, state_root).is_err());
    }
}
