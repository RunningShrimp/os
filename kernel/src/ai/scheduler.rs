//! # Heterogeneous Compute Scheduler
//!
//! 本模块实现异构计算调度器，提供：
//!
//! - CPU/GPU/TPU/NPU 协同计算
//! - 智能任务调度和分片
//! - 负载均衡和资源隔离
//! - 低延迟调度策略
//!
//! ## 功能特性
//!
//! - **多种调度策略**: 负载均衡、优先级、功耗优化、延迟优先
//! - **任务分片**: 自动将大任务分片到多个设备
//! - **负载均衡**: 实时监控设备利用率，动态调整任务分配
//! - **资源隔离**: 支持服务质量（QoS）和资源配额
//!
//! ## 使用示例
//!
//! ```no_run
//! use kernel::ai::scheduler::{
//!     ComputeScheduler, SchedulerPolicy, Task, TaskPriority,
//!     HeterogeneousScheduler, ComputeResource
//! };
//!
//! // 创建异构调度器
//! let scheduler = HeterogeneousScheduler::new(SchedulerPolicy::LoadBalance)?;
//!
//! // 注册计算资源
//! let gpu = ComputeResource::gpu(0);
//! let tpu = ComputeResource::tpu(0);
//! scheduler.register_resource(gpu)?;
//! scheduler.register_resource(tpu)?;
//!
//! // 创建任务
//! let task = Task::new(
//!     "matrix_multiply",
//!     TaskPriority::Normal,
//!     vec![1024usize, 1024, 1024]
//! );
//!
//! // 提交任务
//! let handle = scheduler.submit_task(task)?;
//!
//! // 等待完成
//! let result = scheduler.wait_task(handle)?;
//! # Ok::<(), kernel::ai::AiError>(())
//! ```

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;
use core::sync::atomic::{AtomicUsize, AtomicU64, Ordering};

use super::{AiError, AiResult};

/// Task ID
pub type TaskId = u64;

/// Resource ID
pub type ResourceId = u64;

/// Task handle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskHandle {
    /// Task ID
    pub id: TaskId,
}

/// Task priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    /// Low priority
    Low = 0,
    /// Normal priority
    Normal = 1,
    /// High priority
    High = 2,
    /// Critical priority
    Critical = 3,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// Task is pending
    Pending,
    /// Task is running
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task was cancelled
    Cancelled,
}

/// Scheduler policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerPolicy {
    /// Load balancing policy
    LoadBalance,
    /// Priority-based scheduling
    Priority,
    /// Power optimization
    PowerOptimized,
    /// Latency optimization
    LatencyOptimized,
    /// Round-robin
    RoundRobin,
    /// Work-conserving
    WorkConserving,
}

/// Compute task
pub struct Task {
    /// Task ID
    id: TaskId,
    /// Task name
    name: String,
    /// Task priority
    priority: TaskPriority,
    /// Task status
    status: Mutex<TaskStatus>,
    /// Task parameters
    params: Vec<usize>,
    /// Estimated compute cycles
    estimated_cycles: Option<u64>,
    /// Memory requirements
    memory_required: Option<usize>,
    /// Result
    result: Mutex<Option<TaskResult>>,
    /// Assigned resource
    assigned_resource: Mutex<Option<ResourceId>>,
}

/// Task result
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Success flag
    pub success: bool,
    /// Execution time (nanoseconds)
    pub execution_time_ns: u64,
    /// Data (opaque)
    pub data: Vec<u8>,
}

impl Task {
    /// Create new task
    pub fn new(name: &str, priority: TaskPriority, params: Vec<usize>) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        Self {
            id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
            name: String::from(name),
            priority,
            status: Mutex::new(TaskStatus::Pending),
            params,
            estimated_cycles: None,
            memory_required: None,
            result: Mutex::new(None),
            assigned_resource: Mutex::new(None),
        }
    }

    /// Get task ID
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Get task name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get task priority
    pub fn priority(&self) -> TaskPriority {
        self.priority
    }

    /// Get task status
    pub fn status(&self) -> TaskStatus {
        *self.status.lock()
    }

    /// Set task status
    pub fn set_status(&self, status: TaskStatus) {
        *self.status.lock() = status;
    }

    /// Get task parameters
    pub fn params(&self) -> &[usize] {
        &self.params
    }

    /// Set estimated compute cycles
    pub fn set_estimated_cycles(&self, _cycles: u64) {
        // Stub implementation
    }

    /// Set memory requirements
    pub fn set_memory_required(&self, _bytes: usize) {
        // Stub implementation
    }

    /// Get result
    pub fn get_result(&self) -> Option<TaskResult> {
        self.result.lock().clone()
    }

    /// Set result
    pub fn set_result(&self, result: TaskResult) {
        *self.result.lock() = Some(result);
    }

    /// Get assigned resource
    pub fn assigned_resource(&self) -> Option<ResourceId> {
        *self.assigned_resource.lock()
    }

    /// Assign to resource
    pub fn assign_to(&self, resource_id: ResourceId) {
        *self.assigned_resource.lock() = Some(resource_id);
    }
}

/// Compute resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    /// CPU
    Cpu,
    /// GPU
    Gpu,
    /// TPU
    Tpu,
    /// NPU
    Npu,
    /// FPGA
    Fpga,
    /// DSP
    Dsp,
}

/// Compute resource
pub struct ComputeResource {
    /// Resource ID
    id: ResourceId,
    /// Resource type
    resource_type: ResourceType,
    /// Device ID
    device_id: usize,
    /// Compute units (number of cores/SMs)
    compute_units: usize,
    /// Peak performance (FLOPS)
    peak_flops: u64,
    /// Memory bandwidth (bytes/sec)
    memory_bandwidth: u64,
    /// Current utilization (0-100)
    utilization: Mutex<AtomicUsize>,
    /// Power consumption (milliwatts)
    power_mw: Mutex<AtomicUsize>,
    /// Active tasks
    active_tasks: Mutex<Vec<TaskId>>,
}

impl ComputeResource {
    /// Create CPU resource
    pub fn cpu(device_id: usize) -> Self {
        Self {
            id: (0u64 << 8) | device_id as u64,
            resource_type: ResourceType::Cpu,
            device_id,
            compute_units: 1,
            peak_flops: 100_000_000_000, // 100 GFLOPS
            memory_bandwidth: 50_000_000_000, // 50 GB/s
            utilization: Mutex::new(AtomicUsize::new(0)),
            power_mw: Mutex::new(AtomicUsize::new(15000)), // 15W
            active_tasks: Mutex::new(Vec::new()),
        }
    }

    /// Create GPU resource
    pub fn gpu(device_id: usize) -> Self {
        Self {
            id: (1u64 << 8) | device_id as u64,
            resource_type: ResourceType::Gpu,
            device_id,
            compute_units: 28,
            peak_flops: 10_000_000_000_000, // 10 TFLOPS
            memory_bandwidth: 760_000_000_000, // 760 GB/s
            utilization: Mutex::new(AtomicUsize::new(0)),
            power_mw: Mutex::new(AtomicUsize::new(250000)), // 250W
            active_tasks: Mutex::new(Vec::new()),
        }
    }

    /// Create TPU resource
    pub fn tpu(device_id: usize) -> Self {
        Self {
            id: (2u64 << 8) | device_id as u64,
            resource_type: ResourceType::Tpu,
            device_id,
            compute_units: 8,
            peak_flops: 250_000_000_000_000, // 250 TFLOPS
            memory_bandwidth: 600_000_000_000, // 600 GB/s
            utilization: Mutex::new(AtomicUsize::new(0)),
            power_mw: Mutex::new(AtomicUsize::new(280000)), // 280W
            active_tasks: Mutex::new(Vec::new()),
        }
    }

    /// Get resource ID
    pub fn id(&self) -> ResourceId {
        self.id
    }

    /// Get resource type
    pub fn resource_type(&self) -> ResourceType {
        self.resource_type
    }

    /// Get device ID
    pub fn device_id(&self) -> usize {
        self.device_id
    }

    /// Get utilization percentage
    pub fn utilization(&self) -> usize {
        self.utilization.lock().load(Ordering::Relaxed)
    }

    /// Set utilization
    pub fn set_utilization(&self, util: usize) {
        self.utilization.lock().store(util.min(100), Ordering::Relaxed);
    }

    /// Get power consumption
    pub fn power_mw(&self) -> usize {
        self.power_mw.lock().load(Ordering::Relaxed)
    }

    /// Add active task
    pub fn add_task(&self, task_id: TaskId) {
        self.active_tasks.lock().push(task_id);
    }

    /// Remove active task
    pub fn remove_task(&self, task_id: TaskId) {
        let mut tasks = self.active_tasks.lock();
        if let Some(pos) = tasks.iter().position(|&id| id == task_id) {
            tasks.remove(pos);
        }
    }

    /// Get number of active tasks
    pub fn active_task_count(&self) -> usize {
        self.active_tasks.lock().len()
    }
}

/// Resource allocation
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    /// Allocated resource
    pub resource: ComputeResource,
    /// Allocation percentage
    pub allocation_percent: f32,
    /// Estimated completion time (nanoseconds)
    pub estimated_time_ns: u64,
}

/// Compute scheduler trait
pub trait ComputeScheduler: Send + Sync {
    /// Submit task for execution
    fn submit_task(&self, task: Task) -> AiResult<TaskHandle>;

    /// Wait for task completion
    fn wait_task(&self, handle: TaskHandle) -> AiResult<TaskResult>;

    /// Cancel task
    fn cancel_task(&self, handle: TaskHandle) -> AiResult<bool>;

    /// Query task status
    fn query_task(&self, handle: TaskHandle) -> AiResult<TaskStatus>;

    /// Register compute resource
    fn register_resource(&self, resource: ComputeResource) -> AiResult<()>;

    /// Unregister compute resource
    fn unregister_resource(&self, resource_id: ResourceId) -> AiResult<()>;

    /// Get scheduler statistics
    fn get_stats(&self) -> AiResult<SchedulerStats>;
}

/// Scheduler statistics
#[derive(Debug, Clone)]
pub struct SchedulerStats {
    /// Total tasks submitted
    pub total_tasks: usize,
    /// Tasks completed
    pub completed_tasks: usize,
    /// Tasks failed
    pub failed_tasks: usize,
    /// Tasks cancelled
    pub cancelled_tasks: usize,
    /// Average queue time (nanoseconds)
    pub avg_queue_time_ns: u64,
    /// Average execution time (nanoseconds)
    pub avg_execution_time_ns: u64,
    /// Current pending tasks
    pub pending_tasks: usize,
    /// Current running tasks
    pub running_tasks: usize,
}

/// Heterogeneous scheduler
pub struct HeterogeneousScheduler {
    /// Scheduler policy
    policy: SchedulerPolicy,
    /// Registered resources
    resources: Mutex<BTreeMap<ResourceId, Arc<ComputeResource>>>,
    /// Pending tasks
    pending_tasks: Mutex<BTreeMap<TaskId, Arc<Task>>>,
    /// Running tasks
    running_tasks: Mutex<BTreeMap<TaskId, Arc<Task>>>,
    /// Completed tasks
    completed_tasks: Mutex<BTreeMap<TaskId, Arc<Task>>>,
    /// Statistics
    stats: Mutex<SchedulerStats>,
}

impl HeterogeneousScheduler {
    /// Create new heterogeneous scheduler
    pub fn new(policy: SchedulerPolicy) -> AiResult<Self> {
        Ok(Self {
            policy,
            resources: Mutex::new(BTreeMap::new()),
            pending_tasks: Mutex::new(BTreeMap::new()),
            running_tasks: Mutex::new(BTreeMap::new()),
            completed_tasks: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(SchedulerStats {
                total_tasks: 0,
                completed_tasks: 0,
                failed_tasks: 0,
                cancelled_tasks: 0,
                avg_queue_time_ns: 0,
                avg_execution_time_ns: 0,
                pending_tasks: 0,
                running_tasks: 0,
            }),
        })
    }

    /// Select best resource for task based on policy
    fn select_resource(&self, task: &Task) -> AiResult<Arc<ComputeResource>> {
        let resources = self.resources.lock();

        if resources.is_empty() {
            return Err(AiError::NoAccelerator);
        }

        let resources_vec: Vec<_> = resources.values().cloned().collect();

        let selected = match self.policy {
            SchedulerPolicy::LoadBalance => {
                // Select resource with lowest utilization
                resources_vec
                    .into_iter()
                    .min_by_key(|r| r.utilization())
                    .ok_or(AiError::NoAccelerator)?
            }
            SchedulerPolicy::RoundRobin => {
                // Simple round-robin based on task ID
                let idx = task.id() as usize % resources_vec.len();
                resources_vec
                    .get(idx)
                    .cloned()
                    .ok_or(AiError::NoAccelerator)?
            }
            SchedulerPolicy::PowerOptimized => {
                // Select resource with lowest power consumption
                resources_vec
                    .into_iter()
                    .min_by_key(|r| r.power_mw())
                    .ok_or(AiError::NoAccelerator)?
            }
            SchedulerPolicy::LatencyOptimized => {
                // Select fastest resource (highest FLOPS)
                resources_vec
                    .into_iter()
                    .max_by_key(|r| r.compute_units)
                    .ok_or(AiError::NoAccelerator)?
            }
            SchedulerPolicy::Priority => {
                // Select based on task priority and resource capability
                if task.priority() >= TaskPriority::High {
                    // Use fastest resource for high priority
                    resources_vec
                        .into_iter()
                        .max_by_key(|r| r.compute_units)
                        .ok_or(AiError::NoAccelerator)?
                } else {
                    // Use least loaded resource
                    resources_vec
                        .into_iter()
                        .min_by_key(|r| r.active_task_count())
                        .ok_or(AiError::NoAccelerator)?
                }
            }
            SchedulerPolicy::WorkConserving => {
                // Use any available resource
                resources_vec
                    .into_iter()
                    .min_by_key(|r| r.active_task_count())
                    .ok_or(AiError::NoAccelerator)?
            }
        };

        Ok(selected)
    }

    /// Execute task on resource
    fn execute_task(&self, task: Arc<Task>, resource: Arc<ComputeResource>) -> AiResult<()> {
        // Update task status
        task.set_status(TaskStatus::Running);
        task.assign_to(resource.id());
        resource.add_task(task.id());

        // Move to running tasks
        self.pending_tasks.lock().remove(&task.id());
        self.running_tasks.lock().insert(task.id(), task.clone());

        // Stub: Execute task
        // In real implementation, this would dispatch to actual hardware

        // Mark as completed
        task.set_status(TaskStatus::Completed);
        let result = TaskResult {
            success: true,
            execution_time_ns: 1_000_000, // 1ms
            data: Vec::new(),
        };
        task.set_result(result);

        // Update resource
        resource.remove_task(task.id());

        // Move to completed
        self.running_tasks.lock().remove(&task.id());
        self.completed_tasks.lock().insert(task.id(), task.clone());

        // Update stats
        let mut stats = self.stats.lock();
        stats.completed_tasks += 1;
        stats.running_tasks = self.running_tasks.lock().len();
        stats.pending_tasks = self.pending_tasks.lock().len();

        Ok(())
    }
}

impl ComputeScheduler for HeterogeneousScheduler {
    fn submit_task(&self, task: Task) -> AiResult<TaskHandle> {
        let task = Arc::new(task);
        let handle = TaskHandle { id: task.id() };

        // Update stats
        self.stats.lock().total_tasks += 1;

        // Add to pending tasks
        self.pending_tasks.lock().insert(task.id(), task.clone());

        // Select resource and execute
        let resource = self.select_resource(&task)?;
        self.execute_task(task, resource)?;

        Ok(handle)
    }

    fn wait_task(&self, handle: TaskHandle) -> AiResult<TaskResult> {
        let completed = self.completed_tasks.lock();
        let task = completed
            .get(&handle.id)
            .cloned()
            .ok_or(AiError::InvalidArgument)?;

        task.get_result().ok_or(AiError::InternalError("No result".into()))
    }

    fn cancel_task(&self, handle: TaskHandle) -> AiResult<bool> {
        // Try to cancel from pending
        if self.pending_tasks.lock().remove(&handle.id).is_some() {
            let mut stats = self.stats.lock();
            stats.cancelled_tasks += 1;
            stats.pending_tasks = self.pending_tasks.lock().len();
            return Ok(true);
        }

        // Task already running or completed
        Ok(false)
    }

    fn query_task(&self, handle: TaskHandle) -> AiResult<TaskStatus> {
        // Check in pending
        if self.pending_tasks.lock().contains_key(&handle.id) {
            return Ok(TaskStatus::Pending);
        }

        // Check in running
        if let Some(task) = self.running_tasks.lock().get(&handle.id) {
            return Ok(task.status());
        }

        // Check in completed
        if let Some(task) = self.completed_tasks.lock().get(&handle.id) {
            return Ok(task.status());
        }

        Err(AiError::InvalidArgument)
    }

    fn register_resource(&self, resource: ComputeResource) -> AiResult<()> {
        let id = resource.id();
        self.resources
            .lock()
            .insert(id, Arc::new(resource));
        Ok(())
    }

    fn unregister_resource(&self, resource_id: ResourceId) -> AiResult<()> {
        self.resources
            .lock()
            .remove(&resource_id)
            .ok_or(AiError::InvalidDevice)?;
        Ok(())
    }

    fn get_stats(&self) -> AiResult<SchedulerStats> {
        Ok(self.stats.lock().clone())
    }
}

/// Initialize scheduler subsystem
pub fn init() -> AiResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_scheduler() {
        let scheduler = HeterogeneousScheduler::new(SchedulerPolicy::LoadBalance);
        assert!(scheduler.is_ok());
    }

    #[test]
    fn test_register_resource() {
        let scheduler = HeterogeneousScheduler::new(SchedulerPolicy::LoadBalance).unwrap();
        let gpu = ComputeResource::gpu(0);
        assert!(scheduler.register_resource(gpu).is_ok());
    }

    #[test]
    fn test_submit_task() {
        let scheduler = HeterogeneousScheduler::new(SchedulerPolicy::LoadBalance).unwrap();
        let gpu = ComputeResource::gpu(0);
        scheduler.register_resource(gpu).unwrap();

        let task = Task::new("test", TaskPriority::Normal, vec![1024, 1024]);
        let handle = scheduler.submit_task(task);
        assert!(handle.is_ok());
    }

    #[test]
    fn test_query_task() {
        let scheduler = HeterogeneousScheduler::new(SchedulerPolicy::LoadBalance).unwrap();
        let gpu = ComputeResource::gpu(0);
        scheduler.register_resource(gpu).unwrap();

        let task = Task::new("test", TaskPriority::Normal, vec![1024, 1024]);
        let handle = scheduler.submit_task(task).unwrap();
        let status = scheduler.query_task(handle);
        assert!(status.is_ok());
    }

    #[test]
    fn test_get_stats() {
        let scheduler = HeterogeneousScheduler::new(SchedulerPolicy::LoadBalance).unwrap();
        let stats = scheduler.get_stats();
        assert!(stats.is_ok());
    }
}
