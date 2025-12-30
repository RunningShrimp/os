//! Production monitoring system
//!
//! Provides system monitoring and metrics collection for production environments.
//!
//! ## Modules
//!
//! - `metrics`: System metrics collection and registry
//! - `sampling`: Performance sampling with statistical analysis
//! - `profiler`: CPU profiling with stack sampling
//! - `export`: Data export in various formats (Prometheus, JSON, etc.)
//! - `health`: System health monitoring
//! - `alerting`: Alerting framework
//! - `timeline`: Event timeline tracking

pub mod alerting;
pub mod export;
pub mod health;
pub mod health_integration;
pub mod metrics;
pub mod profiler;
pub mod sampling;
pub mod timeline;
