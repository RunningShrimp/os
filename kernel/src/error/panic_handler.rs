//! Enhanced Panic Handler
//!
//! This module provides an enhanced panic handler that:
//! - Collects structured crash information (registers, stack, process info)
//! - Generates structured crash reports
//! - Integrates with error reporting module
//! - Provides detailed diagnostics for debugging

extern crate alloc;

use alloc::string::String;
use alloc::format;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use crate::error::{UnifiedError, ErrorContext, ErrorSeverity, handle_error};
use crate::log_error; use crate::log_info;

/// Structured crash information
#[derive(Debug, Clone)]
pub struct CrashInfo {
    /// Panic message
    pub message: String,
    /// File location
    pub file: Option<String>,
    /// Line number
    pub line: Option<u32>,
    /// Column number
    pub column: Option<u32>,
    /// Timestamp
    pub timestamp: u64,
    /// CPU ID
    pub cpu_id: usize,
    /// Process ID (if available)
    pub pid: Option<usize>,
    /// Register dump
    pub registers: RegisterDump,
    /// Stack trace (if available)
    pub stack_trace: Vec<usize>,
    /// System state
    pub system_state: SystemState,
}

/// Register dump (architecture-specific)
#[derive(Debug, Clone)]
pub struct RegisterDump {
    #[cfg(target_arch = "x86_64")]
    pub rax: u64,
    #[cfg(target_arch = "x86_64")]
    pub rbx: u64,
    #[cfg(target_arch = "x86_64")]
    pub rcx: u64,
    #[cfg(target_arch = "x86_64")]
    pub rdx: u64,
    #[cfg(target_arch = "x86_64")]
    pub rsi: u64,
    #[cfg(target_arch = "x86_64")]
    pub rdi: u64,
    #[cfg(target_arch = "x86_64")]
    pub rbp: u64,
    #[cfg(target_arch = "x86_64")]
    pub rsp: u64,
    #[cfg(target_arch = "x86_64")]
    pub r8: u64,
    #[cfg(target_arch = "x86_64")]
    pub r9: u64,
    #[cfg(target_arch = "x86_64")]
    pub r10: u64,
    #[cfg(target_arch = "x86_64")]
    pub r11: u64,
    #[cfg(target_arch = "x86_64")]
    pub r12: u64,
    #[cfg(target_arch = "x86_64")]
    pub r13: u64,
    #[cfg(target_arch = "x86_64")]
    pub r14: u64,
    #[cfg(target_arch = "x86_64")]
    pub r15: u64,
    #[cfg(target_arch = "x86_64")]
    pub rip: u64,
    #[cfg(target_arch = "x86_64")]
    pub rflags: u64,
    #[cfg(target_arch = "x86_64")]
    pub cs: u64,
    #[cfg(target_arch = "x86_64")]
    pub ss: u64,

    #[cfg(target_arch = "aarch64")]
    pub x0: u64,
    #[cfg(target_arch = "aarch64")]
    pub x1: u64,
    #[cfg(target_arch = "aarch64")]
    pub x2: u64,
    #[cfg(target_arch = "aarch64")]
    pub x3: u64,
    #[cfg(target_arch = "aarch64")]
    pub x4: u64,
    #[cfg(target_arch = "aarch64")]
    pub x5: u64,
    #[cfg(target_arch = "aarch64")]
    pub x6: u64,
    #[cfg(target_arch = "aarch64")]
    pub x7: u64,
    #[cfg(target_arch = "aarch64")]
    pub x8: u64,
    #[cfg(target_arch = "aarch64")]
    pub x9: u64,
    #[cfg(target_arch = "aarch64")]
    pub x10: u64,
    #[cfg(target_arch = "aarch64")]
    pub x11: u64,
    #[cfg(target_arch = "aarch64")]
    pub x12: u64,
    #[cfg(target_arch = "aarch64")]
    pub x13: u64,
    #[cfg(target_arch = "aarch64")]
    pub x14: u64,
    #[cfg(target_arch = "aarch64")]
    pub x15: u64,
    #[cfg(target_arch = "aarch64")]
    pub x16: u64,
    #[cfg(target_arch = "aarch64")]
    pub x17: u64,
    #[cfg(target_arch = "aarch64")]
    pub x18: u64,
    #[cfg(target_arch = "aarch64")]
    pub x19: u64,
    #[cfg(target_arch = "aarch64")]
    pub x20: u64,
    #[cfg(target_arch = "aarch64")]
    pub x21: u64,
    #[cfg(target_arch = "aarch64")]
    pub x22: u64,
    #[cfg(target_arch = "aarch64")]
    pub x23: u64,
    #[cfg(target_arch = "aarch64")]
    pub x24: u64,
    #[cfg(target_arch = "aarch64")]
    pub x25: u64,
    #[cfg(target_arch = "aarch64")]
    pub x26: u64,
    #[cfg(target_arch = "aarch64")]
    pub x27: u64,
    #[cfg(target_arch = "aarch64")]
    pub x28: u64,
    #[cfg(target_arch = "aarch64")]
    pub x29: u64,
    #[cfg(target_arch = "aarch64")]
    pub x30: u64,
    #[cfg(target_arch = "aarch64")]
    pub sp: u64,
    #[cfg(target_arch = "aarch64")]
    pub pc: u64,
    #[cfg(target_arch = "aarch64")]
    pub pstate: u64,

    #[cfg(target_arch = "riscv64")]
    pub x0: u64,
    #[cfg(target_arch = "riscv64")]
    pub x1: u64,
    #[cfg(target_arch = "riscv64")]
    pub x2: u64,
    #[cfg(target_arch = "riscv64")]
    pub x3: u64,
    #[cfg(target_arch = "riscv64")]
    pub x4: u64,
    #[cfg(target_arch = "riscv64")]
    pub x5: u64,
    #[cfg(target_arch = "riscv64")]
    pub x6: u64,
    #[cfg(target_arch = "riscv64")]
    pub x7: u64,
    #[cfg(target_arch = "riscv64")]
    pub x8: u64,
    #[cfg(target_arch = "riscv64")]
    pub x9: u64,
    #[cfg(target_arch = "riscv64")]
    pub x10: u64,
    #[cfg(target_arch = "riscv64")]
    pub x11: u64,
    #[cfg(target_arch = "riscv64")]
    pub x12: u64,
    #[cfg(target_arch = "riscv64")]
    pub x13: u64,
    #[cfg(target_arch = "riscv64")]
    pub x14: u64,
    #[cfg(target_arch = "riscv64")]
    pub x15: u64,
    #[cfg(target_arch = "riscv64")]
    pub x16: u64,
    #[cfg(target_arch = "riscv64")]
    pub x17: u64,
    #[cfg(target_arch = "riscv64")]
    pub x18: u64,
    #[cfg(target_arch = "riscv64")]
    pub x19: u64,
    #[cfg(target_arch = "riscv64")]
    pub x20: u64,
    #[cfg(target_arch = "riscv64")]
    pub x21: u64,
    #[cfg(target_arch = "riscv64")]
    pub x22: u64,
    #[cfg(target_arch = "riscv64")]
    pub x23: u64,
    #[cfg(target_arch = "riscv64")]
    pub x24: u64,
    #[cfg(target_arch = "riscv64")]
    pub x25: u64,
    #[cfg(target_arch = "riscv64")]
    pub x26: u64,
    #[cfg(target_arch = "riscv64")]
    pub x27: u64,
    #[cfg(target_arch = "riscv64")]
    pub x28: u64,
    #[cfg(target_arch = "riscv64")]
    pub x29: u64,
    #[cfg(target_arch = "riscv64")]
    pub x30: u64,
    #[cfg(target_arch = "riscv64")]
    pub x31: u64,
    #[cfg(target_arch = "riscv64")]
    pub pc: u64,
    #[cfg(target_arch = "riscv64")]
    pub sp: u64,
}

impl RegisterDump {
    /// Create an empty register dump
    pub const fn new() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self {
                rax: 0, rbx: 0, rcx: 0, rdx: 0, rsi: 0, rdi: 0,
                rbp: 0, rsp: 0, r8: 0, r9: 0, r10: 0, r11: 0,
                r12: 0, r13: 0, r14: 0, r15: 0, rip: 0, rflags: 0,
                cs: 0, ss: 0,
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            Self {
                x0: 0, x1: 0, x2: 0, x3: 0, x4: 0, x5: 0, x6: 0, x7: 0,
                x8: 0, x9: 0, x10: 0, x11: 0, x12: 0, x13: 0, x14: 0, x15: 0,
                x16: 0, x17: 0, x18: 0, x19: 0, x20: 0, x21: 0, x22: 0, x23: 0,
                x24: 0, x25: 0, x26: 0, x27: 0, x28: 0, x29: 0, x30: 0,
                sp: 0, pc: 0, pstate: 0,
            }
        }
        #[cfg(target_arch = "riscv64")]
        {
            Self {
                x0: 0, x1: 0, x2: 0, x3: 0, x4: 0, x5: 0, x6: 0, x7: 0,
                x8: 0, x9: 0, x10: 0, x11: 0, x12: 0, x13: 0, x14: 0, x15: 0,
                x16: 0, x17: 0, x18: 0, x19: 0, x20: 0, x21: 0, x22: 0, x23: 0,
                x24: 0, x25: 0, x26: 0, x27: 0, x28: 0, x29: 0, x30: 0, x31: 0,
                pc: 0, sp: 0,
            }
        }
    }
}

/// System state at crash time
#[derive(Debug, Clone)]
pub struct SystemState {
    /// Total number of processes
    pub process_count: usize,
    /// Running processes
    pub running_processes: usize,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// Free memory (bytes)
    pub free_memory: u64,
    /// Error statistics
    pub error_stats: crate::error::ErrorStats,
}

impl SystemState {
    /// Collect current system state
    pub fn collect() -> Self {
        let proc_table = crate::subsystems::process::manager::PROC_TABLE.lock();
        let total_processes = proc_table.iter().count();
        let running_processes = proc_table.iter()
            .filter(|p| p.state == crate::subsystems::process::manager::ProcState::Running)
            .count();
        drop(proc_table);

        let error_stats = crate::error::get_error_stats();

        Self {
            process_count: total_processes,
            running_processes,
            memory_usage: 0, // TODO: Get actual memory usage
            free_memory: 0,  // TODO: Get actual free memory
            error_stats,
        }
    }
}

/// Collect crash information from panic info
pub fn collect_crash_info(info: &PanicInfo) -> CrashInfo {
    let message = if let Some(msg) = info.message() {
        format!("{}", msg)
    } else {
        "No panic message".to_string()
    };

    let (file, line, column) = if let Some(location) = info.location() {
        (Some(location.file().to_string()), Some(location.line()), Some(location.column()))
    } else {
        (None, None, None)
    };

    // Get current CPU ID
    let cpu_id = crate::platform::arch::cpuid();

    // Get current process ID (if available)
    let pid = crate::subsystems::process::manager::myproc();

    // Collect registers (simplified - in real implementation would read from trap frame)
    let registers = RegisterDump::new();

    // Collect stack trace (simplified - would need proper stack unwinding)
    let stack_trace = Vec::new();

    // Collect system state
    let system_state = SystemState::collect();

    CrashInfo {
        message,
        file,
        line,
        column,
        timestamp: crate::subsystems::time::hrtime_nanos(),
        cpu_id,
        pid,
        registers,
        stack_trace,
        system_state,
    }
}

/// Format crash info as structured text
pub fn format_crash_report(crash_info: &CrashInfo) -> String {
    let mut report = String::new();
    
    report.push_str("=== KERNEL PANIC REPORT ===\n\n");
    
    // Basic information
    report.push_str("PANIC INFORMATION:\n");
    report.push_str(&format!("  Message: {}\n", crash_info.message));
    if let Some(ref file) = crash_info.file {
        report.push_str(&format!("  Location: {}", file));
        if let Some(line) = crash_info.line {
            report.push_str(&format!(":{}", line));
            if let Some(col) = crash_info.column {
                report.push_str(&format!(":{}", col));
            }
        }
        report.push_str("\n");
    }
    report.push_str(&format!("  Timestamp: {}\n", crash_info.timestamp));
    report.push_str(&format!("  CPU ID: {}\n", crash_info.cpu_id));
    if let Some(pid) = crash_info.pid {
        report.push_str(&format!("  Process ID: {}\n", pid));
    }
    report.push_str("\n");

    // Register dump
    report.push_str("REGISTERS:\n");
    #[cfg(target_arch = "x86_64")]
    {
        report.push_str(&format!("  RAX: {:#018x}  RBX: {:#018x}  RCX: {:#018x}  RDX: {:#018x}\n",
            crash_info.registers.rax, crash_info.registers.rbx,
            crash_info.registers.rcx, crash_info.registers.rdx));
        report.push_str(&format!("  RSI: {:#018x}  RDI: {:#018x}  RBP: {:#018x}  RSP: {:#018x}\n",
            crash_info.registers.rsi, crash_info.registers.rdi,
            crash_info.registers.rbp, crash_info.registers.rsp));
        report.push_str(&format!("  R8:  {:#018x}  R9:  {:#018x}  R10: {:#018x}  R11: {:#018x}\n",
            crash_info.registers.r8, crash_info.registers.r9,
            crash_info.registers.r10, crash_info.registers.r11));
        report.push_str(&format!("  R12: {:#018x}  R13: {:#018x}  R14: {:#018x}  R15: {:#018x}\n",
            crash_info.registers.r12, crash_info.registers.r13,
            crash_info.registers.r14, crash_info.registers.r15));
        report.push_str(&format!("  RIP: {:#018x}  RFLAGS: {:#018x}\n",
            crash_info.registers.rip, crash_info.registers.rflags));
        report.push_str(&format!("  CS:  {:#018x}  SS:  {:#018x}\n",
            crash_info.registers.cs, crash_info.registers.ss));
    }
    #[cfg(target_arch = "aarch64")]
    {
        report.push_str(&format!("  PC: {:#018x}  SP: {:#018x}  PSTATE: {:#018x}\n",
            crash_info.registers.pc, crash_info.registers.sp, crash_info.registers.pstate));
        report.push_str(&format!("  X0-X7: {:#018x} {:#018x} {:#018x} {:#018x} {:#018x} {:#018x} {:#018x} {:#018x}\n",
            crash_info.registers.x0, crash_info.registers.x1, crash_info.registers.x2, crash_info.registers.x3,
            crash_info.registers.x4, crash_info.registers.x5, crash_info.registers.x6, crash_info.registers.x7));
    }
    #[cfg(target_arch = "riscv64")]
    {
        report.push_str(&format!("  PC: {:#018x}  SP: {:#018x}\n",
            crash_info.registers.pc, crash_info.registers.sp));
    }
    report.push_str("\n");

    // System state
    report.push_str("SYSTEM STATE:\n");
    report.push_str(&format!("  Total processes: {}\n", crash_info.system_state.process_count));
    report.push_str(&format!("  Running processes: {}\n", crash_info.system_state.running_processes));
    report.push_str(&format!("  Total errors: {}\n", crash_info.system_state.error_stats.total_errors));
    report.push_str(&format!("  Critical errors: {}\n", crash_info.system_state.error_stats.critical_errors));
    report.push_str("\n");

    // Stack trace
    if !crash_info.stack_trace.is_empty() {
        report.push_str("STACK TRACE:\n");
        for (i, addr) in crash_info.stack_trace.iter().enumerate() {
            report.push_str(&format!("  #{}: {:#018x}\n", i, addr));
        }
        report.push_str("\n");
    }

    report.push_str("=== END OF PANIC REPORT ===\n");
    
    report
}

/// Report crash to error reporting system
pub fn report_crash(crash_info: &CrashInfo) {
    // Create unified error from crash info
    let error = UnifiedError::Other(format!("Kernel panic: {}", crash_info.message));
    
    // Create error context
    let location = crash_info.file.clone().unwrap_or_else(|| "unknown".to_string());
    let context = format!("{}:{} on CPU {}", location, crash_info.line.unwrap_or(0), crash_info.cpu_id);
    
    // Handle the error using kernel's internal error handling
    let action = handle_error(error, &context);
    
    // Log the result
    match action {
        crate::error::ErrorAction::Log => log_info!("[panic] Crash logged successfully"),
        crate::error::ErrorAction::Panic => log_error!("[panic] Crash reported as panic"),
        _ => log_info!("[panic] Crash handled with action: {:?}", action),
    }
}

/// Enhanced panic handler
pub fn enhanced_panic_handler(info: &PanicInfo) -> ! {
    // Disable interrupts
    crate::platform::arch::intr_off();

    // Collect crash information
    let crash_info = collect_crash_info(info);

    // Format and print crash report
    let crash_report = format_crash_report(&crash_info);
    crate::println!("\n{}", crash_report);

    // Report to error reporting system
    report_crash(&crash_info);

    // Also trigger health monitoring integration if available
    if let Ok(_) = crate::monitoring::health_integration::trigger_degradation_from_error_handling(
        "kernel",
        "critical",
        &format!("Kernel panic: {}", crash_info.message)
    ) {
        crate::println!("[panic] Triggered health monitoring degradation");
    }

    crate::println!("\nSystem halted.");

    // Halt the CPU
    loop {
        crate::platform::arch::wfi();
    }
}

