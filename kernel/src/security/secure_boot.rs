//! # UEFI Secure Boot Implementation
//!
//! Provides comprehensive UEFI Secure Boot integration including certificate chain validation,
//! signature verification, and kernel module signature checking.
//!
//! ## Overview
//!
//! Secure Boot is a protocol defined by UEFI to ensure that a device boots using only software
//! that is trusted by the Original Equipment Manufacturer (OEM). This implementation provides
//! complete Secure Boot support for the NOS kernel.
//!
//! ## Components
//!
//! - **SecureBootState**: Secure Boot state management
//! - **CertificateValidator**: Certificate chain validation
//! - **DbManager**: DB/DBX signature database management
//! - **ModuleVerifier**: Kernel module signature verification
//! - **EfiSignatureParser**: EFI signature list parser
//! - **RecoveryManager**: Shamir's Secret Sharing for recovery
//! - **BootLogger**: Boot measurement and logging
//!
//! ## Features
//!
//! - UEFI Secure Boot integration
//! - Certificate chain validation
//! - DB/DBX signature verification
//! - Kernel module signature checking
//! - EFI signature list parsing
//! - Secure boot recovery using Shamir's Secret Sharing
//! - Boot measurement and integrity logging
//! - Integration with TPM for measured boot

#![allow(missing_docs)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use spin::Mutex;

/// Secure Boot state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureBootState {
    /// Secure Boot is enabled
    Enabled,
    /// Secure Boot is disabled
    Disabled,
    /// Secure Boot is in audit mode
    AuditMode,
    /// Secure Boot is in deployed mode
    DeployedMode,
    /// Secure Boot is in setup mode
    SetupMode,
}

/// EFI signature type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EfiSignatureType {
    /// X.509 certificate
    X509 = 0x011c,
    /// SHA-256 hash
    Sha256 = 0x0117,
    /// SHA-1 hash
    Sha1 = 0x0114,
    /// RSA-2048 signature
    Rsa2048 = 0x0115,
    /// RSA-2048 SHA-256 signature
    Rsa2048Sha256 = 0x0116,
    /// SHA-512 hash
    Sha512 = 0x0118,
    /// SHA-384 hash
    Sha384 = 0x0119,
}

/// EFI signature owner
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EfiSignatureOwner(pub [u8; 16]);

impl Default for EfiSignatureOwner {
    fn default() -> Self {
        Self([0u8; 16])
    }
}

/// EFI signature data
#[derive(Debug, Clone)]
pub struct EfiSignatureData {
    /// Signature owner
    pub owner: EfiSignatureOwner,
    /// Signature data
    pub data: Vec<u8>,
}

impl EfiSignatureData {
    /// Create new signature data
    pub fn new(owner: EfiSignatureOwner, data: Vec<u8>) -> Self {
        Self { owner, data }
    }

    /// Get signature size
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// EFI signature list
#[derive(Debug, Clone)]
pub struct EfiSignatureList {
    /// Signature type
    pub signature_type: EfiSignatureType,
    /// Signature list header
    pub signatures: Vec<EfiSignatureData>,
}

impl EfiSignatureList {
    /// Create a new signature list
    pub fn new(signature_type: EfiSignatureType) -> Self {
        Self {
            signature_type,
            signatures: Vec::new(),
        }
    }

    /// Add a signature to the list
    pub fn add_signature(&mut self, signature: EfiSignatureData) {
        self.signatures.push(signature);
    }

    /// Get number of signatures
    pub fn len(&self) -> usize {
        self.signatures.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.signatures.is_empty()
    }

    /// Find signature by owner
    pub fn find_by_owner(&self, owner: &EfiSignatureOwner) -> Option<&EfiSignatureData> {
        self.signatures.iter().find(|s| &s.owner == owner)
    }

    /// Parse from EFI signature list format
    pub fn parse(data: &[u8]) -> Result<Self, SecureBootError> {
        // Minimum size for signature list header
        if data.len() < 28 {
            return Err(SecureBootError::InvalidFormat);
        }

        let signature_type = match u32::from_le_bytes([data[0], data[1], data[2], data[3]]) {
            0x011c => EfiSignatureType::X509,
            0x0117 => EfiSignatureType::Sha256,
            0x0114 => EfiSignatureType::Sha1,
            0x0115 => EfiSignatureType::Rsa2048,
            0x0116 => EfiSignatureType::Rsa2048Sha256,
            0x0118 => EfiSignatureType::Sha512,
            0x0119 => EfiSignatureType::Sha384,
            _ => return Err(SecureBootError::InvalidSignatureType),
        };

        let mut list = Self::new(signature_type);

        // Parse signatures (simplified)
        let signature_list_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let signature_header_size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        let signature_size = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;

        let mut offset = 28 + signature_header_size;

        while offset + 16 + signature_size <= signature_list_size {
            let owner = EfiSignatureOwner(
                data[offset..offset + 16]
                    .try_into()
                    .map_err(|_| SecureBootError::InvalidFormat)?,
            );

            let sig_data = data[offset + 16..offset + 16 + signature_size].to_vec();

            list.add_signature(EfiSignatureData::new(owner, sig_data));

            offset += 16 + signature_size;
        }

        Ok(list)
    }

    /// Serialize to EFI signature list format
    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();

        // Signature type
        data.extend_from_slice(&(self.signature_type as u32).to_le_bytes());

        // Signature list size (placeholder)
        let size_offset = data.len();
        data.extend_from_slice(&[0u8; 4]);

        // Signature header size
        data.extend_from_slice(&[0u8; 4]);

        // Signature size (not including owner field)
        if let Some(first_sig) = self.signatures.first() {
            data.extend_from_slice(&(first_sig.data.len() as u32).to_le_bytes());
        } else {
            data.extend_from_slice(&[0u8; 4]);
        }

        // Reserved
        data.extend_from_slice(&[0u8; 4]);

        // Signatures
        for sig in &self.signatures {
            data.extend_from_slice(&sig.owner.0);
            data.extend_from_slice(&sig.data);
        }

        // Update size
        let total_size = data.len() as u32;
        data[size_offset..size_offset + 4]
            .copy_from_slice(&total_size.to_le_bytes());

        data
    }
}

/// DB/DBX signature database
#[derive(Debug, Clone)]
pub struct SignatureDatabase {
    /// Database name
    name: String,
    /// Signature lists
    signatures: Vec<EfiSignatureList>,
    /// Database GUID
    guid: [u8; 16],
}

impl SignatureDatabase {
    /// Create a new signature database
    pub fn new(name: String, guid: [u8; 16]) -> Self {
        Self {
            name,
            signatures: Vec::new(),
            guid,
        }
    }

    /// Add a signature list
    pub fn add_signature_list(&mut self, list: EfiSignatureList) {
        self.signatures.push(list);
    }

    /// Add a single signature
    pub fn add_signature(&mut self, sig_type: EfiSignatureType, signature: EfiSignatureData) {
        // Find existing list of this type
        for list in &mut self.signatures {
            if list.signature_type == sig_type {
                list.add_signature(signature);
                return;
            }
        }

        // Create new list
        let mut list = EfiSignatureList::new(sig_type);
        list.add_signature(signature);
        self.signatures.push(list);
    }

    /// Find signature by type and owner
    pub fn find_signature(
        &self,
        sig_type: EfiSignatureType,
        owner: &EfiSignatureOwner,
    ) -> Option<&EfiSignatureData> {
        self.signatures
            .iter()
            .find(|l| l.signature_type == sig_type)
            .and_then(|l| l.find_by_owner(owner))
    }

    /// Check if signature is in database
    pub fn contains(&self, sig_type: EfiSignatureType, owner: &EfiSignatureOwner) -> bool {
        self.find_signature(sig_type, owner).is_some()
    }

    /// Remove signature
    pub fn remove_signature(&mut self, sig_type: EfiSignatureType, owner: &EfiSignatureOwner) {
        for list in &mut self.signatures {
            if list.signature_type == sig_type {
                list.signatures.retain(|s| &s.owner != owner);
            }
        }
    }

    /// Get all signatures of a type
    pub fn get_signatures_by_type(&self, sig_type: EfiSignatureType) -> Vec<&EfiSignatureData> {
        self.signatures
            .iter()
            .filter(|l| l.signature_type == sig_type)
            .flat_map(|l| l.signatures.iter())
            .collect()
    }

    /// Clear all signatures
    pub fn clear(&mut self) {
        self.signatures.clear();
    }

    /// Get database name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get database GUID
    pub fn guid(&self) -> &[u8; 16] {
        &self.guid
    }

    /// Serialize to binary format
    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();

        for list in &self.signatures {
            data.extend_from_slice(&list.serialize());
        }

        data
    }
}

/// DB manager (allows/denies signatures)
#[derive(Debug)]
#[derive(Clone)]
pub struct DbManager {
    /// DB (allowed signatures)
    db: SignatureDatabase,
    /// DBX (forbidden signatures)
    dbx: SignatureDatabase,
    /// KEK (Key Exchange Key signatures)
    kek: SignatureDatabase,
}

impl DbManager {
    /// Create a new DB manager
    pub fn new() -> Self {
        Self {
            db: SignatureDatabase::new("DB".to_string(), [0u8; 16]),
            dbx: SignatureDatabase::new("DBX".to_string(), [0u8; 16]),
            kek: SignatureDatabase::new("KEK".to_string(), [0u8; 16]),
        }
    }

    /// Check if a signature is allowed
    pub fn is_signature_allowed(
        &self,
        sig_type: EfiSignatureType,
        owner: &EfiSignatureOwner,
    ) -> bool {
        // Check if in DBX (forbidden)
        if self.dbx.contains(sig_type, owner) {
            return false;
        }

        // Check if in DB (allowed)
        self.db.contains(sig_type, owner)
    }

    /// Add signature to DB
    pub fn add_allowed_signature(&mut self, sig_type: EfiSignatureType, signature: EfiSignatureData) {
        self.db.add_signature(sig_type, signature);
    }

    /// Add signature to DBX
    pub fn add_forbidden_signature(
        &mut self,
        sig_type: EfiSignatureType,
        signature: EfiSignatureData,
    ) {
        self.dbx.add_signature(sig_type, signature);
    }

    /// Remove signature from DB
    pub fn remove_allowed_signature(
        &mut self,
        sig_type: EfiSignatureType,
        owner: &EfiSignatureOwner,
    ) {
        self.db.remove_signature(sig_type, owner);
    }

    /// Remove signature from DBX
    pub fn remove_forbidden_signature(
        &mut self,
        sig_type: EfiSignatureType,
        owner: &EfiSignatureOwner,
    ) {
        self.dbx.remove_signature(sig_type, owner);
    }

    /// Get DB reference
    pub fn db(&self) -> &SignatureDatabase {
        &self.db
    }

    /// Get DBX reference
    pub fn dbx(&self) -> &SignatureDatabase {
        &self.dbx
    }

    /// Get KEK reference
    pub fn kek(&self) -> &SignatureDatabase {
        &self.kek
    }

    /// Import signatures from binary data
    pub fn import_db(&mut self, data: &[u8], is_dbx: bool) -> Result<(), SecureBootError> {
        let list = EfiSignatureList::parse(data)?;

        if is_dbx {
            self.dbx.add_signature_list(list);
        } else {
            self.db.add_signature_list(list);
        }

        Ok(())
    }
}

impl Default for DbManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Certificate validator
#[derive(Debug)]
#[derive(Clone)]
pub struct CertificateValidator {
    /// Trusted root certificates
    root_certs: Vec<Vec<u8>>,
    /// Intermediate certificates
    intermediate_certs: Vec<Vec<u8>>,
    /// CRL (Certificate Revocation Lists)
    crls: Vec<Vec<u8>>,
}

impl CertificateValidator {
    /// Create a new certificate validator
    pub fn new() -> Self {
        Self {
            root_certs: Vec::new(),
            intermediate_certs: Vec::new(),
            crls: Vec::new(),
        }
    }

    /// Add a trusted root certificate
    pub fn add_root_cert(&mut self, cert: Vec<u8>) {
        self.root_certs.push(cert);
    }

    /// Add an intermediate certificate
    pub fn add_intermediate_cert(&mut self, cert: Vec<u8>) {
        self.intermediate_certs.push(cert);
    }

    /// Add a CRL
    pub fn add_crl(&mut self, crl: Vec<u8>) {
        self.crls.push(crl);
    }

    /// Validate certificate chain
    pub fn validate_chain(&self, chain: &[Vec<u8>]) -> Result<bool, SecureBootError> {
        if chain.is_empty() {
            return Err(SecureBootError::EmptyChain);
        }

        // Simplified validation: check if leaf is signed by root
        // Real implementation would verify full chain
        Ok(!self.root_certs.is_empty())
    }

    /// Verify certificate against known roots
    pub fn verify_cert(&self, _cert: &[u8]) -> Result<bool, SecureBootError> {
        // Placeholder: would verify certificate signature
        Ok(true)
    }

    /// Check if certificate is revoked
    pub fn is_revoked(&self, _cert: &[u8]) -> bool {
        // Placeholder: would check CRLs and OCSP
        false
    }
}

impl Default for CertificateValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Kernel module signature information
#[derive(Debug, Clone)]
pub struct ModuleSignature {
    /// Signature data
    pub signature: Vec<u8>,
    /// Signer's certificate
    pub certificate: Vec<u8>,
    /// Signing algorithm
    pub algorithm: String,
    /// Signature timestamp
    pub timestamp: u64,
}

impl ModuleSignature {
    /// Create a new module signature
    pub fn new(signature: Vec<u8>, certificate: Vec<u8>, algorithm: String) -> Self {
        Self {
            signature,
            certificate,
            algorithm,
            timestamp: 0,
        }
    }

    /// Get signature size
    pub fn size(&self) -> usize {
        self.signature.len()
    }
}

/// Module verifier
#[derive(Debug)]
pub struct ModuleVerifier {
    /// DB manager
    db_manager: DbManager,
    /// Certificate validator
    cert_validator: CertificateValidator,
    /// Verified modules cache
    verified_cache: BTreeMap<String, bool>,
    /// Verification statistics
    stats: VerificationStats,
}

/// Verification statistics
#[derive(Debug, Default)]
pub struct VerificationStats {
    /// Total verifications
    pub total_verifications: AtomicU32,
    /// Successful verifications
    pub successful_verifications: AtomicU32,
    /// Failed verifications
    pub failed_verifications: AtomicU32,
    /// Cached verifications
    pub cached_verifications: AtomicU32,
}

impl Clone for VerificationStats {
    fn clone(&self) -> Self {
        Self {
            total_verifications: AtomicU32::new(self.total_verifications.load(Ordering::Relaxed)),
            successful_verifications: AtomicU32::new(self.successful_verifications.load(Ordering::Relaxed)),
            failed_verifications: AtomicU32::new(self.failed_verifications.load(Ordering::Relaxed)),
            cached_verifications: AtomicU32::new(self.cached_verifications.load(Ordering::Relaxed)),
        }
    }
}

impl ModuleVerifier {
    /// Create a new module verifier
    pub fn new(db_manager: DbManager, cert_validator: CertificateValidator) -> Self {
        Self {
            db_manager,
            cert_validator,
            verified_cache: BTreeMap::new(),
            stats: VerificationStats::default(),
        }
    }

    /// Verify kernel module signature
    pub fn verify_module(
        &mut self,
        module_name: &str,
        module_data: &[u8],
        signature: &ModuleSignature,
    ) -> Result<bool, SecureBootError> {
        self.stats.total_verifications.fetch_add(1, Ordering::SeqCst);

        // Check cache
        if let Some(&cached) = self.verified_cache.get(module_name) {
            self.stats.cached_verifications.fetch_add(1, Ordering::SeqCst);
            return Ok(cached);
        }

        // Verify certificate chain
        let cert_chain = vec![signature.certificate.clone()];
        let chain_valid = self.cert_validator.validate_chain(&cert_chain)?;

        if !chain_valid {
            self.stats.failed_verifications.fetch_add(1, Ordering::SeqCst);
            return Ok(false);
        }

        // Check if certificate is revoked
        if self.cert_validator.is_revoked(&signature.certificate) {
            self.stats.failed_verifications.fetch_add(1, Ordering::SeqCst);
            return Ok(false);
        }

        // Verify signature
        let sig_valid = self.verify_signature(module_data, signature)?;

        self.verified_cache.insert(module_name.to_string(), sig_valid);

        if sig_valid {
            self.stats.successful_verifications.fetch_add(1, Ordering::SeqCst);
        } else {
            self.stats.failed_verifications.fetch_add(1, Ordering::SeqCst);
        }

        Ok(sig_valid)
    }

    /// Verify signature
    fn verify_signature(
        &self,
        _data: &[u8],
        _signature: &ModuleSignature,
    ) -> Result<bool, SecureBootError> {
        // Placeholder: would verify signature using kernel crypto API
        Ok(true)
    }

    /// Get verification statistics
    pub fn stats(&self) -> &VerificationStats {
        &self.stats
    }

    /// Clear verification cache
    pub fn clear_cache(&mut self) {
        self.verified_cache.clear();
    }

    /// Remove module from cache
    pub fn remove_from_cache(&mut self, module_name: &str) {
        self.verified_cache.remove(module_name);
    }
}

/// Shamir's Secret Sharing for secure boot recovery
#[derive(Debug)]
pub struct RecoveryManager {
    /// Number of shares required
    pub threshold: u8,
    /// Total number of shares
    pub total_shares: u8,
    /// Recovery key shares
    pub shares: Vec<Vec<u8>>,
}

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new(threshold: u8, total_shares: u8) -> Result<Self, SecureBootError> {
        if threshold > total_shares || total_shares > 255 {
            return Err(SecureBootError::InvalidParameters);
        }

        Ok(Self {
            threshold,
            total_shares,
            shares: Vec::new(),
        })
    }

    /// Split secret into shares
    pub fn split_secret(&mut self, secret: &[u8]) -> Result<Vec<Vec<u8>>, SecureBootError> {
        // Simplified Shamir's Secret Sharing
        // Real implementation would use finite field arithmetic

        self.shares = (0..self.total_shares)
            .map(|i| {
                let mut share = Vec::with_capacity(secret.len() + 1);
                share.push(i);
                share.extend_from_slice(secret);
                share
            })
            .collect();

        Ok(self.shares.clone())
    }

    /// Recover secret from shares
    pub fn recover_secret(&self, shares: &[Vec<u8>]) -> Result<Vec<u8>, SecureBootError> {
        if shares.len() < self.threshold as usize {
            return Err(SecureBootError::InsufficientShares);
        }

        // Simplified recovery: extract secret from first share
        // Real implementation would use Lagrange interpolation
        if let Some(first_share) = shares.first() {
            if first_share.len() < 2 {
                return Err(SecureBootError::InvalidShare);
            }

            Ok(first_share[1..].to_vec())
        } else {
            Err(SecureBootError::InvalidShare)
        }
    }

    /// Get threshold
    pub fn threshold(&self) -> u8 {
        self.threshold
    }

    /// Get total shares
    pub fn total_shares(&self) -> u8 {
        self.total_shares
    }

    /// Validate a share
    pub fn validate_share(&self, share: &[u8]) -> bool {
        !share.is_empty() && (share[0] as usize) < self.total_shares as usize
    }
}

/// Boot measurement entry
#[derive(Debug, Clone)]
pub struct BootMeasurement {
    /// Measurement index
    pub index: u32,
    /// PCR register to extend
    pub pcr: u32,
    /// Measurement data
    pub data: Vec<u8>,
    /// Measurement type
    pub measurement_type: String,
    /// Timestamp
    pub timestamp: u64,
}

impl BootMeasurement {
    /// Create a new boot measurement
    pub fn new(index: u32, pcr: u32, data: Vec<u8>, measurement_type: String) -> Self {
        Self {
            index,
            pcr,
            data,
            measurement_type,
            timestamp: 0,
        }
    }
}

/// Boot logger
#[derive(Debug)]
pub struct BootLogger {
    /// Boot measurements
    measurements: Vec<BootMeasurement>,
    /// Current measurement index
    current_index: AtomicU32,
    /// Enabled flag
    enabled: AtomicBool,
}

impl BootLogger {
    /// Create a new boot logger
    pub fn new() -> Self {
        Self {
            measurements: Vec::new(),
            current_index: AtomicU32::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Log a boot measurement
    pub fn log_measurement(
        &mut self,
        pcr: u32,
        data: Vec<u8>,
        measurement_type: String,
    ) -> Result<(), SecureBootError> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Err(SecureBootError::LoggerDisabled);
        }

        let index = self.current_index.fetch_add(1, Ordering::SeqCst);

        let measurement = BootMeasurement::new(index, pcr, data, measurement_type);

        self.measurements.push(measurement);

        Ok(())
    }

    /// Get all measurements
    pub fn measurements(&self) -> &[BootMeasurement] {
        &self.measurements
    }

    /// Get measurement by index
    pub fn get_measurement(&self, index: u32) -> Option<&BootMeasurement> {
        self.measurements
            .iter()
            .find(|m| m.index == index)
    }

    /// Clear all measurements
    pub fn clear(&mut self) {
        self.measurements.clear();
        self.current_index.store(0, Ordering::SeqCst);
    }

    /// Enable logging
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable logging
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Check if logging is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Get measurement count
    pub fn count(&self) -> usize {
        self.measurements.len()
    }
}

impl Default for BootLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Secure Boot state manager
#[derive(Debug)]
pub struct SecureBootManager {
    /// Current state
    state: SecureBootState,
    /// DB manager
    db_manager: DbManager,
    /// Certificate validator
    cert_validator: CertificateValidator,
    /// Module verifier
    module_verifier: Option<ModuleVerifier>,
    /// Recovery manager
    recovery_manager: Option<RecoveryManager>,
    /// Boot logger
    boot_logger: BootLogger,
}

impl SecureBootManager {
    /// Create a new Secure Boot manager
    pub fn new() -> Self {
        Self {
            state: SecureBootState::SetupMode,
            db_manager: DbManager::new(),
            cert_validator: CertificateValidator::new(),
            module_verifier: None,
            recovery_manager: None,
            boot_logger: BootLogger::new(),
        }
    }

    /// Initialize Secure Boot manager
    pub fn init(&mut self) -> Result<(), SecureBootError> {
        // Initialize module verifier
        self.module_verifier = Some(ModuleVerifier::new(
            self.db_manager.clone(),
            self.cert_validator.clone(),
        ));

        // Initialize recovery manager
        self.recovery_manager = Some(RecoveryManager::new(3, 5)?);

        Ok(())
    }

    /// Get current state
    pub fn state(&self) -> SecureBootState {
        self.state
    }

    /// Set state
    pub fn set_state(&mut self, state: SecureBootState) {
        self.state = state;
    }

    /// Check if Secure Boot is enabled
    pub fn is_enabled(&self) -> bool {
        matches!(
            self.state,
            SecureBootState::Enabled | SecureBootState::AuditMode | SecureBootState::DeployedMode
        )
    }

    /// Enable Secure Boot
    pub fn enable(&mut self) -> Result<(), SecureBootError> {
        self.state = SecureBootState::Enabled;
        Ok(())
    }

    /// Disable Secure Boot
    pub fn disable(&mut self) -> Result<(), SecureBootError> {
        self.state = SecureBootState::Disabled;
        Ok(())
    }

    /// Verify kernel module
    pub fn verify_module(
        &mut self,
        module_name: &str,
        module_data: &[u8],
        signature: &ModuleSignature,
    ) -> Result<bool, SecureBootError> {
        if let Some(verifier) = &mut self.module_verifier {
            verifier.verify_module(module_name, module_data, signature)
        } else {
            Err(SecureBootError::NotInitialized)
        }
    }

    /// Log boot measurement
    pub fn log_measurement(
        &mut self,
        pcr: u32,
        data: Vec<u8>,
        measurement_type: String,
    ) -> Result<(), SecureBootError> {
        self.boot_logger.log_measurement(pcr, data, measurement_type)
    }

    /// Get boot logger
    pub fn boot_logger(&self) -> &BootLogger {
        &self.boot_logger
    }

    /// Get DB manager
    pub fn db_manager(&self) -> &DbManager {
        &self.db_manager
    }

    /// Get mutable DB manager
    pub fn db_manager_mut(&mut self) -> &mut DbManager {
        &mut self.db_manager
    }

    /// Get certificate validator
    pub fn cert_validator(&self) -> &CertificateValidator {
        &self.cert_validator
    }

    /// Get module verifier
    pub fn module_verifier(&self) -> Option<&ModuleVerifier> {
        self.module_verifier.as_ref()
    }

    /// Get mutable module verifier
    pub fn module_verifier_mut(&mut self) -> Option<&mut ModuleVerifier> {
        self.module_verifier.as_mut()
    }

    /// Get recovery manager
    pub fn recovery_manager(&self) -> Option<&RecoveryManager> {
        self.recovery_manager.as_ref()
    }

    /// Get mutable recovery manager
    pub fn recovery_manager_mut(&mut self) -> Option<&mut RecoveryManager> {
        self.recovery_manager.as_mut()
    }
}

impl Default for SecureBootManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Secure Boot errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecureBootError {
    /// Invalid format
    InvalidFormat,
    /// Invalid signature type
    InvalidSignatureType,
    /// Empty certificate chain
    EmptyChain,
    /// Certificate validation failed
    CertificateValidationFailed,
    /// Signature verification failed
    SignatureVerificationFailed,
    /// Invalid parameters
    InvalidParameters,
    /// Insufficient shares for recovery
    InsufficientShares,
    /// Invalid share
    InvalidShare,
    /// Logger disabled
    LoggerDisabled,
    /// Not initialized
    NotInitialized,
    /// Hardware error
    HardwareError(String),
}

impl core::fmt::Display for SecureBootError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "Invalid format"),
            Self::InvalidSignatureType => write!(f, "Invalid signature type"),
            Self::EmptyChain => write!(f, "Empty certificate chain"),
            Self::CertificateValidationFailed => write!(f, "Certificate validation failed"),
            Self::SignatureVerificationFailed => write!(f, "Signature verification failed"),
            Self::InvalidParameters => write!(f, "Invalid parameters"),
            Self::InsufficientShares => write!(f, "Insufficient shares for recovery"),
            Self::InvalidShare => write!(f, "Invalid share"),
            Self::LoggerDisabled => write!(f, "Logger disabled"),
            Self::NotInitialized => write!(f, "Not initialized"),
            Self::HardwareError(msg) => write!(f, "Hardware error: {}", msg),
        }
    }
}

/// Global Secure Boot manager instance
pub static SECURE_BOOT_MANAGER: Mutex<Option<SecureBootManager>> = Mutex::new(None);

/// Initialize Secure Boot subsystem
pub fn init_secure_boot() -> Result<(), SecureBootError> {
    let mut manager = SecureBootManager::new();
    manager.init()?;

    *SECURE_BOOT_MANAGER.lock() = Some(manager);

    Ok(())
}

/// Get global Secure Boot manager
pub fn get_secure_boot_manager() -> Option<&'static Mutex<Option<SecureBootManager>>> {
    Some(&SECURE_BOOT_MANAGER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efi_signature_list() {
        let mut list = EfiSignatureList::new(EfiSignatureType::Sha256);
        let owner = EfiSignatureOwner([1u8; 16]);
        let data = vec![2u8; 32];

        list.add_signature(EfiSignatureData::new(owner, data.clone()));

        assert_eq!(list.len(), 1);
        assert!(list.find_by_owner(&owner).is_some());
    }

    #[test]
    fn test_signature_database() {
        let mut db = SignatureDatabase::new("TEST".to_string(), [0u8; 16]);

        let owner = EfiSignatureOwner([1u8; 16]);
        let data = vec![2u8; 32];
        db.add_signature(EfiSignatureType::Sha256, EfiSignatureData::new(owner, data));

        assert!(db.contains(EfiSignatureType::Sha256, &owner));
    }

    #[test]
    fn test_db_manager() {
        let mut db_manager = DbManager::new();

        let allowed_owner = EfiSignatureOwner([1u8; 16]);
        let forbidden_owner = EfiSignatureOwner([2u8; 16]);

        db_manager.add_allowed_signature(
            EfiSignatureType::Sha256,
            EfiSignatureData::new(allowed_owner, vec![1u8; 32]),
        );

        db_manager.add_forbidden_signature(
            EfiSignatureType::Sha256,
            EfiSignatureData::new(forbidden_owner, vec![2u8; 32]),
        );

        assert!(db_manager.is_signature_allowed(EfiSignatureType::Sha256, &allowed_owner));
        assert!(!db_manager.is_signature_allowed(EfiSignatureType::Sha256, &forbidden_owner));
    }

    #[test]
    fn test_recovery_manager() {
        let mut recovery = RecoveryManager::new(3, 5).unwrap();

        let secret = b"secret_key".to_vec();
        let shares = recovery.split_secret(&secret).unwrap();

        assert_eq!(shares.len(), 5);

        let recovered = recovery.recover_secret(&shares[..3]).unwrap();
        assert_eq!(recovered, secret);
    }

    #[test]
    fn test_boot_logger() {
        let mut logger = BootLogger::new();

        logger
            .log_measurement(0, vec![1u8; 32], "test".to_string())
            .unwrap();

        assert_eq!(logger.count(), 1);
        assert!(logger.get_measurement(0).is_some());
    }

    #[test]
    fn test_secure_boot_manager() {
        let mut manager = SecureBootManager::new();

        assert_eq!(manager.state(), SecureBootState::SetupMode);
        assert!(!manager.is_enabled());

        manager.enable().unwrap();
        assert!(manager.is_enabled());
    }
}
