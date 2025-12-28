#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! Machine Learning Optimization
//!
//! This module implements ML-based optimization for NOS:
//! - ML-based scheduler
//! - ML-based memory management
//! - ML-based I/O prediction
//! - ML model training
//!
//! Features:
//! - Reinforcement learning for scheduling
//! - Predictive I/O batching
//! - Anomaly detection
//! - Dynamic threshold adjustment
//! - Feedback-driven optimization

pub mod scheduler;
pub mod memory;
pub mod io;
pub mod training;
pub mod metrics;

// Re-export common ML types
pub use scheduler::*;
pub use memory::*;
pub use io::*;
pub use training::*;
pub use metrics::*;
