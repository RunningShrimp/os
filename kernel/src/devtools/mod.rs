//! Developer Tools
//!
//! This module implements developer tools for NOS:
//! - Kernel debugger
//! - Performance profiler
//! - Testing framework
//!
//! Features:
//! - Hardware/software breakpoints
//! - Call stack and memory inspection
//! - Function profiling and flame graphs
//! - Comprehensive testing (unit/integration/fuzz)

pub mod debugger;
pub mod profiler;
pub mod testing;

// Re-export devtools types
pub use debugger::*;
pub use profiler::*;
pub use testing::*;
