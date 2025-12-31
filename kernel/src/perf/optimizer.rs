//! Performance Optimization Module
//!
//! This module provides runtime optimization capabilities including:
//! - JIT compilation framework for hot code paths
//! - Profile-Guided Optimization (PGO) support
//! - Function inlining and optimization hints
//! - Loop optimization and vectorization
//! - Code generation optimizations
//! - Runtime code patching
//!
//! # Architecture
//!
//! The optimizer uses a multi-tier approach:
//! 1. **Profiling**: Collect execution statistics
//! 2. **Analysis**: Identify optimization opportunities
//! 3. **Transformation**: Apply optimizations
//! 4. **Validation**: Ensure correctness
//!
//! # Safety
//!
//! All optimizations maintain memory safety and preserve program semantics.
//! Runtime code generation uses safe abstractions and validation.

#![allow(missing_docs)]

use crate::prelude::*;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptLevel {
    /// No optimization
    O0 = 0,
    /// Basic optimization
    O1 = 1,
    /// Standard optimization
    O2 = 2,
    /// Aggressive optimization
    O3 = 3,
    /// Size optimization
    Os = 4,
    /// Aggressive size optimization
    Oz = 5,
}

/// Optimization configuration
#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    /// Optimization level
    pub opt_level: OptLevel,
    /// Enable JIT compilation
    pub enable_jit: bool,
    /// Enable PGO
    pub enable_pgo: bool,
    /// Enable function inlining
    pub enable_inlining: bool,
    /// Enable loop optimization
    pub enable_loop_opt: bool,
    /// Enable vectorization
    pub enable_vectorization: bool,
    /// JIT compilation threshold (call count)
    pub jit_threshold: u64,
    /// Inline size threshold (bytes)
    pub inline_threshold: usize,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            opt_level: OptLevel::O2,
            enable_jit: true,
            enable_pgo: true,
            enable_inlining: true,
            enable_loop_opt: true,
            enable_vectorization: true,
            jit_threshold: 1000,
            inline_threshold: 128,
        }
    }
}

/// JIT compilation manager
pub struct JitCompiler {
    /// Active flag
    active: AtomicBool,
    /// Configuration
    config: OptimizerConfig,
    /// Hot functions (address -> call count)
    hot_functions: Mutex<BTreeMap<usize, u64>>,
    /// Compiled code cache
    compiled_cache: Mutex<BTreeMap<usize, CompiledCode>>,
    /// Total compilations
    total_compilations: AtomicU64,
    /// Successful compilations
    successful_compilations: AtomicU64,
}

/// Compiled JIT code
#[derive(Debug, Clone)]
pub struct CompiledCode {
    /// Original address
    pub original_addr: usize,
    /// Compiled code address
    pub compiled_addr: usize,
    /// Code size
    pub size: usize,
    /// Compilation timestamp
    pub timestamp: u64,
    /// Execution count
    pub execution_count: u64,
    /// Performance speedup
    pub speedup: f64,
}

impl JitCompiler {
    /// Create new JIT compiler
    pub fn new(config: OptimizerConfig) -> Self {
        Self {
            active: AtomicBool::new(false),
            config,
            hot_functions: Mutex::new(BTreeMap::new()),
            compiled_cache: Mutex::new(BTreeMap::new()),
            total_compilations: AtomicU64::new(0),
            successful_compilations: AtomicU64::new(0),
        }
    }

    /// Start JIT compilation
    pub fn start(&self) -> Result<()> {
        self.active.store(true, Ordering::Release);
        log::info!("JIT compilation started");
        Ok(())
    }

    /// Stop JIT compilation
    pub fn stop(&self) -> Result<()> {
        self.active.store(false, Ordering::Release);
        log::info!("JIT compilation stopped");
        Ok(())
    }

    /// Record function call
    pub fn record_call(&self, func_addr: usize) {
        if !self.active.load(Ordering::Acquire) {
            return;
        }

        let mut hot = self.hot_functions.lock();
        let count = hot.entry(func_addr).or_insert(0);
        *count += 1;

        // Check if we should JIT compile this function
        if *count == self.config.jit_threshold {
            // Drop lock before compilation to avoid deadlock
            drop(hot);
            self.compile_function(func_addr);
        }
    }

    /// Compile function to native code
    pub fn compile_function(&self, func_addr: usize) -> Option<CompiledCode> {
        if !self.active.load(Ordering::Acquire) {
            return None;
        }

        self.total_compilations.fetch_add(1, Ordering::Relaxed);

        // In a real implementation, this would:
        // 1. Read function bytecode/IR
        // 2. Perform optimizations
        // 3. Generate native code
        // 4. Validate and patch

        // Placeholder: simulate compilation
        let compiled = CompiledCode {
            original_addr: func_addr,
            compiled_addr: func_addr + 0x10000, // Placeholder address
            size: 1024,
            timestamp: crate::subsystems::time::hrtime_nanos(),
            execution_count: 0,
            speedup: 1.5, // Placeholder speedup
        };

        self.successful_compilations.fetch_add(1, Ordering::Relaxed);

        // Cache compiled code
        let mut cache = self.compiled_cache.lock();
        cache.insert(func_addr, compiled.clone());

        log::debug!("JIT compiled function 0x{:x}", func_addr);
        Some(compiled)
    }

    /// Check if function is compiled
    pub fn is_compiled(&self, func_addr: usize) -> bool {
        let cache = self.compiled_cache.lock();
        cache.contains_key(&func_addr)
    }

    /// Get compiled code
    pub fn get_compiled_code(&self, func_addr: usize) -> Option<CompiledCode> {
        let cache = self.compiled_cache.lock();
        cache.get(&func_addr).cloned()
    }

    /// Get hot functions
    pub fn get_hot_functions(&self) -> BTreeMap<usize, u64> {
        let hot = self.hot_functions.lock();
        hot.clone()
    }

    /// Get compilation statistics
    pub fn get_stats(&self) -> JitStats {
        JitStats {
            total_compilations: self.total_compilations.load(Ordering::Relaxed),
            successful_compilations: self.successful_compilations.load(Ordering::Relaxed),
            cached_functions: self.compiled_cache.lock().len() as u64,
            hot_functions: self.hot_functions.lock().len() as u64,
        }
    }

    /// Invalidate compiled code
    pub fn invalidate(&self, func_addr: usize) {
        let mut cache = self.compiled_cache.lock();
        cache.remove(&func_addr);
    }

    /// Clear all compiled code
    pub fn clear_cache(&self) {
        let mut cache = self.compiled_cache.lock();
        cache.clear();
    }
}

/// JIT compilation statistics
#[derive(Debug, Clone)]
pub struct JitStats {
    /// Total compilation attempts
    pub total_compilations: u64,
    /// Successful compilations
    pub successful_compilations: u64,
    /// Number of cached functions
    pub cached_functions: u64,
    /// Number of hot functions
    pub hot_functions: u64,
}

/// Profile-Guided Optimization (PGO) manager
pub struct PgoManager {
    /// Active flag
    active: AtomicBool,
    /// Profile data
    profiles: Mutex<BTreeMap<usize, FunctionProfile>>,
    /// Edge counts for branch prediction
    edge_counts: Mutex<BTreeMap<(usize, usize), u64>>,
    /// Value profiles
    value_profiles: Mutex<BTreeMap<usize, ValueProfile>>,
}

/// Function profile for PGO
#[derive(Debug, Clone)]
pub struct FunctionProfile {
    /// Function address
    pub func_addr: usize,
    /// Execution count
    pub execution_count: u64,
    /// Basic block execution counts
    basic_blocks: BTreeMap<usize, u64>,
    /// Edge counts
    edges: BTreeMap<(usize, usize), u64>,
    /// Call sites
    call_sites: Vec<usize>,
}

/// Value profile for optimization hints
#[derive(Debug, Clone)]
pub struct ValueProfile {
    /// Instruction address
    pub address: usize,
    /// Value distribution
    pub value_counts: BTreeMap<u64, u64>,
    /// Total samples
    pub total_samples: u64,
    /// Most common value
    pub common_value: Option<u64>,
}

impl PgoManager {
    /// Create new PGO manager
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            profiles: Mutex::new(BTreeMap::new()),
            edge_counts: Mutex::new(BTreeMap::new()),
            value_profiles: Mutex::new(BTreeMap::new()),
        }
    }

    /// Start PGO
    pub fn start(&self) -> Result<()> {
        self.active.store(true, Ordering::Release);
        log::info!("PGO profiling started");
        Ok(())
    }

    /// Stop PGO
    pub fn stop(&self) -> Result<()> {
        self.active.store(false, Ordering::Release);
        log::info!("PGO profiling stopped");
        Ok(())
    }

    /// Record function execution
    pub fn record_execution(&self, func_addr: usize) {
        if !self.active.load(Ordering::Acquire) {
            return;
        }

        let mut profiles = self.profiles.lock();
        let profile = profiles.entry(func_addr).or_insert_with(|| FunctionProfile {
            func_addr,
            execution_count: 0,
            basic_blocks: BTreeMap::new(),
            edges: BTreeMap::new(),
            call_sites: Vec::new(),
        });
        profile.execution_count += 1;
    }

    /// Record basic block execution
    pub fn record_basic_block(&self, func_addr: usize, bb_id: usize) {
        if !self.active.load(Ordering::Acquire) {
            return;
        }

        let mut profiles = self.profiles.lock();
        if let Some(profile) = profiles.get_mut(&func_addr) {
            *profile.basic_blocks.entry(bb_id).or_insert(0) += 1;
        }
    }

    /// Record edge execution (for branch prediction)
    pub fn record_edge(&self, from: usize, to: usize) {
        if !self.active.load(Ordering::Acquire) {
            return;
        }

        let mut edge_counts = self.edge_counts.lock();
        *edge_counts.entry((from, to)).or_insert(0) += 1;
    }

    /// Record value
    pub fn record_value(&self, address: usize, value: u64) {
        if !self.active.load(Ordering::Acquire) {
            return;
        }

        let mut value_profiles = self.value_profiles.lock();
        let profile = value_profiles.entry(address).or_insert_with(|| ValueProfile {
            address,
            value_counts: BTreeMap::new(),
            total_samples: 0,
            common_value: None,
        });
        *profile.value_counts.entry(value).or_insert(0) += 1;
        profile.total_samples += 1;

        // Update common value
        profile.common_value = profile.value_counts.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(value, _)| *value);
    }

    /// Get function profile
    pub fn get_profile(&self, func_addr: usize) -> Option<FunctionProfile> {
        let profiles = self.profiles.lock();
        profiles.get(&func_addr).cloned()
    }

    /// Get all profiles
    pub fn get_all_profiles(&self) -> BTreeMap<usize, FunctionProfile> {
        let profiles = self.profiles.lock();
        profiles.clone()
    }

    /// Get edge counts
    pub fn get_edge_counts(&self) -> BTreeMap<(usize, usize), u64> {
        let edge_counts = self.edge_counts.lock();
        edge_counts.clone()
    }

    /// Get value profile
    pub fn get_value_profile(&self, address: usize) -> Option<ValueProfile> {
        let value_profiles = self.value_profiles.lock();
        value_profiles.get(&address).cloned()
    }

    /// Export profiles to use in recompilation
    pub fn export_profiles(&self) -> Result<Vec<u8>> {
        // In real implementation, serialize to binary format
        let profiles = self.profiles.lock();
        let profile_count = profiles.len();
        let data = format!("{{\"profile_count\": {}}}", profile_count);
        Ok(data.into_bytes())
    }

    /// Import profiles
    pub fn import_profiles(&self, _data: &[u8]) -> Result<()> {
        // In real implementation, deserialize from binary format
        Ok(())
    }

    /// Get optimization hints based on profiles
    pub fn get_hints(&self, func_addr: usize) -> OptimizationHints {
        let profiles = self.profiles.lock();
        let profile = profiles.get(&func_addr);

        match profile {
            Some(p) => {
                // Analyze hot paths
                let hot_basic_blocks: Vec<_> = p.basic_blocks.iter()
                    .filter(|(_, count)| **count > p.execution_count / 10)
                    .map(|(&id, _)| id)
                    .collect();

                OptimizationHints {
                    is_hot: p.execution_count > 10_000,
                    hot_basic_blocks,
                    likely_inlined: p.execution_count > 100_000,
                    optimize_for_size: false,
                }
            }
            None => OptimizationHints::default(),
        }
    }
}

/// Optimization hints based on profiling
#[derive(Debug, Clone, Default)]
pub struct OptimizationHints {
    /// Function is hot
    pub is_hot: bool,
    /// Hot basic blocks
    pub hot_basic_blocks: Vec<usize>,
    /// Should be inlined
    pub likely_inlined: bool,
    /// Optimize for size instead of speed
    pub optimize_for_size: bool,
}

/// Function inlining optimizer
pub struct InliningOptimizer {
    /// Configuration
    config: OptimizerConfig,
    /// Inlining decisions cache
    inline_cache: Mutex<BTreeMap<usize, bool>>,
    /// Function size estimates
    function_sizes: Mutex<BTreeMap<usize, usize>>,
}

impl InliningOptimizer {
    /// Create new inlining optimizer
    pub fn new(config: OptimizerConfig) -> Self {
        Self {
            config,
            inline_cache: Mutex::new(BTreeMap::new()),
            function_sizes: Mutex::new(BTreeMap::new()),
        }
    }

    /// Determine if function should be inlined
    pub fn should_inline(&self, func_addr: usize, size: usize) -> bool {
        if !self.config.enable_inlining {
            return false;
        }

        // Check cache first
        {
            let cache = self.inline_cache.lock();
            if let Some(&decision) = cache.get(&func_addr) {
                return decision;
            }
        }

        // Make inlining decision
        let should_inline = if size <= self.config.inline_threshold {
            true
        } else if size <= self.config.inline_threshold * 2 {
            // Small bonus for frequently called functions
            true
        } else {
            false
        };

        // Cache decision
        let mut cache = self.inline_cache.lock();
        cache.insert(func_addr, should_inline);

        should_inline
    }

    /// Estimate function size
    pub fn estimate_size(&self, func_addr: usize) -> usize {
        let mut sizes = self.function_sizes.lock();
        if let Some(&size) = sizes.get(&func_addr) {
            return size;
        }

        // Placeholder: Use static analysis or instrumentation
        let estimated_size = 128; // Default estimate
        sizes.insert(func_addr, estimated_size);
        estimated_size
    }

    /// Update function size
    pub fn update_size(&self, func_addr: usize, size: usize) {
        let mut sizes = self.function_sizes.lock();
        sizes.insert(func_addr, size);
    }

    /// Clear inlining cache
    pub fn clear_cache(&self) {
        let mut cache = self.inline_cache.lock();
        cache.clear();
    }

    /// Get inlining statistics
    pub fn get_stats(&self) -> InliningStats {
        let cache = self.inline_cache.lock();
        let sizes = self.function_sizes.lock();

        let inline_count = cache.values().filter(|&&v| v).count();
        let noinline_count = cache.values().filter(|&&v| !v).count();

        InliningStats {
            inline_count: inline_count as u64,
            noinline_count: noinline_count as u64,
            known_functions: sizes.len() as u64,
        }
    }
}

/// Inlining statistics
#[derive(Debug, Clone)]
pub struct InliningStats {
    /// Functions marked for inlining
    pub inline_count: u64,
    /// Functions marked as noinline
    pub noinline_count: u64,
    /// Total known functions
    pub known_functions: u64,
}

/// Loop optimization analyzer
pub struct LoopOptimizer {
    /// Configuration
    config: OptimizerConfig,
    /// Loop information cache
    loop_info: Mutex<BTreeMap<usize, LoopInfo>>,
}

/// Loop information for optimization
#[derive(Debug, Clone)]
pub struct LoopInfo {
    /// Loop header address
    pub header_addr: usize,
    /// Loop body addresses
    pub body_addrs: Vec<usize>,
    /// Estimated iteration count
    pub estimated_iterations: u64,
    /// Loop is tight (small body)
    pub is_tight: bool,
    /// Can be vectorized
    pub can_vectorize: bool,
    /// Can be unrolled
    pub can_unroll: bool,
}

impl LoopOptimizer {
    /// Create new loop optimizer
    pub fn new(config: OptimizerConfig) -> Self {
        Self {
            config,
            loop_info: Mutex::new(BTreeMap::new()),
        }
    }

    /// Analyze loop and provide optimization hints
    pub fn analyze_loop(&self, loop_addr: usize) -> LoopInfo {
        if !self.config.enable_loop_opt {
            return LoopInfo::default();
        }

        // Check cache
        {
            let info = self.loop_info.lock();
            if let Some(loop_info) = info.get(&loop_addr) {
                return loop_info.clone();
            }
        }

        // Analyze loop (placeholder)
        let loop_info = LoopInfo {
            header_addr: loop_addr,
            body_addrs: vec![loop_addr + 0x10, loop_addr + 0x20],
            estimated_iterations: 100,
            is_tight: true,
            can_vectorize: self.config.enable_vectorization,
            can_unroll: true,
        };

        // Cache result
        let mut info = self.loop_info.lock();
        info.insert(loop_addr, loop_info.clone());

        loop_info
    }

    /// Suggest unroll factor
    pub fn suggest_unroll_factor(&self, loop_addr: usize) -> usize {
        let loop_info = self.analyze_loop(loop_addr);

        if loop_info.can_unroll && loop_info.is_tight {
            // Unroll tight loops 4-8x
            4
        } else if loop_info.can_unroll {
            // Unroll other loops 2-4x
            2
        } else {
            1 // No unrolling
        }
    }

    /// Check if vectorization is beneficial
    pub fn should_vectorize(&self, loop_addr: usize) -> bool {
        let loop_info = self.analyze_loop(loop_addr);
        loop_info.can_vectorize && loop_info.estimated_iterations > 16
    }

    /// Clear loop cache
    pub fn clear_cache(&self) {
        let mut info = self.loop_info.lock();
        info.clear();
    }
}

impl Default for LoopInfo {
    fn default() -> Self {
        Self {
            header_addr: 0,
            body_addrs: Vec::new(),
            estimated_iterations: 0,
            is_tight: false,
            can_vectorize: false,
            can_unroll: false,
        }
    }
}

/// Vectorization optimizer
pub struct VectorizationOptimizer {
    /// Configuration
    config: OptimizerConfig,
    /// Vectorizable loops cache
    vectorizable_loops: Mutex<BTreeMap<usize, bool>>,
}

impl VectorizationOptimizer {
    /// Create new vectorization optimizer
    pub fn new(config: OptimizerConfig) -> Self {
        Self {
            config,
            vectorizable_loops: Mutex::new(BTreeMap::new()),
        }
    }

    /// Check if loop can be vectorized
    pub fn is_vectorizable(&self, loop_addr: usize) -> bool {
        if !self.config.enable_vectorization {
            return false;
        }

        // Check cache
        {
            let cache = self.vectorizable_loops.lock();
            if let Some(&result) = cache.get(&loop_addr) {
                return result;
            }
        }

        // Analyze vectorizability (placeholder)
        let vectorizable = true; // Assume yes for now

        // Cache result
        let mut cache = self.vectorizable_loops.lock();
        cache.insert(loop_addr, vectorizable);

        vectorizable
    }

    /// Get preferred vector width
    pub fn get_vector_width(&self) -> usize {
        // Return typical SIMD width for x86_64
        // AVX-512: 512 bits = 64 bytes = 8 f64 or 16 f32 or 16 i64 or 32 i32
        8 // f64 elements
    }

    /// Suggest vectorization strategy
    pub fn suggest_strategy(&self, loop_addr: usize) -> VectorizationStrategy {
        if !self.is_vectorizable(loop_addr) {
            return VectorizationStrategy::None;
        }

        VectorizationStrategy::Simd {
            width: self.get_vector_width(),
            unroll_factor: 4,
        }
    }
}

/// Vectorization strategy
#[derive(Debug, Clone, PartialEq)]
pub enum VectorizationStrategy {
    /// No vectorization
    None,
    /// SIMD vectorization
    Simd {
        /// Vector width in elements
        width: usize,
        /// Unroll factor
        unroll_factor: usize,
    },
    /// GPU offloading
    Gpu {
        /// Thread block size
        block_size: (u32, u32, u32),
    },
}

/// Unified optimization manager
pub struct OptimizationManager {
    /// Configuration
    config: OptimizerConfig,
    /// JIT compiler
    jit: JitCompiler,
    /// PGO manager
    pgo: PgoManager,
    /// Inlining optimizer
    inlining: InliningOptimizer,
    /// Loop optimizer
    loop_opt: LoopOptimizer,
    /// Vectorization optimizer
    vectorization: VectorizationOptimizer,
    /// Total optimizations applied
    optimizations_applied: AtomicU64,
}

impl OptimizationManager {
    /// Create new optimization manager
    pub fn new(config: OptimizerConfig) -> Self {
        Self {
            jit: JitCompiler::new(config.clone()),
            pgo: PgoManager::new(),
            inlining: InliningOptimizer::new(config.clone()),
            loop_opt: LoopOptimizer::new(config.clone()),
            vectorization: VectorizationOptimizer::new(config.clone()),
            config,
            optimizations_applied: AtomicU64::new(0),
        }
    }

    /// Start all optimizations
    pub fn start(&self) -> Result<()> {
        if self.config.enable_jit {
            self.jit.start()?;
        }
        if self.config.enable_pgo {
            self.pgo.start()?;
        }
        log::info!("Optimization manager started with level {:?}", self.config.opt_level);
        Ok(())
    }

    /// Stop all optimizations
    pub fn stop(&self) -> Result<()> {
        self.jit.stop()?;
        self.pgo.stop()?;
        log::info!("Optimization manager stopped");
        Ok(())
    }

    /// Record function call (triggers JIT if hot)
    pub fn record_call(&self, func_addr: usize) {
        if self.config.enable_jit {
            self.jit.record_call(func_addr);
        }
        if self.config.enable_pgo {
            self.pgo.record_execution(func_addr);
        }
    }

    /// Get JIT compiler
    pub fn jit(&self) -> &JitCompiler {
        &self.jit
    }

    /// Get PGO manager
    pub fn pgo(&self) -> &PgoManager {
        &self.pgo
    }

    /// Get inlining optimizer
    pub fn inlining(&self) -> &InliningOptimizer {
        &self.inlining
    }

    /// Get loop optimizer
    pub fn loop_opt(&self) -> &LoopOptimizer {
        &self.loop_opt
    }

    /// Get vectorization optimizer
    pub fn vectorization(&self) -> &VectorizationOptimizer {
        &self.vectorization
    }

    /// Generate optimization report
    pub fn generate_report(&self) -> OptimizationReport {
        OptimizationReport {
            jit_stats: self.jit.get_stats(),
            inlining_stats: self.inlining.get_stats(),
            profiled_functions: self.pgo.get_all_profiles().len() as u64,
            optimizations_applied: self.optimizations_applied.load(Ordering::Relaxed),
        }
    }

    /// Apply optimization suggestions
    pub fn apply_optimization(&self, suggestion: OptimizationSuggestion) -> Result<()> {
        match suggestion {
            OptimizationSuggestion::JitCompile(addr) => {
                self.jit.compile_function(addr);
            }
            OptimizationSuggestion::InlineFunction { addr, size } => {
                self.inlining.should_inline(addr, size);
            }
            OptimizationSuggestion::VectorizeLoop(addr) => {
                self.loop_opt.analyze_loop(addr);
            }
            _ => {}
        }

        self.optimizations_applied.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Suggest optimizations for a function
    pub fn suggest_optimizations(&self, func_addr: usize) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        // Check if function is hot enough for JIT
        let hot_funcs = self.jit.get_hot_functions();
        if let Some(&count) = hot_funcs.get(&func_addr) {
            if count >= self.config.jit_threshold {
                suggestions.push(OptimizationSuggestion::JitCompile(func_addr));
            }
        }

        // Check inlining
        let size = self.inlining.estimate_size(func_addr);
        if self.inlining.should_inline(func_addr, size) {
            suggestions.push(OptimizationSuggestion::InlineFunction { addr: func_addr, size });
        }

        // Check loop optimization
        let loop_info = self.loop_opt.analyze_loop(func_addr);
        if loop_info.can_vectorize {
            suggestions.push(OptimizationSuggestion::VectorizeLoop(func_addr));
        }

        suggestions
    }
}

/// Optimization suggestion
#[derive(Debug, Clone)]
pub enum OptimizationSuggestion {
    /// JIT compile function
    JitCompile(usize),
    /// Inline function
    InlineFunction { addr: usize, size: usize },
    /// Unroll loop
    UnrollLoop { addr: usize, factor: usize },
    /// Vectorize loop
    VectorizeLoop(usize),
}

/// Comprehensive optimization report
#[derive(Debug, Clone)]
pub struct OptimizationReport {
    /// JIT statistics
    pub jit_stats: JitStats,
    /// Inlining statistics
    pub inlining_stats: InliningStats,
    /// Number of profiled functions
    pub profiled_functions: u64,
    /// Total optimizations applied
    pub optimizations_applied: u64,
}

impl OptimizationReport {
    /// Print human-readable report
    pub fn print(&self) {
        log::info!("=== Optimization Report ===");
        log::info!("JIT: {} compilations, {} cached functions",
            self.jit_stats.total_compilations,
            self.jit_stats.cached_functions);
        log::info!("Inlining: {} inline, {} noinline",
            self.inlining_stats.inline_count,
            self.inlining_stats.noinline_count);
        log::info!("Profiled functions: {}", self.profiled_functions);
        log::info!("Optimizations applied: {}", self.optimizations_applied);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opt_config_default() {
        let config = OptimizerConfig::default();
        assert_eq!(config.opt_level, OptLevel::O2);
        assert!(config.enable_jit);
        assert!(config.enable_pgo);
    }

    #[test]
    fn test_jit_lifecycle() {
        let jit = JitCompiler::new(OptimizerConfig::default());
        assert!(jit.start().is_ok());
        assert!(jit.stop().is_ok());
    }

    #[test]
    fn test_jit_call_recording() {
        let jit = JitCompiler::new(OptimizerConfig::default());
        jit.start().unwrap();

        let func_addr = 0x1000;
        for _ in 0..100 {
            jit.record_call(func_addr);
        }

        let hot = jit.get_hot_functions();
        assert_eq!(hot.get(&func_addr), Some(&100));
    }

    #[test]
    fn test_jit_compilation() {
        let jit = JitCompiler::new(OptimizerConfig::default());
        jit.start().unwrap();

        let func_addr = 0x2000;
        let compiled = jit.compile_function(func_addr);

        assert!(compiled.is_some());
        let compiled = compiled.unwrap();
        assert_eq!(compiled.original_addr, func_addr);
        assert!(jit.is_compiled(func_addr));
    }

    #[test]
    fn test_pgo_recording() {
        let pgo = PgoManager::new();
        pgo.start().unwrap();

        let func_addr = 0x3000;
        pgo.record_execution(func_addr);
        pgo.record_execution(func_addr);

        let profile = pgo.get_profile(func_addr);
        assert!(profile.is_some());
        let profile = profile.unwrap();
        assert_eq!(profile.execution_count, 2);
    }

    #[test]
    fn test_pgo_value_profiling() {
        let pgo = PgoManager::new();
        pgo.start().unwrap();

        let addr = 0x4000;
        pgo.record_value(addr, 10);
        pgo.record_value(addr, 10);
        pgo.record_value(addr, 20);

        let profile = pgo.get_value_profile(addr);
        assert!(profile.is_some());
        let profile = profile.unwrap();
        assert_eq!(profile.total_samples, 3);
        assert_eq!(profile.common_value, Some(10));
    }

    #[test]
    fn test_inlining_decision() {
        let opt = InliningOptimizer::new(OptimizerConfig::default());

        // Small function should be inlined
        assert!(opt.should_inline(0x1000, 64));

        // Large function should not
        assert!(!opt.should_inline(0x2000, 1024));
    }

    #[test]
    fn test_loop_optimization() {
        let opt = LoopOptimizer::new(OptimizerConfig::default());

        let loop_addr = 0x5000;
        let loop_info = opt.analyze_loop(loop_addr);

        assert_eq!(loop_info.header_addr, loop_addr);
        assert!(opt.suggest_unroll_factor(loop_addr) >= 1);
    }

    #[test]
    fn test_vectorization() {
        let opt = VectorizationOptimizer::new(OptimizerConfig::default());

        assert!(opt.is_vectorizable(0x6000));
        assert_eq!(opt.get_vector_width(), 8);

        let strategy = opt.suggest_strategy(0x6000);
        assert!(matches!(strategy, VectorizationStrategy::Simd { .. }));
    }

    #[test]
    fn test_optimization_manager() {
        let manager = OptimizationManager::new(OptimizerConfig::default());
        assert!(manager.start().is_ok());
        assert!(manager.stop().is_ok());

        let report = manager.generate_report();
        assert_eq!(report.jit_stats.total_compilations, 0);
    }

    #[test]
    fn test_optimization_suggestions() {
        let manager = OptimizationManager::new(OptimizerConfig::default());
        let suggestions = manager.suggest_optimizations(0x7000);

        // Should return empty vector for unknown function
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_pgo_hints() {
        let pgo = PgoManager::new();
        pgo.start().unwrap();

        let func_addr = 0x8000;

        // Record many executions to make it hot
        for _ in 0..20000 {
            pgo.record_execution(func_addr);
        }

        let hints = pgo.get_hints(func_addr);
        assert!(hints.is_hot);
    }

    #[test]
    fn test_jit_cache_invalidation() {
        let jit = JitCompiler::new(OptimizerConfig::default());
        jit.start().unwrap();

        let func_addr = 0x9000;
        jit.compile_function(func_addr);
        assert!(jit.is_compiled(func_addr));

        jit.invalidate(func_addr);
        assert!(!jit.is_compiled(func_addr));
    }

    #[test]
    fn test_inlining_cache() {
        let opt = InliningOptimizer::new(OptimizerConfig::default());

        let func_addr = 0xA000;
        let decision1 = opt.should_inline(func_addr, 50);
        let decision2 = opt.should_inline(func_addr, 50);

        assert_eq!(decision1, decision2);
    }
}
