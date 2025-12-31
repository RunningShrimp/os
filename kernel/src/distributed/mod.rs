//! # 分布式计算框架
//!
//! 本模块提供完整的分布式计算框架，支持：
//! - MapReduce 编程模型
//! - 分布式文件系统 (HDFS)
//! - RPC 通信框架
//! - 分布式锁服务
//! - 任务调度和资源管理
//! - 容错和故障恢复
//!
//! ## 架构概览
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │           分布式计算框架 (Distributed)               │
//! ├─────────────────────────────────────────────────────┤
//! │  MapReduce  │  HDFS  │  RPC  │  Lock  │  Scheduler │
//! ├─────────────────────────────────────────────────────┤
//! │              容错和故障恢复 (FT)                      │
//! ├─────────────────────────────────────────────────────┤
//! │         网络传输层 (TCP/UDP/SCTP)                   │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## 核心特性
//!
//! - **高吞吐量**: 优化的大规模数据处理
//! - **低延迟**: 高效的通信和调度机制
//! - **水平扩展**: 支持动态添加/移除节点
//! - **容错性**: 自动故障检测和恢复
//! - **数据一致性**: 分布式事务和一致性协议
//! - **云原生集成**: 与容器和编排系统集成

pub mod mapreduce;
pub mod hdfs;
pub mod rpc;
pub mod lock;
pub mod scheduler;
pub mod ft;

pub use mapreduce::{
    MapReduceEngine, MapTask, ReduceTask, PartitionStrategy,
    MapReduceConfig, MapReduceError, MapReduceResult,
};

pub use hdfs::{
    HdfsClient, HdfsBlock, HdfsFileInfo, HdfsConfig, HdfsError,
    NameNodeProxy, DataNodeClient, ReplicaLocator,
};

pub use rpc::{
    RpcFramework, RpcServer, RpcClient, RpcMessage, RpcError,
    SerializationFormat, LoadBalancingStrategy, ServiceRegistry,
};

pub use lock::{
    DistributedLock, LockLevel, LockAcquireResult, LockError,
    LockObserver, DeadlockDetector, LeaseManager,
};

pub use scheduler::{
    TaskScheduler, ResourceManager, TaskQueues, ResourcePool,
    SchedulingPolicy, TaskState, ResourceAllocation, SchedulerError,
};

pub use ft::{
    FaultToleranceFramework, HeartbeatMonitor, FailureDetector,
    CheckpointManager, RecoveryManager, ReplicationManager,
    HealthChecker, FtConfig, FtError,
};

/// 分布式系统节点信息
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId {
    /// 节点唯一标识符
    pub id: alloc::string::String,
    /// 节点主机地址
    pub host: alloc::string::String,
    /// 节点端口
    pub port: u16,
    /// 节点角色
    pub role: NodeRole,
}

/// 节点角色类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeRole {
    /// Master/Coordinator 节点
    Master,
    /// Worker 节点
    Worker,
    /// NameNode (HDFS)
    NameNode,
    /// DataNode (HDFS)
    DataNode,
    /// RPC Server
    RpcServer,
    /// RPC Client
    RpcClient,
}

/// 分布式任务 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId {
    /// 作业 ID
    pub job_id: alloc::string::String,
    /// 任务 ID
    pub task_id: alloc::string::String,
    /// 尝试次数
    pub attempt: u32,
}

impl TaskId {
    /// 创建新的任务 ID
    pub fn new(job_id: alloc::string::String, task_id: alloc::string::String) -> Self {
        Self {
            job_id,
            task_id,
            attempt: 0,
        }
    }

    /// 创建重试任务 ID
    pub fn retry(&self) -> Self {
        Self {
            job_id: self.job_id.clone(),
            task_id: self.task_id.clone(),
            attempt: self.attempt + 1,
        }
    }
}

/// 分布式系统错误类型
#[derive(Debug)]
pub enum DistributedError {
    Network(alloc::string::String),
    Serialization(alloc::string::String),
    NodeUnavailable(alloc::string::String),
    TaskFailed(alloc::string::String, alloc::string::String),
    Timeout(u64),
    ConsistencyViolation(alloc::string::String),
    ResourceExhausted(alloc::string::String),
    Configuration(alloc::string::String),
}

/// 分布式系统配置
#[derive(Debug, Clone)]
pub struct DistributedConfig {
    /// 节点 ID
    pub node_id: NodeId,
    /// 集群成员列表
    pub cluster_members: alloc::vec::Vec<NodeId>,
    /// RPC 配置
    pub rpc_config: crate::distributed::rpc::RpcConfig,
    /// 超时时间（秒）
    pub timeout_secs: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 启用容错
    pub enable_ft: bool,
    /// 心跳间隔（毫秒）
    pub heartbeat_interval_ms: u64,
}

impl Default for DistributedConfig {
    fn default() -> Self {
        Self {
            node_id: NodeId {
                id: alloc::string::String::from("default-node"),
                host: alloc::string::String::from("localhost"),
                port: 8080,
                role: NodeRole::Worker,
            },
            cluster_members: alloc::vec::Vec::new(),
            rpc_config: crate::distributed::rpc::RpcConfig::default(),
            timeout_secs: 30,
            max_retries: 3,
            enable_ft: true,
            heartbeat_interval_ms: 5000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id_creation() {
        let task_id = TaskId::new(alloc::string::String::from("job-1"), alloc::string::String::from("task-1"));
        assert_eq!(task_id.attempt, 0);

        let retry_id = task_id.retry();
        assert_eq!(retry_id.attempt, 1);
        assert_eq!(retry_id.job_id, alloc::string::String::from("job-1"));
        assert_eq!(retry_id.task_id, alloc::string::String::from("task-1"));
    }

    #[test]
    fn test_node_id_equality() {
        let node1 = NodeId {
            id: alloc::string::String::from("node-1"),
            host: alloc::string::String::from("host1"),
            port: 8080,
            role: NodeRole::Master,
        };

        let node2 = NodeId {
            id: alloc::string::String::from("node-1"),
            host: alloc::string::String::from("host1"),
            port: 8080,
            role: NodeRole::Master,
        };

        assert_eq!(node1, node2);
    }
}
