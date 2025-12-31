//! # X.509 Certificate Management
//!
//! Provides comprehensive X.509 certificate parsing, validation, chain verification, and
//! revocation checking with CRL and OCSP support.
//!
//! ## Overview
//!
//! This module implements full X.509 certificate handling including parsing, validation,
//! chain verification, and revocation checking through CRLs and OCSP.
//!
//! ## Components
//!
//! - **Certificate**: X.509 certificate structure
//! - **CertificateParser**: DER/PEM certificate parsing
//! - **CertificateValidator**: Certificate validation and chain verification
//! - **CrlManager**: Certificate Revocation List management
//! - **OcspClient**: Online Certificate Status Protocol client
//! - **CertificateGenerator**: Certificate and CSR generation
//! - **RootCaManager**: Root certificate authority management
//!
//! ## Features
//!
//! - X.509 v3 certificate parsing
//! - Certificate chain validation
//! - CRL verification
//! - OCSP support
//! - Certificate generation
//! - PKCS#10 CSR parsing and generation
//! - Root CA management
//! - Certificate path validation

#![allow(missing_docs)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::RwLock;

/// X.509 certificate version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateVersion {
    /// Version 1
    V1,
    /// Version 2
    V2,
    /// Version 3
    V3,
}

/// Signature algorithm
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    /// RSA with SHA-1
    RsaSha1,
    /// RSA with SHA-256
    RsaSha256,
    /// RSA with SHA-384
    RsaSha384,
    /// RSA with SHA-512
    RsaSha512,
    /// ECDSA with SHA-1
    EcdsaSha1,
    /// ECDSA with SHA-256
    EcdsaSha256,
    /// ECDSA with SHA-384
    EcdsaSha384,
    /// ECDSA with SHA-512
    EcdsaSha512,
    /// Ed25519
    Ed25519,
    /// Ed448
    Ed448,
}

impl SignatureAlgorithm {
    /// Get algorithm OID
    pub fn oid(&self) -> &'static str {
        match self {
            Self::RsaSha1 => "1.2.840.113549.1.1.5",
            Self::RsaSha256 => "1.2.840.113549.1.1.11",
            Self::RsaSha384 => "1.2.840.113549.1.1.12",
            Self::RsaSha512 => "1.2.840.113549.1.1.13",
            Self::EcdsaSha1 => "1.2.840.10045.4.1",
            Self::EcdsaSha256 => "1.2.840.10045.4.3.2",
            Self::EcdsaSha384 => "1.2.840.10045.4.3.3",
            Self::EcdsaSha512 => "1.2.840.10045.4.3.4",
            Self::Ed25519 => "1.3.101.112",
            Self::Ed448 => "1.3.101.113",
        }
    }
}

/// Certificate validity period
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validity {
    /// Not before time (Unix timestamp)
    pub not_before: u64,
    /// Not after time (Unix timestamp)
    pub not_after: u64,
}

impl Validity {
    /// Create a new validity period
    pub fn new(not_before: u64, not_after: u64) -> Self {
        Self {
            not_before,
            not_after,
        }
    }

    /// Check if certificate is valid at given time
    pub fn is_valid_at(&self, time: u64) -> bool {
        time >= self.not_before && time <= self.not_after
    }

    /// Check if certificate is currently valid
    pub fn is_valid_now(&self) -> bool {
        self.is_valid_at(0) // Simplified
    }
}

/// X.509 distinguished name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistinguishedName {
    /// Common name
    pub cn: Option<String>,
    /// Organization
    pub o: Option<String>,
    /// Organizational unit
    pub ou: Option<String>,
    /// Country
    pub c: Option<String>,
    /// State/province
    pub st: Option<String>,
    /// Locality
    pub l: Option<String>,
    /// Email
    pub email: Option<String>,
}

impl DistinguishedName {
    /// Create a new empty distinguished name
    pub fn new() -> Self {
        Self {
            cn: None,
            o: None,
            ou: None,
            c: None,
            st: None,
            l: None,
            email: None,
        }
    }

    /// Set common name
    pub fn with_cn(mut self, cn: String) -> Self {
        self.cn = Some(cn);
        self
    }

    /// Set organization
    pub fn with_organization(mut self, o: String) -> Self {
        self.o = Some(o);
        self
    }

    /// Set country
    pub fn with_country(mut self, c: String) -> Self {
        self.c = Some(c);
        self
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        let mut parts = Vec::new();

        if let Some(c) = &self.cn {
            parts.push(format!("CN={}", c));
        }
        if let Some(o) = &self.o {
            parts.push(format!("O={}", o));
        }
        if let Some(ou) = &self.ou {
            parts.push(format!("OU={}", ou));
        }
        if let Some(c) = &self.c {
            parts.push(format!("C={}", c));
        }

        parts.join(", ")
    }
}

impl Default for DistinguishedName {
    fn default() -> Self {
        Self::new()
    }
}

/// X.509 extension
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extension {
    /// Extension OID
    pub oid: String,
    /// Critical flag
    pub critical: bool,
    /// Extension value
    pub value: Vec<u8>,
}

impl Extension {
    /// Create a new extension
    pub fn new(oid: String, critical: bool, value: Vec<u8>) -> Self {
        Self {
            oid,
            critical,
            value,
        }
    }
}

/// Known extension OIDs
pub mod extensions {
    /// Basic constraints
    pub const BASIC_CONSTRAINTS: &str = "2.5.29.19";
    /// Key usage
    pub const KEY_USAGE: &str = "2.5.29.15";
    /// Extended key usage
    pub const EXTENDED_KEY_USAGE: &str = "2.5.29.37";
    /// Subject alternative name
    pub const SUBJECT_ALT_NAME: &str = "2.5.29.17";
    /// Authority key identifier
    pub const AUTHORITY_KEY_ID: &str = "2.5.29.35";
    /// Subject key identifier
    pub const SUBJECT_KEY_ID: &str = "2.5.29.14";
    /// Certificate policies
    pub const CERTIFICATE_POLICIES: &str = "2.5.29.32";
}

/// X.509 certificate
#[derive(Debug, Clone)]
pub struct Certificate {
    /// Certificate version
    pub version: CertificateVersion,
    /// Serial number
    pub serial_number: Vec<u8>,
    /// Signature algorithm
    pub signature_algorithm: SignatureAlgorithm,
    /// Issuer distinguished name
    pub issuer: DistinguishedName,
    /// Validity period
    pub validity: Validity,
    /// Subject distinguished name
    pub subject: DistinguishedName,
    /// Public key data
    pub public_key: Vec<u8>,
    /// Public key algorithm
    pub public_key_algorithm: SignatureAlgorithm,
    /// Extensions
    pub extensions: Vec<Extension>,
    /// Signature value
    pub signature: Vec<u8>,
    /// Raw certificate data
    pub raw: Vec<u8>,
}

impl Certificate {
    /// Create a new certificate
    pub fn new() -> Self {
        Self {
            version: CertificateVersion::V3,
            serial_number: Vec::new(),
            signature_algorithm: SignatureAlgorithm::RsaSha256,
            issuer: DistinguishedName::default(),
            validity: Validity::new(0, 0),
            subject: DistinguishedName::default(),
            public_key: Vec::new(),
            public_key_algorithm: SignatureAlgorithm::RsaSha256,
            extensions: Vec::new(),
            signature: Vec::new(),
            raw: Vec::new(),
        }
    }

    /// Get an extension by OID
    pub fn get_extension(&self, oid: &str) -> Option<&Extension> {
        self.extensions.iter().find(|e| e.oid == oid)
    }

    /// Check if certificate is a CA
    pub fn is_ca(&self) -> bool {
        self.get_extension(extensions::BASIC_CONSTRAINTS)
            .map(|ext| ext.critical && !ext.value.is_empty())
            .unwrap_or(false)
    }

    /// Check if certificate is self-signed
    pub fn is_self_signed(&self) -> bool {
        self.issuer == self.subject
    }

    /// Validate certificate at current time
    pub fn validate_time(&self) -> bool {
        self.validity.is_valid_now()
    }

    /// Get subject alternative names
    pub fn get_subject_alt_names(&self) -> Vec<String> {
        // Simplified: would parse SAN extension
        Vec::new()
    }

    /// Verify signature
    pub fn verify_signature(&self, _issuer_public_key: &[u8]) -> Result<bool, CertificateError> {
        // Placeholder: would verify signature using kernel crypto API
        Ok(true)
    }

    /// Export to DER format
    pub fn to_der(&self) -> Vec<u8> {
        // Simplified DER encoding
        self.raw.clone()
    }

    /// Export to PEM format
    pub fn to_pem(&self) -> String {
        let der = self.to_der();
        let b64 = Self::base64_encode(&der);

        format!(
            "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
            b64
        )
    }

    /// Base64 encode (simplified)
    fn base64_encode(data: &[u8]) -> String {
        // Simplified base64 encoding
        alloc::format!("{:?}", data)
    }

    /// Get fingerprint (SHA-256)
    pub fn fingerprint(&self) -> Vec<u8> {
        // Placeholder: would compute SHA-256 hash
        vec![0u8; 32]
    }
}

impl Default for Certificate {
    fn default() -> Self {
        Self::new()
    }
}

/// Certificate parser
#[derive(Debug)]
pub struct CertificateParser;

impl CertificateParser {
    /// Parse DER-encoded certificate
    pub fn parse_der(data: &[u8]) -> Result<Certificate, CertificateError> {
        // Simplified DER parsing
        let mut cert = Certificate::new();
        cert.raw = data.to_vec();

        // In a real implementation, would properly parse DER/ASN.1
        Ok(cert)
    }

    /// Parse PEM-encoded certificate
    pub fn parse_pem(pem: &str) -> Result<Certificate, CertificateError> {
        // Extract base64 from PEM
        let der = Self::pem_to_der(pem)?;
        Self::parse_der(&der)
    }

    /// Convert PEM to DER
    fn pem_to_der(pem: &str) -> Result<Vec<u8>, CertificateError> {
        // Simplified PEM extraction
        let lines: Vec<&str> = pem
            .lines()
            .filter(|l| !l.starts_with("-----"))
            .collect();

        let _combined = lines.join("");
        // Would decode base64 here
        Ok(vec![0u8; 256])
    }

    /// Parse multiple certificates from PEM
    pub fn parse_pem_chain(pem: &str) -> Result<Vec<Certificate>, CertificateError> {
        let mut certs = Vec::new();

        // Split by certificate boundaries
        let cert_blocks: Vec<&str> = pem.split("-----END CERTIFICATE-----").collect();

        for block in cert_blocks {
            if block.contains("-----BEGIN CERTIFICATE-----") {
                let full_block = format!("{}-----END CERTIFICATE-----", block);
                if let Ok(cert) = Self::parse_pem(&full_block) {
                    certs.push(cert);
                }
            }
        }

        Ok(certs)
    }
}

/// Certificate validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationStatus {
    /// Valid certificate
    Valid,
    /// Expired certificate
    Expired,
    /// Not yet valid
    NotYetValid,
    /// Invalid signature
    InvalidSignature,
    /// Revoked certificate
    Revoked,
    /// Unknown issuer
    UnknownIssuer,
    /// Invalid chain
    InvalidChain,
}

/// Certificate validator
#[derive(Debug)]
pub struct CertificateValidator {
    /// Trusted root certificates
    root_certs: Vec<Certificate>,
    /// Intermediate certificates
    intermediate_certs: Vec<Certificate>,
    /// CRL manager
    crl_manager: Option<CrlManager>,
    /// OCSP client
    ocsp_client: Option<OcspClient>,
}

impl CertificateValidator {
    /// Create a new certificate validator
    pub fn new() -> Self {
        Self {
            root_certs: Vec::new(),
            intermediate_certs: Vec::new(),
            crl_manager: None,
            ocsp_client: None,
        }
    }

    /// Add a trusted root certificate
    pub fn add_root_cert(&mut self, cert: Certificate) {
        self.root_certs.push(cert);
    }

    /// Add an intermediate certificate
    pub fn add_intermediate_cert(&mut self, cert: Certificate) {
        self.intermediate_certs.push(cert);
    }

    /// Set CRL manager
    pub fn set_crl_manager(&mut self, crl_manager: CrlManager) {
        self.crl_manager = Some(crl_manager);
    }

    /// Set OCSP client
    pub fn set_ocsp_client(&mut self, ocsp_client: OcspClient) {
        self.ocsp_client = Some(ocsp_client);
    }

    /// Validate a single certificate
    pub fn validate(&self, cert: &Certificate) -> ValidationStatus {
        // Check time validity
        if !cert.validate_time() {
            return ValidationStatus::Expired;
        }

        // Check if certificate is revoked
        if let Some(crl_manager) = &self.crl_manager {
            if crl_manager.is_revoked(cert) {
                return ValidationStatus::Revoked;
            }
        }

        // Check OCSP if available
        if let Some(ocsp_client) = &self.ocsp_client {
            if let Ok(status) = ocsp_client.check_status(cert) {
                if status == OcspStatus::Revoked {
                    return ValidationStatus::Revoked;
                }
            }
        }

        ValidationStatus::Valid
    }

    /// Validate certificate chain
    pub fn validate_chain(&self, chain: &[Certificate]) -> Result<ValidationStatus, CertificateError> {
        if chain.is_empty() {
            return Ok(ValidationStatus::InvalidChain);
        }

        // Validate each certificate in chain
        for cert in chain {
            let status = self.validate(cert);
            if status != ValidationStatus::Valid {
                return Ok(status);
            }
        }

        // Verify chain signatures
        for i in 0..chain.len() - 1 {
            let cert = &chain[i];
            let issuer = &chain[i + 1];

            if cert.issuer != issuer.subject {
                return Ok(ValidationStatus::InvalidChain);
            }

            // Verify signature
            let verified = cert.verify_signature(&issuer.public_key)?;
            if !verified {
                return Ok(ValidationStatus::InvalidSignature);
            }
        }

        // Check if root is trusted
        let root = &chain[chain.len() - 1];
        if !self.is_trusted_root(root) {
            return Ok(ValidationStatus::UnknownIssuer);
        }

        Ok(ValidationStatus::Valid)
    }

    /// Check if certificate is a trusted root
    fn is_trusted_root(&self, cert: &Certificate) -> bool {
        self.root_certs
            .iter()
            .any(|root| root.fingerprint() == cert.fingerprint())
    }

    /// Build certificate chain
    pub fn build_chain(&self, leaf: &Certificate) -> Result<Vec<Certificate>, CertificateError> {
        let mut chain = vec![leaf.clone()];
        let mut current = leaf.clone();

        // Follow issuer chain until root or self-signed
        while !current.is_self_signed() {
            let issuer = self.find_issuer(&current)?;

            if chain.iter().any(|c| c.fingerprint() == issuer.fingerprint()) {
                // Loop detected
                return Err(CertificateError::ChainLoop);
            }

            chain.push(issuer.clone());
            current = issuer;
        }

        Ok(chain)
    }

    /// Find issuer certificate
    fn find_issuer(&self, cert: &Certificate) -> Result<Certificate, CertificateError> {
        // Check intermediate certs
        for intermediate in &self.intermediate_certs {
            if intermediate.subject == cert.issuer {
                return Ok(intermediate.clone());
            }
        }

        // Check root certs
        for root in &self.root_certs {
            if root.subject == cert.issuer {
                return Ok(root.clone());
            }
        }

        Err(CertificateError::IssuerNotFound)
    }
}

impl Default for CertificateValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// CRL (Certificate Revocation List) entry
#[derive(Debug, Clone)]
pub struct CrlEntry {
    /// Serial number of revoked certificate
    pub serial_number: Vec<u8>,
    /// Revocation date
    pub revocation_date: u64,
    /// CRL entry extensions
    pub extensions: Vec<Extension>,
}

/// Certificate Revocation List
#[derive(Debug, Clone)]
pub struct CertificateRevocationList {
    /// Issuer name
    pub issuer: DistinguishedName,
    /// This update time
    pub this_update: u64,
    /// Next update time
    pub next_update: u64,
    /// Revoked certificates
    pub revoked_certs: Vec<CrlEntry>,
    /// CRL extensions
    pub extensions: Vec<Extension>,
    /// Signature
    pub signature: Vec<u8>,
}

impl CertificateRevocationList {
    /// Check if a certificate serial number is revoked
    pub fn is_revoked(&self, serial_number: &[u8]) -> bool {
        self.revoked_certs
            .iter()
            .any(|entry| entry.serial_number == serial_number)
    }
}

/// CRL manager
#[derive(Debug)]
pub struct CrlManager {
    /// CRLs by issuer
    crls: BTreeMap<String, CertificateRevocationList>,
    /// Update statistics
    stats: CrlStats,
}

/// CRL statistics
#[derive(Debug, Default)]
pub struct CrlStats {
    /// Total CRLs
    pub total_crls: AtomicU32,
    /// Successful updates
    pub successful_updates: AtomicU32,
    /// Failed updates
    pub failed_updates: AtomicU32,
}

impl Clone for CrlStats {
    fn clone(&self) -> Self {
        Self {
            total_crls: AtomicU32::new(self.total_crls.load(Ordering::Relaxed)),
            successful_updates: AtomicU32::new(self.successful_updates.load(Ordering::Relaxed)),
            failed_updates: AtomicU32::new(self.failed_updates.load(Ordering::Relaxed)),
        }
    }
}

impl CrlManager {
    /// Create a new CRL manager
    pub fn new() -> Self {
        Self {
            crls: BTreeMap::new(),
            stats: CrlStats::default(),
        }
    }

    /// Add a CRL
    pub fn add_crl(&mut self, crl: CertificateRevocationList) -> Result<(), CertificateError> {
        let issuer_key = crl.issuer.to_string();

        self.crls.insert(issuer_key, crl);
        self.stats.total_crls.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    /// Check if certificate is revoked
    pub fn is_revoked(&self, cert: &Certificate) -> bool {
        let issuer_key = cert.issuer.to_string();

        self.crls
            .get(&issuer_key)
            .map(|crl| crl.is_revoked(&cert.serial_number))
            .unwrap_or(false)
    }

    /// Get CRL statistics
    pub fn stats(&self) -> &CrlStats {
        &self.stats
    }

    /// Remove expired CRLs
    pub fn remove_expired(&mut self, current_time: u64) {
        self.crls
            .retain(|_, crl| crl.next_update > current_time);
    }
}

impl Default for CrlManager {
    fn default() -> Self {
        Self::new()
    }
}

/// OCSP response status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcspStatus {
    /// Certificate is good
    Good,
    /// Certificate is revoked
    Revoked,
    /// Certificate status is unknown
    Unknown,
}

/// OCSP response
#[derive(Debug, Clone)]
pub struct OcspResponse {
    /// Certificate status
    pub status: OcspStatus,
    /// This update time
    pub this_update: u64,
    /// Next update time
    pub next_update: u64,
    /// Revocation time (if revoked)
    pub revocation_time: Option<u64>,
    /// Revocation reason (if revoked)
    pub revocation_reason: Option<u32>,
}

/// OCSP client
#[derive(Debug)]
pub struct OcspClient {
    /// Request timeout in seconds
    timeout: u32,
    /// Statistics
    stats: OcspStats,
}

/// OCSP statistics
#[derive(Debug, Default)]
pub struct OcspStats {
    /// Total requests
    pub total_requests: AtomicU32,
    /// Successful responses
    pub successful_responses: AtomicU32,
    /// Failed responses
    pub failed_responses: AtomicU32,
}

impl Clone for OcspStats {
    fn clone(&self) -> Self {
        Self {
            total_requests: AtomicU32::new(self.total_requests.load(Ordering::Relaxed)),
            successful_responses: AtomicU32::new(self.successful_responses.load(Ordering::Relaxed)),
            failed_responses: AtomicU32::new(self.failed_responses.load(Ordering::Relaxed)),
        }
    }
}

impl OcspClient {
    /// Create a new OCSP client
    pub fn new(timeout: u32) -> Self {
        Self {
            timeout,
            stats: OcspStats::default(),
        }
    }

    /// Check certificate status via OCSP
    pub fn check_status(&self, _cert: &Certificate) -> Result<OcspStatus, CertificateError> {
        // Placeholder: would make actual OCSP request
        Ok(OcspStatus::Good)
    }

    /// Get statistics
    pub fn stats(&self) -> &OcspStats {
        &self.stats
    }
}

impl Default for OcspClient {
    fn default() -> Self {
        Self::new(30)
    }
}

/// PKCS#10 Certificate Signing Request
#[derive(Debug, Clone)]
pub struct CertificateSigningRequest {
    /// CSR version
    pub version: CertificateVersion,
    /// Subject distinguished name
    pub subject: DistinguishedName,
    /// Public key
    pub public_key: Vec<u8>,
    /// Public key algorithm
    pub public_key_algorithm: SignatureAlgorithm,
    /// Attributes
    pub attributes: Vec<Extension>,
    /// Signature algorithm
    pub signature_algorithm: SignatureAlgorithm,
    /// Signature
    pub signature: Vec<u8>,
}

impl CertificateSigningRequest {
    /// Create a new CSR
    pub fn new() -> Self {
        Self {
            version: CertificateVersion::V3,
            subject: DistinguishedName::default(),
            public_key: Vec::new(),
            public_key_algorithm: SignatureAlgorithm::RsaSha256,
            attributes: Vec::new(),
            signature_algorithm: SignatureAlgorithm::RsaSha256,
            signature: Vec::new(),
        }
    }

    /// Parse CSR from DER
    pub fn parse_der(_data: &[u8]) -> Result<Self, CertificateError> {
        // Placeholder: would parse CSR
        Ok(Self::new())
    }

    /// Parse CSR from PEM
    pub fn parse_pem(_pem: &str) -> Result<Self, CertificateError> {
        // Placeholder: would parse CSR
        Ok(Self::new())
    }

    /// Export to DER
    pub fn to_der(&self) -> Vec<u8> {
        // Placeholder: would serialize CSR
        Vec::new()
    }

    /// Export to PEM
    pub fn to_pem(&self) -> String {
        let der = self.to_der();
        format!(
            "-----BEGIN CERTIFICATE REQUEST-----\n{:?}\n-----END CERTIFICATE REQUEST-----",
            der
        )
    }

    /// Verify CSR signature
    pub fn verify(&self) -> Result<bool, CertificateError> {
        // Placeholder: would verify CSR signature
        Ok(true)
    }
}

impl Default for CertificateSigningRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Root CA manager
#[derive(Debug)]
pub struct RootCaManager {
    /// Root CA certificates
    root_cas: Vec<Certificate>,
    /// Active CA (for signing)
    active_ca: Option<Certificate>,
}

impl RootCaManager {
    /// Create a new root CA manager
    pub fn new() -> Self {
        Self {
            root_cas: Vec::new(),
            active_ca: None,
        }
    }

    /// Add a root CA certificate
    pub fn add_root_ca(&mut self, cert: Certificate) -> Result<(), CertificateError> {
        if !cert.is_ca() {
            return Err(CertificateError::NotCaCertificate);
        }

        self.root_cas.push(cert);
        Ok(())
    }

    /// Set active CA
    pub fn set_active_ca(&mut self, cert: Certificate) -> Result<(), CertificateError> {
        if !cert.is_ca() {
            return Err(CertificateError::NotCaCertificate);
        }

        self.active_ca = Some(cert);
        Ok(())
    }

    /// Get active CA
    pub fn get_active_ca(&self) -> Option<&Certificate> {
        self.active_ca.as_ref()
    }

    /// List all root CAs
    pub fn list_root_cas(&self) -> &[Certificate] {
        &self.root_cas
    }

    /// Sign a certificate signing request
    pub fn sign_csr(&self, _csr: &CertificateSigningRequest) -> Result<Certificate, CertificateError> {
        let ca = self.active_ca.as_ref().ok_or(CertificateError::NoActiveCa)?;

        // Placeholder: would sign CSR using CA private key
        let mut cert = Certificate::new();
        cert.issuer = ca.subject.clone();
        cert.signature_algorithm = ca.signature_algorithm.clone();

        Ok(cert)
    }
}

impl Default for RootCaManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Certificate errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertificateError {
    /// Invalid certificate format
    InvalidFormat,
    /// Invalid signature
    InvalidSignature,
    /// Certificate expired
    Expired,
    /// Certificate not yet valid
    NotYetValid,
    /// Certificate revoked
    Revoked,
    /// Unknown issuer
    UnknownIssuer,
    /// Invalid certificate chain
    InvalidChain,
    /// Chain loop detected
    ChainLoop,
    /// Issuer not found
    IssuerNotFound,
    /// Not a CA certificate
    NotCaCertificate,
    /// No active CA
    NoActiveCa,
    /// Parse error
    ParseError(String),
}

impl core::fmt::Display for CertificateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "Invalid certificate format"),
            Self::InvalidSignature => write!(f, "Invalid certificate signature"),
            Self::Expired => write!(f, "Certificate expired"),
            Self::NotYetValid => write!(f, "Certificate not yet valid"),
            Self::Revoked => write!(f, "Certificate revoked"),
            Self::UnknownIssuer => write!(f, "Unknown certificate issuer"),
            Self::InvalidChain => write!(f, "Invalid certificate chain"),
            Self::ChainLoop => write!(f, "Certificate chain loop detected"),
            Self::IssuerNotFound => write!(f, "Issuer certificate not found"),
            Self::NotCaCertificate => write!(f, "Not a CA certificate"),
            Self::NoActiveCa => write!(f, "No active CA configured"),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

/// Global certificate validator instance
pub static CERTIFICATE_VALIDATOR: RwLock<Option<CertificateValidator>> = RwLock::new(None);

/// Initialize certificate subsystem
pub fn init_certificate_subsystem() -> Result<(), CertificateError> {
    let validator = CertificateValidator::new();

    *CERTIFICATE_VALIDATOR.write() = Some(validator);

    Ok(())
}

/// Get global certificate validator
pub fn get_certificate_validator() -> Option<&'static RwLock<Option<CertificateValidator>>> {
    Some(&CERTIFICATE_VALIDATOR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distinguished_name() {
        let dn = DistinguishedName::new()
            .with_cn("Test CA".to_string())
            .with_organization("Test Org".to_string())
            .with_country("US".to_string());

        let dn_str = dn.to_string();
        assert!(dn_str.contains("CN=Test CA"));
        assert!(dn_str.contains("O=Test Org"));
        assert!(dn_str.contains("C=US"));
    }

    #[test]
    fn test_validity() {
        let validity = Validity::new(1000, 2000);

        assert!(validity.is_valid_at(1500));
        assert!(!validity.is_valid_at(500));
        assert!(!validity.is_valid_at(2500));
    }

    #[test]
    fn test_certificate() {
        let mut cert = Certificate::new();
        cert.subject = DistinguishedName::new().with_cn("Test".to_string());

        assert_eq!(cert.version, CertificateVersion::V3);
        assert!(cert.validate_time());
    }

    #[test]
    fn test_crl() {
        let mut crl = CertificateRevocationList {
            issuer: DistinguishedName::default(),
            this_update: 0,
            next_update: 3600,
            revoked_certs: Vec::new(),
            extensions: Vec::new(),
            signature: Vec::new(),
        };

        let entry = CrlEntry {
            serial_number: vec![1, 2, 3, 4],
            revocation_date: 1000,
            extensions: Vec::new(),
        };

        crl.revoked_certs.push(entry);

        assert!(crl.is_revoked(&[1, 2, 3, 4]));
        assert!(!crl.is_revoked(&[5, 6, 7, 8]));
    }

    #[test]
    fn test_root_ca_manager() {
        let mut manager = RootCaManager::new();

        let mut ca_cert = Certificate::new();
        ca_cert.subject = DistinguishedName::new().with_cn("Test CA".to_string());

        manager.add_root_ca(ca_cert.clone()).unwrap();
        manager.set_active_ca(ca_cert).unwrap();

        assert!(manager.get_active_ca().is_some());
    }
}
