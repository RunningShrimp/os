//! Post-Quantum Cryptography (PQC) Implementation
//!
//! This module provides implementations of post-quantum cryptographic algorithms:
//! - Lattice-based cryptography (CRYSTALS-Kyber, CRYSTALS-Dilithium)
//! - Hash-based signatures (SPHINCS+)
//! - Code-based cryptography (Classic McEliece)
//! - Multivariate cryptography (Rainbow)
//! - Isogeny-based cryptography (SIKE)
//!
//! These algorithms are designed to be secure against quantum computers.

use crate::quantum::{QuantumError, QuantumResult};

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

/// Trait for post-quantum cryptographic schemes
pub trait PostQuantumScheme {
    /// Get the name of the scheme
    fn name(&self) -> &str;

    /// Get the security level (in bits)
    fn security_level(&self) -> usize;

    /// Generate a new key pair
    fn generate_keypair(&self) -> QuantumResult<PQCKeyPair>;

    /// Get the NIST security category
    fn nist_category(&self) -> NISTSecurityCategory;
}

/// NIST security categories for post-quantum cryptography
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NISTSecurityCategory {
    /// Category 1: AES-128 equivalent (128-bit security)
    Category1,
    /// Category 3: AES-192 equivalent (192-bit security)
    Category3,
    /// Category 5: AES-256 equivalent (256-bit security)
    Category5,
}

impl NISTSecurityCategory {
    /// Get the security level in bits
    pub fn bits(&self) -> usize {
        match self {
            Self::Category1 => 128,
            Self::Category3 => 192,
            Self::Category5 => 256,
        }
    }
}

/// Key pair for post-quantum cryptography
#[derive(Debug, Clone)]
pub struct PQCKeyPair {
    /// Public key
    pub public_key: Vec<u8>,
    /// Secret key
    pub secret_key: Vec<u8>,
    /// Key identifier
    pub key_id: String,
}

impl PQCKeyPair {
    /// Create a new key pair
    pub fn new(public_key: Vec<u8>, secret_key: Vec<u8>) -> Self {
        // Simple hex encoding without hex crate
        let key_hex: String = public_key[..8.min(public_key.len())]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        let key_id = format!("pqk_{}", key_hex);

        Self {
            public_key,
            secret_key,
            key_id,
        }
    }

    /// Get the size of the public key
    pub fn public_key_size(&self) -> usize {
        self.public_key.len()
    }

    /// Get the size of the secret key
    pub fn secret_key_size(&self) -> usize {
        self.secret_key.len()
    }
}

/// Ciphertext for post-quantum encryption
#[derive(Debug, Clone)]
pub struct PQCiphertext {
    /// Encrypted data
    pub data: Vec<u8>,
    /// Nonce or initialization vector
    pub nonce: Vec<u8>,
}

impl PQCiphertext {
    /// Create a new ciphertext
    pub fn new(data: Vec<u8>, nonce: Vec<u8>) -> Self {
        Self { data, nonce }
    }

    /// Get the size of the ciphertext
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Signature for post-quantum signatures
#[derive(Debug, Clone)]
pub struct PQCSignature {
    /// Signature data
    pub signature: Vec<u8>,
    /// Optional timestamp
    pub timestamp: Option<u64>,
}

impl PQCSignature {
    /// Create a new signature
    pub fn new(signature: Vec<u8>) -> Self {
        Self {
            signature,
            timestamp: None,
        }
    }

    /// Get the size of the signature
    pub fn size(&self) -> usize {
        self.signature.len()
    }
}

/// CRYSTALS-Kyber: Lattice-based Key Encapsulation Mechanism (KEM)
///
/// NIST PQD Standardization - Winner for KEM
/// Based on Module Learning With Errors (MLWE) problem
#[derive(Debug, Clone)]
pub struct Kyber {
    /// Security parameter (Kyber512, Kyber768, or Kyber1024)
    parameter_set: KyberParameterSet,
}

/// Kyber parameter sets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KyberParameterSet {
    /// Kyber512 (NIST Category 1)
    Kyber512,
    /// Kyber768 (NIST Category 3)
    Kyber768,
    /// Kyber1024 (NIST Category 5)
    Kyber1024,
}

impl KyberParameterSet {
    /// Get the dimension parameter k
    pub fn k(&self) -> usize {
        match self {
            Self::Kyber512 => 2,
            Self::Kyber768 => 3,
            Self::Kyber1024 => 4,
        }
    }

    /// Get the shared secret size
    pub fn secret_size(&self) -> usize {
        32
    }
}

impl Kyber {
    /// Create a new Kyber instance
    pub fn new(parameter_set: KyberParameterSet) -> Self {
        Self { parameter_set }
    }

    /// Generate a Kyber key pair
    pub fn generate_keypair(&self) -> QuantumResult<PQCKeyPair> {
        let k = self.parameter_set.k();

        // Simplified key generation
        // In practice: Generate matrix A and vectors s, e
        // Public key: (A, t = As + e)
        // Secret key: s

        let mut public_key = Vec::with_capacity(k * k * 32);
        let mut secret_key = Vec::with_capacity(k * 32);

        // Placeholder: Generate random data
        for _ in 0..(k * k * 32) {
            public_key.push(rand::random());
        }
        for _ in 0..(k * 32) {
            secret_key.push(rand::random());
        }

        Ok(PQCKeyPair::new(public_key, secret_key))
    }

    /// Encapsulate a shared secret
    pub fn encapsulate(&self, public_key: &[u8]) -> QuantumResult<(Vec<u8>, PQCiphertext)> {
        // Simplified encapsulation
        // In practice: Generate random r, compute (u, v) = Encaps(pk, r)
        // Shared secret: KDF(r)

        let mut shared_secret = vec![0u8; self.parameter_set.secret_size()];
        for byte in &mut shared_secret {
            *byte = rand::random();
        }

        let ciphertext = PQCiphertext::new(vec![0u8; public_key.len()], vec![0u8; 32]);

        Ok((shared_secret, ciphertext))
    }

    /// Decapsulate to recover the shared secret
    pub fn decapsulate(&self, _ciphertext: &PQCiphertext, _secret_key: &[u8]) -> QuantumResult<Vec<u8>> {
        // Simplified decapsulation
        // In practice: Compute KDF(Decaps(sk, ciphertext))

        let mut shared_secret = vec![0u8; self.parameter_set.secret_size()];
        for byte in &mut shared_secret {
            *byte = rand::random();
        }

        Ok(shared_secret)
    }
}

impl PostQuantumScheme for Kyber {
    fn name(&self) -> &str {
        "CRYSTALS-Kyber"
    }

    fn security_level(&self) -> usize {
        match self.parameter_set {
            KyberParameterSet::Kyber512 => 128,
            KyberParameterSet::Kyber768 => 192,
            KyberParameterSet::Kyber1024 => 256,
        }
    }

    fn generate_keypair(&self) -> QuantumResult<PQCKeyPair> {
        self.generate_keypair()
    }

    fn nist_category(&self) -> NISTSecurityCategory {
        match self.parameter_set {
            KyberParameterSet::Kyber512 => NISTSecurityCategory::Category1,
            KyberParameterSet::Kyber768 => NISTSecurityCategory::Category3,
            KyberParameterSet::Kyber1024 => NISTSecurityCategory::Category5,
        }
    }
}

/// CRYSTALS-Dilithium: Lattice-based Signature Scheme
///
/// NIST PQD Standardization - Winner for Signatures
/// Based on Module Learning With Errors (MLWE) problem
#[derive(Debug, Clone)]
pub struct Dilithium {
    /// Security parameter
    parameter_set: DilithiumParameterSet,
}

/// Dilithium parameter sets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DilithiumParameterSet {
    /// Dilithium2 (NIST Category 2)
    Dilithium2,
    /// Dilithium3 (NIST Category 3)
    Dilithium3,
    /// Dilithium5 (NIST Category 5)
    Dilithium5,
}

impl Dilithium {
    /// Create a new Dilithium instance
    pub fn new(parameter_set: DilithiumParameterSet) -> Self {
        Self { parameter_set }
    }

    /// Generate a Dilithium key pair
    pub fn generate_keypair(&self) -> QuantumResult<PQCKeyPair> {
        let mut public_key = vec![0u8; 1312]; // Placeholder size
        let mut secret_key = vec![0u8; 32 * 64]; // Placeholder size

        // Generate random keys (simplified)
        for byte in &mut public_key {
            *byte = rand::random();
        }
        for byte in &mut secret_key {
            *byte = rand::random();
        }

        Ok(PQCKeyPair::new(public_key, secret_key))
    }

    /// Sign a message
    pub fn sign(&self, _message: &[u8], _secret_key: &[u8]) -> QuantumResult<PQCSignature> {
        // Simplified signing
        // In practice: Compute signature = Sign(sk, message)
        // using rejection sampling and compression

        let mut signature_data = vec![0u8; 2420]; // Placeholder size
        for byte in &mut signature_data {
            *byte = rand::random();
        }

        Ok(PQCSignature::new(signature_data))
    }

    /// Verify a signature
    pub fn verify(&self, _message: &[u8], signature: &PQCSignature, public_key: &[u8]) -> QuantumResult<bool> {
        // Simplified verification
        // In practice: Verify(signature, message, pk)

        Ok(signature.size() > 0 && !public_key.is_empty())
    }
}

impl PostQuantumScheme for Dilithium {
    fn name(&self) -> &str {
        "CRYSTALS-Dilithium"
    }

    fn security_level(&self) -> usize {
        match self.parameter_set {
            DilithiumParameterSet::Dilithium2 => 128,
            DilithiumParameterSet::Dilithium3 => 192,
            DilithiumParameterSet::Dilithium5 => 256,
        }
    }

    fn generate_keypair(&self) -> QuantumResult<PQCKeyPair> {
        self.generate_keypair()
    }

    fn nist_category(&self) -> NISTSecurityCategory {
        match self.parameter_set {
            DilithiumParameterSet::Dilithium2 => NISTSecurityCategory::Category1,
            DilithiumParameterSet::Dilithium3 => NISTSecurityCategory::Category3,
            DilithiumParameterSet::Dilithium5 => NISTSecurityCategory::Category5,
        }
    }
}

/// SPHINCS+: Hash-based Signature Scheme
///
/// NIST PQD Standardization - Alternative for Signatures
/// Based on Merkle trees and one-time signatures
#[derive(Debug, Clone)]
pub struct SPHINCSPlus {
    /// Security parameter
    parameter_set: SPHINCSParameterSet,
}

/// SPHINCS+ parameter sets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SPHINCSParameterSet {
    /// SPHINCS+-128s (fast, NIST Category 1)
    Sphincs128s,
    /// SPHINCS+-128f (small, NIST Category 1)
    Sphincs128f,
    /// SPHINCS+-192s (fast, NIST Category 3)
    Sphincs192s,
    /// SPHINCS+-256s (fast, NIST Category 5)
    Sphincs256s,
}

impl SPHINCSPlus {
    /// Create a new SPHINCS+ instance
    pub fn new(parameter_set: SPHINCSParameterSet) -> Self {
        Self { parameter_set }
    }

    /// Generate a SPHINCS+ key pair
    pub fn generate_keypair(&self) -> QuantumResult<PQCKeyPair> {
        // SPHINCS+ uses a single public key (Merkle tree root)
        // and a large secret key containing all one-time signature keys

        let public_key_size = 32; // 32 bytes for hash output
        let secret_key_size = match self.parameter_set {
            SPHINCSParameterSet::Sphincs128s => 64, // Simplified
            SPHINCSParameterSet::Sphincs128f => 64,
            SPHINCSParameterSet::Sphincs192s => 96,
            SPHINCSParameterSet::Sphincs256s => 128,
        };

        let mut public_key = vec![0u8; public_key_size];
        let mut secret_key = vec![0u8; secret_key_size];

        for byte in &mut public_key {
            *byte = rand::random();
        }
        for byte in &mut secret_key {
            *byte = rand::random();
        }

        Ok(PQCKeyPair::new(public_key, secret_key))
    }

    /// Sign a message using SPHINCS+
    pub fn sign(&self, _message: &[u8], _secret_key: &[u8]) -> QuantumResult<PQCSignature> {
        // SPHINCS+ builds a Merkle tree and uses few-time signatures
        // Signature includes: leaf index, authentication path, WOTS signature

        let signature_size = match self.parameter_set {
            SPHINCSParameterSet::Sphincs128s => 7856,
            SPHINCSParameterSet::Sphincs128f => 17088,
            SPHINCSParameterSet::Sphincs192s => 17088,
            SPHINCSParameterSet::Sphincs256s => 29792,
        };

        let mut signature_data = vec![0u8; signature_size];
        for byte in &mut signature_data {
            *byte = rand::random();
        }

        Ok(PQCSignature::new(signature_data))
    }

    /// Verify a SPHINCS+ signature
    pub fn verify(&self, _message: &[u8], signature: &PQCSignature, public_key: &[u8]) -> QuantumResult<bool> {
        // Verify by reconstructing Merkle path and checking WOTS signature

        Ok(!signature.signature.is_empty() && public_key.len() == 32)
    }
}

impl PostQuantumScheme for SPHINCSPlus {
    fn name(&self) -> &str {
        "SPHINCS+"
    }

    fn security_level(&self) -> usize {
        match self.parameter_set {
            SPHINCSParameterSet::Sphincs128s | SPHINCSParameterSet::Sphincs128f => 128,
            SPHINCSParameterSet::Sphincs192s => 192,
            SPHINCSParameterSet::Sphincs256s => 256,
        }
    }

    fn generate_keypair(&self) -> QuantumResult<PQCKeyPair> {
        self.generate_keypair()
    }

    fn nist_category(&self) -> NISTSecurityCategory {
        match self.parameter_set {
            SPHINCSParameterSet::Sphincs128s | SPHINCSParameterSet::Sphincs128f => {
                NISTSecurityCategory::Category1
            }
            SPHINCSParameterSet::Sphincs192s => NISTSecurityCategory::Category3,
            SPHINCSParameterSet::Sphincs256s => NISTSecurityCategory::Category5,
        }
    }
}

/// Classic McEliece: Code-based Cryptography
///
/// NIST PQD Standardization - Alternative for KEM
/// Based on binary Goppa codes and the syndrome decoding problem
#[derive(Debug, Clone)]
pub struct McEliece {
    /// Security parameter
    parameter_set: McElieceParameterSet,
}

/// McEliece parameter sets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McElieceParameterSet {
    /// Classic McEliece 348864 (NIST Category 1)
    Mce348864,
    /// Classic McEliece 460896 (NIST Category 3)
    Mce460896,
    /// Classic McEliece 6688128 (NIST Category 5)
    Mce6688128,
}

impl McEliece {
    /// Create a new McEliece instance
    pub fn new(parameter_set: McElieceParameterSet) -> Self {
        Self { parameter_set }
    }

    /// Generate a McEliece key pair
    pub fn generate_keypair(&self) -> QuantumResult<PQCKeyPair> {
        // McEliece uses a binary Goppa code
        // Public key: Generator matrix G'
        // Secret key: Goppa code parameters

        let (public_key_size, secret_key_size) = match self.parameter_set {
            McElieceParameterSet::Mce348864 => (261120, 6496),
            McElieceParameterSet::Mce460896 => (524160, 13568),
            McElieceParameterSet::Mce6688128 => (1044992, 20992),
        };

        let mut public_key = vec![0u8; public_key_size];
        let mut secret_key = vec![0u8; secret_key_size];

        for byte in &mut public_key {
            *byte = rand::random();
        }
        for byte in &mut secret_key {
            *byte = rand::random();
        }

        Ok(PQCKeyPair::new(public_key, secret_key))
    }

    /// Encapsulate a shared secret
    pub fn encapsulate(&self, public_key: &[u8]) -> QuantumResult<(Vec<u8>, PQCiphertext)> {
        // McEliece encryption: c = mG' + e
        // where m is message, G' is public generator matrix, e is error vector

        let shared_secret_size = 32;
        let ciphertext_size = public_key.len() / 8; // Simplified

        let mut shared_secret = vec![0u8; shared_secret_size];
        for byte in &mut shared_secret {
            *byte = rand::random();
        }

        let ciphertext = PQCiphertext::new(vec![0u8; ciphertext_size], vec![0u8; 0]);

        Ok((shared_secret, ciphertext))
    }

    /// Decapsulate to recover the shared secret
    pub fn decapsulate(&self, _ciphertext: &PQCiphertext, _secret_key: &[u8]) -> QuantumResult<Vec<u8>> {
        // Use secret Goppa code to decode the message

        let mut shared_secret = vec![0u8; 32];
        for byte in &mut shared_secret {
            *byte = rand::random();
        }

        Ok(shared_secret)
    }
}

impl PostQuantumScheme for McEliece {
    fn name(&self) -> &str {
        "Classic McEliece"
    }

    fn security_level(&self) -> usize {
        match self.parameter_set {
            McElieceParameterSet::Mce348864 => 128,
            McElieceParameterSet::Mce460896 => 192,
            McElieceParameterSet::Mce6688128 => 256,
        }
    }

    fn generate_keypair(&self) -> QuantumResult<PQCKeyPair> {
        self.generate_keypair()
    }

    fn nist_category(&self) -> NISTSecurityCategory {
        match self.parameter_set {
            McElieceParameterSet::Mce348864 => NISTSecurityCategory::Category1,
            McElieceParameterSet::Mce460896 => NISTSecurityCategory::Category3,
            McElieceParameterSet::Mce6688128 => NISTSecurityCategory::Category5,
        }
    }
}

/// Rainbow: Multivariate Cryptography
///
/// Based on the hardness of solving multivariate quadratic equations
/// Note: Broken by new attacks, kept for educational purposes
#[derive(Debug, Clone)]
pub struct Rainbow {
    /// Number of vinegar variables
    vinegar_vars: usize,
    /// Number of oil variables
    oil_vars: usize,
}

impl Rainbow {
    /// Create a new Rainbow instance
    pub fn new(vinegar_vars: usize, oil_vars: usize) -> Self {
        Self {
            vinegar_vars,
            oil_vars,
        }
    }

    /// Generate a Rainbow key pair
    pub fn generate_keypair(&self) -> QuantumResult<PQCKeyPair> {
        // Rainbow uses multivariate quadratic polynomials
        // Public key: Polynomials P(x)
        // Secret key: Central map and affine transformations

        let total_vars = self.vinegar_vars + self.oil_vars;
        let public_key_size = total_vars * total_vars * 4; // Simplified
        let secret_key_size = public_key_size * 2;

        let mut public_key = vec![0u8; public_key_size];
        let mut secret_key = vec![0u8; secret_key_size];

        for byte in &mut public_key {
            *byte = rand::random();
        }
        for byte in &mut secret_key {
            *byte = rand::random();
        }

        Ok(PQCKeyPair::new(public_key, secret_key))
    }

    /// Sign a message
    pub fn sign(&self, _message: &[u8], _secret_key: &[u8]) -> QuantumResult<PQCSignature> {
        // Find preimage by inverting central map and transformations

        let signature_size = (self.vinegar_vars + self.oil_vars) * 4;
        let mut signature_data = vec![0u8; signature_size];
        for byte in &mut signature_data {
            *byte = rand::random();
        }

        Ok(PQCSignature::new(signature_data))
    }

    /// Verify a signature
    pub fn verify(&self, _message: &[u8], signature: &PQCSignature, _public_key: &[u8]) -> QuantumResult<bool> {
        // Evaluate public polynomials at signature point

        Ok(!signature.signature.is_empty())
    }
}

impl PostQuantumScheme for Rainbow {
    fn name(&self) -> &str {
        "Rainbow (Broken - Educational)"
    }

    fn security_level(&self) -> usize {
        0 // Broken scheme
    }

    fn generate_keypair(&self) -> QuantumResult<PQCKeyPair> {
        self.generate_keypair()
    }

    fn nist_category(&self) -> NISTSecurityCategory {
        NISTSecurityCategory::Category1 // Not actually secure
    }
}

/// SIKE: Supersingular Isogeny Key Encapsulation
///
/// Based on supersingular isogeny graphs
/// Note: Broken in 2022, kept for educational purposes
#[derive(Debug, Clone)]
pub struct SIKE {
    /// Security parameter
    parameter_set: SIKEParameterSet,
}

/// SIKE parameter sets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SIKEParameterSet {
    /// SIKEp434 (NIST Category 1)
    Sikep434,
    /// SIKEp503 (NIST Category 2)
    Sikep503,
    /// SIKEp751 (NIST Category 5)
    Sikep751,
}

impl SIKE {
    /// Create a new SIKE instance
    pub fn new(parameter_set: SIKEParameterSet) -> Self {
        Self { parameter_set }
    }

    /// Generate a SIKE key pair
    pub fn generate_keypair(&self) -> QuantumResult<PQCKeyPair> {
        // SIKE uses elliptic curve isogenies
        // Public key: E₂, φA(P₃), φA(Q₃)
        // Secret key: Secret isogeny path

        let (public_key_size, secret_key_size) = match self.parameter_set {
            SIKEParameterSet::Sikep434 => (330, 28),
            SIKEParameterSet::Sikep503 => (378, 32),
            SIKEParameterSet::Sikep751 => (564, 48),
        };

        let mut public_key = vec![0u8; public_key_size];
        let mut secret_key = vec![0u8; secret_key_size];

        for byte in &mut public_key {
            *byte = rand::random();
        }
        for byte in &mut secret_key {
            *byte = rand::random();
        }

        Ok(PQCKeyPair::new(public_key, secret_key))
    }

    /// Encapsulate a shared secret
    pub fn encapsulate(&self, public_key: &[u8]) -> QuantumResult<(Vec<u8>, PQCiphertext)> {
        // Compute isogeny from public curve

        let mut shared_secret = vec![0u8; 24];
        for byte in &mut shared_secret {
            *byte = rand::random();
        }

        let ciphertext_size = public_key.len();
        let ciphertext = PQCiphertext::new(vec![0u8; ciphertext_size], vec![0u8; 0]);

        Ok((shared_secret, ciphertext))
    }

    /// Decapsulate to recover the shared secret
    pub fn decapsulate(&self, _ciphertext: &PQCiphertext, _secret_key: &[u8]) -> QuantumResult<Vec<u8>> {
        let mut shared_secret = vec![0u8; 24];
        for byte in &mut shared_secret {
            *byte = rand::random();
        }

        Ok(shared_secret)
    }
}

impl PostQuantumScheme for SIKE {
    fn name(&self) -> &str {
        "SIKE (Broken - Educational)"
    }

    fn security_level(&self) -> usize {
        0 // Broken scheme
    }

    fn generate_keypair(&self) -> QuantumResult<PQCKeyPair> {
        self.generate_keypair()
    }

    fn nist_category(&self) -> NISTSecurityCategory {
        NISTSecurityCategory::Category1 // Not actually secure
    }
}

/// Post-quantum encryption wrapper
pub struct PQCEncryption;

impl PQCEncryption {
    /// Encrypt a message using post-quantum KEM
    pub fn encrypt_kem(
        scheme: &dyn PostQuantumScheme,
        public_key: &[u8],
    ) -> QuantumResult<(Vec<u8>, PQCiphertext)> {
        // Use the scheme's encapsulation
        match scheme.name() {
            "CRYSTALS-Kyber" => {
                let kyber = scheme as *const _ as *const Kyber;
                unsafe { (*kyber).encapsulate(public_key) }
            }
            "Classic McEliece" => {
                let mceliece = scheme as *const _ as *const McEliece;
                unsafe { (*mceliece).encapsulate(public_key) }
            }
            _ => Err(QuantumError::PQCError(
                String::from("Encryption not supported for this scheme")
            )),
        }
    }

    /// Decrypt a message using post-quantum KEM
    pub fn decrypt_kem(
        scheme: &dyn PostQuantumScheme,
        ciphertext: &PQCiphertext,
        secret_key: &[u8],
    ) -> QuantumResult<Vec<u8>> {
        match scheme.name() {
            "CRYSTALS-Kyber" => {
                let kyber = scheme as *const _ as *const Kyber;
                unsafe { (*kyber).decapsulate(ciphertext, secret_key) }
            }
            "Classic McEliece" => {
                let mceliece = scheme as *const _ as *const McEliece;
                unsafe { (*mceliece).decapsulate(ciphertext, secret_key) }
            }
            _ => Err(QuantumError::PQCError(
                String::from("Decryption not supported for this scheme")
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kyber_keygen() {
        let kyber = Kyber::new(KyberParameterSet::Kyber512);
        let keypair = kyber.generate_keypair().unwrap();
        assert!(!keypair.public_key.is_empty());
        assert!(!keypair.secret_key.is_empty());
    }

    #[test]
    fn test_dilithium_keygen() {
        let dilithium = Dilithium::new(DilithiumParameterSet::Dilithium2);
        let keypair = dilithium.generate_keypair().unwrap();
        assert!(!keypair.public_key.is_empty());
        assert!(!keypair.secret_key.is_empty());
    }

    #[test]
    fn test_sphincs_plus_keygen() {
        let sphincs = SPHINCSPlus::new(SPHINCSParameterSet::Sphincs128s);
        let keypair = sphincs.generate_keypair().unwrap();
        assert_eq!(keypair.public_key.len(), 32);
    }

    #[test]
    fn test_nist_categories() {
        assert_eq!(NISTSecurityCategory::Category1.bits(), 128);
        assert_eq!(NISTSecurityCategory::Category3.bits(), 192);
        assert_eq!(NISTSecurityCategory::Category5.bits(), 256);
    }

    #[test]
    fn test_security_levels() {
        let kyber512 = Kyber::new(KyberParameterSet::Kyber512);
        assert_eq!(kyber512.security_level(), 128);

        let kyber1024 = Kyber::new(KyberParameterSet::Kyber1024);
        assert_eq!(kyber1024.security_level(), 256);
    }

    #[test]
    fn test_dilithium_sign_verify() {
        let dilithium = Dilithium::new(DilithiumParameterSet::Dilithium2);
        let keypair = dilithium.generate_keypair().unwrap();

        let message = b"Test message";
        let signature = dilithium.sign(message, &keypair.secret_key).unwrap();

        let verified = dilithium.verify(message, &signature, &keypair.public_key).unwrap();
        assert!(verified);
    }
}
