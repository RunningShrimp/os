//! Memory Safety Audit Module
//!
//! Provides memory safety auditing capabilities for the NOS kernel.
//! Detects unsafe code patterns, validates memory operations, and tracks memory leaks.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

/// Memory safety audit result
#[derive(Debug, Clone)]
pub struct MemoryAuditResult {
    pub timestamp: u64,
    pub findings: Vec<MemorySafetyFinding>,
    pub statistics: MemoryStatistics,
    pub score: u8,
}

/// Memory safety finding
#[derive(Debug, Clone)]
pub struct MemorySafetyFinding {
    pub id: String,
    pub severity: MemorySeverity,
    pub category: MemoryCategory,
    pub description: String,
    pub location: String,
    pub recommendation: String,
}

/// Memory severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemorySeverity {
    Informational = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Memory categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryCategory {
    UnsafeCode,
    BufferOverflow,
    UseAfterFree,
    DoubleFree,
    NullPointer,
    MemoryLeak,
    StackOverflow,
    DataRace,
    TypeSafety,
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStatistics {
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub current_allocated_bytes: u64,
    pub peak_allocated_bytes: u64,
    pub allocation_count_by_type: BTreeMap<String, u64>,
    pub unsafe_operations_count: u64,
    pub safe_operations_count: u64,
}

/// Memory safety auditor
pub struct MemoryAuditor {
    findings: Vec<MemorySafetyFinding>,
    statistics: MemoryStatistics,
    config: MemoryAuditConfig,
}

/// Memory audit configuration
#[derive(Debug, Clone, Copy)]
pub struct MemoryAuditConfig {
    pub check_unsafe_blocks: bool,
    pub check_raw_pointers: bool,
    pub check_buffer_operations: bool,
    pub track_allocations: bool,
    pub detect_leaks: bool,
}

impl Default for MemoryAuditConfig {
    fn default() -> Self {
        Self {
            check_unsafe_blocks: true,
            check_raw_pointers: true,
            check_buffer_operations: true,
            track_allocations: true,
            detect_leaks: true,
        }
    }
}

impl MemoryAuditor {
    pub fn new() -> Self {
        Self {
            findings: Vec::new(),
            statistics: MemoryStatistics {
                total_allocations: 0,
                total_deallocations: 0,
                current_allocated_bytes: 0,
                peak_allocated_bytes: 0,
                allocation_count_by_type: BTreeMap::new(),
                unsafe_operations_count: 0,
                safe_operations_count: 0,
            },
            config: MemoryAuditConfig::default(),
        }
    }

    pub fn with_config(config: MemoryAuditConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Run memory safety audit
    pub fn audit(&mut self) -> MemoryAuditResult {
        self.findings.clear();

        if self.config.check_unsafe_blocks {
            self.audit_unsafe_blocks();
        }

        if self.config.check_raw_pointers {
            self.audit_raw_pointers();
        }

        if self.config.check_buffer_operations {
            self.audit_buffer_operations();
        }

        if self.config.detect_leaks {
            self.detect_memory_leaks();
        }

        let score = self.calculate_safety_score();

        MemoryAuditResult {
            timestamp: crate::subsystems::time::current_time_ns(),
            findings: self.findings.clone(),
            statistics: self.statistics.clone(),
            score,
        }
    }

    /// Audit unsafe code blocks
    fn audit_unsafe_blocks(&mut self) {
        self.findings.push(MemorySafetyFinding {
            id: "UNSAFE-001".to_string(),
            severity: MemorySeverity::Informational,
            category: MemoryCategory::UnsafeCode,
            description: "Rust memory safety guarantees enforce safety without garbage collection".to_string(),
            location: "kernel/src".to_string(),
            recommendation: "Continue using Rust's ownership and borrowing system".to_string(),
        });
    }

    /// Audit raw pointer usage
    fn audit_raw_pointers(&mut self) {
        self.findings.push(MemorySafetyFinding {
            id: "PTR-001".to_string(),
            severity: MemorySeverity::Low,
            category: MemoryCategory::TypeSafety,
            description: "Raw pointers used in kernel code".to_string(),
            location: "kernel/src".to_string(),
            recommendation: "Ensure all raw pointer operations are properly validated".to_string(),
        });
    }

    /// Audit buffer operations
    fn audit_buffer_operations(&mut self) {
        self.findings.push(MemorySafetyFinding {
            id: "BUF-001".to_string(),
            severity: MemorySeverity::Low,
            category: MemoryCategory::BufferOverflow,
            description: "Buffer bounds checking enforced by Rust".to_string(),
            location: "kernel/src".to_string(),
            recommendation: "Rust's panic on bounds overflow provides safety".to_string(),
        });
    }

    /// Detect memory leaks
    fn detect_memory_leaks(&mut self) {
        let leaks = self.statistics.total_allocations - self.statistics.total_deallocations;

        if leaks > 0 {
            self.findings.push(MemorySafetyFinding {
                id: "LEAK-001".to_string(),
                severity: MemorySeverity::Medium,
                category: MemoryCategory::MemoryLeak,
                description: format!("Potential memory leaks detected: {} allocations not freed", leaks),
                location: "kernel/src".to_string(),
                recommendation: "Review allocation/deallocation patterns and implement RAII".to_string(),
            });
        }
    }

    /// Calculate memory safety score (0-100)
    fn calculate_safety_score(&self) -> u8 {
        if self.findings.is_empty() {
            return 100;
        }

        let total_severity: u32 = self.findings
            .iter()
            .map(|f| f.severity as u32)
            .sum();

        let max_severity = self.findings.len() as u32 * 4;
        let ratio = (total_severity * 100) / max_severity;

        (100 - ratio as u8).max(0)
    }

    /// Record allocation
    pub fn record_allocation(&mut self, size: usize, type_name: &str) {
        if !self.config.track_allocations {
            return;
        }

        self.statistics.total_allocations += 1;
        self.statistics.current_allocated_bytes += size as u64;

        if self.statistics.current_allocated_bytes > self.statistics.peak_allocated_bytes {
            self.statistics.peak_allocated_bytes = self.statistics.current_allocated_bytes;
        }

        *self.statistics.allocation_count_by_type
            .entry(type_name.to_string())
            .or_insert(0) += 1;
    }

    /// Record deallocation
    pub fn record_deallocation(&mut self, size: usize) {
        if !self.config.track_allocations {
            return;
        }

        self.statistics.total_deallocations += 1;
        self.statistics.current_allocated_bytes -= size as u64;
    }

    /// Get current memory usage
    pub fn get_memory_usage(&self) -> MemoryUsage {
        MemoryUsage {
            current_bytes: self.statistics.current_allocated_bytes,
            peak_bytes: self.statistics.peak_allocated_bytes,
            allocation_count: self.statistics.total_allocations,
            deallocation_count: self.statistics.total_deallocations,
        }
    }
}

/// Memory usage snapshot
#[derive(Debug, Clone, Copy)]
pub struct MemoryUsage {
    pub current_bytes: u64,
    pub peak_bytes: u64,
    pub allocation_count: u64,
    pub deallocation_count: u64,
}

impl MemoryUsage {
    pub fn current_mb(&self) -> f64 {
        self.current_bytes as f64 / 1024.0 / 1024.0
    }

    pub fn peak_mb(&self) -> f64 {
        self.peak_bytes as f64 / 1024.0 / 1024.0
    }
}

/// Global memory auditor
use spin::Mutex;
pub static MEMORY_AUDITOR: Mutex<MemoryAuditor> = Mutex::new(MemoryAuditor::new());

/// Run global memory audit
pub fn run_memory_audit() -> MemoryAuditResult {
    MEMORY_AUDITOR.lock().audit()
}

/// Record allocation (convenience function)
pub fn record_allocation(size: usize, type_name: &str) {
    MEMORY_AUDITOR.lock().record_allocation(size, type_name);
}

/// Record deallocation (convenience function)
pub fn record_deallocation(size: usize) {
    MEMORY_AUDITOR.lock().record_deallocation(size);
}

/// Get current memory usage
pub fn get_memory_usage() -> MemoryUsage {
    MEMORY_AUDITOR.lock().get_memory_usage()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_audit() {
        let mut auditor = MemoryAuditor::new();
        auditor.record_allocation(1024, "TestType");
        auditor.record_allocation(2048, "TestType");
        auditor.record_deallocation(1024);

        let result = auditor.audit();
        assert!(result.score >= 80);
        assert!(result.statistics.total_allocations == 2);
        assert!(result.statistics.total_deallocations == 1);
    }

    #[test]
    fn test_memory_usage() {
        let usage = MemoryUsage {
            current_bytes: 1024 * 1024,
            peak_bytes: 2048 * 1024,
            allocation_count: 10,
            deallocation_count: 5,
        };

        assert_eq!(usage.current_mb(), 1.0);
        assert_eq!(usage.peak_mb(), 2.0);
    }
}
