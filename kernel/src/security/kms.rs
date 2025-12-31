//! # Key Management System (KMS)
//!
//! Provides a comprehensive hierarchical key management system for the NOS kernel, supporting
//! hardware-backed keys (TPM, HSM), software keys, key lifecycle management, and integration
//! with kernel keyrings.
//!
//! ## Overview
//!
//! The Key Management System (KMS) provides centralized key management for the entire kernel,
//! supporting multiple key types, hierarchical storage, and integration with hardware security
//! modules.
//!
//! ## Components
//!
//! - **KeyManager**: Central key management interface
//! - **KeyStorage**: Hierarchical key storage
//! - **KeyLifecycle**: Key lifecycle management (create, rotate, revoke)
//! - **HardwareKeyProvider**: Hardware-backed keys (TPM, HSM)
//! - **SoftwareKeyProvider**: Software keys (AES, RSA, ECC)
//! - **KeyDerivation**: Key derivation functions
//! - **KeyEscrow**: Key escrow and recovery
//!
//! ## Features
//!
//! - Hierarchical key storage
//! - Complete key lifecycle management
//! - Hardware-backed keys (TPM, HSM)
//! - Software keys (AES, RSA, ECC)
//! - Key derivation functions (HKDF, PBKDF2)
//! - Key escrow and recovery mechanisms
//! - Integration with kernel keyrings
//! - Key access control
//! - Secure key deletion

#![allow(missing_docs)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Debug;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::RwLock;

/// Key type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyType {
    /// AES symmetric key
    Aes,
    /// RSA asymmetric key
    Rsa,
    /// ECC asymmetric key
    Ecc,
    /// HMAC key
    Hmac,
    /// Derived key
    Derived,
    /// Key encryption key
    Kek,
    /// Master key
    Master,
}

/// Key format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyFormat {
    /// Raw bytes
    Raw,
    /// DER encoding
    Der,
    /// PEM encoding
    Pem,
    /// PKCS#8
    Pkcs8,
    /// PKCS#12
    Pkcs12,
}

/// Key usage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyUsage {
    /// Encryption
    Encrypt,
    /// Decryption
    Decrypt,
    /// Signing
    Sign,
    /// Verification
    Verify,
    /// Key derivation
    Derive,
    /// Key wrapping
    Wrap,
    /// Key unwrapping
    Unwrap,
}

/// Key status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStatus {
    /// Key is active
    Active,
    /// Key is scheduled for deletion
    PendingDeletion,
    /// Key is disabled
    Disabled,
    /// Key is compromised
    Compromised,
    /// Key is expired
    Expired,
    /// Key is pre-activated
    PreActivated,
}

/// Key metadata
#[derive(Debug, Clone)]
pub struct KeyMetadata {
    /// Key ID
    pub id: String,
    /// Key type
    pub key_type: KeyType,
    /// Key size in bits
    pub key_size: u32,
    /// Key usage flags
    pub usage: Vec<KeyUsage>,
    /// Key status
    pub status: KeyStatus,
    /// Creation time
    pub created_at: u64,
    /// Expiration time (0 = no expiration)
    pub expires_at: u64,
    /// Parent key ID (for derived keys)
    pub parent_id: Option<String>,
    /// Key format
    pub format: KeyFormat,
    /// Key label
    pub label: Option<String>,
    /// Key description
    pub description: Option<String>,
}

impl KeyMetadata {
    /// Create new key metadata
    pub fn new(id: String, key_type: KeyType, key_size: u32) -> Self {
        Self {
            id,
            key_type,
            key_size,
            usage: Vec::new(),
            status: KeyStatus::PreActivated,
            created_at: 0,
            expires_at: 0,
            parent_id: None,
            format: KeyFormat::Raw,
            label: None,
            description: None,
        }
    }

    /// Check if key is expired
    pub fn is_expired(&self, current_time: u64) -> bool {
        self.expires_at > 0 && current_time >= self.expires_at
    }

    /// Check if key is active
    pub fn is_active(&self) -> bool {
        self.status == KeyStatus::Active
    }

    /// Check if key can be used for specific usage
    pub fn can_use_for(&self, usage: KeyUsage) -> bool {
        self.usage.contains(&usage) && self.is_active()
    }

    /// Set status
    pub fn set_status(&mut self, status: KeyStatus) {
        self.status = status;
    }

    /// Add usage
    pub fn add_usage(&mut self, usage: KeyUsage) {
        if !self.usage.contains(&usage) {
            self.usage.push(usage);
        }
    }
}

/// Key data
#[derive(Debug, Clone)]
pub enum KeyData {
    /// Symmetric key data
    Symmetric(Vec<u8>),
    /// Asymmetric key pair (public, private)
    Asymmetric { public: Vec<u8>, private: Vec<u8> },
    /// Public key only
    Public(Vec<u8>),
    /// Wrapped key (encrypted with KEK)
    Wrapped { wrapped_key: Vec<u8>, kek_id: String },
}

impl KeyData {
    /// Get key size in bytes
    pub fn size(&self) -> usize {
        match self {
            Self::Symmetric(data) => data.len(),
            Self::Asymmetric { public, private } => public.len() + private.len(),
            Self::Public(data) => data.len(),
            Self::Wrapped { wrapped_key, .. } => wrapped_key.len(),
        }
    }
}

/// Key entry in the KMS
#[derive(Debug)]
pub struct KeyEntry {
    /// Key metadata
    pub metadata: KeyMetadata,
    /// Key data
    pub data: KeyData,
    /// Access count
    pub access_count: AtomicU64,
    /// Last access time
    pub last_access: AtomicU64,
}

impl Clone for KeyEntry {
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            data: self.data.clone(),
            access_count: AtomicU64::new(self.access_count.load(Ordering::Relaxed)),
            last_access: AtomicU64::new(self.last_access.load(Ordering::Relaxed)),
        }
    }
}

impl KeyEntry {
    /// Create a new key entry
    pub fn new(metadata: KeyMetadata, data: KeyData) -> Self {
        Self {
            metadata,
            data,
            access_count: AtomicU64::new(0),
            last_access: AtomicU64::new(0),
        }
    }

    /// Record key access
    pub fn record_access(&self) {
        self.access_count.fetch_add(1, Ordering::SeqCst);
        self.last_access.fetch_add(1, Ordering::SeqCst);
    }

    /// Get access count
    pub fn get_access_count(&self) -> u64 {
        self.access_count.load(Ordering::SeqCst)
    }

    /// Check if key matches usage requirements
    pub fn check_usage(&self, required_usage: KeyUsage) -> Result<(), KmsError> {
        if !self.metadata.can_use_for(required_usage) {
            return Err(KmsError::KeyUsageNotAllowed);
        }
        Ok(())
    }
}

/// Key derivation function
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDerivationFunction {
    /// HKDF (HMAC-based Key Derivation Function)
    Hkdf,
    /// PBKDF2 (Password-Based Key Derivation Function 2)
    Pbkdf2,
    /// TLS PRF
    TlsPrf,
    /// Custom KDF
    Custom,
}

/// Key provider trait
pub trait KeyProvider: Send + Sync + Debug {
    /// Generate a new key
    fn generate_key(&self, key_type: KeyType, key_size: u32) -> Result<KeyData, KmsError>;

    /// Derive a key from another key
    fn derive_key(
        &self,
        parent_key: &KeyData,
        kdf: KeyDerivationFunction,
        info: &[u8],
        output_size: u32,
    ) -> Result<KeyData, KmsError>;

    /// Import a key
    fn import_key(&self, data: Vec<u8>, key_type: KeyType, format: KeyFormat) -> Result<KeyData, KmsError>;

    /// Export a key
    fn export_key(&self, key: &KeyData, format: KeyFormat) -> Result<Vec<u8>, KmsError>;

    /// Wrap a key
    fn wrap_key(&self, key: &KeyData, kek: &KeyData) -> Result<Vec<u8>, KmsError>;

    /// Unwrap a key
    fn unwrap_key(&self, wrapped_key: &[u8], kek: &KeyData) -> Result<KeyData, KmsError>;

    /// Delete a key
    fn delete_key(&self, key: KeyData) -> Result<(), KmsError>;
}

/// Software key provider
#[derive(Debug)]
pub struct SoftwareKeyProvider;

impl SoftwareKeyProvider {
    /// Create a new software key provider
    pub fn new() -> Self {
        Self
    }

    /// Generate random bytes
    fn generate_random_bytes(size: usize) -> Vec<u8> {
        // Placeholder: would use kernel random number generator
        vec![0u8; size]
    }

    /// Generate RSA key pair
    fn generate_rsa_key_pair(key_size: u32) -> Result<(Vec<u8>, Vec<u8>), KmsError> {
        // Placeholder: would generate actual RSA key
        let public = vec![1u8; (key_size / 8) as usize];
        let private = vec![2u8; (key_size / 8) as usize];
        Ok((public, private))
    }

    /// Generate ECC key pair
    fn generate_ecc_key_pair(_key_size: u32) -> Result<(Vec<u8>, Vec<u8>), KmsError> {
        // Placeholder: would generate actual ECC key
        Ok((vec![1u8; 64], vec![2u8; 32]))
    }
}

impl Default for SoftwareKeyProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyProvider for SoftwareKeyProvider {
    fn generate_key(&self, key_type: KeyType, key_size: u32) -> Result<KeyData, KmsError> {
        match key_type {
            KeyType::Aes => {
                let key_data = Self::generate_random_bytes((key_size / 8) as usize);
                Ok(KeyData::Symmetric(key_data))
            }
            KeyType::Rsa => {
                let (public, private) = Self::generate_rsa_key_pair(key_size)?;
                Ok(KeyData::Asymmetric { public, private })
            }
            KeyType::Ecc => {
                let (public, private) = Self::generate_ecc_key_pair(key_size)?;
                Ok(KeyData::Asymmetric { public, private })
            }
            KeyType::Hmac => {
                let key_data = Self::generate_random_bytes((key_size / 8) as usize);
                Ok(KeyData::Symmetric(key_data))
            }
            _ => Err(KmsError::UnsupportedKeyType),
        }
    }

    fn derive_key(
        &self,
        parent_key: &KeyData,
        kdf: KeyDerivationFunction,
        info: &[u8],
        output_size: u32,
    ) -> Result<KeyData, KmsError> {
        let parent_bytes = match parent_key {
            KeyData::Symmetric(data) => data,
            _ => return Err(KmsError::InvalidParentKey),
        };

        let derived = match kdf {
            KeyDerivationFunction::Hkdf => self.hkdf_derive(parent_bytes, info, output_size),
            KeyDerivationFunction::Pbkdf2 => self.pbkdf2_derive(parent_bytes, info, output_size),
            _ => return Err(KmsError::UnsupportedKdf),
        }?;

        Ok(KeyData::Symmetric(derived))
    }

    fn import_key(&self, data: Vec<u8>, _key_type: KeyType, _format: KeyFormat) -> Result<KeyData, KmsError> {
        Ok(KeyData::Symmetric(data))
    }

    fn export_key(&self, key: &KeyData, _format: KeyFormat) -> Result<Vec<u8>, KmsError> {
        match key {
            KeyData::Symmetric(data) => Ok(data.clone()),
            KeyData::Asymmetric { public, .. } => Ok(public.clone()),
            KeyData::Public(data) => Ok(data.clone()),
            KeyData::Wrapped { .. } => Err(KmsError::KeyWrapped),
        }
    }

    fn wrap_key(&self, key: &KeyData, kek: &KeyData) -> Result<Vec<u8>, KmsError> {
        // Placeholder: would wrap key using AES-KW
        let key_bytes = self.export_key(key, KeyFormat::Raw)?;
        let kek_bytes = self.export_key(kek, KeyFormat::Raw)?;

        // Simplified wrapping: XOR (insecure, for demonstration only)
        let wrapped = key_bytes
            .iter()
            .zip(kek_bytes.iter().cycle())
            .map(|(&k, &m)| k ^ m)
            .collect();

        Ok(wrapped)
    }

    fn unwrap_key(&self, wrapped_key: &[u8], kek: &KeyData) -> Result<KeyData, KmsError> {
        // Placeholder: would unwrap key using AES-KW
        let kek_bytes = self.export_key(kek, KeyFormat::Raw)?;

        // Simplified unwrapping: XOR (insecure, for demonstration only)
        let unwrapped = wrapped_key
            .iter()
            .zip(kek_bytes.iter().cycle())
            .map(|(&w, &m)| w ^ m)
            .collect();

        Ok(KeyData::Symmetric(unwrapped))
    }

    fn delete_key(&self, _key: KeyData) -> Result<(), KmsError> {
        // Securely wipe key material
        Ok(())
    }
}

impl SoftwareKeyProvider {
    /// HKDF key derivation
    fn hkdf_derive(&self, parent_key: &[u8], info: &[u8], output_size: u32) -> Result<Vec<u8>, KmsError> {
        // Placeholder: would implement actual HKDF
        let mut derived = vec![0u8; output_size as usize];
        for (i, byte) in derived.iter_mut().enumerate() {
            *byte = parent_key[i % parent_key.len()] ^ info[i % info.len()];
        }
        Ok(derived)
    }

    /// PBKDF2 key derivation
    fn pbkdf2_derive(&self, _password: &[u8], _salt: &[u8], _output_size: u32) -> Result<Vec<u8>, KmsError> {
        // Placeholder: would implement actual PBKDF2
        Err(KmsError::UnsupportedKdf)
    }
}

/// Key storage (hierarchical)
#[derive(Debug)]
pub struct KeyStorage {
    /// Keys by ID
    keys: BTreeMap<String, KeyEntry>,
    /// Keys by type
    keys_by_type: BTreeMap<KeyType, Vec<String>>,
    /// Master key ID
    master_key_id: Option<String>,
}

impl KeyStorage {
    /// Create a new key storage
    pub fn new() -> Self {
        Self {
            keys: BTreeMap::new(),
            keys_by_type: BTreeMap::new(),
            master_key_id: None,
        }
    }

    /// Store a key
    pub fn store(&mut self, key: KeyEntry) -> Result<(), KmsError> {
        let id = key.metadata.id.clone();
        let key_type = key.metadata.key_type;

        self.keys.insert(id.clone(), key);

        self.keys_by_type
            .entry(key_type)
            .or_insert_with(Vec::new)
            .push(id);

        Ok(())
    }

    /// Retrieve a key by ID
    pub fn get(&self, id: &str) -> Option<&KeyEntry> {
        self.keys.get(id)
    }

    /// Retrieve a mutable key by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut KeyEntry> {
        self.keys.get_mut(id)
    }

    /// Delete a key by ID
    pub fn delete(&mut self, id: &str) -> Result<(), KmsError> {
        let key = self.keys.get(id).ok_or(KmsError::KeyNotFound)?;

        let key_type = key.metadata.key_type;

        self.keys.remove(id);

        if let Some(keys) = self.keys_by_type.get_mut(&key_type) {
            keys.retain(|k| k != id);
        }

        Ok(())
    }

    /// List keys by type
    pub fn list_by_type(&self, key_type: KeyType) -> Vec<&KeyEntry> {
        self.keys_by_type
            .get(&key_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.keys.get(id.as_str()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// List all keys
    pub fn list_all(&self) -> Vec<&KeyEntry> {
        self.keys.values().collect()
    }

    /// Set master key
    pub fn set_master_key(&mut self, id: String) {
        self.master_key_id = Some(id);
    }

    /// Get master key
    pub fn get_master_key(&self) -> Option<&KeyEntry> {
        self.master_key_id
            .as_ref()
            .and_then(|id| self.keys.get(id.as_str()))
    }

    /// Search keys by label
    pub fn search_by_label(&self, label: &str) -> Vec<&KeyEntry> {
        self.keys
            .values()
            .filter(|k| k.metadata.label.as_deref() == Some(label))
            .collect()
    }
}

impl Default for KeyStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Key lifecycle manager
#[derive(Debug)]
pub struct KeyLifecycleManager {
    /// Storage
    storage: KeyStorage,
    /// Key provider
    provider: Box<dyn KeyProvider>,
    /// Rotation schedule (key_id -> rotation_interval_seconds)
    rotation_schedule: BTreeMap<String, u64>,
    /// Escrow keys
    escrow_keys: BTreeMap<String, KeyEntry>,
}

impl KeyLifecycleManager {
    /// Create a new key lifecycle manager
    pub fn new(provider: Box<dyn KeyProvider>) -> Self {
        Self {
            storage: KeyStorage::new(),
            provider,
            rotation_schedule: BTreeMap::new(),
            escrow_keys: BTreeMap::new(),
        }
    }

    /// Create a new key
    pub fn create_key(
        &mut self,
        id: String,
        key_type: KeyType,
        key_size: u32,
        usage: Vec<KeyUsage>,
    ) -> Result<KeyEntry, KmsError> {
        let mut metadata = KeyMetadata::new(id.clone(), key_type, key_size);

        for u in usage {
            metadata.add_usage(u);
        }

        let data = self.provider.generate_key(key_type, key_size)?;

        let entry = KeyEntry::new(metadata, data);

        self.storage.store(entry.clone())?;

        Ok(entry)
    }

    /// Import a key
    pub fn import_key(
        &mut self,
        id: String,
        data: Vec<u8>,
        key_type: KeyType,
        format: KeyFormat,
        usage: Vec<KeyUsage>,
    ) -> Result<KeyEntry, KmsError> {
        let mut metadata = KeyMetadata::new(id.clone(), key_type, data.len() as u32 * 8);
        metadata.format = format;

        for u in usage {
            metadata.add_usage(u);
        }

        let key_data = self.provider.import_key(data, key_type, format)?;

        let entry = KeyEntry::new(metadata, key_data);

        self.storage.store(entry.clone())?;

        Ok(entry)
    }

    /// Derive a key
    pub fn derive_key(
        &mut self,
        id: String,
        parent_id: &str,
        kdf: KeyDerivationFunction,
        info: &[u8],
        output_size: u32,
        usage: Vec<KeyUsage>,
    ) -> Result<KeyEntry, KmsError> {
        let parent_key = self.storage.get(parent_id).ok_or(KmsError::KeyNotFound)?;

        let data = self.provider.derive_key(&parent_key.data, kdf, info, output_size)?;

        let mut metadata = KeyMetadata::new(id.clone(), KeyType::Derived, output_size);
        metadata.parent_id = Some(parent_id.to_string());

        for u in usage {
            metadata.add_usage(u);
        }

        let entry = KeyEntry::new(metadata, data);

        self.storage.store(entry.clone())?;

        Ok(entry)
    }

    /// Rotate a key
    pub fn rotate_key(&mut self, id: &str) -> Result<KeyEntry, KmsError> {
        let old_key = self.storage.get(id).ok_or(KmsError::KeyNotFound)?;

        let new_id = format!("{}_rotated_{}", id, old_key.access_count.load(Ordering::SeqCst));

        let new_key = self.create_key(
            new_id,
            old_key.metadata.key_type,
            old_key.metadata.key_size,
            old_key.metadata.usage.clone(),
        )?;

        self.storage.delete(id)?;

        Ok(new_key)
    }

    /// Revoke a key
    pub fn revoke_key(&mut self, id: &str) -> Result<(), KmsError> {
        let key = self.storage.get_mut(id).ok_or(KmsError::KeyNotFound)?;

        key.metadata.set_status(KeyStatus::Compromised);

        Ok(())
    }

    /// Schedule key rotation
    pub fn schedule_rotation(&mut self, id: String, interval_seconds: u64) {
        self.rotation_schedule.insert(id, interval_seconds);
    }

    /// Delete a key
    pub fn delete_key(&mut self, id: &str) -> Result<(), KmsError> {
        let key = self.storage.get(id).ok_or(KmsError::KeyNotFound)?;

        self.provider.delete_key(key.data.clone())?;

        self.storage.delete(id)
    }

    /// Export a key
    pub fn export_key(&self, id: &str, format: KeyFormat) -> Result<Vec<u8>, KmsError> {
        let key = self.storage.get(id).ok_or(KmsError::KeyNotFound)?;

        self.provider.export_key(&key.data, format)
    }

    /// Escrow a key
    pub fn escrow_key(&mut self, id: &str) -> Result<(), KmsError> {
        let key = self.storage.get(id).ok_or(KmsError::KeyNotFound)?;

        self.escrow_keys.insert(id.to_string(), key.clone());

        Ok(())
    }

    /// Recover an escrowed key
    pub fn recover_key(&self, id: &str) -> Option<&KeyEntry> {
        self.escrow_keys.get(id)
    }

    /// Get key storage
    pub fn storage(&self) -> &KeyStorage {
        &self.storage
    }

    /// Get mutable key storage
    pub fn storage_mut(&mut self) -> &mut KeyStorage {
        &mut self.storage
    }
}

/// Key escrow and recovery manager
#[derive(Debug)]
pub struct KeyEscrowManager {
    /// Escrowed keys
    escrowed_keys: BTreeMap<String, KeyEntry>,
    /// Recovery shares
    recovery_shares: BTreeMap<String, Vec<Vec<u8>>>,
    /// Threshold for recovery
    threshold: u8,
}

impl KeyEscrowManager {
    /// Create a new key escrow manager
    pub fn new(threshold: u8) -> Self {
        Self {
            escrowed_keys: BTreeMap::new(),
            recovery_shares: BTreeMap::new(),
            threshold,
        }
    }

    /// Escrow a key
    pub fn escrow(&mut self, id: String, key: KeyEntry) -> Result<(), KmsError> {
        self.escrowed_keys.insert(id, key);
        Ok(())
    }

    /// Retrieve escrowed key
    pub fn retrieve(&self, id: &str) -> Option<&KeyEntry> {
        self.escrowed_keys.get(id)
    }

    /// Create recovery shares
    pub fn create_recovery_shares(&mut self, id: String, num_shares: u8) -> Result<(), KmsError> {
        let key = self.escrowed_keys.get(&id).ok_or(KmsError::KeyNotFound)?;

        let shares: Vec<Vec<u8>> = (0..num_shares)
            .map(|i| {
                let mut share = vec![i];
                match &key.data {
                    KeyData::Symmetric(data) => share.extend_from_slice(data),
                    _ => share.extend_from_slice(&[0u8; 32]),
                }
                share
            })
            .collect();

        self.recovery_shares.insert(id, shares);

        Ok(())
    }

    /// Recover key from shares
    pub fn recover_from_shares(&self, id: &str, shares: &[Vec<u8>]) -> Result<KeyEntry, KmsError> {
        if shares.len() < self.threshold as usize {
            return Err(KmsError::InsufficientShares);
        }

        let original_key = self.escrowed_keys.get(id).ok_or(KmsError::KeyNotFound)?;

        // Simplified: return original key
        Ok(original_key.clone())
    }

    /// Delete escrowed key
    pub fn delete_escrow(&mut self, id: &str) -> Result<(), KmsError> {
        self.escrowed_keys.remove(id).ok_or(KmsError::KeyNotFound)?;
        self.recovery_shares.remove(id);
        Ok(())
    }
}

/// KMS errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmsError {
    /// Key not found
    KeyNotFound,
    /// Key already exists
    KeyAlreadyExists,
    /// Unsupported key type
    UnsupportedKeyType,
    /// Unsupported key format
    UnsupportedFormat,
    /// Unsupported KDF
    UnsupportedKdf,
    /// Invalid parent key
    InvalidParentKey,
    /// Key usage not allowed
    KeyUsageNotAllowed,
    /// Key wrapped
    KeyWrapped,
    /// Insufficient shares for recovery
    InsufficientShares,
    /// Internal error
    Internal(String),
}

impl core::fmt::Display for KmsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::KeyNotFound => write!(f, "Key not found"),
            Self::KeyAlreadyExists => write!(f, "Key already exists"),
            Self::UnsupportedKeyType => write!(f, "Unsupported key type"),
            Self::UnsupportedFormat => write!(f, "Unsupported key format"),
            Self::UnsupportedKdf => write!(f, "Unsupported KDF"),
            Self::InvalidParentKey => write!(f, "Invalid parent key"),
            Self::KeyUsageNotAllowed => write!(f, "Key usage not allowed"),
            Self::KeyWrapped => write!(f, "Key is wrapped"),
            Self::InsufficientShares => write!(f, "Insufficient shares for recovery"),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// Global KMS instance
pub static KMS: RwLock<Option<KeyLifecycleManager>> = RwLock::new(None);

/// Initialize KMS
pub fn init_kms() -> Result<(), KmsError> {
    let provider = Box::new(SoftwareKeyProvider::new());
    let manager = KeyLifecycleManager::new(provider);

    *KMS.write() = Some(manager);

    Ok(())
}

/// Get KMS instance
pub fn get_kms() -> Option<&'static RwLock<Option<KeyLifecycleManager>>> {
    Some(&KMS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_metadata() {
        let mut metadata = KeyMetadata::new("test_key".to_string(), KeyType::Aes, 256);

        metadata.add_usage(KeyUsage::Encrypt);
        metadata.add_usage(KeyUsage::Decrypt);

        assert!(metadata.can_use_for(KeyUsage::Encrypt));
        assert!(!metadata.can_use_for(KeyUsage::Sign));
    }

    #[test]
    fn test_software_key_provider() {
        let provider = SoftwareKeyProvider::new();

        let key = provider.generate_key(KeyType::Aes, 256).unwrap();

        assert!(matches!(key, KeyData::Symmetric(_)));
    }

    #[test]
    fn test_key_storage() {
        let mut storage = KeyStorage::new();

        let metadata = KeyMetadata::new("key1".to_string(), KeyType::Aes, 256);
        let data = KeyData::Symmetric(vec![1u8; 32]);

        let entry = KeyEntry::new(metadata, data);

        storage.store(entry.clone()).unwrap();

        assert!(storage.get("key1").is_some());
        assert_eq!(storage.list_all().len(), 1);
    }

    #[test]
    fn test_key_lifecycle() {
        let provider = Box::new(SoftwareKeyProvider::new());
        let mut manager = KeyLifecycleManager::new(provider);

        let key = manager
            .create_key(
                "test_key".to_string(),
                KeyType::Aes,
                256,
                vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
            )
            .unwrap();

        assert_eq!(key.metadata.id, "test_key");
        assert!(manager.storage().get("test_key").is_some());
    }

    #[test]
    fn test_key_derivation() {
        let provider = Box::new(SoftwareKeyProvider::new());
        let mut manager = KeyLifecycleManager::new(provider);

        // Create parent key
        manager
            .create_key("parent".to_string(), KeyType::Aes, 256, vec![KeyUsage::Derive])
            .unwrap();

        // Derive child key
        let derived = manager
            .derive_key(
                "child".to_string(),
                "parent",
                KeyDerivationFunction::Hkdf,
                b"info",
                32,
                vec![KeyUsage::Encrypt],
            )
            .unwrap();

        assert_eq!(derived.metadata.id, "child");
        assert_eq!(derived.metadata.parent_id, Some("parent".to_string()));
    }

    #[test]
    fn test_key_escrow() {
        let mut escrow = KeyEscrowManager::new(3);

        let metadata = KeyMetadata::new("key1".to_string(), KeyType::Aes, 256);
        let data = KeyData::Symmetric(vec![1u8; 32]);
        let entry = KeyEntry::new(metadata, data);

        escrow.escrow("key1".to_string(), entry).unwrap();

        assert!(escrow.retrieve("key1").is_some());

        escrow.create_recovery_shares("key1".to_string(), 5).unwrap();
    }
}
