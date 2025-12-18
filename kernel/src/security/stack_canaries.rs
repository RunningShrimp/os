//! Stack Canaries Implementation
//! 
//! Provides stack buffer overflow protection through canary values
//! placed between local variables and the return address on the stack.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;

/// Stack canary configuration
#[derive(Debug, Clone)]
pub struct CanaryConfig {
    /// Canary value size in bytes (4, 8, or 16)
    pub canary_size: usize,
    /// Random canary generation interval (in function calls)
    pub randomization_interval: usize,
    /// Enable per-thread canaries
    pub per_thread_canaries: bool,
    /// Enable canary entropy from multiple sources
    pub high_entropy_canaries: bool,
    /// Enable canary validation on context switches
    pub validate_on_context_switch: bool,
    /// Canary corruption action
    pub corruption_action: CanaryCorruptionAction,
}

impl Default for CanaryConfig {
    fn default() -> Self {
        Self {
            canary_size: 8, // 64-bit canaries
            randomization_interval: 1000,
            per_thread_canaries: true,
            high_entropy_canaries: true,
            validate_on_context_switch: true,
            corruption_action: CanaryCorruptionAction::Terminate,
        }
    }
}

/// Actions to take when canary corruption is detected
#[derive(Debug, Clone, PartialEq)]
pub enum CanaryCorruptionAction {
    /// Terminate the process immediately
    Terminate,
    /// Raise a security exception
    RaiseException,
    /// Log and continue (for debugging only)
    LogAndContinue,
    /// Call custom handler
    CustomHandler(fn(&CanaryCorruptionInfo)),
}

/// Information about canary corruption
#[derive(Debug, Clone)]
pub struct CanaryCorruptionInfo {
    /// Thread ID where corruption was detected
    pub thread_id: u64,
    /// Function address where corruption was detected
    pub function_address: usize,
    /// Expected canary value
    pub expected_canary: u64,
    /// Actual canary value found
    pub actual_canary: u64,
    /// Stack pointer at time of detection
    pub stack_pointer: usize,
    /// Timestamp of detection
    pub timestamp: u64,
}

/// Stack canary statistics
#[derive(Debug, Default)]
pub struct CanaryStats {
    /// Total number of canary validations
    pub total_validations: AtomicUsize,
    /// Number of successful validations
    pub successful_validations: AtomicUsize,
    /// Number of canary corruptions detected
    pub corruptions_detected: AtomicUsize,
    /// Number of canary generations
    pub canary_generations: AtomicUsize,
    /// Number of false positives
    pub false_positives: AtomicUsize,
}

/// Per-thread canary context
#[derive(Debug)]
pub struct ThreadCanaryContext {
    /// Thread ID
    pub thread_id: u64,
    /// Current canary value
    pub current_canary: u64,
    /// Canary generation counter
    pub generation_counter: usize,
    /// Function call depth
    pub call_depth: usize,
    /// Stack frame canaries
    pub frame_canaries: Vec<(usize, u64)>, // (stack_pointer, canary_value)
}

impl ThreadCanaryContext {
    pub fn new(thread_id: u64) -> Self {
        Self {
            thread_id,
            current_canary: 0,
            generation_counter: 0,
            call_depth: 0,
            frame_canaries: Vec::new(),
        }
    }

    pub fn push_frame(&mut self, stack_pointer: usize, canary: u64) {
        self.frame_canaries.push((stack_pointer, canary));
        self.call_depth += 1;
    }

    pub fn pop_frame(&mut self) -> Option<(usize, u64)> {
        if self.call_depth > 0 {
            self.call_depth -= 1;
            self.frame_canaries.pop()
        } else {
            None
        }
    }

    pub fn validate_frame(&self, stack_pointer: usize, expected_canary: u64) -> bool {
        self.frame_canaries.iter().any(|(sp, canary)| {
            *sp == stack_pointer && *canary == expected_canary
        })
    }
}

/// Stack canary subsystem
pub struct StackCanarySubsystem {
    /// Configuration
    config: CanaryConfig,
    /// Global canary seed
    global_seed: AtomicU64,
    /// Canary statistics
    stats: CanaryStats,
    /// Per-thread canary contexts
    thread_contexts: Mutex<Vec<ThreadCanaryContext>>,
    /// Canary generation counter
    generation_counter: AtomicUsize,
    /// Last entropy sources
    last_entropy_sources: Mutex<Vec<u64>>,
}

impl StackCanarySubsystem {
    /// Create a new stack canary subsystem
    pub fn new(config: CanaryConfig) -> Self {
        Self {
            config,
            global_seed: AtomicU64::new(0),
            stats: CanaryStats::default(),
            thread_contexts: Mutex::new(Vec::new()),
            generation_counter: AtomicUsize::new(0),
            last_entropy_sources: Mutex::new(Vec::new()),
        }
    }

    /// Initialize the stack canary subsystem
    pub fn initialize(&self) -> Result<(), &'static str> {
        // Initialize global seed with high entropy
        let seed = self.generate_high_entropy_seed()?;
        self.global_seed.store(seed, Ordering::SeqCst);

        // Initialize entropy sources
        let mut sources = self.last_entropy_sources.lock();
        sources.push(seed);
        sources.push(self.get_timestamp_entropy());
        sources.push(self.get_cpu_entropy());
        sources.push(self.get_memory_entropy());

        Ok(())
    }

    /// Generate a high-entropy seed
    fn generate_high_entropy_seed(&self) -> Result<u64, &'static str> {
        let mut entropy = 0u64;

        // Combine multiple entropy sources
        entropy ^= self.get_timestamp_entropy();
        entropy ^= self.get_cpu_entropy();
        entropy ^= self.get_memory_entropy();
        entropy ^= self.get_rdrand_entropy().unwrap_or(0);
        entropy ^= self.get_process_entropy();

        // Add additional mixing
        entropy = entropy.wrapping_mul(0x9e3779b97f4a7c15);
        entropy ^= entropy >> 30;
        entropy = entropy.wrapping_mul(0xbf58476d1ce4e5b9);
        entropy ^= entropy >> 27;
        entropy = entropy.wrapping_mul(0x94d049bb133111eb);
        entropy ^= entropy >> 31;

        Ok(entropy)
    }

    /// Get timestamp-based entropy
    fn get_timestamp_entropy(&self) -> u64 {
        // Use high-resolution timer
        use crate::arch::x86_64::rdtsc;
        rdtsc() as u64
    }

    /// Get CPU-based entropy
    fn get_cpu_entropy(&self) -> u64 {
        // Use CPU-specific information
        let mut entropy = 0u64;
        
        // CPU frequency (simplified)
        entropy ^= 0x1234567890abcdef; // Placeholder for actual CPU frequency
        
        // Core ID
        entropy ^= crate::arch::current_cpu_id() as u64;
        
        entropy
    }

    /// Get memory-based entropy
    fn get_memory_entropy(&self) -> u64 {
        // Use memory layout information
        let mut entropy = 0u64;
        
        // Stack pointer
        entropy ^= self.get_stack_pointer() as u64;
        
        // Heap start (simplified)
        entropy ^= 0xabcdef1234567890; // Placeholder for actual heap start
        
        entropy
    }

    /// Get RDRAND entropy if available
    fn get_rdrand_entropy(&self) -> Option<u64> {
        // Try to use CPU's RDRAND instruction if available
        #[cfg(target_arch = "x86_64")]
        {
            if crate::arch::x86_64::has_rdrand() {
                return Some(crate::arch::x86_64::rdrand64());
            }
        }
        None
    }

    /// Get process-based entropy
    fn get_process_entropy(&self) -> u64 {
        // Use process-specific information
        let mut entropy = 0u64;
        
        // Process ID
        entropy ^= crate::process::current_pid() as u64;
        
        // Parent process ID
        entropy ^= crate::process::parent_pid() as u64;
        
        entropy
    }

    /// Get current stack pointer
    fn get_stack_pointer(&self) -> usize {
        let sp: usize;
        unsafe {
            core::arch::asm!("mov {}, rsp", out(reg) sp);
        }
        sp
    }

    /// Generate a new canary value
    pub fn generate_canary(&self, thread_id: u64) -> u64 {
        let generation = self.generation_counter.fetch_add(1, Ordering::SeqCst);
        let base_seed = self.global_seed.load(Ordering::SeqCst);
        
        let mut canary = base_seed;
        
        // Mix in thread ID
        canary ^= thread_id;
        
        // Mix in generation counter
        canary ^= generation as u64;
        
        // Mix in timestamp
        canary ^= self.get_timestamp_entropy();
        
        // Add additional mixing based on configuration
        if self.config.high_entropy_canaries {
            canary ^= self.get_cpu_entropy();
            canary ^= self.get_memory_entropy();
            
            // Rotate and mix
            canary = canary.rotate_left(13);
            canary ^= canary >> 7;
            canary = canary.rotate_left(17);
        }
        
        // Ensure canary doesn't have common patterns
        canary &= 0xFFFFFFFFFFFFFF00u64; // Clear low byte to avoid null bytes
        canary |= 0xFF; // Set low byte to 0xFF for easy detection
        
        self.stats.canary_generations.fetch_add(1, Ordering::SeqCst);
        
        canary
    }

    /// Get or create thread canary context
    fn get_thread_context(&self, thread_id: u64) -> ThreadCanaryContext {
        let mut contexts = self.thread_contexts.lock();
        
        // Find existing context
        if let Some(ctx) = contexts.iter().find(|ctx| ctx.thread_id == thread_id) {
            return ctx.clone();
        }
        
        // Create new context
        let mut ctx = ThreadCanaryContext::new(thread_id);
        ctx.current_canary = self.generate_canary(thread_id);
        contexts.push(ctx.clone());
        
        ctx
    }

    /// Update thread context
    fn update_thread_context(&self, thread_id: u64, context: ThreadCanaryContext) {
        let mut contexts = self.thread_contexts.lock();
        if let Some(idx) = contexts.iter().position(|ctx| ctx.thread_id == thread_id) {
            contexts[idx] = context;
        } else {
            contexts.push(context);
        }
    }

    /// Insert canary at function entry
    pub fn insert_canary(&self, thread_id: u64) -> u64 {
        let mut context = self.get_thread_context(thread_id);
        
        // Generate new canary if needed
        if context.generation_counter % self.config.randomization_interval == 0 {
            context.current_canary = self.generate_canary(thread_id);
        }
        
        let stack_pointer = self.get_stack_pointer();
        let canary = context.current_canary;
        
        context.push_frame(stack_pointer, canary);
        context.generation_counter += 1;
        
        self.update_thread_context(thread_id, context);
        
        canary
    }

    /// Validate canary at function exit
    pub fn validate_canary(&self, thread_id: u64, expected_canary: u64) -> Result<(), CanaryCorruptionInfo> {
        let stack_pointer = self.get_stack_pointer();
        let context = self.get_thread_context(thread_id);
        
        self.stats.total_validations.fetch_add(1, Ordering::SeqCst);
        
        // Check if canary matches
        if !context.validate_frame(stack_pointer, expected_canary) {
            let corruption_info = CanaryCorruptionInfo {
                thread_id,
                function_address: 0, // Would need to be passed in
                expected_canary,
                actual_canary: 0, // Would need to read from stack
                stack_pointer,
                timestamp: self.get_timestamp_entropy(),
            };
            
            self.stats.corruptions_detected.fetch_add(1, Ordering::SeqCst);
            
            // Handle corruption
            self.handle_canary_corruption(&corruption_info);
            
            return Err(corruption_info);
        }
        
        self.stats.successful_validations.fetch_add(1, Ordering::SeqCst);
        
        // Pop frame from context
        let mut context = context;
        context.pop_frame();
        self.update_thread_context(thread_id, context);
        
        Ok(())
    }

    /// Handle canary corruption
    fn handle_canary_corruption(&self, corruption_info: &CanaryCorruptionInfo) {
        match self.config.corruption_action {
            CanaryCorruptionAction::Terminate => {
                crate::process::terminate_process(corruption_info.thread_id);
            }
            CanaryCorruptionAction::RaiseException => {
                crate::arch::raise_security_exception("Stack canary corruption detected");
            }
            CanaryCorruptionAction::LogAndContinue => {
                log::error!("Stack canary corruption detected: {:?}", corruption_info);
            }
            CanaryCorruptionAction::CustomHandler(handler) => {
                handler(corruption_info);
            }
        }
    }

    /// Validate all canaries for a thread (used on context switch)
    pub fn validate_thread_canaries(&self, thread_id: u64) -> Result<(), Vec<CanaryCorruptionInfo>> {
        if !self.config.validate_on_context_switch {
            return Ok(());
        }
        
        let context = self.get_thread_context(thread_id);
        let mut corruptions = Vec::new();
        
        // Validate all frame canaries
        for (stack_pointer, expected_canary) in &context.frame_canaries {
            // In a real implementation, we would read the actual canary from the stack
            // For now, we'll just validate the structure
            if *expected_canary == 0 {
                let corruption_info = CanaryCorruptionInfo {
                    thread_id,
                    function_address: 0,
                    expected_canary: *expected_canary,
                    actual_canary: 0,
                    stack_pointer: *stack_pointer,
                    timestamp: self.get_timestamp_entropy(),
                };
                corruptions.push(corruption_info);
            }
        }
        
        if !corruptions.is_empty() {
            for corruption in &corruptions {
                self.handle_canary_corruption(corruption);
            }
            Err(corruptions)
        } else {
            Ok(())
        }
    }

    /// Get canary statistics
    pub fn get_stats(&self) -> &CanaryStats {
        &self.stats
    }

    /// Update configuration
    pub fn update_config(&mut self, config: CanaryConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn get_config(&self) -> &CanaryConfig {
        &self.config
    }
}

/// Global stack canary subsystem instance
static mut STACK_CANARY_SUBSYSTEM: Option<StackCanarySubsystem> = None;
static STACK_CANARY_INIT: spin::Once = spin::Once::new();

/// Initialize the global stack canary subsystem
pub fn init_stack_canaries(config: CanaryConfig) -> Result<(), &'static str> {
    STACK_CANARY_INIT.call_once(|| {
        let subsystem = StackCanarySubsystem::new(config);
        if let Err(e) = subsystem.initialize() {
            panic!("Failed to initialize stack canaries: {}", e);
        }
        unsafe {
            STACK_CANARY_SUBSYSTEM = Some(subsystem);
        }
    });
    Ok(())
}

/// Get the global stack canary subsystem
pub fn get_stack_canary_subsystem() -> Option<&'static StackCanarySubsystem> {
    unsafe {
        STACK_CANARY_SUBSYSTEM.as_ref()
    }
}

/// Insert a stack canary (called at function entry)
#[macro_export]
macro_rules! stack_canary_enter {
    () => {
        if let Some(subsystem) = $crate::security::stack_canaries::get_stack_canary_subsystem() {
            let thread_id = $crate::process::current_thread_id();
            let canary = subsystem.insert_canary(thread_id);
            
            // Store canary on stack
            #[cfg(target_arch = "x86_64")]
            unsafe {
                core::arch::asm!(
                    "push {}",
                    in(reg) canary,
                );
            }
        }
    };
}

/// Validate a stack canary (called at function exit)
#[macro_export]
macro_rules! stack_canary_exit {
    () => {
        if let Some(subsystem) = $crate::security::stack_canaries::get_stack_canary_subsystem() {
            let thread_id = $crate::process::current_thread_id();
            
            // Read canary from stack
            #[cfg(target_arch = "x86_64")]
            let canary: u64 = unsafe {
                let mut canary_val: u64;
                core::arch::asm!(
                    "pop {}",
                    out(reg) canary_val,
                );
                canary_val
            };
            
            #[cfg(not(target_arch = "x86_64"))]
            let canary = 0u64; // Placeholder for other architectures
            
            if let Err(corruption) = subsystem.validate_canary(thread_id, canary) {
                // Handle corruption (already done by subsystem)
                log::error!("Stack canary corruption detected: {:?}", corruption);
            }
        }
    };
}

/// Function attribute macro for automatic stack canary protection
#[macro_export]
macro_rules! protected_function {
    ($($item:tt)*) => {
        $crate::stack_canary_enter!();
        let _guard = $crate::security::stack_canaries::StackCanaryGuard::new();
        $($item)*
        $crate::stack_canary_exit!();
    };
}

/// RAII guard for stack canary protection
pub struct StackCanaryGuard {
    thread_id: u64,
    canary: u64,
}

impl StackCanaryGuard {
    pub fn new() -> Self {
        let thread_id = crate::process::current_thread_id();
        let canary = if let Some(subsystem) = get_stack_canary_subsystem() {
            subsystem.insert_canary(thread_id)
        } else {
            0
        };
        
        Self { thread_id, canary }
    }
}

impl Drop for StackCanaryGuard {
    fn drop(&mut self) {
        if let Some(subsystem) = get_stack_canary_subsystem() {
            if let Err(corruption) = subsystem.validate_canary(self.thread_id, self.canary) {
                log::error!("Stack canary corruption detected in RAII guard: {:?}", corruption);
            }
        }
    }
}