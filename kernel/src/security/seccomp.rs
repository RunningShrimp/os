// seccomp (Secure Computing) Implementation
//
// This module implements seccomp filtering to restrict the system calls
// that a process can make, providing a sandboxing mechanism.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

/// seccomp action codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompAction {
    /// Allow the system call
    Allow = 0,
    /// Kill the process
    Kill = 1,
    /// Return errno
    Errno = 2,
    /// Trap the process
    Trap = 3,
    /// Trace the system call
    Trace = 4,
    /// Log the system call
    Log = 5,
}

/// seccomp comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompCmpOp {
    /// Equals
    Eq = 0,
    /// Not equals
    Ne = 1,
    /// Greater than
    Gt = 2,
    /// Greater than or equal
    Ge = 3,
    /// Less than
    Lt = 4,
    /// Less than or equal
    Le = 5,
    /// Masked equals
    MaskedEq = 6,
}

/// seccomp filter rule
#[derive(Debug, Clone)]
pub struct SeccompRule {
    /// System call number
    pub syscall: u32,
    /// Comparison operators
    pub cmp_ops: Vec<SeccompCmpOp>,
    /// Comparison values
    pub cmp_vals: Vec<u64>,
    /// Comparison masks (for MaskedEq)
    pub cmp_masks: Vec<u64>,
    /// Action to take
    pub action: SeccompAction,
    /// Return errno (for Errno action)
    pub errno: u32,
}

/// seccomp filter
#[derive(Debug, Clone)]
pub struct SeccompFilter {
    /// Filter ID
    pub filter_id: u64,
    /// Filter rules
    pub rules: Vec<SeccompRule>,
    /// Default action
    pub default_action: SeccompAction,
    /// Default errno (for Errno default action)
    pub default_errno: u32,
    /// Whether filter is in strict mode
    pub strict_mode: bool,
}

/// seccomp statistics
#[derive(Debug, Default, Clone)]
pub struct SeccompStats {
    pub events_processed: u64,
    /// Total syscalls filtered
    pub total_filtered: u64,
    /// Syscalls allowed
    pub syscalls_allowed: u64,
    /// Syscalls denied
    pub syscalls_denied: u64,
    /// Processes killed
    pub processes_killed: u64,
    /// Syscalls trapped
    pub syscalls_trapped: u64,
    /// Syscalls traced
    pub syscalls_traced: u64,
    /// Syscalls logged
    pub syscalls_logged: u64,
    /// Filters by process
    pub filters_by_process: BTreeMap<u64, u64>,
}

/// seccomp subsystem
pub struct SeccompSubsystem {
    /// Process filters
    process_filters: BTreeMap<u64, SeccompFilter>,
    /// Statistics
    stats: Arc<Mutex<SeccompStats>>,
    /// Next filter ID
    next_filter_id: AtomicU64,
}

impl SeccompSubsystem {
    /// Create new seccomp subsystem
    pub fn new() -> Self {
        Self {
            process_filters: BTreeMap::new(),
            stats: Arc::new(Mutex::new(SeccompStats::default())),
            next_filter_id: AtomicU64::new(1),
        }
    }

    /// Install seccomp filter for process
    pub fn install_filter(
        &mut self,
        pid: u64,
        filter: SeccompFilter,
    ) -> Result<u64, &'static str> {
        let filter_id = self.next_filter_id.fetch_add(1, Ordering::SeqCst);

        self.process_filters.insert(pid, filter.clone());

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.filters_by_process.insert(pid, filter_id);
        }

        Ok(filter_id)
    }

    /// Check if syscall is allowed
    pub fn check_syscall(&self, pid: u64, syscall: u32, args: &[u64]) -> SeccompAction {
        let filter = match self.process_filters.get(&pid) {
            Some(filter) => filter,
            None => return SeccompAction::Allow,
        };

        // Check each rule
        for rule in &filter.rules {
            if rule.syscall == syscall && self.check_rule(rule, args) {
                // Update statistics
                {
                    let mut stats = self.stats.lock();
                    stats.total_filtered += 1;

                    match rule.action {
                        SeccompAction::Allow => stats.syscalls_allowed += 1,
                        SeccompAction::Kill => stats.processes_killed += 1,
                        SeccompAction::Trap => stats.syscalls_trapped += 1,
                        SeccompAction::Trace => stats.syscalls_traced += 1,
                        SeccompAction::Log => stats.syscalls_logged += 1,
                        SeccompAction::Errno => stats.syscalls_denied += 1,
                    }
                }

                return rule.action;
            }
        }

        // Use default action
        let action = filter.default_action;

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_filtered += 1;

            match action {
                SeccompAction::Allow => stats.syscalls_allowed += 1,
                SeccompAction::Kill => stats.processes_killed += 1,
                SeccompAction::Trap => stats.syscalls_trapped += 1,
                SeccompAction::Trace => stats.syscalls_traced += 1,
                SeccompAction::Log => stats.syscalls_logged += 1,
                SeccompAction::Errno => stats.syscalls_denied += 1,
            }
        }

        action
    }

    /// Check if rule matches syscall arguments
    fn check_rule(&self, rule: &SeccompRule, args: &[u64]) -> bool {
        if rule.cmp_ops.len() != args.len() {
            return false;
        }

        for (i, &arg) in args.iter().enumerate() {
            if !self.check_cmp(rule.cmp_ops[i], arg, rule.cmp_vals[i], rule.cmp_masks[i]) {
                return false;
            }
        }

        true
    }

    /// Check comparison operator
    fn check_cmp(&self, op: SeccompCmpOp, arg: u64, val: u64, mask: u64) -> bool {
        match op {
            SeccompCmpOp::Eq => arg == val,
            SeccompCmpOp::Ne => arg != val,
            SeccompCmpOp::Gt => arg > val,
            SeccompCmpOp::Ge => arg >= val,
            SeccompCmpOp::Lt => arg < val,
            SeccompCmpOp::Le => arg <= val,
            SeccompCmpOp::MaskedEq => (arg & mask) == (val & mask),
        }
    }

    /// Remove filter for process
    pub fn remove_filter(&mut self, pid: u64) -> Result<(), &'static str> {
        match self.process_filters.remove(&pid) {
            Some(_) => {
                // Update statistics
                {
                    let mut stats = self.stats.lock();
                    stats.filters_by_process.remove(&pid);
                }
                Ok(())
            }
            None => Err("Filter not found"),
        }
    }

    /// Get filter for process
    pub fn get_filter(&self, pid: u64) -> Option<&SeccompFilter> {
        self.process_filters.get(&pid)
    }

    /// Cleanup process filter
    pub fn cleanup_process(&mut self, pid: u64) {
        self.process_filters.remove(&pid);
    }

    /// Get statistics
    pub fn get_stats(&self) -> SeccompStats {
        self.stats.lock().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        *self.stats.lock() = SeccompStats::default();
    }
}

/// High-level seccomp interface functions

/// Initialize seccomp subsystem
pub fn initialize_seccomp() -> Result<(), i32> {
    // Seccomp is already initialized via global static instance
    Ok(())
}

/// Cleanup seccomp subsystem
pub fn cleanup_seccomp() {
    // Placeholder: In a real implementation, this would clean up seccomp resources
}

/// Install seccomp filter for process
pub fn install_seccomp_filter(pid: u64, filter: SeccompFilter) -> Result<u64, &'static str> {
    let mut guard = crate::security::SECCOMP.lock();
    if let Some(ref mut s) = *guard {
        s.install_filter(pid, filter)
    } else {
        Ok(0)
    }
}

/// Check if syscall is allowed
pub fn check_seccomp_syscall(pid: u64, syscall: u32, args: &[u64]) -> SeccompAction {
    let guard = crate::security::SECCOMP.lock();
    guard.as_ref().map(|s| s.check_syscall(pid, syscall, args)).unwrap_or(SeccompAction::Allow)
}

/// Remove seccomp filter
pub fn remove_seccomp_filter(pid: u64) -> Result<(), &'static str> {
    let mut guard = crate::security::SECCOMP.lock();
    if let Some(ref mut s) = *guard {
        s.remove_filter(pid)
    } else {
        Ok(())
    }
}

/// Get seccomp statistics
pub fn get_seccomp_statistics() -> SeccompStats {
    let guard = crate::security::SECCOMP.lock();
    guard.as_ref().map(|s| s.get_stats()).unwrap_or_default()
}

// Global instance moved to crate::security::SECCOMP (Option)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seccomp_action_codes() {
        assert_eq!(SeccompAction::Allow as u32, 0);
        assert_eq!(SeccompAction::Kill as u32, 1);
        assert_eq!(SeccompAction::Errno as u32, 2);
    }

    #[test]
    fn test_seccomp_cmp_op_codes() {
        assert_eq!(SeccompCmpOp::Eq as u32, 0);
        assert_eq!(SeccompCmpOp::Ne as u32, 1);
        assert_eq!(SeccompCmpOp::Gt as u32, 2);
    }

    #[test]
    fn test_seccomp_rule() {
        let rule = SeccompRule {
            syscall: 60, // exit syscall
            cmp_ops: vec![SeccompCmpOp::Eq],
            cmp_vals: vec![0],
            cmp_masks: vec![0],
            action: SeccompAction::Allow,
            errno: 0,
        };

        assert_eq!(rule.syscall, 60);
        assert_eq!(rule.action, SeccompAction::Allow);
    }

    #[test]
    fn test_seccomp_filter() {
        let filter = SeccompFilter {
            filter_id: 1,
            rules: vec![],
            default_action: SeccompAction::Allow,
            default_errno: 0,
            strict_mode: false,
        };

        assert_eq!(filter.filter_id, 1);
        assert_eq!(filter.default_action, SeccompAction::Allow);
        assert!(!filter.strict_mode);
    }

    #[test]
    fn test_seccomp_subsystem() {
        let mut subsystem = SeccompSubsystem::new();

        let filter = SeccompFilter {
            filter_id: 1,
            rules: vec![],
            default_action: SeccompAction::Kill,
            default_errno: EPERM as u32,
            strict_mode: false,
        };

        let result = subsystem.install_filter(1234, filter);
        assert!(result.is_ok());

        let action = subsystem.check_syscall(1234, 60, &[]);
        assert_eq!(action, SeccompAction::Kill);

        let action = subsystem.check_syscall(5678, 60, &[]);
        assert_eq!(action, SeccompAction::Allow);
    }
}
