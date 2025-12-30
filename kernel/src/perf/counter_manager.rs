//! Performance Counter Manager
//!
//! This module provides a unified interface for managing both hardware and software
//! performance counters. It handles registration, data collection, aggregation, and
//! export to /proc or /sys/fs interfaces with minimal overhead.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::hardware::{HardwareCounterManager, HardwareCounterType};
use super::software::{SoftwareCounterManager, SoftwareCounterType};
use crate::cpu::NCPU;

/// Counter ID type
pub type CounterId = u64;

/// Counter category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CounterCategory {
    Hardware,
    Software,
    Custom,
}

/// Counter trait for dynamic counter registration
pub trait Counter: Send + Sync {
    /// Get counter name
    fn name(&self) -> &str;

    /// Get counter description
    fn description(&self) -> &str;

    /// Get counter value
    fn value(&self) -> u64;

    /// Increment counter
    fn increment(&self, delta: u64);

    /// Reset counter
    fn reset(&self);

    /// Get counter category
    fn category(&self) -> CounterCategory;
}

/// Simple counter implementation
#[derive(Debug)]
pub struct SimpleCounter {
    name: String,
    description: String,
    value: AtomicU64,
    category: CounterCategory,
}

impl SimpleCounter {
    /// Create new simple counter
    pub fn new(name: String, description: String, category: CounterCategory) -> Self {
        Self {
            name,
            description,
            value: AtomicU64::new(0),
            category,
        }
    }

    /// Create new hardware counter
    pub fn new_hardware(name: String, description: String) -> Self {
        Self::new(name, description, CounterCategory::Hardware)
    }

    /// Create new software counter
    pub fn new_software(name: String, description: String) -> Self {
        Self::new(name, description, CounterCategory::Software)
    }

    /// Create new custom counter
    pub fn new_custom(name: String, description: String) -> Self {
        Self::new(name, description, CounterCategory::Custom)
    }
}

impl Counter for SimpleCounter {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn value(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    fn increment(&self, delta: u64) {
        self.value.fetch_add(delta, Ordering::Relaxed);
    }

    fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }

    fn category(&self) -> CounterCategory {
        self.category
    }
}

/// Per-CPU counter snapshot
#[derive(Debug, Clone)]
pub struct PerCpuCounterSnapshot {
    /// CPU ID
    pub cpu_id: usize,
    /// Hardware counter values
    pub hardware_counters: BTreeMap<String, u64>,
    /// Software counter values
    pub software_counters: BTreeMap<String, u64>,
    /// Timestamp
    pub timestamp: u64,
}

impl PerCpuCounterSnapshot {
    /// Create new per-CPU snapshot
    pub fn new(
        cpu_id: usize,
        hardware_counters: BTreeMap<String, u64>,
        software_counters: BTreeMap<String, u64>,
    ) -> Self {
        Self {
            cpu_id,
            hardware_counters,
            software_counters,
            timestamp: nos_api::event::get_time_ns(),
        }
    }
}

/// Global counter snapshot
#[derive(Debug, Clone)]
pub struct CounterSnapshot {
    /// Per-CPU snapshots
    pub per_cpu_snapshots: Vec<PerCpuCounterSnapshot>,
    /// Aggregated hardware counters
    pub aggregated_hardware: BTreeMap<String, u64>,
    /// Aggregated software counters
    pub aggregated_software: BTreeMap<String, u64>,
    /// Custom counter values
    pub custom_counters: BTreeMap<String, u64>,
    /// Timestamp
    pub timestamp: u64,
}

impl CounterSnapshot {
    /// Create new counter snapshot
    pub fn new(
        per_cpu_snapshots: Vec<PerCpuCounterSnapshot>,
        aggregated_hardware: BTreeMap<String, u64>,
        aggregated_software: BTreeMap<String, u64>,
        custom_counters: BTreeMap<String, u64>,
    ) -> Self {
        Self {
            per_cpu_snapshots,
            aggregated_hardware,
            aggregated_software,
            custom_counters,
            timestamp: nos_api::event::get_time_ns(),
        }
    }
}

/// Performance counter manager
pub struct PerformanceCounterManager {
    /// Hardware counter manager
    hardware_manager: HardwareCounterManager,
    /// Software counter manager
    software_manager: SoftwareCounterManager,
    /// Custom counters
    custom_counters: spin::Mutex<BTreeMap<String, Arc<dyn Counter>>>,
    /// Next counter ID
    next_counter_id: AtomicU64,
    /// Counter name to ID mapping
    counter_name_to_id: spin::Mutex<BTreeMap<String, CounterId>>,
    /// Is enabled
    enabled: AtomicU64,
}

impl PerformanceCounterManager {
    /// Create new performance counter manager
    pub fn new() -> Self {
        Self {
            hardware_manager: HardwareCounterManager::new(),
            software_manager: SoftwareCounterManager::new(),
            custom_counters: spin::Mutex::new(BTreeMap::new()),
            next_counter_id: AtomicU64::new(1),
            counter_name_to_id: spin::Mutex::new(BTreeMap::new()),
            enabled: AtomicU64::new(1),
        }
    }

    /// Register custom counter
    pub fn register_counter(&self, counter: Arc<dyn Counter>) -> Result<CounterId, String> {
        let name = counter.name().to_string();

        // Check if counter already exists
        {
            let name_to_id = self.counter_name_to_id.lock();
            if name_to_id.contains_key(&name) {
                return Err(format!("Counter {} already registered", name));
            }
        }

        // Assign new counter ID
        let counter_id = self.next_counter_id.fetch_add(1, Ordering::SeqCst);

        // Register counter
        {
            let mut counters = self.custom_counters.lock();
            counters.insert(name.clone(), counter.clone());
        }

        // Update name to ID mapping
        {
            let mut name_to_id = self.counter_name_to_id.lock();
            name_to_id.insert(name, counter_id);
        }

        Ok(counter_id)
    }

    /// Unregister counter
    pub fn unregister_counter(&self, counter_id: CounterId) -> Result<(), String> {
        // Find counter by ID
        let name_to_remove = {
            let name_to_id = self.counter_name_to_id.lock();
            let mut found_name = None;
            for (name, &id) in name_to_id.iter() {
                if id == counter_id {
                    found_name = Some(name.clone());
                    break;
                }
            }
            found_name
        };

        if let Some(name) = name_to_remove {
            // Remove from counters
            {
                let mut counters = self.custom_counters.lock();
                counters.remove(&name);
            }

            // Remove from mapping
            {
                let mut name_to_id = self.counter_name_to_id.lock();
                name_to_id.remove(&name);
            }

            Ok(())
        } else {
            Err(format!("Counter ID {} not found", counter_id))
        }
    }

    /// Increment hardware counter
    #[inline]
    pub fn increment_hardware(&self, cpu_id: usize, counter_type: HardwareCounterType, delta: u64) {
        if self.is_enabled() {
            self.hardware_manager.increment(cpu_id, counter_type, delta);
        }
    }

    /// Increment software counter
    #[inline]
    pub fn increment_software(&self, cpu_id: usize, counter_type: SoftwareCounterType, delta: u64) {
        if self.is_enabled() {
            self.software_manager.increment(cpu_id, counter_type, delta);
        }
    }

    /// Increment custom counter
    #[inline]
    pub fn increment_custom(&self, name: &str, delta: u64) {
        if !self.is_enabled() {
            return;
        }

        let counters = self.custom_counters.lock();
        if let Some(counter) = counters.get(name) {
            counter.increment(delta);
        }
    }

    /// Get hardware counter value
    pub fn get_hardware_counter(&self, cpu_id: usize, counter_type: HardwareCounterType) -> u64 {
        self.hardware_manager.get_cpu_counter(cpu_id, counter_type)
    }

    /// Get software counter value
    pub fn get_software_counter(&self, cpu_id: usize, counter_type: SoftwareCounterType) -> u64 {
        self.software_manager.get_cpu_counter(cpu_id, counter_type)
    }

    /// Get custom counter value
    pub fn get_custom_counter(&self, name: &str) -> Option<u64> {
        let counters = self.custom_counters.lock();
        counters.get(name).map(|c| c.value())
    }

    /// Create snapshot of all counters
    pub fn snapshot(&self) -> CounterSnapshot {
        let mut per_cpu_snapshots = Vec::new();
        let mut aggregated_hardware = BTreeMap::new();
        let mut aggregated_software = BTreeMap::new();

        // Collect per-CPU snapshots
        for cpu_id in 0..NCPU {
            let hw_counters = self.hardware_manager.get_cpu_counters(cpu_id);
            let sw_counters = self.software_manager.get_cpu_counters(cpu_id);

            per_cpu_snapshots.push(PerCpuCounterSnapshot::new(cpu_id, hw_counters, sw_counters));
        }

        // Aggregate hardware counters
        aggregated_hardware = self.hardware_manager.get_all_counters();

        // Aggregate software counters
        aggregated_software = self.software_manager.get_all_counters();

        // Collect custom counters
        let custom_counters = {
            let counters = self.custom_counters.lock();
            counters
                .iter()
                .map(|(name, counter)| (name.clone(), counter.value()))
                .collect()
        };

        CounterSnapshot::new(
            per_cpu_snapshots,
            aggregated_hardware,
            aggregated_software,
            custom_counters,
        )
    }

    /// Reset all counters
    pub fn reset_all(&self) {
        self.hardware_manager.reset_all_counters();
        self.software_manager.reset_all_counters();

        let counters = self.custom_counters.lock();
        for counter in counters.values() {
            counter.reset();
        }
    }

    /// Reset counters for specific CPU
    pub fn reset_cpu(&self, cpu_id: usize) {
        self.hardware_manager.reset_cpu_counters(cpu_id);
        self.software_manager.reset_cpu_counters(cpu_id);
    }

    /// Enable/disable counters
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled as u64, Ordering::Release);
    }

    /// Check if counters are enabled
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire) != 0
    }

    /// Export counters to /proc format
    pub fn export_to_proc(&self) -> String {
        let snapshot = self.snapshot();
        let mut output = String::new();

        output.push_str("# Performance Counters\n");
        output.push_str(&format!("# Timestamp: {}\n", snapshot.timestamp));
        output.push_str("\n");

        // Hardware counters
        output.push_str("## Hardware Counters (Aggregated)\n");
        for (name, value) in &snapshot.aggregated_hardware {
            output.push_str(&format!("hw_{} {}\n", name, value));
        }
        output.push_str("\n");

        // Software counters
        output.push_str("## Software Counters (Aggregated)\n");
        for (name, value) in &snapshot.aggregated_software {
            output.push_str(&format!("sw_{} {}\n", name, value));
        }
        output.push_str("\n");

        // Custom counters
        output.push_str("## Custom Counters\n");
        for (name, value) in &snapshot.custom_counters {
            output.push_str(&format!("custom_{} {}\n", name, value));
        }
        output.push_str("\n");

        // Per-CPU breakdown
        output.push_str("## Per-CPU Breakdown\n");
        for cpu_snapshot in &snapshot.per_cpu_snapshots {
            output.push_str(&format!("\n### CPU {}\n", cpu_snapshot.cpu_id));

            output.push_str("Hardware:\n");
            for (name, value) in &cpu_snapshot.hardware_counters {
                output.push_str(&format!("  hw_{} {}\n", name, value));
            }

            output.push_str("Software:\n");
            for (name, value) in &cpu_snapshot.software_counters {
                output.push_str(&format!("  sw_{} {}\n", name, value));
            }
        }

        output
    }

    /// Export counters to JSON format
    pub fn export_to_json(&self) -> String {
        let snapshot = self.snapshot();
        let mut output = String::new();

        output.push_str("{\n");
        output.push_str(&format!("  \"timestamp\": {},\n", snapshot.timestamp));
        output.push_str("  \"aggregated_hardware\": {\n");
        {
            let mut first = true;
            for (name, value) in &snapshot.aggregated_hardware {
                if !first {
                    output.push_str(",\n");
                }
                first = false;
                output.push_str(&format!("    \"{}\": {}", name, value));
            }
        }
        output.push_str("\n  },\n");

        output.push_str("  \"aggregated_software\": {\n");
        {
            let mut first = true;
            for (name, value) in &snapshot.aggregated_software {
                if !first {
                    output.push_str(",\n");
                }
                first = false;
                output.push_str(&format!("    \"{}\": {}", name, value));
            }
        }
        output.push_str("\n  },\n");

        output.push_str("  \"custom_counters\": {\n");
        {
            let mut first = true;
            for (name, value) in &snapshot.custom_counters {
                if !first {
                    output.push_str(",\n");
                }
                first = false;
                output.push_str(&format!("    \"{}\": {}", name, value));
            }
        }
        output.push_str("\n  },\n");

        output.push_str("  \"per_cpu\": [\n");
        {
            let mut first_cpu = true;
            for cpu_snapshot in &snapshot.per_cpu_snapshots {
                if !first_cpu {
                    output.push_str(",\n");
                }
                first_cpu = false;
                output.push_str(&format!("    {{\n      \"cpu\": {},\n", cpu_snapshot.cpu_id));
                output.push_str("      \"hardware\": {");
                {
                    let mut first = true;
                    for (name, value) in &cpu_snapshot.hardware_counters {
                        if !first {
                            output.push_str(", ");
                        }
                        first = false;
                        output.push_str(&format!("\"{}\": {}", name, value));
                    }
                }
                output.push_str("},\n");
                output.push_str("      \"software\": {");
                {
                    let mut first = true;
                    for (name, value) in &cpu_snapshot.software_counters {
                        if !first {
                            output.push_str(", ");
                        }
                        first = false;
                        output.push_str(&format!("\"{}\": {}", name, value));
                    }
                }
                output.push_str("}\n    }");
            }
        }
        output.push_str("\n  ]\n");
        output.push_str("}\n");

        output
    }

    /// Get statistics summary
    pub fn get_summary(&self) -> BTreeMap<String, String> {
        let mut summary = BTreeMap::new();
        let snapshot = self.snapshot();

        // Count total counters
        let hw_count = snapshot.aggregated_hardware.len();
        let sw_count = snapshot.aggregated_software.len();
        let custom_count = snapshot.custom_counters.len();
        let total_count = hw_count + sw_count + custom_count;

        summary.insert("total_counters".to_string(), total_count.to_string());
        summary.insert("hardware_counters".to_string(), hw_count.to_string());
        summary.insert("software_counters".to_string(), sw_count.to_string());
        summary.insert("custom_counters".to_string(), custom_count.to_string());
        summary.insert("cpu_count".to_string(), NCPU.to_string());
        summary.insert("enabled".to_string(), self.is_enabled().to_string());
        summary.insert("timestamp".to_string(), snapshot.timestamp.to_string());

        summary
    }

    /// Get hardware manager reference
    pub fn hardware_manager(&self) -> &HardwareCounterManager {
        &self.hardware_manager
    }

    /// Get software manager reference
    pub fn software_manager(&self) -> &SoftwareCounterManager {
        &self.software_manager
    }
}

impl Default for PerformanceCounterManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global performance counter manager
static mut GLOBAL_COUNTER_MANAGER: Option<PerformanceCounterManager> = None;
static COUNTER_MANAGER_INIT: spin::Mutex<bool> = spin::Mutex::new(false);

/// Initialize performance counter manager
pub fn init_counter_manager() {
    let mut is_init = COUNTER_MANAGER_INIT.lock();
    if *is_init {
        return;
    }

    unsafe {
        GLOBAL_COUNTER_MANAGER = Some(PerformanceCounterManager::new());
    }

    *is_init = true;
    log::info!("Performance counter manager initialized");
}

/// Get performance counter manager
pub fn get_counter_manager() -> &'static PerformanceCounterManager {
    unsafe {
        if GLOBAL_COUNTER_MANAGER.is_none() {
            init_counter_manager();
        }
        GLOBAL_COUNTER_MANAGER.as_ref().unwrap()
    }
}

/// Convenience function to create snapshot
pub fn create_snapshot() -> CounterSnapshot {
    get_counter_manager().snapshot()
}

/// Convenience function to export to /proc
pub fn export_counters() -> String {
    get_counter_manager().export_to_proc()
}

/// Convenience function to get summary
pub fn get_counters_summary() -> BTreeMap<String, String> {
    get_counter_manager().get_summary()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let manager = PerformanceCounterManager::new();
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_custom_counter_registration() {
        let manager = PerformanceCounterManager::new();
        let counter = Arc::new(SimpleCounter::new_custom(
            "test_counter".to_string(),
            "Test counter".to_string(),
        ));

        let result = manager.register_counter(counter);
        assert!(result.is_ok());

        let counter_id = result.unwrap();
        let value = manager.get_custom_counter("test_counter");
        assert_eq!(value, Some(0));
    }

    #[test]
    fn test_custom_counter_increment() {
        let manager = PerformanceCounterManager::new();
        let counter = Arc::new(SimpleCounter::new_custom(
            "test_counter".to_string(),
            "Test counter".to_string(),
        ));

        manager.register_counter(counter).unwrap();
        manager.increment_custom("test_counter", 42);

        let value = manager.get_custom_counter("test_counter");
        assert_eq!(value, Some(42));
    }

    #[test]
    fn test_snapshot() {
        let manager = PerformanceCounterManager::new();
        let snapshot = manager.snapshot();

        assert_eq!(snapshot.per_cpu_snapshots.len(), NCPU);
        assert!(!snapshot.aggregated_hardware.is_empty());
        assert!(!snapshot.aggregated_software.is_empty());
    }

    #[test]
    fn test_enable_disable() {
        let manager = PerformanceCounterManager::new();
        assert!(manager.is_enabled());

        manager.set_enabled(false);
        assert!(!manager.is_enabled());

        manager.set_enabled(true);
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_export_to_proc() {
        let manager = PerformanceCounterManager::new();
        let output = manager.export_to_proc();

        assert!(output.contains("# Performance Counters"));
        assert!(output.contains("## Hardware Counters"));
        assert!(output.contains("## Software Counters"));
        assert!(output.contains("## Per-CPU Breakdown"));
    }

    #[test]
    fn test_export_to_json() {
        let manager = PerformanceCounterManager::new();
        let output = manager.export_to_json();

        assert!(output.contains("\"timestamp\":"));
        assert!(output.contains("\"aggregated_hardware\""));
        assert!(output.contains("\"aggregated_software\""));
        assert!(output.contains("\"per_cpu\""));
    }

    #[test]
    fn test_summary() {
        let manager = PerformanceCounterManager::new();
        let summary = manager.get_summary();

        assert!(summary.contains_key("total_counters"));
        assert!(summary.contains_key("hardware_counters"));
        assert!(summary.contains_key("software_counters"));
        assert!(summary.contains_key("cpu_count"));
        assert!(summary.contains_key("enabled"));
    }

    #[test]
    fn test_unregister_counter() {
        let manager = PerformanceCounterManager::new();
        let counter = Arc::new(SimpleCounter::new_custom(
            "test_counter".to_string(),
            "Test counter".to_string(),
        ));

        let counter_id = manager.register_counter(counter).unwrap();
        let result = manager.unregister_counter(counter_id);
        assert!(result.is_ok());

        let value = manager.get_custom_counter("test_counter");
        assert_eq!(value, None);
    }
}
