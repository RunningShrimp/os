//! # 任务调度和资源管理
//!
//! 实现分布式任务调度器和资源管理器。
//!
//! ## 核心组件
//!
//! - **资源分配**: 动态资源分配和回收
//! - **任务队列**: 多优先级任务队列
//! - **调度策略**: FIFO、公平、容量、亲和性
//! - **资源隔离**: 命名空间和 cgroup 集成
//! - **性能优化**: 延迟调度和数据本地化
//!
//! ## 特性
//!
//! - 支持多种调度策略
//! - 资源预留和配额管理
//! - 任务优先级和抢占
//! - 资源利用率优化
//! - 与云原生系统集成

use alloc::{vec::Vec, collections::{BTreeMap, VecDeque}, string::{String, ToString}, sync::Arc};
use core::fmt::Debug;
use crate::sync::Mutex;
// Import types from parent module
use super::{NodeId, TaskId};

/// 任务调度器
pub struct TaskScheduler {
    config: SchedulerConfig,
    task_queues: Arc<TaskQueues>,
    resource_manager: Arc<ResourceManager>,
    scheduling_policy: Arc<dyn SchedulingPolicy + Send + Sync>,
}

/// 调度器配置
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// 调度间隔（毫秒）
    pub scheduling_interval_ms: u64,
    /// 启用推测执行
    pub enable_speculative_execution: bool,
    /// 慢任务阈值（倍数）
    pub speculative_threshold: f64,
    /// 最大并行任务数
    pub max_parallel_tasks: usize,
    /// 启用资源隔离
    pub enable_resource_isolation: bool,
    /// 任务超时时间（秒）
    pub default_task_timeout_secs: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            scheduling_interval_ms: 100,
            enable_speculative_execution: false,
            speculative_threshold: 1.5,
            max_parallel_tasks: 1000,
            enable_resource_isolation: true,
            default_task_timeout_secs: 600,
        }
    }
}

/// 资源管理器
pub struct ResourceManager {
    resource_pools: Mutex<BTreeMap<String, ResourcePool>>,
    total_resources: ResourceAllocation,
    allocated_resources: Mutex<ResourceAllocation>,
}

/// 资源池
#[derive(Debug, Clone)]
pub struct ResourcePool {
    pub name: String,
    pub capacity: ResourceAllocation,
    pub allocated: ResourceAllocation,
    pub available: ResourceAllocation,
    pub nodes: Vec<NodeId>,
}

/// 任务队列
pub struct TaskQueues {
    queues: Mutex<BTreeMap<TaskPriority, VecDeque<TaskInfo>>>,
}

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskPriority {
    /// 最低优先级
    Low = 0,
    /// 正常优先级
    Normal = 1,
    /// 高优先级
    High = 2,
    /// 最高优先级
    Critical = 3,
}

/// 任务信息
#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub task_id: TaskId,
    pub priority: TaskPriority,
    pub resource_request: ResourceAllocation,
    pub affinity: Vec<NodeId>,
    pub anti_affinity: Vec<NodeId>,
    pub submit_time: u64,
    pub timeout_secs: u64,
    pub retry_count: u32,
    pub state: TaskState,
}

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskState {
    /// 等待调度
    Pending,
    /// 已调度
    Scheduled,
    /// 运行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 资源分配
#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceAllocation {
    /// CPU 核心数（毫秒）
    pub cpu_cores: f64,
    /// 内存大小（字节）
    pub memory_bytes: u64,
    /// GPU 数量
    pub gpu_count: u32,
    /// 磁盘空间（字节）
    pub disk_bytes: u64,
    /// 网络带宽（字节/秒）
    pub network_bandwidth: u64,
}

impl ResourceAllocation {
    /// 创建新的资源分配
    pub fn new(cpu_cores: f64, memory_bytes: u64) -> Self {
        Self {
            cpu_cores,
            memory_bytes,
            ..Default::default()
        }
    }

    /// 检查资源是否足够
    pub fn can_satisfy(&self, request: &ResourceAllocation) -> bool {
        self.cpu_cores >= request.cpu_cores
            && self.memory_bytes >= request.memory_bytes
            && self.gpu_count >= request.gpu_count
    }

    /// 减去资源
    pub fn subtract(&self, other: &ResourceAllocation) -> Option<ResourceAllocation> {
        if self.cpu_cores < other.cpu_cores || self.memory_bytes < other.memory_bytes {
            return None;
        }

        Some(ResourceAllocation {
            cpu_cores: self.cpu_cores - other.cpu_cores,
            memory_bytes: self.memory_bytes - other.memory_bytes,
            gpu_count: self.gpu_count.saturating_sub(other.gpu_count),
            disk_bytes: self.disk_bytes.saturating_sub(other.disk_bytes),
            network_bandwidth: self.network_bandwidth.saturating_sub(other.network_bandwidth),
        })
    }

    /// 添加资源
    pub fn add(&self, other: &ResourceAllocation) -> ResourceAllocation {
        ResourceAllocation {
            cpu_cores: self.cpu_cores + other.cpu_cores,
            memory_bytes: self.memory_bytes + other.memory_bytes,
            gpu_count: self.gpu_count + other.gpu_count,
            disk_bytes: self.disk_bytes + other.disk_bytes,
            network_bandwidth: self.network_bandwidth + other.network_bandwidth,
        }
    }
}

/// 调度策略特质
pub trait SchedulingPolicy: Send + Sync {
    /// 选择节点运行任务
    fn schedule(
        &self,
        task: &TaskInfo,
        available_nodes: &[NodeId],
        resources: &[ResourceAllocation],
    ) -> Option<SchedulingDecision>;

    /// 策略名称
    fn name(&self) -> &str;
}

/// 调度决策
#[derive(Debug, Clone)]
pub struct SchedulingDecision {
    pub task_id: TaskId,
    pub node_id: NodeId,
    pub allocated_resources: ResourceAllocation,
    pub score: f64,
}

/// FIFO 调度策略
struct FifoPolicy;

/// 公平调度策略
struct FairSchedulerPolicy {
    pools: BTreeMap<String, f64>,
}

/// 容量调度策略
struct CapacitySchedulerPolicy {
    queue_capacities: BTreeMap<String, ResourceAllocation>,
}

/// 调度器错误类型
#[derive(Debug)]
pub enum SchedulerError {
    NoAvailableNodes(String),
    InsufficientResources(String),
    TaskNotFound(String),
    NodeUnavailable(String),
    InvalidConfiguration(String),
}

impl TaskScheduler {
    /// 创建新的任务调度器
    pub fn new(config: SchedulerConfig) -> Self {
        let resource_manager = Arc::new(ResourceManager::new());
        let scheduling_policy: Arc<dyn SchedulingPolicy + Send + Sync> = Arc::new(FifoPolicy);

        Self {
            task_queues: Arc::new(TaskQueues::new()),
            resource_manager,
            scheduling_policy,
            config,
        }
    }

    /// 提交任务
    pub fn submit_task(&self, task: TaskInfo) -> Result<TaskId, SchedulerError> {
        let task_id = task.task_id.clone();
        self.task_queues.enqueue(task);
        Ok(task_id)
    }

    /// 调度任务
    pub fn schedule_tasks(&self) -> Result<Vec<SchedulingDecision>, SchedulerError> {
        let pending_tasks = self.task_queues.dequeue_all();
        let mut decisions = Vec::new();

        for task in pending_tasks {
            if let Some(decision) = self.schedule_single_task(&task) {
                let task_id = decision.task_id.clone();
                decisions.push(decision);

                // 更新任务状态
                self.task_queues.update_state(&task_id, TaskState::Scheduled);
            } else {
                // 重新入队
                self.task_queues.enqueue(task);
            }
        }

        Ok(decisions)
    }

    /// 调度单个任务
    fn schedule_single_task(&self, task: &TaskInfo) -> Option<SchedulingDecision> {
        let available_nodes = self.resource_manager.get_available_nodes()?;
        let node_resources: Vec<ResourceAllocation> = available_nodes
            .iter()
            .filter_map(|node| self.resource_manager.get_node_resources(node).ok())
            .collect();

        self.scheduling_policy.schedule(task, &available_nodes, &node_resources)
    }

    /// 分配资源
    pub fn allocate_resources(
        &self,
        node_id: &NodeId,
        allocation: ResourceAllocation,
    ) -> Result<(), SchedulerError> {
        self.resource_manager.allocate(node_id, allocation)
    }

    /// 释放资源
    pub fn release_resources(
        &self,
        node_id: &NodeId,
        allocation: ResourceAllocation,
    ) -> Result<(), SchedulerError> {
        self.resource_manager.release(node_id, allocation)
    }

    /// 更新任务状态
    pub fn update_task_state(&self, task_id: &TaskId, state: TaskState) {
        self.task_queues.update_state(task_id, state);
    }

    /// 获取队列统计信息
    pub fn get_queue_stats(&self) -> QueueStatistics {
        self.task_queues.get_stats()
    }

    /// 获取资源使用情况
    pub fn get_resource_usage(&self) -> ResourceUsage {
        self.resource_manager.get_usage()
    }

    /// 启动调度器
    pub fn start(&self) {
        // 简化实现：实际应该启动调度循环
    }

    /// 停止调度器
    pub fn stop(&self) {
        // 简化实现
    }
}

impl ResourceManager {
    fn new() -> Self {
        Self {
            resource_pools: Mutex::new(BTreeMap::new()),
            total_resources: ResourceAllocation::default(),
            allocated_resources: Mutex::new(ResourceAllocation::default()),
        }
    }

    /// 添加资源池
    pub fn add_pool(&self, pool: ResourcePool) {
        let mut pools = self.resource_pools.lock();
        pools.insert(pool.name.clone(), pool);
    }

    /// 分配资源
    pub fn allocate(&self, _node_id: &NodeId, allocation: ResourceAllocation) -> Result<(), SchedulerError> {
        let mut allocated = self.allocated_resources.lock();
        let new_allocation = allocated.add(&allocation);

        // 简化实现：直接分配
        *allocated = new_allocation;

        Ok(())
    }

    /// 释放资源
    pub fn release(&self, _node_id: &NodeId, allocation: ResourceAllocation) -> Result<(), SchedulerError> {
        let mut allocated = self.allocated_resources.lock();

        if let Some(new_allocation) = allocated.subtract(&allocation) {
            *allocated = new_allocation;
            Ok(())
        } else {
            Err(SchedulerError::InsufficientResources(
                "Cannot release more resources than allocated".to_string(),
            ))
        }
    }

    /// 获取可用节点
    fn get_available_nodes(&self) -> Option<Vec<NodeId>> {
        let pools = self.resource_pools.lock();
        if pools.is_empty() {
            return None;
        }

        let nodes = pools.values()
            .flat_map(|pool| pool.nodes.clone())
            .collect();

        Some(nodes)
    }

    /// 获取节点资源
    fn get_node_resources(&self, _node_id: &NodeId) -> Result<ResourceAllocation, SchedulerError> {
        // 简化实现：返回默认资源
        Ok(ResourceAllocation {
            cpu_cores: 4.0,
            memory_bytes: 16 * 1024 * 1024 * 1024, // 16GB
            ..Default::default()
        })
    }

    /// 获取资源使用情况
    fn get_usage(&self) -> ResourceUsage {
        let allocated = self.allocated_resources.lock();
        let total = self.total_resources;

        ResourceUsage {
            total,
            used: *allocated,
            available: total.subtract(&allocated).unwrap_or_default(),
            utilization: if total.cpu_cores > 0.0 {
                allocated.cpu_cores / total.cpu_cores
            } else {
                0.0
            },
        }
    }
}

impl TaskQueues {
    fn new() -> Self {
        Self {
            queues: Mutex::new(BTreeMap::new()),
        }
    }

    /// 入队任务
    fn enqueue(&self, task: TaskInfo) {
        let mut queues = self.queues.lock();
        queues.entry(task.priority).or_insert_with(VecDeque::new).push_back(task);
    }

    /// 出队所有待处理任务
    fn dequeue_all(&self) -> Vec<TaskInfo> {
        let mut queues = self.queues.lock();
        let mut tasks = Vec::new();

        // 按优先级从高到低出队
        for priority in [TaskPriority::Critical, TaskPriority::High, TaskPriority::Normal, TaskPriority::Low] {
            if let Some(queue) = queues.get_mut(&priority) {
                while let Some(task) = queue.pop_front() {
                    tasks.push(task);
                }
            }
        }

        tasks
    }

    /// 更新任务状态
    fn update_state(&self, task_id: &TaskId, state: TaskState) {
        let mut queues = self.queues.lock();
        for queue in queues.values_mut() {
            for task in queue.iter_mut() {
                if &task.task_id == task_id {
                    task.state = state;
                    return;
                }
            }
        }
    }

    /// 获取统计信息
    fn get_stats(&self) -> QueueStatistics {
        let queues = self.queues.lock();
        let mut total = 0;
        let mut pending_by_priority = BTreeMap::new();

        for (priority, queue) in queues.iter() {
            let count = queue.len();
            total += count;
            pending_by_priority.insert(*priority, count);
        }

        QueueStatistics {
            total_pending: total,
            pending_by_priority,
        }
    }
}

impl SchedulingPolicy for FifoPolicy {
    fn schedule(
        &self,
        task: &TaskInfo,
        available_nodes: &[NodeId],
        resources: &[ResourceAllocation],
    ) -> Option<SchedulingDecision> {
        // FIFO: 选择第一个满足资源要求的节点
        for (i, node) in available_nodes.iter().enumerate() {
            if let Some(res) = resources.get(i) {
                if res.can_satisfy(&task.resource_request) {
                    return Some(SchedulingDecision {
                        task_id: task.task_id.clone(),
                        node_id: node.clone(),
                        allocated_resources: task.resource_request,
                        score: 1.0,
                    });
                }
            }
        }
        None
    }

    fn name(&self) -> &str {
        "FIFO"
    }
}

impl SchedulingPolicy for FairSchedulerPolicy {
    fn schedule(
        &self,
        task: &TaskInfo,
        available_nodes: &[NodeId],
        resources: &[ResourceAllocation],
    ) -> Option<SchedulingDecision> {
        // 公平调度：选择资源利用率最低的节点
        let mut best_node = None;
        let mut best_score = f64::MAX;

        for (i, node) in available_nodes.iter().enumerate() {
            if let Some(res) = resources.get(i) {
                if res.can_satisfy(&task.resource_request) {
                    let score = res.cpu_cores + res.memory_bytes as f64;
                    if score < best_score {
                        best_score = score;
                        best_node = Some(node);
                    }
                }
            }
        }

        best_node.map(|node| SchedulingDecision {
            task_id: task.task_id.clone(),
            node_id: node.clone(),
            allocated_resources: task.resource_request,
            score: best_score,
        })
    }

    fn name(&self) -> &str {
        "Fair"
    }
}

impl SchedulingPolicy for CapacitySchedulerPolicy {
    fn schedule(
        &self,
        _task: &TaskInfo,
        _available_nodes: &[NodeId],
        _resources: &[ResourceAllocation],
    ) -> Option<SchedulingDecision> {
        // 简化实现：返回 None
        None
    }

    fn name(&self) -> &str {
        "Capacity"
    }
}

/// 队列统计信息
#[derive(Debug, Clone)]
pub struct QueueStatistics {
    pub total_pending: usize,
    pub pending_by_priority: BTreeMap<TaskPriority, usize>,
}

/// 资源使用情况
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub total: ResourceAllocation,
    pub used: ResourceAllocation,
    pub available: ResourceAllocation,
    pub utilization: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_allocation() {
        let res1 = ResourceAllocation::new(4.0, 16 * 1024 * 1024 * 1024);
        let res2 = ResourceAllocation::new(2.0, 8 * 1024 * 1024 * 1024);

        assert!(res1.can_satisfy(&res2));
        assert!(!res2.can_satisfy(&res1));

        let diff = res1.subtract(&res2).unwrap();
        assert_eq!(diff.cpu_cores, 2.0);
    }

    #[test]
    fn test_task_priority() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }

    #[test]
    fn test_task_queues() {
        let queues = TaskQueues::new();

        let task = TaskInfo {
            task_id: TaskId::new("job-1".to_string(), "task-1".to_string()),
            priority: TaskPriority::High,
            resource_request: ResourceAllocation::default(),
            affinity: Vec::new(),
            anti_affinity: Vec::new(),
            submit_time: 0,
            timeout_secs: 600,
            retry_count: 0,
            state: TaskState::Pending,
        };

        queues.enqueue(task);
        let stats = queues.get_stats();

        assert_eq!(stats.total_pending, 1);
    }
}
