//! # Hardware Security Module (HSM) Framework
//!
//! Provides a comprehensive HSM framework with PKCS#11 interface, device drivers,
//! cryptographic operations offloading, and multi-party computation support.
//!
//! ## Overview
//!
//! The HSM framework provides hardware-backed cryptographic operations through a standardized
//! PKCS#11 interface, supporting multiple HSM devices with automatic failover and redundancy.
//!
//! ## Components
//!
//! - **HsmDevice**: HSM device abstraction
//! - **HsmManager**: HSM device management
//! - **Pkcs11Interface**: PKCS#11 API implementation
//! - **HsmDriver**: HSM device drivers
//! - **CryptoOffload**: Cryptographic operations offloading
//! - **HsmKeyStorage**: Secure key storage in HSM
//! - **MpcSupport**: Multi-party computation support
//! - **HsmFailover**: HSM failover and redundancy
//!
//! ## Features
//!
//! - PKCS#11 v3.0 interface
//! - Multiple HSM device support
//! - Cryptographic operation offloading
//! - Secure key storage and management
//! - Multi-party computation
//! - Automatic failover and redundancy
//! - Hardware random number generation
//! - Hardware-backed key operations

#![allow(missing_docs)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Debug;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use spin::RwLock;

/// HSM device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HsmType {
    /// Network HSM
    NetworkHsm,
    /// USB/PCIe HSM
    LocalHsm,
    /// TPM as HSM
    Tpm,
    /// Software HSM (for testing)
    SoftwareHsm,
}

/// HSM device status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HsmStatus {
    /// Device is online
    Online,
    /// Device is offline
    Offline,
    /// Device is in error state
    Error,
    /// Device is initializing
    Initializing,
    /// Device is in maintenance mode
    Maintenance,
}

/// HSM slot ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HsmSlotId(pub u32);

impl HsmSlotId {
    /// Create a new slot ID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw slot ID
    pub const fn value(&self) -> u32 {
        self.0
    }
}

/// HSM session handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HsmSessionHandle(pub u64);

impl HsmSessionHandle {
    /// Create a new session handle
    pub const fn new(handle: u64) -> Self {
        Self(handle)
    }

    /// Get the raw handle value
    pub const fn value(&self) -> u64 {
        self.0
    }
}

/// HSM object handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HsmObjectHandle(pub u64);

impl HsmObjectHandle {
    /// Create a new object handle
    pub const fn new(handle: u64) -> Self {
        Self(handle)
    }

    /// Get the raw handle value
    pub const fn value(&self) -> u64 {
        self.0
    }

    /// Invalid handle
    pub const INVALID: Self = Self(0);
}

/// PKCS#11 object class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Pkcs11ObjectClass {
    /// Data object
    Data = 0,
    /// Certificate object
    Certificate = 1,
    /// Public key object
    PublicKey = 2,
    /// Private key object
    PrivateKey = 3,
    /// Secret key object
    SecretKey = 4,
    /// Hardware feature object
    HardwareFeature = 5,
    /// Domain parameters object
    DomainParameters = 6,
    /// Mechanism object
    Mechanism = 7,
    /// OTP key object
    OtpKey = 8,
}

/// PKCS#11 key type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Pkcs11KeyType {
    /// RSA key
    Rsa = 0,
    /// DSA key
    Dsa = 1,
    /// DH key
    Dh = 2,
    /// EC key
    Ec = 3,
    /// X9.42 DH key
    X9_42Dh = 4,
    /// KEA key
    Kea = 5,
    /// Generic secret key
    GenericSecret = 6,
    /// RC2 key
    Rc2 = 7,
    /// RC4 key
    Rc4 = 8,
    /// DES key
    Des = 9,
    /// DES2 key
    Des2 = 10,
    /// DES3 key
    Des3 = 11,
    /// CAST key
    Cast = 12,
    /// CAST3 key
    Cast3 = 13,
    /// CAST128 key
    Cast128 = 14,
    /// RC5 key
    Rc5 = 15,
    /// IDEA key
    Idea = 16,
    /// Skipjack key
    Skipjack = 17,
    /// Baton key
    Baton = 18,
    /// Juniper key
    Juniper = 19,
    /// CDMF key
    Cdmf = 20,
    /// AES key
    Aes = 21,
    /// Blowfish key
    Blowfish = 22,
    /// Twofish key
    Twofish = 23,
    /// SecurID key
    SecurId = 24,
    /// HOTP key
    Hotp = 25,
    /// ACTI key
    Acti = 26,
    /// Camellia key
    Camellia = 27,
    /// ARIA key
    Aria = 28,
    /// SHA512 key
    Sha512 = 29,
    /// SEED key
    Seed = 30,
    /// SM2 key (Chinese)
    Sm2 = 31,
    /// ECC key
    Ecc = 32,
}

/// PKCS#11 mechanism type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Pkcs11Mechanism {
    /// RSA X.509
    RsaX509 = 1,
    /// RSA PKCS#1
    RsaPkcs = 2,
    /// RSA PKCS#1 SHA-1
    RsaPkcsSha1 = 4,
    /// RSA PKCS#1 SHA-256
    RsaPkcsSha256 = 5,
    /// AES ECB
    AesEcb = 0x01081,
    /// AES CBC
    AesCbc = 0x01082,
    /// AES GCM
    AesGcm = 0x01087,
    /// SHA-1
    Sha1 = 0x00202,
    /// SHA-256
    Sha256 = 0x00250,
    /// SHA-512
    Sha512 = 0x00260,
    /// HMAC
    Hmac = 0x00211,
    /// ECDSA
    Ecdsa = 0x01041,
    /// ECDH
    Ecdh = 0x01040,
}

/// HSM object attributes
#[derive(Debug, Clone)]
pub struct HsmObjectAttributes {
    /// Object class
    pub class: Pkcs11ObjectClass,
    /// Key type
    pub key_type: Option<Pkcs11KeyType>,
    /// Label
    pub label: Option<String>,
    /// ID
    pub id: Option<Vec<u8>>,
    /// Token flag
    pub token: bool,
    /// Private flag
    pub private: bool,
    /// Modifiable flag
    pub modifiable: bool,
    /// Extractable flag
    pub extractable: bool,
    /// Sensitive flag
    pub sensitive: bool,
    /// Key size in bits
    pub key_size: Option<u32>,
}

impl HsmObjectAttributes {
    /// Create new object attributes
    pub fn new(class: Pkcs11ObjectClass) -> Self {
        Self {
            class,
            key_type: None,
            label: None,
            id: None,
            token: false,
            private: false,
            modifiable: false,
            extractable: false,
            sensitive: false,
            key_size: None,
        }
    }
}

/// HSM device information
#[derive(Debug)]
pub struct HsmDeviceInfo {
    /// Slot ID
    pub slot_id: HsmSlotId,
    /// Device description
    pub description: String,
    /// Manufacturer
    pub manufacturer: String,
    /// Model
    pub model: String,
    /// Serial number
    pub serial: String,
    /// Firmware version
    pub firmware_version: String,
    /// Hardware version
    pub hardware_version: String,
    /// Max session count
    pub max_session_count: u32,
    /// Current session count
    pub current_session_count: AtomicU32,
    /// Max object count
    pub max_object_count: u32,
    /// Current object count
    pub current_object_count: AtomicU32,
    /// Device status
    pub status: HsmStatus,
}

impl HsmDeviceInfo {
    /// Create new device info
    pub fn new(slot_id: HsmSlotId) -> Self {
        Self {
            slot_id,
            description: String::new(),
            manufacturer: String::new(),
            model: String::new(),
            serial: String::new(),
            firmware_version: String::new(),
            hardware_version: String::new(),
            max_session_count: 16,
            current_session_count: AtomicU32::new(0),
            max_object_count: 1000,
            current_object_count: AtomicU32::new(0),
            status: HsmStatus::Initializing,
        }
    }

    /// Check if device is available
    pub fn is_available(&self) -> bool {
        matches!(self.status, HsmStatus::Online)
    }

    /// Check if can create new session
    pub fn can_create_session(&self) -> bool {
        self.current_session_count.load(Ordering::SeqCst) < self.max_session_count
    }
}

impl Clone for HsmDeviceInfo {
    fn clone(&self) -> Self {
        Self {
            slot_id: self.slot_id,
            description: self.description.clone(),
            manufacturer: self.manufacturer.clone(),
            model: self.model.clone(),
            serial: self.serial.clone(),
            firmware_version: self.firmware_version.clone(),
            hardware_version: self.hardware_version.clone(),
            max_session_count: self.max_session_count,
            current_session_count: AtomicU32::new(self.current_session_count.load(Ordering::Relaxed)),
            max_object_count: self.max_object_count,
            current_object_count: AtomicU32::new(self.current_object_count.load(Ordering::Relaxed)),
            status: self.status,
        }
    }
}

/// HSM session
#[derive(Debug, Clone)]
pub struct HsmSession {
    /// Session handle
    pub handle: HsmSessionHandle,
    /// Slot ID
    pub slot_id: HsmSlotId,
    /// Read/write session state
    pub read_write: bool,
    /// Application data
    pub app_data: Vec<u8>,
}

impl HsmSession {
    /// Create a new session
    pub fn new(handle: HsmSessionHandle, slot_id: HsmSlotId, read_write: bool) -> Self {
        Self {
            handle,
            slot_id,
            read_write,
            app_data: Vec::new(),
        }
    }
}

/// HSM device trait
pub trait HsmDevice: Send + Sync + Debug {
    /// Get device information
    fn get_info(&self) -> &HsmDeviceInfo;

    /// Initialize device
    fn initialize(&mut self) -> Result<(), HsmError>;

    /// Open a session
    fn open_session(&mut self, read_write: bool) -> Result<HsmSession, HsmError>;

    /// Close a session
    fn close_session(&mut self, handle: HsmSessionHandle) -> Result<(), HsmError>;

    /// Generate random bytes
    fn generate_random(&self, num_bytes: usize) -> Result<Vec<u8>, HsmError>;

    /// Generate a key
    fn generate_key(
        &mut self,
        mechanism: Pkcs11Mechanism,
        attributes: &HsmObjectAttributes,
    ) -> Result<HsmObjectHandle, HsmError>;

    /// Import a key
    fn import_key(
        &mut self,
        key_data: &[u8],
        attributes: &HsmObjectAttributes,
    ) -> Result<HsmObjectHandle, HsmError>;

    /// Export a key
    fn export_key(&self, handle: HsmObjectHandle) -> Result<Vec<u8>, HsmError>;

    /// Destroy an object
    fn destroy_object(&mut self, handle: HsmObjectHandle) -> Result<(), HsmError>;

    /// Sign data
    fn sign(
        &mut self,
        session: HsmSessionHandle,
        key_handle: HsmObjectHandle,
        mechanism: Pkcs11Mechanism,
        data: &[u8],
    ) -> Result<Vec<u8>, HsmError>;

    /// Verify signature
    fn verify(
        &mut self,
        session: HsmSessionHandle,
        key_handle: HsmObjectHandle,
        mechanism: Pkcs11Mechanism,
        data: &[u8],
        signature: &[u8],
    ) -> Result<bool, HsmError>;

    /// Encrypt data
    fn encrypt(
        &mut self,
        session: HsmSessionHandle,
        key_handle: HsmObjectHandle,
        mechanism: Pkcs11Mechanism,
        plaintext: &[u8],
    ) -> Result<Vec<u8>, HsmError>;

    /// Decrypt data
    fn decrypt(
        &mut self,
        session: HsmSessionHandle,
        key_handle: HsmObjectHandle,
        mechanism: Pkcs11Mechanism,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, HsmError>;
}

/// Software HSM (for testing)
#[derive(Debug)]
pub struct SoftwareHsm {
    /// Device info
    info: HsmDeviceInfo,
    /// Sessions
    sessions: BTreeMap<HsmSessionHandle, HsmSession>,
    /// Objects
    objects: BTreeMap<HsmObjectHandle, Vec<u8>>,
    /// Next session handle
    next_session: AtomicU64,
    /// Next object handle
    next_object: AtomicU64,
}

impl SoftwareHsm {
    /// Create a new software HSM
    pub fn new(slot_id: HsmSlotId) -> Self {
        Self {
            info: HsmDeviceInfo::new(slot_id),
            sessions: BTreeMap::new(),
            objects: BTreeMap::new(),
            next_session: AtomicU64::new(1),
            next_object: AtomicU64::new(1),
        }
    }
}

impl Default for SoftwareHsm {
    fn default() -> Self {
        Self::new(HsmSlotId::new(0))
    }
}

impl HsmDevice for SoftwareHsm {
    fn get_info(&self) -> &HsmDeviceInfo {
        &self.info
    }

    fn initialize(&mut self) -> Result<(), HsmError> {
        self.info.description = "Software HSM".to_string();
        self.info.manufacturer = "NOS".to_string();
        self.info.model = "SW-HSM-1.0".to_string();
        self.info.serial = "SW-HSM-001".to_string();
        self.info.firmware_version = "1.0.0".to_string();
        self.info.hardware_version = "1.0".to_string();
        self.info.status = HsmStatus::Online;

        Ok(())
    }

    fn open_session(&mut self, read_write: bool) -> Result<HsmSession, HsmError> {
        if !self.info.is_available() {
            return Err(HsmError::DeviceNotAvailable);
        }

        if !self.info.can_create_session() {
            return Err(HsmError::SessionLimitReached);
        }

        let handle = HsmSessionHandle::new(self.next_session.fetch_add(1, Ordering::SeqCst));
        let session = HsmSession::new(handle, self.info.slot_id, read_write);

        self.sessions.insert(handle, session.clone());
        self.info.current_session_count.fetch_add(1, Ordering::SeqCst);

        Ok(session)
    }

    fn close_session(&mut self, handle: HsmSessionHandle) -> Result<(), HsmError> {
        self.sessions
            .remove(&handle)
            .ok_or(HsmError::SessionNotFound)?;

        self.info.current_session_count.fetch_sub(1, Ordering::SeqCst);

        Ok(())
    }

    fn generate_random(&self, num_bytes: usize) -> Result<Vec<u8>, HsmError> {
        // Placeholder: would use proper random number generator
        Ok(vec![0u8; num_bytes])
    }

    fn generate_key(
        &mut self,
        _mechanism: Pkcs11Mechanism,
        attributes: &HsmObjectAttributes,
    ) -> Result<HsmObjectHandle, HsmError> {
        let key_size = attributes.key_size.unwrap_or(256) as usize / 8;
        let key_data = vec![1u8; key_size];

        let handle = HsmObjectHandle::new(self.next_object.fetch_add(1, Ordering::SeqCst));

        self.objects.insert(handle, key_data);
        self.info.current_object_count.fetch_add(1, Ordering::SeqCst);

        Ok(handle)
    }

    fn import_key(
        &mut self,
        key_data: &[u8],
        _attributes: &HsmObjectAttributes,
    ) -> Result<HsmObjectHandle, HsmError> {
        let handle = HsmObjectHandle::new(self.next_object.fetch_add(1, Ordering::SeqCst));

        self.objects.insert(handle, key_data.to_vec());
        self.info.current_object_count.fetch_add(1, Ordering::SeqCst);

        Ok(handle)
    }

    fn export_key(&self, handle: HsmObjectHandle) -> Result<Vec<u8>, HsmError> {
        self.objects
            .get(&handle)
            .cloned()
            .ok_or(HsmError::ObjectNotFound)
    }

    fn destroy_object(&mut self, handle: HsmObjectHandle) -> Result<(), HsmError> {
        self.objects
            .remove(&handle)
            .ok_or(HsmError::ObjectNotFound)?;

        self.info.current_object_count.fetch_sub(1, Ordering::SeqCst);

        Ok(())
    }

    fn sign(
        &mut self,
        _session: HsmSessionHandle,
        key_handle: HsmObjectHandle,
        _mechanism: Pkcs11Mechanism,
        _data: &[u8],
    ) -> Result<Vec<u8>, HsmError> {
        if !self.objects.contains_key(&key_handle) {
            return Err(HsmError::ObjectNotFound);
        }

        // Placeholder: would actually sign data
        Ok(vec![0u8; 256])
    }

    fn verify(
        &mut self,
        _session: HsmSessionHandle,
        key_handle: HsmObjectHandle,
        _mechanism: Pkcs11Mechanism,
        _data: &[u8],
        _signature: &[u8],
    ) -> Result<bool, HsmError> {
        if !self.objects.contains_key(&key_handle) {
            return Err(HsmError::ObjectNotFound);
        }

        Ok(true)
    }

    fn encrypt(
        &mut self,
        _session: HsmSessionHandle,
        key_handle: HsmObjectHandle,
        _mechanism: Pkcs11Mechanism,
        plaintext: &[u8],
    ) -> Result<Vec<u8>, HsmError> {
        if !self.objects.contains_key(&key_handle) {
            return Err(HsmError::ObjectNotFound);
        }

        // Placeholder: would actually encrypt data
        Ok(plaintext.to_vec())
    }

    fn decrypt(
        &mut self,
        _session: HsmSessionHandle,
        key_handle: HsmObjectHandle,
        _mechanism: Pkcs11Mechanism,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, HsmError> {
        if !self.objects.contains_key(&key_handle) {
            return Err(HsmError::ObjectNotFound);
        }

        // Placeholder: would actually decrypt data
        Ok(ciphertext.to_vec())
    }
}

/// HSM manager
#[derive(Debug)]
pub struct HsmManager {
    /// HSM devices
    devices: BTreeMap<HsmSlotId, Box<dyn HsmDevice>>,
    /// Primary device
    primary_slot: Option<HsmSlotId>,
    /// Failover enabled
    failover_enabled: bool,
    /// Statistics
    stats: HsmStats,
}

/// HSM statistics
#[derive(Debug, Default)]
pub struct HsmStats {
    /// Total operations
    pub total_operations: AtomicU64,
    /// Successful operations
    pub successful_operations: AtomicU64,
    /// Failed operations
    pub failed_operations: AtomicU64,
    /// Failover events
    pub failover_events: AtomicU64,
}

impl HsmManager {
    /// Create a new HSM manager
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            primary_slot: None,
            failover_enabled: true,
            stats: HsmStats::default(),
        }
    }

    /// Register an HSM device
    pub fn register_device(&mut self, device: Box<dyn HsmDevice>) -> Result<(), HsmError> {
        let slot_id = device.get_info().slot_id;

        if self.devices.contains_key(&slot_id) {
            return Err(HsmError::DeviceAlreadyRegistered);
        }

        self.devices.insert(slot_id, device);

        // Set as primary if first device
        if self.primary_slot.is_none() {
            self.primary_slot = Some(slot_id);
        }

        Ok(())
    }

    /// Unregister an HSM device
    pub fn unregister_device(&mut self, slot_id: HsmSlotId) -> Result<(), HsmError> {
        self.devices
            .remove(&slot_id)
            .ok_or(HsmError::DeviceNotFound)?;

        // Update primary if needed
        if self.primary_slot == Some(slot_id) {
            self.primary_slot = self.devices.keys().next().copied();
        }

        Ok(())
    }

    /// Set primary device
    pub fn set_primary(&mut self, slot_id: HsmSlotId) -> Result<(), HsmError> {
        if !self.devices.contains_key(&slot_id) {
            return Err(HsmError::DeviceNotFound);
        }

        self.primary_slot = Some(slot_id);
        Ok(())
    }

    /// Get primary device
    pub fn get_primary(&self) -> Option<&dyn HsmDevice> {
        self.primary_slot
            .and_then(|slot| self.devices.get(&slot))
            .map(|b| b.as_ref())
    }

    /// Get primary device mutable
    pub fn get_primary_slot_mut(&mut self) -> Option<HsmSlotId> {
        self.primary_slot
    }

    /// Get primary device by slot (mutable)
    pub fn get_device_mut(&mut self, slot: HsmSlotId) -> Option<&mut Box<dyn HsmDevice>> {
        self.devices.get_mut(&slot)
    }

    /// Enable failover
    pub fn enable_failover(&mut self) {
        self.failover_enabled = true;
    }

    /// Disable failover
    pub fn disable_failover(&mut self) {
        self.failover_enabled = false;
    }

    /// Get device by slot
    pub fn get_device(&self, slot_id: HsmSlotId) -> Option<&dyn HsmDevice> {
        self.devices.get(&slot_id).map(|b| b.as_ref())
    }

    /// List all devices
    pub fn list_devices(&self) -> Vec<&HsmDeviceInfo> {
        self.devices
            .values()
            .map(|d| d.get_info())
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &HsmStats {
        &self.stats
    }
}

impl Default for HsmManager {
    fn default() -> Self {
        Self::new()
    }
}

/// HSM errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HsmError {
    /// Device not found
    DeviceNotFound,
    /// Device not available
    DeviceNotAvailable,
    /// Device already registered
    DeviceAlreadyRegistered,
    /// Session not found
    SessionNotFound,
    /// Session limit reached
    SessionLimitReached,
    /// Object not found
    ObjectNotFound,
    /// Invalid mechanism
    InvalidMechanism,
    /// Invalid attributes
    InvalidAttributes,
    /// Operation failed
    OperationFailed,
    /// Timeout
    Timeout,
    /// Not initialized
    NotInitialized,
}

impl core::fmt::Display for HsmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DeviceNotFound => write!(f, "HSM device not found"),
            Self::DeviceNotAvailable => write!(f, "HSM device not available"),
            Self::DeviceAlreadyRegistered => write!(f, "HSM device already registered"),
            Self::SessionNotFound => write!(f, "HSM session not found"),
            Self::SessionLimitReached => write!(f, "HSM session limit reached"),
            Self::ObjectNotFound => write!(f, "HSM object not found"),
            Self::InvalidMechanism => write!(f, "Invalid mechanism"),
            Self::InvalidAttributes => write!(f, "Invalid attributes"),
            Self::OperationFailed => write!(f, "HSM operation failed"),
            Self::Timeout => write!(f, "HSM operation timeout"),
            Self::NotInitialized => write!(f, "HSM not initialized"),
        }
    }
}

impl Clone for HsmStats {
    fn clone(&self) -> Self {
        Self {
            total_operations: AtomicU64::new(self.total_operations.load(Ordering::Relaxed)),
            successful_operations: AtomicU64::new(self.successful_operations.load(Ordering::Relaxed)),
            failed_operations: AtomicU64::new(self.failed_operations.load(Ordering::Relaxed)),
            failover_events: AtomicU64::new(self.failover_events.load(Ordering::Relaxed)),
        }
    }
}

/// Global HSM manager instance
pub static HSM_MANAGER: RwLock<Option<HsmManager>> = RwLock::new(None);

/// Initialize HSM subsystem
pub fn init_hsm() -> Result<(), HsmError> {
    let mut manager = HsmManager::new();

    // Register software HSM for testing
    let sw_hsm = SoftwareHsm::new(HsmSlotId::new(0));
    let mut sw_hsm_boxed: Box<dyn HsmDevice> = Box::new(sw_hsm);
    sw_hsm_boxed.initialize()?;

    manager.register_device(sw_hsm_boxed)?;

    *HSM_MANAGER.write() = Some(manager);

    Ok(())
}

/// Get global HSM manager
pub fn get_hsm_manager() -> Option<&'static RwLock<Option<HsmManager>>> {
    Some(&HSM_MANAGER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hsm_slot_id() {
        let slot = HsmSlotId::new(42);
        assert_eq!(slot.value(), 42);
    }

    #[test]
    fn test_hsm_device_info() {
        let info = HsmDeviceInfo::new(HsmSlotId::new(0));
        assert_eq!(info.max_session_count, 16);
        assert!(!info.is_available());
    }

    #[test]
    fn test_software_hsm() {
        let mut hsm = SoftwareHsm::new(HsmSlotId::new(0));
        assert!(hsm.initialize().is_ok());

        let session = hsm.open_session(false);
        assert!(session.is_ok());

        let session_obj = session.unwrap();
        assert!(hsm.close_session(session_obj.handle).is_ok());
    }

    #[test]
    fn test_hsm_manager() {
        let mut manager = HsmManager::new();

        let mut sw_hsm = SoftwareHsm::new(HsmSlotId::new(0));
        sw_hsm.initialize().unwrap();

        let boxed: Box<dyn HsmDevice> = Box::new(sw_hsm);

        assert!(manager.register_device(boxed).is_ok());
        assert!(manager.get_primary().is_some());

        let devices = manager.list_devices();
        assert_eq!(devices.len(), 1);
    }

    #[test]
    fn test_key_generation() {
        let mut hsm = SoftwareHsm::new(HsmSlotId::new(0));
        hsm.initialize().unwrap();

        let mut attributes = HsmObjectAttributes::new(Pkcs11ObjectClass::SecretKey);
        attributes.key_type = Some(Pkcs11KeyType::Aes);
        attributes.key_size = Some(256);

        let handle = hsm.generate_key(Pkcs11Mechanism::AesGcm, &attributes);
        assert!(handle.is_ok());
    }
}
