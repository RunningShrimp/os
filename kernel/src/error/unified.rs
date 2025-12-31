//! Unified Error Handling Module
//!
//! 统一错误处理模块
//! 提供一致的错误处理机制，包括错误类型定义、转换和传播

use alloc::string::{String, ToString};
use alloc::fmt;

/// 统一内核错误类型
///
/// 这个枚举表示所有可能的内核错误
/// 它为所有内核操作提供统一的错误处理机制
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnifiedError {
    /// 基本错误
    InvalidArgument,
    InvalidAddress,
    InvalidInput,
    InvalidData,
    InvalidState,
    InvalidOperation,
    PermissionDenied,
    NotFound,
    NoProcess,
    NoDevice,
    AlreadyExists,
    AlreadyInProgress,
    FileExists,
    ResourceBusy,
    Busy,
    ResourceUnavailable,
    OutOfMemory,
    OutOfSpace,
    QuotaExceeded,
    NotSupported,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    Interrupted,
    TimedOut,
    WouldBlock,
    IoError,
    BadAddress,
    BadFileDescriptor,
    ConnectionAborted,
    ConnectionReset,
    Unknown,

    /// Resource limit errors
    ResourceLimitExceeded { resource: String, usage: u64, limit: u64 },
    InsufficientResources { resource: String, requested: u64, available: u64 },
    IoQuotaExceeded { operation: String, quota: u64, usage: u64 },
    MemoryLimitExceeded { requested: u64, limit: u64 },

    /// 内存相关错误
    MemoryError(MemoryError),

    /// 文件系统相关错误
    FileSystemError(FileSystemError),

    /// 网络相关错误
    NetworkError(NetworkError),

    /// 进程相关错误
    ProcessError(ProcessError),

    /// 系统调用相关错误
    SyscallError(SyscallError),

    /// 驱动程序相关错误
    DriverError(DriverError),

    /// 安全相关错误
    SecurityError(SecurityError),

    /// 机器学习相关错误
    MlError(MlError),

    /// 虚拟化相关错误
    VirtualizationError(VirtualizationError),

    /// 容器相关错误
    ContainerError(ContainerError),

    /// 设备虚拟化相关错误
    DeviceError(DeviceError),

    /// 快照和迁移相关错误
    SnapshotError(SnapshotError),

    /// 集群相关错误
    ClusterError(ClusterError),

    /// 同步原语相关错误
    SyncError(SyncError),

    /// 原子操作相关错误
    AtomicError(AtomicError),

    /// RCU相关错误
    RcuError(RcuError),

    /// 并行执行相关错误
    ParallelError(ParallelError),

    /// 并发管理相关错误
    ConcurrencyError(ConcurrencyError),

    /// 其他错误
    Other(String),
}

impl fmt::Display for UnifiedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnifiedError::InvalidArgument => write!(f, "Invalid argument provided"),
            UnifiedError::InvalidAddress => write!(f, "Invalid memory address"),
            UnifiedError::InvalidInput => write!(f, "Invalid input"),
            UnifiedError::InvalidData => write!(f, "Invalid data"),
            UnifiedError::InvalidState => write!(f, "Invalid state"),
            UnifiedError::InvalidOperation => write!(f, "Invalid operation"),
            UnifiedError::PermissionDenied => write!(f, "Permission denied"),
            UnifiedError::NotFound => write!(f, "Resource not found"),
            UnifiedError::NoProcess => write!(f, "No such process"),
            UnifiedError::NoDevice => write!(f, "No such device"),
            UnifiedError::AlreadyExists => write!(f, "Resource already exists"),
            UnifiedError::AlreadyInProgress => write!(f, "Operation already in progress"),
            UnifiedError::FileExists => write!(f, "File exists"),
            UnifiedError::ResourceBusy => write!(f, "Resource busy"),
            UnifiedError::Busy => write!(f, "Device busy"),
            UnifiedError::ResourceUnavailable => write!(f, "Resource unavailable"),
            UnifiedError::OutOfMemory => write!(f, "Out of memory"),
            UnifiedError::OutOfSpace => write!(f, "No space left on device"),
            UnifiedError::QuotaExceeded => write!(f, "Quota exceeded"),
            UnifiedError::NotSupported => write!(f, "Operation not supported"),
            UnifiedError::NotADirectory => write!(f, "Not a directory"),
            UnifiedError::IsADirectory => write!(f, "Is a directory"),
            UnifiedError::DirectoryNotEmpty => write!(f, "Directory not empty"),
            UnifiedError::Interrupted => write!(f, "Operation interrupted"),
            UnifiedError::TimedOut => write!(f, "Operation timed out"),
            UnifiedError::WouldBlock => write!(f, "Operation would block"),
            UnifiedError::IoError => write!(f, "I/O error"),
            UnifiedError::BadAddress => write!(f, "Bad address"),
            UnifiedError::BadFileDescriptor => write!(f, "Bad file descriptor"),
            UnifiedError::ConnectionAborted => write!(f, "Connection aborted"),
            UnifiedError::ConnectionReset => write!(f, "Connection reset"),
            UnifiedError::Unknown => write!(f, "Unknown error"),
            UnifiedError::ResourceLimitExceeded { resource, usage, limit } => {
                write!(f, "Resource limit exceeded: {} (usage: {}, limit: {})", resource, usage, limit)
            }
            UnifiedError::InsufficientResources { resource, requested, available } => {
                write!(f, "Insufficient resources: {} (requested: {}, available: {})",
                       resource, requested, available)
            }
            UnifiedError::IoQuotaExceeded { operation, quota, usage } => {
                write!(f, "I/O quota exceeded: {} (usage: {}, quota: {})", operation, usage, quota)
            }
            UnifiedError::MemoryLimitExceeded { requested, limit } => {
                write!(f, "Memory limit exceeded (requested: {}, limit: {})", requested, limit)
            }
            UnifiedError::MemoryError(e) => write!(f, "Memory error: {}", e.to_string()),
            UnifiedError::FileSystemError(e) => write!(f, "Filesystem error: {}", e.to_string()),
            UnifiedError::NetworkError(e) => write!(f, "Network error: {}", e.to_string()),
            UnifiedError::ProcessError(e) => write!(f, "Process error: {}", e.to_string()),
            UnifiedError::SyscallError(e) => write!(f, "System call error: {}", e.to_string()),
            UnifiedError::DriverError(e) => write!(f, "Driver error: {}", e.to_string()),
            UnifiedError::SecurityError(e) => write!(f, "Security error: {}", e.to_string()),
            UnifiedError::MlError(e) => write!(f, "ML error: {}", e.to_string()),
            UnifiedError::VirtualizationError(e) => write!(f, "Virtualization error: {}", e.to_string()),
            UnifiedError::ContainerError(e) => write!(f, "Container error: {}", e.to_string()),
            UnifiedError::DeviceError(e) => write!(f, "Device error: {}", e.to_string()),
            UnifiedError::SnapshotError(e) => write!(f, "Snapshot error: {}", e.to_string()),
            UnifiedError::ClusterError(e) => write!(f, "Cluster error: {}", e.to_string()),
            UnifiedError::SyncError(e) => write!(f, "Sync error: {}", e.to_string()),
            UnifiedError::AtomicError(e) => write!(f, "Atomic error: {}", e.to_string()),
            UnifiedError::RcuError(e) => write!(f, "RCU error: {}", e.to_string()),
            UnifiedError::ParallelError(e) => write!(f, "Parallel error: {}", e.to_string()),
            UnifiedError::ConcurrencyError(e) => write!(f, "Concurrency error: {}", e.to_string()),
            UnifiedError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

/// 内存相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryError {
    OutOfMemory,
    InvalidAlignment,
    InvalidSize,
    CorruptedAllocator,
    TooFragmented,
    InvalidAddress,
    InvalidProtection,
    MappingFailed,
    UnmappingFailed,
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::OutOfMemory => write!(f, "Out of memory"),
            MemoryError::InvalidAlignment => write!(f, "Invalid memory alignment"),
            MemoryError::InvalidSize => write!(f, "Invalid memory size"),
            MemoryError::CorruptedAllocator => write!(f, "Corrupted allocator"),
            MemoryError::TooFragmented => write!(f, "Memory too fragmented"),
            MemoryError::InvalidAddress => write!(f, "Invalid memory address"),
            MemoryError::InvalidProtection => write!(f, "Invalid memory protection"),
            MemoryError::MappingFailed => write!(f, "Memory mapping failed"),
            MemoryError::UnmappingFailed => write!(f, "Memory unmapping failed"),
        }
    }
}

/// 文件系统相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSystemError {
    PathNotFound,
    FileNotFound,
    PermissionDenied,
    FileExists,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    InvalidPath,
    PathTooLong,
    FileSystemFull,
    IoError,
    ResourceBusy,
    OperationNotSupported,
    FileSystemCorrupted,
    QuotaExceeded,
}

impl fmt::Display for FileSystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileSystemError::PathNotFound => write!(f, "Path not found"),
            FileSystemError::FileNotFound => write!(f, "File not found"),
            FileSystemError::PermissionDenied => write!(f, "Permission denied"),
            FileSystemError::FileExists => write!(f, "File exists"),
            FileSystemError::NotADirectory => write!(f, "Not a directory"),
            FileSystemError::IsADirectory => write!(f, "Is a directory"),
            FileSystemError::DirectoryNotEmpty => write!(f, "Directory not empty"),
            FileSystemError::InvalidPath => write!(f, "Invalid path"),
            FileSystemError::PathTooLong => write!(f, "Path too long"),
            FileSystemError::FileSystemFull => write!(f, "Filesystem full"),
            FileSystemError::IoError => write!(f, "I/O error"),
            FileSystemError::ResourceBusy => write!(f, "Resource busy"),
            FileSystemError::OperationNotSupported => write!(f, "Operation not supported"),
            FileSystemError::FileSystemCorrupted => write!(f, "Filesystem corrupted"),
            FileSystemError::QuotaExceeded => write!(f, "Quota exceeded"),
        }
    }
}

/// 网络相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkError {
    ConnectionRefused,
    ConnectionReset,
    BrokenPipe,
    TimedOut,
    HostUnreachable,
    NetworkUnreachable,
    AddressInUse,
    NoBufferSpace,
    MessageTooLarge,
    ProtocolError,
    NetworkDown,
    ConnectionAborted,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::ConnectionRefused => write!(f, "Connection refused"),
            NetworkError::ConnectionReset => write!(f, "Connection reset"),
            NetworkError::BrokenPipe => write!(f, "Broken pipe"),
            NetworkError::TimedOut => write!(f, "Connection timed out"),
            NetworkError::HostUnreachable => write!(f, "Host unreachable"),
            NetworkError::NetworkUnreachable => write!(f, "Network unreachable"),
            NetworkError::AddressInUse => write!(f, "Address already in use"),
            NetworkError::NoBufferSpace => write!(f, "No buffer space available"),
            NetworkError::MessageTooLarge => write!(f, "Message too large"),
            NetworkError::ProtocolError => write!(f, "Protocol error"),
            NetworkError::NetworkDown => write!(f, "Network down"),
            NetworkError::ConnectionAborted => write!(f, "Connection aborted"),
        }
    }
}

/// 进程相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessError {
    ProcessNotFound,
    PermissionDenied,
    InvalidArgument,
    ResourceLimitExceeded,
    ProcessAlreadyExists,
    ProcessTerminated,
    ProcessNotRunning,
    ProcessKilled,
    InvalidState,
    StackOverflow,
    HeapCorruption,
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessError::ProcessNotFound => write!(f, "Process not found"),
            ProcessError::PermissionDenied => write!(f, "Permission denied"),
            ProcessError::InvalidArgument => write!(f, "Invalid argument"),
            ProcessError::ResourceLimitExceeded => write!(f, "Resource limit exceeded"),
            ProcessError::ProcessAlreadyExists => write!(f, "Process already exists"),
            ProcessError::ProcessTerminated => write!(f, "Process terminated"),
            ProcessError::ProcessNotRunning => write!(f, "Process not running"),
            ProcessError::ProcessKilled => write!(f, "Process killed"),
            ProcessError::InvalidState => write!(f, "Invalid process state"),
            ProcessError::StackOverflow => write!(f, "Stack overflow"),
            ProcessError::HeapCorruption => write!(f, "Heap corruption"),
        }
    }
}

/// 系统调用相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyscallError {
    InvalidSyscall,
    PermissionDenied,
    InvalidArgument,
    NotFound,
    OutOfMemory,
    Interrupted,
    IoError,
    WouldBlock,
    NotSupported,
    NotImplemented,
    BadFileDescriptor,
    TooManyOpenFiles,
    NoBufferSpace,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    FileExists,
    CrossDeviceLink,
    FileTooBig,
    NoSpaceLeft,
    BadAddress,
    DeadlockWouldOccur,
    NameTooLong,
    TooManySymlinks,
    ConnectionRefused,
    ConnectionReset,
    BrokenPipe,
    TimedOut,
    NoProcess,
    OperationNotPermitted,
}

impl fmt::Display for SyscallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyscallError::InvalidSyscall => write!(f, "Invalid system call"),
            SyscallError::PermissionDenied => write!(f, "Permission denied"),
            SyscallError::InvalidArgument => write!(f, "Invalid argument"),
            SyscallError::NotFound => write!(f, "Not found"),
            SyscallError::OutOfMemory => write!(f, "Out of memory"),
            SyscallError::Interrupted => write!(f, "Interrupted system call"),
            SyscallError::IoError => write!(f, "I/O error"),
            SyscallError::WouldBlock => write!(f, "Operation would block"),
            SyscallError::NotSupported => write!(f, "Operation not supported"),
            SyscallError::NotImplemented => write!(f, "Not implemented"),
            SyscallError::BadFileDescriptor => write!(f, "Bad file descriptor"),
            SyscallError::TooManyOpenFiles => write!(f, "Too many open files"),
            SyscallError::NoBufferSpace => write!(f, "No buffer space available"),
            SyscallError::NotADirectory => write!(f, "Not a directory"),
            SyscallError::IsADirectory => write!(f, "Is a directory"),
            SyscallError::DirectoryNotEmpty => write!(f, "Directory not empty"),
            SyscallError::FileExists => write!(f, "File exists"),
            SyscallError::CrossDeviceLink => write!(f, "Cross-device link"),
            SyscallError::FileTooBig => write!(f, "File too large"),
            SyscallError::NoSpaceLeft => write!(f, "No space left on device"),
            SyscallError::BadAddress => write!(f, "Bad address"),
            SyscallError::DeadlockWouldOccur => write!(f, "Deadlock would occur"),
            SyscallError::NameTooLong => write!(f, "Name too long"),
            SyscallError::TooManySymlinks => write!(f, "Too many symbolic links"),
            SyscallError::ConnectionRefused => write!(f, "Connection refused"),
            SyscallError::ConnectionReset => write!(f, "Connection reset"),
            SyscallError::BrokenPipe => write!(f, "Broken pipe"),
            SyscallError::TimedOut => write!(f, "Operation timed out"),
            SyscallError::NoProcess => write!(f, "No such process"),
            SyscallError::OperationNotPermitted => write!(f, "Operation not permitted"),
        }
    }
}

/// 驱动程序相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverError {
    DeviceNotFound,
    DeviceBusy,
    DeviceNotConnected,
    UnsupportedOperation,
    HardwareFailure,
    InvalidConfiguration,
    ResourceConflict,
    Timeout,
    FirmwareMissing,
    DriverNotLoaded,
    InvalidParameter,
    DeviceManager(String),
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverError::DeviceNotFound => write!(f, "Device not found"),
            DriverError::DeviceBusy => write!(f, "Device busy"),
            DriverError::DeviceNotConnected => write!(f, "Device not connected"),
            DriverError::UnsupportedOperation => write!(f, "Unsupported operation"),
            DriverError::HardwareFailure => write!(f, "Hardware failure"),
            DriverError::InvalidConfiguration => write!(f, "Invalid configuration"),
            DriverError::ResourceConflict => write!(f, "Resource conflict"),
            DriverError::Timeout => write!(f, "Timeout"),
            DriverError::FirmwareMissing => write!(f, "Firmware missing"),
            DriverError::DriverNotLoaded => write!(f, "Driver not loaded"),
            DriverError::InvalidParameter => write!(f, "Invalid parameter"),
            DriverError::DeviceManager(msg) => write!(f, "Device manager error: {}", msg),
        }
    }
}

/// 安全相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    AccessDenied,
    PermissionDenied,
    AuthenticationFailed,
    AuthorizationFailed,
    SecurityPolicyViolation,
    InvalidCredentials,
    AccountLocked,
    PasswordExpired,
    AccountDisabled,
    InsufficientPrivileges,
    SecurityBreach,
    AttestationFailed(String),
}

impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityError::AccessDenied => write!(f, "Access denied"),
            SecurityError::PermissionDenied => write!(f, "Permission denied"),
            SecurityError::AuthenticationFailed => write!(f, "Authentication failed"),
            SecurityError::AuthorizationFailed => write!(f, "Authorization failed"),
            SecurityError::SecurityPolicyViolation => write!(f, "Security policy violation"),
            SecurityError::InvalidCredentials => write!(f, "Invalid credentials"),
            SecurityError::AccountLocked => write!(f, "Account locked"),
            SecurityError::PasswordExpired => write!(f, "Password expired"),
            SecurityError::AccountDisabled => write!(f, "Account disabled"),
            SecurityError::InsufficientPrivileges => write!(f, "Insufficient privileges"),
            SecurityError::SecurityBreach => write!(f, "Security breach"),
            SecurityError::AttestationFailed(msg) => write!(f, "Attestation failed: {}", msg),
        }
    }
}

/// 云原生编排相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchError {
    DeploymentFailed,
    UpdateFailed,
    RollbackFailed,
    ScalingFailed,
    ServiceNotFound,
    InvalidServiceSpec,
    InvalidDeploymentConfig,
    HealthCheckFailed,
    ResourceLimitExceeded,
    DeploymentTimeout,
}

impl fmt::Display for OrchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrchError::DeploymentFailed => write!(f, "Service deployment failed"),
            OrchError::UpdateFailed => write!(f, "Service update failed"),
            OrchError::RollbackFailed => write!(f, "Service rollback failed"),
            OrchError::ScalingFailed => write!(f, "Service scaling failed"),
            OrchError::ServiceNotFound => write!(f, "Service not found"),
            OrchError::InvalidServiceSpec => write!(f, "Invalid service specification"),
            OrchError::InvalidDeploymentConfig => write!(f, "Invalid deployment configuration"),
            OrchError::HealthCheckFailed => write!(f, "Health check failed"),
            OrchError::ResourceLimitExceeded => write!(f, "Resource limit exceeded"),
            OrchError::DeploymentTimeout => write!(f, "Deployment timeout"),
        }
    }
}

/// 服务网格相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshError {
    MeshNotFound,
    SidecarInjectionFailed,
    MtlsConfigurationFailed,
    CertificateError,
    TrafficPolicyError,
    CircuitBreakerError,
    ServiceNotInMesh,
    InvalidMeshConfig,
    ProxyError,
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MeshError::MeshNotFound => write!(f, "Service mesh not found"),
            MeshError::SidecarInjectionFailed => write!(f, "Sidecar injection failed"),
            MeshError::MtlsConfigurationFailed => write!(f, "mTLS configuration failed"),
            MeshError::CertificateError => write!(f, "Certificate error"),
            MeshError::TrafficPolicyError => write!(f, "Traffic policy error"),
            MeshError::CircuitBreakerError => write!(f, "Circuit breaker error"),
            MeshError::ServiceNotInMesh => write!(f, "Service not in mesh"),
            MeshError::InvalidMeshConfig => write!(f, "Invalid mesh configuration"),
            MeshError::ProxyError => write!(f, "Proxy error"),
        }
    }
}

/// API网关相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayError {
    RouteNotFound,
    InvalidRouteConfig,
    RateLimitExceeded,
    AuthenticationFailed,
    TransformationFailed,
    Timeout,
    RouteConflict,
    InvalidDestination,
}

impl fmt::Display for GatewayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GatewayError::RouteNotFound => write!(f, "Route not found"),
            GatewayError::InvalidRouteConfig => write!(f, "Invalid route configuration"),
            GatewayError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            GatewayError::AuthenticationFailed => write!(f, "Authentication failed"),
            GatewayError::TransformationFailed => write!(f, "Transformation failed"),
            GatewayError::Timeout => write!(f, "Request timeout"),
            GatewayError::RouteConflict => write!(f, "Route conflict"),
            GatewayError::InvalidDestination => write!(f, "Invalid destination"),
        }
    }
}

/// 配置管理相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    ConfigNotFound,
    InvalidConfigValue,
    ValidationFailed,
    VersionConflict,
    WatchFailed,
    RolloutFailed,
    DriftDetected,
    LockAcquisitionFailed,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::ConfigNotFound => write!(f, "Configuration not found"),
            ConfigError::InvalidConfigValue => write!(f, "Invalid configuration value"),
            ConfigError::ValidationFailed => write!(f, "Configuration validation failed"),
            ConfigError::VersionConflict => write!(f, "Configuration version conflict"),
            ConfigError::WatchFailed => write!(f, "Configuration watch failed"),
            ConfigError::RolloutFailed => write!(f, "Configuration rollout failed"),
            ConfigError::DriftDetected => write!(f, "Configuration drift detected"),
            ConfigError::LockAcquisitionFailed => write!(f, "Lock acquisition failed"),
        }
    }
}

/// 密钥管理相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretError {
    SecretNotFound,
    EncryptionFailed,
    DecryptionFailed,
    RotationFailed,
    InjectionFailed,
    HsmError,
    CertificateError,
    InvalidSecretData,
    AccessDenied,
}

impl fmt::Display for SecretError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecretError::SecretNotFound => write!(f, "Secret not found"),
            SecretError::EncryptionFailed => write!(f, "Secret encryption failed"),
            SecretError::DecryptionFailed => write!(f, "Secret decryption failed"),
            SecretError::RotationFailed => write!(f, "Secret rotation failed"),
            SecretError::InjectionFailed => write!(f, "Secret injection failed"),
            SecretError::HsmError => write!(f, "HSM error"),
            SecretError::CertificateError => write!(f, "Certificate error"),
            SecretError::InvalidSecretData => write!(f, "Invalid secret data"),
            SecretError::AccessDenied => write!(f, "Access denied"),
        }
    }
}

/// 可观测性桥接相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeError {
    ExportFailed,
    InvalidSpan,
    InvalidMetric,
    InvalidLogEntry,
    ConnectionFailed,
    SerializationFailed,
    SamplingFailed,
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BridgeError::ExportFailed => write!(f, "Observability export failed"),
            BridgeError::InvalidSpan => write!(f, "Invalid span"),
            BridgeError::InvalidMetric => write!(f, "Invalid metric"),
            BridgeError::InvalidLogEntry => write!(f, "Invalid log entry"),
            BridgeError::ConnectionFailed => write!(f, "Connection failed"),
            BridgeError::SerializationFailed => write!(f, "Serialization failed"),
            BridgeError::SamplingFailed => write!(f, "Sampling failed"),
        }
    }
}

/// 机器学习相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MlError {
    /// 推理错误
    InferenceError(String),
    ModelLoadFailed(String),
    ModelNotFound,
    InvalidModelFormat,
    ModelExecutionFailed(String),
    TensorShapeMismatch,
    InvalidTensorData,
    OutOfMemory,
    UnsupportedOperation(String),

    /// 加速器错误
    AcceleratorNotFound,
    AcceleratorUnavailable,
    AcceleratorInitializationFailed(String),
    DmaError(String),
    MemoryAllocationFailed,
    KernelLaunchFailed(String),
    MultiGpuError(String),

    /// 神经网络错误
    LayerCreationFailed(String),
    ForwardPassFailed(String),
    BackwardPassFailed(String),
    InvalidLayerConfiguration,
    GradientComputationFailed,
    ActivationError(String),

    /// 优化器错误
    OptimizerError(String),
    InvalidLearningRate,
    InvalidOptimizerConfig,
    GradientClippingFailed,
    WeightDecayError(String),
    LrScheduleError(String),

    /// 数据管道错误
    PipelineError(String),
    DataLoadFailed(String),
    PreprocessingFailed(String),
    AugmentationFailed(String),
    BatchCreationFailed(String),
    InvalidDataFormat,
}

impl fmt::Display for MlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MlError::InferenceError(msg) => write!(f, "Inference error: {}", msg),
            MlError::ModelLoadFailed(msg) => write!(f, "Model load failed: {}", msg),
            MlError::ModelNotFound => write!(f, "Model not found"),
            MlError::InvalidModelFormat => write!(f, "Invalid model format"),
            MlError::ModelExecutionFailed(msg) => write!(f, "Model execution failed: {}", msg),
            MlError::TensorShapeMismatch => write!(f, "Tensor shape mismatch"),
            MlError::InvalidTensorData => write!(f, "Invalid tensor data"),
            MlError::OutOfMemory => write!(f, "ML out of memory"),
            MlError::UnsupportedOperation(msg) => write!(f, "Unsupported operation: {}", msg),
            MlError::AcceleratorNotFound => write!(f, "Accelerator not found"),
            MlError::AcceleratorUnavailable => write!(f, "Accelerator unavailable"),
            MlError::AcceleratorInitializationFailed(msg) => write!(f, "Accelerator init failed: {}", msg),
            MlError::DmaError(msg) => write!(f, "DMA error: {}", msg),
            MlError::MemoryAllocationFailed => write!(f, "Memory allocation failed"),
            MlError::KernelLaunchFailed(msg) => write!(f, "Kernel launch failed: {}", msg),
            MlError::MultiGpuError(msg) => write!(f, "Multi-GPU error: {}", msg),
            MlError::LayerCreationFailed(msg) => write!(f, "Layer creation failed: {}", msg),
            MlError::ForwardPassFailed(msg) => write!(f, "Forward pass failed: {}", msg),
            MlError::BackwardPassFailed(msg) => write!(f, "Backward pass failed: {}", msg),
            MlError::InvalidLayerConfiguration => write!(f, "Invalid layer configuration"),
            MlError::GradientComputationFailed => write!(f, "Gradient computation failed"),
            MlError::ActivationError(msg) => write!(f, "Activation error: {}", msg),
            MlError::OptimizerError(msg) => write!(f, "Optimizer error: {}", msg),
            MlError::InvalidLearningRate => write!(f, "Invalid learning rate"),
            MlError::InvalidOptimizerConfig => write!(f, "Invalid optimizer config"),
            MlError::GradientClippingFailed => write!(f, "Gradient clipping failed"),
            MlError::WeightDecayError(msg) => write!(f, "Weight decay error: {}", msg),
            MlError::LrScheduleError(msg) => write!(f, "LR schedule error: {}", msg),
            MlError::PipelineError(msg) => write!(f, "Pipeline error: {}", msg),
            MlError::DataLoadFailed(msg) => write!(f, "Data load failed: {}", msg),
            MlError::PreprocessingFailed(msg) => write!(f, "Preprocessing failed: {}", msg),
            MlError::AugmentationFailed(msg) => write!(f, "Augmentation failed: {}", msg),
            MlError::BatchCreationFailed(msg) => write!(f, "Batch creation failed: {}", msg),
            MlError::InvalidDataFormat => write!(f, "Invalid data format"),
        }
    }
}

impl From<SecurityError> for UnifiedError {
    fn from(err: SecurityError) -> Self {
        UnifiedError::SecurityError(err)
    }
}

impl From<MlError> for UnifiedError {
    fn from(err: MlError) -> Self {
        UnifiedError::MlError(err)
    }
}

impl MlError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            MlError::OutOfMemory => crate::reliability::errno::ENOMEM,
            MlError::ModelNotFound | MlError::AcceleratorNotFound => crate::reliability::errno::ENOENT,
            MlError::InvalidModelFormat | MlError::InvalidTensorData | MlError::InvalidDataFormat => {
                crate::reliability::errno::EINVAL
            },
            MlError::TensorShapeMismatch => crate::reliability::errno::EINVAL,
            MlError::AcceleratorUnavailable => crate::reliability::errno::EAGAIN,
            _ => crate::reliability::errno::EIO,
        }
    }
}

/// 错误严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
    Fatal,
}

/// 错误上下文
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// 错误类型
    pub error: UnifiedError,
    /// 错误严重级别
    pub severity: ErrorSeverity,
    /// 错误发生位置
    pub location: String,
    /// 错误发生时间
    pub timestamp: u64,
    /// 错误描述
    pub description: String,
    /// 错误原因
    pub cause: Option<String>,
    /// 错误恢复建议
    pub recovery_hint: Option<String>,
}

impl ErrorContext {
    /// 创建新的错误上下文
    pub fn new(error: UnifiedError, location: &str) -> Self {
        let description = error.default_description();
        let recovery_hint = error.default_recovery_hint();
        Self {
            severity: error.default_severity(),
            error,
            location: location.to_string(),
            timestamp: get_timestamp(),
            description,
            cause: None,
            recovery_hint,
        }
    }

    /// 设置错误严重级别
    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// 设置错误描述
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// 设置错误原因
    pub fn with_cause(mut self, cause: String) -> Self {
        self.cause = Some(cause);
        self
    }

    /// 设置错误恢复建议
    pub fn with_recovery_hint(mut self, hint: String) -> Self {
        self.recovery_hint = Some(hint);
        self
    }
}

impl UnifiedError {
    /// 获取错误的默认严重级别
    pub fn default_severity(&self) -> ErrorSeverity {
        match self {
            UnifiedError::InvalidArgument | UnifiedError::InvalidAddress | UnifiedError::InvalidInput | UnifiedError::InvalidData | UnifiedError::InvalidOperation => ErrorSeverity::Warning,
            UnifiedError::PermissionDenied | UnifiedError::SecurityError(_) => ErrorSeverity::Error,
            UnifiedError::OutOfMemory | UnifiedError::MemoryError(MemoryError::OutOfMemory) => {
                ErrorSeverity::Critical
            },
            UnifiedError::ResourceBusy | UnifiedError::ResourceUnavailable | UnifiedError::Busy => {
                ErrorSeverity::Warning
            },
            UnifiedError::NotFound | UnifiedError::AlreadyExists | UnifiedError::NoProcess | UnifiedError::NoDevice | UnifiedError::FileExists => ErrorSeverity::Info,
            UnifiedError::MemoryError(_)
            | UnifiedError::FileSystemError(_)
            | UnifiedError::NetworkError(_)
            | UnifiedError::ProcessError(_)
            | UnifiedError::SyscallError(_)
            | UnifiedError::DriverError(_)
            | UnifiedError::MlError(_)
            | UnifiedError::VirtualizationError(_)
            | UnifiedError::ContainerError(_)
            | UnifiedError::DeviceError(_)
            | UnifiedError::SnapshotError(_)
            | UnifiedError::ClusterError(_) => ErrorSeverity::Error,
            UnifiedError::Other(_) => ErrorSeverity::Warning,
            UnifiedError::InvalidState => ErrorSeverity::Error,
            UnifiedError::IoError => ErrorSeverity::Error,
            UnifiedError::BadAddress => ErrorSeverity::Error,
            UnifiedError::BadFileDescriptor => ErrorSeverity::Error,
            UnifiedError::ConnectionAborted => ErrorSeverity::Error,
            UnifiedError::ConnectionReset => ErrorSeverity::Error,
            UnifiedError::Interrupted => ErrorSeverity::Warning,
            UnifiedError::TimedOut => ErrorSeverity::Error,
            UnifiedError::WouldBlock => ErrorSeverity::Warning,
            UnifiedError::QuotaExceeded => ErrorSeverity::Error,
            UnifiedError::NotSupported => ErrorSeverity::Warning,
            UnifiedError::NotADirectory => ErrorSeverity::Warning,
            UnifiedError::IsADirectory => ErrorSeverity::Warning,
            UnifiedError::DirectoryNotEmpty => ErrorSeverity::Warning,
            UnifiedError::AlreadyInProgress => ErrorSeverity::Warning,
            UnifiedError::OutOfSpace => ErrorSeverity::Error,
            UnifiedError::Unknown => ErrorSeverity::Error,
            UnifiedError::MemoryLimitExceeded { .. } => ErrorSeverity::Critical,
            UnifiedError::ResourceLimitExceeded { .. } => ErrorSeverity::Error,
            UnifiedError::InsufficientResources { .. } => ErrorSeverity::Critical,
            UnifiedError::IoQuotaExceeded { .. } => ErrorSeverity::Error,
            UnifiedError::SyncError(_) => ErrorSeverity::Error,
            UnifiedError::AtomicError(_) => ErrorSeverity::Error,
            UnifiedError::RcuError(_) => ErrorSeverity::Error,
            UnifiedError::ParallelError(_) => ErrorSeverity::Error,
            UnifiedError::ConcurrencyError(_) => ErrorSeverity::Error,
        }
    }

    /// 获取错误的默认描述
    pub fn default_description(&self) -> String {
        match self {
            UnifiedError::InvalidArgument => "Invalid argument provided".to_string(),
            UnifiedError::InvalidAddress => "Invalid memory address".to_string(),
            UnifiedError::InvalidInput => "Invalid input".to_string(),
            UnifiedError::InvalidData => "Invalid data".to_string(),
            UnifiedError::InvalidState => "Invalid state".to_string(),
            UnifiedError::InvalidOperation => "Invalid operation".to_string(),
            UnifiedError::PermissionDenied => "Permission denied".to_string(),
            UnifiedError::NotFound => "Resource not found".to_string(),
            UnifiedError::NoProcess => "No such process".to_string(),
            UnifiedError::NoDevice => "No such device".to_string(),
            UnifiedError::AlreadyExists => "Resource already exists".to_string(),
            UnifiedError::FileExists => "File exists".to_string(),
            UnifiedError::ResourceBusy => "Resource is busy".to_string(),
            UnifiedError::ResourceUnavailable => "Resource is unavailable".to_string(),
            UnifiedError::Busy => "Device busy".to_string(),
            UnifiedError::OutOfMemory => "Out of memory".to_string(),
            UnifiedError::OutOfSpace => "No space left on device".to_string(),
            UnifiedError::QuotaExceeded => "Quota exceeded".to_string(),
            UnifiedError::NotSupported => "Operation not supported".to_string(),
            UnifiedError::NotADirectory => "Not a directory".to_string(),
            UnifiedError::IsADirectory => "Is a directory".to_string(),
            UnifiedError::DirectoryNotEmpty => "Directory not empty".to_string(),
            UnifiedError::Interrupted => "Operation interrupted".to_string(),
            UnifiedError::TimedOut => "Operation timed out".to_string(),
            UnifiedError::WouldBlock => "Operation would block".to_string(),
            UnifiedError::IoError => "I/O error".to_string(),
            UnifiedError::BadAddress => "Bad address".to_string(),
            UnifiedError::BadFileDescriptor => "Bad file descriptor".to_string(),
            UnifiedError::ConnectionAborted => "Connection aborted".to_string(),
            UnifiedError::ConnectionReset => "Connection reset".to_string(),
            UnifiedError::AlreadyInProgress => "Operation already in progress".to_string(),
            UnifiedError::Unknown => "Unknown error".to_string(),
            UnifiedError::MemoryError(err) => format!("Memory error: {:?}", err),
            UnifiedError::FileSystemError(err) => format!("File system error: {:?}", err),
            UnifiedError::NetworkError(err) => format!("Network error: {:?}", err),
            UnifiedError::ProcessError(err) => format!("Process error: {:?}", err),
            UnifiedError::SyscallError(err) => format!("System call error: {:?}", err),
            UnifiedError::DriverError(err) => format!("Driver error: {:?}", err),
            UnifiedError::MlError(err) => format!("ML error: {:?}", err),
            UnifiedError::VirtualizationError(err) => format!("Virtualization error: {:?}", err),
            UnifiedError::ContainerError(err) => format!("Container error: {:?}", err),
            UnifiedError::DeviceError(err) => format!("Device error: {:?}", err),
            UnifiedError::SnapshotError(err) => format!("Snapshot error: {:?}", err),
            UnifiedError::ClusterError(err) => format!("Cluster error: {:?}", err),
            UnifiedError::SecurityError(err) => format!("Security error: {:?}", err),
            UnifiedError::Other(msg) => format!("Other error: {}", msg),
            UnifiedError::ResourceLimitExceeded { resource, usage, limit } => {
                format!("Resource limit exceeded: {} (usage: {}, limit: {})", resource, usage, limit)
            }
            UnifiedError::InsufficientResources { resource, requested, available } => {
                format!("Insufficient resources: {} (requested: {}, available: {})", resource, requested, available)
            }
            UnifiedError::IoQuotaExceeded { operation, quota, usage } => {
                format!("I/O quota exceeded: {} (quota: {}, usage: {})", operation, quota, usage)
            }
            UnifiedError::MemoryLimitExceeded { requested, limit } => {
                format!("Memory limit exceeded: requested {}, limit {}", requested, limit)
            }
            UnifiedError::SyncError(err) => format!("Sync error: {:?}", err),
            UnifiedError::AtomicError(err) => format!("Atomic error: {:?}", err),
            UnifiedError::RcuError(err) => format!("RCU error: {:?}", err),
            UnifiedError::ParallelError(err) => format!("Parallel error: {:?}", err),
            UnifiedError::ConcurrencyError(err) => format!("Concurrency error: {:?}", err),
        }
    }

    /// 获取错误的默认恢复建议
    pub fn default_recovery_hint(&self) -> Option<String> {
        match self {
            UnifiedError::InvalidArgument => Some("Check the arguments and try again".to_string()),
            UnifiedError::InvalidAddress => {
                Some("Check the memory address and try again".to_string())
            },
            UnifiedError::InvalidInput => Some("Check the input and try again".to_string()),
            UnifiedError::InvalidData => Some("Check the data and try again".to_string()),
            UnifiedError::InvalidState => Some("Check the system state and try again".to_string()),
            UnifiedError::InvalidOperation => Some("Check the operation and try again".to_string()),
            UnifiedError::PermissionDenied => Some("Check permissions and try again".to_string()),
            UnifiedError::NotFound => {
                Some("Check if the resource exists and try again".to_string())
            },
            UnifiedError::NoProcess => {
                Some("Check if the process exists and try again".to_string())
            },
            UnifiedError::NoDevice => {
                Some("Check if the device is connected and try again".to_string())
            },
            UnifiedError::AlreadyExists => {
                Some("Use a different name or delete the existing resource".to_string())
            },
            UnifiedError::FileExists => {
                Some("Use a different name or delete the existing file".to_string())
            },
            UnifiedError::ResourceBusy => {
                Some("Wait for the resource to become available and try again".to_string())
            },
            UnifiedError::ResourceUnavailable => {
                Some("Check if the resource is available and try again".to_string())
            },
            UnifiedError::Busy => {
                Some("Wait for the device to become available and try again".to_string())
            },
            UnifiedError::OutOfMemory => Some("Free up memory and try again".to_string()),
            UnifiedError::OutOfSpace => Some("Free up disk space and try again".to_string()),
            UnifiedError::QuotaExceeded => Some("Increase quota or free up resources and try again".to_string()),
            UnifiedError::NotSupported => Some("Check if the operation is supported and try again".to_string()),
            UnifiedError::NotADirectory => Some("Check the path and try again".to_string()),
            UnifiedError::IsADirectory => Some("Check the path and try again".to_string()),
            UnifiedError::DirectoryNotEmpty => Some("Remove files from the directory and try again".to_string()),
            UnifiedError::Interrupted => Some("Try the operation again".to_string()),
            UnifiedError::TimedOut => Some("Increase timeout or check system resources and try again".to_string()),
            UnifiedError::WouldBlock => Some("Try the operation again when the resource is available".to_string()),
            UnifiedError::IoError => Some("Check hardware connections and try again".to_string()),
            UnifiedError::BadAddress => Some("Check the memory address and try again".to_string()),
            UnifiedError::BadFileDescriptor => Some("Check the file descriptor and try again".to_string()),
            UnifiedError::ConnectionAborted => Some("Check the connection and try again".to_string()),
            UnifiedError::ConnectionReset => Some("Check the connection and try again".to_string()),
            UnifiedError::Unknown => Some("Check system logs and try again".to_string()),
            UnifiedError::MemoryError(MemoryError::OutOfMemory) => {
                Some("Free up memory and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::InvalidAlignment) => {
                Some("Check memory alignment and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::InvalidSize) => {
                Some("Check memory size and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::CorruptedAllocator) => {
                Some("Restart the system and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::TooFragmented) => {
                Some("Reboot the system and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::InvalidAddress) => {
                Some("Check memory address and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::InvalidProtection) => {
                Some("Check memory protection and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::MappingFailed) => {
                Some("Check memory mapping and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::UnmappingFailed) => {
                Some("Check memory unmapping and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::PathNotFound) => {
                Some("Check the path and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::FileNotFound) => {
                Some("Check the file path and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::PermissionDenied) => {
                Some("Check file permissions and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::FileExists) => {
                Some("Use a different name or delete the existing file".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::NotADirectory) => {
                Some("Check the path and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::IsADirectory) => {
                Some("Check the path and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::DirectoryNotEmpty) => {
                Some("Remove files from the directory and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::InvalidPath) => {
                Some("Check the path and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::PathTooLong) => {
                Some("Use a shorter path and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::FileSystemFull) => {
                Some("Free up disk space and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::IoError) => {
                Some("Check storage device and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::ResourceBusy) => {
                Some("Wait for the resource to become available and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::OperationNotSupported) => {
                Some("Check if the operation is supported and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::FileSystemCorrupted) => {
                Some("Check filesystem integrity and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::QuotaExceeded) => {
                Some("Increase quota or free up resources and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::ConnectionRefused) => {
                Some("Check if the service is running and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::ConnectionReset) => {
                Some("Check the connection and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::BrokenPipe) => {
                Some("Check the pipe and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::TimedOut) => {
                Some("Check network connectivity and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::HostUnreachable) => {
                Some("Check network connectivity and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::NetworkUnreachable) => {
                Some("Check network connectivity and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::AddressInUse) => {
                Some("Use a different address and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::NoBufferSpace) => {
                Some("Free up buffers and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::MessageTooLarge) => {
                Some("Reduce message size and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::ProtocolError) => {
                Some("Check protocol configuration and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::NetworkDown) => {
                Some("Check network connectivity and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::ConnectionAborted) => {
                Some("Check the connection and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::ProcessNotFound) => {
                Some("Check if the process exists and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::PermissionDenied) => {
                Some("Check process permissions and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::InvalidArgument) => {
                Some("Check arguments and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::ResourceLimitExceeded) => {
                Some("Increase resource limits and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::ProcessAlreadyExists) => {
                Some("Use a different process name and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::ProcessTerminated) => {
                Some("Restart the process and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::ProcessNotRunning) => {
                Some("Start the process and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::InvalidState) => {
                Some("Check process state and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::StackOverflow) => {
                Some("Increase stack size and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::HeapCorruption) => {
                Some("Restart the process and try again".to_string())
            },
            UnifiedError::SyscallError(SyscallError::PermissionDenied) => {
                Some("Check permissions and try again".to_string())
            },
            UnifiedError::SyscallError(SyscallError::InvalidArgument) => {
                Some("Check arguments and try again".to_string())
            },
            UnifiedError::SyscallError(SyscallError::NotFound) => {
                Some("Check if the resource exists and try again".to_string())
            },
            UnifiedError::SyscallError(SyscallError::NoProcess) => {
                Some("Check if the process exists and try again".to_string())
            },
            UnifiedError::SyscallError(SyscallError::OperationNotPermitted) => {
                Some("Check permissions and try again".to_string())
            },
            UnifiedError::SyscallError(SyscallError::NotImplemented) => {
                Some("Check if the operation is implemented and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::DeviceNotFound) => {
                Some("Check if the device is connected and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::DeviceBusy) => {
                Some("Wait for the device to become available and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::DeviceNotConnected) => {
                Some("Check if the device is connected and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::UnsupportedOperation) => {
                Some("Check if the operation is supported and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::HardwareFailure) => {
                Some("Check hardware and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::InvalidConfiguration) => {
                Some("Check driver configuration and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::ResourceConflict) => {
                Some("Resolve resource conflicts and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::Timeout) => {
                Some("Increase timeout and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::FirmwareMissing) => {
                Some("Install firmware and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::DriverNotLoaded) => {
                Some("Load the driver and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::InvalidParameter) => {
                Some("Check parameters and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::AccessDenied) => {
                Some("Check access permissions and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::PermissionDenied) => {
                Some("Check permissions and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::AuthenticationFailed) => {
                Some("Check credentials and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::AuthorizationFailed) => {
                Some("Check authorization and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::SecurityPolicyViolation) => {
                Some("Check security policy and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::InvalidCredentials) => {
                Some("Check credentials and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::AccountLocked) => {
                Some("Contact administrator to unlock account".to_string())
            },
            UnifiedError::SecurityError(SecurityError::PasswordExpired) => {
                Some("Change password and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::AccountDisabled) => {
                Some("Contact administrator to enable account".to_string())
            },
            UnifiedError::SecurityError(SecurityError::InsufficientPrivileges) => {
                Some("Use higher privileges and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::SecurityBreach) => {
                Some("Check security logs and try again".to_string())
            },
            UnifiedError::MemoryLimitExceeded { .. } => {
                Some("Free up memory or increase limits and try again".to_string())
            },
            UnifiedError::ResourceLimitExceeded { .. } => {
                Some("Free up resources or increase limits and try again".to_string())
            },
            UnifiedError::InsufficientResources { .. } => {
                Some("Free up resources and try again".to_string())
            },
            UnifiedError::IoQuotaExceeded { .. } => {
                Some("Reduce I/O or increase quota and try again".to_string())
            },
            _ => None,
        }
    }

    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            UnifiedError::InvalidArgument => crate::reliability::errno::EINVAL,
            UnifiedError::InvalidAddress => crate::reliability::errno::EFAULT,
            UnifiedError::InvalidInput => crate::reliability::errno::EINVAL,
            UnifiedError::InvalidData => crate::reliability::errno::EINVAL,
            UnifiedError::InvalidState => crate::reliability::errno::EINVAL,
            UnifiedError::InvalidOperation => crate::reliability::errno::EINVAL,
            UnifiedError::PermissionDenied => crate::reliability::errno::EPERM,
            UnifiedError::NotFound => crate::reliability::errno::ENOENT,
            UnifiedError::NoProcess => crate::reliability::errno::ESRCH,
            UnifiedError::NoDevice => crate::reliability::errno::ENODEV,
            UnifiedError::AlreadyExists => crate::reliability::errno::EEXIST,
            UnifiedError::FileExists => crate::reliability::errno::EEXIST,
            UnifiedError::ResourceBusy => crate::reliability::errno::EBUSY,
            UnifiedError::ResourceUnavailable => crate::reliability::errno::EAGAIN,
            UnifiedError::Busy => crate::reliability::errno::EBUSY,
            UnifiedError::OutOfMemory => crate::reliability::errno::ENOMEM,
            UnifiedError::OutOfSpace => crate::reliability::errno::ENOSPC,
            UnifiedError::QuotaExceeded => crate::reliability::errno::EDQUOT,
            UnifiedError::NotSupported => crate::reliability::errno::EOPNOTSUPP,
            UnifiedError::NotADirectory => crate::reliability::errno::ENOTDIR,
            UnifiedError::IsADirectory => crate::reliability::errno::EISDIR,
            UnifiedError::DirectoryNotEmpty => crate::reliability::errno::ENOTEMPTY,
            UnifiedError::Interrupted => crate::reliability::errno::EINTR,
            UnifiedError::TimedOut => crate::reliability::errno::ETIMEDOUT,
            UnifiedError::WouldBlock => crate::reliability::errno::EAGAIN,
            UnifiedError::IoError => crate::reliability::errno::EIO,
            UnifiedError::BadAddress => crate::reliability::errno::EFAULT,
            UnifiedError::BadFileDescriptor => crate::reliability::errno::EBADF,
            UnifiedError::ConnectionAborted => crate::reliability::errno::ECONNABORTED,
            UnifiedError::ConnectionReset => crate::reliability::errno::ECONNRESET,
            UnifiedError::AlreadyInProgress => crate::reliability::errno::EINPROGRESS,
            UnifiedError::Unknown => crate::reliability::errno::EIO,
            UnifiedError::MemoryError(err) => err.to_errno(),
            UnifiedError::FileSystemError(err) => err.to_errno(),
            UnifiedError::NetworkError(err) => err.to_errno(),
            UnifiedError::MlError(err) => err.to_errno(),
            UnifiedError::VirtualizationError(err) => err.to_errno(),
            UnifiedError::ContainerError(err) => err.to_errno(),
            UnifiedError::DeviceError(err) => err.to_errno(),
            UnifiedError::SnapshotError(err) => err.to_errno(),
            UnifiedError::ClusterError(err) => err.to_errno(),
            UnifiedError::ProcessError(err) => err.to_errno(),
            UnifiedError::SyscallError(err) => err.to_errno(),
            UnifiedError::DriverError(err) => err.to_errno(),
            UnifiedError::SecurityError(err) => err.to_errno(),
            UnifiedError::Other(_) => crate::reliability::errno::EIO,
            UnifiedError::MemoryLimitExceeded { .. } => crate::reliability::errno::ENOMEM,
            UnifiedError::ResourceLimitExceeded { .. } => crate::reliability::errno::EAGAIN,
            UnifiedError::InsufficientResources { .. } => crate::reliability::errno::EAGAIN,
            UnifiedError::IoQuotaExceeded { .. } => crate::reliability::errno::EDQUOT,
            UnifiedError::SyncError(_) => crate::reliability::errno::EIO,
            UnifiedError::AtomicError(_) => crate::reliability::errno::EIO,
            UnifiedError::RcuError(_) => crate::reliability::errno::EIO,
            UnifiedError::ParallelError(_) => crate::reliability::errno::EIO,
            UnifiedError::ConcurrencyError(_) => crate::reliability::errno::EIO,
        }
    }
}

impl MemoryError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            MemoryError::OutOfMemory => crate::reliability::errno::ENOMEM,
            MemoryError::InvalidAlignment => crate::reliability::errno::EINVAL,
            MemoryError::InvalidSize => crate::reliability::errno::EINVAL,
            MemoryError::CorruptedAllocator => crate::reliability::errno::EIO,
            MemoryError::TooFragmented => crate::reliability::errno::ENOMEM,
            MemoryError::InvalidAddress => crate::reliability::errno::EFAULT,
            MemoryError::InvalidProtection => crate::reliability::errno::EINVAL,
            MemoryError::MappingFailed => crate::reliability::errno::ENOMEM,
            MemoryError::UnmappingFailed => crate::reliability::errno::EINVAL,
        }
    }
}

impl FileSystemError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            FileSystemError::PathNotFound => crate::reliability::errno::ENOENT,
            FileSystemError::FileNotFound => crate::reliability::errno::ENOENT,
            FileSystemError::PermissionDenied => crate::reliability::errno::EPERM,
            FileSystemError::FileExists => crate::reliability::errno::EEXIST,
            FileSystemError::NotADirectory => crate::reliability::errno::ENOTDIR,
            FileSystemError::IsADirectory => crate::reliability::errno::EISDIR,
            FileSystemError::DirectoryNotEmpty => crate::reliability::errno::ENOTEMPTY,
            FileSystemError::InvalidPath => crate::reliability::errno::EINVAL,
            FileSystemError::PathTooLong => crate::reliability::errno::ENAMETOOLONG,
            FileSystemError::FileSystemFull => crate::reliability::errno::ENOSPC,
            FileSystemError::IoError => crate::reliability::errno::EIO,
            FileSystemError::ResourceBusy => crate::reliability::errno::EBUSY,
            FileSystemError::OperationNotSupported => crate::reliability::errno::EOPNOTSUPP,
            FileSystemError::FileSystemCorrupted => crate::reliability::errno::EIO,
            FileSystemError::QuotaExceeded => crate::reliability::errno::EDQUOT,
        }
    }
}

impl NetworkError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            NetworkError::ConnectionRefused => crate::reliability::errno::ECONNREFUSED,
            NetworkError::ConnectionReset => crate::reliability::errno::ECONNRESET,
            NetworkError::BrokenPipe => crate::reliability::errno::EPIPE,
            NetworkError::TimedOut => crate::reliability::errno::ETIMEDOUT,
            NetworkError::HostUnreachable => crate::reliability::errno::EHOSTUNREACH,
            NetworkError::NetworkUnreachable => crate::reliability::errno::ENETUNREACH,
            NetworkError::AddressInUse => crate::reliability::errno::EADDRINUSE,
            NetworkError::NoBufferSpace => crate::reliability::errno::ENOBUFS,
            NetworkError::MessageTooLarge => crate::reliability::errno::EMSGSIZE,
            NetworkError::ProtocolError => crate::reliability::errno::EPROTO,
            NetworkError::NetworkDown => crate::reliability::errno::ENETDOWN,
            NetworkError::ConnectionAborted => crate::reliability::errno::ECONNABORTED,
        }
    }
}

impl ProcessError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            ProcessError::ProcessNotFound => crate::reliability::errno::ESRCH,
            ProcessError::PermissionDenied => crate::reliability::errno::EPERM,
            ProcessError::InvalidArgument => crate::reliability::errno::EINVAL,
            ProcessError::ResourceLimitExceeded => crate::reliability::errno::EAGAIN,
            ProcessError::ProcessAlreadyExists => crate::reliability::errno::EEXIST,
            ProcessError::ProcessTerminated => crate::reliability::errno::ESRCH,
            ProcessError::ProcessNotRunning => crate::reliability::errno::ESRCH,
            ProcessError::InvalidState => crate::reliability::errno::EINVAL,
            ProcessError::StackOverflow => crate::reliability::errno::ENOMEM,
            ProcessError::HeapCorruption => crate::reliability::errno::EIO,
            ProcessError::ProcessKilled => crate::reliability::errno::ESRCH,
        }
    }
}

impl SyscallError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            SyscallError::InvalidSyscall => crate::reliability::errno::ENOSYS,
            SyscallError::PermissionDenied => crate::reliability::errno::EPERM,
            SyscallError::InvalidArgument => crate::reliability::errno::EINVAL,
            SyscallError::NotFound => crate::reliability::errno::ENOENT,
            SyscallError::OutOfMemory => crate::reliability::errno::ENOMEM,
            SyscallError::Interrupted => crate::reliability::errno::EINTR,
            SyscallError::IoError => crate::reliability::errno::EIO,
            SyscallError::WouldBlock => crate::reliability::errno::EAGAIN,
            SyscallError::NotSupported => crate::reliability::errno::EOPNOTSUPP,
            SyscallError::NotImplemented => crate::reliability::errno::ENOSYS,
            SyscallError::BadFileDescriptor => crate::reliability::errno::EBADF,
            SyscallError::TooManyOpenFiles => crate::reliability::errno::EMFILE,
            SyscallError::NoBufferSpace => crate::reliability::errno::ENOBUFS,
            SyscallError::NotADirectory => crate::reliability::errno::ENOTDIR,
            SyscallError::IsADirectory => crate::reliability::errno::EISDIR,
            SyscallError::DirectoryNotEmpty => crate::reliability::errno::ENOTEMPTY,
            SyscallError::FileExists => crate::reliability::errno::EEXIST,
            SyscallError::CrossDeviceLink => crate::reliability::errno::EXDEV,
            SyscallError::FileTooBig => crate::reliability::errno::EFBIG,
            SyscallError::NoSpaceLeft => crate::reliability::errno::ENOSPC,
            SyscallError::BadAddress => crate::reliability::errno::EFAULT,
            SyscallError::DeadlockWouldOccur => crate::reliability::errno::EDEADLK,
            SyscallError::NameTooLong => crate::reliability::errno::ENAMETOOLONG,
            SyscallError::TooManySymlinks => crate::reliability::errno::ELOOP,
            SyscallError::ConnectionRefused => crate::reliability::errno::ECONNREFUSED,
            SyscallError::ConnectionReset => crate::reliability::errno::ECONNRESET,
            SyscallError::BrokenPipe => crate::reliability::errno::EPIPE,
            SyscallError::TimedOut => crate::reliability::errno::ETIMEDOUT,
            SyscallError::NoProcess => crate::reliability::errno::ESRCH,
            SyscallError::OperationNotPermitted => crate::reliability::errno::EPERM,
        }
    }
}

impl DriverError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            DriverError::DeviceNotFound => crate::reliability::errno::ENODEV,
            DriverError::DeviceBusy => crate::reliability::errno::EBUSY,
            DriverError::DeviceNotConnected => crate::reliability::errno::ENXIO,
            DriverError::UnsupportedOperation => crate::reliability::errno::EOPNOTSUPP,
            DriverError::HardwareFailure => crate::reliability::errno::EIO,
            DriverError::InvalidConfiguration => crate::reliability::errno::EINVAL,
            DriverError::ResourceConflict => crate::reliability::errno::EBUSY,
            DriverError::Timeout => crate::reliability::errno::ETIMEDOUT,
            DriverError::FirmwareMissing => crate::reliability::errno::ENOENT,
            DriverError::DriverNotLoaded => crate::reliability::errno::ENODEV,
            DriverError::InvalidParameter => crate::reliability::errno::EINVAL,
            DriverError::DeviceManager(_) => crate::reliability::errno::EIO,
        }
    }
}

impl SecurityError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            SecurityError::AccessDenied => crate::reliability::errno::EACCES,
            SecurityError::PermissionDenied => crate::reliability::errno::EPERM,
            SecurityError::AuthenticationFailed => crate::reliability::errno::EACCES,
            SecurityError::AuthorizationFailed => crate::reliability::errno::EACCES,
            SecurityError::SecurityPolicyViolation => crate::reliability::errno::EPERM,
            SecurityError::InvalidCredentials => crate::reliability::errno::EACCES,
            SecurityError::AccountLocked => crate::reliability::errno::EACCES,
            SecurityError::PasswordExpired => crate::reliability::errno::EACCES,
            SecurityError::AccountDisabled => crate::reliability::errno::EACCES,
            SecurityError::InsufficientPrivileges => crate::reliability::errno::EPERM,
            SecurityError::SecurityBreach => crate::reliability::errno::EACCES,
            SecurityError::AttestationFailed(_) => crate::reliability::errno::EACCES,
        }
    }
}

/// 获取当前时间戳（纳秒）
fn get_timestamp() -> u64 {
    // 在实际实现中，这应该从系统时钟获取
    // 这里使用一个简单的实现
    0
}

/// 统一结果类型
pub type UnifiedResult<T> = core::result::Result<T, UnifiedError>;

/// 错误处理宏
/// 用于创建带有位置信息的错误上下文
#[macro_export]
macro_rules! create_error {
    ($error:expr) => {
        ErrorContext::new($error, module_path!())
    };
    ($error:expr, $severity:expr) => {
        ErrorContext::new($error, module_path!()).with_severity($severity)
    };
    ($error:expr, $description:expr, $cause:expr) => {
        ErrorContext::new($error, module_path!())
            .with_description($description.to_string())
            .with_cause($cause.to_string())
    };
}

/// 错误处理宏
/// 用于返回带有位置信息的错误
#[macro_export]
macro_rules! return_error {
    ($error:expr) => {
        return Err(UnifiedError::from($error));
    };
    ($error:expr, $severity:expr) => {
        return Err(UnifiedError::from($error).with_severity($severity));
    };
    ($error:expr, $description:expr, $cause:expr) => {
        return Err(UnifiedError::from($error)
            .with_description($description.to_string())
            .with_cause($cause.to_string()));
    };
}

/// 从其他错误类型转换为统一错误
impl From<crate::subsystems::syscalls::api::SyscallError> for UnifiedError {
    fn from(err: crate::subsystems::syscalls::api::SyscallError) -> Self {
        UnifiedError::SyscallError(SyscallError::from(err))
    }
}

/// 从 syscalls::api::SyscallError 转换为 error::unified::SyscallError
impl From<crate::subsystems::syscalls::api::SyscallError> for SyscallError {
    fn from(err: crate::subsystems::syscalls::api::SyscallError) -> Self {
        match err {
            crate::subsystems::syscalls::api::SyscallError::InvalidArgument => SyscallError::InvalidArgument,
            crate::subsystems::syscalls::api::SyscallError::PermissionDenied => SyscallError::PermissionDenied,
            crate::subsystems::syscalls::api::SyscallError::NotFound => SyscallError::NotFound,
            crate::subsystems::syscalls::api::SyscallError::IoError => SyscallError::IoError,
            crate::subsystems::syscalls::api::SyscallError::NoProcess => SyscallError::NoProcess,
            crate::subsystems::syscalls::api::SyscallError::OperationNotPermitted => SyscallError::OperationNotPermitted,
            crate::subsystems::syscalls::api::SyscallError::NotImplemented => SyscallError::NotImplemented,
        }
    }
}

/// 从 error::unified::SyscallError 转换为 UnifiedError
impl From<SyscallError> for UnifiedError {
    fn from(err: SyscallError) -> Self {
        UnifiedError::SyscallError(err)
    }
}

/// 从系统调用分发器错误转换为统一错误
impl From<crate::subsystems::syscalls::dispatch::dispatcher::DispatcherError> for UnifiedError {
    fn from(err: crate::subsystems::syscalls::dispatch::dispatcher::DispatcherError) -> Self {
        match err {
            crate::subsystems::syscalls::dispatch::dispatcher::DispatcherError::SyscallNotSupported(_) => {
                UnifiedError::NotSupported
            }
            crate::subsystems::syscalls::dispatch::dispatcher::DispatcherError::ServiceUnavailable(_) => {
                UnifiedError::ResourceUnavailable
            }
            crate::subsystems::syscalls::dispatch::dispatcher::DispatcherError::MaxRetriesExceeded(_) => {
                UnifiedError::InvalidOperation
            }
            crate::subsystems::syscalls::dispatch::dispatcher::DispatcherError::CacheError(_) => {
                UnifiedError::IoError
            }
            crate::subsystems::syscalls::dispatch::dispatcher::DispatcherError::InvalidParameters(_) => {
                UnifiedError::InvalidArgument
            }
        }
    }
}

impl From<crate::subsystems::fs::api::error::FsError> for UnifiedError {
    fn from(err: crate::subsystems::fs::api::error::FsError) -> Self {
        UnifiedError::FileSystemError(FileSystemError::from(err))
    }
}

impl From<crate::subsystems::mm::api::AllocError> for UnifiedError {
    fn from(err: crate::subsystems::mm::api::AllocError) -> Self {
        UnifiedError::MemoryError(MemoryError::from(err))
    }
}

impl From<crate::subsystems::mm::api::VmError> for UnifiedError {
    fn from(err: crate::subsystems::mm::api::VmError) -> Self {
        UnifiedError::MemoryError(MemoryError::from(err))
    }
}


/// 从文件系统错误转换为统一错误中的文件系统错误
impl From<crate::subsystems::fs::api::error::FsError> for FileSystemError {
    fn from(err: crate::subsystems::fs::api::error::FsError) -> Self {
        match err {
            crate::subsystems::fs::api::error::FsError::PathNotFound => {
                FileSystemError::PathNotFound
            },
            crate::subsystems::fs::api::error::FsError::FileNotFound => {
                FileSystemError::FileNotFound
            },
            crate::subsystems::fs::api::error::FsError::NotFound => {
                FileSystemError::PathNotFound
            },
            crate::subsystems::fs::api::error::FsError::PermissionDenied => {
                FileSystemError::PermissionDenied
            },
            crate::subsystems::fs::api::error::FsError::FileExists => FileSystemError::FileExists,
            crate::subsystems::fs::api::error::FsError::Exists => FileSystemError::FileExists,
            crate::subsystems::fs::api::error::FsError::NotADirectory => {
                FileSystemError::NotADirectory
            },
            crate::subsystems::fs::api::error::FsError::IsADirectory => {
                FileSystemError::IsADirectory
            },
            crate::subsystems::fs::api::error::FsError::DirectoryNotEmpty => {
                FileSystemError::DirectoryNotEmpty
            },
            crate::subsystems::fs::api::error::FsError::NotEmpty => {
                FileSystemError::DirectoryNotEmpty
            },
            crate::subsystems::fs::api::error::FsError::InvalidPath => FileSystemError::InvalidPath,
            crate::subsystems::fs::api::error::FsError::InvalidInput => FileSystemError::InvalidPath,
            crate::subsystems::fs::api::error::FsError::PathTooLong => FileSystemError::PathTooLong,
            crate::subsystems::fs::api::error::FsError::FileSystemFull => {
                FileSystemError::FileSystemFull
            },
            crate::subsystems::fs::api::error::FsError::NoSpace => {
                FileSystemError::FileSystemFull
            },
            crate::subsystems::fs::api::error::FsError::IoError => FileSystemError::IoError,
            crate::subsystems::fs::api::error::FsError::ResourceBusy => {
                FileSystemError::ResourceBusy
            },
            crate::subsystems::fs::api::error::FsError::Busy => {
                FileSystemError::ResourceBusy
            },
            crate::subsystems::fs::api::error::FsError::OperationNotSupported => {
                FileSystemError::OperationNotSupported
            },
            crate::subsystems::fs::api::error::FsError::NotSupported => {
                FileSystemError::OperationNotSupported
            },
            crate::subsystems::fs::api::error::FsError::QuotaExceeded => {
                FileSystemError::QuotaExceeded
            },
            crate::subsystems::fs::api::error::FsError::NotMounted => {
                FileSystemError::PathNotFound
            },
            crate::subsystems::fs::api::error::FsError::ReadOnly => {
                FileSystemError::IoError
            },
            crate::subsystems::fs::api::error::FsError::InvalidOperation => {
                FileSystemError::InvalidPath
            },
            crate::subsystems::fs::api::error::FsError::Loop => {
                FileSystemError::InvalidPath
            },
            crate::subsystems::fs::api::error::FsError::TooManyLinks => {
                FileSystemError::PathTooLong
            },
        }
    }
}

/// 从内存分配错误转换为统一错误中的内存错误
impl From<crate::subsystems::mm::api::AllocError> for MemoryError {
    fn from(err: crate::subsystems::mm::api::AllocError) -> Self {
        match err {
            crate::subsystems::mm::api::AllocError::OutOfMemory => MemoryError::OutOfMemory,
            crate::subsystems::mm::api::AllocError::InvalidAlignment => {
                MemoryError::InvalidAlignment
            },
            crate::subsystems::mm::api::AllocError::InvalidSize => MemoryError::InvalidSize,
            crate::subsystems::mm::api::AllocError::CorruptedAllocator => {
                MemoryError::CorruptedAllocator
            },
            crate::subsystems::mm::api::AllocError::TooFragmented => MemoryError::TooFragmented,
        }
    }
}

/// 从虚拟内存错误转换为统一错误中的内存错误
impl From<crate::subsystems::mm::api::VmError> for MemoryError {
    fn from(err: crate::subsystems::mm::api::VmError) -> Self {
        match err {
            crate::subsystems::mm::api::VmError::InvalidAddress => MemoryError::InvalidAddress,
            crate::subsystems::mm::api::VmError::InvalidSize => MemoryError::InvalidSize,
            crate::subsystems::mm::api::VmError::InvalidProtection => {
                MemoryError::InvalidProtection
            },
            crate::subsystems::mm::api::VmError::MappingNotFound => MemoryError::InvalidAddress,
            crate::subsystems::mm::api::VmError::PermissionDenied => MemoryError::InvalidProtection,
            crate::subsystems::mm::api::VmError::AddressAlreadyMapped => MemoryError::InvalidAddress,
            crate::subsystems::mm::api::VmError::PageTableError => MemoryError::CorruptedAllocator,
            crate::subsystems::mm::api::VmError::TLBError => MemoryError::InvalidAddress,
        }
    }
}

/// 从 Attestation 错误转换为统一错误
impl From<crate::security::attestation::AttestationError> for UnifiedError {
    fn from(err: crate::security::attestation::AttestationError) -> Self {
        UnifiedError::SecurityError(SecurityError::AttestationFailed(format!("{:?}", err)))
    }
}

/// 虚拟化相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VirtualizationError {
    /// Hypervisor相关错误
    VmNotFound,
    VmAlreadyExists,
    VmCreationFailed(String),
    VmDestructionFailed(String),
    VcpuNotFound,
    VcpuCreationFailed(String),
    VcpuRunFailed(String),
    VmExitUnhandled(String),
    MemoryMappingFailed,
    MemoryUnmappingFailed,
    EptViolation,
    NptViolation,
    InterruptInjectionFailed,
    VmStateInvalid,
    VmNotRunning,
    VmAlreadyRunning,
    VmPaused,
    VmStopped,
    VmStateTransitionFailed,
}

impl fmt::Display for VirtualizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VirtualizationError::VmNotFound => write!(f, "VM not found"),
            VirtualizationError::VmAlreadyExists => write!(f, "VM already exists"),
            VirtualizationError::VmCreationFailed(msg) => write!(f, "VM creation failed: {}", msg),
            VirtualizationError::VmDestructionFailed(msg) => write!(f, "VM destruction failed: {}", msg),
            VirtualizationError::VcpuNotFound => write!(f, "vCPU not found"),
            VirtualizationError::VcpuCreationFailed(msg) => write!(f, "vCPU creation failed: {}", msg),
            VirtualizationError::VcpuRunFailed(msg) => write!(f, "vCPU run failed: {}", msg),
            VirtualizationError::VmExitUnhandled(msg) => write!(f, "VM exit unhandled: {}", msg),
            VirtualizationError::MemoryMappingFailed => write!(f, "Memory mapping failed"),
            VirtualizationError::MemoryUnmappingFailed => write!(f, "Memory unmapping failed"),
            VirtualizationError::EptViolation => write!(f, "EPT violation"),
            VirtualizationError::NptViolation => write!(f, "NPT violation"),
            VirtualizationError::InterruptInjectionFailed => write!(f, "Interrupt injection failed"),
            VirtualizationError::VmStateInvalid => write!(f, "Invalid VM state"),
            VirtualizationError::VmNotRunning => write!(f, "VM is not running"),
            VirtualizationError::VmAlreadyRunning => write!(f, "VM is already running"),
            VirtualizationError::VmPaused => write!(f, "VM is paused"),
            VirtualizationError::VmStopped => write!(f, "VM is stopped"),
            VirtualizationError::VmStateTransitionFailed => write!(f, "VM state transition failed"),
        }
    }
}

impl VirtualizationError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            VirtualizationError::VmNotFound => crate::reliability::errno::ENOENT,
            VirtualizationError::VmAlreadyExists => crate::reliability::errno::EEXIST,
            VirtualizationError::VmCreationFailed(_) => crate::reliability::errno::EINVAL,
            VirtualizationError::VmDestructionFailed(_) => crate::reliability::errno::EIO,
            VirtualizationError::VcpuNotFound => crate::reliability::errno::ENOENT,
            VirtualizationError::VcpuCreationFailed(_) => crate::reliability::errno::EINVAL,
            VirtualizationError::VcpuRunFailed(_) => crate::reliability::errno::EIO,
            VirtualizationError::VmExitUnhandled(_) => crate::reliability::errno::EIO,
            VirtualizationError::MemoryMappingFailed => crate::reliability::errno::ENOMEM,
            VirtualizationError::MemoryUnmappingFailed => crate::reliability::errno::EINVAL,
            VirtualizationError::EptViolation => crate::reliability::errno::EFAULT,
            VirtualizationError::NptViolation => crate::reliability::errno::EFAULT,
            VirtualizationError::InterruptInjectionFailed => crate::reliability::errno::EIO,
            VirtualizationError::VmStateInvalid => crate::reliability::errno::EINVAL,
            VirtualizationError::VmNotRunning => crate::reliability::errno::EINVAL,
            VirtualizationError::VmAlreadyRunning => crate::reliability::errno::EBUSY,
            VirtualizationError::VmPaused => crate::reliability::errno::EBUSY,
            VirtualizationError::VmStopped => crate::reliability::errno::EINVAL,
            VirtualizationError::VmStateTransitionFailed => crate::reliability::errno::EIO,
        }
    }
}

/// 从 DeviceError 转换为 VirtualizationError
impl From<DeviceError> for VirtualizationError {
    fn from(err: DeviceError) -> Self {
        VirtualizationError::VmCreationFailed(format!("Device error: {:?}", err))
    }
}

/// 容器相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerError {
    /// Container相关错误
    ContainerNotFound,
    ContainerAlreadyExists,
    ContainerCreationFailed(String),
    ContainerStartFailed(String),
    ContainerStopFailed(String),
    ContainerDeleteFailed(String),
    ContainerNotRunning,
    ContainerAlreadyRunning,
    ContainerStateInvalid,
    /// Namespace相关错误
    NamespaceCreationFailed(String),
    NamespaceConfigurationFailed(String),
    NamespaceNotFound,
    /// Cgroup相关错误
    CgroupCreationFailed(String),
    CgroupConfigurationFailed(String),
    CgroupNotFound,
    ResourceLimitExceeded(String),
    /// Filesystem相关错误
    RootfsSetupFailed(String),
    PivotRootFailed(String),
    ChrootFailed(String),
    MountFailed(String),
    /// Network相关错误
    NetworkSetupFailed(String),
    VethPairCreationFailed(String),
    BridgeCreationFailed(String),
    /// OCI规范相关错误
    OciSpecInvalid(String),
    OciSpecNotSupported(String),
    OciHookFailed(String),
    /// Isolation相关错误
    IsolationSetupFailed(String),
    SeccompFilterFailed(String),
    CapabilitySetFailed(String),
}

impl fmt::Display for ContainerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerError::ContainerNotFound => write!(f, "Container not found"),
            ContainerError::ContainerAlreadyExists => write!(f, "Container already exists"),
            ContainerError::ContainerCreationFailed(msg) => write!(f, "Container creation failed: {}", msg),
            ContainerError::ContainerStartFailed(msg) => write!(f, "Container start failed: {}", msg),
            ContainerError::ContainerStopFailed(msg) => write!(f, "Container stop failed: {}", msg),
            ContainerError::ContainerDeleteFailed(msg) => write!(f, "Container delete failed: {}", msg),
            ContainerError::ContainerNotRunning => write!(f, "Container is not running"),
            ContainerError::ContainerAlreadyRunning => write!(f, "Container is already running"),
            ContainerError::ContainerStateInvalid => write!(f, "Invalid container state"),
            ContainerError::NamespaceCreationFailed(msg) => write!(f, "Namespace creation failed: {}", msg),
            ContainerError::NamespaceConfigurationFailed(msg) => write!(f, "Namespace configuration failed: {}", msg),
            ContainerError::NamespaceNotFound => write!(f, "Namespace not found"),
            ContainerError::CgroupCreationFailed(msg) => write!(f, "Cgroup creation failed: {}", msg),
            ContainerError::CgroupConfigurationFailed(msg) => write!(f, "Cgroup configuration failed: {}", msg),
            ContainerError::CgroupNotFound => write!(f, "Cgroup not found"),
            ContainerError::ResourceLimitExceeded(res) => write!(f, "Resource limit exceeded: {}", res),
            ContainerError::RootfsSetupFailed(msg) => write!(f, "Rootfs setup failed: {}", msg),
            ContainerError::PivotRootFailed(msg) => write!(f, "Pivot root failed: {}", msg),
            ContainerError::ChrootFailed(msg) => write!(f, "Chroot failed: {}", msg),
            ContainerError::MountFailed(msg) => write!(f, "Mount failed: {}", msg),
            ContainerError::NetworkSetupFailed(msg) => write!(f, "Network setup failed: {}", msg),
            ContainerError::VethPairCreationFailed(msg) => write!(f, "Veth pair creation failed: {}", msg),
            ContainerError::BridgeCreationFailed(msg) => write!(f, "Bridge creation failed: {}", msg),
            ContainerError::OciSpecInvalid(msg) => write!(f, "Invalid OCI spec: {}", msg),
            ContainerError::OciSpecNotSupported(msg) => write!(f, "OCI spec not supported: {}", msg),
            ContainerError::OciHookFailed(msg) => write!(f, "OCI hook failed: {}", msg),
            ContainerError::IsolationSetupFailed(msg) => write!(f, "Isolation setup failed: {}", msg),
            ContainerError::SeccompFilterFailed(msg) => write!(f, "Seccomp filter failed: {}", msg),
            ContainerError::CapabilitySetFailed(msg) => write!(f, "Capability set failed: {}", msg),
        }
    }
}

impl ContainerError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            ContainerError::ContainerNotFound => crate::reliability::errno::ENOENT,
            ContainerError::ContainerAlreadyExists => crate::reliability::errno::EEXIST,
            ContainerError::ContainerCreationFailed(_) => crate::reliability::errno::EINVAL,
            ContainerError::ContainerStartFailed(_) => crate::reliability::errno::EIO,
            ContainerError::ContainerStopFailed(_) => crate::reliability::errno::EIO,
            ContainerError::ContainerDeleteFailed(_) => crate::reliability::errno::EIO,
            ContainerError::ContainerNotRunning => crate::reliability::errno::EINVAL,
            ContainerError::ContainerAlreadyRunning => crate::reliability::errno::EBUSY,
            ContainerError::ContainerStateInvalid => crate::reliability::errno::EINVAL,
            ContainerError::NamespaceCreationFailed(_) => crate::reliability::errno::ENOMEM,
            ContainerError::NamespaceConfigurationFailed(_) => crate::reliability::errno::EINVAL,
            ContainerError::NamespaceNotFound => crate::reliability::errno::ENOENT,
            ContainerError::CgroupCreationFailed(_) => crate::reliability::errno::ENOMEM,
            ContainerError::CgroupConfigurationFailed(_) => crate::reliability::errno::EINVAL,
            ContainerError::CgroupNotFound => crate::reliability::errno::ENOENT,
            ContainerError::ResourceLimitExceeded(_) => crate::reliability::errno::EDQUOT,
            ContainerError::RootfsSetupFailed(_) => crate::reliability::errno::EIO,
            ContainerError::PivotRootFailed(_) => crate::reliability::errno::EIO,
            ContainerError::ChrootFailed(_) => crate::reliability::errno::EIO,
            ContainerError::MountFailed(_) => crate::reliability::errno::EIO,
            ContainerError::NetworkSetupFailed(_) => crate::reliability::errno::EIO,
            ContainerError::VethPairCreationFailed(_) => crate::reliability::errno::ENOMEM,
            ContainerError::BridgeCreationFailed(_) => crate::reliability::errno::EIO,
            ContainerError::OciSpecInvalid(_) => crate::reliability::errno::EINVAL,
            ContainerError::OciSpecNotSupported(_) => crate::reliability::errno::EOPNOTSUPP,
            ContainerError::OciHookFailed(_) => crate::reliability::errno::EIO,
            ContainerError::IsolationSetupFailed(_) => crate::reliability::errno::EIO,
            ContainerError::SeccompFilterFailed(_) => crate::reliability::errno::EIO,
            ContainerError::CapabilitySetFailed(_) => crate::reliability::errno::EPERM,
        }
    }
}

/// 设备虚拟化相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceError {
    /// Virtio设备错误
    VirtioDeviceNotFound,
    VirtioDeviceInitializationFailed(String),
    VirtioDeviceConfigurationFailed(String),
    VirtioQueueNotFound,
    VirtioQueueCreationFailed(String),
    VirtioQueueFull,
    VirtioDescriptorError(String),
    /// MMIO/PIO错误
    MmioAccessFailed(String),
    PioAccessFailed(String),
    InvalidMmioAddress,
    InvalidPioAddress,
    /// 中断相关错误
    InterruptAllocationFailed,
    InterruptRoutingFailed,
    MsiConfigurationFailed(String),
    MsixConfigurationFailed(String),
    /// 热插拔错误
    DeviceHotplugFailed(String),
    DeviceHotunplugFailed(String),
    DeviceNotSupported,
    /// 块设备错误
    BlockDeviceError(String),
    BlockIoFailed(String),
    /// 网络设备错误
    NetworkDeviceError(String),
    NetworkIoFailed(String),
    /// 串口设备错误
    SerialDeviceError(String),
    ConsoleError(String),
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceError::VirtioDeviceNotFound => write!(f, "Virtio device not found"),
            DeviceError::VirtioDeviceInitializationFailed(msg) => write!(f, "Virtio device initialization failed: {}", msg),
            DeviceError::VirtioDeviceConfigurationFailed(msg) => write!(f, "Virtio device configuration failed: {}", msg),
            DeviceError::VirtioQueueNotFound => write!(f, "Virtio queue not found"),
            DeviceError::VirtioQueueCreationFailed(msg) => write!(f, "Virtio queue creation failed: {}", msg),
            DeviceError::VirtioQueueFull => write!(f, "Virtio queue full"),
            DeviceError::VirtioDescriptorError(msg) => write!(f, "Virtio descriptor error: {}", msg),
            DeviceError::MmioAccessFailed(msg) => write!(f, "MMIO access failed: {}", msg),
            DeviceError::PioAccessFailed(msg) => write!(f, "PIO access failed: {}", msg),
            DeviceError::InvalidMmioAddress => write!(f, "Invalid MMIO address"),
            DeviceError::InvalidPioAddress => write!(f, "Invalid PIO address"),
            DeviceError::InterruptAllocationFailed => write!(f, "Interrupt allocation failed"),
            DeviceError::InterruptRoutingFailed => write!(f, "Interrupt routing failed"),
            DeviceError::MsiConfigurationFailed(msg) => write!(f, "MSI configuration failed: {}", msg),
            DeviceError::MsixConfigurationFailed(msg) => write!(f, "MSI-X configuration failed: {}", msg),
            DeviceError::DeviceHotplugFailed(msg) => write!(f, "Device hotplug failed: {}", msg),
            DeviceError::DeviceHotunplugFailed(msg) => write!(f, "Device hotunplug failed: {}", msg),
            DeviceError::DeviceNotSupported => write!(f, "Device not supported"),
            DeviceError::BlockDeviceError(msg) => write!(f, "Block device error: {}", msg),
            DeviceError::BlockIoFailed(msg) => write!(f, "Block I/O failed: {}", msg),
            DeviceError::NetworkDeviceError(msg) => write!(f, "Network device error: {}", msg),
            DeviceError::NetworkIoFailed(msg) => write!(f, "Network I/O failed: {}", msg),
            DeviceError::SerialDeviceError(msg) => write!(f, "Serial device error: {}", msg),
            DeviceError::ConsoleError(msg) => write!(f, "Console error: {}", msg),
        }
    }
}

impl DeviceError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            DeviceError::VirtioDeviceNotFound => crate::reliability::errno::ENOENT,
            DeviceError::VirtioDeviceInitializationFailed(_) => crate::reliability::errno::EIO,
            DeviceError::VirtioDeviceConfigurationFailed(_) => crate::reliability::errno::EINVAL,
            DeviceError::VirtioQueueNotFound => crate::reliability::errno::ENOENT,
            DeviceError::VirtioQueueCreationFailed(_) => crate::reliability::errno::ENOMEM,
            DeviceError::VirtioQueueFull => crate::reliability::errno::EBUSY,
            DeviceError::VirtioDescriptorError(_) => crate::reliability::errno::EIO,
            DeviceError::MmioAccessFailed(_) => crate::reliability::errno::EIO,
            DeviceError::PioAccessFailed(_) => crate::reliability::errno::EIO,
            DeviceError::InvalidMmioAddress => crate::reliability::errno::EFAULT,
            DeviceError::InvalidPioAddress => crate::reliability::errno::EFAULT,
            DeviceError::InterruptAllocationFailed => crate::reliability::errno::ENOMEM,
            DeviceError::InterruptRoutingFailed => crate::reliability::errno::EIO,
            DeviceError::MsiConfigurationFailed(_) => crate::reliability::errno::EIO,
            DeviceError::MsixConfigurationFailed(_) => crate::reliability::errno::EIO,
            DeviceError::DeviceHotplugFailed(_) => crate::reliability::errno::EIO,
            DeviceError::DeviceHotunplugFailed(_) => crate::reliability::errno::EIO,
            DeviceError::DeviceNotSupported => crate::reliability::errno::EOPNOTSUPP,
            DeviceError::BlockDeviceError(_) => crate::reliability::errno::EIO,
            DeviceError::BlockIoFailed(_) => crate::reliability::errno::EIO,
            DeviceError::NetworkDeviceError(_) => crate::reliability::errno::EIO,
            DeviceError::NetworkIoFailed(_) => crate::reliability::errno::EIO,
            DeviceError::SerialDeviceError(_) => crate::reliability::errno::EIO,
            DeviceError::ConsoleError(_) => crate::reliability::errno::EIO,
        }
    }
}

/// 快照和迁移相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    /// 序列化错误
    SerializationFailed(String),
    DeserializationFailed(String),
    /// 存储错误
    StorageWriteFailed(String),
    StorageReadFailed(String),
    StorageNotAvailable,
    InsufficientStorageSpace(u64),
    /// 内存快照错误
    MemorySnapshotFailed(String),
    MemoryRestoreFailed(String),
    DirtyPageTrackingFailed(String),
    /// CPU状态错误
    CpuStateCaptureFailed(String),
    CpuStateRestoreFailed(String),
    /// 设备状态错误
    DeviceStateSaveFailed(String),
    DeviceStateRestoreFailed(String),
    /// 迁移错误
    MigrationFailed(String),
    MigrationNotSupported,
    MigrationTargetUnavailable,
    MigrationProtocolError(String),
    MigrationDataCorrupted,
    /// 增量快照错误
    IncrementalSnapshotFailed(String),
    DeltaCalculationFailed(String),
    /// 压缩错误
    CompressionFailed(String),
    DecompressionFailed(String),
    /// 快照管理错误
    SnapshotNotFound,
    SnapshotCorrupted,
    SnapshotVersionMismatch,
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnapshotError::SerializationFailed(msg) => write!(f, "Serialization failed: {}", msg),
            SnapshotError::DeserializationFailed(msg) => write!(f, "Deserialization failed: {}", msg),
            SnapshotError::StorageWriteFailed(msg) => write!(f, "Storage write failed: {}", msg),
            SnapshotError::StorageReadFailed(msg) => write!(f, "Storage read failed: {}", msg),
            SnapshotError::StorageNotAvailable => write!(f, "Storage not available"),
            SnapshotError::InsufficientStorageSpace(needed) => write!(f, "Insufficient storage space: needed {} bytes", needed),
            SnapshotError::MemorySnapshotFailed(msg) => write!(f, "Memory snapshot failed: {}", msg),
            SnapshotError::MemoryRestoreFailed(msg) => write!(f, "Memory restore failed: {}", msg),
            SnapshotError::DirtyPageTrackingFailed(msg) => write!(f, "Dirty page tracking failed: {}", msg),
            SnapshotError::CpuStateCaptureFailed(msg) => write!(f, "CPU state capture failed: {}", msg),
            SnapshotError::CpuStateRestoreFailed(msg) => write!(f, "CPU state restore failed: {}", msg),
            SnapshotError::DeviceStateSaveFailed(msg) => write!(f, "Device state save failed: {}", msg),
            SnapshotError::DeviceStateRestoreFailed(msg) => write!(f, "Device state restore failed: {}", msg),
            SnapshotError::MigrationFailed(msg) => write!(f, "Migration failed: {}", msg),
            SnapshotError::MigrationNotSupported => write!(f, "Migration not supported"),
            SnapshotError::MigrationTargetUnavailable => write!(f, "Migration target unavailable"),
            SnapshotError::MigrationProtocolError(msg) => write!(f, "Migration protocol error: {}", msg),
            SnapshotError::MigrationDataCorrupted => write!(f, "Migration data corrupted"),
            SnapshotError::IncrementalSnapshotFailed(msg) => write!(f, "Incremental snapshot failed: {}", msg),
            SnapshotError::DeltaCalculationFailed(msg) => write!(f, "Delta calculation failed: {}", msg),
            SnapshotError::CompressionFailed(msg) => write!(f, "Compression failed: {}", msg),
            SnapshotError::DecompressionFailed(msg) => write!(f, "Decompression failed: {}", msg),
            SnapshotError::SnapshotNotFound => write!(f, "Snapshot not found"),
            SnapshotError::SnapshotCorrupted => write!(f, "Snapshot corrupted"),
            SnapshotError::SnapshotVersionMismatch => write!(f, "Snapshot version mismatch"),
        }
    }
}

impl SnapshotError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            SnapshotError::SerializationFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::DeserializationFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::StorageWriteFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::StorageReadFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::StorageNotAvailable => crate::reliability::errno::EIO,
            SnapshotError::InsufficientStorageSpace(_) => crate::reliability::errno::ENOSPC,
            SnapshotError::MemorySnapshotFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::MemoryRestoreFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::DirtyPageTrackingFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::CpuStateCaptureFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::CpuStateRestoreFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::DeviceStateSaveFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::DeviceStateRestoreFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::MigrationFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::MigrationNotSupported => crate::reliability::errno::EOPNOTSUPP,
            SnapshotError::MigrationTargetUnavailable => crate::reliability::errno::EHOSTUNREACH,
            SnapshotError::MigrationProtocolError(_) => crate::reliability::errno::EPROTO,
            SnapshotError::MigrationDataCorrupted => crate::reliability::errno::EIO,
            SnapshotError::IncrementalSnapshotFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::DeltaCalculationFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::CompressionFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::DecompressionFailed(_) => crate::reliability::errno::EIO,
            SnapshotError::SnapshotNotFound => crate::reliability::errno::ENOENT,
            SnapshotError::SnapshotCorrupted => crate::reliability::errno::EIO,
            SnapshotError::SnapshotVersionMismatch => crate::reliability::errno::EINVAL,
        }
    }
}

/// TEE (Trusted Execution Environment) errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeeError {
    /// Enclave creation failed
    EnclaveCreationFailed,
    /// Enclave initialization failed
    EnclaveInitFailed,
    /// Enclave not found
    EnclaveNotFound,
    /// Invalid enclave ID
    InvalidEnclaveId,
    /// Enclave memory allocation failed
    MemoryAllocationFailed,
    /// Enclave entry point invalid
    InvalidEntryPoint,
    /// Enclave attestation failed
    AttestationFailed(String),
    /// TEE not supported on this platform
    NotSupported,
    /// TEE device not available
    DeviceUnavailable,
    /// Invalid enclave configuration
    InvalidConfiguration,
    /// Enclave destruction failed
    EnclaveDestructionFailed,
    /// Secure channel establishment failed
    SecureChannelFailed,
    /// Key provisioning failed
    KeyProvisioningFailed,
    /// Enclave memory encryption failed
    MemoryEncryptionFailed,
    /// Invalid signature
    InvalidSignature,
    /// Measurement verification failed
    MeasurementFailed,
    /// SGX-specific error
    SgxError(String),
    /// SEV-specific error
    SevError(String),
}

impl fmt::Display for TeeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TeeError::EnclaveCreationFailed => write!(f, "Failed to create enclave"),
            TeeError::EnclaveInitFailed => write!(f, "Failed to initialize enclave"),
            TeeError::EnclaveNotFound => write!(f, "Enclave not found"),
            TeeError::InvalidEnclaveId => write!(f, "Invalid enclave ID"),
            TeeError::MemoryAllocationFailed => write!(f, "Failed to allocate enclave memory"),
            TeeError::InvalidEntryPoint => write!(f, "Invalid enclave entry point"),
            TeeError::AttestationFailed(msg) => write!(f, "Enclave attestation failed: {}", msg),
            TeeError::NotSupported => write!(f, "TEE not supported on this platform"),
            TeeError::DeviceUnavailable => write!(f, "TEE device unavailable"),
            TeeError::InvalidConfiguration => write!(f, "Invalid enclave configuration"),
            TeeError::EnclaveDestructionFailed => write!(f, "Failed to destroy enclave"),
            TeeError::SecureChannelFailed => write!(f, "Failed to establish secure channel"),
            TeeError::KeyProvisioningFailed => write!(f, "Failed to provision keys"),
            TeeError::MemoryEncryptionFailed => write!(f, "Failed to encrypt enclave memory"),
            TeeError::InvalidSignature => write!(f, "Invalid enclave signature"),
            TeeError::MeasurementFailed => write!(f, "Enclave measurement verification failed"),
            TeeError::SgxError(msg) => write!(f, "SGX error: {}", msg),
            TeeError::SevError(msg) => write!(f, "SEV error: {}", msg),
        }
    }
}

/// Secure Boot errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecureBootError {
    /// Signature verification failed
    SignatureVerificationFailed,
    /// Certificate validation failed
    CertificateValidationFailed,
    /// Certificate chain invalid
    InvalidCertificateChain,
    /// Certificate expired
    CertificateExpired,
    /// Certificate revoked
    CertificateRevoked,
    /// Key not found in DB
    KeyNotFound,
    /// Key found in DBX (forbidden list)
    KeyInForbiddenList,
    /// TPM operation failed
    TpmError(String),
    /// Secure Boot state invalid
    InvalidState,
    /// DB/DBX database corrupted
    DatabaseCorrupted,
    /// Module signature verification failed
    ModuleVerificationFailed,
    /// Boot measurement failed
    MeasurementFailed,
    /// Recovery failed
    RecoveryFailed,
    /// Secure Boot not enabled
    NotEnabled,
    /// Invalid signature format
    InvalidSignatureFormat,
    /// Hash verification failed
    HashVerificationFailed,
}

impl fmt::Display for SecureBootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecureBootError::SignatureVerificationFailed => write!(f, "Signature verification failed"),
            SecureBootError::CertificateValidationFailed => write!(f, "Certificate validation failed"),
            SecureBootError::InvalidCertificateChain => write!(f, "Invalid certificate chain"),
            SecureBootError::CertificateExpired => write!(f, "Certificate expired"),
            SecureBootError::CertificateRevoked => write!(f, "Certificate revoked"),
            SecureBootError::KeyNotFound => write!(f, "Key not found in signature database"),
            SecureBootError::KeyInForbiddenList => write!(f, "Key found in forbidden list"),
            SecureBootError::TpmError(msg) => write!(f, "TPM error: {}", msg),
            SecureBootError::InvalidState => write!(f, "Invalid Secure Boot state"),
            SecureBootError::DatabaseCorrupted => write!(f, "Signature database corrupted"),
            SecureBootError::ModuleVerificationFailed => write!(f, "Module signature verification failed"),
            SecureBootError::MeasurementFailed => write!(f, "Boot measurement failed"),
            SecureBootError::RecoveryFailed => write!(f, "Secure Boot recovery failed"),
            SecureBootError::NotEnabled => write!(f, "Secure Boot is not enabled"),
            SecureBootError::InvalidSignatureFormat => write!(f, "Invalid signature format"),
            SecureBootError::HashVerificationFailed => write!(f, "Hash verification failed"),
        }
    }
}

/// Key management errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    /// Key not found
    KeyNotFound,
    /// Key already exists
    KeyAlreadyExists,
    /// Invalid key type
    InvalidKeyType,
    /// Invalid key data
    InvalidKeyData,
    /// Key revocation failed
    KeyRevocationFailed,
    /// Permission denied for key operation
    PermissionDenied,
    /// Key quota exceeded
    QuotaExceeded,
    /// Key payload too large
    PayloadTooLarge,
    /// HSM operation failed
    HsmError(String),
    /// TPM operation failed
    TpmError(String),
    /// Key authentication failed
    AuthenticationFailed,
    /// Invalid key description
    InvalidDescription,
    /// Keyring not found
    KeyringNotFound,
    /// Keyring full
    KeyringFull,
    /// Invalid key ID
    InvalidKeyId,
    /// Key expired
    KeyExpired,
    /// Key revoked
    KeyRevoked,
    /// Invalid key permissions
    InvalidPermissions,
    /// Operation not supported
    NotSupported,
}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyError::KeyNotFound => write!(f, "Key not found"),
            KeyError::KeyAlreadyExists => write!(f, "Key already exists"),
            KeyError::InvalidKeyType => write!(f, "Invalid key type"),
            KeyError::InvalidKeyData => write!(f, "Invalid key data"),
            KeyError::KeyRevocationFailed => write!(f, "Key revocation failed"),
            KeyError::PermissionDenied => write!(f, "Permission denied for key operation"),
            KeyError::QuotaExceeded => write!(f, "Key quota exceeded"),
            KeyError::PayloadTooLarge => write!(f, "Key payload too large"),
            KeyError::HsmError(msg) => write!(f, "HSM error: {}", msg),
            KeyError::TpmError(msg) => write!(f, "TPM error: {}", msg),
            KeyError::AuthenticationFailed => write!(f, "Key authentication failed"),
            KeyError::InvalidDescription => write!(f, "Invalid key description"),
            KeyError::KeyringNotFound => write!(f, "Keyring not found"),
            KeyError::KeyringFull => write!(f, "Keyring is full"),
            KeyError::InvalidKeyId => write!(f, "Invalid key ID"),
            KeyError::KeyExpired => write!(f, "Key has expired"),
            KeyError::KeyRevoked => write!(f, "Key has been revoked"),
            KeyError::InvalidPermissions => write!(f, "Invalid key permissions"),
            KeyError::NotSupported => write!(f, "Key operation not supported"),
        }
    }
}

/// Audit subsystem errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditError {
    /// Audit log initialization failed
    InitFailed,
    /// Failed to log audit event
    LoggingFailed,
    /// Audit rule already exists
    RuleAlreadyExists,
    /// Audit rule not found
    RuleNotFound,
    /// Invalid audit rule
    InvalidRule,
    /// Audit log persistence failed
    PersistenceFailed,
    /// Audit log corruption detected
    LogCorrupted,
    /// Audit buffer overflow
    BufferOverflow,
    /// Invalid audit filter
    InvalidFilter,
    /// Audit daemon not running
    DaemonNotRunning,
    /// Permission denied for audit operation
    PermissionDenied,
    /// Audit configuration invalid
    InvalidConfiguration,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Audit notification failed
    NotificationFailed,
}

impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditError::InitFailed => write!(f, "Audit log initialization failed"),
            AuditError::LoggingFailed => write!(f, "Failed to log audit event"),
            AuditError::RuleAlreadyExists => write!(f, "Audit rule already exists"),
            AuditError::RuleNotFound => write!(f, "Audit rule not found"),
            AuditError::InvalidRule => write!(f, "Invalid audit rule"),
            AuditError::PersistenceFailed => write!(f, "Audit log persistence failed"),
            AuditError::LogCorrupted => write!(f, "Audit log corruption detected"),
            AuditError::BufferOverflow => write!(f, "Audit buffer overflow"),
            AuditError::InvalidFilter => write!(f, "Invalid audit filter"),
            AuditError::DaemonNotRunning => write!(f, "Audit daemon not running"),
            AuditError::PermissionDenied => write!(f, "Permission denied for audit operation"),
            AuditError::InvalidConfiguration => write!(f, "Invalid audit configuration"),
            AuditError::RateLimitExceeded => write!(f, "Audit rate limit exceeded"),
            AuditError::NotificationFailed => write!(f, "Audit notification failed"),
        }
    }
}

/// MAC (Mandatory Access Control) errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacError {
    /// Policy loading failed
    PolicyLoadFailed,
    /// Policy compilation failed
    PolicyCompilationFailed,
    /// Policy not found
    PolicyNotFound,
    /// Invalid policy format
    InvalidPolicyFormat,
    /// Policy already loaded
    PolicyAlreadyLoaded,
    /// Access denied by MAC policy
    AccessDenied,
    /// Subject not found
    SubjectNotFound,
    /// Object not found
    ObjectNotFound,
    /// Invalid security label
    InvalidLabel,
    /// Label not found
    LabelNotFound,
    /// Policy validation failed
    ValidationFailed,
    /// Context not found
    ContextNotFound,
    /// Permission denied for policy operation
    PermissionDenied,
    /// Multiple policy conflict
    PolicyConflict,
    /// Transition denied
    TransitionDenied,
    /// Type enforcement violation
    TypeEnforcementViolation,
    /// Invalid security context
    InvalidContext,
    /// Operation not supported
    NotSupported,
}

impl fmt::Display for MacError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MacError::PolicyLoadFailed => write!(f, "MAC policy loading failed"),
            MacError::PolicyCompilationFailed => write!(f, "MAC policy compilation failed"),
            MacError::PolicyNotFound => write!(f, "MAC policy not found"),
            MacError::InvalidPolicyFormat => write!(f, "Invalid MAC policy format"),
            MacError::PolicyAlreadyLoaded => write!(f, "MAC policy already loaded"),
            MacError::AccessDenied => write!(f, "Access denied by MAC policy"),
            MacError::SubjectNotFound => write!(f, "MAC subject not found"),
            MacError::ObjectNotFound => write!(f, "MAC object not found"),
            MacError::InvalidLabel => write!(f, "Invalid security label"),
            MacError::LabelNotFound => write!(f, "Security label not found"),
            MacError::ValidationFailed => write!(f, "MAC policy validation failed"),
            MacError::ContextNotFound => write!(f, "MAC context not found"),
            MacError::PermissionDenied => write!(f, "Permission denied for MAC policy operation"),
            MacError::PolicyConflict => write!(f, "Multiple MAC policy conflict"),
            MacError::TransitionDenied => write!(f, "Security transition denied"),
            MacError::TypeEnforcementViolation => write!(f, "Type enforcement violation"),
            MacError::InvalidContext => write!(f, "Invalid security context"),
            MacError::NotSupported => write!(f, "MAC operation not supported"),
        }
    }
}

/// Cluster-related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterError {
    /// Distributed lock manager errors
    DlmError(String),
    
    /// Consensus errors
    ConsensusError(String),
    
    /// Failover errors
    FailoverError(String),
    
    /// Membership errors
    MembershipError(String),
    
    /// Load balancing errors
    BalanceError(String),
    
    /// Node not found in cluster
    NodeNotFound(u64),
    
    /// Cluster not initialized
    NotInitialized,
    
    /// Cluster is shutting down
    ShuttingDown,
    
    /// RPC communication failure
    RpcFailed(String),
    
    /// Operation timeout
    Timeout,
    
    /// Configuration error
    ConfigurationError(String),
    
    /// Network partition detected
    NetworkPartition,
    
    /// No quorum available
    NoQuorum,
    
    /// Leadership lost during operation
    LeadershipLost,
    
    /// Split-brain condition detected
    SplitBrain,
    
    /// Fencing operation failed
    FencingFailed {
        node_id: u64,
        reason: String,
    },
    
    /// State synchronization failed
    SyncFailed(String),
    
    /// Cluster is partitioned
    Partitioned,
    
    /// Invalid cluster state
    InvalidState(String),
    
    /// Permission denied for cluster operation
    PermissionDenied,
}

impl fmt::Display for ClusterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClusterError::DlmError(msg) => write!(f, "Distributed lock manager error: {}", msg),
            ClusterError::ConsensusError(msg) => write!(f, "Consensus error: {}", msg),
            ClusterError::FailoverError(msg) => write!(f, "Failover error: {}", msg),
            ClusterError::MembershipError(msg) => write!(f, "Membership error: {}", msg),
            ClusterError::BalanceError(msg) => write!(f, "Load balancing error: {}", msg),
            ClusterError::NodeNotFound(node_id) => write!(f, "Node {} not found in cluster", node_id),
            ClusterError::NotInitialized => write!(f, "Cluster not initialized"),
            ClusterError::ShuttingDown => write!(f, "Cluster is shutting down"),
            ClusterError::RpcFailed(msg) => write!(f, "RPC failed: {}", msg),
            ClusterError::Timeout => write!(f, "Cluster operation timeout"),
            ClusterError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            ClusterError::NetworkPartition => write!(f, "Network partition detected"),
            ClusterError::NoQuorum => write!(f, "No quorum available"),
            ClusterError::LeadershipLost => write!(f, "Leadership lost during operation"),
            ClusterError::SplitBrain => write!(f, "Split-brain condition detected"),
            ClusterError::FencingFailed { node_id, reason } => {
                write!(f, "Fencing failed for node {}: {}", node_id, reason)
            }
            ClusterError::SyncFailed(msg) => write!(f, "State synchronization failed: {}", msg),
            ClusterError::Partitioned => write!(f, "Cluster is partitioned"),
            ClusterError::InvalidState(msg) => write!(f, "Invalid cluster state: {}", msg),
            ClusterError::PermissionDenied => write!(f, "Permission denied for cluster operation"),
        }
    }
}

impl ClusterError {
    /// Convert to POSIX errno
    pub fn to_errno(&self) -> i32 {
        match self {
            ClusterError::DlmError(_) => crate::reliability::errno::EIO,
            ClusterError::ConsensusError(_) => crate::reliability::errno::EIO,
            ClusterError::FailoverError(_) => crate::reliability::errno::EIO,
            ClusterError::MembershipError(_) => crate::reliability::errno::EIO,
            ClusterError::BalanceError(_) => crate::reliability::errno::EIO,
            ClusterError::NodeNotFound(_) => crate::reliability::errno::ENOENT,
            ClusterError::NotInitialized => crate::reliability::errno::EIO,
            ClusterError::ShuttingDown => crate::reliability::errno::EIO,
            ClusterError::RpcFailed(_) => crate::reliability::errno::EIO,
            ClusterError::Timeout => crate::reliability::errno::ETIMEDOUT,
            ClusterError::ConfigurationError(_) => crate::reliability::errno::EINVAL,
            ClusterError::NetworkPartition => crate::reliability::errno::EIO,
            ClusterError::NoQuorum => crate::reliability::errno::EIO,
            ClusterError::LeadershipLost => crate::reliability::errno::EIO,
            ClusterError::SplitBrain => crate::reliability::errno::EIO,
            ClusterError::FencingFailed { .. } => crate::reliability::errno::EIO,
            ClusterError::SyncFailed(_) => crate::reliability::errno::EIO,
            ClusterError::Partitioned => crate::reliability::errno::EIO,
            ClusterError::InvalidState(_) => crate::reliability::errno::EINVAL,
            ClusterError::PermissionDenied => crate::reliability::errno::EPERM,
        }
    }
}

/// SDN (Software Defined Networking) related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdnError {
    OpenFlowProtocolError,
    ControllerConnectionFailed,
    ControllerTimeout,
    FlowTableFull,
    FlowNotFound,
    InvalidFlowRule,
    FlowMatchFailed,
    FlowActionFailed,
    SwitchNotFound,
    PortNotFound,
    InvalidDatapathId,
    StatisticsError,
    BarrierFailed,
    GroupTableFull,
    MeterTableFull,
    PacketInQueueFull,
}

impl fmt::Display for SdnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SdnError::OpenFlowProtocolError => write!(f, "OpenFlow protocol error"),
            SdnError::ControllerConnectionFailed => write!(f, "Controller connection failed"),
            SdnError::ControllerTimeout => write!(f, "Controller timeout"),
            SdnError::FlowTableFull => write!(f, "Flow table is full"),
            SdnError::FlowNotFound => write!(f, "Flow not found"),
            SdnError::InvalidFlowRule => write!(f, "Invalid flow rule"),
            SdnError::FlowMatchFailed => write!(f, "Flow match failed"),
            SdnError::FlowActionFailed => write!(f, "Flow action failed"),
            SdnError::SwitchNotFound => write!(f, "Switch not found"),
            SdnError::PortNotFound => write!(f, "Port not found"),
            SdnError::InvalidDatapathId => write!(f, "Invalid datapath ID"),
            SdnError::StatisticsError => write!(f, "Statistics error"),
            SdnError::BarrierFailed => write!(f, "Barrier failed"),
            SdnError::GroupTableFull => write!(f, "Group table is full"),
            SdnError::MeterTableFull => write!(f, "Meter table is full"),
            SdnError::PacketInQueueFull => write!(f, "Packet-in queue is full"),
        }
    }
}

impl SdnError {
    pub fn to_errno(&self) -> i32 {
        match self {
            SdnError::FlowTableFull | SdnError::GroupTableFull | SdnError::MeterTableFull | 
            SdnError::PacketInQueueFull => crate::reliability::errno::ENOMEM,
            SdnError::FlowNotFound | SdnError::SwitchNotFound | SdnError::PortNotFound => {
                crate::reliability::errno::ENOENT
            },
            SdnError::InvalidFlowRule | SdnError::InvalidDatapathId => crate::reliability::errno::EINVAL,
            SdnError::ControllerConnectionFailed | SdnError::ControllerTimeout => {
                crate::reliability::errno::ETIMEDOUT
            },
            _ => crate::reliability::errno::EIO,
        }
    }
}

/// VXLAN related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VxlanError {
    InvalidVni,
    VniInUse,
    VtepNotFound,
    InvalidRemoteAddress,
    MulticastGroupError,
    SocketCreationFailed,
    PacketTooLarge,
    InvalidVxlanHeader,
    DecapsulationFailed,
    EncapsulationFailed,
    VtepTableFull,
    InterfaceNotFound,
}

impl fmt::Display for VxlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VxlanError::InvalidVni => write!(f, "Invalid VNI"),
            VxlanError::VniInUse => write!(f, "VNI already in use"),
            VxlanError::VtepNotFound => write!(f, "VTEP not found"),
            VxlanError::InvalidRemoteAddress => write!(f, "Invalid remote address"),
            VxlanError::MulticastGroupError => write!(f, "Multicast group error"),
            VxlanError::SocketCreationFailed => write!(f, "Socket creation failed"),
            VxlanError::PacketTooLarge => write!(f, "Packet too large"),
            VxlanError::InvalidVxlanHeader => write!(f, "Invalid VXLAN header"),
            VxlanError::DecapsulationFailed => write!(f, "Decapsulation failed"),
            VxlanError::EncapsulationFailed => write!(f, "Encapsulation failed"),
            VxlanError::VtepTableFull => write!(f, "VTEP table is full"),
            VxlanError::InterfaceNotFound => write!(f, "Interface not found"),
        }
    }
}

impl VxlanError {
    pub fn to_errno(&self) -> i32 {
        match self {
            VxlanError::InvalidVni | VxlanError::InvalidRemoteAddress | VxlanError::InvalidVxlanHeader => {
                crate::reliability::errno::EINVAL
            },
            VxlanError::VniInUse => crate::reliability::errno::EEXIST,
            VxlanError::VtepNotFound | VxlanError::InterfaceNotFound => crate::reliability::errno::ENOENT,
            VxlanError::VtepTableFull => crate::reliability::errno::ENOMEM,
            VxlanError::PacketTooLarge => crate::reliability::errno::EMSGSIZE,
            _ => crate::reliability::errno::EIO,
        }
    }
}

/// Network bridge related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetBridgeError {
    BridgeNotFound,
    BridgeAlreadyExists,
    PortNotFound,
    PortAlreadyExists,
    PortNotInBridge,
    TooManyPorts,
    InvalidPortConfiguration,
    MacTableFull,
    MacTableError,
    FdbFailed,
    BridgeNotOperational,
    VlanError,
}

impl fmt::Display for NetBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetBridgeError::BridgeNotFound => write!(f, "Bridge not found"),
            NetBridgeError::BridgeAlreadyExists => write!(f, "Bridge already exists"),
            NetBridgeError::PortNotFound => write!(f, "Port not found"),
            NetBridgeError::PortAlreadyExists => write!(f, "Port already exists"),
            NetBridgeError::PortNotInBridge => write!(f, "Port not in bridge"),
            NetBridgeError::TooManyPorts => write!(f, "Too many ports"),
            NetBridgeError::InvalidPortConfiguration => write!(f, "Invalid port configuration"),
            NetBridgeError::MacTableFull => write!(f, "MAC table is full"),
            NetBridgeError::MacTableError => write!(f, "MAC table error"),
            NetBridgeError::FdbFailed => write!(f, "FDB operation failed"),
            NetBridgeError::BridgeNotOperational => write!(f, "Bridge not operational"),
            NetBridgeError::VlanError => write!(f, "VLAN error"),
        }
    }
}

impl NetBridgeError {
    pub fn to_errno(&self) -> i32 {
        match self {
            NetBridgeError::BridgeNotFound | NetBridgeError::PortNotFound => crate::reliability::errno::ENOENT,
            NetBridgeError::BridgeAlreadyExists | NetBridgeError::PortAlreadyExists => {
                crate::reliability::errno::EEXIST
            },
            NetBridgeError::MacTableFull => crate::reliability::errno::ENOMEM,
            NetBridgeError::TooManyPorts => crate::reliability::errno::EAGAIN,
            NetBridgeError::InvalidPortConfiguration => crate::reliability::errno::EINVAL,
            _ => crate::reliability::errno::EIO,
        }
    }
}

/// QoS related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QosError {
    QdiscNotFound,
    QdiscAlreadyExists,
    ClassNotFound,
    ClassAlreadyExists,
    FilterNotFound,
    FilterAlreadyExists,
    InvalidQdiscType,
    InvalidClassId,
    InvalidFilterType,
    RateLimitExceeded,
    PriorityError,
    HtbConfigurationError,
    HfscConfigurationError,
    PolicingFailed,
    ShapingFailed,
    RedConfigurationError,
    QosTableFull,
}

impl fmt::Display for QosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QosError::QdiscNotFound => write!(f, "Qdisc not found"),
            QosError::QdiscAlreadyExists => write!(f, "Qdisc already exists"),
            QosError::ClassNotFound => write!(f, "Class not found"),
            QosError::ClassAlreadyExists => write!(f, "Class already exists"),
            QosError::FilterNotFound => write!(f, "Filter not found"),
            QosError::FilterAlreadyExists => write!(f, "Filter already exists"),
            QosError::InvalidQdiscType => write!(f, "Invalid qdisc type"),
            QosError::InvalidClassId => write!(f, "Invalid class ID"),
            QosError::InvalidFilterType => write!(f, "Invalid filter type"),
            QosError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            QosError::PriorityError => write!(f, "Priority error"),
            QosError::HtbConfigurationError => write!(f, "HTB configuration error"),
            QosError::HfscConfigurationError => write!(f, "HFSC configuration error"),
            QosError::PolicingFailed => write!(f, "Policing failed"),
            QosError::ShapingFailed => write!(f, "Traffic shaping failed"),
            QosError::RedConfigurationError => write!(f, "RED configuration error"),
            QosError::QosTableFull => write!(f, "QoS table is full"),
        }
    }
}

impl QosError {
    pub fn to_errno(&self) -> i32 {
        match self {
            QosError::QdiscNotFound | QosError::ClassNotFound | QosError::FilterNotFound => {
                crate::reliability::errno::ENOENT
            },
            QosError::QdiscAlreadyExists | QosError::ClassAlreadyExists | QosError::FilterAlreadyExists => {
                crate::reliability::errno::EEXIST
            },
            QosError::InvalidQdiscType | QosError::InvalidClassId | QosError::InvalidFilterType => {
                crate::reliability::errno::EINVAL
            },
            QosError::QosTableFull => crate::reliability::errno::ENOMEM,
            QosError::RateLimitExceeded => crate::reliability::errno::EBUSY,
            _ => crate::reliability::errno::EIO,
        }
    }
}

/// Connection tracking related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConntrackError {
    ConnTableFull,
    InvalidConnTuple,
    ConnNotFound,
    InvalidConnState,
    NatFailed,
    NatTableFull,
    ExpectationTableFull,
    HelperFailed,
    TimeoutError,
    SynchronizationFailed,
    InvalidProtocol,
    TupleMismatch,
    ConnExpired,
    InvalidNatType,
}

impl fmt::Display for ConntrackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConntrackError::ConnTableFull => write!(f, "Connection table is full"),
            ConntrackError::InvalidConnTuple => write!(f, "Invalid connection tuple"),
            ConntrackError::ConnNotFound => write!(f, "Connection not found"),
            ConntrackError::InvalidConnState => write!(f, "Invalid connection state"),
            ConntrackError::NatFailed => write!(f, "NAT operation failed"),
            ConntrackError::NatTableFull => write!(f, "NAT table is full"),
            ConntrackError::ExpectationTableFull => write!(f, "Expectation table is full"),
            ConntrackError::HelperFailed => write!(f, "Helper failed"),
            ConntrackError::TimeoutError => write!(f, "Connection timeout"),
            ConntrackError::SynchronizationFailed => write!(f, "Synchronization failed"),
            ConntrackError::InvalidProtocol => write!(f, "Invalid protocol"),
            ConntrackError::TupleMismatch => write!(f, "Tuple mismatch"),
            ConntrackError::ConnExpired => write!(f, "Connection expired"),
            ConntrackError::InvalidNatType => write!(f, "Invalid NAT type"),
        }
    }
}

impl ConntrackError {
    pub fn to_errno(&self) -> i32 {
        match self {
            ConntrackError::ConnTableFull | ConntrackError::NatTableFull | 
            ConntrackError::ExpectationTableFull => crate::reliability::errno::ENOMEM,
            ConntrackError::ConnNotFound => crate::reliability::errno::ENOENT,
            ConntrackError::InvalidConnTuple | ConntrackError::InvalidConnState | 
            ConntrackError::InvalidProtocol | ConntrackError::TupleMismatch | 
            ConntrackError::InvalidNatType => crate::reliability::errno::EINVAL,
            ConntrackError::TimeoutError => crate::reliability::errno::ETIMEDOUT,
            _ => crate::reliability::errno::EIO,
        }
    }
}

/// Load balancer related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LbError {
    VipNotFound,
    VipAlreadyExists,
    BackendNotFound,
    BackendAlreadyExists,
    BackendUnavailable,
    HealthCheckFailed,
    InvalidBackendConfiguration,
    NoHealthyBackends,
    LoadBalancingAlgorithmError,
    SessionPersistenceError,
    SslTerminationFailed,
    ConnectionDrainingFailed,
    TooManyBackends,
    InvalidHealthCheckConfig,
}

impl fmt::Display for LbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LbError::VipNotFound => write!(f, "VIP not found"),
            LbError::VipAlreadyExists => write!(f, "VIP already exists"),
            LbError::BackendNotFound => write!(f, "Backend not found"),
            LbError::BackendAlreadyExists => write!(f, "Backend already exists"),
            LbError::BackendUnavailable => write!(f, "Backend unavailable"),
            LbError::HealthCheckFailed => write!(f, "Health check failed"),
            LbError::InvalidBackendConfiguration => write!(f, "Invalid backend configuration"),
            LbError::NoHealthyBackends => write!(f, "No healthy backends"),
            LbError::LoadBalancingAlgorithmError => write!(f, "Load balancing algorithm error"),
            LbError::SessionPersistenceError => write!(f, "Session persistence error"),
            LbError::SslTerminationFailed => write!(f, "SSL termination failed"),
            LbError::ConnectionDrainingFailed => write!(f, "Connection draining failed"),
            LbError::TooManyBackends => write!(f, "Too many backends"),
            LbError::InvalidHealthCheckConfig => write!(f, "Invalid health check configuration"),
        }
    }
}

impl LbError {
    pub fn to_errno(&self) -> i32 {
        match self {
            LbError::VipNotFound | LbError::BackendNotFound => crate::reliability::errno::ENOENT,
            LbError::VipAlreadyExists | LbError::BackendAlreadyExists => crate::reliability::errno::EEXIST,
            LbError::TooManyBackends => crate::reliability::errno::ENOMEM,
            LbError::InvalidBackendConfiguration | LbError::InvalidHealthCheckConfig => {
                crate::reliability::errno::EINVAL
            },
            LbError::NoHealthyBackends => crate::reliability::errno::EAGAIN,
            LbError::BackendUnavailable => crate::reliability::errno::EHOSTUNREACH,
            _ => crate::reliability::errno::EIO,
        }
    }
}

/// Tunnel related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TunnelError {
    TunnelNotFound,
    TunnelAlreadyExists,
    InvalidTunnelType,
    InvalidLocalAddress,
    InvalidRemoteAddress,
    TunnelCreationFailed,
    TunnelDeletionFailed,
    IpsecError,
    GreError,
    IpipError,
    SitError,
    MtuError,
    KeepaliveFailed,
    TunnelNotOperational,
    CryptoError,
    KeyManagementFailed,
}

impl fmt::Display for TunnelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TunnelError::TunnelNotFound => write!(f, "Tunnel not found"),
            TunnelError::TunnelAlreadyExists => write!(f, "Tunnel already exists"),
            TunnelError::InvalidTunnelType => write!(f, "Invalid tunnel type"),
            TunnelError::InvalidLocalAddress => write!(f, "Invalid local address"),
            TunnelError::InvalidRemoteAddress => write!(f, "Invalid remote address"),
            TunnelError::TunnelCreationFailed => write!(f, "Tunnel creation failed"),
            TunnelError::TunnelDeletionFailed => write!(f, "Tunnel deletion failed"),
            TunnelError::IpsecError => write!(f, "IPsec error"),
            TunnelError::GreError => write!(f, "GRE error"),
            TunnelError::IpipError => write!(f, "IPIP error"),
            TunnelError::SitError => write!(f, "SIT error"),
            TunnelError::MtuError => write!(f, "MTU error"),
            TunnelError::KeepaliveFailed => write!(f, "Keepalive failed"),
            TunnelError::TunnelNotOperational => write!(f, "Tunnel not operational"),
            TunnelError::CryptoError => write!(f, "Crypto error"),
            TunnelError::KeyManagementFailed => write!(f, "Key management failed"),
        }
    }
}

impl TunnelError {
    pub fn to_errno(&self) -> i32 {
        match self {
            TunnelError::TunnelNotFound => crate::reliability::errno::ENOENT,
            TunnelError::TunnelAlreadyExists => crate::reliability::errno::EEXIST,
            TunnelError::InvalidTunnelType | TunnelError::InvalidLocalAddress | 
            TunnelError::InvalidRemoteAddress => crate::reliability::errno::EINVAL,
            TunnelError::MtuError => crate::reliability::errno::EMSGSIZE,
            _ => crate::reliability::errno::EIO,
        }
    }
}

/// Benchmarking errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BenchError {
    /// Benchmark execution failed
    ExecutionFailed(String),
    /// Benchmark timeout
    Timeout,
    /// Invalid benchmark configuration
    InvalidConfiguration(String),
    /// Benchmark not found
    BenchmarkNotFound(String),
    /// Suite not found
    SuiteNotFound(String),
    /// Sample collection failed
    SampleCollectionFailed,
    /// Statistical analysis failed
    StatisticalAnalysisFailed,
    /// Comparison failed
    ComparisonFailed(String),
    /// History not available
    HistoryNotAvailable,
    /// Export failed
    ExportFailed(String),
}

impl fmt::Display for BenchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BenchError::ExecutionFailed(msg) => write!(f, "Benchmark execution failed: {}", msg),
            BenchError::Timeout => write!(f, "Benchmark execution timeout"),
            BenchError::InvalidConfiguration(msg) => write!(f, "Invalid benchmark configuration: {}", msg),
            BenchError::BenchmarkNotFound(name) => write!(f, "Benchmark not found: {}", name),
            BenchError::SuiteNotFound(name) => write!(f, "Benchmark suite not found: {}", name),
            BenchError::SampleCollectionFailed => write!(f, "Failed to collect benchmark samples"),
            BenchError::StatisticalAnalysisFailed => write!(f, "Statistical analysis failed"),
            BenchError::ComparisonFailed(msg) => write!(f, "Benchmark comparison failed: {}", msg),
            BenchError::HistoryNotAvailable => write!(f, "Benchmark history not available"),
            BenchError::ExportFailed(msg) => write!(f, "Failed to export benchmark results: {}", msg),
        }
    }
}

/// Profiler errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfError {
    /// Profiler already active
    AlreadyActive,
    /// Profiler not active
    NotActive,
    /// Invalid configuration
    InvalidConfiguration(String),
    /// Memory limit exceeded
    MemoryLimitExceeded,
    /// Buffer overflow
    BufferOverflow,
    /// Invalid handle
    InvalidHandle,
    /// Sampling failed
    SamplingFailed,
    /// Stack trace capture failed
    StackTraceFailed,
    /// Symbol resolution failed
    SymbolResolutionFailed,
}

impl fmt::Display for ProfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProfError::AlreadyActive => write!(f, "Profiler already active"),
            ProfError::NotActive => write!(f, "Profiler not active"),
            ProfError::InvalidConfiguration(msg) => write!(f, "Invalid profiler configuration: {}", msg),
            ProfError::MemoryLimitExceeded => write!(f, "Profiler memory limit exceeded"),
            ProfError::BufferOverflow => write!(f, "Profiler buffer overflow"),
            ProfError::InvalidHandle => write!(f, "Invalid profiler handle"),
            ProfError::SamplingFailed => write!(f, "Profiler sampling failed"),
            ProfError::StackTraceFailed => write!(f, "Failed to capture stack trace"),
            ProfError::SymbolResolutionFailed => write!(f, "Failed to resolve symbols"),
        }
    }
}

/// Metrics collection errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetricsError {
    /// Metric not found
    MetricNotFound(String),
    /// Invalid metric name
    InvalidMetricName(String),
    /// Metric type mismatch
    MetricTypeMismatch(String),
    /// Collection failed
    CollectionFailed(String),
    /// Aggregation failed
    AggregationFailed(String),
    /// Export failed
    ExportFailed(String),
    /// Storage full
    StorageFull,
    /// Invalid metric value
    InvalidValue(String),
}

impl fmt::Display for MetricsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetricsError::MetricNotFound(name) => write!(f, "Metric not found: {}", name),
            MetricsError::InvalidMetricName(name) => write!(f, "Invalid metric name: {}", name),
            MetricsError::MetricTypeMismatch(msg) => write!(f, "Metric type mismatch: {}", msg),
            MetricsError::CollectionFailed(msg) => write!(f, "Metrics collection failed: {}", msg),
            MetricsError::AggregationFailed(msg) => write!(f, "Metrics aggregation failed: {}", msg),
            MetricsError::ExportFailed(msg) => write!(f, "Metrics export failed: {}", msg),
            MetricsError::StorageFull => write!(f, "Metrics storage full"),
            MetricsError::InvalidValue(msg) => write!(f, "Invalid metric value: {}", msg),
        }
    }
}

/// Tracing errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceError {
    /// Trace session not found
    SessionNotFound(u64),
    /// Trace session already active
    SessionAlreadyActive(u64),
    /// Buffer allocation failed
    BufferAllocationFailed,
    /// Buffer overflow
    BufferOverflow,
    /// Invalid trace filter
    InvalidFilter(String),
    /// Event serialization failed
    SerializationFailed,
    /// Event deserialization failed
    DeserializationFailed,
    /// Trace file I/O error
    IoError(String),
}

impl fmt::Display for TraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraceError::SessionNotFound(id) => write!(f, "Trace session not found: {}", id),
            TraceError::SessionAlreadyActive(id) => write!(f, "Trace session already active: {}", id),
            TraceError::BufferAllocationFailed => write!(f, "Failed to allocate trace buffer"),
            TraceError::BufferOverflow => write!(f, "Trace buffer overflow"),
            TraceError::InvalidFilter(msg) => write!(f, "Invalid trace filter: {}", msg),
            TraceError::SerializationFailed => write!(f, "Failed to serialize trace event"),
            TraceError::DeserializationFailed => write!(f, "Failed to deserialize trace event"),
            TraceError::IoError(msg) => write!(f, "Trace I/O error: {}", msg),
        }
    }
}

/// Report generation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportError {
    /// Report generation failed
    GenerationFailed(String),
    /// Data not available
    DataNotAvailable(String),
    /// Analysis failed
    AnalysisFailed(String),
    /// Export failed
    ExportFailed(String),
    /// Invalid report format
    InvalidFormat(String),
    /// Template not found
    TemplateNotFound(String),
    /// Historical data missing
    HistoricalDataMissing,
}

impl fmt::Display for ReportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReportError::GenerationFailed(msg) => write!(f, "Report generation failed: {}", msg),
            ReportError::DataNotAvailable(msg) => write!(f, "Report data not available: {}", msg),
            ReportError::AnalysisFailed(msg) => write!(f, "Report analysis failed: {}", msg),
            ReportError::ExportFailed(msg) => write!(f, "Report export failed: {}", msg),
            ReportError::InvalidFormat(msg) => write!(f, "Invalid report format: {}", msg),
            ReportError::TemplateNotFound(name) => write!(f, "Report template not found: {}", name),
            ReportError::HistoricalDataMissing => write!(f, "Historical data missing for comparison"),
        }
    }
}

/// Memory optimization errors (Track EK)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocError {
    OutOfMemory,
    InvalidSize,
    InvalidAlignment,
    TooFragmented,
    NumaNodeUnavailable,
    AllocationFailed,
}

impl fmt::Display for AllocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AllocError::OutOfMemory => write!(f, "Out of memory"),
            AllocError::InvalidSize => write!(f, "Invalid allocation size"),
            AllocError::InvalidAlignment => write!(f, "Invalid alignment"),
            AllocError::TooFragmented => write!(f, "Memory too fragmented"),
            AllocError::NumaNodeUnavailable => write!(f, "NUMA node unavailable"),
            AllocError::AllocationFailed => write!(f, "Allocation failed"),
        }
    }
}

/// Paging optimization errors (Track EK)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PagingError {
    InvalidPte,
    PageTableAllocFailed,
    TlbFlushFailed,
    PromotionFailed,
    PageWalkError,
    InvalidAddress,
    PermissionDenied,
}

impl fmt::Display for PagingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PagingError::InvalidPte => write!(f, "Invalid page table entry"),
            PagingError::PageTableAllocFailed => write!(f, "Page table allocation failed"),
            PagingError::TlbFlushFailed => write!(f, "TLB flush failed"),
            PagingError::PromotionFailed => write!(f, "Huge page promotion failed"),
            PagingError::PageWalkError => write!(f, "Page walk error"),
            PagingError::InvalidAddress => write!(f, "Invalid address"),
            PagingError::PermissionDenied => write!(f, "Permission denied"),
        }
    }
}

/// Zero page optimization errors (Track EK)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZeroPageError {
    NotInitialized,
    AllocationFailed,
    CowFailed,
    InvalidAddress,
    PermissionDenied,
}

impl fmt::Display for ZeroPageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZeroPageError::NotInitialized => write!(f, "Zero page not initialized"),
            ZeroPageError::AllocationFailed => write!(f, "Page allocation failed"),
            ZeroPageError::CowFailed => write!(f, "Copy-on-write failed"),
            ZeroPageError::InvalidAddress => write!(f, "Invalid address"),
            ZeroPageError::PermissionDenied => write!(f, "Permission denied"),
        }
    }
}

/// Kernel memory errors (Track EK)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmemError {
    OutOfMemory,
    InvalidSize,
    SlabCacheError,
    PerCpuError,
    CgroupLimitExceeded,
    AllocationFailed,
}

impl fmt::Display for KmemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KmemError::OutOfMemory => write!(f, "Out of memory"),
            KmemError::InvalidSize => write!(f, "Invalid allocation size"),
            KmemError::SlabCacheError => write!(f, "Slab cache error"),
            KmemError::PerCpuError => write!(f, "Per-CPU cache error"),
            KmemError::CgroupLimitExceeded => write!(f, "Memory cgroup limit exceeded"),
            KmemError::AllocationFailed => write!(f, "Allocation failed"),
        }
    }
}

/// Memory mapping errors (Track EK)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MmapError {
    InvalidAddress,
    InvalidLength,
    InvalidProtection,
    InvalidFlags,
    PermissionDenied,
    OutOfMemory,
    MappingFailed,
    UnmappingFailed,
}

impl fmt::Display for MmapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MmapError::InvalidAddress => write!(f, "Invalid address"),
            MmapError::InvalidLength => write!(f, "Invalid length"),
            MmapError::InvalidProtection => write!(f, "Invalid protection flags"),
            MmapError::InvalidFlags => write!(f, "Invalid flags"),
            MmapError::PermissionDenied => write!(f, "Permission denied"),
            MmapError::OutOfMemory => write!(f, "Out of memory"),
            MmapError::MappingFailed => write!(f, "Mapping failed"),
            MmapError::UnmappingFailed => write!(f, "Unmapping failed"),
        }
    }
}

/// Memory optimization errors (Track EK)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemOptError {
    NotInitialized,
    OptimizationFailed,
    InvalidPolicy,
    NumaError,
    StatisticsError,
}

impl fmt::Display for MemOptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemOptError::NotInitialized => write!(f, "Memory optimizer not initialized"),
            MemOptError::OptimizationFailed => write!(f, "Optimization failed"),
            MemOptError::InvalidPolicy => write!(f, "Invalid optimization policy"),
            MemOptError::NumaError => write!(f, "NUMA error"),
            MemOptError::StatisticsError => write!(f, "Statistics error"),
        }
    }
}

// ============================================================================
// Concurrency Optimization Errors
// ============================================================================

/// Synchronization primitive errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncError {
    /// Lock operation timed out
    LockTimeout,
    /// Lock acquisition would deadlock
    WouldDeadlock,
    /// Invalid lock state
    InvalidLockState,
    /// Lock is held by another owner
    LockHeld,
    /// Operation not supported on this lock type
    NotSupported,
    /// Futex operation failed
    FutexFailed,
    /// Wait queue overflow
    QueueFull,
    /// Invalid argument
    InvalidArgument,
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::LockTimeout => write!(f, "Lock acquisition timed out"),
            SyncError::WouldDeadlock => write!(f, "Operation would cause deadlock"),
            SyncError::InvalidLockState => write!(f, "Invalid lock state"),
            SyncError::LockHeld => write!(f, "Lock is held by another owner"),
            SyncError::NotSupported => write!(f, "Operation not supported"),
            SyncError::FutexFailed => write!(f, "Futex operation failed"),
            SyncError::QueueFull => write!(f, "Wait queue is full"),
            SyncError::InvalidArgument => write!(f, "Invalid argument"),
        }
    }
}

/// Atomic operation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomicError {
    /// CAS operation failed after max retries
    CasFailed,
    /// Invalid memory ordering for operation
    InvalidOrdering,
    /// Null pointer in lock-free operation
    NullPointer,
    /// Memory allocation failed
    AllocationFailed,
    /// Operation not supported
    NotSupported,
    /// ABA problem detected
    ABAProblem,
    /// Invalid alignment
    InvalidAlignment,
}

impl fmt::Display for AtomicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtomicError::CasFailed => write!(f, "CAS operation failed"),
            AtomicError::InvalidOrdering => write!(f, "Invalid memory ordering"),
            AtomicError::NullPointer => write!(f, "Null pointer"),
            AtomicError::AllocationFailed => write!(f, "Allocation failed"),
            AtomicError::NotSupported => write!(f, "Operation not supported"),
            AtomicError::ABAProblem => write!(f, "ABA problem detected"),
            AtomicError::InvalidAlignment => write!(f, "Invalid alignment"),
        }
    }
}

/// RCU (Read-Copy-Update) errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuError {
    /// Grace period timeout
    GracePeriodTimeout,
    /// Callback queue overflow
    CallbackOverflow,
    /// Invalid callback function
    InvalidCallback,
    /// Not in read-side critical section
    NotInReadSection,
    /// Already in read-side critical section
    AlreadyInReadSection,
    /// CPU stall detected
    CpuStall,
    /// RCU not initialized
    NotInitialized,
    /// Invalid state
    InvalidState,
}

impl fmt::Display for RcuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RcuError::GracePeriodTimeout => write!(f, "Grace period timed out"),
            RcuError::CallbackOverflow => write!(f, "Callback queue overflow"),
            RcuError::InvalidCallback => write!(f, "Invalid callback function"),
            RcuError::NotInReadSection => write!(f, "Not in read-side critical section"),
            RcuError::AlreadyInReadSection => write!(f, "Already in read-side critical section"),
            RcuError::CpuStall => write!(f, "CPU stall detected"),
            RcuError::NotInitialized => write!(f, "RCU not initialized"),
            RcuError::InvalidState => write!(f, "Invalid RCU state"),
        }
    }
}

/// Parallel execution errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelError {
    /// Workqueue is full
    QueueFull,
    /// Thread pool exhausted
    NoThreads,
    /// Invalid CPU mask
    InvalidCpuMask,
    /// CPU not available
    CpuNotAvailable,
    /// Work item too large
    ItemTooLarge,
    /// Affinity set failed
    AffinityFailed,
    /// Workqueue not found
    WorkqueueNotFound,
    /// Operation not supported
    NotSupported,
    /// Invalid argument
    InvalidArgument,
    /// Thread creation failed
    ThreadCreateFailed,
}

impl fmt::Display for ParallelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParallelError::QueueFull => write!(f, "Workqueue is full"),
            ParallelError::NoThreads => write!(f, "No available threads"),
            ParallelError::InvalidCpuMask => write!(f, "Invalid CPU mask"),
            ParallelError::CpuNotAvailable => write!(f, "CPU not available"),
            ParallelError::ItemTooLarge => write!(f, "Work item too large"),
            ParallelError::AffinityFailed => write!(f, "Failed to set CPU affinity"),
            ParallelError::WorkqueueNotFound => write!(f, "Workqueue not found"),
            ParallelError::NotSupported => write!(f, "Operation not supported"),
            ParallelError::InvalidArgument => write!(f, "Invalid argument"),
            ParallelError::ThreadCreateFailed => write!(f, "Failed to create thread"),
        }
    }
}

/// Concurrency management errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcurrencyError {
    /// Deadlock detected
    Deadlock,
    /// Lock not registered
    LockNotRegistered,
    /// Invalid lock ID
    InvalidLockId,
    /// Dependency cycle detected
    DependencyCycle,
    /// Contention limit exceeded
    ContentionLimitExceeded,
    /// Statistics not available
    StatsNotAvailable,
    /// Operation not supported
    NotSupported,
}

impl fmt::Display for ConcurrencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConcurrencyError::Deadlock => write!(f, "Deadlock detected"),
            ConcurrencyError::LockNotRegistered => write!(f, "Lock not registered"),
            ConcurrencyError::InvalidLockId => write!(f, "Invalid lock ID"),
            ConcurrencyError::DependencyCycle => write!(f, "Dependency cycle detected"),
            ConcurrencyError::ContentionLimitExceeded => write!(f, "Contention limit exceeded"),
            ConcurrencyError::StatsNotAvailable => write!(f, "Statistics not available"),
            ConcurrencyError::NotSupported => write!(f, "Operation not supported"),
        }
    }
}
