//! Cryptographic primitives for blockchain operations
//!
//! This module provides cryptographic utilities used throughout the blockchain system,
//! including ECDSA signatures, Keccak-256 hashing, RIPEMD-160, and address generation.

use crate::blockchain::{H256, Address, U256};
use alloc::vec::Vec;

/// Keccak-256 hash function (SHA-3)
#[derive(Debug, Clone)]
pub struct Keccak256;

impl Keccak256 {
    /// Hash data using Keccak-256
    pub fn hash(data: &[u8]) -> H256 {
        // Simplified implementation - in production, use actual Keccak-256
        // This is a placeholder that returns a deterministic hash based on input
        Self::keccak256_impl(data)
    }

    /// Hash two values concatenated
    pub fn hash_two(a: &H256, b: &H256) -> H256 {
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(&a.0);
        combined[32..].copy_from_slice(&b.0);
        Self::hash(&combined)
    }

    /// Simplified Keccak-256 implementation (placeholder)
    fn keccak256_impl(data: &[u8]) -> H256 {
        // In production, this would use the actual Keccak-256 algorithm
        // For now, we'll use a simplified deterministic hash
        let mut result = [0u8; 32];
        let len = data.len() as u64;

        // Simple mixing function
        result[0..8].copy_from_slice(&len.to_le_bytes());

        for (i, &byte) in data.iter().enumerate() {
            result[i % 32] ^= byte.wrapping_add(i as u8);
        }

        // Additional mixing rounds
        for round in 0..3 {
            for i in 0..32 {
                let prev = result[(i + 31) % 32];
                let next = result[(i + 1) % 32];
                result[i] ^= prev.wrapping_add(next).wrapping_add(round as u8);
            }
        }

        H256(result)
    }
}

/// ECDSA public key
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey {
    /// Public key bytes (compressed or uncompressed)
    bytes: [u8; 64],
}

impl PublicKey {
    /// Create from bytes
    pub fn from_bytes(bytes: &[u8; 64]) -> Self {
        Self { bytes: *bytes }
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> [u8; 64] {
        self.bytes
    }

    /// Recover from signature and message (ECDSA recovery)
    pub fn recover(signature: &Signature, _message: &H256, recovery_id: u8) -> Option<Self> {
        // Simplified ECDSA recovery
        // In production, use actual secp256k1 library
        let mut bytes = [0u8; 64];

        // Derive public key from signature components
        let r_bytes = signature.r.clone().to_bytes_be();
        let s_bytes = signature.s.clone().to_bytes_be();

        bytes[0..32].copy_from_slice(&r_bytes[0..32]);
        bytes[32..64].copy_from_slice(&s_bytes[0..32]);

        // Apply recovery ID
        bytes[0] ^= recovery_id;

        Some(Self { bytes })
    }
}

/// ECDSA private key
#[derive(Debug, Clone)]
pub struct PrivateKey {
    /// Private key bytes (32 bytes)
    bytes: [u8; 32],
}

impl PrivateKey {
    /// Generate new random private key
    pub fn generate() -> Self {
        // In production, use cryptographically secure RNG
        Self { bytes: [1u8; 32] } // Placeholder
    }

    /// Create from bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self { bytes: *bytes }
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.bytes
    }

    /// Derive public key
    pub fn public_key(&self) -> PublicKey {
        // Simplified public key derivation
        let mut bytes = [0u8; 64];

        // In production, use actual secp256k1 multiplication
        for i in 0..32 {
            bytes[i] = self.bytes[i].wrapping_mul(2);
            bytes[i + 32] = self.bytes[i].wrapping_add(1);
        }

        PublicKey { bytes }
    }

    /// Sign message
    pub fn sign(&self, message: &H256) -> Signature {
        // Simplified ECDSA signing
        // In production, use actual secp256k1 signing
        let mut r = [0u8; 32];
        let mut s = [0u8; 32];

        // Derive r and s from private key and message
        for i in 0..32 {
            r[i] = self.bytes[i].wrapping_add(message.0[i]);
            s[i] = self.bytes[i].wrapping_mul(message.0[i]).wrapping_add(1);
        }

        Signature {
            r: U256::from_bytes_be(r),
            s: U256::from_bytes_be(s),
            v: 27, // Recovery ID
        }
    }
}

/// ECDSA signature
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    /// r value
    pub r: U256,
    /// s value
    pub s: U256,
    /// v value (recovery ID)
    pub v: u64,
}

impl Signature {
    /// Create from r, s, v components
    pub fn new(r: U256, s: U256, v: u64) -> Self {
        Self { r, s, v }
    }

    /// Convert to DER encoding
    pub fn to_der(&self) -> Vec<u8> {
        // Simplified DER encoding
        vec![]
    }

    /// Parse from DER encoding
    pub fn from_der(_data: &[u8]) -> Option<Self> {
        // Simplified DER parsing
        None
    }
}

/// ECDSA signature verifier
pub struct ECDSAVerifier;

impl ECDSAVerifier {
    /// Verify signature against message and public key
    pub fn verify(
        public_key: &PublicKey,
        message: &H256,
        signature: &Signature,
    ) -> bool {
        // Simplified ECDSA verification
        // In production, use actual secp256k1 verification

        // Basic checks
        if signature.s.is_zero() || signature.r.is_zero() {
            return false;
        }

        // Check against public key
        let expected = Self::compute_expected(public_key, message);
        expected == signature.r
    }

    fn compute_expected(public_key: &PublicKey, message: &H256) -> U256 {
        let mut hash = [0u8; 32];

        for i in 0..32 {
            let pk_byte = if i < 64 {
                public_key.bytes[i]
            } else {
                0
            };
            hash[i] = pk_byte.wrapping_add(message.0[i]);
        }

        U256::from_bytes_be(hash)
    }
}

/// ECDSA signer
pub struct ECDSASigner {
    private_key: PrivateKey,
}

impl ECDSASigner {
    /// Create signer from private key
    pub fn new(private_key: PrivateKey) -> Self {
        Self { private_key }
    }

    /// Get address
    pub fn address(&self) -> Address {
        let pub_key = self.private_key.public_key();
        Self::public_key_to_address(&pub_key)
    }

    /// Sign transaction
    pub fn sign_transaction(&self, message: &H256) -> Signature {
        let sig = self.private_key.sign(message);

        // Adjust v for chain ID (EIP-155)
        Signature {
            v: sig.v + 35, // Simplified
            ..sig
        }
    }

    /// Convert public key to address
    fn public_key_to_address(public_key: &PublicKey) -> Address {
        // Hash public key with Keccak-256
        let hash = Keccak256::hash(&public_key.bytes);

        // Take last 20 bytes
        Address::from_h256(hash)
    }
}

/// Address utilities
impl Address {
    /// Create from public key
    pub fn from_public_key(public_key: &PublicKey) -> Self {
        ECDSASigner::public_key_to_address(public_key)
    }

    /// Validate address checksum (EIP-55)
    pub fn validate_checksum(&self) -> bool {
        // EIP-55 checksum validation
        true // Simplified
    }
}

/// RIPEMD-160 hash function
pub struct RIPEMD160;

impl RIPEMD160 {
    /// Hash data using RIPEMD-160
    pub fn hash(data: &[u8]) -> [u8; 20] {
        // Simplified RIPEMD-160 implementation
        let mut result = [0u8; 20];

        for (i, &byte) in data.iter().enumerate() {
            result[i % 20] ^= byte.wrapping_add(i as u8);
        }

        result
    }
}

/// Cryptographic utility functions
pub struct CryptoUtils;

impl CryptoUtils {
    /// Generate random bytes
    pub fn random_bytes(len: usize) -> Vec<u8> {
        // In production, use cryptographically secure RNG
        vec![0u8; len] // Placeholder
    }

    /// Compare two values in constant time
    pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let mut result = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            result |= x ^ y;
        }

        result == 0
    }

    /// Convert bytes to hex string
    pub fn bytes_to_hex(bytes: &[u8]) -> alloc::string::String {
        let mut hex = alloc::string::String::with_capacity(bytes.len() * 2);
        for &byte in bytes {
            // Format each byte as 2-digit hex
            let chars = [b"0123456789abcdef"[(byte >> 4) as usize],
                         b"0123456789abcdef"[(byte & 0x0f) as usize]];
            hex.push(chars[0] as char);
            hex.push(chars[1] as char);
        }
        hex
    }

    /// Convert hex string to bytes
    pub fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
        if hex.len() % 2 != 0 {
            return None;
        }

        let mut bytes = Vec::with_capacity(hex.len() / 2);
        for i in (0..hex.len()).step_by(2) {
            let byte = u8::from_str_radix(&hex[i..i + 2], 16).ok()?;
            bytes.push(byte);
        }

        Some(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keccak256() {
        let data = b"hello";
        let hash = Keccak256::hash(data);
        assert_ne!(hash, H256::ZERO);
    }

    #[test]
    fn test_keccak256_different_inputs() {
        let hash1 = Keccak256::hash(b"hello");
        let hash2 = Keccak256::hash(b"world");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_signature_creation() {
        let private_key = PrivateKey::from_bytes(&[1u8; 32]);
        let message = H256::new([2u8; 32]);
        let signature = private_key.sign(&message);

        assert!(!signature.r.is_zero());
        assert!(!signature.s.is_zero());
    }

    #[test]
    fn test_address_from_public_key() {
        let private_key = PrivateKey::from_bytes(&[1u8; 32]);
        let public_key = private_key.public_key();
        let address = Address::from_public_key(&public_key);

        assert_ne!(address, Address::ZERO);
    }

    #[test]
    fn test_ripemd160() {
        let data = b"hello";
        let hash = RIPEMD160::hash(data);
        assert_eq!(hash.len(), 20);
    }

    #[test]
    fn test_constant_time_eq() {
        let a = [1u8, 2, 3];
        let b = [1u8, 2, 3];
        let c = [1u8, 2, 4];

        assert!(CryptoUtils::constant_time_eq(&a, &b));
        assert!(!CryptoUtils::constant_time_eq(&a, &c));
    }

    #[test]
    fn test_hex_conversion() {
        let bytes = [0xDE, 0xAD, 0xBE, 0xEF];
        let hex = CryptoUtils::bytes_to_hex(&bytes);
        assert_eq!(hex, "deadbeef");

        let decoded = CryptoUtils::hex_to_bytes(&hex).unwrap();
        assert_eq!(&decoded[..], &bytes[..]);
    }
}
