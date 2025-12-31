//! # CFI Type Metadata System
//!
//! This module provides comprehensive type metadata for Control Flow Integrity (CFI).
//!
//! ## Overview
//!
//! The type metadata system tracks:
//! - Function pointer types and their valid targets
//! - Virtual table (vtable) layouts for dynamic dispatch
//! - Type relationships for inheritance checking
//! - Function signatures for indirect call validation
//!
//! ## Features
//!
//! - **Automatic Type Registration**: Compile-time type ID generation
//! - **VTable Validation**: Runtime vtable integrity checks
//! - **Function Pointer Tracking**: Comprehensive target validation
//! - **Type Relationships**: Inheritance and interface tracking
//! - **Fine-grained Validation**: Per-function type checking
//!
//! ## Usage
//!
//! ```no_run
//! use kernel::security::cfi::types::{CfiTypeMetadata, CfiTypeTable};
//!
//! // Create type metadata
//! let metadata = CfiTypeMetadata::function(
//!     "my_function",
//!     0x12345,
//!     &[0x11111, 0x22222],  // Valid targets
//! );
//!
//! // Register in global table
//! let mut table = CfiTypeTable::new();
//! table.register_type(metadata);
//!
//! // Validate indirect call
//! if table.is_valid_call(0x12345, 0x11111) {
//!     // Safe to call
//! }
//! ```

#![allow(dead_code)]

use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::{
    fmt,
    hash::{Hash, Hasher},
    sync::atomic::{AtomicU64, Ordering},
};

use spin::Mutex;

/// Unique type identifier for CFI metadata
pub type CfiTypeId = u64;

/// Hash function for generating type IDs from type names
#[derive(Debug)]
pub struct CfiTypeHasher {
    state: u64,
}

impl CfiTypeHasher {
    pub fn new() -> Self {
        Self {
            state: 0x9e3779b97f4a7c15, // Golden ratio prime
        }
    }

    pub fn hash_bytes(&mut self, bytes: &[u8]) -> u64 {
        let mut hash = self.state;
        for &byte in bytes {
            hash = hash.wrapping_mul(0x100000001b3).wrapping_add(byte as u64);
            hash ^= hash >> 44;
        }
        hash
    }

    pub fn hash_str(&mut self, s: &str) -> u64 {
        self.hash_bytes(s.as_bytes())
    }
}

impl Default for CfiTypeHasher {
    fn default() -> Self {
        Self::new()
    }
}

/// Type categories for CFI validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CfiTypeCategory {
    /// Regular function pointer
    FunctionPointer,
    /// Virtual method (in vtable)
    VirtualMethod,
    /// Interface method
    InterfaceMethod,
    /// Static function
    StaticFunction,
    /// Callback function
    Callback,
    /// Unknown/Other
    Other,
}

/// Function signature metadata
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    /// Return type identifier
    pub return_type: CfiTypeId,
    /// Parameter type identifiers
    pub param_types: Vec<CfiTypeId>,
    /// Whether this is a varargs function
    pub is_varargs: bool,
    /// Calling convention
    pub calling_convention: CallingConvention,
}

/// Supported calling conventions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallingConvention {
    /// C calling convention
    C,
    /// Fastcall (x86-specific)
    FastCall,
    /// Thiscall (x86 Microsoft-specific)
    ThisCall,
    /// Stdcall (Windows API)
    StdCall,
    /// Vector call (for SIMD)
    VectorCall,
    /// AArch64 calling convention
    AArch64,
    /// RISC-V calling convention
    RiscV,
}

/// CFI type metadata for tracking function pointers and vtables
#[derive(Debug, Clone)]
pub struct CfiTypeMetadata {
    /// Unique type identifier
    pub type_id: CfiTypeId,
    /// Human-readable type name
    pub type_name: String,
    /// Type category
    pub category: CfiTypeCategory,
    /// Function signature (if applicable)
    pub signature: Option<FunctionSignature>,
    /// VTable address (for virtual methods)
    pub vtable_addr: Option<usize>,
    /// Valid indirect call targets for this type
    pub valid_targets: Vec<usize>,
    /// Base type ID (for inheritance)
    pub base_type_id: Option<CfiTypeId>,
    /// Related interfaces
    pub interfaces: Vec<CfiTypeId>,
    /// Type flags
    pub flags: CfiTypeFlags,
}

/// Bitflags for type properties
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CfiTypeFlags(u32);

impl CfiTypeFlags {
    /// No flags
    pub const EMPTY: Self = Self(0);
    /// Type is final (cannot be inherited)
    pub const FINAL: Self = Self(1 << 0);
    /// Type is abstract (cannot be instantiated)
    pub const ABSTRACT: Self = Self(1 << 1);
    /// Type is sealed (limited inheritance)
    pub const SEALED: Self = Self(1 << 2);
    /// Type has destructor
    pub const HAS_DESTRUCTOR: Self = Self(1 << 3);
    /// Type is polymorphic (has virtual methods)
    pub const POLYMORPHIC: Self = Self(1 << 4);
    /// Type is exported (visible across modules)
    pub const EXPORTED: Self = Self(1 << 5);
    /// Type is validated (metadata verified)
    pub const VALIDATED: Self = Self(1 << 6);

    pub fn empty() -> Self {
        Self::EMPTY
    }

    pub fn contains(self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn insert(&mut self, flag: Self) {
        self.0 |= flag.0;
    }

    pub fn remove(&mut self, flag: Self) {
        self.0 &= !flag.0;
    }
}

impl CfiTypeMetadata {
    /// Create metadata for a regular function pointer
    pub fn function(
        name: &str,
        address: usize,
        valid_targets: &[usize],
    ) -> Self {
        let mut hasher = CfiTypeHasher::new();
        let type_id = hasher.hash_str(name);

        Self {
            type_id,
            type_name: name.to_string(),
            category: CfiTypeCategory::FunctionPointer,
            signature: None,
            vtable_addr: None,
            valid_targets: valid_targets.to_vec(),
            base_type_id: None,
            interfaces: Vec::new(),
            flags: CfiTypeFlags::empty(),
        }
    }

    /// Create metadata for a virtual method
    pub fn virtual_method(
        name: &str,
        address: usize,
        vtable_addr: usize,
        valid_targets: &[usize],
    ) -> Self {
        let mut hasher = CfiTypeHasher::new();
        let type_id = hasher.hash_str(name);

        let mut flags = CfiTypeFlags::empty();
        flags.insert(CfiTypeFlags::POLYMORPHIC);

        Self {
            type_id,
            type_name: name.to_string(),
            category: CfiTypeCategory::VirtualMethod,
            signature: None,
            vtable_addr: Some(vtable_addr),
            valid_targets: valid_targets.to_vec(),
            base_type_id: None,
            interfaces: Vec::new(),
            flags,
        }
    }

    /// Create metadata for a callback function
    pub fn callback(name: &str, address: usize) -> Self {
        let mut hasher = CfiTypeHasher::new();
        let type_id = hasher.hash_str(name);

        Self {
            type_id,
            type_name: name.to_string(),
            category: CfiTypeCategory::Callback,
            signature: None,
            vtable_addr: None,
            valid_targets: Vec::new(),
            base_type_id: None,
            interfaces: Vec::new(),
            flags: CfiTypeFlags::empty(),
        }
    }

    /// Create metadata with full function signature
    pub fn with_signature(
        mut self,
        return_type: CfiTypeId,
        param_types: Vec<CfiTypeId>,
        calling_convention: CallingConvention,
    ) -> Self {
        self.signature = Some(FunctionSignature {
            return_type,
            param_types,
            is_varargs: false,
            calling_convention,
        });
        self
    }

    /// Add inheritance relationship
    pub fn with_base_type(mut self, base_type_id: CfiTypeId) -> Self {
        self.base_type_id = Some(base_type_id);
        self
    }

    /// Add interface implementation
    pub fn with_interface(mut self, interface_id: CfiTypeId) -> Self {
        self.interfaces.push(interface_id);
        self
    }

    /// Set type flags
    pub fn with_flags(mut self, flags: CfiTypeFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Check if this is a virtual method
    pub fn is_virtual(&self) -> bool {
        self.category == CfiTypeCategory::VirtualMethod
    }

    /// Check if this type is polymorphic
    pub fn is_polymorphic(&self) -> bool {
        self.flags.contains(CfiTypeFlags::POLYMORPHIC)
    }

    /// Check if call to target is valid for this type
    pub fn is_valid_target(&self, target: usize) -> bool {
        self.valid_targets.contains(&target)
    }
}

impl fmt::Display for CfiTypeMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CfiTypeMetadata{{ id={}, name={}, category={:?} }}",
            self.type_id, self.type_name, self.category
        )
    }
}

/// Global CFI type table
pub struct CfiTypeTable {
    /// Type ID to metadata mapping
    types: BTreeMap<CfiTypeId, CfiTypeMetadata>,
    /// Name to type ID mapping for quick lookup
    name_to_id: BTreeMap<String, CfiTypeId>,
    /// Address to type ID mapping for runtime validation
    addr_to_id: BTreeMap<usize, CfiTypeId>,
    /// Statistics
    stats: CfiTypeTableStats,
    /// Type ID counter for generating new IDs
    next_type_id: AtomicU64,
    /// Global table lock
    lock: Mutex<()>,
}

/// Type table statistics
#[derive(Debug, Default)]
pub struct CfiTypeTableStats {
    /// Total registered types
    pub total_types: AtomicU64,
    /// Function pointer types
    pub function_pointers: AtomicU64,
    /// Virtual methods
    pub virtual_methods: AtomicU64,
    /// Interface methods
    pub interface_methods: AtomicU64,
    /// Validation checks performed
    pub validation_checks: AtomicU64,
    /// Validations passed
    pub validations_passed: AtomicU64,
    /// Validations failed
    pub validations_failed: AtomicU64,
}

impl CfiTypeTable {
    /// Create a new CFI type table
    pub fn new() -> Self {
        Self {
            types: BTreeMap::new(),
            name_to_id: BTreeMap::new(),
            addr_to_id: BTreeMap::new(),
            stats: CfiTypeTableStats::default(),
            next_type_id: AtomicU64::new(1),
            lock: Mutex::new(()),
        }
    }

    /// Register a type in the table
    pub fn register_type(&mut self, metadata: CfiTypeMetadata) -> Result<(), CfiError> {
        let _guard = self.lock.lock();

        let type_id = metadata.type_id;
        let type_name = metadata.type_name.clone();
        let category = metadata.category;

        // Insert into all indexes
        self.types.insert(type_id, metadata);
        self.name_to_id.insert(type_name, type_id);

        // Update statistics
        self.stats.total_types.fetch_add(1, Ordering::Relaxed);
        match category {
            CfiTypeCategory::FunctionPointer => {
                self.stats.function_pointers.fetch_add(1, Ordering::Relaxed);
            }
            CfiTypeCategory::VirtualMethod => {
                self.stats.virtual_methods.fetch_add(1, Ordering::Relaxed);
            }
            CfiTypeCategory::InterfaceMethod => {
                self.stats.interface_methods.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }

        Ok(())
    }

    /// Register address mapping for a type
    pub fn register_address(&mut self, type_id: CfiTypeId, addr: usize) {
        let _guard = self.lock.lock();
        self.addr_to_id.insert(addr, type_id);
    }

    /// Get type metadata by ID
    pub fn get_type(&self, type_id: CfiTypeId) -> Option<&CfiTypeMetadata> {
        self.types.get(&type_id)
    }

    /// Get type metadata by name
    pub fn get_type_by_name(&self, name: &str) -> Option<&CfiTypeMetadata> {
        if let Some(type_id) = self.name_to_id.get(name) {
            self.types.get(type_id)
        } else {
            None
        }
    }

    /// Get type ID by address
    pub fn get_type_id_by_addr(&self, addr: usize) -> Option<CfiTypeId> {
        self.addr_to_id.get(&addr).copied()
    }

    /// Get valid targets for a type
    pub fn get_valid_targets(&self, type_id: CfiTypeId) -> &[usize] {
        if let Some(metadata) = self.types.get(&type_id) {
            &metadata.valid_targets
        } else {
            &[]
        }
    }

    /// Validate an indirect call from one type to another
    pub fn is_valid_call(&self, from_type: CfiTypeId, to_addr: usize) -> bool {
        self.stats.validation_checks.fetch_add(1, Ordering::Relaxed);

        // Get the source type metadata
        let from_metadata = match self.get_type(from_type) {
            Some(m) => m,
            None => {
                self.stats.validations_failed.fetch_add(1, Ordering::Relaxed);
                return false;
            }
        };

        // Check if target address is in valid targets list
        let is_valid = from_metadata.is_valid_target(to_addr);

        if is_valid {
            self.stats.validations_passed.fetch_add(1, Ordering::Relaxed);
        } else {
            self.stats.validations_failed.fetch_add(1, Ordering::Relaxed);
        }

        is_valid
    }

    /// Validate virtual method call (checks vtable integrity)
    pub fn is_valid_virtual_call(
        &self,
        vtable_addr: usize,
        method_offset: usize,
        target_addr: usize,
    ) -> bool {
        // Get type ID from vtable address
        let type_id = match self.get_type_id_by_addr(vtable_addr) {
            Some(id) => id,
            None => return false,
        };

        // Get type metadata
        let metadata = match self.get_type(type_id) {
            Some(m) => m,
            None => return false,
        };

        // Verify this is a virtual method
        if !metadata.is_virtual() {
            return false;
        }

        // Check if target is valid
        metadata.is_valid_target(target_addr)
    }

    /// Generate a new unique type ID
    pub fn generate_type_id(&self) -> CfiTypeId {
        self.next_type_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Get statistics
    pub fn get_stats(&self) -> &CfiTypeTableStats {
        &self.stats
    }

    /// Get all registered types
    pub fn get_all_types(&self) -> impl Iterator<Item = &CfiTypeMetadata> {
        self.types.values()
    }

    /// Find types by category
    pub fn find_types_by_category(&self, category: CfiTypeCategory) -> Vec<CfiTypeMetadata> {
        self.types
            .values()
            .filter(|m| m.category == category)
            .cloned()
            .collect()
    }

    /// Get inheritance chain for a type
    pub fn get_inheritance_chain(&self, type_id: CfiTypeId) -> Vec<CfiTypeId> {
        let mut chain = Vec::new();
        let mut current_id = type_id;

        while let Some(metadata) = self.get_type(current_id) {
            chain.push(current_id);
            if let Some(base_id) = metadata.base_type_id {
                current_id = base_id;
            } else {
                break;
            }
        }

        chain
    }

    /// Check if type implements an interface
    pub fn implements_interface(&self, type_id: CfiTypeId, interface_id: CfiTypeId) -> bool {
        if let Some(metadata) = self.get_type(type_id) {
            if metadata.interfaces.contains(&interface_id) {
                return true;
            }
            // Check base types
            if let Some(base_id) = metadata.base_type_id {
                return self.implements_interface(base_id, interface_id);
            }
        }
        false
    }
}

impl Default for CfiTypeTable {
    fn default() -> Self {
        Self::new()
    }
}

/// CFI errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CfiError {
    /// Type already registered
    TypeAlreadyRegistered { type_id: CfiTypeId },
    /// Type not found
    TypeNotFound { type_id: CfiTypeId },
    /// Invalid vtable
    InvalidVTable { address: usize },
    /// Validation failed
    ValidationFailed { from_type: CfiTypeId, target: usize },
}

impl fmt::Display for CfiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TypeAlreadyRegistered { type_id } => {
                write!(f, "Type {} already registered", type_id)
            }
            Self::TypeNotFound { type_id } => {
                write!(f, "Type {} not found", type_id)
            }
            Self::InvalidVTable { address } => {
                write!(f, "Invalid vtable at address 0x{:x}", address)
            }
            Self::ValidationFailed { from_type, target } => {
                write!(
                    f,
                    "CFI validation failed: type {} cannot call address 0x{:x}",
                    from_type, target
                )
            }
        }
    }
}

/// Global CFI type table instance
static GLOBAL_TYPE_TABLE: Mutex<Option<CfiTypeTable>> = Mutex::new(None);

/// Initialize the global CFI type table
pub fn init_type_table() {
    let mut table = GLOBAL_TYPE_TABLE.lock();
    if table.is_none() {
        *table = Some(CfiTypeTable::new());
    }
}

/// Get the global CFI type table
pub fn get_type_table() -> &'static Mutex<Option<CfiTypeTable>> {
    &GLOBAL_TYPE_TABLE
}

/// Convenience function to register a function pointer type
pub fn register_function_type(
    name: &str,
    address: usize,
    valid_targets: &[usize],
) -> Result<CfiTypeId, CfiError> {
    let metadata = CfiTypeMetadata::function(name, address, valid_targets);
    let type_id = metadata.type_id;

    if let Some(table) = GLOBAL_TYPE_TABLE.lock().as_mut() {
        table.register_type(metadata)?;
        Ok(type_id)
    } else {
        // Initialize if not already initialized
        init_type_table();
        register_function_type(name, address, valid_targets)
    }
}

/// Convenience function to validate an indirect call
pub fn validate_indirect_call(from_type: CfiTypeId, target_addr: usize) -> bool {
    if let Some(table) = GLOBAL_TYPE_TABLE.lock().as_ref() {
        table.is_valid_call(from_type, target_addr)
    } else {
        // If CFI not initialized, fail open (allow the call)
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_id_generation() {
        let hasher1 = CfiTypeHasher::new();
        let hasher2 = CfiTypeHasher::new();

        let id1 = hasher1.hash_str("test_function");
        let id2 = hasher2.hash_str("test_function");

        assert_eq!(id1, id2);

        let id3 = hasher1.hash_str("other_function");
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_function_metadata() {
        let metadata = CfiTypeMetadata::function(
            "my_function",
            0x1000,
            &[0x2000, 0x3000],
        );

        assert_eq!(metadata.type_name, "my_function");
        assert_eq!(metadata.category, CfiTypeCategory::FunctionPointer);
        assert!(metadata.is_valid_target(0x2000));
        assert!(!metadata.is_valid_target(0x9999));
    }

    #[test]
    fn test_virtual_method_metadata() {
        let metadata = CfiTypeMetadata::virtual_method(
            "virtual_method",
            0x1000,
            0x5000,
            &[0x2000, 0x3000],
        );

        assert_eq!(metadata.type_name, "virtual_method");
        assert_eq!(metadata.category, CfiTypeCategory::VirtualMethod);
        assert!(metadata.is_virtual());
        assert!(metadata.is_polymorphic());
        assert_eq!(metadata.vtable_addr, Some(0x5000));
    }

    #[test]
    fn test_type_table_registration() {
        let mut table = CfiTypeTable::new();
        let metadata = CfiTypeMetadata::function("test", 0x1000, &[0x2000]);

        assert!(table.register_type(metadata).is_ok());
        assert_eq!(table.stats.total_types.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_type_validation() {
        let mut table = CfiTypeTable::new();
        let metadata = CfiTypeMetadata::function("test", 0x1000, &[0x2000, 0x3000]);
        let type_id = metadata.type_id;

        table.register_type(metadata).unwrap();
        table.register_address(type_id, 0x1000);

        assert!(table.is_valid_call(type_id, 0x2000));
        assert!(!table.is_valid_call(type_id, 0x9999));
    }

    #[test]
    fn test_type_flags() {
        let mut flags = CfiTypeFlags::empty();

        assert!(!flags.contains(CfiTypeFlags::FINAL));
        assert!(!flags.contains(CfiTypeFlags::POLYMORPHIC));

        flags.insert(CfiTypeFlags::FINAL);
        flags.insert(CfiTypeFlags::POLYMORPHIC);

        assert!(flags.contains(CfiTypeFlags::FINAL));
        assert!(flags.contains(CfiTypeFlags::POLYMORPHIC));

        flags.remove(CfiTypeFlags::FINAL);
        assert!(!flags.contains(CfiTypeFlags::FINAL));
        assert!(flags.contains(CfiTypeFlags::POLYMORPHIC));
    }

    #[test]
    fn test_inheritance_chain() {
        let mut table = CfiTypeTable::new();

        // Create inheritance hierarchy: Base -> Derived
        let base = CfiTypeMetadata::function("base", 0x1000, &[])
            .with_flags(CfiTypeFlags::POLYMORPHIC);
        let derived = CfiTypeMetadata::function("derived", 0x2000, &[])
            .with_base_type(base.type_id);

        let base_id = base.type_id;
        let derived_id = derived.type_id;

        table.register_type(base).unwrap();
        table.register_type(derived).unwrap();

        let chain = table.get_inheritance_chain(derived_id);
        assert_eq!(chain, vec![derived_id, base_id]);
    }
}
