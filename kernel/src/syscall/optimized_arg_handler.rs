//! Optimized System Call Argument Handling
//!
//! This module provides optimized system call argument handling with:
//! - Fast argument validation
//! - Argument caching
//! - Batch argument processing
//! - Zero-copy argument handling
//! - Type-safe argument parsing

use core::ptr::{null_mut, NonNull};
use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use crate::subsystems::sync::SpinLock;
use nos_api::{Result, Error};

/// Maximum number of arguments for a system call
pub const MAX_SYSCALL_ARGS: usize = 6;

/// Argument validation result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgValidationResult {
    Valid,
    InvalidPointer,
    InvalidSize,
    InvalidRange,
    PermissionDenied,
    TooManyArgs,
}

/// Argument type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    U8,
    U16,
    U32,
    U64,
    USize,
    I8,
    I16,
    I32,
    I64,
    ISize,
    Pointer,
    Buffer { size: usize },
    String { max_len: usize },
    Array { element_type: ArgType, count: usize },
    Struct { size: usize },
}

/// Argument descriptor
#[derive(Debug, Clone)]
pub struct ArgDescriptor {
    /// Argument type
    pub arg_type: ArgType,
    /// Whether argument is input (true) or output (false)
    pub is_input: bool,
    /// Whether argument is optional
    pub is_optional: bool,
    /// Minimum value (for numeric types)
    pub min_value: Option<isize>,
    /// Maximum value (for numeric types)
    pub max_value: Option<isize>,
    /// Custom validation function
    pub validator: Option<fn(usize) -> ArgValidationResult>,
}

impl ArgDescriptor {
    /// Create a new argument descriptor
    pub fn new(arg_type: ArgType) -> Self {
        Self {
            arg_type,
            is_input: true,
            is_optional: false,
            min_value: None,
            max_value: None,
            validator: None,
        }
    }
    
    /// Set as input argument
    pub fn input(mut self) -> Self {
        self.is_input = true;
        self
    }
    
    /// Set as output argument
    pub fn output(mut self) -> Self {
        self.is_input = false;
        self
    }
    
    /// Set as optional argument
    pub fn optional(mut self) -> Self {
        self.is_optional = true;
        self
    }
    
    /// Set minimum value
    pub fn min(mut self, value: isize) -> Self {
        self.min_value = Some(value);
        self
    }
    
    /// Set maximum value
    pub fn max(mut self, value: isize) -> Self {
        self.max_value = Some(value);
        self
    }
    
    /// Set custom validator
    pub fn validate_with(mut self, validator: fn(usize) -> ArgValidationResult) -> Self {
        self.validator = Some(validator);
        self
    }
}

/// System call argument descriptor
#[derive(Debug, Clone)]
pub struct SyscallArgDescriptor {
    /// System call number
    pub syscall_num: u32,
    /// Argument descriptors
    pub args: Vec<ArgDescriptor>,
    /// Fast-path handler
    pub fast_path_handler: Option<fn(&[usize]) -> isize>,
}

/// Argument validation cache
#[derive(Debug)]
pub struct ArgValidationCache {
    /// Cache entries
    entries: BTreeMap<usize, ArgValidationResult>,
    /// Cache hit count
    hits: AtomicUsize,
    /// Cache miss count
    misses: AtomicUsize,
    /// Maximum cache size
    max_size: usize,
}

impl ArgValidationCache {
    /// Create a new argument validation cache
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
            max_size,
        }
    }
    
    /// Get validation result from cache
    pub fn get(&self, addr: usize) -> Option<ArgValidationResult> {
        if let Some(&result) = self.entries.get(&addr) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(result)
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
    
    /// Put validation result in cache
    pub fn put(&mut self, addr: usize, result: ArgValidationResult) {
        // If cache is full, remove oldest entries
        if self.entries.len() >= self.max_size {
            // Simple eviction: remove first half of entries
            let keys: Vec<usize> = self.entries.keys().take(self.max_size / 2).cloned().collect();
            for key in keys {
                self.entries.remove(&key);
            }
        }
        
        self.entries.insert(addr, result);
    }
    
    /// Get cache hit ratio
    pub fn hit_ratio(&self) -> f32 {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        
        if hits + misses == 0 {
            return 0.0;
        }
        
        hits as f32 / (hits + misses) as f32
    }
    
    /// Clear cache
    pub fn clear(&mut self) {
        self.entries.clear();
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
    }
}

/// Optimized argument handler
pub struct OptimizedArgHandler {
    /// System call descriptors
    syscall_descriptors: BTreeMap<u32, SyscallArgDescriptor>,
    /// Argument validation cache
    validation_cache: SpinLock<ArgValidationCache>,
    /// Argument processing statistics
    stats: ArgHandlerStats,
}

/// Argument handler statistics
#[derive(Debug, Default)]
pub struct ArgHandlerStats {
    /// Total arguments processed
    pub total_args_processed: AtomicUsize,
    /// Valid arguments
    pub valid_args: AtomicUsize,
    /// Invalid arguments
    pub invalid_args: AtomicUsize,
    /// Cache hits
    pub cache_hits: AtomicUsize,
    /// Cache misses
    pub cache_misses: AtomicUsize,
    /// Fast-path hits
    pub fast_path_hits: AtomicUsize,
    /// Slow-path calls
    pub slow_path_calls: AtomicUsize,
}

impl OptimizedArgHandler {
    /// Create a new optimized argument handler
    pub fn new() -> Self {
        Self {
            syscall_descriptors: BTreeMap::new(),
            validation_cache: Spin_lock!(ArgValidationCache::new(1024)),
            stats: ArgHandlerStats::default(),
        }
    }
    
    /// Register a system call with its argument descriptors
    pub fn register_syscall(&mut self, descriptor: SyscallArgDescriptor) {
        self.syscall_descriptors.insert(descriptor.syscall_num, descriptor);
    }
    
    /// Validate a single argument
    pub fn validate_arg(&self, arg: usize, descriptor: &ArgDescriptor) -> ArgValidationResult {
        // Check cache first
        let cache = self.validation_cache.lock();
        if let Some(result) = cache.get(arg) {
            self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            return result;
        }
        
        self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
        
        // Perform validation
        let result = self.validate_arg_internal(arg, descriptor);
        
        // Cache the result
        drop(cache); // Release lock before caching
        self.validation_cache.lock().put(arg, result);
        
        result
    }
    
    /// Internal argument validation
    fn validate_arg_internal(&self, arg: usize, descriptor: &ArgDescriptor) -> ArgValidationResult {
        // Check custom validator first
        if let Some(validator) = descriptor.validator {
            let result = validator(arg);
            if result != ArgValidationResult::Valid {
                return result;
            }
        }
        
        // Type-specific validation
        match descriptor.arg_type {
            ArgType::U8 => {
                if arg > 0xFF {
                    ArgValidationResult::InvalidRange
                } else {
                    ArgValidationResult::Valid
                }
            }
            ArgType::U16 => {
                if arg > 0xFFFF {
                    ArgValidationResult::InvalidRange
                } else {
                    ArgValidationResult::Valid
                }
            }
            ArgType::U32 => {
                if arg > 0xFFFFFFFF {
                    ArgValidationResult::InvalidRange
                } else {
                    ArgValidationResult::Valid
                }
            }
            ArgType::U64 | ArgType::USize => ArgValidationResult::Valid,
            
            ArgType::I8 => {
                if arg > 0x7F {
                    ArgValidationResult::InvalidRange
                } else {
                    ArgValidationResult::Valid
                }
            }
            ArgType::I16 => {
                if arg > 0x7FFF {
                    ArgValidationResult::InvalidRange
                } else {
                    ArgValidationResult::Valid
                }
            }
            ArgType::I32 => {
                if arg > 0x7FFFFFFF {
                    ArgValidationResult::InvalidRange
                } else {
                    ArgValidationResult::Valid
                }
            }
            ArgType::I64 | ArgType::ISize => ArgValidationResult::Valid,
            
            ArgType::Pointer => {
                if arg == 0 {
                    if descriptor.is_optional {
                        ArgValidationResult::Valid
                    } else {
                        ArgValidationResult::InvalidPointer
                    }
                } else {
                    // Check if pointer is in valid range
                    // In a real implementation, this would check against valid memory ranges
                    if arg < 0x1000 || arg > 0xFFFFFFFFFFFFF000 {
                        ArgValidationResult::InvalidPointer
                    } else {
                        ArgValidationResult::Valid
                    }
                }
            }
            
            ArgType::Buffer { size } => {
                if arg == 0 {
                    if descriptor.is_optional {
                        ArgValidationResult::Valid
                    } else {
                        ArgValidationResult::InvalidPointer
                    }
                } else if arg < 0x1000 || arg > 0xFFFFFFFFFFFFF000 {
                    ArgValidationResult::InvalidPointer
                } else if arg + size > 0xFFFFFFFFFFFFF000 {
                    ArgValidationResult::InvalidRange
                } else {
                    ArgValidationResult::Valid
                }
            }
            
            ArgType::String { max_len } => {
                if arg == 0 {
                    if descriptor.is_optional {
                        ArgValidationResult::Valid
                    } else {
                        ArgValidationResult::InvalidPointer
                    }
                } else if arg < 0x1000 || arg > 0xFFFFFFFFFFFFF000 {
                    ArgValidationResult::InvalidPointer
                } else {
                    // In a real implementation, this would validate the string
                    ArgValidationResult::Valid
                }
            }
            
            ArgType::Array { element_type: _, count } => {
                if arg == 0 {
                    if descriptor.is_optional {
                        ArgValidationResult::Valid
                    } else {
                        ArgValidationResult::InvalidPointer
                    }
                } else if arg < 0x1000 || arg > 0xFFFFFFFFFFFFF000 {
                    ArgValidationResult::InvalidPointer
                } else {
                    ArgValidationResult::Valid
                }
            }
            
            ArgType::Struct { size } => {
                if arg == 0 {
                    if descriptor.is_optional {
                        ArgValidationResult::Valid
                    } else {
                        ArgValidationResult::InvalidPointer
                    }
                } else if arg < 0x1000 || arg > 0xFFFFFFFFFFFFF000 {
                    ArgValidationResult::InvalidPointer
                } else if arg + size > 0xFFFFFFFFFFFFF000 {
                    ArgValidationResult::InvalidRange
                } else {
                    ArgValidationResult::Valid
                }
            }
        }
    }
    
    /// Validate all arguments for a system call
    pub fn validate_syscall_args(&self, syscall_num: u32, args: &[usize]) -> Result<Vec<ArgValidationResult>> {
        if args.len() > MAX_SYSCALL_ARGS {
            self.stats.invalid_args.fetch_add(args.len(), Ordering::Relaxed);
            return Err(Error::InvalidArgument);
        }
        
        let descriptor = self.syscall_descriptors.get(&syscall_num)
            .ok_or(Error::NotFound)?;
        
        let mut results = Vec::with_capacity(args.len());
        
        for (i, &arg) in args.iter().enumerate() {
            let arg_desc = if i < descriptor.args.len() {
                &descriptor.args[i]
            } else {
                // Too many arguments
                self.stats.invalid_args.fetch_add(1, Ordering::Relaxed);
                return Err(Error::InvalidArgument);
            };
            
            let result = self.validate_arg(arg, arg_desc);
            results.push(result);
            
            if result != ArgValidationResult::Valid {
                self.stats.invalid_args.fetch_add(1, Ordering::Relaxed);
            } else {
                self.stats.valid_args.fetch_add(1, Ordering::Relaxed);
            }
            
            self.stats.total_args_processed.fetch_add(1, Ordering::Relaxed);
        }
        
        Ok(results)
    }
    
    /// Process system call arguments with fast-path
    pub fn process_syscall_args_fast(&self, syscall_num: u32, args: &[usize]) -> Result<isize> {
        let descriptor = self.syscall_descriptors.get(&syscall_num)
            .ok_or(Error::NotFound)?;
        
        // Check if we have a fast-path handler
        if let Some(handler) = descriptor.fast_path_handler {
            self.stats.fast_path_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(handler(args));
        }
        
        self.stats.slow_path_calls.fetch_add(1, Ordering::Relaxed);
        Err(Error::NotImplemented)
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> ArgHandlerStats {
        ArgHandlerStats {
            total_args_processed: AtomicUsize::new(self.stats.total_args_processed.load(Ordering::Relaxed)),
            valid_args: AtomicUsize::new(self.stats.valid_args.load(Ordering::Relaxed)),
            invalid_args: AtomicUsize::new(self.stats.invalid_args.load(Ordering::Relaxed)),
            cache_hits: AtomicUsize::new(self.stats.cache_hits.load(Ordering::Relaxed)),
            cache_misses: AtomicUsize::new(self.stats.cache_misses.load(Ordering::Relaxed)),
            fast_path_hits: AtomicUsize::new(self.stats.fast_path_hits.load(Ordering::Relaxed)),
            slow_path_calls: AtomicUsize::new(self.stats.slow_path_calls.load(Ordering::Relaxed)),
        }
    }
    
    /// Clear validation cache
    pub fn clear_cache(&self) {
        self.validation_cache.lock().clear();
    }
    
    /// Get cache hit ratio
    pub fn cache_hit_ratio(&self) -> f32 {
        self.validation_cache.lock().hit_ratio()
    }
}

/// Zero-copy argument buffer
#[derive(Debug)]
pub struct ZeroCopyArgBuffer {
    /// Buffer address
    pub addr: usize,
    /// Buffer size
    pub size: usize,
    /// Whether buffer is read-only
    pub read_only: bool,
    /// Reference count
    pub ref_count: AtomicUsize,
}

impl ZeroCopyArgBuffer {
    /// Create a new zero-copy argument buffer
    pub fn new(addr: usize, size: usize, read_only: bool) -> Self {
        Self {
            addr,
            size,
            read_only,
            ref_count: AtomicUsize::new(1),
        }
    }
    
    /// Get a pointer to the buffer
    pub fn as_ptr(&self) -> *mut u8 {
        self.addr as *mut u8
    }
    
    /// Get a slice of the buffer
    pub unsafe fn as_slice(&self) -> &[u8] {
        core::slice::from_raw_parts(self.addr as *const u8, self.size)
    }
    
    /// Get a mutable slice of the buffer
    pub unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        if self.read_only {
            panic!("Attempted to get mutable slice of read-only buffer");
        }
        core::slice::from_raw_parts_mut(self.addr as *mut u8, self.size)
    }
    
    /// Increment reference count
    pub fn inc_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Decrement reference count
    pub fn dec_ref(&self) -> bool {
        let old_count = self.ref_count.fetch_sub(1, Ordering::Relaxed);
        old_count > 1
    }
    
    /// Get reference count
    pub fn ref_count(&self) -> usize {
        self.ref_count.load(Ordering::Relaxed)
    }
}

/// Batch argument processor
#[derive(Debug)]
pub struct BatchArgProcessor {
    /// System calls to process
    syscalls: Vec<(u32, Vec<usize>)>,
    /// Results
    results: Vec<Result<isize>>,
    /// Validation results
    validation_results: Vec<Vec<ArgValidationResult>>,
}

impl BatchArgProcessor {
    /// Create a new batch argument processor
    pub fn new() -> Self {
        Self {
            syscalls: Vec::new(),
            results: Vec::new(),
            validation_results: Vec::new(),
        }
    }
    
    /// Add a system call to the batch
    pub fn add_syscall(&mut self, syscall_num: u32, args: Vec<usize>) {
        self.syscalls.push((syscall_num, args));
    }
    
    /// Process all system calls in the batch
    pub fn process(&mut self, arg_handler: &OptimizedArgHandler) {
        for (syscall_num, args) in &self.syscalls {
            // Validate arguments
            let validation_result = arg_handler.validate_syscall_args(*syscall_num, args);
            
            match validation_result {
                Ok(results) => {
                    self.validation_results.push(results);
                    
                    // Check if all arguments are valid
                    if results.iter().all(|&r| r == ArgValidationResult::Valid) {
                        // Try fast-path processing
                        match arg_handler.process_syscall_args_fast(*syscall_num, args) {
                            Ok(result) => {
                                self.results.push(Ok(result));
                            }
                            Err(_) => {
                                // Fast-path failed, would use regular syscall
                                self.results.push(Err(Error::NotImplemented));
                            }
                        }
                    } else {
                        // Invalid arguments
                        self.results.push(Err(Error::InvalidArgument));
                    }
                }
                Err(e) => {
                    self.validation_results.push(Vec::new());
                    self.results.push(Err(e));
                }
            }
        }
    }
    
    /// Get results
    pub fn get_results(&self) -> &[Result<isize>] {
        &self.results
    }
    
    /// Get validation results
    pub fn get_validation_results(&self) -> &[Vec<ArgValidationResult>] {
        &self.validation_results
    }
    
    /// Clear the batch
    pub fn clear(&mut self) {
        self.syscalls.clear();
        self.results.clear();
        self.validation_results.clear();
    }
}

/// Initialize common system call argument descriptors
pub fn init_common_syscall_args(arg_handler: &mut OptimizedArgHandler) {
    // Read syscall
    arg_handler.register_syscall(SyscallArgDescriptor {
        syscall_num: 0x2000,
        args: vec![
            ArgDescriptor::new(ArgType::U32).input(), // fd
            ArgDescriptor::new(ArgType::Buffer { size: 0 }).input().optional(), // buf (size determined by count)
            ArgDescriptor::new(ArgType::USize).input(), // count
        ],
        fast_path_handler: None,
    });
    
    // Write syscall
    arg_handler.register_syscall(SyscallArgDescriptor {
        syscall_num: 0x2001,
        args: vec![
            ArgDescriptor::new(ArgType::U32).input(), // fd
            ArgDescriptor::new(ArgType::Buffer { size: 0 }).input().optional(), // buf (size determined by count)
            ArgDescriptor::new(ArgType::USize).input(), // count
        ],
        fast_path_handler: None,
    });
    
    // Open syscall
    arg_handler.register_syscall(SyscallArgDescriptor {
        syscall_num: 0x2002,
        args: vec![
            ArgDescriptor::new(ArgType::String { max_len: 256 }).input(), // pathname
            ArgDescriptor::new(ArgType::U32).input(), // flags
            ArgDescriptor::new(ArgType::U32).input().optional(), // mode
        ],
        fast_path_handler: None,
    });
    
    // Close syscall
    arg_handler.register_syscall(SyscallArgDescriptor {
        syscall_num: 0x2003,
        args: vec![
            ArgDescriptor::new(ArgType::U32).input(), // fd
        ],
        fast_path_handler: None,
    });
    
    // Mmap syscall
    arg_handler.register_syscall(SyscallArgDescriptor {
        syscall_num: 0x3000,
        args: vec![
            ArgDescriptor::new(ArgType::USize).input(), // addr
            ArgDescriptor::new(ArgType::USize).input(), // length
            ArgDescriptor::new(ArgType::U32).input(), // prot
            ArgDescriptor::new(ArgType::U32).input(), // flags
            ArgDescriptor::new(ArgType::U32).input(), // fd
            ArgDescriptor::new(ArgType::USize).input(), // offset
        ],
        fast_path_handler: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_arg_descriptor() {
        let desc = ArgDescriptor::new(ArgType::U32)
            .input()
            .min(0)
            .max(1000);
        
        assert!(matches!(desc.arg_type, ArgType::U32));
        assert!(desc.is_input);
        assert!(!desc.is_optional);
        assert_eq!(desc.min_value, Some(0));
        assert_eq!(desc.max_value, Some(1000));
    }
    
    #[test]
    fn test_arg_validation_cache() {
        let mut cache = ArgValidationCache::new(4);
        
        // Test empty cache
        assert_eq!(cache.get(0x1000), None);
        
        // Test cache miss
        cache.put(0x1000, ArgValidationResult::Valid);
        assert_eq!(cache.get(0x1000), Some(ArgValidationResult::Valid));
        
        // Test cache hit
        assert_eq!(cache.get(0x1000), Some(ArgValidationResult::Valid));
        
        // Test cache hit ratio
        assert!(cache.hit_ratio() > 0.0);
    }
    
    #[test]
    fn test_optimized_arg_handler() {
        let mut handler = OptimizedArgHandler::new();
        
        // Register a simple syscall
        handler.register_syscall(SyscallArgDescriptor {
            syscall_num: 0x1000,
            args: vec![
                ArgDescriptor::new(ArgType::U32).input(),
                ArgDescriptor::new(ArgType::U32).input(),
            ],
            fast_path_handler: None,
        });
        
        // Test argument validation
        let result = handler.validate_syscall_args(0x1000, &[100, 200]);
        assert!(result.is_ok());
        
        let results = result.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], ArgValidationResult::Valid);
        assert_eq!(results[1], ArgValidationResult::Valid);
    }
    
    #[test]
    fn test_zero_copy_arg_buffer() {
        let buffer = ZeroCopyArgBuffer::new(0x1000, 1024, false);
        
        assert_eq!(buffer.addr, 0x1000);
        assert_eq!(buffer.size, 1024);
        assert!(!buffer.read_only);
        assert_eq!(buffer.ref_count(), 1);
        
        buffer.inc_ref();
        assert_eq!(buffer.ref_count(), 2);
        
        assert!(buffer.dec_ref());
        assert_eq!(buffer.ref_count(), 1);
        
        assert!(!buffer.dec_ref());
        assert_eq!(buffer.ref_count(), 0);
    }
    
    #[test]
    fn test_batch_arg_processor() {
        let mut processor = BatchArgProcessor::new();
        let mut handler = OptimizedArgHandler::new();
        
        // Register a simple syscall
        handler.register_syscall(SyscallArgDescriptor {
            syscall_num: 0x1000,
            args: vec![
                ArgDescriptor::new(ArgType::U32).input(),
                ArgDescriptor::new(ArgType::U32).input(),
            ],
            fast_path_handler: None,
        });
        
        // Add syscalls to batch
        processor.add_syscall(0x1000, vec![100, 200]);
        processor.add_syscall(0x1000, vec![300, 400]);
        
        // Process batch
        processor.process(&handler);
        
        // Check results
        let results = processor.get_results();
        assert_eq!(results.len(), 2);
        
        let validation_results = processor.get_validation_results();
        assert_eq!(validation_results.len(), 2);
    }
}