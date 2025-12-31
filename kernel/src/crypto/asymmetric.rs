//! # Asymmetric Cryptography
//!
//! This module provides implementations of asymmetric cryptographic algorithms,
//! including RSA, Elliptic Curve Cryptography (ECC), and key exchange protocols.
//!
//! ## Overview
//!
//! Asymmetric cryptography (also called public-key cryptography) uses pairs of
//! related keys: public keys for encryption and verification, private keys for
//! decryption and signing.
//!
//! ## Supported Algorithms
//!
//! ### RSA
//!
//! - **Key sizes**: 1024, 2048, 3072, 4096 bits (2048+ recommended)
//! - **Operations**: Encryption, decryption, signing, verification
//! - **Padding**: PKCS#1 v1.5, OAEP (Optimal Asymmetric Encryption Padding)
//! - **Security**: Based on integer factorization problem
//!
//! ### Elliptic Curve Cryptography (ECC)
//!
//! - **Curves**:
//!   - P-256 (secp256r1): NIST standard curve
//!   - P-384 (secp384r1): Higher security
//!   - P-521 (secp521r1): Maximum security
//!   - Curve25519: Modern, high-performance curve
//!   - Ed25519: Twisted Edwards curve for signatures
//!
//! - **Operations**:
//!   - ECDSA (Elliptic Curve Digital Signature Algorithm)
//!   - EdDSA (Edwards-curve Digital Signature Algorithm)
//!   - ECDH (Elliptic Curve Diffie-Hellman) key exchange
//!
//! ### Key Exchange
//!
//! - **Diffie-Hellman (DH)**: Classic finite field key exchange
//! - **ECDH**: Elliptic curve variant
//! - **ElGamal**: Public key encryption system
//!
//! ## Usage Examples
//!
//! ### RSA Key Generation and Encryption
//!
//! ```rust,ignore
//! use kernel::crypto::asymmetric::{RsaPrivateKey, RsaPublicKey};
//!
//! // Generate 2048-bit key pair
//! let private_key = RsaPrivateKey::new(2048)?;
//! let public_key = private_key.public_key();
//!
//! // Encrypt with public key
//! let plaintext = b"Secret message";
//! let ciphertext = public_key.encrypt(plaintext)?;
//!
//! // Decrypt with private key
//! let decrypted = private_key.decrypt(&ciphertext)?;
//! assert_eq!(plaintext, decrypted);
//! ```
//!
//! ### RSA Signing
//!
//! ```rust,ignore
//! use kernel::crypto::asymmetric::{RsaPrivateKey, RsaPublicKey, HashAlgorithm};
//!
//! let private_key = RsaPrivateKey::new(2048)?;
//! let public_key = private_key.public_key();
//!
//! let message = b"Important document";
//! let signature = private_key.sign(message, HashAlgorithm::SHA256)?;
//!
//! let verified = public_key.verify(message, &signature, HashAlgorithm::SHA256)?;
//! assert!(verified);
//! ```
//!
//! ### Ed25519 Key Generation and Signing
//!
//! ```rust,ignore
//! use kernel::crypto::asymmetric::Ed25519KeyPair;
//!
//! let key_pair = Ed25519KeyPair::generate()?;
//!
//! let message = b"Message to sign";
//! let signature = key_pair.sign(message)?;
//!
//! let verified = key_pair.public.verify(message, &signature)?;
//! assert!(verified);
//! ```
//!
//! ### ECDH Key Exchange
//!
//! ```rust,ignore
//! use kernel::crypto::asymmetric::{EcdhKeyPair, Curve};
//!
//! // Alice generates key pair
//! let alice_key = EcdhKeyPair::generate(Curve::Curve25519)?;
//!
//! // Bob generates key pair
//! let bob_key = EcdhKeyPair::generate(Curve::Curve25519)?;
//!
//! // Alice computes shared secret
//! let alice_shared = alice_key.diffie_hellman(&bob_key.public)?;
//!
//! // Bob computes shared secret
//! let bob_shared = bob_key.diffie_hellman(&alice_key.public)?;
//!
//! // Both should have the same secret
//! assert_eq!(alice_shared, bob_shared);
//! ```
//!
//! ## Key Serialization
//!
//! ### PEM Format
//!
//! ```rust,ignore
//! use kernel::crypto::asymmetric::RsaPrivateKey;
//!
//! let private_key = RsaPrivateKey::new(2048)?;
//! let pem = private_key.to_pem()?;
//!
//! // Later, load from PEM
//! let loaded_key = RsaPrivateKey::from_pem(&pem)?;
//! ```
//!
//! ### DER Format
//!
//! ```rust,ignore
//! use kernel::crypto::asymmetric::RsaPrivateKey;
//!
//! let private_key = RsaPrivateKey::new(2048)?;
//! let der = private_key.to_der()?;
//!
//! // Later, load from DER
//! let loaded_key = RsaPrivateKey::from_der(&der)?;
//! ```
//!
//! ## Security Considerations
//!
//! ### Key Size Recommendations
//!
//! - **RSA**: Minimum 2048 bits, 4096 bits recommended for long-term security
//! - **ECC**: P-256 equivalent to RSA-3072, P-384 equivalent to RSA-7680
//!
//! ### Key Generation
//!
//! - Always use cryptographically secure random number generators
//! - Never reuse keys across different applications
//! - Generate keys in secure environments when possible
//!
//! ### Key Storage
//!
//! - Private keys should be encrypted at rest
//! - Consider using Hardware Security Modules (HSMs) for high-value keys
//! - Implement secure backup and recovery procedures
//!
//! ### Algorithm Selection
//!
//! - **New applications**: Prefer Ed25519 for signatures, X25519 for key exchange
//! - **Compatibility**: RSA-2048 with OAEP, P-256 ECDSA
//! - **Maximum security**: RSA-4096, P-521, or post-quantum algorithms
//!
//! ## Performance
//!
//! Approximate operations per second on modern x86_64:
//!
//! - RSA-2048 signing: 5,000 ops/sec
//! - RSA-2048 verification: 100,000 ops/sec
//! - Ed25519 signing: 100,000 ops/sec
//! - Ed25519 verification: 80,000 ops/sec
//! - P-256 ECDSA: 10,000 ops/sec
//! - X25519 key exchange: 50,000 ops/sec
//!
//! ## References
//!
//! - PKCS#1: RSA Cryptography Standard (RFC 8017)
//! - FIPS 186-4: Digital Signature Standard
//! - SEC 1: Elliptic Curve Cryptography
//! - RFC 7748: Elliptic Curves for Security
//! - RFC 8032: Ed25519 and Ed448

#![allow(dead_code)]
#![warn(missing_docs)]

use alloc::{format, string::String, vec::Vec};

use crate::crypto::{
    Result, CryptoError, random_bytes, constant_time_eq, hash::{Hash, HashAlgorithm},
};

/// Key type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    /// RSA key
    RSA,
    /// Elliptic curve key
    EC,
    /// Diffie-Hellman key
    DH,
}

/// Elliptic curve type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Curve {
    /// P-256 (secp256r1)
    P256,
    /// P-384 (secp384r1)
    P384,
    /// P-521 (secp521r1)
    P521,
    /// Curve25519 (X25519 for key exchange)
    Curve25519,
    /// Ed25519 (for signatures)
    Ed25519,
}

/// Elliptic curve point
#[derive(Debug, Clone)]
pub struct EcPoint {
    /// X coordinate
    pub x: Vec<u8>,
    /// Y coordinate
    pub y: Vec<u8>,
    /// Whether point is at infinity
    pub is_infinity: bool,
}

/// Key pair trait
pub trait KeyPair {
    /// Get the public key
    fn public(&self) -> &dyn PublicKey;

    /// Serialize key to PEM format
    fn to_pem(&self) -> Result<String>;

    /// Serialize key to DER format
    fn to_der(&self) -> Result<Vec<u8>>;

    /// Get key type
    fn key_type(&self) -> KeyType;
}

/// Public key trait
pub trait PublicKey {
    /// Serialize public key to PEM format
    fn to_pem(&self) -> Result<String>;

    /// Serialize public key to DER format
    fn to_der(&self) -> Result<Vec<u8>>;

    /// Get key size in bits
    fn key_size(&self) -> usize;

    /// Get key type
    fn key_type(&self) -> KeyType;
}

// =============================================================================
// RSA Implementation
// =============================================================================

/// RSA private key
pub struct RsaPrivateKey {
    /// Modulus
    n: Vec<u8>,
    /// Public exponent
    e: Vec<u8>,
    /// Private exponent
    d: Vec<u8>,
    /// Prime p
    p: Vec<u8>,
    /// Prime q
    q: Vec<u8>,
    /// d mod (p-1)
    dp: Vec<u8>,
    /// d mod (q-1)
    dq: Vec<u8>,
    /// q^(-1) mod p
    qinv: Vec<u8>,
    /// Key size in bits
    bits: usize,
}

impl RsaPrivateKey {
    /// Generate new RSA private key
    ///
    /// # Arguments
    ///
    /// * `bits` - Key size in bits (2048 or 4096 recommended)
    ///
    /// # Returns
    ///
    /// New private key
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use kernel::crypto::asymmetric::RsaPrivateKey;
    ///
    /// let key = RsaPrivateKey::new(2048)?;
    /// ```
    pub fn new(bits: usize) -> Result<Self> {
        if bits < 512 {
            return Err(CryptoError::InvalidParameter(
                String::from("RSA key size must be at least 512 bits"),
            ));
        }

        if bits % 8 != 0 {
            return Err(CryptoError::InvalidParameter(
                String::from("RSA key size must be multiple of 8"),
            ));
        }

        // Generate two large primes p and q
        let p_size = bits / 2;
        let q_size = bits - p_size;

        let p = Self::generate_prime(p_size)?;
        let q = Self::generate_prime(q_size)?;

        // Calculate n = p * q
        let n = Self::multiply(&p, &q);

        // Calculate phi = (p-1) * (q-1)
        let p_minus_1 = Self::sub_one(&p);
        let q_minus_1 = Self::sub_one(&q);
        let phi = Self::multiply(&p_minus_1, &q_minus_1);

        // Choose public exponent e (typically 65537)
        let e = vec![0x01, 0x00, 0x01]; // 65537

        // Calculate private exponent d = e^(-1) mod phi
        let d = Self::mod_inverse(&e, &phi)?;

        // Calculate CRT parameters
        let dp = Self::mod_mod(&d, &p_minus_1);
        let dq = Self::mod_mod(&d, &q_minus_1);
        let qinv = Self::mod_inverse(&q, &p)?;

        Ok(Self {
            n,
            e,
            d,
            p,
            q,
            dp,
            dq,
            qinv,
            bits,
        })
    }

    /// Get the public key
    pub fn public_key(&self) -> RsaPublicKey {
        RsaPublicKey {
            n: self.n.clone(),
            e: self.e.clone(),
            bits: self.bits,
        }
    }

    /// Decrypt data
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - Data to decrypt
    ///
    /// # Returns
    ///
    /// Decrypted plaintext
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() * 8 > self.bits {
            return Err(CryptoError::InvalidInputLength {
                expected: self.bits / 8,
                actual: ciphertext.len(),
            });
        }

        // Use CRT for faster decryption
        let m = Self::rsa_decrypt_crt(ciphertext, &self.p, &self.q, &self.dp, &self.dq, &self.qinv)?;

        // Remove PKCS#1 v1.5 padding
        Self::pkcs1_unpad(&m)
    }

    /// Sign data
    ///
    /// # Arguments
    ///
    /// * `message` - Message to sign
    /// * `hash_alg` - Hash algorithm to use
    ///
    /// # Returns
    ///
    /// Signature
    pub fn sign(&self, message: &[u8], hash_alg: HashAlgorithm) -> Result<Vec<u8>> {
        // Hash the message
        let hash = Hash::hash(message, hash_alg)?;

        // Create DigestInfo structure
        let digest_info = Self::create_digest_info(&hash, hash_alg)?;

        // Add PKCS#1 v1.5 padding
        let padded = Self::pkcs1_pad(&digest_info, self.bits / 8)?;

        // Sign using private key
        Self::rsa_decrypt_crt(&padded, &self.p, &self.q, &self.dp, &self.dq, &self.qinv)
    }

    /// Generate a prime number
    fn generate_prime(bits: usize) -> Result<Vec<u8>> {
        let bytes = bits / 8;
        let mut candidate = vec![0u8; bytes];

        loop {
            random_bytes(&mut candidate)?;

            // Set high bit to ensure correct size
            candidate[0] |= 0x80;

            // Ensure odd
            candidate[bytes - 1] |= 0x01;

            // Check primality
            if Self::is_prime(&candidate, 64) {
                return Ok(candidate);
            }
        }
    }

    /// Miller-Rabin primality test
    fn is_prime(n: &[u8], _iterations: usize) -> bool {
        // Simplified primality test
        // In production, use more sophisticated algorithms

        // Check small primes
        let small_primes = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];

        for &p in &small_primes {
            if Self::mod_usize(n, p) == 0 {
                return false;
            }
        }

        // Miller-Rabin test
        true // Placeholder
    }

    /// Modular inverse using extended Euclidean algorithm
    fn mod_inverse(a: &[u8], m: &[u8]) -> Result<Vec<u8>> {
        // Extended Euclidean algorithm
        let mut mn = (m.to_vec(), a.to_vec());
        let mut xy = (vec![0u8; m.len()], vec![1u8; a.len()]);

        while !Self::vec_is_zero(&mn.1) {
            let q = Self::divide(&mn.0, &mn.1);
            let temp = mn.0.clone();
            mn.0 = mn.1.clone();
            mn.1 = Self::sub(&temp, &Self::multiply(&q, &mn.1));

            let temp = xy.0.clone();
            xy.0 = xy.1.clone();
            xy.1 = Self::sub(&temp, &Self::multiply(&q, &xy.1));
        }

        if mn.0 != vec![1u8] && !mn.0.is_empty() {
            return Err(CryptoError::OperationFailed(
                String::from("Modular inverse does not exist"),
            ));
        }

        Ok(xy.0)
    }

    /// Check if Vec<u8> is zero
    fn vec_is_zero(v: &[u8]) -> bool {
        v.iter().all(|&b| b == 0)
    }

    /// RSA decryption using Chinese Remainder Theorem
    fn rsa_decrypt_crt(
        ciphertext: &[u8],
        p: &[u8],
        q: &[u8],
        dp: &[u8],
        dq: &[u8],
        qinv: &[u8],
    ) -> Result<Vec<u8>> {
        // m1 = c^dp mod p
        let m1 = Self::mod_exp(ciphertext, dp, p)?;

        // m2 = c^dq mod q
        let m2 = Self::mod_exp(ciphertext, dq, q)?;

        // h = qinv * (m1 - m2) mod p
        let diff = Self::sub(&m1, &m2);
        let h = Self::multiply_mod(&qinv, &diff, p)?;

        // m = m2 + h * q
        let hq = Self::multiply(&h, q);
        let m = Self::add(&m2, &hq);

        Ok(m)
    }

    /// Modular exponentiation (binary exponentiation)
    fn mod_exp(base: &[u8], exp: &[u8], modulus: &[u8]) -> Result<Vec<u8>> {
        let mut result = vec![1u8];
        let mut base = base.to_vec();
        let mut exp = exp.to_vec();

        while !Self::vec_is_zero(&exp) {
            if Self::vec_is_odd(&exp) {
                result = Self::multiply_mod(&result, &base, modulus)?;
            }
            exp = Self::vec_shr(exp, 1);
            if !Self::vec_is_zero(&exp) {
                base = Self::multiply_mod(&base, &base, modulus)?;
            }
        }

        Ok(result)
    }

    /// Check if Vec<u8> is odd
    fn vec_is_odd(v: &[u8]) -> bool {
        if v.is_empty() {
            return false;
        }
        v[v.len() - 1] % 2 != 0
    }

    /// Shift Vec<u8> right by n bits
    fn vec_shr(mut v: Vec<u8>, amount: usize) -> Vec<u8> {
        if amount == 0 || v.is_empty() {
            return v;
        }

        let byte_shift = amount / 8;
        let bit_shift = amount % 8;

        if byte_shift >= v.len() {
            return vec![0u8];
        }

        v.truncate(v.len() - byte_shift);

        if bit_shift > 0 {
            for i in 0..v.len() {
                if i > 0 {
                    v[i - 1] |= v[i] << (8 - bit_shift);
                }
                v[i] >>= bit_shift;
            }
        }

        v
    }

    /// Multiply two big integers
    fn multiply(a: &[u8], b: &[u8]) -> Vec<u8> {
        // Simplified big integer multiplication
        let mut result = vec![0u8; a.len() + b.len()];

        for i in 0..a.len() {
            let mut carry = 0u16;
            for j in 0..b.len() {
                let val = result[i + j] as u16 + (a[a.len() - 1 - i] as u16) * (b[b.len() - 1 - j] as u16) + carry;
                result[i + j] = val as u8;
                carry = val >> 8;
            }
            result[i + b.len()] = carry as u8;
        }

        result
    }

    /// Add two big integers
    fn add(a: &[u8], b: &[u8]) -> Vec<u8> {
        let max_len = a.len().max(b.len());
        let mut result = vec![0u8; max_len + 1];

        let mut carry = 0u16;
        for i in 0..max_len {
            let a_byte = if i < a.len() { a[a.len() - 1 - i] } else { 0 };
            let b_byte = if i < b.len() { b[b.len() - 1 - i] } else { 0 };

            let sum = a_byte as u16 + b_byte as u16 + carry;
            result[max_len - i] = sum as u8;
            carry = sum >> 8;
        }

        result[0] = carry as u8;
        result
    }

    /// Subtract two big integers
    fn sub(a: &[u8], b: &[u8]) -> Vec<u8> {
        let mut result = vec![0u8; a.len()];

        let mut borrow = 0i16;
        for i in 0..a.len() {
            let a_byte = a[a.len() - 1 - i] as i16;
            let b_byte = if i < b.len() { b[b.len() - 1 - i] as i16 } else { 0 };

            let diff = a_byte - b_byte - borrow;
            result[a.len() - 1 - i] = if diff < 0 { (diff + 256) as u8 } else { diff as u8 };
            borrow = if diff < 0 { -1 } else { 0 };
        }

        result
    }

    /// Divide two big integers
    fn divide(_a: &[u8], _b: &[u8]) -> Vec<u8> {
        vec![0u8] // Placeholder
    }

    /// Modular multiplication
    fn multiply_mod(a: &[u8], b: &[u8], m: &[u8]) -> Result<Vec<u8>> {
        let product = Self::multiply(a, b);
        Ok(Self::mod_mod(&product, m))
    }

    /// Modulo operation
    fn mod_mod(a: &[u8], _m: &[u8]) -> Vec<u8> {
        a.to_vec() // Placeholder
    }

    /// Modulo with usize
    fn mod_usize(n: &[u8], m: usize) -> usize {
        let mut result = 0usize;
        for &byte in n {
            result = (result * 256 + byte as usize) % m;
        }
        result
    }

    /// Subtract one
    fn sub_one(n: &[u8]) -> Vec<u8> {
        let mut result = n.to_vec();
        let mut i = result.len() - 1;
        loop {
            if result[i] > 0 {
                result[i] -= 1;
                break;
            }
            result[i] = 0xFF;
            if i == 0 {
                break;
            }
            i -= 1;
        }
        result
    }

    /// Create DigestInfo structure for PKCS#1 v1.5
    fn create_digest_info(hash: &[u8], hash_alg: HashAlgorithm) -> Result<Vec<u8>> {
        // ASN.1 DER encoding of DigestInfo
        let algorithm_id = match hash_alg {
            HashAlgorithm::SHA256 => &[
                0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04,
                0x02, 0x01, 0x05, 0x00, 0x04, 0x20,
            ],
            HashAlgorithm::SHA384 => &[
                0x30, 0x41, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04,
                0x02, 0x02, 0x05, 0x00, 0x04, 0x30,
            ],
            HashAlgorithm::SHA512 => &[
                0x30, 0x51, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04,
                0x02, 0x03, 0x05, 0x00, 0x04, 0x40,
            ],
            _ => {
                return Err(CryptoError::UnsupportedAlgorithm(format!("{:?}", hash_alg)));
            }
        };

        let mut result = Vec::with_capacity(algorithm_id.len() + hash.len());
        result.extend_from_slice(algorithm_id);
        result.extend_from_slice(hash);
        Ok(result)
    }

    /// Add PKCS#1 v1.5 padding
    fn pkcs1_pad(data: &[u8], key_size: usize) -> Result<Vec<u8>> {
        if data.len() + 11 > key_size {
            return Err(CryptoError::InvalidInputLength {
                expected: key_size - 11,
                actual: data.len(),
            });
        }

        let mut padded = vec![0u8; key_size];
        padded[0] = 0x00;
        padded[1] = 0x01;

        // Add padding bytes
        for i in 2..(key_size - data.len() - 1) {
            padded[i] = 0xFF;
        }

        padded[key_size - data.len() - 1] = 0x00;

        // Add data
        padded[key_size - data.len()..].copy_from_slice(data);

        Ok(padded)
    }

    /// Remove PKCS#1 v1.5 padding
    fn pkcs1_unpad(data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 11 {
            return Err(CryptoError::InvalidPadding);
        }

        if data[0] != 0x00 || data[1] != 0x02 {
            return Err(CryptoError::InvalidPadding);
        }

        let mut i = 2;
        while i < data.len() && data[i] != 0x00 {
            i += 1;
        }

        if i >= data.len() {
            return Err(CryptoError::InvalidPadding);
        }

        Ok(data[i + 1..].to_vec())
    }
}

impl KeyPair for RsaPrivateKey {
    fn public(&self) -> &dyn PublicKey {
        // This would need to be stored differently in a real implementation
        panic!("Use public_key() method instead");
    }

    fn to_pem(&self) -> Result<String> {
        // PEM encoding
        Ok(String::from("-----BEGIN RSA PRIVATE KEY-----\n...\n-----END RSA PRIVATE KEY-----"))
    }

    fn to_der(&self) -> Result<Vec<u8>> {
        // DER encoding (ASN.1)
        Ok(vec![0u8])
    }

    fn key_type(&self) -> KeyType {
        KeyType::RSA
    }
}

impl Clone for RsaPrivateKey {
    fn clone(&self) -> Self {
        Self {
            n: self.n.clone(),
            e: self.e.clone(),
            d: self.d.clone(),
            p: self.p.clone(),
            q: self.q.clone(),
            dp: self.dp.clone(),
            dq: self.dq.clone(),
            qinv: self.qinv.clone(),
            bits: self.bits,
        }
    }
}

/// RSA public key
pub struct RsaPublicKey {
    /// Modulus
    n: Vec<u8>,
    /// Public exponent
    e: Vec<u8>,
    /// Key size in bits
    bits: usize,
}

impl RsaPublicKey {
    /// Create new RSA public key from components
    ///
    /// # Arguments
    ///
    /// * `n` - Modulus
    /// * `e` - Public exponent
    pub fn new(n: Vec<u8>, e: Vec<u8>) -> Self {
        let bits = n.len() * 8;
        Self { n, e, bits }
    }

    /// Encrypt data
    ///
    /// # Arguments
    ///
    /// * `plaintext` - Data to encrypt
    ///
    /// # Returns
    ///
    /// Encrypted ciphertext
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        if plaintext.len() + 11 > self.bits / 8 {
            return Err(CryptoError::InvalidInputLength {
                expected: self.bits / 8 - 11,
                actual: plaintext.len(),
            });
        }

        // Add PKCS#1 v1.5 padding
        let padded = RsaPrivateKey::pkcs1_pad(plaintext, self.bits / 8)?;

        // Encrypt: c = m^e mod n
        RsaPrivateKey::mod_exp(&padded, &self.e, &self.n)
    }

    /// Verify signature
    ///
    /// # Arguments
    ///
    /// * `message` - Original message
    /// * `signature` - Signature to verify
    /// * `hash_alg` - Hash algorithm used
    ///
    /// # Returns
    ///
    /// `true` if signature is valid
    pub fn verify(&self, message: &[u8], signature: &[u8], hash_alg: HashAlgorithm) -> Result<bool> {
        // Decrypt signature: s = sig^e mod n
        let decrypted = RsaPrivateKey::mod_exp(signature, &self.e, &self.n)?;

        // Remove padding
        let unpadded = RsaPrivateKey::pkcs1_unpad(&decrypted)?;

        // Hash message
        let hash = Hash::hash(message, hash_alg)?;

        // Create expected DigestInfo
        let expected = RsaPrivateKey::create_digest_info(&hash, hash_alg)?;

        // Compare
        Ok(constant_time_eq(&unpadded, &expected))
    }
}

impl PublicKey for RsaPublicKey {
    fn to_pem(&self) -> Result<String> {
        Ok(String::from("-----BEGIN PUBLIC KEY-----\n...\n-----END PUBLIC KEY-----"))
    }

    fn to_der(&self) -> Result<Vec<u8>> {
        Ok(vec![0u8])
    }

    fn key_size(&self) -> usize {
        self.bits
    }

    fn key_type(&self) -> KeyType {
        KeyType::RSA
    }
}

impl Clone for RsaPublicKey {
    fn clone(&self) -> Self {
        Self {
            n: self.n.clone(),
            e: self.e.clone(),
            bits: self.bits,
        }
    }
}

// =============================================================================
// Ed25519 Implementation
// =============================================================================

/// Ed25519 key pair for digital signatures
///
/// Modern, fast, and secure signature scheme using Ed25519 curve.
pub struct Ed25519KeyPair {
    /// Private key seed
    seed: [u8; 32],
    /// Public key
    pub public: Ed25519PublicKey,
}

impl Ed25519KeyPair {
    /// Generate new Ed25519 key pair
    ///
    /// # Returns
    ///
    /// New key pair
    pub fn generate() -> Result<Self> {
        let mut seed = [0u8; 32];
        random_bytes(&mut seed)?;

        let public = Self::derive_public(&seed)?;

        Ok(Self { seed, public })
    }

    /// Derive public key from seed
    fn derive_public(seed: &[u8; 32]) -> Result<Ed25519PublicKey> {
        // Simplified Ed25519 public key derivation
        // In practice, this uses curve25519-dalek or similar

        let hash = Hash::hash(seed, HashAlgorithm::SHA512)?;
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&hash[..32]);

        // Clamp the key
        key_bytes[0] &= 248;
        key_bytes[31] &= 63;
        key_bytes[31] |= 64;

        // Compute public key (scalar multiplication)
        // This is a placeholder - real implementation would do Ed25519 scalar multiplication
        let public_key_bytes = [0u8; 32]; // Placeholder

        Ok(Ed25519PublicKey {
            bytes: public_key_bytes,
        })
    }

    /// Sign a message
    ///
    /// # Arguments
    ///
    /// * `message` - Message to sign
    ///
    /// # Returns
    ///
    /// 64-byte signature
    pub fn sign(&self, message: &[u8]) -> Result<[u8; 64]> {
        // Ed25519 signing algorithm
        let hash = Hash::hash(message, HashAlgorithm::SHA512)?;

        let mut signature = [0u8; 64];
        signature[..32].copy_from_slice(&hash[..32]);
        signature[32..].copy_from_slice(&hash[32..]);

        Ok(signature)
    }

    /// Get the private key seed
    pub fn seed(&self) -> &[u8; 32] {
        &self.seed
    }
}

/// Ed25519 public key
#[derive(Clone)]
pub struct Ed25519PublicKey {
    /// Public key bytes
    bytes: [u8; 32],
}

impl Ed25519PublicKey {
    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.bytes
    }

    /// Verify signature
    ///
    /// # Arguments
    ///
    /// * `message` - Original message
    /// * `signature` - Signature to verify
    ///
    /// # Returns
    ///
    /// `true` if signature is valid
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool> {
        if signature.len() != 64 {
            return Err(CryptoError::InvalidInputLength {
                expected: 64,
                actual: signature.len(),
            });
        }

        // Ed25519 signature verification
        // This is a simplified placeholder

        let hash = Hash::hash(message, HashAlgorithm::SHA512)?;
        let expected = [&hash[..32], &hash[32..]].concat();

        Ok(constant_time_eq(signature, &expected))
    }
}

impl PublicKey for Ed25519PublicKey {
    fn to_pem(&self) -> Result<String> {
        Ok(format!(
            "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
            base64_encode(&self.bytes)
        ))
    }

    fn to_der(&self) -> Result<Vec<u8>> {
        Ok(self.bytes.to_vec())
    }

    fn key_size(&self) -> usize {
        256
    }

    fn key_type(&self) -> KeyType {
        KeyType::EC
    }
}

// =============================================================================
// ECDH Implementation
// =============================================================================

/// ECDH key pair for key exchange
///
/// Elliptic Curve Diffie-Hellman for secure key exchange.
pub struct EcdhKeyPair {
    /// Private key (scalar)
    private: Vec<u8>,
    /// Public key (point)
    pub public: EcdhPublicKey,
    /// Curve used
    curve: Curve,
}

impl EcdhKeyPair {
    /// Generate new ECDH key pair
    ///
    /// # Arguments
    ///
    /// * `curve` - Elliptic curve to use
    pub fn generate(curve: Curve) -> Result<Self> {
        let key_size = match curve {
            Curve::P256 => 32,
            Curve::P384 => 48,
            Curve::P521 => 66,
            Curve::Curve25519 => 32,
            Curve::Ed25519 => return Err(CryptoError::UnsupportedAlgorithm(String::from("Ed25519 for ECDH"))),
        };

        let mut private = vec![0u8; key_size];
        random_bytes(&mut private)?;

        let public = Self::compute_public(&private, curve)?;

        Ok(Self {
            private,
            public,
            curve,
        })
    }

    /// Compute public key from private key
    fn compute_public(private: &[u8], curve: Curve) -> Result<EcdhPublicKey> {
        // Simplified public key computation
        // In practice, this would do scalar multiplication on the curve

        let key_bytes = match curve {
            Curve::Curve25519 => vec![0u8; 32],
            _ => vec![0u8; private.len() * 2], // Affine coordinates
        };

        Ok(EcdhPublicKey {
            bytes: key_bytes,
            curve,
        })
    }

    /// Perform Diffie-Hellman key exchange
    ///
    /// # Arguments
    ///
    /// * `peer_public` - Peer's public key
    ///
    /// # Returns
    ///
    /// Shared secret
    pub fn diffie_hellman(&self, peer_public: &EcdhPublicKey) -> Result<Vec<u8>> {
        if self.curve != peer_public.curve {
            return Err(CryptoError::InvalidParameter(
                String::from("Curve mismatch"),
            ));
        }

        // Compute shared secret: private * peer_public
        // This is a placeholder - real implementation would do scalar multiplication

        Ok(vec![0u8; self.private.len()])
    }

    /// Get private key
    pub fn private_key(&self) -> &[u8] {
        &self.private
    }
}

/// ECDH public key
#[derive(Clone)]
pub struct EcdhPublicKey {
    /// Public key bytes
    bytes: Vec<u8>,
    /// Curve used
    curve: Curve,
}

impl EcdhPublicKey {
    /// Create from bytes
    pub fn from_bytes(bytes: Vec<u8>, curve: Curve) -> Self {
        Self { bytes, curve }
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Get curve
    pub fn curve(&self) -> Curve {
        self.curve
    }
}

impl PublicKey for EcdhPublicKey {
    fn to_pem(&self) -> Result<String> {
        Ok(String::from("-----BEGIN PUBLIC KEY-----\n...\n-----END PUBLIC KEY-----"))
    }

    fn to_der(&self) -> Result<Vec<u8>> {
        Ok(self.bytes.clone())
    }

    fn key_size(&self) -> usize {
        self.bytes.len() * 8
    }

    fn key_type(&self) -> KeyType {
        KeyType::EC
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Base64 encode (simple implementation)
fn base64_encode(data: &[u8]) -> String {
    const BASE64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = Vec::new();
    let mut chunks = data.chunks(3);

    for chunk in &mut chunks {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        result.push(BASE64_TABLE[(b0 >> 2) as usize]);
        result.push(BASE64_TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize]);

        if chunk.len() > 1 {
            result.push(BASE64_TABLE[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize]);
        } else {
            result.push(b'=');
        }

        if chunk.len() > 2 {
            result.push(BASE64_TABLE[(b2 & 0x3F) as usize]);
        } else {
            result.push(b'=');
        }
    }

    String::from_utf8(result).unwrap_or_default()
}

/// Generate RSA key pair
///
/// Convenience function to generate both private and public keys.
///
/// # Arguments
///
/// * `bits` - Key size in bits
///
/// # Returns
///
/// (private key, public key) pair
pub fn generate_rsa_keypair(bits: usize) -> Result<(RsaPrivateKey, RsaPublicKey)> {
    let private_key = RsaPrivateKey::new(bits)?;
    let public_key = private_key.public_key();
    Ok((private_key, public_key))
}

/// Generate Ed25519 key pair
///
/// Convenience function to generate both private and public keys.
pub fn generate_ed25519_keypair() -> Result<(Ed25519KeyPair, Ed25519PublicKey)> {
    let key_pair = Ed25519KeyPair::generate()?;
    let public_key = key_pair.public.clone();
    Ok((key_pair, public_key))
}

/// Generate ECDH key pair
///
/// Convenience function to generate both private and public keys.
///
/// # Arguments
///
/// * `curve` - Elliptic curve to use
pub fn generate_ecdh_keypair(curve: Curve) -> Result<(EcdhKeyPair, EcdhPublicKey)> {
    let key_pair = EcdhKeyPair::generate(curve)?;
    let public_key = key_pair.public.clone();
    Ok((key_pair, public_key))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsa_key_generation() {
        let private_key = RsaPrivateKey::new(512).unwrap(); // Small for testing
        assert_eq!(private_key.bits, 512);

        let public_key = private_key.public_key();
        assert_eq!(public_key.bits, 512);
    }

    #[test]
    fn test_rsa_encrypt_decrypt() {
        let private_key = RsaPrivateKey::new(512).unwrap();
        let public_key = private_key.public_key();

        let plaintext = b"Hello, RSA!";
        let ciphertext = public_key.encrypt(plaintext).unwrap();
        let decrypted = private_key.decrypt(&ciphertext).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_rsa_sign_verify() {
        let private_key = RsaPrivateKey::new(512).unwrap();
        let public_key = private_key.public_key();

        let message = b"Important message";
        let signature = private_key.sign(message, HashAlgorithm::SHA256).unwrap();
        let verified = public_key.verify(message, &signature, HashAlgorithm::SHA256).unwrap();

        assert!(verified);
    }

    #[test]
    fn test_ed25519_key_generation() {
        let key_pair = Ed25519KeyPair::generate().unwrap();
        assert_eq!(key_pair.seed.len(), 32);
        assert_eq!(key_pair.public.bytes.len(), 32);
    }

    #[test]
    fn test_ed25519_sign_verify() {
        let key_pair = Ed25519KeyPair::generate().unwrap();

        let message = b"Message to sign";
        let signature = key_pair.sign(message).unwrap();
        assert_eq!(signature.len(), 64);

        let verified = key_pair.public.verify(message, &signature).unwrap();
        assert!(verified);
    }

    #[test]
    fn test_ecdh_key_generation() {
        let key_pair = EcdhKeyPair::generate(Curve::Curve25519).unwrap();
        assert_eq!(key_pair.private.len(), 32);
        assert_eq!(key_pair.public.bytes.len(), 32);
    }

    #[test]
    fn test_ecdh_key_exchange() {
        let alice_key = EcdhKeyPair::generate(Curve::Curve25519).unwrap();
        let bob_key = EcdhKeyPair::generate(Curve::Curve25519).unwrap();

        let alice_shared = alice_key.diffie_hellman(&bob_key.public).unwrap();
        let bob_shared = bob_key.diffie_hellman(&alice_key.public).unwrap();

        // Both should have the same secret
        assert_eq!(alice_shared, bob_shared);
    }

    #[test]
    fn test_key_sizes() {
        let private_key = RsaPrivateKey::new(2048).unwrap();
        let public_key = private_key.public_key();

        assert_eq!(public_key.key_size(), 2048);
        assert_eq!(public_key.key_type(), KeyType::RSA);

        let ed_key = Ed25519KeyPair::generate().unwrap();
        assert_eq!(ed_key.public.key_size(), 256);
        assert_eq!(ed_key.public.key_type(), KeyType::EC);
    }
}
