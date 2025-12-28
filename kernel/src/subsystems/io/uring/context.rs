//! io_uring Context Management
//!
//! This module implements io_uring context management:
//! - io_uring instance creation and destruction
//! - Multi-queue management
//! - io_uring configuration
//! - io_uring statistics
//!
//! Features:
//! - io_uring v2 interface
//! - Multiple io_uring instances
//! - Per-instance statistics
//! - Context lifecycle management

use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;

use super::queue::*;
use core::sync::atomic;
use super::buffers::*;
use core::sync::atomic;
use super::ops::*;
use core::sync::atomic;

// ============================================================================
// io_uring Context Constants
// ============================================================================

/// io_uring instance ID
pub type IoringInstanceId = u32;

/// Invalid instance ID
pub const INVALID_INSTANCE_ID: IoringInstanceId = 0;

/// Default instance name
pub const DEFAULT_INSTANCE_NAME: &str = "default_uring";

/// Maximum io_uring instances
pub const MAX_INSTANCES: usize = 256;

// ============================================================================
// io_uring Instance
// ============================================================================

/// io_uring instance
#[derive(Debug, Clone)]
pub struct IoringInstance {
    /// Instance ID
    pub instance_id: IoringInstanceId,
    
    /// Instance name
    pub name: String,
    
    /// Submission queue
    pub submission_queue: Arc<SubmissionQueue>,
    
    /// Completion queue
    pub completion_queue: Arc<CompletionQueue>,
    
    /// Buffer manager
    pub buffer_manager: Arc<BufferManager>,
    
    /// I/O operation manager
    pub op_manager: Arc<IoOpManager>,
    
    /// Instance flags
    pub flags: IoringInstanceFlags,
    
    /// Queue depth
    pub queue_depth: usize,
    
    /// Number of CPUs
    pub num_cpus: usize,
    
    /// Instance state
    pub state: AtomicU32, // Stores IoringInstanceState as u32
    
    /// Creation time
    pub created_at: u64,
    
    /// Last activity time
    pub last_activity: AtomicU64,
    
    /// Instance statistics
    pub stats: Mutex<IoringInstanceStats>,
}

/// io_uring instance flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoringInstanceFlags {
    /// Enable polling mode
    pub poll_enabled: bool,
    
    /// Enable interrupt mode
    pub interrupt_enabled: bool,
    
    /// Enable single issuer mode
    pub single_issuer: bool,
    
    /// Enable kernel-side buffer
    pub kernel_side_buffer: bool,
    
    /// Enable taskfile mode
    pub taskfile_mode: bool,
    
    /// Enable SQPOLL mode
    pub sqpoll_mode: bool,
    
    /// Enable busy polling
    pub busy_poll: bool,
}

impl Default for IoringInstanceFlags {
    fn default() -> Self {
        Self {
            poll_enabled: true,
            interrupt_enabled: true,
            single_issuer: false,
            kernel_side_buffer: true,
            taskfile_mode: false,
            sqpoll_mode: false,
            busy_poll: false,
        }
    }
}

/// io_uring instance state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoringInstanceState {
    /// Instance is created but not initialized
    Created,
    
    /// Instance is initialized and ready
    Initialized,
    
    /// Instance is active
    Active,
    
    /// Instance is paused
    Paused,
    
    /// Instance is destroyed
    Destroyed,
}

/// io_uring instance statistics
#[derive(Debug, Clone, Copy)]
pub struct IoringInstanceStats {
    /// Total submissions
    pub total_submissions: u64,
    
    /// Total completions
    pub total_completions: u64,
    
    /// Successful I/O operations
    pub successful_ios: u64,
    
    /// Failed I/O operations
    pub failed_ios: u64,
    
    /// Cancelled operations
    pub cancelled_ios: u64,
    
    /// Timed out operations
    pub timeout_ios: u64,
    
    /// Total bytes transferred
    pub total_bytes_transferred: u64,
    
    /// Average I/O latency (nanoseconds)
    pub average_io_latency_ns: u64,
    
    /// Peak I/O depth
    pub peak_io_depth: usize,
    
    /// Current I/O depth
    pub current_io_depth: usize,
    
    /// Total interrupts received
    pub total_interrupts: u64,
    
    /// Total polls performed
    pub total_polls: u64,
}

impl Default for IoringInstanceStats {
    fn default() -> Self {
        Self {
            total_submissions: 0,
            total_completions: 0,
            successful_ios: 0,
            failed_ios: 0,
            cancelled_ios: 0,
            timeout_ios: 0,
            total_bytes_transferred: 0,
            average_io_latency_ns: 0,
            peak_io_depth: 0,
            current_io_depth: 0,
            total_interrupts: 0,
            total_polls: 0,
        }
    }
}

impl IoringInstance {
    /// Create new io_uring instance
    pub fn new(instance_id: IoringInstanceId, name: String, queue_depth: usize,
               num_cpus: usize, flags: IoringInstanceFlags) -> Self {
        
        let queue_flags = IoringQueueFlags {
            kernel_side_buffer: flags.kernel_side_buffer,
            disable_kernel_buffer: !flags.kernel_side_buffer,
            taskfile_mode: flags.taskfile_mode,
            sqpoll_mode: flags.sqpoll_mode,
            single_issuer: flags.single_issuer,
        };
        
        let submission_queue = Arc::new(SubmissionQueue::new(queue_depth, queue_flags));
        let completion_queue = Arc::new(CompletionQueue::new(queue_depth, queue_flags));
        let buffer_manager = Arc::new(BufferManager::new());
        let op_manager = Arc::new(IoOpManager::new());
        
        Self {
            instance_id,
            name,
            submission_queue,
            completion_queue,
            buffer_manager,
            op_manager,
            flags,
            queue_depth,
            num_cpus,
            state: AtomicU32::new(IoringInstanceState::Created as u32),
            created_at: crate::subsystems::time::timestamp_nanos(),
            last_activity: AtomicU64::new(crate::subsystems::time::timestamp_nanos()),
            stats: Mutex::new(IoringInstanceStats::default()),
        }
    }
    
    /// Initialize instance
    pub fn initialize(&self) -> Result<(), IoringError> {
        // Set state to initialized
        self.state.store(IoringInstanceState::Initialized as u32, Ordering::Release);
        
        crate::println!("[io_uring] Initialized io_uring instance {} ({})",
                        self.instance_id, self.name);
        
        Ok(())
    }
    
    /// Activate instance
    pub fn activate(&self) {
        // Set state to active
        self.state.store(IoringInstanceState::Active as u32, Ordering::Release);
        self.last_activity.store(crate::subsystems::time::timestamp_nanos(), Ordering::Release);
        
        crate::println!("[io_uring] Activated io_uring instance {}", self.instance_id);
    }
    
    /// Pause instance
    pub fn pause(&self) {
        // Set state to paused
        self.state.store(IoringInstanceState::Paused as u32, Ordering::Release);
        
        crate::println!("[io_uring] Paused io_uring instance {}", self.instance_id);
    }
    
    /// Destroy instance
    pub fn destroy(&self) {
        // Set state to destroyed
        self.state.store(IoringInstanceState::Destroyed as u32, Ordering::Release);
        
        crate::println!("[io_uring] Destroyed io_uring instance {}", self.instance_id);
    }
    
    /// Update activity timestamp
    pub fn update_activity(&self) {
        self.last_activity.store(crate::subsystems::time::timestamp_nanos(), Ordering::Release);
    }
    
    /// Get instance state
    pub fn get_state(&self) -> IoringInstanceState {
        unsafe {
            core::mem::transmute_copy(self.state.load(Ordering::Acquire))
        }
    }
    
    /// Check if instance is active
    pub fn is_active(&self) -> bool {
        self.get_state() == IoringInstanceState::Active
    }
    
    /// Check if instance is paused
    pub fn is_paused(&self) -> bool {
        self.get_state() == IoringInstanceState::Paused
    }
    
    /// Get instance statistics
    pub fn get_stats(&self) -> IoringInstanceStats {
        let mut stats = self.stats.lock();
        
        // Update statistics from subcomponents
        let sq_stats = self.submission_queue.get_stats();
        let cq_stats = self.completion_queue.get_stats();
        let op_stats = self.op_manager.get_stats();
        
        stats.total_submissions = sq_stats.total_submissions;
        stats.total_completions = cq_stats.total_submissions;
        stats.successful_ios = cq_stats.successful_submissions;
        stats.failed_ios = cq_stats.failed_submissions;
        stats.current_io_depth = op_stats.active_operations;
        stats.cancelled_ios = op_stats.cancelled_operations;
        stats.timeout_ios = op_stats.timeout_operations;
        
        if stats.total_completions > 0 {
            stats.average_io_latency_ns = 0; // Would need actual latency tracking
        }
        
        *stats
    }
    
    /// Get idle time (nanoseconds since last activity)
    pub fn get_idle_ns(&self) -> u64 {
        crate::subsystems::time::timestamp_nanos() - self.last_activity.load(Ordering::Acquire)
    }
}

/// io_uring context error
#[derive(Debug, Clone)]
pub enum IoringError {
    /// Invalid parameters
    InvalidParameters,
    
    /// Instance not found
    InstanceNotFound {
        instance_id: IoringInstanceId,
    },
    
    /// Instance already exists
    InstanceAlreadyExists {
        instance_id: IoringInstanceId,
    },
    
    /// Queue creation failed
    QueueCreationFailed {
        reason: String,
    },
    
    /// Buffer allocation failed
    BufferAllocationFailed {
        reason: String,
    },
    
    /// Initialization failed
    InitializationFailed {
        reason: String,
    },
}

// ============================================================================
// io_uring Context Manager
// ============================================================================

/// io_uring context manager
pub struct IoringContextManager {
    /// io_uring instances
    pub instances: Mutex<BTreeMap<IoringInstanceId, Arc<IoringInstance>>>,
    
    /// Next instance ID
    pub next_instance_id: AtomicU32,
    
    /// Total instances created
    pub total_instances: AtomicUsize,
    
    /// Active instances count
    pub active_instances: AtomicUsize,
    
    /// Manager statistics
    pub stats: Mutex<IoringContextStats>,
    
    /// Number of CPUs
    pub num_cpus: usize,
}

/// io_uring context statistics
#[derive(Debug, Clone, Copy)]
pub struct IoringContextStats {
    pub total_instances: usize,
    pub active_instances: usize,
    pub paused_instances: usize,
    pub destroyed_instances: usize,
    pub total_submissions: u64,
    pub total_completions: u64,
    pub total_bytes_transferred: u64,
    pub peak_io_depth: usize,
}

impl Default for IoringContextStats {
    fn default() -> Self {
        Self {
            total_instances: 0,
            active_instances: 0,
            paused_instances: 0,
            destroyed_instances: 0,
            total_submissions: 0,
            total_completions: 0,
            total_bytes_transferred: 0,
            peak_io_depth: 0,
        }
    }
}

impl IoringContextManager {
    /// Create new io_uring context manager
    pub fn new(num_cpus: usize) -> Self {
        Self {
            instances: Mutex::new(BTreeMap::new()),
            next_instance_id: AtomicU32::new(1),
            total_instances: AtomicUsize::new(0),
            active_instances: AtomicUsize::new(0),
            stats: Mutex::new(IoringContextStats::default()),
            num_cpus,
        }
    }
    
    /// Create io_uring instance
    pub fn create_instance(&self, name: String, queue_depth: usize, 
                        flags: IoringInstanceFlags) -> Result<IoringInstanceId, IoringError> {
        
        // Validate queue depth
        if queue_depth < MIN_QUEUE_ENTRIES || queue_depth > MAX_QUEUE_ENTRIES {
            return Err(IoringError::InvalidParameters);
        }
        
        let instance_id = self.next_instance_id.fetch_add(1, Ordering::Relaxed);
        
        let instance = Arc::new(IoringInstance::new(instance_id, name, queue_depth,
                                                     self.num_cpus, flags));
        
        let mut instances = self.instances.lock();
        instances.insert(instance_id, instance);
        self.total_instances.fetch_add(1, Ordering::Relaxed);
        
        crate::println!("[io_uring] Created io_uring instance {} ({})",
                        instance_id, name);
        
        Ok(instance_id)
    }
    
    /// Get instance by ID
    pub fn get_instance(&self, instance_id: IoringInstanceId) 
        -> Option<Arc<IoringInstance>> {
        
        let instances = self.instances.lock();
        instances.get(&instance_id).cloned()
    }
    
    /// Destroy instance
    pub fn destroy_instance(&self, instance_id: IoringInstanceId) 
        -> Result<(), IoringError> {
        
        let mut instances = self.instances.lock();
        
        if let Some(instance) = instances.remove(&instance_id) {
            instance.destroy();
            
            crate::println!("[io_uring] Destroyed io_uring instance {}", instance_id);
            
            Ok(())
        } else {
            Err(IoringError::InstanceNotFound { instance_id })
        }
    }
    
    /// Get all instances
    pub fn get_all_instances(&self) -> Vec<Arc<IoringInstance>> {
        let instances = self.instances.lock();
        instances.values().cloned().collect()
    }
    
    /// Get active instances
    pub fn get_active_instances(&self) -> Vec<Arc<IoringInstance>> {
        let instances = self.instances.lock();
        instances.values().filter(|i| i.is_active()).cloned().collect()
    }
    
    /// Get context statistics
    pub fn get_stats(&self) -> IoringContextStats {
        let mut stats = self.stats.lock();
        
        stats.total_instances = self.total_instances.load(Ordering::Relaxed);
        
        let instances = self.instances.lock();
        
        stats.active_instances = instances.values().filter(|i| i.is_active()).count();
        stats.paused_instances = instances.values().filter(|i| i.is_paused()).count();
        stats.destroyed_instances = instances.values().filter(|i| {
            i.get_state() == IoringInstanceState::Destroyed
        }).count();
        
        // Aggregate statistics from all instances
        let mut total_submissions = 0u64;
        let mut total_completions = 0u64;
        let mut total_bytes = 0u64;
        let mut max_depth = 0usize;
        
        for instance in instances.values() {
            let instance_stats = instance.get_stats();
            total_submissions += instance_stats.total_submissions;
            total_completions += instance_stats.total_completions;
            total_bytes += instance_stats.total_bytes_transferred;
            if instance_stats.current_io_depth > max_depth {
                max_depth = instance_stats.current_io_depth;
            }
        }
        
        stats.total_submissions = total_submissions;
        stats.total_completions = total_completions;
        stats.total_bytes_transferred = total_bytes;
        stats.peak_io_depth = max_depth;
        
        *stats
    }
    
    /// Destroy all instances
    pub fn destroy_all(&self) {
        let mut instances = self.instances.lock();
        let count = instances.len();
        
        for (instance_id, instance) in instances.iter() {
            instance.destroy();
            crate::println!("[io_uring] Destroyed io_uring instance {}", instance_id);
        }
        
        instances.clear();
        
        crate::println!("[io_uring] Destroyed all {} io_uring instances", count);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ioring_instance_creation() {
        let instance = IoringInstance::new(
            1,
            String::from("test"),
            128,
            4,
            IoringInstanceFlags::default()
        );
        
        assert_eq!(instance.instance_id, 1);
        assert_eq!(instance.name, "test");
        assert_eq!(instance.queue_depth, 128);
        assert_eq!(instance.get_state(), IoringInstanceState::Created);
    }

    #[test]
    fn test_ioring_instance_lifecycle() {
        let instance = IoringInstance::new(
            1,
            String::from("test"),
            128,
            4,
            IoringInstanceFlags::default()
        );
        
        instance.initialize().unwrap();
        assert_eq!(instance.get_state(), IoringInstanceState::Initialized);
        
        instance.activate();
        assert!(instance.is_active());
        assert_eq!(instance.get_state(), IoringInstanceState::Active);
        
        instance.pause();
        assert!(instance.is_paused());
        assert_eq!(instance.get_state(), IoringInstanceState::Paused);
        
        instance.destroy();
        assert_eq!(instance.get_state(), IoringInstanceState::Destroyed);
    }

    #[test]
    fn test_ioring_context_manager() {
        let manager = IoringContextManager::new(4);
        
        let instance_id = manager.create_instance(
            String::from("test"),
            128,
            IoringInstanceFlags::default()
        ).unwrap();
        
        let instance = manager.get_instance(instance_id).unwrap();
        assert_eq!(instance.instance_id, instance_id);
        
        manager.destroy_instance(instance_id).unwrap();
        assert!(manager.get_instance(instance_id).is_none());
    }

    #[test]
    fn test_ioring_statistics() {
        let manager = IoringContextManager::new(4);
        
        let instance_id = manager.create_instance(
            String::from("test"),
            128,
            IoringInstanceFlags::default()
        ).unwrap();
        
        let stats = manager.get_stats();
        assert_eq!(stats.total_instances, 1);
        assert_eq!(stats.active_instances, 0);
    }
}
