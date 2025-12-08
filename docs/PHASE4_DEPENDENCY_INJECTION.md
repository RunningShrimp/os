# NOS内核依赖注入和控制反转设计文档

## 概述

本文档描述了NOS内核依赖注入（Dependency Injection, DI）和控制反转（Inversion of Control, IoC）的设计，旨在降低模块间耦合度，提高系统的可测试性、可配置性和可扩展性。

## 设计目标

### 1. 降低耦合度
- 消除硬编码的依赖关系
- 支持模块的独立开发和测试
- 便于模块的替换和升级

### 2. 提高可测试性
- 支持依赖的模拟和存根
- 便于单元测试和集成测试
- 支持测试环境的快速搭建

### 3. 增强可配置性
- 支持运行时依赖配置
- 支持多种实现策略
- 支持条件依赖和可选依赖

### 4. 支持插件化架构
- 支持动态模块加载
- 支持模块的热替换
- 支持第三方模块集成

## 核心概念

### 1. 依赖注入容器

依赖注入容器是DI框架的核心组件，负责：
- 管理服务注册和解析
- 处理依赖关系和生命周期
- 提供依赖解析服务

### 2. 服务定义

服务是可注入的组件，具有：
- 唯一的服务标识符
- 明确的接口定义
- 生命周期管理策略
- 依赖关系声明

### 3. 依赖关系

依赖关系描述服务间的依赖：
- 强依赖（必需依赖）
- 弱依赖（可选依赖）
- 循环依赖检测和处理
- 依赖版本兼容性

## 架构设计

### 1. 整体架构图

```
┌─────────────────────────────────────────────────┐
│              应用层和用户代码                  │
├─────────────────────────────────────────────────┤
│              服务接口层                      │
│         (服务接口定义和抽象)                 │
├─────────────────────────────────────────────────┤
│              DI容器层                        │
│    (容器管理、依赖解析、生命周期)            │
├─────────────────────────────────────────────────┤
│              服务实现层                        │
│        (具体服务实现和配置)                 │
├─────────────────────────────────────────────────┤
│              配置和策略层                      │
│     (注入策略、生命周期策略、配置管理)          │
└─────────────────────────────────────────────────┘
```

### 2. 核心组件

#### 2.1 服务接口定义

```rust
// 服务标识符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ServiceId {
    pub namespace: u32,
    pub name: u32,
    pub version: u32,
}

impl ServiceId {
    pub const fn new(namespace: u32, name: u32, version: u32) -> Self {
        Self { namespace, name, version }
    }
    
    pub fn to_string(&self) -> String {
        format!("service:{}:{}:{}", self.namespace, self.name, self.version)
    }
}

// 服务特征标记
pub trait ServiceMarker: 'static {
    const SERVICE_ID: ServiceId;
    const INTERFACE_VERSION: InterfaceVersion;
}

// 服务接口
pub trait ServiceInterface: Send + Sync {
    fn service_id(&self) -> ServiceId;
    fn interface_version(&self) -> InterfaceVersion;
    fn health_check(&self) -> ServiceHealth;
    fn get_dependencies(&self) -> Vec<ServiceId>;
}
```

#### 2.2 依赖注入容器

```rust
// 容器配置
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    pub enable_circular_dependency_detection: bool,
    pub enable_lazy_initialization: bool,
    pub enable_service_caching: bool,
    pub max_service_instances: Option<usize>,
    pub default_lifecycle_strategy: LifecycleStrategy,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            enable_circular_dependency_detection: true,
            enable_lazy_initialization: true,
            enable_service_caching: true,
            max_service_instances: Some(1000),
            default_lifecycle_strategy: LifecycleStrategy::Singleton,
        }
    }
}

// 依赖注入容器
pub struct DIContainer {
    config: ContainerConfig,
    services: HashMap<ServiceId, ServiceEntry>,
    instances: HashMap<ServiceId, ServiceInstance>,
    dependency_graph: DependencyGraph,
    lifecycle_manager: Box<dyn LifecycleManager>,
    configuration_manager: Box<dyn ConfigurationManager>,
}

impl DIContainer {
    pub fn new(config: ContainerConfig) -> Self {
        Self {
            config,
            services: HashMap::new(),
            instances: HashMap::new(),
            dependency_graph: DependencyGraph::new(),
            lifecycle_manager: Box::new(DefaultLifecycleManager::new()),
            configuration_manager: Box::new(DefaultConfigurationManager::new()),
        }
    }
    
    // 注册服务工厂
    pub fn register_service_factory<F, I>(&mut self, factory: F) -> Result<(), DIError>
    where
        F: Fn(&DIContainer) -> Result<I, DIError> + 'static,
        I: ServiceInterface + 'static,
    {
        let service_id = I::SERVICE_ID;
        
        // 检查服务是否已注册
        if self.services.contains_key(&service_id) {
            return Err(DIError::ServiceAlreadyRegistered(service_id));
        }
        
        // 创建服务条目
        let entry = ServiceEntry {
            service_id,
            interface_id: InterfaceId::of::<I>(),
            factory: Box::new(factory),
            lifecycle_strategy: self.config.default_lifecycle_strategy,
            dependencies: I::get_dependencies(&service_id),
            registration_time: self.get_current_time(),
        };
        
        // 注册到容器
        self.services.insert(service_id, entry);
        
        // 更新依赖图
        self.dependency_graph.add_service(service_id, &entry.dependencies)?;
        
        Ok(())
    }
    
    // 注册服务实例
    pub fn register_service_instance<I>(&mut self, instance: I) -> Result<(), DIError>
    where
        I: ServiceInterface + 'static,
    {
        let service_id = I::SERVICE_ID;
        
        if self.instances.contains_key(&service_id) {
            return Err(DIError::ServiceAlreadyRegistered(service_id));
        }
        
        let serviceInstance = ServiceInstance {
            service_id,
            instance: Box::new(instance),
            lifecycle_state: LifecycleState::Registered,
            creation_time: self.get_current_time(),
            access_count: 0,
        };
        
        self.instances.insert(service_id, ServiceInstance);
        Ok(())
    }
    
    // 解析服务
    pub fn resolve_service<I>(&self) -> Result<Arc<I>, DIError>
    where
        I: ServiceInterface + 'static,
    {
        let serviceId = I::SERVICE_ID;
        
        // 检查循环依赖
        if self.config.enable_circular_dependency_detection {
            self.dependency_graph.check_circular_dependency(&service_id)?;
        }
        
        // 尝试从实例缓存获取
        if let Some(instance) = self.instances.get(&service_id) {
            return Ok(Arc::clone(&instance.instance) as Arc<I>);
        }
        
        // 创建新实例
        let instance = self.create_service_instance::<I>(&service_id)?;
        
        // 注入依赖
        self.inject_dependencies(&instance)?;
        
        // 初始化服务
        self.lifecycle_manager.initialize(&instance)?;
        
        Ok(Arc::clone(&instance.instance) as Arc<I>)
    }
    
    // 创建服务实例
    fn create_service_instance<I>(&self, service_id: &ServiceId) -> Result<ServiceInstance, DIError>
    where
        I: ServiceInterface + 'static,
    {
        let entry = self.services.get(service_id)
            .ok_or(DIError::ServiceNotFound(*service_id))?;
        
        // 调用工厂函数创建实例
        let instance = (entry.factory)(self)?;
        
        Ok(ServiceInstance {
            service_id: *service_id,
            instance: Box::new(instance),
            lifecycle_state: LifecycleState::Created,
            creation_time: self.get_current_time(),
            access_count: 0,
        })
    }
    
    // 注入依赖
    fn inject_dependencies<I>(&self, instance: &ServiceInstance) -> Result<(), DIError>
    where
        I: ServiceInterface + 'static,
    {
        let serviceId = I::SERVICE_ID;
        let dependencies = self.get_service_dependencies(&service_id)?;
        
        for dep_id in dependencies {
            let dep_service = self.resolve_service_by_id(&dep_id)?;
            
            // 使用反射或特设机制注入依赖
            self.inject_dependency(instance, &dep_service)?;
        }
        
        Ok(())
    }
}
```

#### 2.3 生命周期管理

```rust
// 生命周期状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Unregistered,
    Registered,
    Created,
    Initializing,
    Initialized,
    Starting,
    Running,
    Stopping,
    Stopped,
    Destroyed,
    Error(DIError),
}

// 生命周期策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleStrategy {
    Singleton,      // 单例模式
    Transient,      // 瞬时模式，每次请求创建新实例
    Scoped,         // 作用域模式，在特定作用域内有效
    Custom(Box<dyn CustomLifecycleStrategy>), // 自定义策略
}

// 生命周期管理器接口
pub trait LifecycleManager {
    fn initialize(&self, instance: &ServiceInstance) -> Result<(), DIError>;
    fn start(&self, instance: &ServiceInstance) -> Result<(), DIError>;
    fn stop(&self, instance: &ServiceInstance) -> Result<(), DIError>;
    fn destroy(&self, instance: &ServiceInstance) -> Result<(), DIError>;
    fn get_state(&self, service_id: ServiceId) -> Option<LifecycleState>;
}

// 默认生命周期管理器
pub struct DefaultLifecycleManager {
    state_map: HashMap<ServiceId, LifecycleState>,
    event_listeners: Vec<Box<dyn LifecycleEventListener>>,
}

impl LifecycleManager for DefaultLifecycleManager {
    fn initialize(&self, instance: &ServiceInstance) -> Result<(), DIError> {
        // 执行初始化逻辑
        self.set_state(instance.service_id, LifecycleState::Initializing);
        
        // 调用服务的初始化方法
        if let Some(init_service) = instance.instance.as_any().downcast_ref::<dyn Initializable>() {
            init_service.initialize()?;
        }
        
        self.set_state(instance.service_id, LifecycleState::Initialized);
        self.notify_listeners(instance.service_id, LifecycleEvent::Initialized);
        
        Ok(())
    }
    
    fn start(&self, instance: &ServiceInstance) -> Result<(), DIError> {
        self.set_state(instance.service_id, LifecycleState::Starting);
        
        // 调用服务的启动方法
        if let Some(startable_service) = instance.instance.as_any().downcast_ref::<dyn Startable>() {
            startable_service.start()?;
        }
        
        self.set_state(instance.service_id, LifecycleState::Running);
        self.notify_listeners(instance.service_id, LifecycleEvent::Started);
        
        Ok(())
    }
    
    fn stop(&self, instance: &ServiceInstance) -> Result<(), DIError> {
        self.set_state(instance.service_id, LifecycleState::Stopping);
        
        // 调用服务的停止方法
        if let Some(stoppable_service) = instance.instance.as_any().downcast_ref::<dyn Stoppable>() {
            stoppable_service.stop()?;
        }
        
        self.set_state(instance.service_id, LifecycleState::Stopped);
        self.notify_listeners(instance.service_id, LifecycleEvent::Stopped);
        
        Ok(())
    }
    
    fn destroy(&self, instance: &ServiceInstance) -> Result<(), DIError> {
        self.set_state(instance.service_id, LifecycleState::Destroyed);
        
        // 调用服务的销毁方法
        if let Some(destroyable_service) = instance.instance.as_any().downcast_ref::<dyn Destroyable>() {
            destroyable_service.destroy()?;
        }
        
        self.notify_listeners(instance.service_id, LifecycleEvent::Destroyed);
        
        Ok(())
    }
}
```

### 3. 依赖关系管理

#### 3.1 依赖图

```rust
// 依赖图结构
pub struct DependencyGraph {
    nodes: HashMap<ServiceId, DependencyNode>,
    edges: Vec<DependencyEdge>,
    circular_detector: CircularDependencyDetector,
}

// 依赖节点
#[derive(Debug, Clone)]
pub struct DependencyNode {
    pub service_id: ServiceId,
    pub dependencies: Vec<ServiceId>,
    pub dependents: Vec<ServiceId>,
    pub dependency_level: u32,
}

// 依赖边
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    pub from: ServiceId,
    pub to: ServiceId,
    pub dependency_type: DependencyType,
    pub strength: DependencyStrength,
}

// 依赖类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    Required,    // 必需依赖
    Optional,    // 可选依赖
    Conditional,  // 条件依赖
}

// 依赖强度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DependencyStrength {
    Weak = 0,     // 弱依赖
    Normal = 1,   // 普通依赖
    Strong = 2,    // 强依赖
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            circular_detector: CircularDependencyDetector::new(),
        }
    }
    
    // 添加服务依赖
    pub fn add_service(&mut self, service_id: ServiceId, dependencies: &[ServiceId]) -> Result<(), DIError> {
        // 创建依赖节点
        let node = DependencyNode {
            service_id,
            dependencies: dependencies.to_vec(),
            dependents: Vec::new(),
            dependency_level: 0, // 将在拓扑排序时计算
        };
        
        self.nodes.insert(service_id, node);
        
        // 添加依赖边
        for &dep_id in dependencies {
            let edge = DependencyEdge {
                from: service_id,
                to: dep_id,
                dependency_type: DependencyType::Required,
                strength: DependencyStrength::Normal,
            };
            self.edges.push(edge);
        }
        
        Ok(())
    }
    
    // 拓扑排序
    pub fn topological_sort(&self) -> Result<Vec<ServiceId>, DIError> {
        // 检测循环依赖
        self.circular_detector.detect(&self.nodes, &self.edges)?;
        
        // 实现拓扑排序算法
        let mut in_degree: HashMap<ServiceId, u32> = HashMap::new();
        let mut queue: VecDeque<ServiceId> = VecDeque::new();
        let mut result: Vec<ServiceId> = Vec::new();
        
        // 计算入度
        for node in self.nodes.values() {
            in_degree.insert(node.service_id, node.dependencies.len() as u32);
            
            if node.dependencies.is_empty() {
                queue.push_back(node.service_id);
            }
        }
        
        // 拓扑排序
        while let Some(service_id) = queue.pop_front() {
            result.push(service_id);
            
            // 更新依赖该服务的其他服务的入度
            if let Some(node) = self.nodes.get(&service_id) {
                for &dep_id in &node.dependents {
                    if let Some(degree) = in_degree.get_mut(&dep_id) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dep_id);
                        }
                    }
                }
            }
        }
        
        // 检查是否所有服务都被处理
        if result.len() != self.nodes.len() {
            return Err(DIError::CircularDependencyDetected);
        }
        
        Ok(result)
    }
}
```

#### 3.2 循环依赖检测

```rust
// 循环依赖检测器
pub struct CircularDependencyDetector {
    visited: HashSet<ServiceId>,
    recursion_stack: Vec<ServiceId>,
    detection_enabled: bool,
}

impl CircularDependencyDetector {
    pub fn new() -> Self {
        Self {
            visited: HashSet::new(),
            recursion_stack: Vec::new(),
            detection_enabled: true,
        }
    }
    
    // 检测循环依赖
    pub fn detect(&mut self, nodes: &HashMap<ServiceId, DependencyNode>, edges: &[DependencyEdge]) -> Result<(), DIError> {
        if !self.detection_enabled {
            return Ok(());
        }
        
        self.visited.clear();
        self.recursion_stack.clear();
        
        // 对每个节点进行DFS检测
        for service_id in nodes.keys() {
            if !self.visited.contains(service_id) {
                self.dfs_detect(service_id, nodes, edges)?;
            }
        }
        
        Ok(())
    }
    
    // 深度优先搜索检测
    fn dfs_detect(&mut self, service_id: ServiceId, nodes: &HashMap<ServiceId, DependencyNode>, edges: &[DependencyEdge]) -> Result<(), DIError> {
        self.visited.insert(service_id);
        self.recursion_stack.push(service_id);
        
        if let Some(node) = nodes.get(&service_id) {
            for &dep_id in &node.dependencies {
                if self.recursion_stack.contains(&dep_id) {
                    // 发现循环依赖
                    let cycle = self.extract_cycle(&dep_id);
                    return Err(DIError::CircularDependencyDetected);
                }
                
                if !self.visited.contains(&dep_id) {
                    self.dfs_detect(dep_id, nodes, edges)?;
                }
            }
        }
        
        self.recursion_stack.pop();
        Ok(())
    }
    
    // 提取循环路径
    fn extract_cycle(&self, start_id: &ServiceId) -> Vec<ServiceId> {
        let mut cycle = Vec::new();
        let mut found = false;
        
        for &service_id in &self.recursion_stack {
            cycle.push(service_id);
            if service_id == *start_id {
                found = true;
                break;
            }
        }
        
        if !found {
            cycle.clear();
        }
        
        cycle
    }
}
```

### 4. 配置管理

#### 4.1 配置接口

```rust
// 配置管理器接口
pub trait ConfigurationManager {
    fn get_configuration(&self, key: &str) -> Option<ConfigurationValue>;
    fn set_configuration(&mut self, key: &str, value: ConfigurationValue) -> Result<(), ConfigurationError>;
    fn get_service_configuration(&self, service_id: ServiceId) -> HashMap<String, ConfigurationValue>;
    fn set_service_configuration(&mut self, service_id: ServiceId, config: HashMap<String, ConfigurationValue>) -> Result<(), ConfigurationError>;
    fn reload_configuration(&mut self) -> Result<(), ConfigurationError>;
    fn save_configuration(&self) -> Result<(), ConfigurationError>;
}

// 默认配置管理器
pub struct DefaultConfigurationManager {
    global_config: HashMap<String, ConfigurationValue>,
    service_configs: HashMap<ServiceId, HashMap<String, ConfigurationValue>>,
    config_sources: Vec<Box<dyn ConfigurationSource>>,
}

impl ConfigurationManager for DefaultConfigurationManager {
    fn get_configuration(&self, key: &str) -> Option<ConfigurationValue> {
        // 优先从环境变量获取
        if let Some(value) = self.get_environment_config(key) {
            return Some(value);
        }
        
        // 从全局配置获取
        self.global_config.get(key).cloned()
    }
    
    fn set_configuration(&mut self, key: &str, value: ConfigurationValue) -> Result<(), ConfigurationError> {
        self.global_config.insert(key.to_string(), value);
        
        // 通知配置变更监听器
        self.notify_config_change(key, &value);
        
        Ok(())
    }
    
    fn get_service_configuration(&self, service_id: ServiceId) -> HashMap<String, ConfigurationValue> {
        self.service_configs.get(&service_id)
            .cloned()
            .unwrap_or(HashMap::new())
    }
    
    fn set_service_configuration(&mut self, service_id: ServiceId, config: HashMap<String, ConfigurationValue>) -> Result<(), ConfigurationError> {
        self.service_configs.insert(service_id, config);
        
        // 通知服务配置变更
        self.notify_service_config_change(service_id, &config);
        
        Ok(())
    }
}
```

## 使用示例

### 1. 服务定义

```rust
// 定义服务接口
pub trait FileSystemService: ServiceInterface {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, ServiceError>;
    fn write_file(&self, path: &str, data: &[u8]) -> Result<(), ServiceError>;
    fn delete_file(&self, path: &str) -> Result<(), ServiceError>;
}

// 实现服务标记
impl ServiceMarker for dyn FileSystemService {
    const SERVICE_ID: ServiceId = ServiceId::new(1, 1, 1);
    const INTERFACE_VERSION: InterfaceVersion = InterfaceVersion::new(1, 0, 0);
}

// 实现服务
pub struct DefaultFileSystemService {
    config: Arc<RwLock<FileSystemConfig>>,
}

impl FileSystemService for DefaultFileSystemService {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, ServiceError> {
        // 实现文件读取逻辑
        Ok(Vec::new())
    }
    
    fn write_file(&self, path: &str, data: &[u8]) -> Result<(), ServiceError> {
        // 实现文件写入逻辑
        Ok(())
    }
    
    fn delete_file(&self, path: &str) -> Result<(), ServiceError> {
        // 实现文件删除逻辑
        Ok(())
    }
}
```

### 2. 服务注册

```rust
// 在模块初始化时注册服务
fn register_services(container: &mut DIContainer) -> Result<(), DIError> {
    // 注册文件系统服务
    container.register_service_factory(|_| {
        let config = container.get_service_configuration(
            <dyn FileSystemService as ServiceMarker>::SERVICE_ID
        );
        
        Ok(Box::new(DefaultFileSystemService::new(config)) as Box<dyn FileSystemService>)
    })?;
    
    // 注册其他服务...
    
    Ok(())
}
```

### 3. 服务使用

```rust
// 在其他模块中使用服务
fn use_filesystem_service(container: &DIContainer) -> Result<(), DIError> {
    // 解析文件系统服务
    let fs_service = container.resolve_service::<dyn FileSystemService>()?;
    
    // 使用服务
    let data = fs_service.read_file("/test.txt")?;
    fs_service.write_file("/output.txt", &data)?;
    
    Ok(())
}
```

## 性能优化

### 1. 服务缓存

```rust
// 服务缓存策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStrategy {
    NoCache,        // 不缓存
    WeakCache,       // 弱引用缓存
    StrongCache,      // 强引用缓存
    Custom(Box<dyn CustomCacheStrategy>), // 自定义策略
}

// 服务缓存实现
pub struct ServiceCache {
    cache: HashMap<ServiceId, CacheEntry>,
    strategy: CacheStrategy,
    max_size: usize,
    eviction_policy: EvictionPolicy,
}

impl ServiceCache {
    pub fn get<T>(&self, service_id: ServiceId) -> Option<Arc<T>>
    where
        T: ServiceInterface + 'static,
    {
        match self.strategy {
            CacheStrategy::NoCache => None,
            CacheStrategy::WeakCache => self.get_weak_cached(service_id),
            CacheStrategy::StrongCache => self.get_strong_cached(service_id),
            CacheStrategy::Custom(ref custom) => custom.get_cached(service_id),
        }
    }
    
    fn get_weak_cached<T>(&self, service_id: ServiceId) -> Option<Arc<T>> {
        self.cache.get(&service_id)
            .and_then(|entry| {
                if let Some(weak_ref) = entry.instance.as_any().downcast_ref::<Weak<T>>() {
                    weak_ref.upgrade()
                } else {
                    None
                }
            })
    }
}
```

### 2. 延迟初始化

```rust
// 延迟初始化包装器
pub struct LazyService<T> {
    factory: Box<dyn Fn() -> Result<T, DIError>>,
    instance: OnceCell<Result<T, DIError>>,
}

impl<T> LazyService<T> {
    pub fn new<F>(factory: F) -> Self
    where
        F: Fn() -> Result<T, DIError> + 'static,
    {
        Self {
            factory: Box::new(factory),
            instance: OnceCell::new(None),
        }
    }
    
    pub fn get(&self) -> Result<&T, DIError> {
        self.instance.get_or_try_init(|| {
            (self.factory)()
        })
    }
}
```

## 测试支持

### 1. 测试容器

```rust
// 测试专用容器
pub struct TestContainer {
    container: DIContainer,
    mocks: HashMap<ServiceId, Box<dyn ServiceInterface>>,
    test_config: TestConfiguration,
}

impl TestContainer {
    pub fn new() -> Self {
        let mut container = DIContainer::new(ContainerConfig::test_config());
        Self {
            container,
            mocks: HashMap::new(),
            test_config: TestConfiguration::default(),
        }
    }
    
    // 注册模拟服务
    pub fn register_mock<I>(&mut self, mock: I) -> Result<(), DIError>
    where
        I: ServiceInterface + 'static,
    {
        let service_id = I::SERVICE_ID;
        self.mocks.insert(service_id, Box::new(mock));
        self.container.register_service_instance(mock)?;
        Ok(())
    }
    
    // 验证模拟调用
    pub fn verify_mock_calls<I>(&self, expected_calls: &[MockCall]) -> bool
    where
        I: ServiceInterface + 'static,
    {
        if let Some(mock) = self.mocks.get(&I::SERVICE_ID) {
            mock.verify_calls(expected_calls)
        } else {
            false
        }
    }
}
```

### 2. 模拟服务

```rust
// 模拟服务基类
pub struct MockService<T> {
    service_id: ServiceId,
    calls: RefCell<Vec<ServiceCall>>,
    responses: HashMap<String, T>,
}

impl<T> MockService<T> {
    pub fn new() -> Self {
        Self {
            service_id: T::SERVICE_ID,
            calls: RefCell::new(Vec::new()),
            responses: HashMap::new(),
        }
    }
    
    // 设置响应
    pub fn set_response(&mut self, method: &str, response: T) {
        self.responses.insert(method.to_string(), response);
    }
    
    // 验证调用
    pub fn verify_calls(&self, expected_calls: &[ServiceCall]) -> bool {
        let actual_calls = self.calls.borrow();
        
        if actual_calls.len() != expected_calls.len() {
            return false;
        }
        
        for (i, expected_call) in expected_calls.iter().enumerate() {
            if i >= actual_calls.len() {
                return false;
            }
            
            if &actual_calls[i] != expected_call {
                return false;
            }
        }
        
        true
    }
}
```

## 迁移策略

### 1. 渐进式迁移

1. **第一阶段**：建立DI框架基础设施
2. **第二阶段**：迁移核心服务（内存、进程、文件系统）
3. **第三阶段**：迁移其他服务和模块
4. **第四阶段**：优化和性能调优

### 2. 兼容性保证

- 提供适配器模式支持旧代码
- 渐进式替换硬编码依赖
- 保持API向后兼容
- 提供迁移工具和文档

### 3. 验证和测试

- 单元测试覆盖所有DI组件
- 集成测试验证服务解析
- 性能测试确保DI开销可接受
- 压力测试验证容器稳定性

## 验收标准

### 1. 功能完整性
- [ ] DI容器支持服务注册和解析
- [ ] 支持多种生命周期策略
- [ ] 循环依赖检测和处理
- [ ] 配置管理功能完整

### 2. 性能指标
- [ ] 服务解析延迟<100μs
- [ ] 内存开销<5%
- [ ] 缓存命中率>80%
- [ ] 并发访问性能良好

### 3. 可维护性
- [ ] 代码重复率<10%
- [ ] 测试覆盖率>90%
- [ ] 文档完整性>95%
- [ ] 迁移工具完善

## 结论

通过实施这个依赖注入和控制反转设计，NOS内核将获得：

1. **松耦合架构**：模块间依赖关系清晰且可配置
2. **高可测试性**：支持模拟和单元测试
3. **良好扩展性**：支持插件化和动态模块加载
4. **配置灵活性**：支持运行时配置和策略调整

这个DI/IoC设计为NOS内核的模块化开发和长期维护提供了坚实的基础。