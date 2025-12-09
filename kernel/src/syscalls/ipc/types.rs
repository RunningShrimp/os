//! IPC模块类型定义
//! 
//! 本模块定义了进程间通信相关的类型、枚举和结构体，包括：
//! - 管道和命名管道类型
//! - 共享内存类型
//! - 消息队列类型
//! - 信号量类型

use alloc::string::String;
use alloc::vec::Vec;

/// IPC对象类型枚举
/// 
/// 定义IPC对象的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcObjectType {
    /// 管道
    Pipe,
    /// 命名管道
    NamedPipe,
    /// 共享内存
    SharedMemory,
    /// 消息队列
    MessageQueue,
    /// 信号量
    Semaphore,
    /// 共享文件描述符
    SharedFileDescriptor,
}

/// IPC权限标志
/// 
/// 定义IPC对象的访问权限。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcPermissions {
    /// 所有者读权限
    pub owner_read: bool,
    /// 所有者写权限
    pub owner_write: bool,
    /// 所有者执行权限
    pub owner_execute: bool,
    /// 组读权限
    pub group_read: bool,
    /// 组写权限
    pub group_write: bool,
    /// 组执行权限
    pub group_execute: bool,
    /// 其他读权限
    pub other_read: bool,
    /// 其他写权限
    pub other_write: bool,
    /// 其他执行权限
    pub other_execute: bool,
}

impl IpcPermissions {
    /// 创建新的IPC权限
    pub fn new() -> Self {
        Self {
            owner_read: true,
            owner_write: true,
            owner_execute: false,
            group_read: true,
            group_write: false,
            group_execute: false,
            other_read: true,
            other_write: false,
            other_execute: false,
        }
    }

    /// 转换为八进制权限表示
    pub fn to_octal(&self) -> u16 {
        let mut mode = 0u16;
        
        if self.owner_read { mode |= 0o400; }
        if self.owner_write { mode |= 0o200; }
        if self.owner_execute { mode |= 0o100; }
        if self.group_read { mode |= 0o040; }
        if self.group_write { mode |= 0o020; }
        if self.group_execute { mode |= 0o010; }
        if self.other_read { mode |= 0o004; }
        if self.other_write { mode |= 0o002; }
        if self.other_execute { mode |= 0o001; }
        
        mode
    }

    /// 从八进制权限创建
    pub fn from_octal(mode: u16) -> Self {
        Self {
            owner_read: (mode & 0o400) != 0,
            owner_write: (mode & 0o200) != 0,
            owner_execute: (mode & 0o100) != 0,
            group_read: (mode & 0o040) != 0,
            group_write: (mode & 0o020) != 0,
            group_execute: (mode & 0o010) != 0,
            other_read: (mode & 0o004) != 0,
            other_write: (mode & 0o002) != 0,
            other_execute: (mode & 0o001) != 0,
        }
    }
}

impl Default for IpcPermissions {
    fn default() -> Self {
        Self::new()
    }
}

/// 管道信息
/// 
/// 包含管道的详细属性信息。
#[derive(Debug, Clone)]
pub struct PipeInfo {
    /// 管道ID
    pub pipe_id: u32,
    /// 管道类型
    pub pipe_type: IpcObjectType,
    /// 读端文件描述符
    pub read_fd: i32,
    /// 写端文件描述符
    pub write_fd: i32,
    /// 缓冲区大小
    pub buffer_size: u32,
    /// 当前缓冲区使用量
    pub buffer_used: u32,
    /// 是否为命名管道
    pub is_named: bool,
    /// 管道名称（命名管道）
    pub name: Option<String>,
    /// 创建进程ID
    pub creator_pid: u32,
    /// 权限设置
    pub permissions: IpcPermissions,
}

/// 共享内存信息
/// 
/// 包含共享内存的详细属性信息。
#[derive(Debug, Clone)]
pub struct SharedMemoryInfo {
    /// 共享内存ID
    pub shm_id: u32,
    /// 内存大小
    pub size: u64,
    /// 内存地址
    pub address: u64,
    /// 附加进程数
    pub attached_processes: u32,
    /// 创建进程ID
    pub creator_pid: u32,
    /// 权限设置
    pub permissions: IpcPermissions,
    /// 创建标志
    pub creation_flags: u32,
    /// 最后附加时间
    pub last_attach_time: u64,
    /// 最后分离时间
    pub last_detach_time: u64,
}

/// 消息队列信息
/// 
/// 包含消息队列的详细属性信息。
#[derive(Debug, Clone)]
pub struct MessageQueueInfo {
    /// 消息队列ID
    pub queue_id: u32,
    /// 队列键值
    pub key: i32,
    /// 最大消息数
    pub max_messages: u32,
    /// 当前消息数
    pub current_messages: u32,
    /// 最大消息大小
    pub max_message_size: u32,
    /// 当前队列大小
    pub current_queue_size: u32,
    /// 创建进程ID
    pub creator_pid: u32,
    /// 权限设置
    pub permissions: IpcPermissions,
    /// 最后发送时间
    pub last_send_time: u64,
    /// 最后接收时间
    pub last_receive_time: u64,
}

/// 消息结构体
/// 
/// 表示消息队列中的消息。
#[derive(Debug, Clone)]
pub struct Message {
    /// 消息类型
    pub message_type: i64,
    /// 消息数据
    pub data: Vec<u8>,
    /// 消息优先级
    pub priority: u8,
    /// 发送进程ID
    pub sender_pid: u32,
    /// 发送时间戳
    pub timestamp: u64,
}

/// 信号量信息
/// 
/// 包含信号量的详细属性信息。
#[derive(Debug, Clone)]
pub struct SemaphoreInfo {
    /// 信号量ID
    pub sem_id: u32,
    /// 信号量键值
    pub key: i32,
    /// 当前值
    pub current_value: u32,
    /// 初始值
    pub initial_value: u32,
    /// 等待进程数
    pub waiting_processes: u32,
    /// 最大值
    pub max_value: u32,
    /// 创建进程ID
    pub creator_pid: u32,
    /// 权限设置
    pub permissions: IpcPermissions,
    /// 最后操作时间
    pub last_operation_time: u64,
}

/// IPC操作参数
/// 
/// 包含IPC操作所需的参数。
#[derive(Debug, Clone)]
pub struct IpcOperationParams {
    /// 操作类型
    pub operation_type: IpcObjectType,
    /// 对象名称或键值
    pub name_or_key: String,
    /// 权限设置
    pub permissions: Option<IpcPermissions>,
    /// 操作标志
    pub flags: u32,
    /// 大小或数量
    pub size_or_count: u64,
    /// 超时时间
    pub timeout: Option<u32>,
}

impl Default for IpcOperationParams {
    fn default() -> Self {
        Self {
            operation_type: IpcObjectType::Pipe,
            name_or_key: String::new(),
            permissions: None,
            flags: 0,
            size_or_count: 0,
            timeout: None,
        }
    }
}

/// IPC统计信息
/// 
/// 包含IPC使用的统计信息。
#[derive(Debug, Clone)]
pub struct IpcStats {
    /// 总IPC对象数
    pub total_objects: u32,
    /// 管道数量
    pub pipe_count: u32,
    /// 共享内存数量
    pub shared_memory_count: u32,
    /// 消息队列数量
    pub message_queue_count: u32,
    /// 信号量数量
    pub semaphore_count: u32,
    /// 总内存使用量
    pub total_memory_usage: u64,
    /// 活跃连接数
    pub active_connections: u32,
    /// 待处理消息数
    pub pending_messages: u32,
}

/// IPC错误类型
/// 
/// 定义IPC模块特有的错误类型。
#[derive(Debug, Clone)]
pub enum IpcError {
    /// 对象不存在
    ObjectNotFound,
    /// 对象已存在
    ObjectExists,
    /// 权限不足
    PermissionDenied,
    /// 无效参数
    InvalidArgument,
    /// 资源不足
    ResourceExhausted,
    /// 队列满
    QueueFull,
    /// 队列空
    QueueEmpty,
    /// 内存不足
    OutOfMemory,
    /// 超时
    Timeout,
    /// 连接被拒绝
    ConnectionRefused,
    /// 系统调用不支持
    UnsupportedSyscall,
}

impl IpcError {
    /// 获取错误码
    pub fn error_code(&self) -> i32 {
        match self {
            IpcError::ObjectNotFound => -2,
            IpcError::ObjectExists => -17,
            IpcError::PermissionDenied => -13,
            IpcError::InvalidArgument => -22,
            IpcError::ResourceExhausted => -11,
            IpcError::QueueFull => -105,
            IpcError::QueueEmpty => -42,
            IpcError::OutOfMemory => -12,
            IpcError::Timeout => -110,
            IpcError::ConnectionRefused => -111,
            IpcError::UnsupportedSyscall => -38,
        }
    }

    /// 获取错误描述
    pub fn error_message(&self) -> &str {
        match self {
            IpcError::ObjectNotFound => "IPC object not found",
            IpcError::ObjectExists => "IPC object already exists",
            IpcError::PermissionDenied => "Permission denied",
            IpcError::InvalidArgument => "Invalid argument",
            IpcError::ResourceExhausted => "Resource exhausted",
            IpcError::QueueFull => "IPC queue full",
            IpcError::QueueEmpty => "IPC queue empty",
            IpcError::OutOfMemory => "Out of memory",
            IpcError::Timeout => "IPC operation timeout",
            IpcError::ConnectionRefused => "Connection refused",
            IpcError::UnsupportedSyscall => "Unsupported syscall",
        }
    }
}

/// IPC对象管理特征
/// 
/// 定义IPC对象管理的基本操作接口。
pub trait IpcObjectManager: Send + Sync {
    /// 创建IPC对象
    fn create_object(&mut self, params: IpcOperationParams) -> Result<u32, IpcError>;
    
    /// 获取IPC对象
    fn get_object(&self, id: u32) -> Option<IpcObjectInfo>;
    
    /// 删除IPC对象
    fn delete_object(&mut self, id: u32) -> Result<(), IpcError>;
    
    /// 列出IPC对象
    fn list_objects(&self, object_type: Option<IpcObjectType>) -> Vec<IpcObjectInfo>;
    
    /// 获取对象权限
    fn get_object_permissions(&self, id: u32) -> Option<IpcPermissions>;
    
    /// 设置对象权限
    fn set_object_permissions(&mut self, id: u32, permissions: IpcPermissions) -> Result<(), IpcError>;
}

/// IPC对象信息
/// 
/// 表示IPC对象的通用信息。
#[derive(Debug, Clone)]
pub struct IpcObjectInfo {
    /// 对象ID
    pub id: u32,
    /// 对象类型
    pub object_type: IpcObjectType,
    /// 对象名称
    pub name: String,
    /// 对象大小
    pub size: u64,
    /// 权限设置
    pub permissions: IpcPermissions,
    /// 创建进程ID
    pub creator_pid: u32,
    /// 创建时间
    pub creation_time: u64,
    /// 最后访问时间
    pub last_access_time: u64,
    /// 引用计数
    pub reference_count: u32,
}

/// 管道管理特征
/// 
/// 定义管道管理的基本操作接口。
pub trait PipeManager: Send + Sync {
    /// 创建管道
    fn create_pipe(&mut self, flags: u32) -> Result<(i32, i32), IpcError>;
    
    /// 创建命名管道
    fn create_named_pipe(&mut self, name: &str, permissions: IpcPermissions) -> Result<i32, IpcError>;
    
    /// 关闭管道
    fn close_pipe(&mut self, fd: i32) -> Result<(), IpcError>;
    
    /// 从管道读取
    fn read_pipe(&mut self, fd: i32, buffer: &mut [u8], flags: u32) -> Result<usize, IpcError>;
    
    /// 向管道写入
    fn write_pipe(&mut self, fd: i32, data: &[u8], flags: u32) -> Result<usize, IpcError>;
    
    /// 获取管道信息
    fn get_pipe_info(&self, fd: i32) -> Option<PipeInfo>;
}

/// 共享内存管理特征
/// 
/// 定义共享内存管理的基本操作接口。
pub trait SharedMemoryManager: Send + Sync {
    /// 创建共享内存
    fn create_shared_memory(&mut self, size: u64, permissions: IpcPermissions, flags: u32) -> Result<u32, IpcError>;
    
    /// 附加共享内存
    fn attach_shared_memory(&mut self, id: u32, address: Option<u64>, flags: u32) -> Result<u64, IpcError>;
    
    /// 分离共享内存
    fn detach_shared_memory(&mut self, address: u64) -> Result<(), IpcError>;
    
    /// 控制共享内存
    fn control_shared_memory(&mut self, id: u32, command: i32, buffer: &mut [u8]) -> Result<(), IpcError>;
    
    /// 获取共享内存信息
    fn get_shared_memory_info(&self, id: u32) -> Option<SharedMemoryInfo>;
    
    /// 删除共享内存
    fn delete_shared_memory(&mut self, id: u32) -> Result<(), IpcError>;
}

/// 消息队列管理特征
/// 
/// 定义消息队列管理的基本操作接口。
pub trait MessageQueueManager: Send + Sync {
    /// 创建消息队列
    fn create_message_queue(&mut self, key: i32, permissions: IpcPermissions, flags: u32) -> Result<u32, IpcError>;
    
    /// 发送消息
    fn send_message(&mut self, id: u32, message: Message, flags: u32) -> Result<(), IpcError>;
    
    /// 接收消息
    fn receive_message(&mut self, id: u32, buffer: &mut [u8], flags: u32) -> Result<Message, IpcError>;
    
    /// 获取消息队列信息
    fn get_message_queue_info(&self, id: u32) -> Option<MessageQueueInfo>;
    
    /// 控制消息队列
    fn control_message_queue(&mut self, id: u32, command: i32, buffer: &mut [u8]) -> Result<(), IpcError>;
    
    /// 删除消息队列
    fn delete_message_queue(&mut self, id: u32) -> Result<(), IpcError>;
}

/// 信号量管理特征
/// 
/// 定义信号量管理的基本操作接口。
pub trait SemaphoreManager: Send + Sync {
    /// 创建信号量
    fn create_semaphore(&mut self, initial_value: u32, permissions: IpcPermissions, flags: u32) -> Result<u32, IpcError>;
    
    /// 等待信号量（P操作）
    fn wait_semaphore(&mut self, id: u32, operation: i32) -> Result<i32, IpcError>;
    
    /// 释放信号量（V操作）
    fn signal_semaphore(&mut self, id: u32, operation: i32) -> Result<i32, IpcError>;
    
    /// 获取信号量信息
    fn get_semaphore_info(&self, id: u32) -> Option<SemaphoreInfo>;
    
    /// 控制信号量
    fn control_semaphore(&mut self, id: u32, command: i32, buffer: &mut [u8]) -> Result<(), IpcError>;
    
    /// 删除信号量
    fn delete_semaphore(&mut self, id: u32) -> Result<(), IpcError>;
}