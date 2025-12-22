//! Static Analyzer Types
//! 
//! This module defines all the core types used by the static analyzer.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::AtomicBool;

/// Analysis level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisLevel {
    /// Basic analysis
    Basic,
    /// Medium analysis
    Medium,
    /// Deep analysis
    Deep,
    /// Complete analysis
    Complete,
}

/// Analysis type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisType {
    /// Data flow analysis
    DataFlowAnalysis,
    /// Control flow analysis
    ControlFlowAnalysis,
    /// Pointer analysis
    PointerAnalysis,
    /// Side effect analysis
    SideEffectAnalysis,
    /// Dead code detection
    DeadCodeDetection,
    /// Security analysis
    SecurityAnalysis,
    /// Memory leak detection
    MemoryLeakDetection,
    /// Race condition detection
    RaceConditionDetection,
    /// Data race detection
    DataRaceDetection,
    /// Buffer overflow detection
    BufferOverflowDetection,
    /// Integer overflow detection
    IntegerOverflowDetection,
    /// Null pointer dereference detection
    NullPointerDereference,
    /// Uninitialized variable detection
    UninitializedVariableDetection,
}

/// Analysis precision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisPrecision {
    /// Conservative analysis
    Conservative,
    /// Balanced precision
    Balanced,
    /// Precise analysis
    Precise,
}

/// Widening strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WideningStrategy {
    /// Standard widening
    Standard,
    /// Prioritized widening
    Prioritized,
    /// Path sensitive widening
    PathSensitive,
    /// Adaptive widening
    Adaptive,
}

/// Static analyzer configuration
#[derive(Debug, Clone)]
pub struct StaticAnalyzerConfig {
    /// Analysis level
    pub level: AnalysisLevel,
    /// Analysis precision
    pub precision: AnalysisPrecision,
    /// Widening strategy
    pub widening: WideningStrategy,
    /// Maximum analysis time in milliseconds
    pub max_time_ms: u64,
    /// Maximum memory usage in bytes
    pub max_memory: u64,
    /// Enable verbose output
    pub verbose: bool,
    /// Enable debug mode
    pub debug: bool,
}

impl Default for StaticAnalyzerConfig {
    fn default() -> Self {
        Self {
            level: AnalysisLevel::Basic,
            precision: AnalysisPrecision::Balanced,
            widening: WideningStrategy::Standard,
            max_time_ms: 10000,
            max_memory: 1024 * 1024 * 100, // 100MB
            verbose: false,
            debug: false,
        }
    }
}

/// Static analysis result
#[derive(Debug, Clone)]
pub struct StaticAnalysisResult {
    /// Analysis type
    pub analysis_type: AnalysisType,
    /// Result message
    pub message: String,
    /// Result location
    pub location: String,
    /// Result severity
    pub severity: Severity,
    /// Result data
    pub data: Option<alloc::collections::BTreeMap<String, String>>,
}

/// Result severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Information
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical
    Critical,
}

/// Analysis statistics
#[derive(Debug, Clone)]
pub struct AnalysisStats {
    /// Total analysis time in milliseconds
    pub total_time_ms: u64,
    /// Number of analysis passes
    pub passes: u32,
    /// Number of results found
    pub results: u32,
    /// Memory usage in bytes
    pub memory_usage: u64,
}

impl AnalysisStats {
    /// Create new analysis statistics
    pub fn new() -> Self {
        Self {
            total_time_ms: 0,
            passes: 0,
            results: 0,
            memory_usage: 0,
        }
    }
}
