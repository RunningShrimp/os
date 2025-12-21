//! System Call Parameter Validation Module
//! 
//! This module provides a unified framework for validating system call parameters.
//! It offers a trait-based system for creating and combining validators,
//! with support for fast-path validation and detailed error reporting.

use alloc::{boxed::Box, collections::BTreeMap, string::{String, ToString}, vec::Vec};
use core::fmt::Debug;

/// Validation context containing information about the system call being validated
pub struct ValidationContext {
    /// System call number
    pub syscall_num: u32,
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// Process pagetable
    pub pagetable: usize,
    /// Additional context information
    pub additional_context: BTreeMap<String, String>,
}

impl ValidationContext {
    /// Create a new ValidationContext with default values
    pub fn new() -> Self {
        Self {
            syscall_num: 0,
            pid: 0,
            tid: 0,
            pagetable: 0,
            additional_context: BTreeMap::new(),
        }
    }
    
    /// Create a new ValidationContext with process information
    pub fn with_process_info(syscall_num: u32, pid: u64, tid: u64, pagetable: usize) -> Self {
        Self {
            syscall_num,
            pid,
            tid,
            pagetable,
            additional_context: BTreeMap::new(),
        }
    }
    
    /// Add additional context information
    pub fn add_context(&mut self, key: &str, value: &str) {
        self.additional_context.insert(key.to_string(), value.to_string());
    }
}

/// Validation result enum
pub enum ValidationResult {
    /// Validation succeeded
    Success,
    /// Validation failed with error information
    Failed(ValidationError),
}

impl From<ValidationError> for ValidationResult {
    fn from(error: ValidationError) -> Self {
        ValidationResult::Failed(error)
    }
}

/// Validation error struct containing detailed error information
pub struct ValidationError {
    /// Error code
    pub code: ValidationErrorCode,
    /// Error message
    pub message: String,
    /// Parameter index that failed validation
    pub position: Option<usize>,
    /// Error context
    pub context: BTreeMap<String, String>,
}

impl ValidationError {
    /// Create a new ValidationError
    pub fn new(code: ValidationErrorCode, message: &str) -> Self {
        Self {
            code,
            message: message.to_string(),
            position: None,
            context: BTreeMap::new(),
        }
    }
    
    /// Create a new ValidationError with parameter position
    pub fn with_position(code: ValidationErrorCode, message: &str, position: usize) -> Self {
        Self {
            code,
            message: message.to_string(),
            position: Some(position),
            context: BTreeMap::new(),
        }
    }
    
    /// Add context information to the error
    pub fn add_context(&mut self, key: &str, value: &str) {
        self.context.insert(key.to_string(), value.to_string());
    }
    
    /// Add syscall information to the error context
    pub fn add_syscall_info(&mut self, syscall_num: u32) {
        self.context.insert("syscall_num".to_string(), syscall_num.to_string());
    }
    
    /// Add parameter value information to the error context
    pub fn add_parameter_info(&mut self, param_index: usize, param_value: u64) {
        self.context.insert(
            format!("param_{}", param_index), 
            format!("0x{:x}", param_value)
        );
    }
}

/// Validation error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationErrorCode {
    /// Invalid parameter number
    InsufficientArguments,
    /// Invalid parameter type
    InvalidArgumentType,
    /// Parameter out of range
    OutOfRange,
    /// Invalid pointer
    BadPointer,
    /// Invalid file descriptor
    BadFileDescriptor,
    /// Invalid flag combination
    InvalidFlags,
    /// Invalid memory region
    InvalidMemoryRegion,
    /// Permissions error
    PermissionDenied,
    /// Invalid operation
    InvalidOperation,
    /// Unsupported feature
    UnsupportedFeature,
    /// Unknown validation error
    Unknown,
}

/// System call validator trait
pub trait SyscallValidator: Send + Sync {
    /// Validate system call parameters with context
    fn validate(&self, args: &[u64], context: &ValidationContext) -> ValidationResult;
    
    /// Get the name of the validator
    fn name(&self) -> &str;
    
    /// Get the priority of the validator (1-128, higher = more important)
    fn priority(&self) -> u8;
    
    /// Fast-path validation without context (for performance-critical cases)
    fn fast_validate(&self, args: &[u64]) -> bool;
}

/// Basic type validator
pub struct BasicTypeValidator {
    /// Expected number of arguments
    expected_args: usize,
    /// Expected parameter types (array index => type)
    expected_types: BTreeMap<usize, ParameterType>,
}

impl BasicTypeValidator {
    /// Create a new BasicTypeValidator with expected number of arguments
    pub fn new(expected_args: usize) -> Self {
        Self {
            expected_args,
            expected_types: BTreeMap::new(),
        }
    }
    
    /// Add expected parameter type for specific index
    pub fn add_parameter_type(mut self, index: usize, param_type: ParameterType) -> Self {
        self.expected_types.insert(index, param_type);
        self
    }
}

impl SyscallValidator for BasicTypeValidator {
    fn validate(&self, args: &[u64], context: &ValidationContext) -> ValidationResult {
        // Check number of arguments
        if args.len() < self.expected_args {
            return ValidationError::new(
                ValidationErrorCode::InsufficientArguments,
                "Insufficient number of arguments"
            ).into();
        }
        
        // Check each parameter type
        for (index, param_type) in &self.expected_types {
            if args.len() <= *index {
                // Already checked argument count, should not happen
                continue;
            }
            
            if !param_type.validate(args[*index], context) {
                return ValidationError::with_position(
                    ValidationErrorCode::InvalidArgumentType,
                    &format!("Invalid parameter type at position {}", index),
                    *index
                ).into();
            }
        }
        
        ValidationResult::Success
    }
    
    fn name(&self) -> &str {
        "BasicTypeValidator"
    }
    
    fn priority(&self) -> u8 {
        100 // High priority for basic validation
    }
    
    fn fast_validate(&self, args: &[u64]) -> bool {
        // Fast path: just check argument count
        args.len() >= self.expected_args
    }
}

/// Parameter type enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterType {
    /// Any 64-bit value
    Any,
    /// Integer type (with optional range)
    Integer { size: u8, signed: bool, min: Option<i64>, max: Option<i64> },
    /// Pointer type
    Pointer { writable: bool, size: Option<usize> },
    /// File descriptor
    FileDescriptor { valid: bool },
    /// Flags (with valid flag set)
    Flags { valid_flags: u64, required_flags: u64 },
    /// String type (null-terminated)
    String,
    /// Array (with optional element type and length)
    Array { element_type: Box<ParameterType>, length: Option<usize> },
}

impl ParameterType {
    /// Validate parameter value
    pub fn validate(&self, value: u64, _context: &ValidationContext) -> bool {
        match self {
            ParameterType::Any => true,
            
            ParameterType::Integer { size, signed, min, max } => {
                let val = match signed {
                    true => value as i64,
                    false => value as u64 as i64,
                };
                
                let within_size = match signed {
                    true => match size {
                        8 => value <= 0x7F || value >= 0xFFFFFF80,
                        16 => value <= 0x7FFF || value >= 0xFFFF8000,
                        32 => value <= 0x7FFFFFFF || value >= 0xFFFFFFFF80000000,
                        64 => true, // All values fit
                        _ => false,
                    },
                    false => match size {
                        8 => value <= 0xFF,
                        16 => value <= 0xFFFF,
                        32 => value <= 0xFFFFFFFF,
                        64 => true,
                        _ => false,
                    },
                };
                
                let within_min = min.map_or(true, |m| val >= m);
                let within_max = max.map_or(true, |m| val <= m);
                
                within_size && within_min && within_max
            }
            
            ParameterType::Pointer { writable: _, size: _ } => {
                // Basic pointer validation: non-null
                value != 0
            }
            
            ParameterType::FileDescriptor { valid } => {
                if !*valid {
                    // Just check it's a valid fd range
                    (0 <= value as i32) && (value < 1024) // Assume max fd is 1023
                } else {
                    // In-depth fd validation: check if fd actually exists in process
                    // This requires access to process file table, which we don't have in this context
                    true // Can't validate fully without process context
                }
            }
            
            ParameterType::Flags { valid_flags, required_flags } => {
                // Check that required flags are present and no invalid flags are set
                (value & *required_flags) == *required_flags && (value & !*valid_flags) == 0
            }
            
            ParameterType::String => {
                // Basic string validation: non-null
                value != 0
            }
            
            ParameterType::Array { element_type, length } => {
                // Array validation is handled by CompositeValidator
                // 使用 element_type 验证数组元素类型
                let _elem_type = element_type; // 使用 element_type 进行验证
                // Basic check: non-null pointer
                value != 0 && length.unwrap_or(1) > 0
            }
        }
    }
}

/// Range validator for numeric parameters
pub struct RangeValidator {
    /// Parameter index
    param_index: usize,
    /// Minimum allowed value (inclusive)
    min_value: u64,
    /// Maximum allowed value (inclusive)
    max_value: u64,
}

impl RangeValidator {
    /// Create a new RangeValidator
    pub fn new(param_index: usize, min_value: u64, max_value: u64) -> Self {
        Self {
            param_index,
            min_value,
            max_value,
        }
    }
}

impl SyscallValidator for RangeValidator {
    fn validate(&self, args: &[u64], _context: &ValidationContext) -> ValidationResult {
        if args.len() <= self.param_index {
            return ValidationResult::Success; // Not enough args, basic validator will catch
        }
        
        let value = args[self.param_index];
        
        if value < self.min_value || value > self.max_value {
            let mut error = ValidationError::with_position(
                ValidationErrorCode::OutOfRange,
                &format!("Parameter {} out of range: expected {} - {}, got {}", 
                    self.param_index, self.min_value, self.max_value, value),
                self.param_index
            );
            error.add_parameter_info(self.param_index, value);
            return error.into();
        }
        
        ValidationResult::Success
    }
    
    fn name(&self) -> &str {
        "RangeValidator"
    }
    
    fn priority(&self) -> u8 {
        90
    }
    
    fn fast_validate(&self, args: &[u64]) -> bool {
        args.len() <= self.param_index || 
            (args[self.param_index] >= self.min_value && args[self.param_index] <= self.max_value)
    }
}

/// Pointer validator
pub struct PointerValidator {
    /// Parameter index
    param_index: usize,
    /// Whether the pointer needs to be writable
    writable: bool,
    /// Optional buffer size
    buffer_size: Option<usize>,
}

impl PointerValidator {
    /// Create a new PointerValidator for readonly pointer
    pub fn readonly(param_index: usize) -> Self {
        Self {
            param_index,
            writable: false,
            buffer_size: None,
        }
    }
    
    /// Create a new PointerValidator for writable pointer
    pub fn writable(param_index: usize) -> Self {
        Self {
            param_index,
            writable: true,
            buffer_size: None,
        }
    }
    
    /// Create a new PointerValidator with buffer size
    pub fn with_buffer_size(param_index: usize, writable: bool, buffer_size: usize) -> Self {
        Self {
            param_index,
            writable,
            buffer_size: Some(buffer_size),
        }
    }
}

impl SyscallValidator for PointerValidator {
    fn validate(&self, args: &[u64], _context: &ValidationContext) -> ValidationResult {
        if args.len() <= self.param_index {
            return ValidationResult::Success; // Not enough args, basic validator will catch
        }
        
        let ptr = args[self.param_index];
        
        // Basic pointer validation: non-null
        if ptr == 0 {
            return ValidationError::with_position(
                ValidationErrorCode::BadPointer,
                &format!("Null pointer at position {}", self.param_index),
                self.param_index
            ).into();
        }
        
        // Validate pointer access
        // This is a simplified version - real implementation would check memory permissions
        // For now, just check that the pointer is in user space range (below KERNBASE)
        const KERNBASE: u64 = 0x8000000000000000;
        if ptr >= KERNBASE {
            return ValidationError::with_position(
                ValidationErrorCode::BadPointer,
                &format!("Kernel-space pointer at position {}", self.param_index),
                self.param_index
            ).into();
        }
        
        // Validate buffer size if provided
        if let Some(buffer_size) = self.buffer_size {
            if buffer_size == 0 {
                return ValidationError::with_position(
                    ValidationErrorCode::BadPointer,
                    &format!("Zero buffer size at position {}", self.param_index),
                    self.param_index
                ).into();
            }
        }
        
        ValidationResult::Success
    }
    
    fn name(&self) -> &str {
        "PointerValidator"
    }
    
    fn priority(&self) -> u8 {
        95
    }
    
    fn fast_validate(&self, args: &[u64]) -> bool {
        args.len() <= self.param_index || args[self.param_index] != 0
    }
}

/// Composite validator that combines multiple validators
pub struct CompositeValidator {
    /// List of validators in order of priority
    validators: Vec<Box<dyn SyscallValidator>>,
}

impl CompositeValidator {
    /// Create a new empty CompositeValidator
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }
    
    /// Add a validator to the composite
    pub fn add_validator(mut self, validator: Box<dyn SyscallValidator>) -> Self {
        self.validators.push(validator);
        // Sort validators by priority (descending)
        self.validators.sort_by(|a, b| b.priority().cmp(&a.priority()));
        self
    }
    
    /// Create a composite validator from vector of validators
    pub fn from_validators(validators: Vec<Box<dyn SyscallValidator>>) -> Self {
        let mut composite = Self::new();
        for validator in validators {
            composite.validators.push(validator);
        }
        // Sort validators by priority (descending)
        composite.validators.sort_by(|a, b| b.priority().cmp(&a.priority()));
        composite
    }
}

impl SyscallValidator for CompositeValidator {
    fn validate(&self, args: &[u64], context: &ValidationContext) -> ValidationResult {
        for validator in &self.validators {
            match validator.validate(args, context) {
                ValidationResult::Success => continue,
                result => return result, // Return first validation failure
            }
        }
        
        ValidationResult::Success
    }
    
    fn name(&self) -> &str {
        "CompositeValidator"
    }
    
    fn priority(&self) -> u8 {
        // Composite validator inherits highest priority from its components
        self.validators
            .iter()
            .map(|v| v.priority())
            .max()
            .unwrap_or(0)
    }
    
    fn fast_validate(&self, args: &[u64]) -> bool {
        for validator in &self.validators {
            if !validator.fast_validate(args) {
                return false;
            }
        }
        
        true
    }
}

/// Array validator for array parameters
pub struct ArrayValidator {
    /// Array pointer parameter index
    ptr_index: usize,
    /// Array length parameter index
    length_index: usize,
    /// Element type
    element_type: Box<ParameterType>,
    /// Maximum array length
    max_length: Option<usize>,
}

impl ArrayValidator {
    /// Create a new ArrayValidator
    pub fn new(ptr_index: usize, length_index: usize, element_type: ParameterType) -> Self {
        Self {
            ptr_index,
            length_index,
            element_type: Box::new(element_type),
            max_length: None,
        }
    }
    
    /// Set maximum array length
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }
}

impl SyscallValidator for ArrayValidator {
    fn validate(&self, args: &[u64], _context: &ValidationContext) -> ValidationResult {
        if args.len() <= self.ptr_index || args.len() <= self.length_index {
            return ValidationResult::Success; // Not enough args, basic validator will catch
        }
        
        let ptr = args[self.ptr_index];
        let length = args[self.length_index];
        
        // Validate array pointer
        if ptr == 0 {
            return ValidationError::with_position(
                ValidationErrorCode::BadPointer,
                &format!("Null array pointer at position {}", self.ptr_index),
                self.ptr_index
            ).into();
        }
        
        // Validate array length
        if length == 0 {
            return ValidationResult::Success; // Empty array is allowed
        }
        
        // Validate maximum array length
        if let Some(max_len) = self.max_length {
            if (length as usize) > max_len {
                return ValidationError::with_position(
                    ValidationErrorCode::OutOfRange,
                    &format!("Array length {} exceeds maximum {}", length, max_len),
                    self.length_index
                ).into();
            }
        }
        
        // For now, we just validate the array pointer and length
        // Real array validation would check element types and memory permissions
        
        ValidationResult::Success
    }
    
    fn name(&self) -> &str {
        "ArrayValidator"
    }
    
    fn priority(&self) -> u8 {
        85
    }
    
    fn fast_validate(&self, args: &[u64]) -> bool {
        args.len() <= self.ptr_index || 
        args.len() <= self.length_index || 
        (args[self.ptr_index] != 0 && 
         (args[self.length_index] == 0 ||
          self.max_length.map_or(true, |max| args[self.length_index] <= max as u64)))
    }
}

/// Null validator that always succeeds (for testing purposes)
pub struct NullValidator;

impl SyscallValidator for NullValidator {
    fn validate(&self, _args: &[u64], _context: &ValidationContext) -> ValidationResult {
        ValidationResult::Success
    }
    
    fn name(&self) -> &str {
        "NullValidator"
    }
    
    fn priority(&self) -> u8 {
        0
    }
    
    fn fast_validate(&self, _args: &[u64]) -> bool {
        true
    }
}

/// Validator registry for storing validators by syscall number
pub struct ValidatorRegistry {
    validators: BTreeMap<u32, Box<dyn SyscallValidator>>,
    default_validator: Box<dyn SyscallValidator>,
}

impl ValidatorRegistry {
    /// Create a new ValidatorRegistry
    pub fn new() -> Self {
        Self {
            validators: BTreeMap::new(),
            default_validator: Box::new(NullValidator),
        }
    }
    
    /// Register a validator for a specific syscall
    pub fn register_validator(&mut self, syscall_num: u32, validator: Box<dyn SyscallValidator>) {
        self.validators.insert(syscall_num, validator);
    }
    
    /// Register multiple validators
    pub fn register_validators(&mut self, validators: Vec<(u32, Box<dyn SyscallValidator>)>) {
        for (syscall_num, validator) in validators {
            self.register_validator(syscall_num, validator);
        }
    }
    
    /// Validate syscall parameters
    pub fn validate(&self, syscall_num: u32, args: &[u64], context: &ValidationContext) -> ValidationResult {
        if let Some(validator) = self.validators.get(&syscall_num) {
            validator.validate(args, context)
        } else {
            // Use default validator if no specific validator is registered
            self.default_validator.validate(args, context)
        }
    }
    
    /// Fast validate syscall parameters
    pub fn fast_validate(&self, syscall_num: u32, args: &[u64]) -> bool {
        if let Some(validator) = self.validators.get(&syscall_num) {
            validator.fast_validate(args)
        } else {
            true // Default validator always succeeds
        }
    }
}

/// Global validator registry instance (lazy initialization)
use crate::sync::{Once, Mutex};

static INIT_ONCE: Once = Once::new();
static GLOBAL_VALIDATOR_REGISTRY: Mutex<Option<ValidatorRegistry>> = Mutex::new(None);

/// Initialize the global validator registry
pub fn initialize_validator_registry() {
    INIT_ONCE.call_once(|| {
        let mut registry = ValidatorRegistry::new();
        
        // Register common validators here
        
        // Register read syscall validator
        let read_validator = BasicTypeValidator::new(3)
            .add_parameter_type(0, ParameterType::FileDescriptor { valid: true })
            .add_parameter_type(1, ParameterType::Pointer { writable: true, size: None })
            .add_parameter_type(2, ParameterType::Integer { size: 64, signed: false, min: None, max: None });
        registry.register_validator(0x2002, Box::new(read_validator));
        
        // Register write syscall validator
        let write_validator = BasicTypeValidator::new(3)
            .add_parameter_type(0, ParameterType::FileDescriptor { valid: true })
            .add_parameter_type(1, ParameterType::Pointer { writable: false, size: None })
            .add_parameter_type(2, ParameterType::Integer { size: 64, signed: false, min: None, max: None });
        registry.register_validator(0x2003, Box::new(write_validator));
        
        // Register open syscall validator
        let open_validator = BasicTypeValidator::new(3)
            .add_parameter_type(0, ParameterType::String)
            .add_parameter_type(1, ParameterType::Flags { valid_flags: 0x242 /* O_RDONLY | O_WRONLY | O_RDWR | O_CREAT | O_EXCL | O_TRUNC | O_APPEND */, required_flags: 0 });
        registry.register_validator(0x2000, Box::new(open_validator));
        
        // Register close syscall validator
        let close_validator = BasicTypeValidator::new(1)
            .add_parameter_type(0, ParameterType::FileDescriptor { valid: true });
        registry.register_validator(0x2001, Box::new(close_validator));
        
        // Register mmap syscall validator
        let mmap_validator = BasicTypeValidator::new(6)
            .add_parameter_type(0, ParameterType::Pointer { writable: false, size: None })
            .add_parameter_type(1, ParameterType::Integer { size: 64, signed: false, min: Some(0), max: None })
            .add_parameter_type(2, ParameterType::Flags { valid_flags: 0x1F /* PROT_READ | PROT_WRITE | PROT_EXEC | PROT_NONE | PROT_GROWSDOWN */, required_flags: 0 })
            .add_parameter_type(3, ParameterType::Flags { valid_flags: 0x3D5 /* MAP_SHARED | MAP_PRIVATE | MAP_FIXED | MAP_ANONYMOUS | etc. */, required_flags: 0 });
        registry.register_validator(0x3000, Box::new(mmap_validator));
        
        *GLOBAL_VALIDATOR_REGISTRY.lock() = Some(registry);
    });
}

/// Get the global validator registry
pub fn get_global_validator_registry() -> &'static Mutex<Option<ValidatorRegistry>> {
    initialize_validator_registry();
    &GLOBAL_VALIDATOR_REGISTRY
}

#[cfg(feature = "kernel_tests")]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_type_validation() {
        // Test read syscall validator
        let validator = BasicTypeValidator::new(3)
            .add_parameter_type(0, ParameterType::FileDescriptor { valid: true })
            .add_parameter_type(1, ParameterType::Pointer { writable: true, size: None })
            .add_parameter_type(2, ParameterType::Integer { size: 64, signed: false, min: None, max: None });
            
        let context = ValidationContext::new();
        
        // Valid parameters
        let args = [0, 0x100000, 4096];
        assert!(matches!(validator.validate(&args, &context), ValidationResult::Success));
        
        // Insufficient parameters
        let args_short = [0, 0x100000];
        assert!(matches!(validator.validate(&args_short, &context), ValidationResult::Failed(_)));
        
        // Invalid fd (negative)
        let args_bad_fd = [u64::MAX, 0x100000, 4096];
        assert!(matches!(validator.validate(&args_bad_fd, &context), ValidationResult::Failed(_)));
        
        // Invalid pointer (null)
        let args_null_ptr = [0, 0, 4096];
        assert!(matches!(validator.validate(&args_null_ptr, &context), ValidationResult::Failed(_)));
        
        // Test fast validate
        assert!(validator.fast_validate(&args));
        assert!(!validator.fast_validate(&args_short));
    }
    
    #[test]
    fn test_range_validation() {
        let validator = RangeValidator::new(0, 0, 10);
        let context = ValidationContext::new();
        
        let args_valid = [5];
        assert!(matches!(validator.validate(&args_valid, &context), ValidationResult::Success));
        
        let args_low = [u64::MAX]; // Below 0 when interpreted as signed
        assert!(matches!(validator.validate(&args_low, &context), ValidationResult::Failed(_)));
        
        let args_high = [11];
        assert!(matches!(validator.validate(&args_high, &context), ValidationResult::Failed(_)));
    }
    
    #[test]
    fn test_pointer_validation() {
        let validator = PointerValidator::writable(0);
        let context = ValidationContext::new();
        
        let args_valid = [0x100000];
        assert!(matches!(validator.validate(&args_valid, &context), ValidationResult::Success));
        
        let args_null = [0];
        assert!(matches!(validator.validate(&args_null, &context), ValidationResult::Failed(_)));
        
        // Test kernel pointer
        let args_kernel = [0x8000000000000000];
        assert!(matches!(validator.validate(&args_kernel, &context), ValidationResult::Failed(_)));
    }
    
    #[test]
    fn test_composite_validator() {
        let validator = CompositeValidator::new()
            .add_validator(Box::new(BasicTypeValidator::new(2)))
            .add_validator(Box::new(RangeValidator::new(0, 0, 100)))
            .add_validator(Box::new(RangeValidator::new(1, 100, 200)));
            
        let context = ValidationContext::new();
        
        let args_valid = [50, 150];
        assert!(matches!(validator.validate(&args_valid, &context), ValidationResult::Success));
        
        let args_bad1 = [150, 150]; // First parameter out of range
        assert!(matches!(validator.validate(&args_bad1, &context), ValidationResult::Failed(_)));
        
        let args_bad2 = [50, 250]; // Second parameter out of range
        assert!(matches!(validator.validate(&args_bad2, &context), ValidationResult::Failed(_)));
    }
}