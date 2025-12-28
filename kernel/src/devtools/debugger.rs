//! Kernel Debugger
//!
//! This module implements kernel debugging tools:
//! - Breakpoints
//! - Variable inspection
//! - Call stack tracing
//! - Memory inspection
//!
//! Features:
//! - Hardware breakpoints
//! - Software breakpoints
//! - Watchpoints
//! - Register view

use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;

// ============================================================================
// Debugger Constants
// ============================================================================

/// Maximum breakpoints
pub const MAX_BREAKPOINTS: usize = 1 << 8;

/// Maximum watchpoints
pub const MAX_WATCHPOINTS: usize = 1 << 6;

/// Maximum call stack frames
pub const MAX_CALL_STACK: usize = 1 << 10;

// ============================================================================
// Breakpoint Types
// ============================================================================

/// Breakpoint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointType {
    /// Hardware breakpoint
    Hardware,
    
    /// Software breakpoint
    Software,
    
    /// Read watchpoint
    ReadWatch,
    
    /// Write watchpoint
    WriteWatch,
    
    /// Read/Write watchpoint
    ReadWriteWatch,
}

/// Breakpoint state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointState {
    /// Breakpoint is active
    Active,
    
    /// Breakpoint is disabled
    Disabled,
    
    /// Breakpoint was hit
    Hit,
    
    /// Breakpoint condition not met
    ConditionFailed,
}

// ============================================================================
// Breakpoint
// ============================================================================

/// Breakpoint
#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub breakpoint_id: String,
    pub address: usize,
    pub breakpoint_type: BreakpointType,
    pub state: Mutex<BreakpointState>,
    pub enabled: AtomicBool,
    pub hit_count: AtomicU64,
    pub condition: Option<BreakpointCondition>,
    pub command_on_hit: Option<String>,
    pub created_at: u64,
}

/// Breakpoint condition
#[derive(Debug, Clone)]
pub enum BreakpointCondition {
    /// Value at address equals
    Equals(u64),
    
    /// Value at address is greater than
    GreaterThan(u64),
    
    /// Value at address is less than
    LessThan(u64),
    
    /// Custom condition
    Custom(String),
}

impl Breakpoint {
    pub fn new(breakpoint_id: String, address: usize, breakpoint_type: BreakpointType) -> Self {
        Self {
            breakpoint_id,
            address,
            breakpoint_type,
            state: Mutex::new(BreakpointState::Active),
            enabled: AtomicBool::new(true),
            hit_count: AtomicU64::new(0),
            condition: None,
            command_on_hit: None,
            created_at: crate::subsystems::time::timestamp_nanos(),
        }
    }

    pub fn with_condition(mut self, condition: BreakpointCondition) -> Self {
        self.condition = Some(condition);
        self
    }

    pub fn with_command(mut self, command: String) -> Self {
        self.command_on_hit = Some(command);
        self
    }

    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn hit(&self) -> bool {
        self.hit_count.fetch_add(1, Ordering::Relaxed);
        true
    }

    pub fn get_hit_count(&self) -> u64 {
        self.hit_count.load(Ordering::Relaxed)
    }

    pub fn get_state(&self) -> BreakpointState {
        *self.state.lock()
    }

    pub fn set_state(&self, state: BreakpointState) {
        *self.state.lock() = state;
    }
}

// ============================================================================
// Call Stack Frame
// ============================================================================

/// Call stack frame
#[derive(Debug, Clone)]
pub struct CallStackFrame {
    pub frame_id: usize,
    pub return_address: usize,
    pub function_name: String,
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
    pub local_variables: BTreeMap<String, u64>,
    pub registers: BTreeMap<String, u64>,
}

impl CallStackFrame {
    pub fn new(frame_id: usize, return_address: usize, function_name: String) -> Self {
        Self {
            frame_id,
            return_address,
            function_name,
            file_path: None,
            line_number: None,
            local_variables: BTreeMap::new(),
            registers: BTreeMap::new(),
        }
    }

    pub fn with_location(mut self, file_path: String, line_number: usize) -> Self {
        self.file_path = Some(file_path);
        self.line_number = Some(line_number);
        self
    }

    pub fn with_local_variable(mut self, name: String, value: u64) -> Self {
        self.local_variables.insert(name, value);
        self
    }

    pub fn with_register(mut self, reg: String, value: u64) -> Self {
        self.registers.insert(reg, value);
        self
    }
}

// ============================================================================
// Memory Inspector
// ============================================================================

/// Memory inspector
pub struct MemoryInspector {
    pub read_cache: Mutex<BTreeMap<usize, [u8; 16]>>,
    pub watchpoints: Mutex<Vec<Arc<Breakpoint>>>>,
    pub next_watchpoint_id: AtomicU64,
    pub stats: Mutex<MemoryInspectorStats>,
}

/// Memory inspector statistics
#[derive(Debug, Clone, Copy)]
pub struct MemoryInspectorStats {
    pub total_reads: u64,
    pub total_writes: u64,
    pub watchpoint_hits: u64,
    pub cache_hits: u64,
}

impl Default for MemoryInspectorStats {
    fn default() -> Self {
        Self {
            total_reads: 0,
            total_writes: 0,
            watchpoint_hits: 0,
            cache_hits: 0,
        }
    }
}

impl MemoryInspector {
    pub fn new() -> Self {
        Self {
            read_cache: Mutex::new(BTreeMap::new()),
            watchpoints: Mutex::new(Vec::new()),
            next_watchpoint_id: AtomicU64::new(1),
            stats: Mutex::new(MemoryInspectorStats::default()),
        }
    }

    pub fn read_bytes(&self, address: usize, count: usize) -> Result<Vec<u8>, String> {
        if count == 0 || count > 16 {
            return Err("Invalid count".to_string());
        }

        self.stats.lock().total_reads.fetch_add(1, Ordering::Relaxed);

        let mut cache = self.read_cache.lock();
        
        if let Some(cached) = cache.get(&address) {
            self.stats.lock().cache_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(cached[..count].to_vec());
        }

        // In real implementation, would read actual memory
        let data = {
    let mut v = alloc::vec::Vec::new();
    v.resize(count, 0u8);
    v
};
        cache.insert(address, [0u8; 16]);

        Ok(data)
    }

    pub fn write_bytes(&self, address: usize, data: &[u8]) -> Result<(), String> {
        if data.len() > 16 {
            return Err("Data too large".to_string());
        }

        self.stats.lock().total_writes.fetch_add(1, Ordering::Relaxed);

        // Check watchpoints
        let watchpoints = self.watchpoints.lock();
        for watchpoint in watchpoints.iter() {
            if watchpoint.is_enabled() && watchpoint.address == address {
                self.stats.lock().watchpoint_hits.fetch_add(1, Ordering::Relaxed);
                crate::println!("[memory_inspector] Watchpoint hit at address {:#x}", address);
            }
        }

        // In real implementation, would write to actual memory
        Ok(())
    }

    pub fn add_watchpoint(&self, address: usize, watchpoint_type: BreakpointType) -> Result<String, String> {
        let mut watchpoints = self.watchpoints.lock();
        
        if watchpoints.len() >= MAX_WATCHPOINTS {
            return Err("Maximum watchpoints reached".to_string());
        }

        let watchpoint_id = { let mut s = alloc::string::String::from("watch-"); s.push_str(&self.next_watchpoint_id.fetch_add(1, Ordering::Relaxed.to_string()); s });
        let watchpoint = Arc::new(Breakpoint::new(watchpoint_id.clone(), address, watchpoint_type));

        watchpoints.push(watchpoint);
        crate::println!("[memory_inspector] Added watchpoint {} at address {:#x}", watchpoint_id, address);

        Ok(watchpoint_id)
    }

    pub fn remove_watchpoint(&self, watchpoint_id: String) -> Result<(), String> {
        let mut watchpoints = self.watchpoints.lock();
        
        let original_len = watchpoints.len();
        watchpoints.retain(|w| w.breakpoint_id != watchpoint_id);
        
        if watchpoints.len() == original_len {
            return Err(alloc::string::String::from("Watchpoint ") + &watchpoint_id.to_string() + alloc::string::String::from(" not found"));
        }

        crate::println!("[memory_inspector] Removed watchpoint {}", watchpoint_id);
        Ok(())
    }

    pub fn get_stats(&self) -> MemoryInspectorStats {
        *self.stats.lock()
    }
}

// ============================================================================
// Kernel Debugger
// ============================================================================

/// Kernel debugger
pub struct KernelDebugger {
    pub breakpoints: Mutex<Vec<Arc<Breakpoint>>>>,
    pub call_stack: Mutex<Vec<CallStackFrame>>,
    pub memory_inspector: Arc<MemoryInspector>,
    pub next_breakpoint_id: AtomicU64,
    pub enabled: AtomicBool,
    pub stats: Mutex<DebuggerStats>,
}

/// Debugger statistics
#[derive(Debug, Clone, Copy)]
pub struct DebuggerStats {
    pub total_breakpoints: usize,
    pub active_breakpoints: usize,
    pub breakpoint_hits: u64,
    pub call_stack_depth: AtomicUsize,
    pub stepping_mode: bool,
}

impl Default for DebuggerStats {
    fn default() -> Self {
        Self {
            total_breakpoints: 0,
            active_breakpoints: 0,
            breakpoint_hits: 0,
            call_stack_depth: AtomicUsize::new(0),
            stepping_mode: false,
        }
    }
}

impl KernelDebugger {
    pub fn new() -> Self {
        Self {
            breakpoints: Mutex::new(Vec::new()),
            call_stack: Mutex::new(Vec::new()),
            memory_inspector: Arc::new(MemoryInspector::new()),
            next_breakpoint_id: AtomicU64::new(1),
            enabled: AtomicBool::new(false),
            stats: Mutex::new(DebuggerStats::default()),
        }
    }

    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
        crate::println!("[debugger] Debugger enabled");
    }

    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
        crate::println!("[debugger] Debugger disabled");
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn add_breakpoint(&self, address: usize, breakpoint_type: BreakpointType) -> Result<String, String> {
        if !self.is_enabled() {
            return Err("Debugger not enabled".to_string());
        }

        let mut breakpoints = self.breakpoints.lock();
        
        if breakpoints.len() >= MAX_BREAKPOINTS {
            return Err("Maximum breakpoints reached".to_string());
        }

        let breakpoint_id = { let mut s = alloc::string::String::from("bp-"); s.push_str(&self.next_breakpoint_id.fetch_add(1, Ordering::Relaxed.to_string()); s });
        let breakpoint = Arc::new(Breakpoint::new(breakpoint_id.clone(), address, breakpoint_type));

        breakpoints.push(breakpoint);
        crate::println!("[debugger] Added breakpoint {} at address {:#x}", breakpoint_id, address);

        let mut stats = self.stats.lock();
        stats.total_breakpoints = breakpoints.len();
        stats.active_breakpoints = breakpoints.len();

        Ok(breakpoint_id)
    }

    pub fn remove_breakpoint(&self, breakpoint_id: String) -> Result<(), String> {
        let mut breakpoints = self.breakpoints.lock();
        
        let original_len = breakpoints.len();
        breakpoints.retain(|b| b.breakpoint_id != breakpoint_id);
        
        if breakpoints.len() == original_len {
            return Err(alloc::string::String::from("Breakpoint ") + &breakpoint_id.to_string() + alloc::string::String::from(" not found"));
        }

        let mut stats = self.stats.lock();
        stats.total_breakpoints = breakpoints.len();
        stats.active_breakpoints = breakpoints.iter().filter(|b| b.is_enabled()).count();

        crate::println!("[debugger] Removed breakpoint {}", breakpoint_id);
        Ok(())
    }

    pub fn enable_breakpoint(&self, breakpoint_id: String) -> Result<(), String> {
        let breakpoints = self.breakpoints.lock();
        let breakpoint = breakpoints.iter()
            .find(|b| b.breakpoint_id == breakpoint_id)
            .ok_or(alloc::string::String::from("Breakpoint ") + &breakpoint_id.to_string() + alloc::string::String::from(" not found"))?;

        breakpoint.enable();

        let mut stats = self.stats.lock();
        stats.active_breakpoints = breakpoints.iter().filter(|b| b.is_enabled()).count();

        crate::println!("[debugger] Enabled breakpoint {}", breakpoint_id);
        Ok(())
    }

    pub fn disable_breakpoint(&self, breakpoint_id: String) -> Result<(), String> {
        let breakpoints = self.breakpoints.lock();
        let breakpoint = breakpoints.iter()
            .find(|b| b.breakpoint_id == breakpoint_id)
            .ok_or(alloc::string::String::from("Breakpoint ") + &breakpoint_id.to_string() + alloc::string::String::from(" not found"))?;

        breakpoint.disable();

        let mut stats = self.stats.lock();
        stats.active_breakpoints = breakpoints.iter().filter(|b| b.is_enabled()).count();

        crate::println!("[debugger] Disabled breakpoint {}", breakpoint_id);
        Ok(())
    }

    pub fn push_call_frame(&self, frame: CallStackFrame) -> Result<(), String> {
        if !self.is_enabled() {
            return Err("Debugger not enabled".to_string());
        }

        let mut call_stack = self.call_stack.lock();
        
        if call_stack.len() >= MAX_CALL_STACK {
            call_stack.remove(0);
        }

        call_stack.push(frame);
        self.stats.lock().call_stack_depth.store(call_stack.len(), Ordering::Relaxed);

        crate::println!("[debugger] Pushed call frame: {}", frame.function_name);
        Ok(())
    }

    pub fn pop_call_frame(&self) -> Result<Option<CallStackFrame>, String> {
        if !self.is_enabled() {
            return Err("Debugger not enabled".to_string());
        }

        let mut call_stack = self.call_stack.lock();
        let frame = call_stack.pop();

        self.stats.lock().call_stack_depth.store(call_stack.len(), Ordering::Relaxed);

        if let Some(ref frame) = frame {
            crate::println!("[debugger] Popped call frame: {}", frame.function_name);
        }

        Ok(frame)
    }

    pub fn get_call_stack(&self) -> Vec<CallStackFrame> {
        self.call_stack.lock().clone()
    }

    pub fn get_memory_inspector(&self) -> Arc<MemoryInspector> {
        self.memory_inspector.clone()
    }

    pub fn get_stats(&self) -> DebuggerStats {
        let mut stats = self.stats.lock();
        stats.active_breakpoints = self.breakpoints.lock().iter().filter(|b| b.is_enabled()).count();
        *stats
    }
}
