//! Feature, load, and resource managers

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// 功能管理器
#[derive(Debug, Clone)]
pub struct FeatureManager {
    /// 功能列表
    pub features: BTreeMap<String, Feature>,
    /// 功能依赖
    pub dependencies: BTreeMap<String, Vec<String>>,
    /// 功能状态
    pub feature_states: BTreeMap<String, FeatureState>,
}

/// 功能
#[derive(Debug, Clone)]
pub struct Feature {
    /// 功能ID
    pub id: String,
    /// 功能名称
    pub name: String,
    /// 功能描述
    pub description: String,
    /// 功能类别
    pub category: FeatureCategory,
    /// 重要性级别
    pub importance_level: ImportanceLevel,
    /// 资源需求
    pub resource_requirements: ResourceRequirements,
    /// 质量要求
    pub quality_requirements: QualityRequirements,
    /// 启用状态
    pub enabled: bool,
}

/// 功能类别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureCategory {
    /// 核心功能
    Core,
    /// 重要功能
    Important,
    /// 辅助功能
    Auxiliary,
    /// 可选功能
    Optional,
    /// 实验性功能
    Experimental,
}

/// 重要性级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportanceLevel {
    /// 关键
    Critical = 5,
    /// 重要
    Important = 4,
    /// 一般
    Normal = 3,
    /// 次要
    Minor = 2,
    /// 可选
    Optional = 1,
}

/// 资源需求
#[derive(Debug, Clone)]
pub struct ResourceRequirements {
    /// CPU需求（百分比）
    pub cpu_requirement: f64,
    /// 内存需求（MB）
    pub memory_requirement: u64,
    /// 带宽需求（Mbps）
    pub bandwidth_requirement: f64,
    /// 存储需求（MB）
    pub storage_requirement: u64,
    /// I/O需求
    pub io_requirement: IORequirement,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu_requirement: 10.0,
            memory_requirement: 512,
            bandwidth_requirement: 1.0,
            storage_requirement: 1024,
            io_requirement: IORequirement::Medium,
        }
    }
}

/// I/O需求
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IORequirement {
    /// 低
    Low,
    /// 中等
    Medium,
    /// 高
    High,
    /// 极高
    VeryHigh,
}

/// 质量要求
#[derive(Debug, Clone, Default)]
pub struct QualityRequirements {
    /// 最大响应时间（毫秒）
    pub max_response_time_ms: u64,
    /// 最小吞吐量
    pub min_throughput: f64,
    /// 最大错误率（百分比）
    pub max_error_rate: f64,
    /// 最小可用性（百分比）
    pub min_availability: f64,
}

/// 功能状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureState {
    /// 启用
    Enabled,
    /// 禁用
    Disabled,
    /// 降级
    Degraded,
    /// 维护中
    Maintenance,
}

/// 负载管理器
#[derive(Debug, Clone)]
pub struct LoadManager {
    /// 负载策略
    pub load_strategies: BTreeMap<String, LoadStrategy>,
    /// 当前负载状态
    pub current_load: LoadStatus,
    /// 负载历史
    pub load_history: Vec<LoadSnapshot>,
}

/// 负载策略
#[derive(Debug, Clone)]
pub struct LoadStrategy {
    /// 策略ID
    pub id: String,
    /// 策略名称
    pub name: String,
    /// 策略类型
    pub strategy_type: LoadStrategyType,
    /// 负载阈值
    pub load_thresholds: LoadThresholds,
    /// 负载分配算法
    pub allocation_algorithm: AllocationAlgorithm,
    /// 策略参数
    pub parameters: BTreeMap<String, String>,
}

/// 负载策略类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStrategyType {
    /// 固定分配
    FixedAllocation,
    /// 动态分配
    DynamicAllocation,
    /// 基于优先级
    PriorityBased,
    /// 基于权重
    WeightBased,
    /// 自适应分配
    AdaptiveAllocation,
}

/// 负载阈值
#[derive(Debug, Clone)]
pub struct LoadThresholds {
    /// 正常负载阈值
    pub normal_threshold: f64,
    /// 高负载阈值
    pub high_threshold: f64,
    /// 过载阈值
    pub overload_threshold: f64,
    /// 严重过载阈值
    pub severe_overload_threshold: f64,
}

/// 分配算法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationAlgorithm {
    /// 轮询
    RoundRobin,
    /// 加权轮询
    WeightedRoundRobin,
    /// 最少连接
    LeastConnections,
    /// 响应时间
    ResponseTime,
    /// 资源使用
    ResourceUsage,
    /// 自适应
    Adaptive,
}

/// 负载状态
#[derive(Debug, Clone)]
pub struct LoadStatus {
    /// 总负载
    pub total_load: f64,
    /// 可用容量
    pub available_capacity: f64,
    /// 负载百分比
    pub load_percentage: f64,
    /// 负载等级
    pub load_level: LoadLevel,
    /// 更新时间
    pub last_updated: u64,
}

/// 负载等级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadLevel {
    /// 低负载
    Low,
    /// 正常负载
    Normal,
    /// 高负载
    High,
    /// 过载
    Overload,
    /// 严重过载
    SevereOverload,
}

/// 负载快照
#[derive(Debug, Clone)]
pub struct LoadSnapshot {
    /// 时间戳
    pub timestamp: u64,
    /// 负载状态
    pub load_status: LoadStatus,
    /// 各组件负载
    pub component_loads: BTreeMap<String, f64>,
}

/// 资源管理器
#[derive(Debug, Clone)]
pub struct ResourceManager {
    /// 资源池
    pub resource_pools: BTreeMap<String, ResourcePool>,
    /// 资源分配
    pub resource_allocations: BTreeMap<String, ResourceAllocation>,
    /// 资源使用统计
    pub usage_statistics: ResourceUsageStatistics,
}

/// 资源池
#[derive(Debug, Clone)]
pub struct ResourcePool {
    /// 池ID
    pub id: String,
    /// 池名称
    pub name: String,
    /// 资源类型
    pub resource_type: ResourceType,
    /// 总容量
    pub total_capacity: u64,
    /// 已分配容量
    pub allocated_capacity: u64,
    /// 可用容量
    pub available_capacity: u64,
    /// 保留容量
    pub reserved_capacity: u64,
    /// 池状态
    pub status: PoolStatus,
}

/// 资源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    /// CPU
    CPU,
    /// 内存
    Memory,
    /// 存储
    Storage,
    /// 网络
    Network,
    /// GPU
    GPU,
    /// 自定义资源
    Custom,
}

/// 池状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolStatus {
    /// 可用
    Available,
    /// 部分可用
    PartiallyAvailable,
    /// 已满
    Full,
    /// 维护中
    Maintenance,
    /// 不可用
    Unavailable,
}

/// 资源分配
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    /// 分配ID
    pub id: String,
    /// 资源池ID
    pub pool_id: String,
    /// 分配给
    pub allocated_to: String,
    /// 分配数量
    pub allocated_amount: u64,
    /// 分配时间
    pub allocation_time: u64,
    /// 到期时间
    pub expiry_time: Option<u64>,
    /// 分配状态
    pub status: AllocationStatus,
}

/// 分配状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationStatus {
    /// 活动
    Active,
    /// 已完成
    Completed,
    /// 已过期
    Expired,
    /// 已释放
    Released,
}

/// 资源使用统计
#[derive(Debug, Clone, Default)]
pub struct ResourceUsageStatistics {
    /// 总分配次数
    pub total_allocations: u64,
    /// 总释放次数
    pub total_releases: u64,
    /// 平均使用时间（秒）
    pub avg_usage_duration: u64,
    /// 峰值使用量
    pub peak_usage: u64,
    /// 当前使用量
    pub current_usage: u64,
    /// 使用率（百分比）
    pub utilization_rate: f64,
}
