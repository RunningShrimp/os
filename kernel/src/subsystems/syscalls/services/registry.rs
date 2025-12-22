//! 服务注册表模块
//! 
//! 本模块实现了服务的注册、发现和管理功能，包括：
//! - ServiceRegistry: 核心服务注册表
//! - 服务依赖关系管理
//! - 服务生命周期状态跟踪
//! - 服务查询和过滤功能
//! 
//! 服务注册表是整个服务管理系统的核心，负责维护所有已注册服务的信息。

use nos_nos_error_handling::unified::KernelError;
use crate::syscalls::services::traits::*;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

// 定义本地Result类型别名
pub type Result<T> = core::result::Result<T, KernelError>;
/// Version struct for syscall versioning (semantic versioning: major.minor.patch)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Create a new Version instance
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
    
    /// Parse a version string (e.g., "1.2.3") into a Version instance
    pub fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(KernelError::InvalidInput("Version string must be in format major.minor.patch".to_string()));
        }
        
        let major = parts[0].parse::<u32>()
            .map_err(|_| KernelError::InvalidInput("Invalid major version".to_string()))?;
        let minor = parts[1].parse::<u32>()
            .map_err(|_| KernelError::InvalidInput("Invalid minor version".to_string()))?;
        let patch = parts[2].parse::<u32>()
            .map_err(|_| KernelError::InvalidInput("Invalid patch version".to_string()))?;
        
        Ok(Self { major, minor, patch })
    }
    
    /// Convert Version to string format
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// 服务注册表
/// 
/// 核心服务管理组件，负责：
/// - 服务的注册和注销
/// - 服务依赖关系管理
/// - 服务状态跟踪
/// - 服务查询和发现
/// 
/// 使用线程安全的内部状态，支持并发访问。
pub struct ServiceRegistry {
    /// 已注册的服务映射
    /// 
    /// 键：服务名称，值：服务实例和元数据
    services: Arc<Mutex<BTreeMap<String, ServiceEntry>>>,
    
    /// 系统调用服务映射
    ///
    /// 键：系统调用号，值：(服务名称, 版本号)
    /// 用于快速查找处理特定系统调用的服务
    syscall_mapping: Arc<Mutex<BTreeMap<u32, BTreeMap<Version, String>>>>,
    
    /// 系统调用默认版本映射
    ///
    /// 键：系统调用号，值：默认版本号
    syscall_default_versions: Arc<Mutex<BTreeMap<u32, Version>>>,
    
    /// 服务依赖图
    /// 
    /// 用于管理服务间的依赖关系和启动顺序
    dependency_graph: Arc<Mutex<DependencyGraph>>,
}

/// 服务注册条目
/// 
/// 包含服务实例及其相关元数据。
#[derive(Debug)]
pub struct ServiceEntry {
    /// 服务实例
    pub service: Arc<dyn Service>,
    /// 服务元数据
    pub metadata: ServiceMetadata,
    /// 注册时间戳
    pub registration_time: u64,
}

/// 服务元数据
/// 
/// 包含服务的描述性信息。
#[derive(Debug, Clone)]
pub struct ServiceMetadata {
    /// 服务类型
    pub service_type: ServiceType,
    /// 服务优先级
    pub priority: u32,
    /// 是否为系统调用服务
    pub is_syscall_service: bool,
    /// 自定义标签
    pub tags: Vec<String>,
}

/// 服务类型枚举
/// 
/// 定义不同类型的服务。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceType {
    /// 核心系统服务
    Core,
    /// 系统调用服务
    Syscall,
    /// 网络服务
    Network,
    /// 文件系统服务
    FileSystem,
    /// 进程管理服务
    Process,
    /// 内存管理服务
    Memory,
    /// 设备驱动服务
    Device,
    /// 性能优化服务
    PerformanceOptimization,
    /// 调度器优化服务
    SchedulerOptimization,
    /// 零拷贝I/O优化服务
    ZeroCopyOptimization,
    /// 用户自定义服务
    Custom,
}

/// 依赖图
/// 
/// 管理服务间的依赖关系。
#[derive(Debug)]
pub struct DependencyGraph {
    /// 依赖关系映射
    /// 
    /// 键：服务名称，值：依赖的服务名称列表
    dependencies: BTreeMap<String, Vec<String>>,
    /// 反向依赖映射
    /// 
    /// 键：服务名称，值：依赖此服务的服务名称列表
    dependents: BTreeMap<String, Vec<String>>,
}

impl ServiceRegistry {
    /// 创建新的服务注册表
    /// 
    /// 初始化一个空的服务注册表。
    pub fn new() -> Self {
        Self {
            services: Arc::new(Mutex::new(BTreeMap::new())),
            syscall_mapping: Arc::new(Mutex::new(BTreeMap::new())),
            syscall_default_versions: Arc::new(Mutex::new(BTreeMap::new())),
            dependency_graph: Arc::new(Mutex::new(DependencyGraph::new())),
        }
    }
    
    /// 注册服务
    /// 
    /// 将服务注册到注册表中。
    /// 
    /// # 参数
    /// 
    /// * `service` - 要注册的服务实例
    /// * `metadata` - 服务元数据
    /// 
    /// # 返回值
    /// 
    /// * `Ok(())` - 注册成功
    /// * `Err(Error)` - 注册失败，包含错误信息
    pub fn register_service(
        &self,
        service: Arc<dyn Service>,
        metadata: ServiceMetadata,
    ) -> Result<()> {
        let service_name = service.name().to_string();
        
        // 检查服务是否已存在
        {
            let services = self.services.lock();
            if services.contains_key(&service_name) {
                return Err(KernelError::AlreadyExists("Service already exists".to_string()));
            }
        }
        
        // 验证依赖关系
        self.validate_dependencies(&service)?;
        
        // 创建服务条目
        let entry = ServiceEntry {
            service: service.clone(),
            metadata: metadata.clone(),
            registration_time: self.get_current_timestamp(),
        };
        
        // 注册服务
        {
            let mut services = self.services.lock();
            services.insert(service_name.clone(), entry);
        }
        
        // 更新依赖图
        {
            let mut dep_graph = self.dependency_graph.lock();
            dep_graph.add_service(&service_name, &metadata);
        }
        
        // 如果是系统调用服务，更新系统调用映射
        if metadata.is_syscall_service {
            self.update_syscall_mapping(&service_name)?;
        }
        
        Ok(())
    }
    
    /// 注销服务
    /// 
    /// 从注册表中移除服务。
    /// 
    /// # 参数
    /// 
    /// * `name` - 要注销的服务名称
    /// 
    /// # 返回值
    /// 
    /// * `Ok(())` - 注销成功
    /// * `Err(KernelError)` - 注销失败，包含错误信息
    pub fn unregister_service(&self, name: &str) -> Result<()> {
        // 检查是否有其他服务依赖此服务
        {
            let dep_graph = self.dependency_graph.lock();
            if let Some(dependents) = dep_graph.get_dependents(name) {
                if !dependents.is_empty() {
                    return Err(KernelError::InvalidOperation(format!("Service has dependents: {:?}", dependents)));
                }
            }
        }
        
        // 移除服务
        {
            let mut services = self.services.lock();
            if let Some(mut entry) = services.remove(name) {
                // 停止服务
                let _ = entry.service.stop();
            } else {
                return Err(KernelError::NotFound(format!("Service not found: {}", name)));
            }
        }
        
        // 更新依赖图
        {
            let mut dep_graph = self.dependency_graph.lock();
            dep_graph.remove_service(name);
        }
        
        // 清理系统调用映射
        {
            let mut syscall_map = self.syscall_mapping.lock();
            for (_, version_map) in syscall_map.iter_mut() {
                version_map.retain(|_, service_name| service_name != name);
            }
            // 清理空的版本映射
            syscall_map.retain(|_, version_map| !version_map.is_empty());
        }
        
        Ok(())
    }
    
    /// 获取服务实例（不可变引用）
    ///
    /// 根据名称获取已注册的服务实例。
    ///
    /// # 参数
    ///
    /// * `name` - 服务名称
    ///
    /// # 返回值
    ///
    /// * `Ok(Some(Arc<dyn Service>))` - 找到服务引用
    /// * `Ok(None)` - 服务不存在
    pub fn get_service_ref(&self, name: &str) -> Result<Option<Arc<dyn Service>>> {
        let services = self.services.lock();
        if let Some(entry) = services.get(name) {
            // Return a clone of the Arc to the service
            Ok(Some(entry.service.clone()))
        } else {
            Ok(None)
        }
    }

    /// 获取服务实例（可变引用）
    ///
    /// 根据名称获取已注册的服务实例，用于修改操作。
    /// 注意：由于服务存储在 Arc 中，此方法不再可用。
    /// 请使用内部可变性模式（如 Mutex 或 RefCell）来实现可变访问。
    ///
    /// # 参数
    ///
    /// * `name` - 服务名称
    ///
    /// # 返回值
    ///
    /// * `Err(Error)` - 不支持的操作
    #[deprecated(note = "Use Arc<Service> with internal mutability instead")]
    pub fn get_service_mut_ref(&self, _name: &str) -> Result<Option<&mut dyn Service>> {
        Err(KernelError::InvalidOperation("Operation not supported: Use Arc<Service> with internal mutability instead".to_string()))
    }
    
    /// 获取系统调用服务
    ///
    /// 根据系统调用号和版本获取对应的服务。
    /// 如果版本为None，则使用默认版本。
    ///
    /// # 参数
    ///
    /// * `syscall_number` - 系统调用号
    /// * `version` - 系统调用版本（可选）
    ///
    /// # 返回值
    ///
    /// * `Ok(Some(String))` - 找到服务名称
    /// * `Ok(None)` - 未找到对应服务
    pub fn get_syscall_service(&self, syscall_number: u32, version: Option<Version>) -> Result<Option<String>> {
        let syscall_map = self.syscall_mapping.lock();
        
        if let Some(version_map) = syscall_map.get(&syscall_number) {
            // 确定要使用的版本
            let target_version = if let Some(version) = version {
                version
            } else {
                // 使用默认版本
                let default_versions = self.syscall_default_versions.lock();
                match default_versions.get(&syscall_number) {
                    Some(v) => *v,
                    None => return Ok(None), // 没有默认版本
                }
            };
            
            // 查找最匹配的版本
            // 查找小于等于目标版本的最新版本
            let matching_version = version_map.range(..=target_version).last();
            
            if let Some((_, service_name)) = matching_version {
                Ok(Some(service_name.clone()))
            } else {
                // 没有找到兼容的版本
                Ok(None)
            }
        } else {
            // 没有找到该系统调用
            Ok(None)
        }
    }
    
    /// 获取系统调用服务的所有版本
    ///
    /// 根据系统调用号获取所有可用的版本。
    ///
    /// # 参数
    ///
    /// * `syscall_number` - 系统调用号
    ///
    /// # 返回值
    ///
    /// * `Ok(Option<Vec<Version>>)` - 找到的版本列表
    pub fn get_syscall_versions(&self, syscall_number: u32) -> Result<Option<Vec<Version>>> {
        let syscall_map = self.syscall_mapping.lock();
        
        if let Some(version_map) = syscall_map.get(&syscall_number) {
            Ok(Some(version_map.keys().cloned().collect()))
        } else {
            Ok(None)
        }
    }
    
    /// 设置系统调用的默认版本
    ///
    /// # 参数
    ///
    /// * `syscall_number` - 系统调用号
    /// * `version` - 要设置的默认版本
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 设置成功
    /// * `Err(Error)` - 设置失败
    pub fn set_syscall_default_version(&self, syscall_number: u32, version: Version) -> Result<()> {
        let syscall_map = self.syscall_mapping.lock();
        
        if let Some(version_map) = syscall_map.get(&syscall_number) {
            if version_map.contains_key(&version) {
                drop(syscall_map);
                let mut default_versions = self.syscall_default_versions.lock();
                default_versions.insert(syscall_number, version);
                Ok(())
            } else {
                Err(KernelError::NotFound("Version not found for syscall".to_string()))
            }
        } else {
            Err(KernelError::NotFound("Syscall not found".to_string()))
        }
    }
    
    /// 列出所有服务
    /// 
    /// 返回所有已注册服务的名称列表。
    /// 
    /// # 返回值
    /// 
    /// * `Vec<String>` - 服务名称列表
    pub fn list_services(&self) -> Vec<String> {
        let services = self.services.lock();
        services.keys().cloned().collect()
    }
    
    /// 按类型列出服务
    /// 
    /// 返回指定类型的所有服务。
    /// 
    /// # 参数
    /// 
    /// * `service_type` - 服务类型
    /// 
    /// # 返回值
    /// 
    /// * `Vec<String>` - 服务名称列表
    pub fn list_services_by_type(&self, service_type: ServiceType) -> Vec<String> {
        let services = self.services.lock();
        services
            .iter()
            .filter(|(_, entry)| entry.metadata.service_type == service_type)
            .map(|(name, _)| name.clone())
            .collect()
    }
    
    /// 获取服务状态
    /// 
    /// 返回指定服务的当前状态。
    /// 
    /// # 参数
    /// 
    /// * `name` - 服务名称
    /// 
    /// # 返回值
    /// 
    /// * `Ok(ServiceStatus)` - 服务状态
    /// * `Err(KernelError)` - 获取失败
    pub fn get_service_status(&self, name: &str) -> Result<ServiceStatus> {
        let services = self.services.lock();
        if let Some(entry) = services.get(name) {
            Ok(entry.service.status())
        } else {
            Err(KernelError::NotFound(format!("Service not found: {}", name)))
        }
    }
    
    /// 获取服务依赖关系
    /// 
    /// 返回指定服务的依赖列表。
    /// 
    /// # 参数
    /// 
    /// * `name` - 服务名称
    /// 
    /// # 返回值
    /// 
    /// * `Ok(Vec<String>)` - 依赖服务列表
    pub fn get_service_dependencies(&self, name: &str) -> Result<Vec<String>> {
        let dep_graph = self.dependency_graph.lock();
        Ok(dep_graph.get_dependencies(name).cloned().unwrap_or_default())
    }
    
    /// 计算服务启动顺序
    /// 
    /// 根据依赖关系计算服务的启动顺序。
    /// 
    /// # 返回值
    /// 
    /// * `Ok(Vec<String>)` - 按启动顺序排列的服务名称列表
    /// * `Err(Error)` - 计算失败（如存在循环依赖）
    pub fn calculate_startup_order(&self) -> Result<Vec<String>> {
        let dep_graph = self.dependency_graph.lock();
        dep_graph.topological_sort()
    }
    
    /// 验证依赖关系
    /// 
    /// 验证服务的依赖关系是否有效。
    /// 
    /// # 参数
    /// 
    /// * `service` - 要验证的服务
    /// 
    /// # 返回值
    /// 
    /// * `Ok(())` - 验证通过
    /// * `Err(KernelError)` - 验证失败
    fn validate_dependencies(&self, service: &Arc<dyn Service>) -> Result<()> {
        let dependencies = service.dependencies();
        let services = self.services.lock();
        
        for dep in dependencies {
            if !services.contains_key(dep) {
                return Err(KernelError::NotFound(format!("Dependency not found: {}", dep)));
            }
        }
        
        Ok(())
    }
    
    /// 更新系统调用映射
    ///
    /// 为系统调用服务更新系统调用号映射。
    ///
    /// # 参数
    ///
    /// * `service_name` - 服务名称
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 更新成功
    /// * `Err(KernelError)` - 更新失败
    fn update_syscall_mapping(&self, service_name: &str) -> Result<()> {
        let services = self.services.lock();
        let service_entry = services.get(service_name)
            .ok_or_else(|| KernelError::NotFound(format!("Service not found: {}", service_name)))?;

        // 尝试将服务转换为 SyscallService
        let syscall_service = service_entry.service.as_any()
            .downcast_ref::<dyn SyscallService>();

        if let Some(syscall_service) = syscall_service {
            // 获取服务支持的系统调用
            let syscalls = syscall_service.supported_syscalls();
            // 解析服务版本
            let service_version = Version::from_str(syscall_service.version())
                .map_err(|e| KernelError::InvalidInput(format!("Invalid version: {}", e)))?;
            drop(services); // 释放服务锁

            // 更新系统调用映射
            let mut syscall_map = self.syscall_mapping.lock();
            for syscall in syscalls {
                // 获取或创建版本映射
                let version_map = syscall_map.entry(syscall).or_default();
                
                // 插入新的版本
                version_map.insert(service_version, service_name.to_string());
                
                // 更新默认版本为最新版本
                let mut default_versions = self.syscall_default_versions.lock();
                let current_default = default_versions.get(&syscall);
                
                if current_default.is_none() || service_version > *current_default.unwrap() {
                    default_versions.insert(syscall, service_version);
                }
            }
        }

        Ok(())
    }
    
    /// 获取当前时间戳
    /// 
    /// 返回当前的时间戳（简化实现）。
    fn get_current_timestamp(&self) -> u64 {
        // 这里应该实现真实的时间戳获取
        // 暂时返回固定值
        0
    }
}

impl DependencyGraph {
    /// 创建新的依赖图
    pub fn new() -> Self {
        Self {
            dependencies: BTreeMap::new(),
            dependents: BTreeMap::new(),
        }
    }
    
    /// 添加服务
    /// 
    /// 将服务添加到依赖图中。
    pub fn add_service(&mut self, name: &str, metadata: &ServiceMetadata) {
        // 初始化依赖关系（这里需要从服务实例获取）
        self.dependencies.insert(name.to_string(), Vec::new());
        self.dependents.insert(name.to_string(), Vec::new());
    }
    
    /// 移除服务
    /// 
    /// 从依赖图中移除服务。
    pub fn remove_service(&mut self, name: &str) {
        self.dependencies.remove(name);
        self.dependents.remove(name);
        
        // 从其他服务的依赖列表中移除
        for deps in self.dependencies.values_mut() {
            deps.retain(|dep| dep != name);
        }
        
        // 从其他服务的依赖者列表中移除
        for dependents in self.dependents.values_mut() {
            dependents.retain(|dependent| dependent != name);
        }
    }
    
    /// 获取服务的依赖
    pub fn get_dependencies(&self, name: &str) -> Option<&Vec<String>> {
        self.dependencies.get(name)
    }
    
    /// 获取服务的依赖者
    pub fn get_dependents(&self, name: &str) -> Option<&Vec<String>> {
        self.dependents.get(name)
    }
    
    /// 拓扑排序
    /// 
    /// 计算服务的启动顺序。
    pub fn topological_sort(&self) -> Result<Vec<String>, KernelError> {
        let mut visited = BTreeMap::new();
        let mut result = Vec::new();
        let temp_mark = 1;
        let perm_mark = 2;
        
        for service in self.dependencies.keys() {
            if !visited.contains_key(service) {
                self.visit(service, &mut visited, &mut result, temp_mark, perm_mark)?;
            }
        }
        
        result.reverse();
        Ok(result)
    }
    
    /// 深度优先访问
    fn visit(
        &self,
        service: &str,
        visited: &mut BTreeMap<String, u32>,
        result: &mut Vec<String>,
        temp_mark: u32,
        perm_mark: u32,
    ) -> Result<(), KernelError> {
        if let Some(mark) = visited.get(service) {
            if *mark == temp_mark {
                return Err(KernelError::InvalidOperation(format!("Circular dependency detected: {}", service)));
            }
            if *mark == perm_mark {
                return Ok(());
            }
        }
        
        visited.insert(service.to_string(), temp_mark);
        
        if let Some(deps) = self.dependencies.get(service) {
            for dep in deps {
                self.visit(dep, visited, result, temp_mark, perm_mark)?;
            }
        }
        
        visited.insert(service.to_string(), perm_mark);
        result.push(service.to_string());
        
        Ok(())
    }
}

// 为Service trait添加as_any方法，用于类型转换
pub trait ServiceAsAny {
    fn as_any(&self) -> &dyn core::any::Any;
}

impl<T: Service + core::any::Any> ServiceAsAny for T {
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}

/// 服务注册错误类型
/// 
/// 定义服务注册过程中可能出现的错误。
#[derive(Debug, Clone)]
pub enum ServiceRegistryError {
    /// 服务已存在
    ServiceAlreadyExists(String),
    /// 服务未找到
    ServiceNotFound(String),
    /// 依赖服务未找到
    DependencyNotFound(String, String),
    /// 循环依赖
    CircularDependency(String),
    /// 服务有依赖者
    ServiceHasDependents(String, Vec<String>),
}

impl core::fmt::Display for ServiceRegistryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ServiceRegistryError::ServiceAlreadyExists(name) => {
                write!(f, "Service '{}' already exists", name)
            }
            ServiceRegistryError::ServiceNotFound(name) => {
                write!(f, "Service '{}' not found", name)
            }
            ServiceRegistryError::DependencyNotFound(service, dep) => {
                write!(f, "Dependency '{}' for service '{}' not found", dep, service)
            }
            ServiceRegistryError::CircularDependency(service) => {
                write!(f, "Circular dependency detected involving service '{}'", service)
            }
            ServiceRegistryError::ServiceHasDependents(service, dependents) => {
                write!(f, "Service '{}' has dependents: {:?}", service, dependents)
            }
        }
    }
}