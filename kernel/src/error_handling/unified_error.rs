//! Unified Error Handling System
//!
//! 统一错误处理系统
//! 提供统一的错误类型、错误上下文和错误处理机制

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use core::any::TypeId;

use super::*;
use super::unified::{KernelError, KernelResult};
use crate::syscalls::common::SyscallError;
use super::error_classifier::{ErrorClassification, ClassificationType, Urgency, RecoveryComplexity};
use crate::compat::DefaultHasherBuilder;

/// 统一错误类型 - 所有错误的基础类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnifiedError {
    /// 系统调用错误
    Syscall(SyscallError),
    
    /// 内存错误
    Memory(MemoryError),
    
    /// 文件系统错误
    FileSystem(FileSystemError),
    
    /// 网络错误
    Network(NetworkError),
    
    /// 进程错误
    Process(ProcessError),
    
    /// 设备错误
    Device(DeviceError),
    
    /// 安全错误
    Security(SecurityError),
    
    /// 配置错误
    Configuration(ConfigurationError),
    
    /// 硬件错误
    Hardware(HardwareError),
    
    /// 超时错误
    Timeout(TimeoutError),
    
    /// 数据错误
    Data(DataError),
    
    /// 协议错误
    Protocol(ProtocolError),
    
    /// 资源错误
    Resource(ResourceError),
    
    /// 用户错误
    User(UserError),
    
    /// 接口错误
    Interface(InterfaceError),
    
    /// 未知错误
    Unknown(UnknownError),
}

/// 统一结果类型
pub type UnifiedResult<T> = Result<T, UnifiedError>;

/// 内存错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryError {
    /// 内存不足
    OutOfMemory,
    /// 无效地址
    InvalidAddress,
    /// 地址对齐错误
    AlignmentError,
    /// 内存保护错误
    ProtectionError,
    /// 内存碎片
    FragmentationError,
    /// 内存泄漏
    MemoryLeak,
    /// 缓冲区溢出
    BufferOverflow,
    /// 双重释放
    DoubleFree,
    /// 使用已释放内存
    UseAfterFree,
}

/// 文件系统错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSystemError {
    /// 文件未找到
    FileNotFound,
    /// 权限被拒绝
    PermissionDenied,
    /// 文件已存在
    FileExists,
    /// 不是目录
    NotADirectory,
    /// 是目录
    IsADirectory,
    /// 目录不为空
    DirectoryNotEmpty,
    /// 磁盘空间不足
    DiskFull,
    /// 文件系统损坏
    Corruption,
    /// 文件系统只读
    ReadOnly,
    /// 无效路径
    InvalidPath,
    /// 跨设备链接
    CrossDeviceLink,
    /// 文件名过长
    NameTooLong,
    /// 符号链接过多
    TooManySymlinks,
    /// 文件系统忙
    FileSystemBusy,
}

/// 网络错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkError {
    /// 连接被拒绝
    ConnectionRefused,
    /// 连接超时
    ConnectionTimeout,
    /// 连接重置
    ConnectionReset,
    /// 主机不可达
    HostUnreachable,
    /// 网络不可达
    NetworkUnreachable,
    /// 地址已在使用
    AddressInUse,
    /// 地址不可用
    AddressNotAvailable,
    /// 协议错误
    ProtocolError,
    /// 数据包损坏
    PacketCorruption,
    /// 网络超时
    NetworkTimeout,
    /// 带宽限制
    BandwidthLimit,
    /// 连接数限制
    ConnectionLimit,
}

/// 进程错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessError {
    /// 进程未找到
    ProcessNotFound,
    /// 权限被拒绝
    PermissionDenied,
    /// 资源限制
    ResourceLimit,
    /// 进程已存在
    ProcessExists,
    /// 进程已终止
    ProcessTerminated,
    /// 进程挂起
    ProcessSuspended,
    /// 进程状态错误
    InvalidProcessState,
    /// 进程内存错误
    ProcessMemoryError,
    /// 进程调度错误
    ProcessSchedulingError,
    /// 进程通信错误
    ProcessCommunicationError,
}

/// 设备错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceError {
    /// 设备未找到
    DeviceNotFound,
    /// 设备忙
    DeviceBusy,
    /// 设备不可用
    DeviceUnavailable,
    /// 设备错误
    DeviceError,
    /// 设备超时
    DeviceTimeout,
    /// 设备配置错误
    DeviceConfigurationError,
    /// 设备驱动错误
    DeviceDriverError,
    /// 设备固件错误
    DeviceFirmwareError,
    /// 设备硬件错误
    DeviceHardwareError,
}

/// 安全错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    /// 认证失败
    AuthenticationFailed,
    /// 授权失败
    AuthorizationFailed,
    /// 权限被拒绝
    PermissionDenied,
    /// 访问被拒绝
    AccessDenied,
    /// 安全策略违反
    SecurityPolicyViolation,
    /// 加密错误
    EncryptionError,
    /// 解密错误
    DecryptionError,
    /// 签名验证失败
    SignatureVerificationFailed,
    /// 证书错误
    CertificateError,
    /// 安全上下文错误
    SecurityContextError,
}

/// 配置错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigurationError {
    /// 配置文件未找到
    ConfigFileNotFound,
    /// 配置文件格式错误
    ConfigFileFormatError,
    /// 配置项未找到
    ConfigItemNotFound,
    /// 无效配置值
    InvalidConfigValue,
    /// 配置项冲突
    ConfigConflict,
    /// 配置验证失败
    ConfigValidationFailed,
    /// 配置权限错误
    ConfigPermissionError,
    /// 配置版本不兼容
    ConfigVersionIncompatible,
    /// 配置依赖错误
    ConfigDependencyError,
}

/// 硬件错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardwareError {
    /// 硬件故障
    HardwareFailure,
    /// 硬件超时
    HardwareTimeout,
    /// 硬件配置错误
    HardwareConfigurationError,
    /// 硬件兼容性问题
    HardwareCompatibilityIssue,
    /// 硬件资源冲突
    HardwareResourceConflict,
    /// 硬件驱动问题
    HardwareDriverIssue,
    /// 硬件固件问题
    HardwareFirmwareIssue,
    /// 硬件过热
    HardwareOverheating,
    /// 硬件电源问题
    HardwarePowerIssue,
}

/// 超时错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeoutError {
    /// 操作超时
    OperationTimeout,
    /// 连接超时
    ConnectionTimeout,
    /// 读取超时
    ReadTimeout,
    /// 写入超时
    WriteTimeout,
    /// 等待超时
    WaitTimeout,
    /// 锁超时
    LockTimeout,
    /// 响应超时
    ResponseTimeout,
    /// 系统调用超时
    SystemCallTimeout,
    /// 初始化超时
    InitializationTimeout,
}

/// 数据错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataError {
    /// 数据损坏
    DataCorruption,
    /// 数据格式错误
    DataFormatError,
    /// 数据验证失败
    DataValidationFailed,
    /// 数据编码错误
    DataEncodingError,
    /// 数据解码错误
    DataDecodingError,
    /// 数据截断
    DataTruncation,
    /// 数据溢出
    DataOverflow,
    /// 数据类型不匹配
    DataTypeMismatch,
    /// 数据版本不兼容
    DataVersionIncompatible,
}

/// 协议错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    /// 协议不支持的版本
    UnsupportedProtocolVersion,
    /// 协议错误
    ProtocolError,
    /// 协议违反
    ProtocolViolation,
    /// 协议超时
    ProtocolTimeout,
    /// 协议协商失败
    ProtocolNegotiationFailed,
    /// 协议状态错误
    ProtocolStateError,
    /// 协议消息错误
    ProtocolMessageError,
    /// 协议头错误
    ProtocolHeaderError,
    /// 协议校验和错误
    ProtocolChecksumError,
}

/// 资源错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceError {
    /// 资源不足
    ResourceExhausted,
    /// 资源未找到
    ResourceNotFound,
    /// 资源忙
    ResourceBusy,
    /// 资源不可用
    ResourceUnavailable,
    /// 资源配额超出
    ResourceQuotaExceeded,
    /// 资源权限错误
    ResourcePermissionError,
    /// 资源状态错误
    ResourceStateError,
    /// 资源依赖错误
    ResourceDependencyError,
    /// 资源配置错误
    ResourceConfigurationError,
}

/// 用户错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserError {
    /// 无效用户输入
    InvalidInput,
    /// 用户权限不足
    InsufficientPrivileges,
    /// 用户认证失败
    UserAuthenticationFailed,
    /// 用户未找到
    UserNotFound,
    /// 用户已存在
    UserExists,
    /// 用户状态错误
    InvalidUserState,
    /// 用户配额超出
    UserQuotaExceeded,
    /// 用户策略违反
    UserPolicyViolation,
    /// 用户会话错误
    UserSessionError,
}

/// 接口错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterfaceError {
    /// 接口未找到
    InterfaceNotFound,
    /// 接口不可用
    InterfaceUnavailable,
    /// 接口版本不兼容
    InterfaceVersionIncompatible,
    /// 接口协议错误
    InterfaceProtocolError,
    /// 接口超时
    InterfaceTimeout,
    /// 接口配置错误
    InterfaceConfigurationError,
    /// 接口状态错误
    InterfaceStateError,
    /// 接口权限错误
    InterfacePermissionError,
    /// 接口依赖错误
    InterfaceDependencyError,
}

/// 未知错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownError {
    /// 未知错误码
    UnknownErrorCode(u32),
    /// 未知错误消息
    UnknownErrorMessage(String),
    /// 未知错误类型
    UnknownErrorType(String),
    /// 未知错误源
    UnknownErrorSource(String),
}

/// 错误优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorPriority {
    /// 低优先级
    Low = 1,
    /// 正常优先级
    Normal = 2,
    /// 高优先级
    High = 3,
    /// 紧急优先级
    Critical = 4,
    /// 致命优先级
    Fatal = 5,
}

/// 错误签名
#[derive(Debug, Clone)]
pub struct ErrorSignature {
    /// 错误类型
    pub error_type: String,
    /// 错误类别
    pub error_category: String,
    /// 错误严重级别
    pub severity: ErrorSeverity,
    /// 错误优先级
    pub priority: ErrorPriority,
    /// 错误源模块
    pub source_module: String,
    /// 错误源函数
    pub source_function: String,
    /// 错误源文件
    pub source_file: String,
    /// 错误源行号
    pub source_line: u32,
}

/// 错误指纹
#[derive(Debug, Clone)]
pub struct ErrorFingerprint {
    /// 指纹ID
    pub fingerprint_id: String,
    /// 指纹哈希
    pub fingerprint_hash: u64,
    /// 相似错误组
    pub similar_error_group: String,
    /// 错误模式
    pub error_pattern: String,
    /// 创建时间
    pub created_at: u64,
    /// 最后出现时间
    pub last_seen_at: u64,
    /// 出现次数
    pub occurrence_count: u64,
}

/// 增强的错误上下文
#[derive(Debug, Clone)]
pub struct EnhancedErrorContext {
    /// 基础上下文信息
    pub basic_context: BasicContext,
    
    /// 系统状态快照
    pub system_state: SystemStateSnapshot,
    
    /// 执行环境信息
    pub execution_environment: ExecutionEnvironment,
    
    /// 相关资源信息
    pub related_resources: RelatedResources,
    
    /// 错误传播路径
    pub error_propagation_path: ErrorPropagationPath,
    
    /// 用户上下文信息
    pub user_context: UserContext,
}

/// 基础上下文信息
#[derive(Debug, Clone)]
pub struct BasicContext {
    /// 时间戳
    pub timestamp: u64,
    /// 进程ID
    pub process_id: u32,
    /// 线程ID
    pub thread_id: u32,
    /// CPU ID
    pub cpu_id: u32,
    /// 错误消息
    pub error_message: String,
    /// 错误描述
    pub error_description: String,
    /// 错误源
    pub source: ErrorSource,
}

/// 执行环境信息
#[derive(Debug, Clone)]
pub struct ExecutionEnvironment {
    /// 环境变量
    pub environment_variables: BTreeMap<String, String>,
    /// 命令行参数
    pub command_line_args: Vec<String>,
    /// 工作目录
    pub working_directory: String,
    /// 用户ID
    pub user_id: u32,
    /// 组ID
    pub group_id: u32,
    /// 权限集合
    pub privileges: Vec<String>,
}

/// 相关资源信息
#[derive(Debug, Clone)]
pub struct RelatedResources {
    /// 文件描述符
    pub file_descriptors: Vec<i32>,
    /// 内存地址
    pub memory_addresses: Vec<u64>,
    /// 网络连接
    pub network_connections: Vec<String>,
    /// 设备ID
    pub device_ids: Vec<String>,
    /// 锁对象
    pub lock_objects: Vec<String>,
}

/// 错误传播路径
#[derive(Debug, Clone)]
pub struct ErrorPropagationPath {
    /// 传播栈
    pub propagation_stack: Vec<PropagationFrame>,
    /// 传播深度
    pub propagation_depth: u32,
    /// 传播时间
    pub propagation_time_ms: u64,
    /// 传播模式
    pub propagation_pattern: PropagationPattern,
}

/// 传播帧
#[derive(Debug, Clone)]
pub struct PropagationFrame {
    /// 模块名
    pub module_name: String,
    /// 函数名
    pub function_name: String,
    /// 文件名
    pub file_name: String,
    /// 行号
    pub line_number: u32,
    /// 时间戳
    pub timestamp: u64,
    /// 传播类型
    pub propagation_type: PropagationType,
}

/// 传播类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropagationType {
    /// 直接传播
    Direct,
    /// 间接传播
    Indirect,
    /// 异步传播
    Asynchronous,
    /// 回调传播
    Callback,
    /// 事件传播
    Event,
}

/// 传播模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropagationPattern {
    /// 线性传播
    Linear,
    /// 指数传播
    Exponential,
    /// 级联传播
    Cascade,
    /// 循环传播
    Circular,
    /// 随机传播
    Random,
}

/// 用户上下文信息
#[derive(Debug, Clone)]
pub struct UserContext {
    /// 用户名
    pub username: String,
    /// 用户会话ID
    pub session_id: String,
    /// 用户角色
    pub user_role: String,
    /// 用户权限
    pub user_permissions: Vec<String>,
    /// 用户操作
    pub user_operation: String,
    /// 用户输入
    pub user_input: Option<String>,
}

/// 错误元数据
#[derive(Debug, Clone)]
pub struct ErrorMetadata {
    /// 错误ID
    pub error_id: u64,
    /// 错误签名
    pub error_signature: ErrorSignature,
    /// 错误指纹
    pub error_fingerprint: ErrorFingerprint,
    /// 错误分类信息
    pub classification: ErrorClassification,
    /// 错误严重级别
    pub severity: ErrorSeverity,
    /// 错误优先级
    pub priority: ErrorPriority,
    /// 错误标签
    pub tags: Vec<String>,
    /// 自定义属性
    pub custom_attributes: BTreeMap<String, String>,
}

/// 错误ID生成器
static ERROR_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl UnifiedError {
    /// 创建新的统一错误
    pub fn new(error: UnifiedError) -> Self {
        error
    }

    /// 获取错误类型名称
    pub fn error_type_name(&self) -> &'static str {
        match self {
            UnifiedError::Syscall(_) => "Syscall",
            UnifiedError::Memory(_) => "Memory",
            UnifiedError::FileSystem(_) => "FileSystem",
            UnifiedError::Network(_) => "Network",
            UnifiedError::Process(_) => "Process",
            UnifiedError::Device(_) => "Device",
            UnifiedError::Security(_) => "Security",
            UnifiedError::Configuration(_) => "Configuration",
            UnifiedError::Hardware(_) => "Hardware",
            UnifiedError::Timeout(_) => "Timeout",
            UnifiedError::Data(_) => "Data",
            UnifiedError::Protocol(_) => "Protocol",
            UnifiedError::Resource(_) => "Resource",
            UnifiedError::User(_) => "User",
            UnifiedError::Interface(_) => "Interface",
            UnifiedError::Unknown(_) => "Unknown",
        }
    }

    /// 获取错误严重级别
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            UnifiedError::Syscall(syscall_err) => match syscall_err {
                SyscallError::InvalidSyscall => ErrorSeverity::Error,
                SyscallError::PermissionDenied => ErrorSeverity::Error,
                SyscallError::InvalidArgument => ErrorSeverity::Warning,
                SyscallError::NotFound => ErrorSeverity::Warning,
                SyscallError::OutOfMemory => ErrorSeverity::Critical,
                SyscallError::Interrupted => ErrorSeverity::Info,
                SyscallError::IoError => ErrorSeverity::Error,
                SyscallError::WouldBlock => ErrorSeverity::Info,
                SyscallError::NotSupported => ErrorSeverity::Warning,
                SyscallError::BadFileDescriptor => ErrorSeverity::Error,
                SyscallError::TooManyOpenFiles => ErrorSeverity::Error,
                SyscallError::NoBufferSpace => ErrorSeverity::Warning,
                SyscallError::NotADirectory => ErrorSeverity::Warning,
                SyscallError::IsADirectory => ErrorSeverity::Warning,
                SyscallError::DirectoryNotEmpty => ErrorSeverity::Warning,
                SyscallError::FileExists => ErrorSeverity::Warning,
                SyscallError::CrossDeviceLink => ErrorSeverity::Warning,
                SyscallError::FileTooBig => ErrorSeverity::Error,
                SyscallError::NoSpaceLeft => ErrorSeverity::Critical,
                SyscallError::BadAddress => ErrorSeverity::Error,
                SyscallError::DeadlockWouldOccur => ErrorSeverity::Error,
                SyscallError::NameTooLong => ErrorSeverity::Warning,
                SyscallError::TooManySymlinks => ErrorSeverity::Warning,
                SyscallError::ConnectionRefused => ErrorSeverity::Error,
                SyscallError::ConnectionReset => ErrorSeverity::Error,
                SyscallError::BrokenPipe => ErrorSeverity::Error,
                SyscallError::TimedOut => ErrorSeverity::Warning,
            },
            UnifiedError::Memory(mem_err) => match mem_err {
                MemoryError::OutOfMemory => ErrorSeverity::Critical,
                MemoryError::InvalidAddress => ErrorSeverity::Error,
                MemoryError::AlignmentError => ErrorSeverity::Error,
                MemoryError::ProtectionError => ErrorSeverity::Error,
                MemoryError::FragmentationError => ErrorSeverity::Warning,
                MemoryError::MemoryLeak => ErrorSeverity::Critical,
                MemoryError::BufferOverflow => ErrorSeverity::Critical,
                MemoryError::DoubleFree => ErrorSeverity::Error,
                MemoryError::UseAfterFree => ErrorSeverity::Critical,
            },
            UnifiedError::FileSystem(fs_err) => match fs_err {
                FileSystemError::FileNotFound => ErrorSeverity::Warning,
                FileSystemError::PermissionDenied => ErrorSeverity::Error,
                FileSystemError::FileExists => ErrorSeverity::Warning,
                FileSystemError::NotADirectory => ErrorSeverity::Warning,
                FileSystemError::IsADirectory => ErrorSeverity::Warning,
                FileSystemError::DirectoryNotEmpty => ErrorSeverity::Warning,
                FileSystemError::DiskFull => ErrorSeverity::Critical,
                FileSystemError::Corruption => ErrorSeverity::Critical,
                FileSystemError::ReadOnly => ErrorSeverity::Error,
                FileSystemError::InvalidPath => ErrorSeverity::Warning,
                FileSystemError::CrossDeviceLink => ErrorSeverity::Warning,
                FileSystemError::NameTooLong => ErrorSeverity::Warning,
                FileSystemError::TooManySymlinks => ErrorSeverity::Warning,
                FileSystemError::FileSystemBusy => ErrorSeverity::Error,
            },
            UnifiedError::Network(net_err) => match net_err {
                NetworkError::ConnectionRefused => ErrorSeverity::Error,
                NetworkError::ConnectionTimeout => ErrorSeverity::Warning,
                NetworkError::ConnectionReset => ErrorSeverity::Warning,
                NetworkError::HostUnreachable => ErrorSeverity::Error,
                NetworkError::NetworkUnreachable => ErrorSeverity::Error,
                NetworkError::AddressInUse => ErrorSeverity::Warning,
                NetworkError::AddressNotAvailable => ErrorSeverity::Warning,
                NetworkError::ProtocolError => ErrorSeverity::Error,
                NetworkError::PacketCorruption => ErrorSeverity::Error,
                NetworkError::NetworkTimeout => ErrorSeverity::Warning,
                NetworkError::BandwidthLimit => ErrorSeverity::Warning,
                NetworkError::ConnectionLimit => ErrorSeverity::Error,
            },
            UnifiedError::Process(proc_err) => match proc_err {
                ProcessError::ProcessNotFound => ErrorSeverity::Warning,
                ProcessError::PermissionDenied => ErrorSeverity::Error,
                ProcessError::ResourceLimit => ErrorSeverity::Error,
                ProcessError::ProcessExists => ErrorSeverity::Warning,
                ProcessError::ProcessTerminated => ErrorSeverity::Info,
                ProcessError::ProcessSuspended => ErrorSeverity::Info,
                ProcessError::InvalidProcessState => ErrorSeverity::Error,
                ProcessError::ProcessMemoryError => ErrorSeverity::Error,
                ProcessError::ProcessSchedulingError => ErrorSeverity::Warning,
                ProcessError::ProcessCommunicationError => ErrorSeverity::Error,
            },
            UnifiedError::Device(dev_err) => match dev_err {
                DeviceError::DeviceNotFound => ErrorSeverity::Warning,
                DeviceError::DeviceBusy => ErrorSeverity::Warning,
                DeviceError::DeviceUnavailable => ErrorSeverity::Error,
                DeviceError::DeviceError => ErrorSeverity::Error,
                DeviceError::DeviceTimeout => ErrorSeverity::Warning,
                DeviceError::DeviceConfigurationError => ErrorSeverity::Error,
                DeviceError::DeviceDriverError => ErrorSeverity::Error,
                DeviceError::DeviceFirmwareError => ErrorSeverity::Error,
                DeviceError::DeviceHardwareError => ErrorSeverity::Critical,
            },
            UnifiedError::Security(sec_err) => match sec_err {
                SecurityError::AuthenticationFailed => ErrorSeverity::Error,
                SecurityError::AuthorizationFailed => ErrorSeverity::Error,
                SecurityError::PermissionDenied => ErrorSeverity::Error,
                SecurityError::AccessDenied => ErrorSeverity::Error,
                SecurityError::SecurityPolicyViolation => ErrorSeverity::Error,
                SecurityError::EncryptionError => ErrorSeverity::Error,
                SecurityError::DecryptionError => ErrorSeverity::Error,
                SecurityError::SignatureVerificationFailed => ErrorSeverity::Error,
                SecurityError::CertificateError => ErrorSeverity::Error,
                SecurityError::SecurityContextError => ErrorSeverity::Error,
            },
            UnifiedError::Configuration(config_err) => match config_err {
                ConfigurationError::ConfigFileNotFound => ErrorSeverity::Warning,
                ConfigurationError::ConfigFileFormatError => ErrorSeverity::Error,
                ConfigurationError::ConfigItemNotFound => ErrorSeverity::Warning,
                ConfigurationError::InvalidConfigValue => ErrorSeverity::Error,
                ConfigurationError::ConfigConflict => ErrorSeverity::Error,
                ConfigurationError::ConfigValidationFailed => ErrorSeverity::Error,
                ConfigurationError::ConfigPermissionError => ErrorSeverity::Error,
                ConfigurationError::ConfigVersionIncompatible => ErrorSeverity::Error,
                ConfigurationError::ConfigDependencyError => ErrorSeverity::Error,
            },
            UnifiedError::Hardware(hw_err) => match hw_err {
                HardwareError::HardwareFailure => ErrorSeverity::Critical,
                HardwareError::HardwareTimeout => ErrorSeverity::Error,
                HardwareError::HardwareConfigurationError => ErrorSeverity::Error,
                HardwareError::HardwareCompatibilityIssue => ErrorSeverity::Error,
                HardwareError::HardwareResourceConflict => ErrorSeverity::Error,
                HardwareError::HardwareDriverIssue => ErrorSeverity::Error,
                HardwareError::HardwareFirmwareIssue => ErrorSeverity::Error,
                HardwareError::HardwareOverheating => ErrorSeverity::Critical,
                HardwareError::HardwarePowerIssue => ErrorSeverity::Critical,
            },
            UnifiedError::Timeout(to_err) => match to_err {
                TimeoutError::OperationTimeout => ErrorSeverity::Warning,
                TimeoutError::ConnectionTimeout => ErrorSeverity::Warning,
                TimeoutError::ReadTimeout => ErrorSeverity::Warning,
                TimeoutError::WriteTimeout => ErrorSeverity::Warning,
                TimeoutError::WaitTimeout => ErrorSeverity::Warning,
                TimeoutError::LockTimeout => ErrorSeverity::Warning,
                TimeoutError::ResponseTimeout => ErrorSeverity::Warning,
                TimeoutError::SystemCallTimeout => ErrorSeverity::Warning,
                TimeoutError::InitializationTimeout => ErrorSeverity::Error,
            },
            UnifiedError::Data(data_err) => match data_err {
                DataError::DataCorruption => ErrorSeverity::Critical,
                DataError::DataFormatError => ErrorSeverity::Error,
                DataError::DataValidationFailed => ErrorSeverity::Error,
                DataError::DataEncodingError => ErrorSeverity::Error,
                DataError::DataDecodingError => ErrorSeverity::Error,
                DataError::DataTruncation => ErrorSeverity::Warning,
                DataError::DataOverflow => ErrorSeverity::Error,
                DataError::DataTypeMismatch => ErrorSeverity::Error,
                DataError::DataVersionIncompatible => ErrorSeverity::Error,
            },
            UnifiedError::Protocol(proto_err) => match proto_err {
                ProtocolError::UnsupportedProtocolVersion => ErrorSeverity::Error,
                ProtocolError::ProtocolError => ErrorSeverity::Error,
                ProtocolError::ProtocolViolation => ErrorSeverity::Error,
                ProtocolError::ProtocolTimeout => ErrorSeverity::Warning,
                ProtocolError::ProtocolNegotiationFailed => ErrorSeverity::Error,
                ProtocolError::ProtocolStateError => ErrorSeverity::Error,
                ProtocolError::ProtocolMessageError => ErrorSeverity::Error,
                ProtocolError::ProtocolHeaderError => ErrorSeverity::Error,
                ProtocolError::ProtocolChecksumError => ErrorSeverity::Error,
            },
            UnifiedError::Resource(res_err) => match res_err {
                ResourceError::ResourceExhausted => ErrorSeverity::Critical,
                ResourceError::ResourceNotFound => ErrorSeverity::Warning,
                ResourceError::ResourceBusy => ErrorSeverity::Warning,
                ResourceError::ResourceUnavailable => ErrorSeverity::Error,
                ResourceError::ResourceQuotaExceeded => ErrorSeverity::Error,
                ResourceError::ResourcePermissionError => ErrorSeverity::Error,
                ResourceError::ResourceStateError => ErrorSeverity::Error,
                ResourceError::ResourceDependencyError => ErrorSeverity::Error,
                ResourceError::ResourceConfigurationError => ErrorSeverity::Error,
            },
            UnifiedError::User(user_err) => match user_err {
                UserError::InvalidInput => ErrorSeverity::Warning,
                UserError::InsufficientPrivileges => ErrorSeverity::Error,
                UserError::UserAuthenticationFailed => ErrorSeverity::Error,
                UserError::UserNotFound => ErrorSeverity::Warning,
                UserError::UserExists => ErrorSeverity::Warning,
                UserError::InvalidUserState => ErrorSeverity::Error,
                UserError::UserQuotaExceeded => ErrorSeverity::Error,
                UserError::UserPolicyViolation => ErrorSeverity::Error,
                UserError::UserSessionError => ErrorSeverity::Error,
            },
            UnifiedError::Interface(iface_err) => match iface_err {
                InterfaceError::InterfaceNotFound => ErrorSeverity::Warning,
                InterfaceError::InterfaceUnavailable => ErrorSeverity::Error,
                InterfaceError::InterfaceVersionIncompatible => ErrorSeverity::Error,
                InterfaceError::InterfaceProtocolError => ErrorSeverity::Error,
                InterfaceError::InterfaceTimeout => ErrorSeverity::Warning,
                InterfaceError::InterfaceConfigurationError => ErrorSeverity::Error,
                InterfaceError::InterfaceStateError => ErrorSeverity::Error,
                InterfaceError::InterfacePermissionError => ErrorSeverity::Error,
                InterfaceError::InterfaceDependencyError => ErrorSeverity::Error,
            },
            UnifiedError::Unknown(_) => ErrorSeverity::Warning,
        }
    }

    /// 获取错误优先级
    pub fn priority(&self) -> ErrorPriority {
        match self.severity() {
            ErrorSeverity::Info => ErrorPriority::Low,
            ErrorSeverity::Low => ErrorPriority::Low,
            ErrorSeverity::Warning => ErrorPriority::Normal,
            ErrorSeverity::Medium => ErrorPriority::Normal,
            ErrorSeverity::High => ErrorPriority::High,
            ErrorSeverity::Error => ErrorPriority::High,
            ErrorSeverity::Critical => ErrorPriority::Critical,
            ErrorSeverity::Fatal => ErrorPriority::Fatal,
        }
    }

    /// 生成错误签名
    pub fn generate_signature(&self, source: &ErrorSource) -> ErrorSignature {
        ErrorSignature {
            error_type: self.error_type_name().to_string(),
            error_category: format!("{:?}", self.category()),
            severity: self.severity(),
            priority: self.priority(),
            source_module: source.module.clone(),
            source_function: source.function.clone(),
            source_file: source.file.clone(),
            source_line: source.line,
        }
    }

    /// 生成错误指纹
    pub fn generate_fingerprint(&self, signature: &ErrorSignature) -> ErrorFingerprint {
        let fingerprint_data = format!(
            "{}:{}:{}:{}:{}",
            signature.error_type,
            signature.error_category,
            signature.source_module,
            signature.source_function,
            signature.source_line
        );
        
        let fingerprint_hash = {
            use core::hash::{Hash, Hasher};
            let mut hasher = <DefaultHasherBuilder as core::hash::BuildHasher>::build_hasher(&DefaultHasherBuilder);
            fingerprint_data.hash(&mut hasher);
            hasher.finish()
        };
        
        ErrorFingerprint {
            fingerprint_id: format!("fp_{:x}", fingerprint_hash),
            fingerprint_hash,
            similar_error_group: format!("group_{}", signature.error_type),
            error_pattern: signature.error_type.clone(),
            created_at: crate::time::get_timestamp(),
            last_seen_at: crate::time::get_timestamp(),
            occurrence_count: 1,
        }
    }

    /// 获取错误类别
    pub fn category(&self) -> ErrorCategory {
        match self {
            UnifiedError::Syscall(_) => ErrorCategory::System,
            UnifiedError::Memory(_) => ErrorCategory::Memory,
            UnifiedError::FileSystem(_) => ErrorCategory::FileSystem,
            UnifiedError::Network(_) => ErrorCategory::Network,
            UnifiedError::Process(_) => ErrorCategory::Process,
            UnifiedError::Device(_) => ErrorCategory::Device,
            UnifiedError::Security(_) => ErrorCategory::Security,
            UnifiedError::Configuration(_) => ErrorCategory::Configuration,
            UnifiedError::Hardware(_) => ErrorCategory::Hardware,
            UnifiedError::Timeout(_) => ErrorCategory::Timeout,
            UnifiedError::Data(_) => ErrorCategory::Data,
            UnifiedError::Protocol(_) => ErrorCategory::Protocol,
            UnifiedError::Resource(_) => ErrorCategory::Resource,
            UnifiedError::User(_) => ErrorCategory::User,
            UnifiedError::Interface(_) => ErrorCategory::Interface,
            UnifiedError::Unknown(_) => ErrorCategory::System,
        }
    }

    /// 生成错误元数据
    pub fn generate_metadata(&self, source: &ErrorSource) -> ErrorMetadata {
        let error_id = ERROR_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        let signature = self.generate_signature(source);
        let fingerprint = self.generate_fingerprint(&signature);
        
        ErrorMetadata {
            error_id,
            error_signature: signature,
            error_fingerprint: fingerprint,
            classification: ErrorClassification {
                id: format!("cls_{}", error_id),
                name: format!("Classification for {}", self.error_type_name()),
                classification_type: ClassificationType::UnknownIssue,
                severity: self.severity(),
                urgency: Urgency::Medium,
                impact_scope: ImpactScope::Local,
                recovery_complexity: RecoveryComplexity::Medium,
                requires_immediate_action: self.severity() >= ErrorSeverity::Error,
                auto_recoverable: self.severity() < ErrorSeverity::Critical,
                recommended_response_time: 300,
                tags: vec![self.error_type_name().to_string()],
                confidence: 1.0,
            },
            severity: self.severity(),
            priority: self.priority(),
            tags: vec![self.error_type_name().to_string()],
            custom_attributes: BTreeMap::new(),
        }
    }
}

// 错误转换实现
impl From<SyscallError> for UnifiedError {
    fn from(err: SyscallError) -> Self {
        UnifiedError::Syscall(err)
    }
}

impl From<KernelError> for UnifiedError {
    fn from(err: KernelError) -> Self {
        match err {
            KernelError::Syscall(syscall_err) => UnifiedError::Syscall(syscall_err),
            KernelError::OutOfMemory => UnifiedError::Memory(MemoryError::OutOfMemory),
            KernelError::InvalidArgument => UnifiedError::User(UserError::InvalidInput),
            KernelError::NotFound => UnifiedError::FileSystem(FileSystemError::FileNotFound),
            KernelError::PermissionDenied => UnifiedError::Security(SecurityError::PermissionDenied),
            KernelError::IoError => UnifiedError::FileSystem(FileSystemError::Corruption),
            KernelError::NotSupported => UnifiedError::Interface(InterfaceError::InterfaceUnavailable),
            KernelError::AlreadyExists => UnifiedError::FileSystem(FileSystemError::FileExists),
            KernelError::ResourceBusy => UnifiedError::Resource(ResourceError::ResourceBusy),
            KernelError::Timeout => UnifiedError::Timeout(TimeoutError::OperationTimeout),
        }
    }
}

impl From<UnifiedError> for KernelError {
    fn from(err: UnifiedError) -> Self {
        match err {
            UnifiedError::Syscall(syscall_err) => KernelError::Syscall(syscall_err),
            UnifiedError::Memory(mem_err) => match mem_err {
                MemoryError::OutOfMemory => KernelError::OutOfMemory,
                MemoryError::InvalidAddress => KernelError::InvalidArgument,
                MemoryError::AlignmentError => KernelError::InvalidArgument,
                MemoryError::ProtectionError => KernelError::PermissionDenied,
                MemoryError::FragmentationError => KernelError::InvalidArgument,
                MemoryError::MemoryLeak => KernelError::OutOfMemory,
                MemoryError::BufferOverflow => KernelError::InvalidArgument,
                MemoryError::DoubleFree => KernelError::InvalidArgument,
                MemoryError::UseAfterFree => KernelError::InvalidArgument,
            },
            UnifiedError::FileSystem(fs_err) => match fs_err {
                FileSystemError::FileNotFound => KernelError::NotFound,
                FileSystemError::PermissionDenied => KernelError::PermissionDenied,
                FileSystemError::FileExists => KernelError::AlreadyExists,
                FileSystemError::NotADirectory => KernelError::InvalidArgument,
                FileSystemError::IsADirectory => KernelError::InvalidArgument,
                FileSystemError::DirectoryNotEmpty => KernelError::InvalidArgument,
                FileSystemError::DiskFull => KernelError::OutOfMemory,
                FileSystemError::Corruption => KernelError::IoError,
                FileSystemError::ReadOnly => KernelError::PermissionDenied,
                FileSystemError::InvalidPath => KernelError::InvalidArgument,
                FileSystemError::CrossDeviceLink => KernelError::InvalidArgument,
                FileSystemError::NameTooLong => KernelError::InvalidArgument,
                FileSystemError::TooManySymlinks => KernelError::InvalidArgument,
                FileSystemError::FileSystemBusy => KernelError::ResourceBusy,
            },
            UnifiedError::Network(_) => KernelError::IoError,
            UnifiedError::Process(_) => KernelError::InvalidArgument,
            UnifiedError::Device(_) => KernelError::IoError,
            UnifiedError::Security(_) => KernelError::PermissionDenied,
            UnifiedError::Configuration(_) => KernelError::InvalidArgument,
            UnifiedError::Hardware(_) => KernelError::IoError,
            UnifiedError::Timeout(to_err) => KernelError::Timeout,
            UnifiedError::Data(_) => KernelError::InvalidArgument,
            UnifiedError::Protocol(_) => KernelError::IoError,
            UnifiedError::Resource(res_err) => match res_err {
                ResourceError::ResourceExhausted => KernelError::OutOfMemory,
                ResourceError::ResourceNotFound => KernelError::NotFound,
                ResourceError::ResourceBusy => KernelError::ResourceBusy,
                ResourceError::ResourceUnavailable => KernelError::NotSupported,
                ResourceError::ResourceQuotaExceeded => KernelError::ResourceBusy,
                ResourceError::ResourcePermissionError => KernelError::PermissionDenied,
                ResourceError::ResourceStateError => KernelError::InvalidArgument,
                ResourceError::ResourceDependencyError => KernelError::InvalidArgument,
                ResourceError::ResourceConfigurationError => KernelError::InvalidArgument,
            },
            UnifiedError::User(user_err) => match user_err {
                UserError::InvalidInput => KernelError::InvalidArgument,
                UserError::InsufficientPrivileges => KernelError::PermissionDenied,
                UserError::UserAuthenticationFailed => KernelError::PermissionDenied,
                UserError::UserNotFound => KernelError::NotFound,
                UserError::UserExists => KernelError::AlreadyExists,
                UserError::InvalidUserState => KernelError::InvalidArgument,
                UserError::UserQuotaExceeded => KernelError::ResourceBusy,
                UserError::UserPolicyViolation => KernelError::PermissionDenied,
                UserError::UserSessionError => KernelError::InvalidArgument,
            },
            UnifiedError::Interface(iface_err) => match iface_err {
                InterfaceError::InterfaceNotFound => KernelError::NotFound,
                InterfaceError::InterfaceUnavailable => KernelError::NotSupported,
                InterfaceError::InterfaceVersionIncompatible => KernelError::InvalidArgument,
                InterfaceError::InterfaceProtocolError => KernelError::IoError,
                InterfaceError::InterfaceTimeout => KernelError::Timeout,
                InterfaceError::InterfaceConfigurationError => KernelError::InvalidArgument,
                InterfaceError::InterfaceStateError => KernelError::InvalidArgument,
                InterfaceError::InterfacePermissionError => KernelError::PermissionDenied,
                InterfaceError::InterfaceDependencyError => KernelError::InvalidArgument,
            },
            UnifiedError::Unknown(_) => KernelError::InvalidArgument,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_error_creation() {
        let error = UnifiedError::Memory(MemoryError::OutOfMemory);
        assert_eq!(error.error_type_name(), "Memory");
        assert_eq!(error.severity(), ErrorSeverity::Critical);
        assert_eq!(error.priority(), ErrorPriority::Critical);
    }

    #[test]
    fn test_error_conversion() {
        let syscall_error = SyscallError::OutOfMemory;
        let unified_error = UnifiedError::from(syscall_error);
        
        match unified_error {
            UnifiedError::Syscall(SyscallError::OutOfMemory) => {},
            _ => panic!("Expected Syscall::OutOfMemory"),
        }
    }

    #[test]
    fn test_error_signature_generation() {
        let error = UnifiedError::FileSystem(FileSystemError::FileNotFound);
        let source = ErrorSource {
            module: "test_module".to_string(),
            function: "test_function".to_string(),
            file: "test.rs".to_string(),
            line: 42,
            column: 0,
            process_id: 123,
            thread_id: 456,
            cpu_id: 0,
        };
        
        let signature = error.generate_signature(&source);
        assert_eq!(signature.error_type, "FileSystem");
        assert_eq!(signature.source_module, "test_module");
        assert_eq!(signature.source_function, "test_function");
        assert_eq!(signature.source_line, 42);
    }

    #[test]
    fn test_error_fingerprint_generation() {
        let error = UnifiedError::Network(NetworkError::ConnectionRefused);
        let source = ErrorSource {
            module: "network_module".to_string(),
            function: "connect".to_string(),
            file: "network.rs".to_string(),
            line: 100,
            column: 0,
            process_id: 123,
            thread_id: 456,
            cpu_id: 0,
        };
        
        let signature = error.generate_signature(&source);
        let fingerprint = error.generate_fingerprint(&signature);
        
        assert!(!fingerprint.fingerprint_id.is_empty());
        assert_eq!(fingerprint.similar_error_group, "group_Network");
        assert_eq!(fingerprint.error_pattern, "Network");
        assert_eq!(fingerprint.occurrence_count, 1);
    }
}