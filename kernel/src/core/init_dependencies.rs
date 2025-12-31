//! 初始化依赖关系管理 / Initialization Dependency Management
//!
//! 本模块提供内核组件初始化依赖关系的定义、验证和分析功能。
//! This module provides definition, validation, and analysis of kernel component initialization dependencies.
//!
//! ## 主要功能 / Main Features
//!
//! - **依赖图定义**: 定义组件之间的依赖关系
//! - **拓扑排序**: 确定初始化的正确顺序
//! - **并行安全分析**: 识别可以并行执行的组件
//! - **循环检测**: 防止循环依赖导致的死锁
//! - **性能估算**: 估算组件初始化时间和资源需求
//!
//! ## 使用示例 / Usage Example
//!
//! ```rust
//! use kernel::core::init_dependencies::{InitDependency, DependencyGraph, ComponentCategory};
//!
//! // 定义依赖关系
//! let deps = vec![
//!     InitDependency::new("memory", ComponentCategory::Early, vec![], 100),
//!     InitDependency::new("vfs", ComponentCategory::Parallel, vec!["memory"], 200),
//! ];
//!
//! // 创建依赖图
//! let graph = DependencyGraph::new(deps).unwrap();
//!
//! // 获取并行安全级别
//! let safety = graph.analyze_parallel_safety();
//!
//! // 获取初始化顺序
//! let order = graph.topological_sort();
//! ```

#![allow(dead_code)]

use core::fmt;

use crate::error::{Error, Result};
use crate::prelude::*;

// ============================================================================
// 组件分类 / Component Categories
// ============================================================================

/// 组件初始化类别 / Component initialization category
///
/// 定义组件的初始化时机和并行安全性：
/// Defines component initialization timing and parallel safety:
///
/// - **Early**: 早期初始化，必须在主CPU上串行执行（如：中断、控制台）
/// - **Parallel**: 可以并行执行的组件（如：文件系统、驱动程序）
/// - **Late**: 晚期初始化，依赖并行组件完成（如：网络、服务）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComponentCategory {
    /// 早期初始化 - 必须串行执行
    /// Early initialization - must run serially
    Early = 0,

    /// 并行初始化 - 可以并行执行
    /// Parallel initialization - can run in parallel
    Parallel = 1,

    /// 晚期初始化 - 依赖并行组件
    /// Late initialization - depends on parallel components
    Late = 2,
}

impl ComponentCategory {
    /// 检查是否可以并行执行
    /// Check if this category can run in parallel
    pub fn is_parallel_safe(self) -> bool {
        matches!(self, Self::Parallel)
    }

    /// 检查是否为早期初始化
    /// Check if this is early initialization
    pub fn is_early(self) -> bool {
        matches!(self, Self::Early)
    }

    /// 检查是否为晚期初始化
    /// Check if this is late initialization
    pub fn is_late(self) -> bool {
        matches!(self, Self::Late)
    }
}

// ============================================================================
// 资源需求 / Resource Requirements
// ============================================================================

/// 组件初始化的资源需求
/// Resource requirements for component initialization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceRequirements {
    /// 估算的内存需求（字节）
    /// Estimated memory requirement in bytes
    pub memory_bytes: usize,

    /// 是否需要特定硬件访问
    /// Whether specific hardware access is needed
    pub needs_hardware_access: bool,

    /// 是否需要中断处理
    /// Whether interrupt handling is needed
    pub needs_interrupts: bool,

    /// CPU 亲和性建议（Some(cpu_id) 表示绑定到特定CPU）
    /// CPU affinity hint (Some(cpu_id) means bind to specific CPU)
    pub cpu_affinity: Option<usize>,

    /// 优先级（0-10，数字越小优先级越高）
    /// Priority (0-10, lower number means higher priority)
    pub priority: u8,
}

impl ResourceRequirements {
    /// 默认资源需求
    /// Default resource requirements
    pub const fn default() -> Self {
        Self {
            memory_bytes: 0,
            needs_hardware_access: false,
            needs_interrupts: false,
            cpu_affinity: None,
            priority: 5,
        }
    }

    /// 创建最小资源需求配置
    /// Create minimal resource requirements
    pub const fn minimal() -> Self {
        Self {
            memory_bytes: 4096,
            needs_hardware_access: false,
            needs_interrupts: false,
            cpu_affinity: None,
            priority: 5,
        }
    }

    /// 创建标准资源需求配置
    /// Create standard resource requirements
    pub const fn standard() -> Self {
        Self {
            memory_bytes: 65536,
            needs_hardware_access: false,
            needs_interrupts: false,
            cpu_affinity: None,
            priority: 5,
        }
    }

    /// 创建高资源需求配置
    /// Create high resource requirements
    pub const fn intensive() -> Self {
        Self {
            memory_bytes: 1048576, // 1MB
            needs_hardware_access: true,
            needs_interrupts: true,
            cpu_affinity: None,
            priority: 7,
        }
    }

    /// 创建硬件初始化的资源需求
    /// Create resource requirements for hardware initialization
    pub const fn hardware() -> Self {
        Self {
            memory_bytes: 16384,
            needs_hardware_access: true,
            needs_interrupts: true,
            cpu_affinity: Some(0), // Hardware init on boot CPU
            priority: 0,           // Highest priority
        }
    }
}

// ============================================================================
// 初始化依赖 / Initialization Dependencies
// ============================================================================

/// 单个组件的初始化依赖定义
/// Initialization dependency definition for a single component
#[derive(Clone)]
pub struct InitDependency {
    /// 组件唯一标识符
    /// Component unique identifier
    pub name: String,

    /// 组件类别
    /// Component category
    pub category: ComponentCategory,

    /// 此组件依赖的其他组件名称列表
    /// List of component names that this component depends on
    pub dependencies: Vec<String>,

    /// 估算初始化时间（微秒）
    /// Estimated initialization time in microseconds
    pub estimated_time_us: u64,

    /// 资源需求
    /// Resource requirements
    pub resources: ResourceRequirements,

    /// 组件描述
    /// Component description
    pub description: &'static str,

    /// 失败时是否可以继续
    /// Whether system can continue if this component fails
    pub optional: bool,
}

impl InitDependency {
    /// 创建新的初始化依赖
    /// Create new initialization dependency
    pub fn new(
        name: impl Into<String>,
        category: ComponentCategory,
        dependencies: Vec<impl Into<String>>,
        estimated_time_us: u64,
    ) -> Self {
        Self {
            name: name.into(),
            category,
            dependencies: dependencies.into_iter().map(|s| s.into()).collect(),
            estimated_time_us,
            resources: ResourceRequirements::default(),
            description: "",
            optional: false,
        }
    }

    /// 设置资源需求
    /// Set resource requirements
    pub fn with_resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = resources;
        self
    }

    /// 设置描述
    /// Set description
    pub fn with_description(mut self, desc: &'static str) -> Self {
        self.description = desc;
        self
    }

    /// 设置为可选组件
    /// Set as optional component
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// 检查是否可以并行执行
    /// Check if this component can run in parallel
    pub fn is_parallel_safe(&self) -> bool {
        self.category.is_parallel_safe()
    }

    /// 检查是否为早期组件
    /// Check if this is an early component
    pub fn is_early(&self) -> bool {
        self.category.is_early()
    }

    /// 检查是否为晚期组件
    /// Check if this is a late component
    pub fn is_late(&self) -> bool {
        self.category.is_late()
    }
}

impl fmt::Debug for InitDependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InitDependency")
            .field("name", &self.name)
            .field("category", &self.category)
            .field("dependencies", &self.dependencies)
            .field("estimated_time_us", &self.estimated_time_us)
            .field("resources", &self.resources)
            .field("optional", &self.optional)
            .finish()
    }
}

// ============================================================================
// 并行安全性分析 / Parallel Safety Analysis
// ============================================================================

/// 并行安全性分析结果
/// Parallel safety analysis result
#[derive(Debug, Clone)]
pub struct ParallelSafety {
    /// 可以并行执行的组件组
    /// Groups of components that can run in parallel
    pub parallel_groups: Vec<Vec<String>>,

    /// 必须串行执行的组件
    /// Components that must run serially
    pub serial_components: Vec<String>,

    /// 预期的并行度（1.0 表示完全串行，>1.0 表示有并行）
    /// Expected parallelism degree (1.0 = fully serial, >1.0 = has parallelism)
    pub parallelism_degree: f64,

    /// 预期的加速比
    /// Expected speedup ratio
    pub expected_speedup: f64,
}

impl ParallelSafety {
    /// 创建无并行安全性（所有组件串行）
    /// Create no parallel safety (all components serial)
    pub fn fully_serial() -> Self {
        Self {
            parallel_groups: Vec::new(),
            serial_components: Vec::new(),
            parallelism_degree: 1.0,
            expected_speedup: 1.0,
        }
    }

    /// 计算理论最佳加速比（假设无限CPU）
    /// Calculate theoretical optimal speedup (assuming infinite CPUs)
    pub fn theoretical_speedup(&self) -> f64 {
        if self.parallelism_degree <= 1.0 {
            1.0
        } else {
            // Amdahl's Law: Speedup = 1 / ((1-P) + P/N)
            // where P is parallelizable portion, N is number of processors
            // With infinite CPUs: Speedup = 1 / (1 - P)
            let parallelizable_ratio = (self.parallelism_degree - 1.0) / self.parallelism_degree;
            1.0 / (1.0 - parallelizable_ratio)
        }
    }

    /// 计算给定CPU数量的实际加速比
    /// Calculate actual speedup for given number of CPUs
    pub fn actual_speedup(&self, num_cpus: usize) -> f64 {
        if self.parallelism_degree <= 1.0 || num_cpus <= 1 {
            1.0
        } else {
            let parallelizable_ratio = (self.parallelism_degree - 1.0) / self.parallelism_degree;
            let n = num_cpus as f64;
            1.0 / ((1.0 - parallelizable_ratio) + (parallelizable_ratio / n))
        }
    }
}

// ============================================================================
// 依赖图 / Dependency Graph
// ============================================================================

/// 组件依赖关系图
/// Component dependency graph
///
/// 使用邻接表表示的依赖关系图，支持：
/// Dependency graph using adjacency list representation, supporting:
/// - 拓扑排序
/// - 循环依赖检测
/// - 并行安全性分析
/// - 依赖验证
pub struct DependencyGraph {
    /// 所有组件的依赖定义
    /// Dependency definitions for all components
    components: Vec<InitDependency>,

    /// 组件名称到索引的映射
    /// Component name to index mapping
    name_to_index: BTreeMap<String, usize>,
}

impl DependencyGraph {
    /// 创建新的依赖图
    /// Create new dependency graph
    pub fn new(components: Vec<InitDependency>) -> Result<Self> {
        let mut name_to_index = BTreeMap::new();

        // 构建名称映射
        for (idx, comp) in components.iter().enumerate() {
            if name_to_index.contains_key(&comp.name) {
                return Err(Error::Other(format!(
                    "Duplicate component name: {}",
                    comp.name
                )));
            }
            name_to_index.insert(comp.name.clone(), idx);
        }

        let graph = Self {
            components,
            name_to_index,
        };

        // 验证依赖关系
        graph.validate_dependencies()?;

        Ok(graph)
    }

    /// 验证所有依赖关系是否有效
    /// Validate all dependencies are valid
    fn validate_dependencies(&self) -> Result<()> {
        for comp in &self.components {
            for dep_name in &comp.dependencies {
                if !self.name_to_index.contains_key(dep_name) {
                    return Err(Error::NotFound);
                }

                // 检查跨类别的依赖是否合理
                if let Some(dep_idx) = self.name_to_index.get(dep_name) {
                    let dep_comp = &self.components[*dep_idx];

                    // 晚期组件不能依赖早期或并行组件
                    if comp.is_late() && (dep_comp.is_early() || dep_comp.is_parallel_safe()) {
                        // 这是正常的，late 组件可以依赖 parallel 组件
                    }

                    // 早期组件不能依赖并行或晚期组件
                    if comp.is_early() && (dep_comp.is_parallel_safe() || dep_comp.is_late()) {
                        return Err(Error::Other(format!(
                            "Early component '{}' cannot depend on {} component '{}'",
                            comp.name,
                            if dep_comp.is_parallel_safe() { "parallel" } else { "late" },
                            dep_name
                        )));
                    }

                    // 并行组件不能依赖晚期组件
                    if comp.is_parallel_safe() && dep_comp.is_late() {
                        return Err(Error::Other(format!(
                            "Parallel component '{}' cannot depend on late component '{}'",
                            comp.name, dep_name
                        )));
                    }
                }
            }
        }

        // 检测循环依赖
        self.detect_cycles()?;

        Ok(())
    }

    /// 使用 DFS 检测循环依赖
    /// Detect cyclic dependencies using DFS
    fn detect_cycles(&self) -> Result<()> {
        let n = self.components.len();
        let mut visited = vec![false; n];
        let mut recursion_stack = vec![false; n];

        for i in 0..n {
            if !visited[i] {
                if self.dfs_cycle_detect(i, &mut visited, &mut recursion_stack)? {
                    return Err(Error::Other(format!(
                        "Cyclic dependency detected involving component '{}'",
                        self.components[i].name
                    )));
                }
            }
        }

        Ok(())
    }

    /// DFS 循环检测辅助函数
    /// DFS cycle detection helper
    fn dfs_cycle_detect(
        &self,
        idx: usize,
        visited: &mut [bool],
        recursion_stack: &mut [bool],
    ) -> Result<bool> {
        visited[idx] = true;
        recursion_stack[idx] = true;

        if let Some(comp) = self.components.get(idx) {
            for dep_name in &comp.dependencies {
                if let Some(&dep_idx) = self.name_to_index.get(dep_name) {
                    if !visited[dep_idx] {
                        if self.dfs_cycle_detect(dep_idx, visited, recursion_stack)? {
                            return Ok(true);
                        }
                    } else if recursion_stack[dep_idx] {
                        return Ok(true);
                    }
                }
            }
        }

        recursion_stack[idx] = false;
        Ok(false)
    }

    /// 拓扑排序 - 返回按依赖顺序排列的组件索引
    /// Topological sort - returns component indices in dependency order
    pub fn topological_sort(&self) -> Result<Vec<usize>> {
        let n = self.components.len();
        let mut in_degree = vec![0usize; n];

        // 计算每个节点的入度
        for (i, comp) in self.components.iter().enumerate() {
            for dep_name in &comp.dependencies {
                if let Some(&dep_idx) = self.name_to_index.get(dep_name) {
                    in_degree[i] += 1;
                    // 我们记录的是"依赖"关系，不是"被依赖"关系
                    // 所以入度应该是依赖的数量
                    // 这里需要调整逻辑
                }
            }
        }

        // 重新计算：in_degree[i] = number of unresolved dependencies for component i
        in_degree.fill(0);
        for (i, comp) in self.components.iter().enumerate() {
            in_degree[i] = comp.dependencies.len();
        }

        // 使用队列进行拓扑排序
        let mut queue = Vec::new();
        for (i, &degree) in in_degree.iter().enumerate() {
            if degree == 0 {
                queue.push(i);
            }
        }

        let mut result = Vec::with_capacity(n);

        while let Some(idx) = queue.pop() {
            result.push(idx);

            // 减少依赖此组件的其他节点的入度
            // 这里需要反向查找
            for (i, comp) in self.components.iter().enumerate() {
                if comp.dependencies.contains(&self.components[idx].name) {
                    in_degree[i] -= 1;
                    if in_degree[i] == 0 {
                        queue.push(i);
                    }
                }
            }
        }

        if result.len() != n {
            return Err(Error::Other("Cycle detected in dependency graph".into()));
        }

        Ok(result)
    }

    /// 分析并行安全性
    /// Analyze parallel safety
    pub fn analyze_parallel_safety(&self) -> ParallelSafety {
        let sorted_indices = match self.topological_sort() {
            Ok(indices) => indices,
            Err(_) => return ParallelSafety::fully_serial(),
        };

        let mut parallel_groups: Vec<Vec<String>> = Vec::new();
        let mut serial_components = Vec::new();
        let mut current_level_ready: Vec<String> = Vec::new();
        let mut completed: BTreeSet<String> = BTreeSet::new();
        let mut total_parallel_time = 0u64;
        let mut total_serial_time = 0u64;

        // 按拓扑顺序分析
        for &idx in &sorted_indices {
            let comp = &self.components[idx];

            // 检查所有依赖是否都已完成
            let all_deps_ready = comp
                .dependencies
                .iter()
                .all(|dep| completed.contains(dep));

            if !all_deps_ready {
                // 还有依赖未完成，不能并行
                serial_components.push(comp.name.clone());
                total_serial_time += comp.estimated_time_us;
                continue;
            }

            if comp.is_parallel_safe() {
                current_level_ready.push(comp.name.clone());
                total_parallel_time += comp.estimated_time_us;
            } else {
                // 串行组件
                if !current_level_ready.is_empty() {
                    parallel_groups.push(core::mem::take(&mut current_level_ready));
                }
                serial_components.push(comp.name.clone());
                total_serial_time += comp.estimated_time_us;
                completed.insert(comp.name.clone());
            }
        }

        // 添加最后一组并行组件
        if !current_level_ready.is_empty() {
            parallel_groups.push(current_level_ready);
        }

        // 计算并行度
        let total_time = total_parallel_time + total_serial_time;
        let parallelism_degree = if total_time > 0 {
            (total_time as f64) / (total_serial_time as f64)
        } else {
            1.0
        };

        // 估算加速比（假设有足够的CPU）
        let num_parallel_groups = parallel_groups.len() as f64;
        let expected_speedup = if total_parallel_time > 0 {
            // 简化模型：假设所有并行组可以完全并行
            let serial_ratio = total_serial_time as f64 / total_time as f64;
            1.0 / (serial_ratio + (1.0 - serial_ratio) / num_parallel_groups.max(1.0))
        } else {
            1.0
        };

        ParallelSafety {
            parallel_groups,
            serial_components,
            parallelism_degree,
            expected_speedup,
        }
    }

    /// 获取组件
    /// Get component by name
    pub fn get_component(&self, name: &str) -> Option<&InitDependency> {
        self.name_to_index
            .get(name)
            .and_then(|&idx| self.components.get(idx))
    }

    /// 获取所有组件
    /// Get all components
    pub fn components(&self) -> &[InitDependency] {
        &self.components
    }

    /// 获取早期组件
    /// Get early components
    pub fn early_components(&self) -> Vec<&InitDependency> {
        self.components
            .iter()
            .filter(|c| c.is_early())
            .collect()
    }

    /// 获取并行组件
    /// Get parallel components
    pub fn parallel_components(&self) -> Vec<&InitDependency> {
        self.components
            .iter()
            .filter(|c| c.is_parallel_safe())
            .collect()
    }

    /// 获取晚期组件
    /// Get late components
    pub fn late_components(&self) -> Vec<&InitDependency> {
        self.components
            .iter()
            .filter(|c| c.is_late())
            .collect()
    }

    /// 计算总估算时间（串行执行）
    /// Calculate total estimated time (serial execution)
    pub fn total_serial_time(&self) -> u64 {
        self.components.iter().map(|c| c.estimated_time_us).sum()
    }

    /// 计算并行执行的理论最佳时间
    /// Calculate theoretical optimal parallel execution time
    pub fn optimal_parallel_time(&self, num_cpus: usize) -> u64 {
        let safety = self.analyze_parallel_safety();
        let speedup = safety.actual_speedup(num_cpus);
        ((self.total_serial_time() as f64) / speedup) as u64
    }

    /// 获取组件索引
    /// Get component index by name
    fn get_index(&self, name: &str) -> Option<usize> {
        self.name_to_index.get(name).copied()
    }
}

// ============================================================================
// 预定义的内核依赖 / Predefined Kernel Dependencies
// ============================================================================

/// 创建标准内核依赖图
/// Create standard kernel dependency graph
pub fn create_standard_kernel_deps() -> Result<DependencyGraph> {
    let deps = vec![
        // ===== 早期初始化（串行）=====
        // Early initialization (serial)

        InitDependency::new(
            "boot_params",
            ComponentCategory::Early,
            vec![],
            50,
        )
        .with_description("Boot parameters initialization")
        .with_resources(ResourceRequirements::minimal()),

        InitDependency::new(
            "console",
            ComponentCategory::Early,
            vec!["boot_params"],
            100,
        )
        .with_description("Console and early output")
        .with_resources(ResourceRequirements::hardware()),

        InitDependency::new(
            "interrupts",
            ComponentCategory::Early,
            vec!["boot_params"],
            150,
        )
        .with_description("Interrupt handling")
        .with_resources(ResourceRequirements::hardware()),

        InitDependency::new(
            "boot_cpu",
            ComponentCategory::Early,
            vec!["interrupts"],
            80,
        )
        .with_description("Boot CPU initialization")
        .with_resources(ResourceRequirements::hardware()),

        // ===== 并行初始化 =====
        // Parallel initialization

        InitDependency::new(
            "phys_memory",
            ComponentCategory::Parallel,
            vec!["boot_cpu"],
            200,
        )
        .with_description("Physical memory management")
        .with_resources(ResourceRequirements::standard()),

        InitDependency::new(
            "heap",
            ComponentCategory::Parallel,
            vec!["phys_memory"],
            100,
        )
        .with_description("Kernel heap allocator")
        .with_resources(ResourceRequirements::standard()),

        InitDependency::new(
            "vm",
            ComponentCategory::Parallel,
            vec!["phys_memory", "heap"],
            300,
        )
        .with_description("Virtual memory management")
        .with_resources(ResourceRequirements::intensive()),

        InitDependency::new(
            "rcu",
            ComponentCategory::Parallel,
            vec!["boot_cpu"],
            80,
        )
        .with_description("RCU synchronization")
        .with_resources(ResourceRequirements::minimal()),

        InitDependency::new(
            "timer",
            ComponentCategory::Parallel,
            vec!["interrupts"],
            120,
        )
        .with_description("Timer subsystem")
        .with_resources(ResourceRequirements::standard()),

        InitDependency::new(
            "drivers",
            ComponentCategory::Parallel,
            vec!["interrupts", "phys_memory"],
            500,
        )
        .with_description("Device drivers")
        .with_resources(ResourceRequirements::hardware()),

        InitDependency::new(
            "ramfs",
            ComponentCategory::Parallel,
            vec!["vm"],
            150,
        )
        .with_description("RAM filesystem")
        .with_resources(ResourceRequirements::standard()),

        InitDependency::new(
            "ext4",
            ComponentCategory::Parallel,
            vec!["vm"],
            200,
        )
        .with_description("EXT4 filesystem")
        .with_resources(ResourceRequirements::standard()),

        InitDependency::new(
            "procfs",
            ComponentCategory::Parallel,
            vec!["vm"],
            100,
        )
        .with_description("Proc filesystem")
        .with_resources(ResourceRequirements::minimal()),

        InitDependency::new(
            "sysfs",
            ComponentCategory::Parallel,
            vec!["vm"],
            100,
        )
        .with_description("Sys filesystem")
        .with_resources(ResourceRequirements::minimal()),

        InitDependency::new(
            "vfs_mount",
            ComponentCategory::Parallel,
            vec!["ramfs", "ext4", "procfs", "sysfs"],
            150,
        )
        .with_description("VFS root mount")
        .with_resources(ResourceRequirements::standard()),

        InitDependency::new(
            "libc",
            ComponentCategory::Parallel,
            vec!["heap"],
            200,
        )
        .with_description("C standard library")
        .with_resources(ResourceRequirements::standard()),

        InitDependency::new(
            "aio",
            ComponentCategory::Parallel,
            vec!["libc"],
            150,
        )
        .with_description("Async I/O subsystem")
        .with_resources(ResourceRequirements::standard())
        .optional(),

        InitDependency::new(
            "scheduler",
            ComponentCategory::Parallel,
            vec!["timer", "rcu"],
            300,
        )
        .with_description("Process scheduler")
        .with_resources(ResourceRequirements::standard()),

        InitDependency::new(
            "threading",
            ComponentCategory::Parallel,
            vec!["scheduler"],
            200,
        )
        .with_description("Threading subsystem")
        .with_resources(ResourceRequirements::standard()),

        // ===== 晚期初始化 =====
        // Late initialization

        InitDependency::new(
            "process",
            ComponentCategory::Late,
            vec!["vm", "scheduler", "vfs_mount"],
            400,
        )
        .with_description("Process subsystem")
        .with_resources(ResourceRequirements::intensive()),

        InitDependency::new(
            "syscalls",
            ComponentCategory::Late,
            vec!["process"],
            300,
        )
        .with_description("System call dispatcher")
        .with_resources(ResourceRequirements::standard()),

        InitDependency::new(
            "ipc",
            ComponentCategory::Late,
            vec!["process"],
            250,
        )
        .with_description("IPC subsystem")
        .with_resources(ResourceRequirements::standard()),

        InitDependency::new(
            "network",
            ComponentCategory::Late,
            vec!["drivers", "syscalls"],
            600,
        )
        .with_description("Network stack")
        .with_resources(ResourceRequirements::intensive())
        .optional(),

        InitDependency::new(
            "security",
            ComponentCategory::Late,
            vec!["process", "syscalls"],
            200,
        )
        .with_description("Security subsystem")
        .with_resources(ResourceRequirements::standard()),

        InitDependency::new(
            "services",
            ComponentCategory::Late,
            vec!["syscalls", "ipc"],
            300,
        )
        .with_description("Service layer")
        .with_resources(ResourceRequirements::standard())
        .optional(),

        InitDependency::new(
            "monitoring",
            ComponentCategory::Late,
            vec!["services"],
            150,
        )
        .with_description("Monitoring and metrics")
        .with_resources(ResourceRequirements::standard())
        .optional(),

        InitDependency::new(
            "compat",
            ComponentCategory::Late,
            vec!["syscalls"],
            250,
        )
        .with_description("Compatibility layer")
        .with_resources(ResourceRequirements::standard()),

        InitDependency::new(
            "smp",
            ComponentCategory::Late,
            vec!["scheduler", "threading"],
            200,
        )
        .with_description("SMP initialization")
        .with_resources(ResourceRequirements::hardware()),
    ];

    DependencyGraph::new(deps)
}

// ============================================================================
// 单元测试 / Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_category() {
        assert!(ComponentCategory::Early.is_early());
        assert!(ComponentCategory::Parallel.is_parallel_safe());
        assert!(ComponentCategory::Late.is_late());
    }

    #[test]
    fn test_resource_requirements() {
        let minimal = ResourceRequirements::minimal();
        assert!(minimal.memory_bytes > 0);
        assert!(!minimal.needs_hardware_access);

        let hardware = ResourceRequirements::hardware();
        assert!(hardware.needs_hardware_access);
        assert_eq!(hardware.cpu_affinity, Some(0));
        assert_eq!(hardware.priority, 0);
    }

    #[test]
    fn test_dependency_creation() {
        let dep = InitDependency::new("test", ComponentCategory::Parallel, vec!["dep1"], 100)
            .with_description("Test component")
            .optional();

        assert_eq!(dep.name, "test");
        assert!(dep.is_parallel_safe());
        assert!(dep.optional);
        assert_eq!(dep.dependencies.len(), 1);
    }

    #[test]
    fn test_simple_dependency_graph() {
        let deps = vec![
            InitDependency::new("a", ComponentCategory::Early, vec![], 100),
            InitDependency::new("b", ComponentCategory::Parallel, vec!["a"], 100),
            InitDependency::new("c", ComponentCategory::Late, vec!["b"], 100),
        ];

        let graph = DependencyGraph::new(deps).unwrap();
        let sorted = graph.topological_sort().unwrap();

        assert_eq!(graph.components().len(), 3);
        assert_eq!(sorted.len(), 3);
        // a must come before b, b before c
        let a_idx = sorted.iter().position(|&i| &graph.components[i].name == "a").unwrap();
        let b_idx = sorted.iter().position(|&i| &graph.components[i].name == "b").unwrap();
        let c_idx = sorted.iter().position(|&i| &graph.components[i].name == "c").unwrap();
        assert!(a_idx < b_idx);
        assert!(b_idx < c_idx);
    }

    #[test]
    fn test_cycle_detection() {
        let deps = vec![
            InitDependency::new("a", ComponentCategory::Parallel, vec!["b"], 100),
            InitDependency::new("b", ComponentCategory::Parallel, vec!["c"], 100),
            InitDependency::new("c", ComponentCategory::Parallel, vec!["a"], 100),
        ];

        let result = DependencyGraph::new(deps);
        assert!(result.is_err());
    }

    #[test]
    fn test_parallel_safety_analysis() {
        let deps = vec![
            InitDependency::new("early", ComponentCategory::Early, vec![], 100),
            InitDependency::new("p1", ComponentCategory::Parallel, vec!["early"], 100),
            InitDependency::new("p2", ComponentCategory::Parallel, vec!["early"], 100),
            InitDependency::new("late", ComponentCategory::Late, vec!["p1", "p2"], 100),
        ];

        let graph = DependencyGraph::new(deps).unwrap();
        let safety = graph.analyze_parallel_safety();

        // p1 and p2 should be in parallel group
        assert!(!safety.parallel_groups.is_empty());
        assert!(safety.parallelism_degree > 1.0);
        assert!(safety.expected_speedup > 1.0);
    }

    #[test]
    fn test_standard_kernel_deps() {
        let graph = create_standard_kernel_deps().unwrap();
        assert!(!graph.components().is_empty());

        let early = graph.early_components();
        let parallel = graph.parallel_components();
        let late = graph.late_components();

        assert!(!early.is_empty());
        assert!(!parallel.is_empty());
        assert!(!late.is_empty());

        let safety = graph.analyze_parallel_safety();
        assert!(safety.parallelism_degree > 1.0);
    }
}
