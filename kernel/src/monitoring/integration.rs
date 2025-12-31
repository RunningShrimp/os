//! Monitoring integration and instrumentation helpers
//!
//! This module provides integration helpers and instrumentation macros
//! for the observability stack (tracing, metrics, logging).

extern crate alloc;

use alloc::{string::String, string::ToString};

use crate::monitoring::{
    enhanced_metrics::get_registry,
    logging::get_logger,
    tracing::get_tracer,
};

/// Instrumentation scope that automatically manages span lifecycle
pub struct InstrumentationScope {
    /// Span handle
    handle: Option<crate::monitoring::SpanHandle>,
    /// Component name
    component: String,
    /// Operation name
    operation: String,
}

impl InstrumentationScope {
    /// Create a new instrumentation scope
    pub fn new(component: &str, operation: &str) -> Self {
        let tracer = get_tracer();
        let handle = tracer.map(|t| t.start_span(&format!("{}:{}", component, operation)));

        Self {
            handle,
            component: component.to_string(),
            operation: operation.to_string(),
        }
    }

    /// Add an attribute to the current span
    pub fn add_attribute(&self, key: &str, value: &str) {
        if let Some(tracer) = get_tracer() {
            if let Some(handle) = self.handle {
                let mut attrs = alloc::collections::BTreeMap::new();
                attrs.insert(key.to_string(), value.to_string());
                let _ = tracer.add_attributes(handle, attrs);
            }
        }
    }

    /// Record an error in the current span
    pub fn record_error(&self, error: &str) {
        self.add_attribute("error", error);
        if let Some(logger) = get_logger() {
            logger.error(&self.component, &format!("{}: {}", self.operation, error));
        }
    }

    /// Record a metric value
    pub fn record_metric(&self, metric_name: &str, value: u64) {
        if let Some(registry) = get_registry() {
            // Try to observe as histogram, fall back to counter if not found
            // This is a simplified approach - in production you'd want more sophisticated handling
            registry.histogram_observe(metric_name, value);
        }
    }
}

impl Drop for InstrumentationScope {
    fn drop(&mut self) {
        if let Some(tracer) = get_tracer() {
            if let Some(handle) = self.handle.take() {
                tracer.end_span(handle);
            }
        }
    }
}

/// Instrument a function call with tracing
#[macro_export]
macro_rules! instrument {
    ($component:expr, $operation:expr, $block:block) => {{
        let _scope = $crate::monitoring::integration::InstrumentationScope::new($component, $operation);
        $block
    }};
}

/// Record a system call with tracing and metrics
#[inline]
pub fn record_syscall(syscall_name: &str, duration_ns: u64, success: bool) {
    // Update counters
    if let Some(registry) = get_registry() {
        if success {
            registry.counter_inc("syscalls_success_total");
        } else {
            registry.counter_inc("syscalls_failed_total");
        }
        registry.counter_inc("syscalls_total");

        // Record duration in milliseconds
        let duration_ms = duration_ns / 1_000_000;
        registry.histogram_observe("syscall_duration_ms", duration_ms);
    }

    // Log if slow or failed
    if !success || duration_ns > 1_000_000 {
        if let Some(logger) = get_logger() {
            let level = if success { "debug" } else { "warn" };
            let message = format!(
                "syscall {} took {}μs",
                syscall_name,
                duration_ns / 1000
            );

            match level {
                "debug" => logger.debug("syscall", &message),
                "warn" => logger.warn("syscall", &message),
                _ => logger.info("syscall", &message),
            }
        }
    }
}

/// Record a context switch
#[inline]
pub fn record_context_switch(from_pid: u64, to_pid: u64) {
    if let Some(registry) = get_registry() {
        registry.counter_inc("context_switches_total");
    }

    if let Some(logger) = get_logger() {
        logger.debug(
            "scheduler",
            &format!("context switch: PID {} -> PID {}", from_pid, to_pid),
        );
    }
}

/// Record memory allocation
#[inline]
pub fn record_memory_allocation(size_bytes: u64) {
    if let Some(registry) = get_registry() {
        registry.counter_inc_by("memory_allocations_total", 1);
        registry.histogram_observe("memory_allocation_bytes", size_bytes);
    }
}

/// Record I/O operation
#[inline]
pub fn record_io_operation(operation: &str, device: &str, bytes: u64, duration_ns: u64) {
    if let Some(registry) = get_registry() {
        registry.counter_inc("io_operations_total");
        registry.histogram_observe("io_duration_ms", duration_ns / 1_000_000);
        registry.histogram_observe("io_bytes", bytes);
    }

    if let Some(logger) = get_logger() {
        logger.debug(
            "io",
            &format!(
                "{} on {}: {} bytes in {}μs",
                operation,
                device,
                bytes,
                duration_ns / 1000
            ),
        );
    }
}

/// Record network operation
#[inline]
pub fn record_network_operation(_operation: &str, bytes: u64, duration_ns: u64) {
    if let Some(registry) = get_registry() {
        registry.counter_inc("network_operations_total");
        registry.histogram_observe("network_duration_ms", duration_ns / 1_000_000);
        registry.histogram_observe("network_bytes", bytes);
    }
}

/// Get observability statistics
pub fn get_observability_stats() -> ObservabilityStats {
    let mut stats = ObservabilityStats::default();

    if let Some(tracer) = get_tracer() {
        let tracer_stats = tracer.get_stats();
        stats.tracer_total_spans = tracer_stats.total_spans;
        stats.tracer_active_spans = tracer_stats.active_spans;
        stats.tracer_dropped_spans = tracer_stats.dropped_spans;
    }

    if let Some(registry) = get_registry() {
        stats.metrics_count = registry.metric_count();
    }

    if let Some(logger) = get_logger() {
        let logger_stats = logger.get_stats();
        stats.logger_total_logs = logger_stats.total_logs;
        stats.logger_current_entries = logger_stats.current_entries;
        stats.logger_dropped_logs = logger_stats.dropped_logs;
    }

    stats
}

/// Observability statistics
#[derive(Debug, Default, Clone, Copy)]
pub struct ObservabilityStats {
    /// Tracer total spans
    pub tracer_total_spans: u64,
    /// Tracer active spans
    pub tracer_active_spans: usize,
    /// Tracer dropped spans
    pub tracer_dropped_spans: u64,
    /// Metrics count
    pub metrics_count: usize,
    /// Logger total logs
    pub logger_total_logs: usize,
    /// Logger current entries
    pub logger_current_entries: usize,
    /// Logger dropped logs
    pub logger_dropped_logs: usize,
}

/// Performance guard for timing operations
pub struct PerformanceGuard {
    /// Component name
    component: String,
    /// Operation name
    operation: String,
    /// Start time
    start_time: u64,
}

impl PerformanceGuard {
    /// Create a new performance guard
    #[inline]
    pub fn new(component: &str, operation: &str) -> Self {
        Self {
            component: component.to_string(),
            operation: operation.to_string(),
            start_time: crate::subsystems::time::hrtime_nanos(),
        }
    }

    /// Complete the guard and record duration
    #[inline]
    pub fn complete(self) {
        let duration = crate::subsystems::time::hrtime_nanos().saturating_sub(self.start_time);

        if let Some(logger) = get_logger() {
            logger.debug(
                &self.component,
                &format!("{} completed in {}μs", self.operation, duration / 1000),
            );
        }

        if let Some(registry) = get_registry() {
            registry.histogram_observe(
                &format!("{}_duration_ms", self.operation),
                duration / 1_000_000,
            );
        }
    }
}

impl Drop for PerformanceGuard {
    fn drop(&mut self) {
        let duration = crate::subsystems::time::hrtime_nanos().saturating_sub(self.start_time);

        if let Some(logger) = get_logger() {
            logger.debug(
                &self.component,
                &format!("{} completed in {}μs", self.operation, duration / 1000),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instrumentation_scope() {
        let scope = InstrumentationScope::new("test", "test_operation");
        scope.add_attribute("test_key", "test_value");
        // Scope ends when dropped
    }

    #[test]
    fn test_performance_guard() {
        let guard = PerformanceGuard::new("test", "test_op");
        // Simulate some work
        let _ = 1 + 1;
        drop(guard); // Records duration
    }
}
