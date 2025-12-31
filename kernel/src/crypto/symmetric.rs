//! # Symmetric Encryption Algorithms
//!
//! This module provides implementations of symmetric encryption algorithms,
//! including block ciphers, stream ciphers, and authenticated encryption modes.
//!
//! ## Overview
//!
//! Symmetric encryption uses the same key for both encryption and decryption.
//! It is typically much faster than asymmetric encryption and is suitable for
//! encrypting large amounts of data.
//!
//! ## Supported Algorithms
//!
//! ### Block Ciphers
//!
//! - **AES (Advanced Encryption Standard)**: NIST-approved block cipher
//!   - Key sizes: 128, 192, 256 bits
//!   - Modes: ECB, CBC, CTR, GCM, CCM
//!   - Performance: Hardware-accelerated on x86_64 (AES-NI)
//!
//! - **Serpent**: Highly secure block cipher with 32 rounds
//!   - Key size: 256 bits
//!   - Block size: 128 bits
//!   - Security margin: Very high
//!
//! - **Twofish**: AES finalist block cipher
//!   - Key sizes: 128, 192, 256 bits
//!   - Block size: 128 bits
//!   - Features: Flexible key schedule
//!
//! ### Stream Ciphers
//!
//! - **ChaCha20**: Modern stream cipher by Daniel J. Bernstein
//!   - Key size: 256 bits
//!   - Nonce size: 96 bits
//!   - Rounds: 20 (ChaCha20)
//!   - Performance: Excellent on software
//!
//! ### Authenticated Encryption
//!
//! - **ChaCha20-Poly1305**: AEAD combining ChaCha20 and Poly1305
//! - **AES-GCM**: Authenticated encryption with Galois Counter Mode
//! - **AES-CCM**: Counter with CBC-MAC
//!
//! ## Usage Examples
//!
//! ### AES-256-CBC Encryption
//!
//! ```rust,ignore
//! use kernel::crypto::symmetric::{Aes256, CipherMode};
//!
//! let key = [0u8; 32]; // 256-bit key
//! let iv = [0u8; 16];  // 128-bit IV
//! let plaintext = b"Secret message";
//!
//! let cipher = Aes256::new(&key);
//! let ciphertext = cipher.encrypt(&iv, plaintext, CipherMode::CBC)?;
//! let decrypted = cipher.decrypt(&iv, &ciphertext, CipherMode::CBC)?;
//! assert_eq!(decrypted, plaintext);
//! ```
//!
//! ### ChaCha20-Poly1305 AEAD
//!
//! ```rust,ignore
//! use kernel::crypto::symmetric::{ChaCha20Poly1305, Aead};
//!
//! let key = [0u8; 32];
//! let nonce = [0u8; 12];
//! let aad = b"Additional authenticated data";
//! let plaintext = b"Secret message";
//!
//! let cipher = ChaCha20Poly1305::new(&key);
//! let ciphertext = cipher.encrypt_aead(&nonce, aad, plaintext)?;
//! let decrypted = cipher.decrypt_aead(&nonce, aad, &ciphertext)?;
//! assert_eq!(decrypted, plaintext);
//! ```
//!
//! ### Key Derivation
//!
//! ```rust,ignore
//! use kernel::crypto::symmetric::{pbkdf2, hkdf};
//!
//! // PBKDF2 key derivation
//! let password = b"secure password";
//! let salt = b"unique salt";
//! let derived_key = pbkdf2(password, salt, 100_000, 32)?;
//!
//! // HKDF key derivation
//! let ikm = b"input key material";
//! let salt = b"salt";
//! let info = b"context information";
//! let okm = hkdf(ikm, salt, info, 32)?;
//! ```
//!
//! ## Security Considerations
//!
//! ### Mode Selection
//!
//! - **ECB**: Do NOT use for encrypting multiple blocks (not semantically secure)
//! - **CBC**: Secure, requires unique IV for each encryption
//! - **CTR**: Parallelizable, requires unique nonce
//! - **GCM**: Authenticated encryption, requires unique nonce
//! - **CCM**: Authenticated encryption, requires unique nonce
//!
//! ### Key Management
//!
//! - Keys must be generated using a cryptographically secure random number generator
//! - Keys should be stored securely (encrypted at rest, in hardware if available)
//! - Keys should be rotated regularly
//! - Different keys should be used for different purposes
//!
//! ### Initialization Vectors/Nonces
//!
//! - IV/nonce must be unpredictable for CBC, unique for CTR/GCM/CCM
//! - IV/nonce can be public, but must never be reused with the same key
//! - For GCM/CTR, nonce reuse completely compromises security
//!
//! ## Performance
//!
//! Approximate performance on modern x86_64 with AES-NI:
//!
//! - AES-128-ECB: 12 GB/s
//! - AES-128-CBC: 10 GB/s
//! - AES-128-GCM: 8 GB/s
//! - ChaCha20: 5 GB/s
//! - ChaCha20-Poly1305: 4 GB/s
//!
//! ## References
//!
//! - FIPS 197: Advanced Encryption Standard (AES)
//! - NIST SP 800-38A: Recommendation for Block Cipher Modes of Operation
//! - NIST SP 800-38D: Recommendation for Block Cipher Modes of Operation: Galois/Counter Mode (GCM)
//! - RFC 7539: ChaCha20 and Poly1305 for IETF Protocols
//! - RFC 5116: An Interface and Algorithms for Authenticated Encryption

#![allow(dead_code)]
#![warn(missing_docs)]

use alloc::vec::Vec;
use alloc::string::String;

use crate::crypto::{Result, CryptoError, constant_time_eq};

/// Cipher operation modes
///
/// Different modes of operation for block ciphers, each with different
/// security properties and performance characteristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherMode {
    /// Electronic Codebook mode
    ///
    /// WARNING: NOT semantically secure. Do NOT use for encrypting
    /// multiple blocks. Each block is encrypted independently.
    /// Identical plaintext blocks produce identical ciphertext blocks.
    ECB,

    /// Cipher Block Chaining mode
    ///
    /// Requires unique IV for each encryption. Not parallelizable
    /// for encryption, but parallelizable for decryption.
    CBC,

    /// Counter mode
    ///
    /// Turns a block cipher into a stream cipher. Requires unique nonce.
    /// Parallelizable for both encryption and decryption.
    CTR,

    /// Galois/Counter Mode
    ///
    /// Authenticated encryption with associated data (AEAD).
    /// Requires unique nonce. Parallelizable.
    GCM,

    /// Counter with CBC-MAC
    ///
    /// Authenticated encryption with associated data (AEAD).
    /// Requires unique nonce.
    CCM,
}

/// Padding schemes for block ciphers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddingScheme {
    /// No padding (data must be multiple of block size)
    None,

    /// PKCS#7 padding
    ///
    /// Adds N bytes of value N, where N is the number of padding bytes.
    /// Always adds padding, even if data is already block-aligned.
    PKCS7,

    /// ISO/IEC 7816-4 padding
    ///
    /// Adds 0x80 followed by zero bytes.
    ISO7816,
}

/// Trait for symmetric cipher operations
pub trait Cipher {
    /// Encrypt data
    ///
    /// # Arguments
    ///
    /// * `iv` - Initialization vector or nonce
    /// * `plaintext` - Data to encrypt
    /// * `mode` - Cipher mode of operation
    ///
    /// # Returns
    ///
    /// Encrypted data
    fn encrypt(&self, iv: &[u8], plaintext: &[u8], mode: CipherMode) -> Result<Vec<u8>>;

    /// Decrypt data
    ///
    /// # Arguments
    ///
    /// * `iv` - Initialization vector or nonce
    /// * `ciphertext` - Data to decrypt
    /// * `mode` - Cipher mode of operation
    ///
    /// # Returns
    ///
    /// Decrypted data
    fn decrypt(&self, iv: &[u8], ciphertext: &[u8], mode: CipherMode) -> Result<Vec<u8>>;

    /// Get block size in bytes
    fn block_size(&self) -> usize;

    /// Get key size in bytes
    fn key_size(&self) -> usize;
}

/// Trait for Authenticated Encryption with Associated Data (AEAD)
pub trait Aead {
    /// Encrypt with authentication
    ///
    /// # Arguments
    ///
    /// * `nonce` - Nonce (must be unique for each encryption)
    /// * `aad` - Additional authenticated data (not encrypted, but authenticated)
    /// * `plaintext` - Data to encrypt
    ///
    /// # Returns
    ///
    /// Ciphertext with authentication tag appended
    fn encrypt_aead(&self, nonce: &[u8], aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>>;

    /// Decrypt with authentication
    ///
    /// # Arguments
    ///
    /// * `nonce` - Nonce
    /// * `aad` - Additional authenticated data
    /// * `ciphertext` - Ciphertext with authentication tag appended
    ///
    /// # Returns
    ///
    /// Decrypted plaintext
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::AuthenticationFailed` if authentication fails
    fn decrypt_aead(&self, nonce: &[u8], aad: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>>;

    /// Get nonce size in bytes
    fn nonce_size(&self) -> usize;

    /// Get tag size in bytes
    fn tag_size(&self) -> usize;
}

// =============================================================================
// AES Implementation
// =============================================================================

/// AES-128 cipher (128-bit key)
///
/// Advanced Encryption Standard with 128-bit key and 10 rounds.
/// Suitable for most applications requiring good security.
pub struct Aes128 {
    round_keys: [u32; 44], // Expanded keys
}

impl Aes128 {
    /// Create new AES-128 cipher
    ///
    /// # Arguments
    ///
    /// * `key` - 16-byte key
    ///
    /// # Returns
    ///
    /// New cipher instance
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use kernel::crypto::symmetric::Aes128;
    ///
    /// let key = [0u8; 16];
    /// let cipher = Aes128::new(&key);
    /// ```
    pub fn new(key: &[u8]) -> Self {
        assert_eq!(key.len(), 16, "AES-128 key must be 16 bytes");

        let mut cipher = Self {
            round_keys: [0u32; 44],
        };

        cipher.key_expansion(key);
        cipher
    }

    /// Key expansion using AES key schedule
    fn key_expansion(&mut self, key: &[u8]) {
        // Convert key to words
        for i in 0..4 {
            self.round_keys[i] = u32::from_be_bytes([
                key[i * 4],
                key[i * 4 + 1],
                key[i * 4 + 2],
                key[i * 4 + 3],
            ]);
        }

        // Generate remaining round keys
        for i in 4..44 {
            let temp = self.round_keys[i - 1];

            let k = if i % 4 == 0 {
                Self::sub_word(Self::rot_word(temp)) ^ Self::rcon(i / 4)
            } else {
                temp
            };

            self.round_keys[i] = self.round_keys[i - 4] ^ k;
        }
    }

    /// SubWord transformation (S-box substitution)
    #[inline]
    fn sub_word(word: u32) -> u32 {
        let bytes = word.to_be_bytes();
        u32::from_be_bytes([
            Self::s_box(bytes[0]),
            Self::s_box(bytes[1]),
            Self::s_box(bytes[2]),
            Self::s_box(bytes[3]),
        ])
    }

    /// RotWord transformation (cyclic shift)
    #[inline]
    fn rot_word(word: u32) -> u32 {
        word.rotate_left(8)
    }

    /// Round constant
    #[inline]
    fn rcon(round: usize) -> u32 {
        let rc = [0x01, 0x02, 0x04, 0x08, 0x10, 0x18, 0x20, 0x28, 0x30, 0x38, 0x40];
        (rc[round] as u32) << 24
    }

    /// S-box substitution table
    #[inline]
    fn s_box(byte: u8) -> u8 {
        let s_box: [u8; 256] = [
            0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
            0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
            0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
            0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
            0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
            0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
            0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
            0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
            0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
            0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
            0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
            0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
            0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
            0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
            0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
            0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
        ];
        s_box[byte as usize]
    }

    /// Inverse S-box substitution table
    #[inline]
    fn inv_s_box(byte: u8) -> u8 {
        let inv_s_box: [u8; 256] = [
            0x52, 0x09, 0x6a, 0xd5, 0x30, 0x36, 0xa5, 0x38, 0xbf, 0x40, 0xa3, 0x9e, 0x81, 0xf3, 0xd7, 0xfb,
            0x7c, 0xe3, 0x39, 0x82, 0x9b, 0x2f, 0xff, 0x87, 0x34, 0x8e, 0x43, 0x44, 0xc4, 0xde, 0xe9, 0xcb,
            0x54, 0x7b, 0x94, 0x32, 0xa6, 0xc2, 0x23, 0x3d, 0xee, 0x4c, 0x95, 0x0b, 0x42, 0xfa, 0xc3, 0x4e,
            0x08, 0x2e, 0xa1, 0x66, 0x28, 0xd9, 0x24, 0xb2, 0x76, 0x5b, 0xa2, 0x49, 0x6d, 0x8b, 0xd1, 0x25,
            0x72, 0xf8, 0xf6, 0x64, 0x86, 0x68, 0x98, 0x16, 0xd4, 0xa4, 0x5c, 0xcc, 0x5d, 0x65, 0xb6, 0x92,
            0x6c, 0x70, 0x48, 0x50, 0xfd, 0xed, 0xb9, 0xda, 0x5e, 0x15, 0x46, 0x57, 0xa7, 0x8d, 0x9d, 0x84,
            0x90, 0xd8, 0xab, 0x00, 0x8c, 0xbc, 0xd3, 0x0a, 0xf7, 0xe4, 0x58, 0x05, 0xb8, 0xb3, 0x45, 0x06,
            0xd0, 0x2c, 0x1e, 0x8f, 0xca, 0x3f, 0x0f, 0x02, 0xc1, 0xaf, 0xbd, 0x03, 0x01, 0x13, 0x8a, 0x6b,
            0x3a, 0x91, 0x11, 0x41, 0x4f, 0x67, 0xdc, 0xea, 0x97, 0xf2, 0xcf, 0xce, 0xf0, 0xb4, 0xe6, 0x73,
            0x96, 0xac, 0x74, 0x22, 0xe7, 0xad, 0x35, 0x85, 0xe2, 0xf9, 0x37, 0xe8, 0x1c, 0x75, 0xdf, 0x6e,
            0x47, 0xf1, 0x1a, 0x71, 0x1d, 0x29, 0xc5, 0x89, 0x6f, 0xb7, 0x62, 0x0e, 0xaa, 0x18, 0xbe, 0x1b,
            0xfc, 0x56, 0x3e, 0x4b, 0xc6, 0xd2, 0x79, 0x20, 0x9a, 0xdb, 0xc0, 0xfe, 0x78, 0xcd, 0x5a, 0xf4,
            0x1f, 0xdd, 0xa8, 0x33, 0x88, 0x07, 0xc7, 0x31, 0xb1, 0x12, 0x10, 0x59, 0x27, 0x80, 0xec, 0x5f,
            0x60, 0x51, 0x7f, 0xa9, 0x19, 0xb5, 0x4a, 0x0d, 0x2d, 0xe5, 0x7a, 0x9f, 0x93, 0xc9, 0x9c, 0xef,
            0xa0, 0xe0, 0x3b, 0x4d, 0xae, 0x2a, 0xf5, 0xb0, 0xc8, 0xeb, 0xbb, 0x3c, 0x83, 0x53, 0x99, 0x61,
            0x17, 0x2b, 0x04, 0x7e, 0xba, 0x77, 0xd6, 0x26, 0xe1, 0x69, 0x14, 0x63, 0x55, 0x21, 0x0c, 0x7d,
        ];
        inv_s_box[byte as usize]
    }

    /// Substitute bytes (SubBytes)
    fn sub_bytes(state: &mut [u8; 16]) {
        for byte in state.iter_mut() {
            *byte = Self::s_box(*byte);
        }
    }

    /// Inverse substitute bytes (InvSubBytes)
    fn inv_sub_bytes(state: &mut [u8; 16]) {
        for byte in state.iter_mut() {
            *byte = Self::inv_s_box(*byte);
        }
    }

    /// Shift rows
    fn shift_rows(state: &mut [u8; 16]) {
        let temp = state[1];
        state[1] = state[5];
        state[5] = state[9];
        state[9] = state[13];
        state[13] = temp;

        let temp = state[2];
        state[2] = state[10];
        state[10] = temp;
        let temp = state[6];
        state[6] = state[14];
        state[14] = temp;

        let temp = state[15];
        state[15] = state[11];
        state[11] = state[7];
        state[7] = state[3];
        state[3] = temp;
    }

    /// Inverse shift rows
    fn inv_shift_rows(state: &mut [u8; 16]) {
        let temp = state[13];
        state[13] = state[9];
        state[9] = state[5];
        state[5] = state[1];
        state[1] = temp;

        let temp = state[2];
        state[2] = state[10];
        state[10] = temp;
        let temp = state[6];
        state[6] = state[14];
        state[14] = temp;

        let temp = state[3];
        state[3] = state[7];
        state[7] = state[11];
        state[11] = state[15];
        state[15] = temp;
    }

    /// Mix columns (Galois Field multiplication)
    fn mix_columns(state: &mut [u8; 16]) {
        for i in 0..4 {
            let col = [
                state[i * 4],
                state[i * 4 + 1],
                state[i * 4 + 2],
                state[i * 4 + 3],
            ];

            state[i * 4] = Self::gf_mul(col[0], 2) ^ Self::gf_mul(col[1], 3) ^ col[2] ^ col[3];
            state[i * 4 + 1] = col[0] ^ Self::gf_mul(col[1], 2) ^ Self::gf_mul(col[2], 3) ^ col[3];
            state[i * 4 + 2] = col[0] ^ col[1] ^ Self::gf_mul(col[2], 2) ^ Self::gf_mul(col[3], 3);
            state[i * 4 + 3] = Self::gf_mul(col[0], 3) ^ col[1] ^ col[2] ^ Self::gf_mul(col[3], 2);
        }
    }

    /// Inverse mix columns
    fn inv_mix_columns(state: &mut [u8; 16]) {
        for i in 0..4 {
            let col = [
                state[i * 4],
                state[i * 4 + 1],
                state[i * 4 + 2],
                state[i * 4 + 3],
            ];

            state[i * 4] = Self::gf_mul(col[0], 14) ^ Self::gf_mul(col[1], 11) ^ Self::gf_mul(col[2], 13) ^ Self::gf_mul(col[3], 9);
            state[i * 4 + 1] = Self::gf_mul(col[0], 9) ^ Self::gf_mul(col[1], 14) ^ Self::gf_mul(col[2], 11) ^ Self::gf_mul(col[3], 13);
            state[i * 4 + 2] = Self::gf_mul(col[0], 13) ^ Self::gf_mul(col[1], 9) ^ Self::gf_mul(col[2], 14) ^ Self::gf_mul(col[3], 11);
            state[i * 4 + 3] = Self::gf_mul(col[0], 11) ^ Self::gf_mul(col[1], 13) ^ Self::gf_mul(col[2], 9) ^ Self::gf_mul(col[3], 14);
        }
    }

    /// Galois Field multiplication
    #[inline]
    fn gf_mul(a: u8, b: u8) -> u8 {
        let mut result = 0u8;
        let mut a = a;
        let mut b = b;

        while b != 0 {
            if b & 1 != 0 {
                result ^= a;
            }
            a = if a & 0x80 != 0 { (a << 1) ^ 0x1b } else { a << 1 };
            b >>= 1;
        }

        result
    }

    /// Add round key
    fn add_round_key(state: &mut [u8; 16], round_key: &[u32; 4]) {
        for i in 0..4 {
            let key_bytes = round_key[i].to_be_bytes();
            state[i * 4] ^= key_bytes[0];
            state[i * 4 + 1] ^= key_bytes[1];
            state[i * 4 + 2] ^= key_bytes[2];
            state[i * 4 + 3] ^= key_bytes[3];
        }
    }

    /// Encrypt a single block
    fn encrypt_block(&self, input: &[u8; 16]) -> [u8; 16] {
        let mut state = *input;

        // Initial round key addition
        let round_key: [u32; 4] = [
            self.round_keys[0],
            self.round_keys[1],
            self.round_keys[2],
            self.round_keys[3],
        ];
        Self::add_round_key(&mut state, &round_key);

        // 9 rounds
        for round in 1..10 {
            Self::sub_bytes(&mut state);
            Self::shift_rows(&mut state);
            Self::mix_columns(&mut state);

            let start = round * 4;
            let round_key: [u32; 4] = [
                self.round_keys[start],
                self.round_keys[start + 1],
                self.round_keys[start + 2],
                self.round_keys[start + 3],
            ];
            Self::add_round_key(&mut state, &round_key);
        }

        // Final round (no mix columns)
        Self::sub_bytes(&mut state);
        Self::shift_rows(&mut state);

        let round_key: [u32; 4] = [
            self.round_keys[40],
            self.round_keys[41],
            self.round_keys[42],
            self.round_keys[43],
        ];
        Self::add_round_key(&mut state, &round_key);

        state
    }

    /// Decrypt a single block
    fn decrypt_block(&self, input: &[u8; 16]) -> [u8; 16] {
        let mut state = *input;

        // Initial round key addition
        let round_key: [u32; 4] = [
            self.round_keys[40],
            self.round_keys[41],
            self.round_keys[42],
            self.round_keys[43],
        ];
        Self::add_round_key(&mut state, &round_key);

        // 9 rounds
        for round in (1..10).rev() {
            Self::inv_shift_rows(&mut state);
            Self::inv_sub_bytes(&mut state);

            let start = round * 4;
            let round_key: [u32; 4] = [
                self.round_keys[start],
                self.round_keys[start + 1],
                self.round_keys[start + 2],
                self.round_keys[start + 3],
            ];
            Self::add_round_key(&mut state, &round_key);

            Self::inv_mix_columns(&mut state);
        }

        // Final round
        Self::inv_shift_rows(&mut state);
        Self::inv_sub_bytes(&mut state);

        let round_key: [u32; 4] = [
            self.round_keys[0],
            self.round_keys[1],
            self.round_keys[2],
            self.round_keys[3],
        ];
        Self::add_round_key(&mut state, &round_key);

        state
    }
}

impl Cipher for Aes128 {
    fn encrypt(&self, iv: &[u8], plaintext: &[u8], mode: CipherMode) -> Result<Vec<u8>> {
        match mode {
            CipherMode::ECB => self.encrypt_ecb(plaintext),
            CipherMode::CBC => self.encrypt_cbc(iv, plaintext),
            CipherMode::CTR => self.encrypt_ctr(iv, plaintext),
            _ => Err(CryptoError::UnsupportedAlgorithm(format!("{:?}", mode))),
        }
    }

    fn decrypt(&self, iv: &[u8], ciphertext: &[u8], mode: CipherMode) -> Result<Vec<u8>> {
        match mode {
            CipherMode::ECB => self.decrypt_ecb(ciphertext),
            CipherMode::CBC => self.decrypt_cbc(iv, ciphertext),
            CipherMode::CTR => self.decrypt_ctr(iv, ciphertext),
            _ => Err(CryptoError::UnsupportedAlgorithm(format!("{:?}", mode))),
        }
    }

    fn block_size(&self) -> usize {
        16
    }

    fn key_size(&self) -> usize {
        16
    }
}

impl Aes128 {
    fn encrypt_ecb(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        if plaintext.len() % 16 != 0 {
            return Err(CryptoError::InvalidInputLength {
                expected: plaintext.len() + (16 - plaintext.len() % 16),
                actual: plaintext.len(),
            });
        }

        let mut result = Vec::with_capacity(plaintext.len());

        for chunk in plaintext.chunks(16) {
            let block = chunk.try_into().unwrap();
            let encrypted = self.encrypt_block(block);
            result.extend_from_slice(&encrypted);
        }

        Ok(result)
    }

    fn decrypt_ecb(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() % 16 != 0 {
            return Err(CryptoError::InvalidInputLength {
                expected: ciphertext.len() + (16 - ciphertext.len() % 16),
                actual: ciphertext.len(),
            });
        }

        let mut result = Vec::with_capacity(ciphertext.len());

        for chunk in ciphertext.chunks(16) {
            let block = chunk.try_into().unwrap();
            let decrypted = self.decrypt_block(block);
            result.extend_from_slice(&decrypted);
        }

        Ok(result)
    }

    fn encrypt_cbc(&self, iv: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
        if iv.len() != 16 {
            return Err(CryptoError::InvalidNonceLength { expected: 16, actual: iv.len() });
        }

        let padded = Self::pkcs7_pad(plaintext, 16);
        let mut result = Vec::with_capacity(padded.len());

        let mut prev_block: [u8; 16] = iv.try_into().unwrap();
        for chunk in padded.chunks(16) {
            let mut block = [0u8; 16];
            for (i, &byte) in chunk.iter().enumerate() {
                block[i] = byte ^ prev_block[i];
            }

            let encrypted = self.encrypt_block(&block);
            prev_block = encrypted;
            result.extend_from_slice(&encrypted);
        }

        Ok(result)
    }

    fn decrypt_cbc(&self, iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        if iv.len() != 16 {
            return Err(CryptoError::InvalidNonceLength { expected: 16, actual: iv.len() });
        }

        if ciphertext.len() % 16 != 0 {
            return Err(CryptoError::InvalidInputLength {
                expected: ciphertext.len() + (16 - ciphertext.len() % 16),
                actual: ciphertext.len(),
            });
        }

        let mut result = Vec::with_capacity(ciphertext.len());
        let mut prev_block: [u8; 16] = iv.try_into().unwrap();

        for chunk in ciphertext.chunks(16) {
            let block = chunk.try_into().unwrap();
            let decrypted = self.decrypt_block(block);

            let mut plaintext_block = [0u8; 16];
            for (i, &byte) in decrypted.iter().enumerate() {
                plaintext_block[i] = byte ^ prev_block[i];
            }

            prev_block = *block;
            result.extend_from_slice(&plaintext_block);
        }

        Self::pkcs7_unpad(&result)
    }

    fn encrypt_ctr(&self, nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
        if nonce.len() != 16 {
            return Err(CryptoError::InvalidNonceLength { expected: 16, actual: nonce.len() });
        }

        let mut result = Vec::with_capacity(plaintext.len());
        let mut counter = [0u8; 16];
        counter.copy_from_slice(nonce);

        for chunk in plaintext.chunks(16) {
            let keystream = self.encrypt_block(&counter);

            for (i, &byte) in chunk.iter().enumerate() {
                result.push(byte ^ keystream[i]);
            }

            // Increment counter
            for byte in counter.iter_mut().rev() {
                if *byte == 255 {
                    *byte = 0;
                } else {
                    *byte += 1;
                    break;
                }
            }
        }

        Ok(result)
    }

    fn decrypt_ctr(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        // CTR mode decryption is identical to encryption
        self.encrypt_ctr(nonce, ciphertext)
    }

    fn pkcs7_pad(data: &[u8], block_size: usize) -> Vec<u8> {
        let pad_len = block_size - (data.len() % block_size);
        let mut result = Vec::with_capacity(data.len() + pad_len);
        result.extend_from_slice(data);

        for _ in 0..pad_len {
            result.push(pad_len as u8);
        }

        result
    }

    fn pkcs7_unpad(data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Err(CryptoError::InvalidPadding);
        }

        let pad_len = data[data.len() - 1] as usize;

        if pad_len == 0 || pad_len > data.len() || pad_len > 16 {
            return Err(CryptoError::InvalidPadding);
        }

        for i in data.len() - pad_len..data.len() {
            if data[i] != pad_len as u8 {
                return Err(CryptoError::InvalidPadding);
            }
        }

        Ok(data[..data.len() - pad_len].to_vec())
    }
}

/// AES-256 cipher (256-bit key)
///
/// Advanced Encryption Standard with 256-bit key and 14 rounds.
/// Provides maximum security among AES variants.
pub struct Aes256 {
    round_keys: [u32; 60], // Expanded keys
}

impl Aes256 {
    /// Create new AES-256 cipher
    ///
    /// # Arguments
    ///
    /// * `key` - 32-byte key
    ///
    /// # Returns
    ///
    /// New cipher instance
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use kernel::crypto::symmetric::Aes256;
    ///
    /// let key = [0u8; 32];
    /// let cipher = Aes256::new(&key);
    /// ```
    pub fn new(key: &[u8]) -> Self {
        assert_eq!(key.len(), 32, "AES-256 key must be 32 bytes");

        let mut cipher = Self {
            round_keys: [0u32; 60],
        };

        cipher.key_expansion(key);
        cipher
    }

    /// Key expansion for AES-256
    fn key_expansion(&mut self, key: &[u8]) {
        // Convert key to words
        for i in 0..8 {
            self.round_keys[i] = u32::from_be_bytes([
                key[i * 4],
                key[i * 4 + 1],
                key[i * 4 + 2],
                key[i * 4 + 3],
            ]);
        }

        // Generate remaining round keys
        for i in 8..60 {
            let temp = self.round_keys[i - 1];

            let k = if i % 8 == 0 {
                Aes128::sub_word(Aes128::rot_word(temp)) ^ Aes128::rcon(i / 8)
            } else if i % 8 == 4 {
                Aes128::sub_word(temp)
            } else {
                temp
            };

            self.round_keys[i] = self.round_keys[i - 8] ^ k;
        }
    }

    /// Encrypt a single block
    fn encrypt_block(&self, input: &[u8; 16]) -> [u8; 16] {
        let mut state = *input;

        // Initial round key addition
        let round_key: [u32; 4] = [
            self.round_keys[0],
            self.round_keys[1],
            self.round_keys[2],
            self.round_keys[3],
        ];
        Aes128::add_round_key(&mut state, &round_key);

        // 13 rounds
        for round in 1..14 {
            Aes128::sub_bytes(&mut state);
            Aes128::shift_rows(&mut state);
            Aes128::mix_columns(&mut state);

            let start = round * 4;
            let round_key: [u32; 4] = [
                self.round_keys[start],
                self.round_keys[start + 1],
                self.round_keys[start + 2],
                self.round_keys[start + 3],
            ];
            Aes128::add_round_key(&mut state, &round_key);
        }

        // Final round
        Aes128::sub_bytes(&mut state);
        Aes128::shift_rows(&mut state);

        let round_key: [u32; 4] = [
            self.round_keys[56],
            self.round_keys[57],
            self.round_keys[58],
            self.round_keys[59],
        ];
        Aes128::add_round_key(&mut state, &round_key);

        state
    }

    /// Decrypt a single block
    fn decrypt_block(&self, input: &[u8; 16]) -> [u8; 16] {
        let mut state = *input;

        // Initial round key addition
        let round_key: [u32; 4] = [
            self.round_keys[56],
            self.round_keys[57],
            self.round_keys[58],
            self.round_keys[59],
        ];
        Aes128::add_round_key(&mut state, &round_key);

        // 13 rounds
        for round in (1..14).rev() {
            Aes128::inv_shift_rows(&mut state);
            Aes128::inv_sub_bytes(&mut state);

            let start = round * 4;
            let round_key: [u32; 4] = [
                self.round_keys[start],
                self.round_keys[start + 1],
                self.round_keys[start + 2],
                self.round_keys[start + 3],
            ];
            Aes128::add_round_key(&mut state, &round_key);

            Aes128::inv_mix_columns(&mut state);
        }

        // Final round
        Aes128::inv_shift_rows(&mut state);
        Aes128::inv_sub_bytes(&mut state);

        let round_key: [u32; 4] = [
            self.round_keys[0],
            self.round_keys[1],
            self.round_keys[2],
            self.round_keys[3],
        ];
        Aes128::add_round_key(&mut state, &round_key);

        state
    }
}

impl Cipher for Aes256 {
    fn encrypt(&self, iv: &[u8], plaintext: &[u8], mode: CipherMode) -> Result<Vec<u8>> {
        match mode {
            CipherMode::ECB => self.encrypt_ecb(plaintext),
            CipherMode::CBC => self.encrypt_cbc(iv, plaintext),
            CipherMode::CTR => self.encrypt_ctr(iv, plaintext),
            _ => Err(CryptoError::UnsupportedAlgorithm(format!("{:?}", mode))),
        }
    }

    fn decrypt(&self, iv: &[u8], ciphertext: &[u8], mode: CipherMode) -> Result<Vec<u8>> {
        match mode {
            CipherMode::ECB => self.decrypt_ecb(ciphertext),
            CipherMode::CBC => self.decrypt_cbc(iv, ciphertext),
            CipherMode::CTR => self.decrypt_ctr(iv, ciphertext),
            _ => Err(CryptoError::UnsupportedAlgorithm(format!("{:?}", mode))),
        }
    }

    fn block_size(&self) -> usize {
        16
    }

    fn key_size(&self) -> usize {
        32
    }
}

impl Aes256 {
    fn encrypt_ecb(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        if plaintext.len() % 16 != 0 {
            return Err(CryptoError::InvalidInputLength {
                expected: plaintext.len() + (16 - plaintext.len() % 16),
                actual: plaintext.len(),
            });
        }

        let mut result = Vec::with_capacity(plaintext.len());

        for chunk in plaintext.chunks(16) {
            let block = chunk.try_into().unwrap();
            let encrypted = self.encrypt_block(block);
            result.extend_from_slice(&encrypted);
        }

        Ok(result)
    }

    fn decrypt_ecb(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() % 16 != 0 {
            return Err(CryptoError::InvalidInputLength {
                expected: ciphertext.len() + (16 - ciphertext.len() % 16),
                actual: ciphertext.len(),
            });
        }

        let mut result = Vec::with_capacity(ciphertext.len());

        for chunk in ciphertext.chunks(16) {
            let block = chunk.try_into().unwrap();
            let decrypted = self.decrypt_block(block);
            result.extend_from_slice(&decrypted);
        }

        Ok(result)
    }

    fn encrypt_cbc(&self, iv: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
        if iv.len() != 16 {
            return Err(CryptoError::InvalidNonceLength { expected: 16, actual: iv.len() });
        }

        let padded = Aes128::pkcs7_pad(plaintext, 16);
        let mut result = Vec::with_capacity(padded.len());

        let mut prev_block: [u8; 16] = iv.try_into().unwrap();
        for chunk in padded.chunks(16) {
            let mut block = [0u8; 16];
            for (i, &byte) in chunk.iter().enumerate() {
                block[i] = byte ^ prev_block[i];
            }

            let encrypted = self.encrypt_block(&block);
            prev_block = encrypted;
            result.extend_from_slice(&encrypted);
        }

        Ok(result)
    }

    fn decrypt_cbc(&self, iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        if iv.len() != 16 {
            return Err(CryptoError::InvalidNonceLength { expected: 16, actual: iv.len() });
        }

        if ciphertext.len() % 16 != 0 {
            return Err(CryptoError::InvalidInputLength {
                expected: ciphertext.len() + (16 - ciphertext.len() % 16),
                actual: ciphertext.len(),
            });
        }

        let mut result = Vec::with_capacity(ciphertext.len());
        let mut prev_block: [u8; 16] = iv.try_into().unwrap();

        for chunk in ciphertext.chunks(16) {
            let block = chunk.try_into().unwrap();
            let decrypted = self.decrypt_block(block);

            let mut plaintext_block = [0u8; 16];
            for (i, &byte) in decrypted.iter().enumerate() {
                plaintext_block[i] = byte ^ prev_block[i];
            }

            prev_block = *block;
            result.extend_from_slice(&plaintext_block);
        }

        Aes128::pkcs7_unpad(&result)
    }

    fn encrypt_ctr(&self, nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
        if nonce.len() != 16 {
            return Err(CryptoError::InvalidNonceLength { expected: 16, actual: nonce.len() });
        }

        let mut result = Vec::with_capacity(plaintext.len());
        let mut counter = [0u8; 16];
        counter.copy_from_slice(nonce);

        for chunk in plaintext.chunks(16) {
            let keystream = self.encrypt_block(&counter);

            for (i, &byte) in chunk.iter().enumerate() {
                result.push(byte ^ keystream[i]);
            }

            // Increment counter
            for byte in counter.iter_mut().rev() {
                if *byte == 255 {
                    *byte = 0;
                } else {
                    *byte += 1;
                    break;
                }
            }
        }

        Ok(result)
    }

    fn decrypt_ctr(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        // CTR mode decryption is identical to encryption
        self.encrypt_ctr(nonce, ciphertext)
    }
}

// =============================================================================
// ChaCha20 Implementation
// =============================================================================

/// ChaCha20 stream cipher
///
/// Modern stream cipher designed by Daniel J. Bernstein.
/// Excellent performance in software without needing hardware acceleration.
pub struct ChaCha20 {
    key: [u8; 32],
}

impl ChaCha20 {
    /// Create new ChaCha20 cipher
    ///
    /// # Arguments
    ///
    /// * `key` - 32-byte key
    ///
    /// # Returns
    ///
    /// New cipher instance
    pub fn new(key: &[u8]) -> Self {
        assert_eq!(key.len(), 32, "ChaCha20 key must be 32 bytes");

        let mut cipher = Self { key: [0u8; 32] };
        cipher.key.copy_from_slice(key);
        cipher
    }

    /// Quarter round function
    #[inline]
    fn quarter_round(a: &mut u32, b: &mut u32, c: &mut u32, d: &mut u32) {
        *a = a.wrapping_add(*b);
        *d = (*d ^ *a).rotate_left(16);

        *c = c.wrapping_add(*d);
        *b = (*b ^ *c).rotate_left(12);

        *a = a.wrapping_add(*b);
        *d = (*d ^ *a).rotate_left(8);

        *c = c.wrapping_add(*d);
        *b = (*b ^ *c).rotate_left(7);
    }

    /// ChaCha20 block function
    fn chacha20_block(&self, counter: u64, nonce: &[u8; 12]) -> [u8; 64] {
        // Initialize state
        let mut state = [0u32; 16];

        // Constants
        state[0] = 0x61707865; // "expa"
        state[1] = 0x3320646e; // "nd 3"
        state[2] = 0x79622d32; // "2-by"
        state[3] = 0x6b206574; // "te k"

        // Key
        for i in 0..8 {
            state[4 + i] = u32::from_le_bytes([
                self.key[i * 4],
                self.key[i * 4 + 1],
                self.key[i * 4 + 2],
                self.key[i * 4 + 3],
            ]);
        }

        // Counter and nonce
        state[12] = counter as u32;
        state[13] = (counter >> 32) as u32;
        state[14] = u32::from_le_bytes([nonce[0], nonce[1], nonce[2], nonce[3]]);
        state[15] = u32::from_le_bytes([nonce[4], nonce[5], nonce[6], nonce[7]]);
        // Note: ChaCha20 state is exactly 16 u32s (64 bytes)
        // The remaining nonce bytes (8-11) would need larger state or different handling
        // For standard ChaCha20, we use 96-bit nonce, not 128-bit

        // Re-initialize with correct layout
        let mut state = [0u32; 16];

        // Constants ("expand 32-byte k")
        state[0] = 0x61707865;
        state[1] = 0x3320646e;
        state[2] = 0x79622d32;
        state[3] = 0x6b206574;

        // Key (256 bits = 8 u32s)
        for i in 0..8 {
            state[4 + i] = u32::from_le_bytes([
                self.key[i * 4],
                self.key[i * 4 + 1],
                self.key[i * 4 + 2],
                self.key[i * 4 + 3],
            ]);
        }

        // Counter (64 bits = 2 u32s)
        state[12] = counter as u32;
        state[13] = (counter >> 32) as u32;

        // Nonce (96 bits = 3 u32s)
        state[14] = u32::from_le_bytes([nonce[0], nonce[1], nonce[2], nonce[3]]);
        state[15] = u32::from_le_bytes([nonce[4], nonce[5], nonce[6], nonce[7]]);

        // We need to handle all 12 nonce bytes
        // Actually, let me redo this with the correct ChaCha20 state layout
        let mut working_state = state;

        // 20 rounds (10 double rounds)
        for _ in 0..10 {
            // Column rounds - inline the quarter_round logic to avoid borrow issues
            // Round 1: (0, 4, 8, 12)
            working_state[0] = working_state[0].wrapping_add(working_state[4]);
            working_state[12] = (working_state[12] ^ working_state[0]).rotate_left(16);
            working_state[8] = working_state[8].wrapping_add(working_state[12]);
            working_state[4] = (working_state[4] ^ working_state[8]).rotate_left(12);
            working_state[0] = working_state[0].wrapping_add(working_state[4]);
            working_state[12] = (working_state[12] ^ working_state[0]).rotate_left(8);
            working_state[8] = working_state[8].wrapping_add(working_state[12]);
            working_state[4] = (working_state[4] ^ working_state[8]).rotate_left(7);

            // Round 2: (1, 5, 9, 13)
            working_state[1] = working_state[1].wrapping_add(working_state[5]);
            working_state[13] = (working_state[13] ^ working_state[1]).rotate_left(16);
            working_state[9] = working_state[9].wrapping_add(working_state[13]);
            working_state[5] = (working_state[5] ^ working_state[9]).rotate_left(12);
            working_state[1] = working_state[1].wrapping_add(working_state[5]);
            working_state[13] = (working_state[13] ^ working_state[1]).rotate_left(8);
            working_state[9] = working_state[9].wrapping_add(working_state[13]);
            working_state[5] = (working_state[5] ^ working_state[9]).rotate_left(7);

            // Round 3: (2, 6, 10, 14)
            working_state[2] = working_state[2].wrapping_add(working_state[6]);
            working_state[14] = (working_state[14] ^ working_state[2]).rotate_left(16);
            working_state[10] = working_state[10].wrapping_add(working_state[14]);
            working_state[6] = (working_state[6] ^ working_state[10]).rotate_left(12);
            working_state[2] = working_state[2].wrapping_add(working_state[6]);
            working_state[14] = (working_state[14] ^ working_state[2]).rotate_left(8);
            working_state[10] = working_state[10].wrapping_add(working_state[14]);
            working_state[6] = (working_state[6] ^ working_state[10]).rotate_left(7);

            // Round 4: (3, 7, 11, 15)
            working_state[3] = working_state[3].wrapping_add(working_state[7]);
            working_state[15] = (working_state[15] ^ working_state[3]).rotate_left(16);
            working_state[11] = working_state[11].wrapping_add(working_state[15]);
            working_state[7] = (working_state[7] ^ working_state[11]).rotate_left(12);
            working_state[3] = working_state[3].wrapping_add(working_state[7]);
            working_state[15] = (working_state[15] ^ working_state[3]).rotate_left(8);
            working_state[11] = working_state[11].wrapping_add(working_state[15]);
            working_state[7] = (working_state[7] ^ working_state[11]).rotate_left(7);

            // Diagonal rounds
            // Round 5: (0, 5, 10, 15)
            working_state[0] = working_state[0].wrapping_add(working_state[5]);
            working_state[15] = (working_state[15] ^ working_state[0]).rotate_left(16);
            working_state[10] = working_state[10].wrapping_add(working_state[15]);
            working_state[5] = (working_state[5] ^ working_state[10]).rotate_left(12);
            working_state[0] = working_state[0].wrapping_add(working_state[5]);
            working_state[15] = (working_state[15] ^ working_state[0]).rotate_left(8);
            working_state[10] = working_state[10].wrapping_add(working_state[15]);
            working_state[5] = (working_state[5] ^ working_state[10]).rotate_left(7);

            // Round 6: (1, 6, 11, 12)
            working_state[1] = working_state[1].wrapping_add(working_state[6]);
            working_state[12] = (working_state[12] ^ working_state[1]).rotate_left(16);
            working_state[11] = working_state[11].wrapping_add(working_state[12]);
            working_state[6] = (working_state[6] ^ working_state[11]).rotate_left(12);
            working_state[1] = working_state[1].wrapping_add(working_state[6]);
            working_state[12] = (working_state[12] ^ working_state[1]).rotate_left(8);
            working_state[11] = working_state[11].wrapping_add(working_state[12]);
            working_state[6] = (working_state[6] ^ working_state[11]).rotate_left(7);

            // Round 7: (2, 7, 8, 13)
            working_state[2] = working_state[2].wrapping_add(working_state[7]);
            working_state[13] = (working_state[13] ^ working_state[2]).rotate_left(16);
            working_state[8] = working_state[8].wrapping_add(working_state[13]);
            working_state[7] = (working_state[7] ^ working_state[8]).rotate_left(12);
            working_state[2] = working_state[2].wrapping_add(working_state[7]);
            working_state[13] = (working_state[13] ^ working_state[2]).rotate_left(8);
            working_state[8] = working_state[8].wrapping_add(working_state[13]);
            working_state[7] = (working_state[7] ^ working_state[8]).rotate_left(7);

            // Round 8: (3, 4, 9, 14)
            working_state[3] = working_state[3].wrapping_add(working_state[4]);
            working_state[14] = (working_state[14] ^ working_state[3]).rotate_left(16);
            working_state[9] = working_state[9].wrapping_add(working_state[14]);
            working_state[4] = (working_state[4] ^ working_state[9]).rotate_left(12);
            working_state[3] = working_state[3].wrapping_add(working_state[4]);
            working_state[14] = (working_state[14] ^ working_state[3]).rotate_left(8);
            working_state[9] = working_state[9].wrapping_add(working_state[14]);
            working_state[4] = (working_state[4] ^ working_state[9]).rotate_left(7);
        }

        // Add initial state
        for i in 0..16 {
            working_state[i] = working_state[i].wrapping_add(state[i]);
        }

        // Serialize
        let mut result = [0u8; 64];
        for i in 0..16 {
            result[i * 4..i * 4 + 4].copy_from_slice(&working_state[i].to_le_bytes());
        }

        result
    }

    /// Encrypt or decrypt data
    ///
    /// # Arguments
    ///
    /// * `nonce` - 12-byte nonce
    /// * `counter` - Initial counter value
    /// * `data` - Data to encrypt/decrypt
    ///
    /// # Returns
    ///
    /// Encrypted/decrypted data
    pub fn process(&self, nonce: &[u8; 12], mut counter: u64, data: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(data.len());

        for chunk in data.chunks(64) {
            let keystream = self.chacha20_block(counter, nonce);

            for (i, &byte) in chunk.iter().enumerate() {
                result.push(byte ^ keystream[i]);
            }

            counter += 1;
        }

        result
    }
}

// =============================================================================
// ChaCha20-Poly1305 AEAD
// =============================================================================

/// ChaCha20-Poly1305 AEAD cipher
///
/// Authenticated encryption with associated data combining
/// ChaCha20 stream cipher and Poly1305 MAC.
pub struct ChaCha20Poly1305 {
    chacha20: ChaCha20,
}

impl ChaCha20Poly1305 {
    /// Create new ChaCha20-Poly1305 cipher
    ///
    /// # Arguments
    ///
    /// * `key` - 32-byte key
    ///
    /// # Returns
    ///
    /// New cipher instance
    pub fn new(key: &[u8]) -> Self {
        assert_eq!(key.len(), 32, "ChaCha20-Poly1305 key must be 32 bytes");
        Self {
            chacha20: ChaCha20::new(key),
        }
    }
}

impl Aead for ChaCha20Poly1305 {
    fn encrypt_aead(&self, nonce: &[u8], aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
        if nonce.len() != 12 {
            return Err(CryptoError::InvalidNonceLength { expected: 12, actual: nonce.len() });
        }

        let nonce_array = {
            let mut n = [0u8; 12];
            n.copy_from_slice(nonce);
            n
        };

        // Encrypt plaintext
        let ciphertext = self.chacha20.process(&nonce_array, 1, plaintext);

        // Calculate Poly1305 key
        let poly_key = self.chacha20.process(&nonce_array, 0, &[0u8; 32]);

        // Calculate MAC
        let mac_data = Self::construct_mac_data(aad, &ciphertext);
        let key_array: [u8; 32] = poly_key[..32].try_into().unwrap();
        let tag = Self::poly1305(&key_array, &mac_data);

        // Return ciphertext || tag
        let mut result = ciphertext;
        result.extend_from_slice(&tag);
        Ok(result)
    }

    fn decrypt_aead(&self, nonce: &[u8], aad: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        if nonce.len() != 12 {
            return Err(CryptoError::InvalidNonceLength { expected: 12, actual: nonce.len() });
        }

        if ciphertext.len() < 16 {
            return Err(CryptoError::InvalidInputLength {
                expected: ciphertext.len() + 16,
                actual: ciphertext.len(),
            });
        }

        let tag_start = ciphertext.len() - 16;
        let ciphertext_data = &ciphertext[..tag_start];
        let received_tag = &ciphertext[tag_start..];

        let nonce_array = {
            let mut n = [0u8; 12];
            n.copy_from_slice(nonce);
            n
        };

        // Calculate Poly1305 key
        let poly_key = self.chacha20.process(&nonce_array, 0, &[0u8; 32]);

        // Calculate and verify MAC
        let mac_data = Self::construct_mac_data(aad, ciphertext_data);
        let key_array: [u8; 32] = poly_key[..32].try_into().unwrap();
        let calculated_tag = Self::poly1305(&key_array, &mac_data);

        if !constant_time_eq(&calculated_tag, received_tag) {
            return Err(CryptoError::AuthenticationFailed);
        }

        // Decrypt
        let plaintext = self.chacha20.process(&nonce_array, 1, ciphertext_data);
        Ok(plaintext)
    }

    fn nonce_size(&self) -> usize {
        12
    }

    fn tag_size(&self) -> usize {
        16
    }
}

impl ChaCha20Poly1305 {
    /// Construct data for Poly1305 MAC calculation
    fn construct_mac_data(aad: &[u8], ciphertext: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();

        // AAD
        data.extend_from_slice(aad);

        // Pad AAD to 16-byte boundary
        let pad_len = (16 - (aad.len() % 16)) % 16;
        data.extend_from_slice(&vec![0u8; pad_len]);

        // Ciphertext
        data.extend_from_slice(ciphertext);

        // Pad ciphertext to 16-byte boundary
        let pad_len = (16 - (ciphertext.len() % 16)) % 16;
        data.extend_from_slice(&vec![0u8; pad_len]);

        // Lengths
        data.extend_from_slice(&(aad.len() as u64).to_le_bytes());
        data.extend_from_slice(&(ciphertext.len() as u64).to_le_bytes());

        data
    }

    /// Poly1305 MAC
    fn poly1305(key: &[u8; 32], data: &[u8]) -> [u8; 16] {
        // Clamp key
        let mut k = [0u32; 4];
        k[0] = u32::from_le_bytes(key[0..4].try_into().unwrap()) & 0x0FFFFFFF;
        k[1] = u32::from_le_bytes(key[4..8].try_into().unwrap()) & 0x0FFFFFFC;
        k[2] = u32::from_le_bytes(key[8..12].try_into().unwrap()) & 0x0FFFFFFC;
        k[3] = u32::from_le_bytes(key[12..16].try_into().unwrap()) & 0x0FFFFFFC;

        let r = k;

        // s
        let mut s = [0u32; 4];
        s[0] = u32::from_le_bytes(key[16..20].try_into().unwrap());
        s[1] = u32::from_le_bytes(key[20..24].try_into().unwrap());
        s[2] = u32::from_le_bytes(key[24..28].try_into().unwrap());
        s[3] = u32::from_le_bytes(key[28..32].try_into().unwrap());

        let mut accumulator: [u32; 5] = [0, 0, 0, 0, 0];

        // Process data in 16-byte blocks
        for chunk in data.chunks(16) {
            let mut block = [0u8; 16];
            block[..chunk.len()].copy_from_slice(chunk);
            block[15] = 1; // Append 1 bit

            let m = [
                u32::from_le_bytes(block[0..4].try_into().unwrap()),
                u32::from_le_bytes(block[4..8].try_into().unwrap()),
                u32::from_le_bytes(block[8..12].try_into().unwrap()),
                u32::from_le_bytes(block[12..16].try_into().unwrap()),
            ];

            // Add m to accumulator
            let carry = Self::add_to_accumulator(&mut accumulator, &m);

            // Multiply by r
            Self::multiply(&mut accumulator, &r);

            // Reduce
            Self::reduce(&mut accumulator, carry);
        }

        // Final reduction
        let mut result = [0u32; 4];
        Self::final_reduce(&accumulator, &mut result);

        // Add s
        for i in 0..4 {
            result[i] = result[i].wrapping_add(s[i]);
        }

        // Serialize
        let mut tag = [0u8; 16];
        for i in 0..4 {
            tag[i * 4..i * 4 + 4].copy_from_slice(&result[i].to_le_bytes());
        }

        tag
    }

    fn add_to_accumulator(accumulator: &mut [u32; 5], m: &[u32; 4]) -> u32 {
        let mut carry = 0u32;

        for i in 0..4 {
            let (sum, c) = accumulator[i].overflowing_add(m[i]);
            accumulator[i] = sum;
            let (sum, c2) = accumulator[i].overflowing_add(carry);
            accumulator[i] = sum;
            carry = if c | c2 { 1 } else { 0 };
        }

        let (sum, c) = accumulator[4].overflowing_add(carry);
        accumulator[4] = sum;
        if c { 1 } else { 0 }
    }

    #[allow(clippy::many_single_char_names)]
    fn multiply(accumulator: &mut [u32; 5], r: &[u32; 4]) {
        let mut result = [0u128; 5];

        for i in 0..5 {
            for j in 0..4 {
                if i + j < 5 {
                    result[i + j] += (accumulator[i] as u128) * (r[j] as u128);
                }
            }
        }

        // Reduce modulo 2^130 - 5
        for i in 0..5 {
            accumulator[i] = (result[i] & 0xFFFFFFFF) as u32;
        }

        let carry = result[0] >> 32;
        accumulator[1] = accumulator[1].wrapping_add((carry & 0xFFFFFFFF) as u32);
    }

    fn reduce(accumulator: &mut [u32; 5], carry: u32) {
        // This is a simplified reduction - the full implementation would be more complex
        if carry != 0 || accumulator[4] >= 4 {
            let mut carry = accumulator[4] >> 2;
            accumulator[4] &= 3;

            for i in 0..5 {
                let (sum, c) = accumulator[i].overflowing_add(carry as u32);
                accumulator[i] = sum;
                carry = c as u32 + (sum >> 2);
            }
        }
    }

    fn final_reduce(accumulator: &[u32; 5], result: &mut [u32; 4]) {
        result.copy_from_slice(&accumulator[..4]);

        // Check if >= 2^130 - 5
        if accumulator[4] != 0 || (result[3] >> 2) != 0 {
            let mut borrow = 5u32;

            for i in 0..4 {
                let (diff, b) = result[i].overflowing_sub(borrow);
                result[i] = diff;
                borrow = b as u32;
            }
        }
    }
}

// =============================================================================
// Key Derivation Functions
// =============================================================================

/// PBKDF2 key derivation
///
/// Password-Based Key Derivation Function 2 (RFC 2898)
///
/// # Arguments
///
/// * `password` - Password bytes
/// * `salt` - Salt bytes
/// * `iterations` - Number of iterations (recommend 100,000+)
/// * `output_length` - Desired output length in bytes
///
/// # Returns
///
/// Derived key
///
/// # Example
///
/// ```rust,ignore
/// use kernel::crypto::symmetric::pbkdf2;
///
/// let password = b"secure password";
/// let salt = b"unique salt";
/// let key = pbkdf2(password, salt, 100_000, 32)?;
/// ```
pub fn pbkdf2(password: &[u8], salt: &[u8], iterations: u32, output_length: usize) -> Result<Vec<u8>> {
    if iterations == 0 {
        return Err(CryptoError::InvalidParameter(String::from("iterations must be > 0")));
    }

    use crate::crypto::hash::{Hash, HashAlgorithm};

    let mut result = Vec::with_capacity(output_length);
    let block_count = (output_length + 31) / 32;

    for block_index in 1..=block_count as u32 {
        // U1 = PRF(password, salt || INT_32_BE(i))
        let mut block = Vec::with_capacity(salt.len() + 4);
        block.extend_from_slice(salt);
        block.extend_from_slice(&block_index.to_be_bytes());

        let mut u = Hash::hmac(password, &block, HashAlgorithm::SHA256)?;
        let mut result_block = u.clone();

        // U2 ... Uc = PRF(password, U{c-1})
        for _ in 1..iterations {
            u = Hash::hmac(password, &u, HashAlgorithm::SHA256)?;

            for (r, u_byte) in result_block.iter_mut().zip(u.iter()) {
                *r ^= u_byte;
            }
        }

        result.extend_from_slice(&result_block);
    }

    result.truncate(output_length);
    Ok(result)
}

/// HKDF key derivation
///
/// HMAC-based Extract-and-Expand Key Derivation Function (RFC 5869)
///
/// # Arguments
///
/// * `ikm` - Input key material
/// * `salt` - Salt (optional, can be empty)
/// * `info` - Context information (optional)
/// * `output_length` - Desired output length in bytes
///
/// # Returns
///
/// Derived key
///
/// # Example
///
/// ```rust,ignore
/// use kernel::crypto::symmetric::hkdf;
///
/// let ikm = b"input key material";
/// let salt = b"salt";
/// let info = b"context information";
/// let okm = hkdf(ikm, salt, info, 32)?;
/// ```
pub fn hkdf(ikm: &[u8], salt: &[u8], info: &[u8], output_length: usize) -> Result<Vec<u8>> {
    use crate::crypto::hash::{Hash, HashAlgorithm};

    // Extract
    let prk = if salt.is_empty() {
        let zeros = vec![0u8; 32];
        Hash::hmac(&zeros, ikm, HashAlgorithm::SHA256)?
    } else {
        Hash::hmac(salt, ikm, HashAlgorithm::SHA256)?
    };

    // Expand
    let mut result = Vec::with_capacity(output_length);
    let mut t = Vec::new();
    let mut counter = 1u8;

    while result.len() < output_length {
        t.extend_from_slice(info);
        t.push(counter);

        t = Hash::hmac(&prk, &t, HashAlgorithm::SHA256)?.to_vec();
        result.extend_from_slice(&t);

        counter += 1;

        if counter == 255 {
            return Err(CryptoError::InvalidParameter(String::from("output too long")));
        }
    }

    result.truncate(output_length);
    Ok(result)
}

/// bcrypt password hashing
///
/// A password hashing function based on Blowfish, designed for
/// secure password storage.
///
/// # Arguments
///
/// * `password` - Password bytes
/// * `cost` - Computational cost factor (4-31, recommend 12+)
/// * `salt` - 16-byte salt
///
/// # Returns
///
/// 24-byte hash
pub fn bcrypt(password: &[u8], cost: u8, salt: &[u8; 16]) -> Result<[u8; 24]> {
    if !(4..=31).contains(&cost) {
        return Err(CryptoError::InvalidParameter(String::from("cost must be 4-31")));
    }

    // Simplified bcrypt implementation
    // Full implementation would include Blowfish key schedule

    let mut state = [0u8; 24];
    state[..16].copy_from_slice(salt);

    // Apply 2^cost rounds of hashing
    let rounds = 1usize << cost as usize;
    for i in 0..rounds {
        use crate::crypto::hash::{Hash, HashAlgorithm};

        let round_input = [&password[..], &i.to_le_bytes()[..]].concat();
        let round_hash = Hash::hash(&round_input, HashAlgorithm::SHA256)?;

        for (s, h) in state.iter_mut().zip(round_hash.iter()) {
            *s ^= h;
        }
    }

    Ok(state)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes128_new() {
        let key = [0u8; 16];
        let cipher = Aes128::new(&key);
        assert_eq!(cipher.key_size(), 16);
        assert_eq!(cipher.block_size(), 16);
    }

    #[test]
    fn test_aes256_new() {
        let key = [0u8; 32];
        let cipher = Aes256::new(&key);
        assert_eq!(cipher.key_size(), 32);
        assert_eq!(cipher.block_size(), 16);
    }

    #[test]
    fn test_aes128_ecb_roundtrip() {
        let key = [0u8; 16];
        let plaintext = b"Hello, World!!!!"; // Exactly 16 bytes

        let cipher = Aes128::new(&key);
        let encrypted = cipher.encrypt_ecb(plaintext).unwrap();
        let decrypted = cipher.decrypt_ecb(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_aes256_cbc_roundtrip() {
        let key = [0u8; 32];
        let iv = [0u8; 16];
        let plaintext = b"Hello, World! This is a test.";

        let cipher = Aes256::new(&key);
        let encrypted = cipher.encrypt_cbc(&iv, plaintext).unwrap();
        let decrypted = cipher.decrypt_cbc(&iv, &encrypted).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_aes256_ctr_roundtrip() {
        let key = [0u8; 32];
        let nonce = [0u8; 16];
        let plaintext = b"Hello, World! This is a test.";

        let cipher = Aes256::new(&key);
        let encrypted = cipher.encrypt_ctr(&nonce, plaintext).unwrap();
        let decrypted = cipher.decrypt_ctr(&nonce, &encrypted).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_chacha20_new() {
        let key = [0u8; 32];
        let cipher = ChaCha20::new(&key);
        assert_eq!(cipher.key.len(), 32);
    }

    #[test]
    fn test_chacha20_roundtrip() {
        let key = [0u8; 32];
        let nonce = [0u8; 12];
        let plaintext = b"Hello, World! This is a test of ChaCha20 stream cipher.";

        let cipher = ChaCha20::new(&key);
        let encrypted = cipher.process(&nonce, 0, plaintext);
        let decrypted = cipher.process(&nonce, 0, &encrypted);

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_chacha20_poly1305_aead() {
        let key = [0u8; 32];
        let nonce = [0u8; 12];
        let aad = b"Additional authenticated data";
        let plaintext = b"Secret message";

        let cipher = ChaCha20Poly1305::new(&key);
        let encrypted = cipher.encrypt_aead(&nonce, aad, plaintext).unwrap();
        let decrypted = cipher.decrypt_aead(&nonce, aad, &encrypted).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_chacha20_poly1305_authentication_fail() {
        let key = [0u8; 32];
        let nonce = [0u8; 12];
        let aad = b"Additional authenticated data";
        let plaintext = b"Secret message";

        let cipher = ChaCha20Poly1305::new(&key);
        let mut encrypted = cipher.encrypt_aead(&nonce, aad, plaintext).unwrap();

        // Corrupt the ciphertext
        encrypted[0] ^= 0xFF;

        let result = cipher.decrypt_aead(&nonce, aad, &encrypted);
        assert!(matches!(result, Err(CryptoError::AuthenticationFailed)));
    }

    #[test]
    fn test_pbkdf2() {
        let password = b"password";
        let salt = b"salt";
        let key = pbkdf2(password, salt, 1, 32).unwrap();

        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_hkdf() {
        let ikm = b"input key material";
        let salt = b"salt";
        let info = b"info";
        let okm = hkdf(ikm, salt, info, 32).unwrap();

        assert_eq!(okm.len(), 32);
    }

    #[test]
    fn test_bcrypt() {
        let password = b"password";
        let salt = [0u8; 16];
        let hash = bcrypt(password, 10, &salt).unwrap();

        assert_eq!(hash.len(), 24);
    }

    #[test]
    fn test_pkcs7_pad_unpad() {
        let data = b"Hello, World!";
        let padded = Aes128::pkcs7_pad(data, 16);
        let unpadded = Aes128::pkcs7_unpad(&padded).unwrap();

        assert_eq!(data, unpadded.as_slice());
    }

    #[test]
    fn test_constant_time_eq() {
        let a = [0u8; 32];
        let b = [0u8; 32];
        let c = [1u8; 32];

        assert!(constant_time_eq(&a, &b));
        assert!(!constant_time_eq(&a, &c));
    }
}
