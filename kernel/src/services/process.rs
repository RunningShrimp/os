//! Process Management Service for Hybrid Architecture
//!
//! This module provides process management as a microkernel service.
//! It wraps the core process management functionality from `crate::process::manager`
//! and exposes it via IPC for service-based access.
//!
//! **Architecture**:
//! - Core process management: `crate::process::manager` (process table, scheduling)
//! - Service layer: This module (IPC interface, resource limits, security)
//!
//! **Responsibilities**:
//! - Process lifecycle management via IPC
//! - Resource limits and quotas
//! - Process security and isolation
//! - Process statistics and monitoring

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::string::String;
use alloc::string::ToString;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use crate::sync::Mutex;
use crate::reliability::errno::{EINVAL, ENOMEM, EFAULT, EPERM, ESRCH, ECHILD};
use crate::process::{ProcState, Proc, NPROC, NOFILE};
use crate::microkernel::{
    service_registry::{ServiceRegistry, ServiceId, ServiceCategory, ServiceInfo, ServiceStatus, InterfaceVersion},
    ipc::{IpcManager, IpcMessage},
    scheduler::{MicroScheduler, SchedulingPolicy, CpuAffinity},
};

// ============================================================================
// Process Service Configuration and Constants
// ============================================================================

/// Process service configuration
pub const PROCESS_SERVICE_NAME: &str = "process_manager";
pub const PROCESS_SERVICE_VERSION: InterfaceVersion = InterfaceVersion::new(1, 0, 0);
pub const PROCESS_SERVICE_QUEUE_SIZE: usize = 2048;
pub const MAX_PROCESSES: usize = 1024;
pub const DEFAULT_STACK_SIZE: usize = 8 * 1024 * 1024; // 8MB
pub const PROCESS_GC_INTERVAL: u64 = 60_000_000_000; // 60 seconds

// ============================================================================
// Process Service Messages
// ============================================================================

/// Process service message types
#[derive(Debug, Clone, Copy)]
pub enum ProcessMessageType {
    CreateProcess = 1,
    ForkProcess = 2,
    ExitProcess = 3,
    WaitProcess = 4,
    KillProcess = 5,
    GetProcessInfo = 6,
    SetProcessPriority = 7,
    SetProcessAffinity = 8,
    SuspendProcess = 9,
    ResumeProcess = 10,
    GetProcessStats = 11,
    SetResourceLimits = 12,
    GetResourceLimits = 13,
    ProcessSignal = 14,
    GetChildren = 15,
}

/// Process creation request
#[derive(Debug, Clone)]
pub struct ProcessCreationRequest {
    pub binary_path: String,
    pub arguments: Vec<String>,
    pub environment: Vec<String>,
    pub working_directory: String,
    pub uid: u32,
    pub gid: u32,
    pub priority: i32,
    pub scheduling_policy: SchedulingPolicy,
    pub cpu_affinity: CpuAffinity,
    pub resource_limits: ResourceLimits,
}

/// Process creation response
#[derive(Debug, Clone)]
pub struct ProcessCreationResponse {
    pub process_id: u64,
    pub thread_id: u64,
    pub success: bool,
    pub error_code: i32,
}

/// Process information
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub process_id: u64,
    pub parent_id: u64,
    pub thread_group_id: u64,
    pub session_id: u64,
    pub state: ProcessState,
    pub priority: i32,
    pub scheduling_policy: SchedulingPolicy,
    pub cpu_affinity: CpuAffinity,
    pub uid: u32,
    pub gid: u32,
    pub creation_time: u64,
    pub start_time: u64,
    pub cpu_time: u64,
    pub memory_usage: ProcessMemoryUsage,
    pub file_descriptors: u32,
    pub signal_mask: u64,
    pub exit_code: Option<i32>,
}

/// Process state (enhanced)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Created,
    Runnable,
    Running,
    Sleeping,
    Stopped,
    Zombie,
    Dead,
    Suspended,
}

impl ProcessState {
    pub fn is_alive(&self) -> bool {
        matches!(self, Self::Created | Self::Runnable | Self::Running |
                          Self::Sleeping | Self::Stopped | Self::Suspended)
    }

    pub fn is_running(&self) -> bool {
        matches!(self, Self::Runnable | Self::Running)
    }

    pub fn is_zombie(&self) -> bool {
        matches!(self, Self::Zombie)
    }
}

/// Process memory usage statistics
#[derive(Debug, Clone)]
pub struct ProcessMemoryUsage {
    pub virtual_memory: usize,
    pub physical_memory: usize,
    pub shared_memory: usize,
    pub code_memory: usize,
    pub data_memory: usize,
    pub stack_memory: usize,
    pub heap_memory: usize,
}

/// Resource limits for processes
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_cpu_time: u64,        // CPU time in nanoseconds
    pub max_memory: usize,         // Maximum memory in bytes
    pub max_files: u32,            // Maximum open file descriptors
    pub max_processes: u32,        // Maximum child processes
    pub max_stack_size: usize,     // Maximum stack size
}

impl ResourceLimits {
    pub const fn default() -> Self {
        Self {
            max_cpu_time: u64::MAX,
            max_memory: usize::MAX,
            max_files: 1024,
            max_processes: 64,
            max_stack_size: DEFAULT_STACK_SIZE,
        }
    }
}

/// Process signal information
#[derive(Debug, Clone)]
pub struct ProcessSignal {
    pub signal_number: i32,
    pub sender_id: u64,
    pub timestamp: u64,
    pub data: Option<u64>,
}

// ============================================================================
// Process Service Statistics
// ============================================================================

/// Comprehensive process service statistics
#[derive(Debug)]
pub struct ProcessServiceStats {
    pub total_processes: AtomicUsize,
    pub running_processes: AtomicUsize,
    pub sleeping_processes: AtomicUsize,
    pub zombie_processes: AtomicUsize,
    pub suspended_processes: AtomicUsize,

    pub total_forks: AtomicU64,
    pub total_exits: AtomicU64,
    pub successful_forks: AtomicU64,
    pub failed_forks: AtomicU64,

    pub average_cpu_time: f64,
    pub average_memory_usage: f64,
    pub context_switches: AtomicU64,

    pub last_gc_time: u64,
    pub gc_count: u64,
    pub orphaned_processes_collected: u64,
}

impl ProcessServiceStats {
    pub const fn new() -> Self {
        Self {
            total_processes: AtomicUsize::new(0),
            running_processes: AtomicUsize::new(0),
            sleeping_processes: AtomicUsize::new(0),
            zombie_processes: AtomicUsize::new(0),
            suspended_processes: AtomicUsize::new(0),
            total_forks: AtomicU64::new(0),
            total_exits: AtomicU64::new(0),
            successful_forks: AtomicU64::new(0),
            failed_forks: AtomicU64::new(0),
            average_cpu_time: 0.0,
            average_memory_usage: 0.0,
            context_switches: AtomicU64::new(0),
            last_gc_time: 0,
            gc_count: 0,
            orphaned_processes_collected: 0,
        }
    }

    pub fn increment_total_forks(&self) {
        self.total_forks.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_successful_forks(&self) {
        self.successful_forks.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_failed_forks(&self) {
        self.failed_forks.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_total_exits(&self) {
        self.total_exits.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_context_switches(&self) {
        self.context_switches.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get_fork_success_rate(&self) -> f64 {
        let total = self.total_forks.load(Ordering::SeqCst);
        if total == 0 {
            0.0
        } else {
            self.successful_forks.load(Ordering::SeqCst) as f64 / total as f64
        }
    }
}

// ============================================================================
// Process Management Service Implementation
// ============================================================================

/// Process management service
pub struct ProcessManagementService {
    pub service_id: ServiceId,
    pub ipc_queue_id: u64,
    // Scheduler is accessed via global function, not stored
    pub stats: Mutex<ProcessServiceStats>,
    pub process_table: Mutex<BTreeMap<u64, ProcessInfo>>,
    pub resource_limits: Mutex<BTreeMap<u64, ResourceLimits>>,
    pub pending_signals: Mutex<BTreeMap<u64, Vec<ProcessSignal>>>,
    pub next_process_id: AtomicU64,
    pub next_thread_id: AtomicU64,
}

impl ProcessManagementService {
    pub fn new(_scheduler: &'static mut MicroScheduler, ipc_manager: &IpcManager) -> Result<Self, i32> {
        // Create IPC queue for process service
        let ipc_queue_id = ipc_manager.create_message_queue(
            0, // owner_id (will be set to service ID)
            PROCESS_SERVICE_QUEUE_SIZE,
            8192, // max message size
        )?;

        Ok(Self {
            service_id: 0, // Will be set during registration
            ipc_queue_id,
            stats: Mutex::new(ProcessServiceStats::new()),
            process_table: Mutex::new(BTreeMap::new()),
            resource_limits: Mutex::new(BTreeMap::new()),
            pending_signals: Mutex::new(BTreeMap::new()),
            next_process_id: AtomicU64::new(1),
            next_thread_id: AtomicU64::new(1),
        })
    }

    pub fn register_service(&mut self, registry: &ServiceRegistry) -> Result<ServiceId, i32> {
        let service_info = ServiceInfo::new(
            0, // Will be assigned by registry
            PROCESS_SERVICE_NAME.to_string(),
            "Advanced process management service for hybrid architecture".to_string(),
            ServiceCategory::Process,
            PROCESS_SERVICE_VERSION,
            0, // owner_id (kernel process)
        );

        self.service_id = registry.register_service(service_info)?;

        // Set IPC channel for the service
        registry.set_service_ipc_channel(self.service_id, self.ipc_queue_id)?;

        Ok(self.service_id)
    }

    pub fn handle_message(&self, message: IpcMessage) -> Result<Vec<u8>, i32> {
        match message.message_type {
            msg_type if msg_type == ProcessMessageType::CreateProcess as u32 => {
                self.handle_create_process(&message.data, message.sender_id)
            }
            msg_type if msg_type == ProcessMessageType::ForkProcess as u32 => {
                self.handle_fork_process(&message.data, message.sender_id)
            }
            msg_type if msg_type == ProcessMessageType::ExitProcess as u32 => {
                self.handle_exit_process(&message.data, message.sender_id)
            }
            msg_type if msg_type == ProcessMessageType::KillProcess as u32 => {
                self.handle_kill_process(&message.data, message.sender_id)
            }
            msg_type if msg_type == ProcessMessageType::GetProcessInfo as u32 => {
                self.handle_get_process_info(&message.data, message.sender_id)
            }
            msg_type if msg_type == ProcessMessageType::SetProcessPriority as u32 => {
                self.handle_set_process_priority(&message.data, message.sender_id)
            }
            msg_type if msg_type == ProcessMessageType::GetProcessStats as u32 => {
                self.handle_get_process_stats()
            }
            msg_type if msg_type == ProcessMessageType::ProcessSignal as u32 => {
                self.handle_process_signal(&message.data, message.sender_id)
            }
            _ => Err(EINVAL),
        }
    }

    fn handle_create_process(&self, data: &[u8], sender_id: u64) -> Result<Vec<u8>, i32> {
        if data.is_empty() {
            return Err(EINVAL);
        }

        // Parse process creation request (simplified)
        let process_id = self.next_process_id.fetch_add(1, Ordering::SeqCst);
        let thread_id = self.next_thread_id.fetch_add(1, Ordering::SeqCst);

        // Create thread in microkernel scheduler
        let scheduler = crate::microkernel::scheduler::get_scheduler()
            .ok_or(EFAULT)?;
        let scheduler_tid = scheduler.create_thread(0, SchedulingPolicy::Normal)?;
        scheduler.set_thread_state(scheduler_tid, crate::process::thread::ThreadState::Runnable)?;

        // Create process info
        let current_time = crate::time::get_time_ns();
        let process_info = ProcessInfo {
            process_id,
            parent_id: sender_id,
            thread_group_id: process_id,
            session_id: sender_id, // Inherit from parent
            state: ProcessState::Created,
            priority: 0,
            scheduling_policy: SchedulingPolicy::Normal,
            cpu_affinity: CpuAffinity::new(),
            uid: 0,
            gid: 0,
            creation_time: current_time,
            start_time: current_time,
            cpu_time: 0,
            memory_usage: ProcessMemoryUsage {
                virtual_memory: DEFAULT_STACK_SIZE,
                physical_memory: 0,
                shared_memory: 0,
                code_memory: 0,
                data_memory: 0,
                stack_memory: DEFAULT_STACK_SIZE,
                heap_memory: 0,
            },
            file_descriptors: 3, // stdin, stdout, stderr
            signal_mask: 0,
            exit_code: None,
        };

        // Add to process table
        {
            let mut table = self.process_table.lock();
            table.insert(process_id, process_info);
        }

        // Set default resource limits
        {
            let mut limits = self.resource_limits.lock();
            limits.insert(process_id, ResourceLimits::default());
        }

        // Update statistics
        {
            let stats = self.stats.lock();
            stats.total_processes.fetch_add(1, Ordering::SeqCst);
            stats.increment_successful_forks();
        }

        let response = ProcessCreationResponse {
            process_id,
            thread_id: scheduler_tid as u64,
            success: true,
            error_code: 0,
        };

        Ok(unsafe { core::slice::from_raw_parts(
            &response as *const _ as *const u8,
            core::mem::size_of::<ProcessCreationResponse>()
        ).to_vec() })
    }

    fn handle_fork_process(&self, _data: &[u8], sender_id: u64) -> Result<Vec<u8>, i32> {
        let parent_id = sender_id;

        // Get parent process info
        let parent_info = {
            let table = self.process_table.lock();
            table.get(&parent_id).cloned()
        };

        if parent_info.is_none() {
            return Err(ESRCH);
        }

        let parent_info = parent_info.unwrap();

        // Create child process
        let child_id = self.next_process_id.fetch_add(1, Ordering::SeqCst);

        // Create thread for child process
        let scheduler = crate::microkernel::scheduler::get_scheduler()
            .ok_or(EFAULT)?;
        let child_tid = scheduler.create_thread(parent_info.priority, parent_info.scheduling_policy)?;

        // Copy parent process info to child
        let child_info = ProcessInfo {
            process_id: child_id,
            parent_id,
            thread_group_id: child_id,
            session_id: parent_info.session_id,
            state: ProcessState::Created,
            priority: parent_info.priority,
            scheduling_policy: parent_info.scheduling_policy,
            cpu_affinity: parent_info.cpu_affinity,
            uid: parent_info.uid,
            gid: parent_info.gid,
            creation_time: crate::time::get_time_ns(),
            start_time: crate::time::get_time_ns(),
            cpu_time: 0,
            memory_usage: parent_info.memory_usage.clone(),
            file_descriptors: parent_info.file_descriptors,
            signal_mask: parent_info.signal_mask,
            exit_code: None,
        };

        // Add child to process table
        {
            let mut table = self.process_table.lock();
            table.insert(child_id, child_info);
        }

        // Copy resource limits from parent
        {
            let parent_limits_opt = {
                let limits = self.resource_limits.lock();
                limits.get(&parent_id).cloned()
            };

            let mut limits = self.resource_limits.lock();
            if let Some(parent_limits) = parent_limits_opt {
                limits.insert(child_id, parent_limits);
            } else {
                limits.insert(child_id, ResourceLimits::default());
            }
        }

        // Update statistics
        {
            let stats = self.stats.lock();
            stats.total_processes.fetch_add(1, Ordering::SeqCst);
            stats.increment_total_forks();
            stats.increment_successful_forks();
        }

        let response = ProcessCreationResponse {
            process_id: child_id,
            thread_id: child_tid as u64,
            success: true,
            error_code: 0,
        };

        Ok(unsafe { core::slice::from_raw_parts(
            &response as *const _ as *const u8,
            core::mem::size_of::<ProcessCreationResponse>()
        ).to_vec() })
    }

    fn handle_exit_process(&self, data: &[u8], sender_id: u64) -> Result<Vec<u8>, i32> {
        if data.len() < 8 {
            return Err(EINVAL);
        }

        let process_id = sender_id;
        let exit_code = unsafe { *(data.as_ptr() as *const i32) };

        // Update process state to zombie
        {
            let mut table = self.process_table.lock();
            if let Some(process) = table.get_mut(&process_id) {
                process.state = ProcessState::Zombie;
                process.exit_code = Some(exit_code);
            }
        }

        // Update statistics
        {
            let stats = self.stats.lock();
            stats.increment_total_exits();
            stats.zombie_processes.fetch_add(1, Ordering::SeqCst);
            stats.running_processes.fetch_sub(1, Ordering::SeqCst);
        }

        Ok(vec![1]) // Success
    }

    fn handle_kill_process(&self, data: &[u8], sender_id: u64) -> Result<Vec<u8>, i32> {
        if data.len() < 8 {
            return Err(EINVAL);
        }

        let target_id = unsafe { *(data.as_ptr() as *const u64) };
        let signal = if data.len() >= 12 {
            unsafe { *((data.as_ptr() as *const u8).add(8) as *const i32) }
        } else {
            9 // SIGKILL
        };

        // Find target process
        {
            let mut table = self.process_table.lock();
            if let Some(process) = table.get_mut(&target_id) {
                match signal {
                    9 => { // SIGKILL
                        process.state = ProcessState::Dead;
                        table.remove(&target_id);
                    }
                    19 => { // SIGSTOP
                        process.state = ProcessState::Suspended;
                    }
                    18 => { // SIGCONT
                        if process.state == ProcessState::Suspended {
                            process.state = ProcessState::Runnable;
                        }
                    }
                    _ => {
                        // Add to pending signals
                        let mut signals = self.pending_signals.lock();
                        signals.entry(target_id)
                            .or_insert_with(Vec::new)
                            .push(ProcessSignal {
                                signal_number: signal,
                                sender_id: sender_id,
                                timestamp: crate::time::get_time_ns(),
                                data: None,
                            });
                    }
                }
            } else {
                return Err(ESRCH);
            }
        }

        Ok(vec![1]) // Success
    }

    fn handle_get_process_info(&self, data: &[u8], _sender_id: u64) -> Result<Vec<u8>, i32> {
        if data.len() < 8 {
            return Err(EINVAL);
        }

        let process_id = unsafe { *(data.as_ptr() as *const u64) };

        let process_info = {
            let table = self.process_table.lock();
            table.get(&process_id).cloned()
        };

        if let Some(info) = process_info {
            Ok(unsafe { core::slice::from_raw_parts(
                &info as *const _ as *const u8,
                core::mem::size_of::<ProcessInfo>()
            ).to_vec() })
        } else {
            Err(ESRCH)
        }
    }

    fn handle_set_process_priority(&self, data: &[u8], _sender_id: u64) -> Result<Vec<u8>, i32> {
        if data.len() < 12 {
            return Err(EINVAL);
        }

        let process_id = unsafe { *(data.as_ptr() as *const u64) };
        let priority = unsafe { *((data.as_ptr() as *const u8).add(8) as *const i32) };

        // Update process priority
        {
            let mut table = self.process_table.lock();
            if let Some(process) = table.get_mut(&process_id) {
                process.priority = priority;

                // Update scheduler priority if thread exists
                if let Some(scheduler) = crate::microkernel::scheduler::get_scheduler() {
                    let _ = scheduler.set_priority(process.process_id as usize, priority);
                }
            } else {
                return Err(ESRCH);
            }
        }

        Ok(vec![1]) // Success
    }

    fn handle_get_process_stats(&self) -> Result<Vec<u8>, i32> {
        // Update current statistics
        self.update_stats();

        let stats = self.stats.lock();
        Ok(unsafe { core::slice::from_raw_parts(
            &*stats as *const _ as *const u8,
            core::mem::size_of::<ProcessServiceStats>()
        ).to_vec() })
    }

    fn handle_process_signal(&self, data: &[u8], sender_id: u64) -> Result<Vec<u8>, i32> {
        if data.len() < 12 {
            return Err(EINVAL);
        }

        let target_id = unsafe { *(data.as_ptr() as *const u64) };
        let signal = unsafe { *((data.as_ptr() as *const u8).add(8) as *const i32) };

        // Add signal to pending queue
        {
            let mut signals = self.pending_signals.lock();
            signals.entry(target_id)
                .or_insert_with(Vec::new)
                .push(ProcessSignal {
                    signal_number: signal,
                    sender_id: sender_id,
                    timestamp: crate::time::get_time_ns(),
                    data: None,
                });
        }

        Ok(vec![1]) // Success
    }

    pub fn run_process_gc(&self) -> Result<usize, i32> {
        let mut cleaned_processes = 0;
        let current_time = crate::time::get_time_ns();

        // Clean up zombie processes that have been reaped
        {
            let mut table = self.process_table.lock();
            let mut limits = self.resource_limits.lock();
            let mut signals = self.pending_signals.lock();

            table.retain(|pid, process| {
                let should_remove = process.state == ProcessState::Zombie &&
                    (current_time - process.creation_time) > PROCESS_GC_INTERVAL;

                if should_remove {
                    cleaned_processes += 1;
                    limits.remove(pid);
                    signals.remove(pid);
                }

                !should_remove
            });
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.gc_count += 1;
            stats.last_gc_time = current_time;
            stats.orphaned_processes_collected += cleaned_processes as u64;
        }

        Ok(cleaned_processes)
    }

    pub fn update_stats(&self) {
        let table = self.process_table.lock();
        let mut stats = self.stats.lock();

        // Reset counters
        stats.running_processes.store(0, Ordering::SeqCst);
        stats.sleeping_processes.store(0, Ordering::SeqCst);
        stats.zombie_processes.store(0, Ordering::SeqCst);
        stats.suspended_processes.store(0, Ordering::SeqCst);

        let mut total_memory = 0usize;
        let mut total_processes = 0usize;

        for process in table.values() {
            total_processes += 1;
            total_memory += process.memory_usage.physical_memory;

            match process.state {
                ProcessState::Running | ProcessState::Runnable => {
                    stats.running_processes.fetch_add(1, Ordering::SeqCst);
                }
                ProcessState::Sleeping => {
                    stats.sleeping_processes.fetch_add(1, Ordering::SeqCst);
                }
                ProcessState::Zombie => {
                    stats.zombie_processes.fetch_add(1, Ordering::SeqCst);
                }
                ProcessState::Suspended => {
                    stats.suspended_processes.fetch_add(1, Ordering::SeqCst);
                }
                _ => {}
            }
        }

        stats.total_processes.store(total_processes, Ordering::SeqCst);

        // Calculate averages
        if total_processes > 0 {
            stats.average_memory_usage = total_memory as f64 / total_processes as f64;
        }
    }

    pub fn get_pending_signals(&self, process_id: u64) -> Vec<ProcessSignal> {
        let mut signals = self.pending_signals.lock();
        signals.remove(&process_id).unwrap_or_default()
    }

    pub fn enforce_resource_limits(&self) -> Vec<u64> {
        let mut violated_processes = Vec::new();
        let current_time = crate::time::get_time_ns();

        let table = self.process_table.lock();
        let limits = self.resource_limits.lock();

        for (pid, process) in table.iter() {
            if let Some(limit) = limits.get(pid) {
                // Check CPU time limit
                if process.cpu_time > limit.max_cpu_time {
                    violated_processes.push(*pid);
                    continue;
                }

                // Check memory limit
                if process.memory_usage.virtual_memory > limit.max_memory {
                    violated_processes.push(*pid);
                }

                // Check file descriptor limit
                if process.file_descriptors > limit.max_files {
                    violated_processes.push(*pid);
                }

                // Check process age (simple implementation)
                let age = current_time - process.creation_time;
                if age > limit.max_cpu_time {
                    violated_processes.push(*pid);
                }
            }
        }

        violated_processes
    }
}

// ============================================================================
// Global Process Service Instance
// ============================================================================

static mut GLOBAL_PROCESS_SERVICE: Option<Arc<ProcessManagementService>> = None;
static PROCESS_SERVICE_INIT: AtomicBool = AtomicBool::new(false);

/// Initialize process management service
pub fn init() -> Result<(), i32> {
    if PROCESS_SERVICE_INIT.load(Ordering::SeqCst) {
        return Ok(());
    }

    // Get required dependencies
    let scheduler = crate::microkernel::scheduler::get_scheduler()
        .ok_or(EFAULT)?;

    let ipc_manager = crate::microkernel::ipc::get_ipc_manager()
        .ok_or(EFAULT)?;

    let mut service = ProcessManagementService::new(scheduler, ipc_manager)?;

    // Register with service registry
    let registry = crate::microkernel::service_registry::get_service_registry()
        .ok_or(EFAULT)?;

    service.register_service(registry)?;

    // Set service to running state
    registry.update_service_status(service.service_id, ServiceStatus::Running)?;

    let arc_service = Arc::new(service);

    unsafe {
        GLOBAL_PROCESS_SERVICE = Some(arc_service);
    }

    PROCESS_SERVICE_INIT.store(true, Ordering::SeqCst);
    crate::println!("services/process: advanced process management service initialized");

    Ok(())
}

/// Get global process management service
pub fn get_process_service() -> Option<Arc<ProcessManagementService>> {
    unsafe {
        GLOBAL_PROCESS_SERVICE.clone()
    }
}

/// Legacy API compatibility functions

/// Create a new process by forking (legacy compatibility)
pub fn proc_fork() -> Option<usize> {
    if let Some(service) = get_process_service() {
        if let Ok(response_data) = service.handle_fork_process(&[], 0) {
            if response_data.len() >= core::mem::size_of::<ProcessCreationResponse>() {
                let response: ProcessCreationResponse = unsafe { core::ptr::read(response_data.as_ptr() as *const _) };
                if response.success {
                    return Some(response.process_id as usize);
                }
            }
        }
    }

    // Fallback to original implementation
    crate::process::fork().map(|pid| pid as usize)
}

/// Exit current process (legacy compatibility)
pub fn proc_exit(status: i32) {
    crate::process::exit(status);
}

/// Wait for child process to exit (legacy compatibility)
pub fn proc_wait(status: *mut i32) -> Option<usize> {
    crate::process::wait(status).map(|pid| pid as usize)
}

/// Kill a process (legacy compatibility)
pub fn proc_kill(pid: usize) -> bool {
    if let Some(service) = get_process_service() {
        let pid_data = (pid as u64).to_le_bytes();
        if service.handle_kill_process(&pid_data, 0).is_ok() {
            return true;
        }
    }

    // Fallback to original implementation
    crate::process::kill(pid)
}

/// Get current process ID (legacy compatibility)
pub fn proc_getpid() -> usize {
    crate::process::getpid() as usize
}

/// Get process statistics (legacy compatibility)
pub struct ProcessStats {
    pub total_processes: usize,
    pub runnable_processes: usize,
    pub sleeping_processes: usize,
    pub zombie_processes: usize,
}

pub fn proc_get_stats() -> ProcessStats {
    get_process_service()
        .map(|s| {
            let stats = s.stats.lock();
            ProcessStats {
                total_processes: stats.total_processes.load(Ordering::SeqCst),
                runnable_processes: stats.running_processes.load(Ordering::SeqCst),
                sleeping_processes: stats.sleeping_processes.load(Ordering::SeqCst),
                zombie_processes: stats.zombie_processes.load(Ordering::SeqCst),
            }
        })
        .unwrap_or_else(|| {
            let table = crate::process::PROC_TABLE.lock();
            let mut stats = ProcessStats {
                total_processes: 0,
                runnable_processes: 0,
                sleeping_processes: 0,
                zombie_processes: 0,
            };

            for proc in table.iter() {
                match proc.state {
                    ProcState::Runnable => {
                        stats.total_processes += 1;
                        stats.runnable_processes += 1;
                    }
                    ProcState::Sleeping => {
                        stats.total_processes += 1;
                        stats.sleeping_processes += 1;
                    }
                    ProcState::Zombie => {
                        stats.total_processes += 1;
                        stats.zombie_processes += 1;
                    }
                    ProcState::Running => {
                        stats.total_processes += 1;
                        stats.runnable_processes += 1;
                    }
                    _ => {}
                }
            }

            stats
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_state() {
        assert!(ProcessState::Running.is_alive());
        assert!(ProcessState::Runnable.is_alive());
        assert!(ProcessState::Sleeping.is_alive());
        assert!(!ProcessState::Zombie.is_alive());
        assert!(!ProcessState::Dead.is_alive());

        assert!(ProcessState::Running.is_running());
        assert!(ProcessState::Runnable.is_running());
        assert!(!ProcessState::Sleeping.is_running());
        assert!(ProcessState::Zombie.is_zombie());
    }

    #[test]
    fn test_resource_limits() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_cpu_time, u64::MAX);
        assert_eq!(limits.max_memory, usize::MAX);
        assert_eq!(limits.max_files, 1024);
        assert_eq!(limits.max_processes, 64);
    }

    #[test]
    fn test_process_service_stats() {
        let mut stats = ProcessServiceStats::new();

        stats.increment_total_forks();
        stats.increment_successful_forks();
        stats.increment_failed_forks();
        stats.increment_total_exits();

        assert_eq!(stats.total_forks.load(Ordering::SeqCst), 1);
        assert_eq!(stats.successful_forks.load(Ordering::SeqCst), 1);
        assert_eq!(stats.failed_forks.load(Ordering::SeqCst), 1);
        assert_eq!(stats.total_exits.load(Ordering::SeqCst), 1);
        assert_eq!(stats.get_fork_success_rate(), 1.0);
    }

    #[test]
    fn test_process_memory_usage() {
        let usage = ProcessMemoryUsage {
            virtual_memory: 1024 * 1024,
            physical_memory: 512 * 1024,
            shared_memory: 64 * 1024,
            code_memory: 128 * 1024,
            data_memory: 256 * 1024,
            stack_memory: 64 * 1024,
            heap_memory: 128 * 1024,
        };

        assert_eq!(usage.virtual_memory, 1024 * 1024);
        assert_eq!(usage.physical_memory, 512 * 1024);
        assert_eq!(usage.shared_memory, 64 * 1024);
    }
}