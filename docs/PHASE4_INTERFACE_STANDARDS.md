# NOS内核模块间接口标准化设计文档

## 概述

本文档定义了NOS内核模块间标准化接口的设计，旨在建立统一的通信协议，降低模块间耦合度，提高系统的可维护性和可扩展性。

## 设计原则

### 1. 接口隔离原则
- 模块间只能通过定义的接口进行交互
- 隐藏内部实现细节
- 支持接口的独立演进

### 2. 类型安全原则
- 使用强类型系统定义接口
- 编译时检查接口兼容性
- 避免运行时类型转换错误

### 3. 错误处理原则
- 统一的错误类型和错误传播机制
- 明确的错误码和错误信息
- 支持错误恢复和重试机制

### 4. 异步支持原则
- 支持异步操作和回调机制
- 非阻塞接口设计
- 超时和取消支持

## 核心接口定义

### 1. 基础服务接口

#### 1.1 服务标识和版本

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
        format!("{}.{}.{}", self.namespace, self.name, self.version)
    }
}

// 服务版本信息
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl ServiceVersion {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self { major, minor, patch }
    }
    
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
    
    pub fn is_compatible(&self, other: &ServiceVersion) -> bool {
        self.major == other.major && self.minor >= other.minor
    }
}
```

#### 1.2 服务状态和健康检查

```rust
// 服务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Uninitialized,
    Initializing,
    Ready,
    Starting,
    Running,
    Stopping,
    Stopped,
    Error(ServiceError),
    Degraded(ServiceDegradationReason),
}

// 服务健康状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Warning(WarningReason),
    Critical(CriticalReason),
    Unknown,
}

// 健康检查接口
pub trait HealthCheck {
    fn check_health(&self) -> HealthStatus;
    fn get_metrics(&self) -> ServiceMetrics;
    fn reset_metrics(&mut self);
}
```

#### 1.3 服务生命周期管理

```rust
// 生命周期事件
#[derive(Debug, Clone)]
pub enum LifecycleEvent {
    Initializing,
    Initialized,
    Starting,
    Started,
    Stopping,
    Stopped,
    Error(ServiceError),
}

// 生命周期监听器
pub trait LifecycleListener {
    fn on_lifecycle_event(&self, service_id: ServiceId, event: LifecycleEvent);
}

// 生命周期管理接口
pub trait LifecycleManager {
    fn register_listener(&mut self, listener: Box<dyn LifecycleListener>);
    fn unregister_listener(&mut self, listener_id: ListenerId);
    fn start_service(&mut self, service_id: ServiceId) -> Result<(), ServiceError>;
    fn stop_service(&mut self, service_id: ServiceId) -> Result<(), ServiceError>;
    fn restart_service(&mut self, service_id: ServiceId) -> Result<(), ServiceError>;
}
```

### 2. 通信接口标准

#### 2.1 消息定义和序列化

```rust
// 消息头
#[derive(Debug, Clone)]
pub struct MessageHeader {
    pub message_id: u64,
    pub source_service: ServiceId,
    pub target_service: ServiceId,
    pub message_type: MessageType,
    pub priority: MessagePriority,
    pub timestamp: u64,
    pub timeout: Option<u64>,
}

// 消息优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
    Background = 4,
}

// 消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Request,
    Response,
    Notification,
    Event,
    Heartbeat,
    Error,
}

// 标准消息接口
pub trait Message {
    fn header(&self) -> &MessageHeader;
    fn payload(&self) -> &[u8];
    fn serialize(&self) -> Result<Vec<u8>, SerializationError>;
    fn deserialize(data: &[u8]) -> Result<Box<dyn Message>, SerializationError>;
}
```

#### 2.2 请求-响应模式

```rust
// 请求消息
pub struct RequestMessage {
    header: MessageHeader,
    request_id: u64,
    operation: Operation,
    parameters: Vec<Parameter>,
}

// 响应消息
pub struct ResponseMessage {
    header: MessageHeader,
    request_id: u64,
    status: ResponseStatus,
    result: Option<OperationResult>,
    error: Option<ServiceError>,
}

// 响应状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseStatus {
    Success,
    PartialSuccess,
    Failure,
    Timeout,
    Cancelled,
}

// 操作参数
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub value: ParameterValue,
    pub parameter_type: ParameterType,
}

// 参数值类型
#[derive(Debug, Clone)]
pub enum ParameterValue {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Binary(Vec<u8>),
    Array(Vec<ParameterValue>),
    Object(HashMap<String, ParameterValue>),
}
```

#### 2.3 异步通信支持

```rust
// 异步请求句柄
pub struct AsyncRequestHandle {
    pub request_id: u64,
    pub completion_callback: Option<Box<dyn CompletionCallback>>,
    pub timeout_callback: Option<Box<dyn TimeoutCallback>>,
    pub cancel_callback: Option<Box<dyn CancelCallback>>,
}

// 完成回调
pub trait CompletionCallback {
    fn on_complete(&self, request_id: u64, result: OperationResult);
}

// 超时回调
pub trait TimeoutCallback {
    fn on_timeout(&self, request_id: u64);
}

// 取消回调
pub trait CancelCallback {
    fn on_cancelled(&self, request_id: u64);
}

// 异步通信接口
pub trait AsyncCommunication {
    fn send_async_request(&mut self, request: RequestMessage) -> Result<AsyncRequestHandle, CommunicationError>;
    fn wait_for_response(&self, handle: &AsyncRequestHandle) -> Result<ResponseMessage, CommunicationError>;
    fn cancel_request(&mut self, handle: AsyncRequestHandle) -> Result<(), CommunicationError>;
    fn poll_response(&self, handle: &AsyncRequestHandle) -> Option<ResponseMessage>;
}
```

### 3. 资源管理接口

#### 3.1 资源标识和分配

```rust
// 资源标识符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId {
    pub resource_type: ResourceType,
    pub instance_id: u32,
    pub namespace: u32,
}

// 资源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Memory,
    FileDescriptor,
    NetworkSocket,
    Device,
    Semaphore,
    SharedMemory,
    Timer,
}

// 资源描述符
#[derive(Debug, Clone)]
pub struct ResourceDescriptor {
    pub resource_id: ResourceId,
    pub access_mode: AccessMode,
    pub sharing_mode: SharingMode,
    pub owner: ServiceId,
    pub reference_count: u32,
}

// 访问模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    ReadOnly,
    WriteOnly,
    ReadWrite,
    Execute,
    Custom(u32),
}

// 共享模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharingMode {
    Exclusive,
    Shared,
    SharedRead,
    SharedWrite,
}
```

#### 3.2 资源生命周期管理

```rust
// 资源状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceState {
    Available,
    Allocated,
    InUse,
    Reserved,
    Error(ResourceError),
}

// 资源管理器接口
pub trait ResourceManager {
    fn allocate_resource(&mut self, resource_type: ResourceType, access_mode: AccessMode) -> Result<ResourceId, ResourceError>;
    fn deallocate_resource(&mut self, resource_id: ResourceId) -> Result<(), ResourceError>;
    fn get_resource_info(&self, resource_id: ResourceId) -> Option<ResourceDescriptor>;
    fn list_resources(&self, resource_type: ResourceType) -> Vec<ResourceDescriptor>;
    fn get_usage_statistics(&self) -> ResourceUsageStatistics;
}

// 资源使用统计
#[derive(Debug, Clone, Default)]
pub struct ResourceUsageStatistics {
    pub total_allocated: u32,
    pub peak_usage: u32,
    pub allocation_failures: u32,
    pub average_lifetime: u64,
    pub fragmentation_ratio: f32,
}
```

### 4. 错误处理标准化

#### 4.1 统一错误类型系统

```rust
// 根错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum KernelError {
    // 服务层错误
    ServiceError(ServiceError),
    
    // 通信错误
    CommunicationError(CommunicationError),
    
    // 资源错误
    ResourceError(ResourceError),
    
    // 验证错误
    ValidationError(ValidationError),
    
    // 系统错误
    SystemError(SystemError),
    
    // 硬件错误
    HardwareError(HardwareError),
    
    // 未知错误
    Unknown(String),
}

// 服务错误
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceError {
    ServiceNotFound(ServiceId),
    ServiceUnavailable(ServiceId),
    ServiceNotReady(ServiceId),
    ServiceStopped(ServiceId),
    InvalidOperation(Operation),
    OperationFailed(Operation, String),
    Timeout(Operation),
    PermissionDenied(ServiceId, Operation),
    ResourceExhausted(ResourceType),
}

// 通信错误
#[derive(Debug, Clone, PartialEq)]
pub enum CommunicationError {
    MessageTooLarge,
    MessageCorrupted,
    ConnectionFailed,
    ConnectionLost,
    Timeout,
    InvalidMessageFormat,
    UnsupportedMessageType,
    QueueFull,
    QueueEmpty,
}
```

#### 4.2 错误传播和恢复

```rust
// 错误传播策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorPropagationStrategy {
    Propagate,      // 向上传播错误
    HandleLocally,   // 在本地处理错误
    Retry,          // 重试操作
    Fallback,       // 使用备用方案
    Ignore,         // 忽略错误
}

// 错误恢复接口
pub trait ErrorRecovery {
    fn can_recover(&self, error: &KernelError) -> bool;
    fn attempt_recovery(&mut self, error: &KernelError) -> Result<(), KernelError>;
    fn get_recovery_strategy(&self, error: &KernelError) -> ErrorPropagationStrategy;
    fn log_error(&self, error: &KernelError);
}

// 错误上下文
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub error: KernelError,
    pub service_id: Option<ServiceId>,
    pub operation: Option<Operation>,
    pub timestamp: u64,
    pub stack_trace: Vec<String>,
    pub recovery_attempts: u32,
}
```

### 5. 配置和发现接口

#### 5.1 服务发现和注册

```rust
// 服务注册信息
#[derive(Debug, Clone)]
pub struct ServiceRegistration {
    pub service_id: ServiceId,
    pub service_name: String,
    pub service_version: ServiceVersion,
    pub capabilities: Vec<ServiceCapability>,
    pub dependencies: Vec<ServiceId>,
    pub interface: Box<dyn ServiceInterface>,
    pub metadata: ServiceMetadata,
}

// 服务能力
#[derive(Debug, Clone)]
pub struct ServiceCapability {
    pub capability_name: String,
    pub capability_version: ServiceVersion,
    pub parameters: Vec<ParameterDefinition>,
    pub performance_metrics: Vec<PerformanceMetric>,
}

// 服务发现接口
pub trait ServiceDiscovery {
    fn register_service(&mut self, registration: ServiceRegistration) -> Result<(), ServiceError>;
    fn unregister_service(&mut self, serviceId: ServiceId) -> Result<(), ServiceError>;
    fn find_service(&self, service_name: &str, version: Option<ServiceVersion>) -> Option<ServiceRegistration>;
    fn list_services(&self) -> Vec<ServiceRegistration>;
    fn get_service_dependencies(&self, service_id: ServiceId) -> Vec<ServiceId>;
}
```

#### 5.2 配置管理

```rust
// 配置项定义
#[derive(Debug, Clone)]
pub struct ConfigurationItem {
    pub key: String,
    pub value: ConfigurationValue,
    pub value_type: ConfigurationType,
    pub default_value: Option<ConfigurationValue>,
    pub description: String,
    pub is_readonly: bool,
    pub is_sensitive: bool,
}

// 配置值类型
#[derive(Debug, Clone)]
pub enum ConfigurationValue {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Array(Vec<ConfigurationValue>),
    Object(HashMap<String, ConfigurationValue>),
    Binary(Vec<u8>),
}

// 配置管理接口
pub trait ConfigurationManager {
    fn get_configuration(&self, key: &str) -> Option<ConfigurationValue>;
    fn set_configuration(&mut self, key: &str, value: ConfigurationValue) -> Result<(), ConfigurationError>;
    fn list_configurations(&self) -> Vec<ConfigurationItem>;
    fn reload_configuration(&mut self) -> Result<(), ConfigurationError>;
    fn save_configuration(&self) -> Result<(), ConfigurationError>;
    fn validate_configuration(&self, key: &str, value: &ConfigurationValue) -> Result<(), ValidationError>;
}
```

## 接口实现指导原则

### 1. 实现一致性

- 所有接口实现必须遵循相同的命名约定
- 错误处理必须使用统一的错误类型
- 参数验证必须在接口层面进行
- 文档必须包含使用示例

### 2. 性能考虑

- 避免不必要的内存分配
- 使用零拷贝技术传递大数据
- 实现批量操作接口
- 支持异步操作以避免阻塞

### 3. 安全性要求

- 所有输入必须进行验证
- 敏感数据必须进行保护
- 实现访问控制检查
- 支持审计日志记录

### 4. 可测试性设计

- 接口必须支持模拟实现
- 提供测试辅助工具
- 支持依赖注入
- 避免全局状态

## 接口版本管理

### 1. 版本兼容性策略

```rust
// 接口版本
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InterfaceVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl InterfaceVersion {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self { major, minor, patch }
    }
    
    pub fn is_compatible(&self, required: &InterfaceVersion) -> bool {
        self.major == required.major && self.minor >= required.minor
    }
    
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// 版本化接口
pub trait VersionedInterface {
    fn get_interface_version(&self) -> InterfaceVersion;
    fn check_compatibility(&self, required_version: InterfaceVersion) -> bool;
}
```

### 2. 接口演进策略

- 主版本号变化表示不兼容的接口变更
- 次版本号变化表示向后兼容的功能添加
- 修订版本号变化表示向后兼容的错误修复
- 支持多版本接口并存

## 性能优化指导

### 1. 批量操作接口

```rust
// 批量请求
pub struct BatchRequest {
    pub requests: Vec<RequestMessage>,
    pub batch_id: u64,
    pub execution_strategy: BatchExecutionStrategy,
}

// 批量响应
pub struct BatchResponse {
    pub batch_id: u64,
    pub responses: Vec<ResponseMessage>,
    pub execution_statistics: BatchExecutionStatistics,
}

// 批量执行策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchExecutionStrategy {
    Sequential,      // 顺序执行
    Parallel,         // 并行执行
    Optimized,        // 优化执行顺序
    Transactional,    // 事务性执行
}

// 批量操作接口
pub trait BatchOperations {
    fn execute_batch(&mut self, batch: BatchRequest) -> Result<BatchResponse, BatchError>;
    fn cancel_batch(&mut self, batch_id: u64) -> Result<(), BatchError>;
    fn get_batch_status(&self, batch_id: u64) -> Option<BatchStatus>;
}
```

### 2. 缓存和预取

```rust
// 缓存策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStrategy {
    NoCache,
    WriteThrough,
    WriteBack,
    WriteAround,
    ReadThrough,
}

// 缓存接口
pub trait CacheInterface<K, V> {
    fn get(&self, key: &K) -> Option<V>;
    fn put(&mut self, key: K, value: V) -> Option<V>;
    fn remove(&mut self, key: &K) -> Option<V>;
    fn clear(&mut self);
    fn get_statistics(&self) -> CacheStatistics;
}

// 缓存统计
#[derive(Debug, Clone, Default)]
pub struct CacheStatistics {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size: usize,
    pub max_size: usize,
    pub hit_ratio: f32,
}
```

## 验收标准

### 1. 接口完整性
- [ ] 所有核心服务接口已定义
- [ ] 通信协议标准化完成
- [ ] 资源管理接口统一
- [ ] 错误处理机制完善

### 2. 实现质量
- [ ] 接口实现遵循设计原则
- [ ] 性能优化措施到位
- [ ] 安全性要求满足
- [ ] 可测试性设计良好

### 3. 文档和工具
- [ ] 接口文档完整
- [ ] 使用示例丰富
- [ ] 工具支持完善
- [ ] 版本管理机制就绪

## 结论

通过实施这个标准化接口设计，NOS内核将获得：

1. **统一的通信协议**：模块间使用一致的消息格式和通信模式
2. **标准化的错误处理**：统一的错误类型和传播机制
3. **高效的资源管理**：标准化的资源分配和生命周期管理
4. **灵活的配置系统**：支持动态配置和服务发现
5. **良好的性能特性**：批量操作、缓存优化和异步支持

这个接口标准化设计为NOS内核模块间的松耦合通信提供了坚实的基础，支持系统的持续演进和优化。