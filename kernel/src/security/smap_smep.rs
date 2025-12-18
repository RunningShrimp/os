// Supervisor Mode Access Prevention (SMAP) and Supervisor Mode Execution Prevention (SMEP)
//
// This module implements SMAP and SMEP security features that prevent the kernel
// from accidentally accessing or executing user-space memory, which helps mitigate
// privilege escalation vulnerabilities.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;

use crate::arch;
// TODO: Implement X86Feature and X86Cpu in arch module
// use crate::arch::{self, X86Feature, X86Cpu};
use crate::types::stubs::VirtAddr;

/// SMAP/SMEP configuration
#[derive(Debug, Clone)]
pub struct SmapSmepConfig {
    /// Whether SMAP is enabled
    pub smap_enabled: bool,
    /// Whether SMEP is enabled
    pub smep_enabled: bool,
    /// Whether to enable strict checking (always enforce)
    pub strict_mode: bool,
    /// Whether to log violations
    pub log_violations: bool,
    /// Whether to kill violating processes
    pub kill_violations: bool,
    /// Whether to allow kernel override with AC flag
    pub allow_override: bool,
    /// Threshold for violations before killing process
    pub violation_threshold: u32,
}

impl Default for SmapSmepConfig {
    fn default() -> Self {
        Self {
            smap_enabled: true,
            smep_enabled: true,
            strict_mode: false,
            log_violations: true,
            kill_violations: false,
            allow_override: true,
            violation_threshold: 10,
        }
    }
}

/// Types of SMAP/SMEP violations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViolationType {
    /// SMAP violation: kernel accessed user memory
    SmapViolation,
    /// SMEP violation: kernel executed user memory
    SmepViolation,
    /// Combined violation
    BothViolation,
}

/// Severity levels for violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// SMAP/SMEP violation information
#[derive(Debug, Clone)]
pub struct SmapSmepViolation {
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// Type of violation
    pub violation_type: ViolationType,
    /// Address that caused the violation
    pub faulting_address: VirtAddr,
    /// Instruction pointer
    pub instruction_pointer: VirtAddr,
    /// Stack trace (if available)
    pub stack_trace: Vec<VirtAddr>,
    /// Severity level
    pub severity: ViolationSeverity,
    /// Timestamp
    pub timestamp: u64,
    /// Whether process was killed
    pub process_killed: bool,
}

/// SMAP/SMEP statistics
#[derive(Debug, Default, Clone)]
pub struct SmapSmepStats {
    pub events_processed: u64,
    /// Number of SMAP violations
    pub smap_violations: u64,
    /// Number of SMEP violations
    pub smep_violations: u64,
    /// Number of processes killed due to violations
    pub processes_killed: u64,
    /// Number of successful overrides (using AC flag)
    pub successful_overrides: u64,
    /// Total violations by process
    pub violations_by_process: BTreeMap<u64, u32>,
    /// Violations by severity
    pub violations_by_severity: BTreeMap<ViolationSeverity, u64>,
}

/// Per-process SMAP/SMEP state
#[derive(Debug, Clone)]
pub struct ProcessSmapSmepState {
    /// Process ID
    pub pid: u64,
    /// Violation count
    pub violation_count: u32,
    /// Last violation timestamp
    pub last_violation: Option<u64>,
    /// Whether process is marked for termination
    pub marked_for_termination: bool,
    /// Allowed memory regions (for legitimate cross-space access)
    pub allowed_regions: Vec<(VirtAddr, usize)>,
}

/// SMAP/SMEP subsystem
pub struct SmapSmepSubsystem {
    /// Configuration
    config: SmapSmepConfig,
    /// Per-process state
    process_states: BTreeMap<u64, ProcessSmapSmepState>,
    /// Violation history
    violation_history: Vec<SmapSmepViolation>,
    /// Statistics
    stats: Arc<Mutex<SmapSmepStats>>,
    /// Whether hardware supports SMAP/SMEP
    hardware_supported: bool,
    /// Current SMAP state (enabled/disabled)
    smap_current_state: AtomicBool,
    /// Current SMEP state (enabled/disabled)
    smep_current_state: AtomicBool,
}

impl SmapSmepSubsystem {
    /// Create a new SMAP/SMEP subsystem
    pub fn new(config: SmapSmepConfig) -> Self {
        let hardware_supported = Self::check_hardware_support();

        Self {
            config,
            process_states: BTreeMap::new(),
            violation_history: Vec::new(),
            stats: Arc::new(Mutex::new(SmapSmepStats::default())),
            hardware_supported,
            smap_current_state: AtomicBool::new(false),
            smep_current_state: AtomicBool::new(false),
        }
    }

    /// Check if hardware supports SMAP/SMEP
    fn check_hardware_support() -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            X86Cpu::has_feature(X86Feature::SMAP) && X86Cpu::has_feature(X86Feature::SMEP)
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            // For non-x86 architectures, we'll emulate SMAP/SMEP in software
            true
        }
    }

    /// Initialize SMAP/SMEP subsystem
    pub fn init(&mut self) -> Result<(), &'static str> {
        if !self.hardware_supported {
            return Err("Hardware does not support SMAP/SMEP");
        }

        // Enable SMAP if configured
        if self.config.smap_enabled {
            self.enable_smap()?;
        }

        // Enable SMEP if configured
        if self.config.smep_enabled {
            self.enable_smep()?;
        }

        Ok(())
    }

    /// Enable SMAP
    pub fn enable_smap(&self) -> Result<(), &'static str> {
        if !self.hardware_supported {
            return Err("Hardware does not support SMAP");
        }

        #[cfg(target_arch = "x86_64")]
        {
            // Enable SMAP by setting CR4.SMAP
            unsafe {
                let mut cr4: u64;
                core::arch::asm!("mov {}, cr4", out(reg) cr4);
                cr4 |= 1 << 21; // Set SMAP bit
                core::arch::asm!("mov cr4, {}", in(reg) cr4);
            }
        }

        self.smap_current_state.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Disable SMAP
    pub fn disable_smap(&self) -> Result<(), &'static str> {
        if !self.hardware_supported {
            return Err("Hardware does not support SMAP");
        }

        #[cfg(target_arch = "x86_64")]
        {
            // Disable SMAP by clearing CR4.SMAP
            unsafe {
                let mut cr4: u64;
                core::arch::asm!("mov {}, cr4", out(reg) cr4);
                cr4 &= !(1 << 21); // Clear SMAP bit
                core::arch::asm!("mov cr4, {}", in(reg) cr4);
            }
        }

        self.smap_current_state.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Enable SMEP
    pub fn enable_smep(&self) -> Result<(), &'static str> {
        if !self.hardware_supported {
            return Err("Hardware does not support SMEP");
        }

        #[cfg(target_arch = "x86_64")]
        {
            // Enable SMEP by setting CR4.SMEP
            unsafe {
                let mut cr4: u64;
                core::arch::asm!("mov {}, cr4", out(reg) cr4);
                cr4 |= 1 << 20; // Set SMEP bit
                core::arch::asm!("mov cr4, {}", in(reg) cr4);
            }
        }

        self.smep_current_state.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Disable SMEP
    pub fn disable_smep(&self) -> Result<(), &'static str> {
        if !self.hardware_supported {
            return Err("Hardware does not support SMEP");
        }

        #[cfg(target_arch = "x86_64")]
        {
            // Disable SMEP by clearing CR4.SMEP
            unsafe {
                let mut cr4: u64;
                core::arch::asm!("mov {}, cr4", out(reg) cr4);
                cr4 &= !(1 << 20); // Clear SMEP bit
                core::arch::asm!("mov cr4, {}", in(reg) cr4);
            }
        }

        self.smep_current_state.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Initialize SMAP/SMEP for a process
    pub fn init_process(&mut self, pid: u64) {
        let process_state = ProcessSmapSmepState {
            pid,
            violation_count: 0,
            last_violation: None,
            marked_for_termination: false,
            allowed_regions: Vec::new(),
        };

        self.process_states.insert(pid, process_state);
    }

    /// Handle a SMAP/SMEP violation
    pub fn handle_violation(
        &mut self,
        pid: u64,
        tid: u64,
        violation_type: ViolationType,
        faulting_address: VirtAddr,
        instruction_pointer: VirtAddr,
        stack_trace: Option<Vec<VirtAddr>>,
    ) -> bool {
        // Check if this is in allowed regions
        if let Some(process_state) = self.process_states.get(&pid) {
            if self.is_address_allowed(&process_state.allowed_regions, faulting_address) {
                // This is allowed access, don't treat as violation
                return false;
            }
        }

        let severity = self.determine_severity(&violation_type, faulting_address);
        let timestamp = self.get_timestamp();

        let violation = SmapSmepViolation {
            pid,
            tid,
            violation_type,
            faulting_address,
            instruction_pointer,
            stack_trace: stack_trace.unwrap_or_default(),
            severity,
            timestamp,
            process_killed: false,
        };

        // Update process state
        let should_kill = if let Some(process_state) = self.process_states.get_mut(&pid) {
            process_state.violation_count += 1;
            process_state.last_violation = Some(timestamp);

            // Extract needed data before checking kill condition
            let violation_count = process_state.violation_count;
            let is_critical = severity == ViolationSeverity::Critical;
            let strict_mode = self.config.strict_mode;
            let kill_violations = self.config.kill_violations;
            let violation_threshold = self.config.violation_threshold;

            // Check if we should kill the process using extracted data
            let should_kill = if strict_mode {
                true
            } else if kill_violations {
                violation_count >= violation_threshold
            } else {
                is_critical
            };

            if should_kill {
                process_state.marked_for_termination = true;
            }
            should_kill
        } else {
            false
        };

        // 释放借用后再调用kill_process
        if should_kill {
            self.kill_process(pid);
        }

        // Update statistics
        {
                let mut stats = self.stats.lock();
                match violation_type {
                    ViolationType::SmapViolation => stats.smap_violations += 1,
                    ViolationType::SmepViolation => stats.smep_violations += 1,
                    ViolationType::BothViolation => {
                        stats.smap_violations += 1;
                        stats.smep_violations += 1;
                    }
                }

                *stats.violations_by_process.entry(pid).or_insert(0) += 1;
                *stats.violations_by_severity.entry(severity).or_insert(0) += 1;

                if should_kill {
                    stats.processes_killed += 1;
                }
        }

        // Log violation
        if self.config.log_violations {
            self.log_violation(&violation);
        }

        self.violation_history.push(violation);
        should_kill
    }

    /// Check if address is in allowed regions
    fn is_address_allowed(&self, allowed_regions: &[(VirtAddr, usize)], addr: VirtAddr) -> bool {
        allowed_regions.iter().any(|(base, size)| {
            let start = base.as_usize();
            let end = start + *size;
            let a = addr.as_usize();
            a >= start && a < end
        })
    }

    /// Determine severity of violation
    fn determine_severity(&self, violation_type: &ViolationType, addr: VirtAddr) -> ViolationSeverity {
        match violation_type {
            ViolationType::SmepViolation => ViolationSeverity::Critical,
            ViolationType::BothViolation => ViolationSeverity::Critical,
            ViolationType::SmapViolation => {
                // Determine severity based on address range
                if addr.as_usize() < 0x1000 {
                    ViolationSeverity::High
                } else if addr.as_usize() < 0x100000 {
                    ViolationSeverity::Medium
                } else {
                    ViolationSeverity::Low
                }
            }
        }
    }

    /// Check if process should be killed
    fn should_kill_process(&self, process_state: &ProcessSmapSmepState, severity: &ViolationSeverity) -> bool {
        if self.config.strict_mode {
            return true;
        }

        if self.config.kill_violations {
            return process_state.violation_count >= self.config.violation_threshold;
        }

        *severity == ViolationSeverity::Critical
    }

    /// Kill a process
    fn kill_process(&self, pid: u64) {
        // Send SIGKILL to the process
        crate::syscalls::signal::kill_process(pid as usize, 9);
    }

    /// Log a violation
    fn log_violation(&self, violation: &SmapSmepViolation) {
        let violation_type_str = match violation.violation_type {
            ViolationType::SmapViolation => "SMAP",
            ViolationType::SmepViolation => "SMEP",
            ViolationType::BothViolation => "SMAP/SMEP",
        };

        crate::println!(
            "[SMAP/SMEP] {} violation in process {}: addr={:#x}, ip={:#x}, severity={:?}",
            violation_type_str, violation.pid, violation.faulting_address.as_usize(),
            violation.instruction_pointer.as_usize(), violation.severity
        );

        if !violation.stack_trace.is_empty() {
            crate::println!("[SMAP/SMEP] Stack trace:");
            for (i, addr) in violation.stack_trace.iter().enumerate().take(8) {
                crate::println!("  {}: {:#x}", i, addr.as_usize());
            }
        }
    }

    /// Get current timestamp
    fn get_timestamp(&self) -> u64 {
        crate::time::get_timestamp()
    }

    /// Add allowed memory region for cross-space access
    pub fn add_allowed_region(&mut self, pid: u64, base: VirtAddr, size: usize) {
        if let Some(process_state) = self.process_states.get_mut(&pid) {
            process_state.allowed_regions.push((base, size));
        }
    }

    /// Remove allowed memory region
    pub fn remove_allowed_region(&mut self, pid: u64, base: VirtAddr, size: usize) {
        if let Some(process_state) = self.process_states.get_mut(&pid) {
            process_state.allowed_regions.retain(|(b, s)| !(*b == base && *s == size));
        }
    }

    /// Cleanup process state
    pub fn cleanup_process(&mut self, pid: u64) {
        self.process_states.remove(&pid);
    }

    /// Get configuration
    pub fn config(&self) -> &SmapSmepConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: SmapSmepConfig) {
        self.config = config;

        // Apply new settings
        if self.config.smap_enabled && !self.smap_current_state.load(Ordering::SeqCst) {
            let _ = self.enable_smap();
        } else if !self.config.smap_enabled && self.smap_current_state.load(Ordering::SeqCst) {
            let _ = self.disable_smap();
        }

        if self.config.smep_enabled && !self.smep_current_state.load(Ordering::SeqCst) {
            let _ = self.enable_smep();
        } else if !self.config.smep_enabled && self.smep_current_state.load(Ordering::SeqCst) {
            let _ = self.disable_smep();
        }
    }

    /// Get current SMAP state
    pub fn smap_enabled(&self) -> bool {
        self.smap_current_state.load(Ordering::SeqCst)
    }

    /// Get current SMEP state
    pub fn smep_enabled(&self) -> bool {
        self.smep_current_state.load(Ordering::SeqCst)
    }

    /// Get statistics
    pub fn get_stats(&self) -> SmapSmepStats {
        self.stats.lock().clone()
    }

    /// Get violation history
    pub fn get_violation_history(&self) -> &[SmapSmepViolation] {
        &self.violation_history
    }

    /// Clear violation history
    pub fn clear_violation_history(&mut self) {
        self.violation_history.clear();
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        *self.stats.lock() = SmapSmepStats::default();
    }
}

/// High-level SMAP/SMEP interface functions

/// Initialize SMAP/SMEP subsystem
pub fn init_smap_smep(config: SmapSmepConfig) -> Result<(), &'static str> {
    *crate::security::SMAP_SMEP.lock() = Some(SmapSmepSubsystem::new(config));
    Ok(())
}

/// Check if SMAP is enabled
pub fn is_smap_enabled() -> bool {
    let guard = crate::security::SMAP_SMEP.lock();
    guard.as_ref().map(|s| s.smap_enabled()).unwrap_or(false)
}

/// Check if SMEP is enabled
pub fn is_smep_enabled() -> bool {
    let guard = crate::security::SMAP_SMEP.lock();
    guard.as_ref().map(|s| s.smep_enabled()).unwrap_or(false)
}

/// Temporarily disable SMAP for legitimate kernel access
pub fn disable_smap_temporarily() -> Result<(), &'static str> {
    let mut guard = crate::security::SMAP_SMEP.lock();
    if let Some(ref mut s) = *guard {
        s.disable_smap()
    } else {
        Ok(())
    }
}

/// Re-enable SMAP after temporary disable
pub fn enable_smap_temporarily() -> Result<(), &'static str> {
    let mut guard = crate::security::SMAP_SMEP.lock();
    if let Some(ref mut s) = *guard {
        s.enable_smap()
    } else {
        Ok(())
    }
}

/// Add allowed memory region for process
pub fn add_smap_allowed_region(pid: u64, base: VirtAddr, size: usize) {
    if let Some(ref mut s) = *crate::security::SMAP_SMEP.lock() {
        s.add_allowed_region(pid, base, size);
    }
}

/// Get SMAP/SMEP statistics
pub fn get_smap_smep_statistics() -> SmapSmepStats {
    let guard = crate::security::SMAP_SMEP.lock();
    guard.as_ref().map(|s| s.get_stats()).unwrap_or_default()
}

/// Update SMAP/SMEP configuration
pub fn update_smap_smep_config(config: SmapSmepConfig) -> Result<(), &'static str> {
    let mut guard = crate::security::SMAP_SMEP.lock();
    if let Some(ref mut s) = *guard {
        s.update_config(config);
    } else {
        *guard = Some(SmapSmepSubsystem::new(config));
    }
    Ok(())
}

/// Initialize SMAP/SMEP (alias for compatibility)
pub fn initialize_smap_smep(config: SmapSmepConfig) -> Result<(), &'static str> {
    init_smap_smep(config)
}

/// Cleanup SMAP/SMEP (placeholder implementation)
pub fn cleanup_smap_smep() -> Result<(), &'static str> {
    // TODO: Implement cleanup
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smap_smep_config_default() {
        let config = SmapSmepConfig::default();
        assert!(config.smap_enabled);
        assert!(config.smep_enabled);
        assert!(!config.strict_mode);
        assert!(config.log_violations);
        assert!(!config.kill_violations);
        assert_eq!(config.violation_threshold, 10);
    }

    #[test]
    fn test_violation_severity() {
        let config = SmapSmepConfig::default();
        let subsystem = SmapSmepSubsystem::new(config);

        let smap_severity = subsystem.determine_severity(
            &ViolationType::SmapViolation,
            VirtAddr::new(0x1000),
        );
        assert_eq!(smap_severity, ViolationSeverity::High);

        let smep_severity = subsystem.determine_severity(
            &ViolationType::SmepViolation,
            VirtAddr::new(0x1000),
        );
        assert_eq!(smep_severity, ViolationSeverity::Critical);
    }

    #[test]
    fn test_process_state() {
        let state = ProcessSmapSmepState {
            pid: 1234,
            violation_count: 0,
            last_violation: None,
            marked_for_termination: false,
            allowed_regions: Vec::new(),
        };

        assert_eq!(state.pid, 1234);
        assert_eq!(state.violation_count, 0);
        assert!(!state.marked_for_termination);
    }

    #[test]
    fn test_violation_creation() {
        let violation = SmapSmepViolation {
            pid: 1234,
            tid: 5678,
            violation_type: ViolationType::SmapViolation,
            faulting_address: VirtAddr::new(0x1000),
            instruction_pointer: VirtAddr::new(0x2000),
            stack_trace: vec![VirtAddr::new(0x3000)],
            severity: ViolationSeverity::Medium,
            timestamp: 123456789,
            process_killed: false,
        };

        assert_eq!(violation.pid, 1234);
        assert_eq!(violation.tid, 5678);
        assert_eq!(violation.violation_type, ViolationType::SmapViolation);
        assert_eq!(violation.severity, ViolationSeverity::Medium);
    }
}
