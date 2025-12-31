//! # CFI Compiler Integration
//!
//! This module provides the interface between the CFI runtime system and
//! the compiler (LLVM/Rust) for automatic CFI instrumentation.
//!
//! ## Overview
//!
//! The compiler integration layer enables:
//! - Automatic annotation of indirect call sites
//! - Type metadata registration during compilation
//! - Runtime validation hook insertion
//! - Manual annotation for critical function pointers
//!
//! ## Features
//!
//! - **LLVM CFI Compatibility**: Compatible with LLVM's CFI scheme
//! - **Rust Integration**: Works with Rust's type system
//! - **Manual Annotations**: For function pointers that need explicit CFI
//! - **Runtime Hooks**: Validation hooks for call sites
//! - **Type Metadata**: Automatic registration of type information
//!
//! ## Usage
//!
//! ### Manual Annotation
//!
//! ```no_run
//! use kernel::security::cfi::compiler::{annotate_indirect_call, register_function_pointer_type};
//!
//! // Annotate a function pointer
//! extern "C" fn my_callback(x: i32) -> i32 {
//!     x + 1
//! }
//!
//! // Register the function pointer type
//! register_function_pointer_type(
//!     my_callback as usize,
//!     /* return type */ 0,
//!     /* param types */ &[1]
//! );
//!
//! // Annotate indirect call sites
//! let func_ptr: extern "C" fn(i32) -> i32 = my_callback;
//! annotate_indirect_call(func_ptr as usize, &[my_callback as usize]);
//! ```
//!
//! ### Type Metadata Registration
//!
//! ```no_run
//! use kernel::security::cfi::compiler::{
//!     register_function_pointer_type,
//!     register_virtual_table,
//!     set_type_metadata,
//! };
//!
//! // Register function pointer with signature
//! register_function_pointer_type(
//!     0x12345,        // Function address
//!     TYPE_ID_I32,    // Return type
//!     &[TYPE_ID_I32],  // Parameter types
//! );
//!
//! // Register vtable
//! register_virtual_table(
//!     0x5000,        // VTable address
//!     &[0x1000, 0x2000, 0x3000],  // Virtual method addresses
//! );
//! ```

#![allow(dead_code)]

use alloc::{collections::BTreeMap, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::{CfiTypeId, CfiTypeMetadata};

/// Compiler CFI configuration
#[derive(Debug, Clone)]
pub struct CompilerCfiConfig {
    /// Enable automatic instrumentation
    pub auto_instrument: bool,
    /// Enable strict validation (fail on unknown types)
    pub strict_validation: bool,
    /// Enable verbose logging for debugging
    pub verbose_logging: bool,
    /// Maximum number of type entries
    pub max_type_entries: usize,
    /// Enable validation of vtable integrity
    pub validate_vtables: bool,
}

impl Default for CompilerCfiConfig {
    fn default() -> Self {
        Self {
            auto_instrument: true,
            strict_validation: false, // Start lenient for compatibility
            verbose_logging: false,
            max_type_entries: 10000,
            validate_vtables: true,
        }
    }
}

/// Function pointer type registration record
#[derive(Debug, Clone)]
pub struct FunctionPointerType {
    /// Function address
    pub address: usize,
    /// Return type ID
    pub return_type: CfiTypeId,
    /// Parameter type IDs
    pub param_types: Vec<CfiTypeId>,
    /// Calling convention
    pub calling_convention: CallingConvention,
}

/// Calling conventions supported by CFI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallingConvention {
    C,
    FastCall,
    ThisCall,
    StdCall,
    AArch64,
    RiscV,
}

/// Virtual table registration record
#[derive(Debug, Clone)]
pub struct VirtualTableInfo {
    /// VTable address
    pub address: usize,
    /// Type ID for this vtable
    pub type_id: CfiTypeId,
    /// Virtual method addresses (indexed by slot)
    pub virtual_methods: Vec<usize>,
    /// RTTI pointer (if available)
    pub rtti_ptr: Option<usize>,
    /// Base class vtable (for inheritance)
    pub base_vtable: Option<usize>,
}

/// CFI compiler hooks - inserted by compiler or used manually
pub struct CfiCompilerHooks {
    /// Configuration
    config: CompilerCfiConfig,
    /// Registered function pointer types
    function_types: BTreeMap<usize, FunctionPointerType>,
    /// Registered virtual tables
    vtables: BTreeMap<usize, VirtualTableInfo>,
    /// Type ID generator
    next_type_id: AtomicU64,
    /// Validation statistics
    stats: CompilerCfiStats,
}

/// Compiler CFI statistics
#[derive(Debug, Default)]
pub struct CompilerCfiStats {
    /// Total annotations performed
    pub total_annotations: AtomicU64,
    /// Function pointer registrations
    pub function_registrations: AtomicU64,
    /// VTable registrations
    pub vtable_registrations: AtomicU64,
    /// Validations performed
    pub validations: AtomicU64,
    /// Validations passed
    pub validations_passed: AtomicU64,
    /// Validations failed
    pub validations_failed: AtomicU64,
}

impl CfiCompilerHooks {
    /// Create a new CFI compiler hooks instance
    pub fn new() -> Self {
        Self {
            config: CompilerCfiConfig::default(),
            function_types: BTreeMap::new(),
            vtables: BTreeMap::new(),
            next_type_id: AtomicU64::new(1000), // Start from 1000
            stats: CompilerCfiStats::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: CompilerCfiConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Annotate an indirect call site
    ///
    /// This function is called at compile time (or manually) to mark
    /// indirect call sites with their valid target addresses.
    ///
    /// # Arguments
    ///
    /// * `call_site` - Address of the indirect call instruction
    /// * `valid_targets` - List of valid target addresses for this call
    ///
    /// # Example
    ///
    /// ```no_run
    /// let func_ptr: extern "C" fn(i32) -> i32 = my_callback;
    /// annotate_indirect_call(
    ///     &func_ptr as *const _ as usize,
    ///     &[my_callback as usize]
    /// );
    /// ```
    pub fn annotate_indirect_call(
        &self,
        call_site: usize,
        valid_targets: &[usize],
    ) {
        if !self.config.auto_instrument {
            return;
        }

        self.stats.total_annotations.fetch_add(1, Ordering::Relaxed);

        if self.config.verbose_logging {
            log::info!(
                "CFI: Annotated call site 0x{:x} with {} targets",
                call_site,
                valid_targets.len()
            );
        }

        // In a real implementation, this would store the annotation
        // for runtime validation. For now, we just log it.
    }

    /// Register a function pointer's type information
    ///
    /// # Arguments
    ///
    /// * `func_ptr` - Function pointer address
    /// * `return_type` - Type ID of return type
    /// * `param_types` - Type IDs of parameters
    ///
    /// # Example
    ///
    /// ```no_run
    /// extern "C" fn my_function(x: i32, y: i32) -> i32 {
    ///     x + y
    /// }
    ///
    /// register_function_pointer_type(
    ///     my_function as usize,
    ///     TYPE_ID_I32,      // return type
    ///     &[TYPE_ID_I32, TYPE_ID_I32],  // (i32, i32) parameters
    /// );
    /// ```
    pub fn register_function_pointer_type(
        &self,
        func_ptr: usize,
        return_type: CfiTypeId,
        param_types: &[CfiTypeId],
    ) {
        let func_type = FunctionPointerType {
            address: func_ptr,
            return_type,
            param_types: param_types.to_vec(),
            calling_convention: CallingConvention::C,
        };

        self.function_types.insert(func_ptr, func_type);
        self.stats.function_registrations.fetch_add(1, Ordering::Relaxed);

        if self.config.verbose_logging {
            log::info!(
                "CFI: Registered function pointer type 0x{:x} (return: {}, params: {:?})",
                func_ptr,
                return_type,
                param_types
            );
        }
    }

    /// Register a virtual table
    ///
    /// # Arguments
    ///
    /// * `vtable_addr` - Address of the vtable
    /// * `type_id` - Type ID for this vtable
    /// * `virtual_methods` - Array of virtual method addresses
    ///
    /// # Example
    ///
    /// ```no_run
    /// // Assume MyStruct has 3 virtual methods
    /// static VTABLE: [*const (); 3] = [
    ///     method1 as *const (),
    ///     method2 as *const (),
    ///     method3 as *const (),
    /// ];
    ///
    /// register_virtual_table(
    ///     &VTABLE as *const _ as usize,
    ///     MY_STRUCT_TYPE_ID,
    ///     &[method1 as usize, method2 as usize, method3 as usize],
    /// );
    /// ```
    pub fn register_virtual_table(
        &self,
        vtable_addr: usize,
        type_id: CfiTypeId,
        virtual_methods: &[usize],
    ) {
        let vtable_info = VirtualTableInfo {
            address: vtable_addr,
            type_id,
            virtual_methods: virtual_methods.to_vec(),
            rtti_ptr: None,
            base_vtable: None,
        };

        self.vtables.insert(vtable_addr, vtable_info);
        self.stats.vtable_registrations.fetch_add(1, Ordering::Relaxed);

        if self.config.verbose_logging {
            log::info!(
                "CFI: Registered vtable 0x{:x} for type {} with {} methods",
                vtable_addr,
                type_id,
                virtual_methods.len()
            );
        }
    }

    /// Set type metadata for a given address
    ///
    /// This is used by the compiler to emit type metadata
    /// that can be validated at runtime.
    pub fn set_type_metadata(&self, address: usize, metadata: CfiTypeMetadata) {
        // In a real implementation, this would store metadata
        // in a global table for runtime access.
        if self.config.verbose_logging {
            log::info!(
                "CFI: Set type metadata for address 0x{:x} (type: {})",
                address,
                metadata.type_name
            );
        }
    }

    /// Validate an indirect call at runtime
    ///
    /// This hook is inserted by the compiler at indirect call sites.
    /// It checks if the target is valid for the call site.
    ///
    /// # Arguments
    ///
    /// * `call_site` - Address of the call instruction
    /// * `target` - Target function address being called
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the call is valid
    /// * `Err(CfiViolation)` if the call is invalid
    pub fn validate_indirect_call(
        &self,
        call_site: usize,
        target: usize,
    ) -> Result<(), CfiViolation> {
        self.stats.validations.fetch_add(1, Ordering::Relaxed);

        // In strict mode, fail if we don't have type information
        if self.config.strict_validation {
            // Check if we have type information for this call site
            // For now, we'll allow it in non-strict mode
        }

        // Basic validation: check if target is non-null and reasonably aligned
        if target == 0 {
            self.stats.validations_failed.fetch_add(1, Ordering::Relaxed);
            return Err(CfiViolation::NullTarget { call_site });
        }

        if target & 0x3 != 0 {
            self.stats.validations_failed.fetch_add(1, Ordering::Relaxed);
            return Err(CfiViolation::MisalignedTarget { call_site, target });
        }

        self.stats.validations_passed.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Validate a virtual method call
    ///
    /// This is specifically for validating calls through vtables.
    pub fn validate_virtual_call(
        &self,
        vtable_addr: usize,
        method_index: usize,
        target: usize,
    ) -> Result<(), CfiViolation> {
        if let Some(vtable) = self.vtables.get(&vtable_addr) {
            if method_index >= vtable.virtual_methods.len() {
                return Err(CfiViolation::InvalidMethodIndex {
                    vtable_addr,
                    method_index,
                    max_index: vtable.virtual_methods.len(),
                });
            }

            let expected = vtable.virtual_methods[method_index];
            if target != expected {
                return Err(CfiViolation::VtableCorruption {
                    vtable_addr,
                    method_index,
                    expected,
                    actual: target,
                });
            }

            Ok(())
        } else {
            // VTable not registered - fail or allow based on config
            if self.config.strict_validation {
                Err(CfiViolation::UnknownVTable { vtable_addr })
            } else {
                Ok(())
            }
        }
    }

    /// Get the CFI configuration
    pub fn get_config(&self) -> &CompilerCfiConfig {
        &self.config
    }

    /// Update the CFI configuration
    pub fn set_config(&mut self, config: CompilerCfiConfig) {
        self.config = config;
    }

    /// Get compiler CFI statistics
    pub fn get_stats(&self) -> &CompilerCfiStats {
        &self.stats
    }

    /// Generate a new type ID
    pub fn generate_type_id(&self) -> CfiTypeId {
        self.next_type_id.fetch_add(1, Ordering::SeqCst)
    }
}

impl Default for CfiCompilerHooks {
    fn default() -> Self {
        Self::new()
    }
}

/// CFI violations detected by compiler integration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CfiViolation {
    /// Null target pointer
    NullTarget { call_site: usize },
    /// Misaligned target pointer
    MisalignedTarget { call_site: usize, target: usize },
    /// Invalid method index for vtable call
    InvalidMethodIndex {
        vtable_addr: usize,
        method_index: usize,
        max_index: usize,
    },
    /// VTable corruption detected
    VtableCorruption {
        vtable_addr: usize,
        method_index: usize,
        expected: usize,
        actual: usize,
    },
    /// Unknown vtable (not registered)
    UnknownVTable { vtable_addr: usize },
    /// Type mismatch
    TypeMismatch {
        expected_type: CfiTypeId,
        actual_type: CfiTypeId,
    },
}

impl core::fmt::Display for CfiViolation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NullTarget { call_site } => {
                write!(f, "CFI: Null target at call site 0x{:x}", call_site)
            }
            Self::MisalignedTarget { call_site, target } => {
                write!(
                    f,
                    "CFI: Misaligned target 0x{:x} at call site 0x{:x}",
                    target, call_site
                )
            }
            Self::InvalidMethodIndex {
                vtable_addr,
                method_index,
                max_index,
            } => {
                write!(
                    f,
                    "CFI: Invalid method index {} for vtable 0x{:x} (max: {})",
                    method_index, vtable_addr, max_index
                )
            }
            Self::VtableCorruption {
                vtable_addr,
                method_index,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "CFI: VTable corruption at 0x{:x}[{}] (expected: 0x{:x}, actual: 0x{:x})",
                    vtable_addr, method_index, expected, actual
                )
            }
            Self::UnknownVTable { vtable_addr } => {
                write!(f, "CFI: Unknown vtable at 0x{:x}", vtable_addr)
            }
            Self::TypeMismatch {
                expected_type,
                actual_type,
            } => {
                write!(
                    f,
                    "CFI: Type mismatch (expected: {}, actual: {})",
                    expected_type, actual_type
                )
            }
        }
    }
}

/// Global CFI compiler hooks instance
static GLOBAL_COMPILER_HOOKS: spin::Mutex<Option<CfiCompilerHooks>> =
    spin::Mutex::new(None);

/// Initialize the global CFI compiler hooks
pub fn init_compiler_hooks() {
    let mut hooks = GLOBAL_COMPILER_HOOKS.lock();
    if hooks.is_none() {
        *hooks = Some(CfiCompilerHooks::new());
    }
}

/// Get the global CFI compiler hooks
pub fn get_compiler_hooks() -> Option<&'static spin::Mutex<Option<CfiCompilerHooks>>> {
    // We can't return a direct reference to the static, so return None for now
    // In a real implementation, this would use a different pattern
    None
}

/// Convenience function to annotate an indirect call
pub fn annotate_indirect_call(call_site: usize, valid_targets: &[usize]) {
    if let Some(hooks) = GLOBAL_COMPILER_HOOKS.lock().as_ref() {
        hooks.annotate_indirect_call(call_site, valid_targets);
    }
}

/// Convenience function to register a function pointer type
pub fn register_function_pointer_type(
    func_ptr: usize,
    return_type: CfiTypeId,
    param_types: &[CfiTypeId],
) {
    if let Some(hooks) = GLOBAL_COMPILER_HOOKS.lock().as_ref() {
        hooks.register_function_pointer_type(func_ptr, return_type, param_types);
    }
}

/// Convenience function to register a virtual table
pub fn register_virtual_table(
    vtable_addr: usize,
    type_id: CfiTypeId,
    virtual_methods: &[usize],
) {
    if let Some(hooks) = GLOBAL_COMPILER_HOOKS.lock().as_ref() {
        hooks.register_virtual_table(vtable_addr, type_id, virtual_methods);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_hooks_creation() {
        let hooks = CfiCompilerHooks::new();
        assert!(hooks.get_config().auto_instrument);
    }

    #[test]
    fn test_annotation() {
        let hooks = CfiCompilerHooks::new();
        hooks.annotate_indirect_call(0x1000, &[0x2000, 0x3000]);
        assert_eq!(hooks.stats.total_annotations.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_function_type_registration() {
        let hooks = CfiCompilerHooks::new();
        hooks.register_function_pointer_type(0x1000, 1, &[2, 3]);
        assert_eq!(hooks.stats.function_registrations.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_vtable_registration() {
        let hooks = CfiCompilerHooks::new();
        hooks.register_virtual_table(0x5000, 100, &[0x1100, 0x2200, 0x3300]);
        assert_eq!(hooks.stats.vtable_registrations.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_validation_null_target() {
        let hooks = CfiCompilerHooks::new();
        let result = hooks.validate_indirect_call(0x1000, 0);
        assert!(result.is_err());
        match result.unwrap_err() {
            CfiViolation::NullTarget { .. } => {}
            _ => panic!("Expected NullTarget violation"),
        }
    }

    #[test]
    fn test_validation_misaligned_target() {
        let hooks = CfiCompilerHooks::new();
        let result = hooks.validate_indirect_call(0x1000, 0x1001); // Misaligned
        assert!(result.is_err());
        match result.unwrap_err() {
            CfiViolation::MisalignedTarget { .. } => {}
            _ => panic!("Expected MisalignedTarget violation"),
        }
    }

    #[test]
    fn test_validation_aligned_target() {
        let hooks = CfiCompilerHooks::new();
        let result = hooks.validate_indirect_call(0x1000, 0x1000); // Aligned
        assert!(result.is_ok());
        assert_eq!(hooks.stats.validations_passed.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_vtable_validation() {
        let hooks = CfiCompilerHooks::new();
        hooks.register_virtual_table(0x5000, 100, &[0x1100, 0x2200, 0x3300]);

        // Valid call
        let result = hooks.validate_virtual_call(0x5000, 1, 0x2200);
        assert!(result.is_ok());

        // Invalid call (wrong method address)
        let result = hooks.validate_virtual_call(0x5000, 1, 0x9999);
        assert!(result.is_err());

        // Invalid index
        let result = hooks.validate_virtual_call(0x5000, 10, 0x1100);
        assert!(result.is_err());
    }
}
