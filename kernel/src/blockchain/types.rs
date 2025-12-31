//! Core types for the blockchain system

use core::fmt;
use alloc::vec::Vec;
use alloc::string::String;

/// 256-bit hash
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct H256(pub [u8; 32]);

impl H256 {
    /// Zero hash
    pub const ZERO: Self = Self([0u8; 32]);

    /// Create from byte array
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create from slice (returns None if slice is not 32 bytes)
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() == 32 {
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(slice);
            Some(Self(bytes))
        } else {
            None
        }
    }

    /// Convert to byte array
    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Check if zero
    pub const fn is_zero(&self) -> bool {
        // Manually check each byte instead of using iter().all()
        let mut i = 0;
        while i < 32 {
            if self.0[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }
}

impl Default for H256 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl AsRef<[u8]> for H256 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::LowerHex for H256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

/// 256-bit unsigned integer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct U256(pub [u64; 4]);

impl U256 {
    /// Zero
    pub const ZERO: Self = Self([0, 0, 0, 0]);

    /// One
    pub const ONE: Self = Self([1, 0, 0, 0]);

    /// Maximum value
    pub const MAX: Self = Self([u64::MAX, u64::MAX, u64::MAX, u64::MAX]);

    /// Create from u64
    pub const fn from(value: u64) -> Self {
        Self([value, 0, 0, 0])
    }

    /// Create from array
    pub const fn from_array(arr: [u64; 4]) -> Self {
        Self(arr)
    }

    /// Convert to bytes (big-endian)
    pub fn to_bytes_be(self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        for i in 0..4 {
            let start = (3 - i) * 8;
            bytes[start..start + 8].copy_from_slice(&self.0[i].to_be_bytes());
        }
        bytes
    }

    /// Create from bytes (big-endian)
    pub fn from_bytes_be(bytes: [u8; 32]) -> Self {
        let mut limbs = [0u64; 4];
        for i in 0..4 {
            let start = i * 8;
            limbs[3 - i] = u64::from_be_bytes([
                bytes[start],
                bytes[start + 1],
                bytes[start + 2],
                bytes[start + 3],
                bytes[start + 4],
                bytes[start + 5],
                bytes[start + 6],
                bytes[start + 7],
            ]);
        }
        Self(limbs)
    }

    /// Check if zero
    pub const fn is_zero(&self) -> bool {
        self.0[0] == 0 && self.0[1] == 0 && self.0[2] == 0 && self.0[3] == 0
    }

    /// Addition with overflow check
    pub fn checked_add(&self, other: &Self) -> Option<Self> {
        let mut result = [0u64; 4];
        let mut carry = 0u64;

        for i in 0..4 {
            let (sum, overflow) = self.0[i].overflowing_add(other.0[i]);
            let (sum2, overflow2) = sum.overflowing_add(carry);
            result[i] = sum2;
            carry = (overflow | overflow2) as u64;
        }

        if carry != 0 {
            return None; // Overflow
        }

        Some(Self(result))
    }

    /// Subtraction with underflow check
    pub fn checked_sub(&self, other: &Self) -> Option<Self> {
        let mut result = [0u64; 4];
        let mut borrow = 0u64;

        for i in 0..4 {
            let (diff, overflow) = self.0[i].overflowing_sub(other.0[i]);
            let (diff2, overflow2) = diff.overflowing_sub(borrow);
            result[i] = diff2;
            borrow = (overflow | overflow2) as u64;
        }

        if borrow != 0 {
            return None; // Underflow
        }

        Some(Self(result))
    }

    /// Multiplication
    pub fn mul(&self, other: &Self) -> Self {
        let mut result = U512::ZERO;

        for i in 0..4 {
            let mut carry = 0u128;
            for j in 0..4 {
                if i + j >= 8 {
                    continue;
                }
                let product = (self.0[i] as u128) * (other.0[j] as u128);
                let sum = product + carry + result.0[i + j] as u128;
                result.0[i + j] = sum as u64;
                carry = sum >> 64;
            }
        }

        // Take lower 256 bits
        Self([result.0[0], result.0[1], result.0[2], result.0[3]])
    }

    /// Division
    pub fn div(&self, divisor: &Self) -> Self {
        if divisor.is_zero() {
            panic!("Division by zero");
        }

        // Simple division algorithm
        let mut quotient = Self::ZERO;
        let mut remainder = Self::ZERO;

        for i in (0..256).rev() {
            // Shift remainder left by 1
            remainder = remainder.shl(1);

            // Add bit i of dividend to remainder
            let word_idx = 3 - (i / 64);
            let bit_idx = i % 64;
            if (self.0[word_idx] >> bit_idx) & 1 == 1 {
                remainder.0[word_idx] |= 1 << bit_idx;
            }

            // If remainder >= divisor, subtract and set quotient bit
            if remainder.ge(divisor) {
                remainder = remainder.sub(divisor);
                let q_word_idx = 3 - (i / 64);
                let q_bit_idx = i % 64;
                quotient.0[q_word_idx] |= 1 << q_bit_idx;
            }
        }

        quotient
    }

    // Helper: shift left
    fn shl(&self, bits: usize) -> Self {
        if bits >= 256 {
            return Self::ZERO;
        }

        let word_shift = bits / 64;
        let bit_shift = bits % 64;

        let mut result = [0u64; 4];

        for i in 0..4 {
            if i + word_shift < 4 {
                result[i + word_shift] |= self.0[i] << bit_shift;
            }
            if bit_shift > 0 && i + word_shift + 1 < 4 {
                result[i + word_shift + 1] |= self.0[i] >> (64 - bit_shift);
            }
        }

        Self(result)
    }

    // Helper: greater than or equal
    fn ge(&self, other: &Self) -> bool {
        for i in (0..4).rev() {
            if self.0[i] > other.0[i] {
                return true;
            }
            if self.0[i] < other.0[i] {
                return false;
            }
        }
        true
    }

    // Helper: subtraction (no underflow check)
    fn sub(&self, other: &Self) -> Self {
        let mut result = [0u64; 4];
        let mut borrow = 0u64;

        for i in 0..4 {
            let (diff, overflow) = self.0[i].overflowing_sub(other.0[i]);
            let (diff2, overflow2) = diff.overflowing_sub(borrow);
            result[i] = diff2;
            borrow = (overflow | overflow2) as u64;
        }

        Self(result)
    }

    /// Convert to usize (with truncation)
    pub fn as_usize(&self) -> usize {
        self.0[0] as usize
    }
}

impl Default for U256 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<u64> for U256 {
    fn from(value: u64) -> Self {
        Self::from(value)
    }
}

impl From<u32> for U256 {
    fn from(value: u32) -> Self {
        Self([value as u64, 0, 0, 0])
    }
}

impl PartialOrd for U256 {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for U256 {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        for i in (0..4).rev() {
            if self.0[i] != other.0[i] {
                return self.0[i].cmp(&other.0[i]);
            }
        }
        core::cmp::Ordering::Equal
    }
}

/// 512-bit unsigned integer (for intermediate calculations)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct U512(pub [u64; 8]);

impl U512 {
    /// Zero
    pub const ZERO: Self = Self([0u64; 8]);
}

/// Bloom filter for logs (2048 bits)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bloom {
    data: [u8; 256],
}

impl Bloom {
    /// Create empty bloom filter
    pub fn new() -> Self {
        Self { data: [0u8; 256] }
    }

    /// Add data to bloom filter
    pub fn add(&mut self, hash: &H256) {
        // Use three hash functions
        for i in 0..3 {
            let idx = ((hash.0[i * 2] as usize) << 8) | (hash.0[i * 2 + 1] as usize);
            let bit = idx % 2048;
            self.data[bit / 8] |= 1 << (bit % 8);
        }
    }

    /// Check if bloom filter possibly contains data (may have false positives)
    pub fn contains(&self, hash: &H256) -> bool {
        for i in 0..3 {
            let idx = ((hash.0[i * 2] as usize) << 8) | (hash.0[i * 2 + 1] as usize);
            let bit = idx % 2048;
            if (self.data[bit / 8] & (1 << (bit % 8))) == 0 {
                return false;
            }
        }
        true
    }
}

impl Default for Bloom {
    fn default() -> Self {
        Self::new()
    }
}

/// Block header
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// Parent block hash
    pub parent_hash: H256,

    /// Uncle hash
    pub uncles_hash: H256,

    /// Coinbase address (beneficiary of mining reward)
    pub coinbase: Address,

    /// State root hash (Merkle Patricia Trie)
    pub state_root: H256,

    /// Transaction root hash
    pub transactions_root: H256,

    /// Receipt root hash
    pub receipts_root: H256,

    /// Bloom filter for logs
    pub logs_bloom: Bloom,

    /// Difficulty for this block
    pub difficulty: U256,

    /// Block number
    pub number: u64,

    /// Gas limit
    pub gas_limit: u64,

    /// Gas used
    pub gas_used: u64,

    /// Timestamp
    pub timestamp: u64,

    /// Extra data
    pub extra_data: Vec<u8>,

    /// Mix hash (for PoW)
    pub mix_hash: H256,

    /// Nonce (for PoW)
    pub nonce: u64,

    /// Base fee per gas (EIP-1559)
    pub base_fee_per_gas: Option<U256>,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            parent_hash: H256::ZERO,
            uncles_hash: H256::ZERO,
            coinbase: Address::ZERO,
            state_root: H256::ZERO,
            transactions_root: H256::ZERO,
            receipts_root: H256::ZERO,
            logs_bloom: Bloom::default(),
            difficulty: U256::ZERO,
            number: 0,
            gas_limit: 30_000_000,
            gas_used: 0,
            timestamp: 0,
            extra_data: Vec::new(),
            mix_hash: H256::ZERO,
            nonce: 0,
            base_fee_per_gas: None,
        }
    }
}

/// Address (20 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Address(pub [u8; 20]);

impl Address {
    /// Zero address
    pub const ZERO: Self = Self([0u8; 20]);

    /// Create from byte array
    pub const fn new(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    /// Create from H256 (takes last 20 bytes)
    pub fn from_h256(hash: H256) -> Self {
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&hash.0[12..]);
        Self(addr)
    }

    /// Convert to H256 (padded with zeros)
    pub fn to_h256(self) -> H256 {
        let mut hash = [0u8; 32];
        hash[12..].copy_from_slice(&self.0);
        H256(hash)
    }
}

impl Default for Address {
    fn default() -> Self {
        Self::ZERO
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Transaction
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    /// Nonce
    pub nonce: U256,

    /// Gas price (or max fee per gas for EIP-1559)
    pub gas_price: U256,

    /// Gas limit
    pub gas_limit: u64,

    /// Recipient address (None for contract creation)
    pub to: Option<Address>,

    /// Value to transfer
    pub value: U256,

    /// Input data
    pub data: Vec<u8>,

    /// Signature v
    pub v: u64,

    /// Signature r
    pub r: U256,

    /// Signature s
    pub s: U256,

    /// Chain ID (for EIP-155)
    pub chain_id: Option<u64>,
}

impl Transaction {
    /// Get sender address from signature
    pub fn sender(&self) -> Result<Address, String> {
        // This is a simplified version - real implementation uses ECDSA recovery
        Ok(Address::ZERO) // Placeholder
    }

    /// Compute transaction hash
    pub fn hash(&self) -> H256 {
        // Simplified - real implementation uses RLP encoding
        H256::ZERO
    }
}

/// Block
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// Header
    pub header: Header,

    /// Transactions
    pub transactions: Vec<Transaction>,

    /// Uncle blocks
    pub uncles: Vec<Header>,
}

/// Receipt
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    /// Whether transaction was successful
    pub success: bool,

    /// Gas used
    pub gas_used: u64,

    /// Log bloom
    pub bloom: Bloom,

    /// Logs
    pub logs: Vec<Log>,
}

/// Log entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Log {
    /// Contract address that emitted the log
    pub address: Address,

    /// Topics (up to 4)
    pub topics: Vec<H256>,

    /// Data
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u256_addition() {
        let a = U256::from(100);
        let b = U256::from(50);
        let sum = a.checked_add(&b).unwrap();
        assert_eq!(sum.0[0], 150);
    }

    #[test]
    fn test_u256_overflow() {
        let a = U256::MAX;
        let b = U256::ONE;
        assert!(a.checked_add(&b).is_none());
    }

    #[test]
    fn test_h256_from_slice() {
        let bytes = [1u8; 32];
        let hash = H256::from_slice(&bytes).unwrap();
        assert_eq!(hash.0, bytes);
    }

    #[test]
    fn test_bloom_filter() {
        let mut bloom = Bloom::new();
        let hash = H256::new([1u8; 32]);

        bloom.add(&hash);
        assert!(bloom.contains(&hash));

        let hash2 = H256::new([2u8; 32]);
        assert!(!bloom.contains(&hash2));
    }
}
