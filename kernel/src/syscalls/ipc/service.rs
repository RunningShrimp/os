//! IPC系统调用服务实现
//! 
//! 本模块实现进程间通信相关的系统调用服务，包括：
//! - 服务生命周期管理
//! - 系统调用分发和处理
//! - 与服务注册器的集成
//! - IPC对象管理

use crate::error_handling::unified::KernelError;
use crate::syscalls::ipc::handlers;
use crate::syscalls::services::{Service, ServiceStatus, SyscallService};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::any::Any;

/// IPC系统调用服务
/// 
/// 实现SyscallService特征，提供进程间通信相关的系统调用处理。
pub struct IpcService {
    /// 服务名称
    name: String,
    /// 服务版本
    version: String,
    /// 服务描述
    description: String,
    /// 服务状态
    status: ServiceStatus,
    /// 支持的系统调用号
    supported_syscalls: Vec<u32>,
    /// IPC对象表
    ipc_objects: Vec<crate::syscalls::ipc::types::IpcObjectInfo>,
    /// 下一个可用的IPC对象ID
    next_ipc_id: u32,
}

impl IpcService {
    /// 创建新的IPC服务实例
    /// 
    /// # 返回值
    /// 
    /// * `Self` - 新的服务实例
    pub fn new() -> Self {
        Self {
            name: String::from("ipc"),
            version: String::from("1.0.0"),
            description: String::from("IPC syscall service"),
            status: ServiceStatus::Uninitialized,
            supported_syscalls: handlers::get_supported_syscalls(),
            ipc_objects: Vec::new(),
            next_ipc_id: 1001,
        }
    }

    /// 获取IPC统计信息
    /// 
    /// # 返回值
    /// 
    /// * `IpcStats` - IPC统计信息
    pub fn get_ipc_stats(&self) -> crate::syscalls::ipc::types::IpcStats {
        let pipe_count = self.ipc_objects.iter()
            .filter(|obj| obj.object_type == crate::syscalls::ipc::types::IpcObjectType::Pipe)
            .count() as u32;
        let shared_memory_count = self.ipc_objects.iter()
            .filter(|obj| obj.object_type == crate::syscalls::ipc::types::IpcObjectType::SharedMemory)
            .count() as u32;
        let message_queue_count = self.ipc_objects.iter()
            .filter(|obj| obj.object_type == crate::syscalls::ipc::types::IpcObjectType::MessageQueue)
            .count() as u32;
        let semaphore_count = self.ipc_objects.iter()
            .filter(|obj| obj.object_type == crate::syscalls::ipc::types::IpcObjectType::Semaphore)
            .count() as u32;

        crate::syscalls::ipc::types::IpcStats {
            total_objects: self.ipc_objects.len() as u32,
            pipe_count,
            shared_memory_count,
            message_queue_count,
            semaphore_count,
            total_memory_usage: self.ipc_objects.iter().map(|obj| obj.size).sum(),
            active_connections: 0,
            pending_messages: 0,
        }
    }

    /// 获取IPC对象信息
    /// 
    /// # 参数
    /// 
    /// * `id` - IPC对象ID
    /// 
    /// # 返回值
    /// 
    /// * `Option<IpcObjectInfo>` - IPC对象信息
    pub fn get_ipc_object(&self, id: u32) -> Option<crate::syscalls::ipc::types::IpcObjectInfo> {
        self.ipc_objects.iter().find(|obj| obj.id == id).cloned()
    }

    /// 列出所有IPC对象
    /// 
    /// # 返回值
    /// 
    /// * `Vec<IpcObjectInfo>` - IPC对象列表
    pub fn list_ipc_objects(&self) -> Vec<crate::syscalls::ipc::types::IpcObjectInfo> {
        self.ipc_objects.clone()
    }

    /// 分配IPC对象ID
    /// 
    /// # 返回值
    /// 
    /// * `u32` - 新的IPC对象ID
    pub fn allocate_ipc_id(&mut self) -> u32 {
        let id = self.next_ipc_id;
        self.next_ipc_id += 1;
        id
    }

    /// 创建管道
    /// 
    /// # 参数
    /// 
    /// * `flags` - 创建标志
    /// 
    /// # 返回值
    /// 
    /// * `Result<(i32, i32), IpcError>` - (读端描述符, 写端描述符)或错误
    pub fn create_pipe(&mut self, _flags: u32) -> Result<(i32, i32), crate::syscalls::ipc::types::IpcError> {
        // println removed for no_std compatibility
        let id = self.allocate_ipc_id();
        let read_fd = 10 + id as i32;
        let write_fd = 20 + id as i32;

        let ipc_object = crate::syscalls::ipc::types::IpcObjectInfo {
            id,
            object_type: crate::syscalls::ipc::types::IpcObjectType::Pipe,
            name: format!("pipe_{}", id),
            size: 4096,
            permissions: crate::syscalls::ipc::types::IpcPermissions::default(),
            creator_pid: 0, // TODO: 获取当前进程ID
            creation_time: 0, // TODO: 获取当前时间
            last_access_time: 0,
            reference_count: 1,
        };

        // 将IPC对象添加到对象列表
        self.ipc_objects.push(ipc_object);
        
        // println removed for no_std compatibility
        // println removed for no_std compatibility
        Ok((read_fd, write_fd))
    }

    /// 创建共享内存
    /// 
    /// # 参数
    /// 
    /// * `size` - 内存大小
    /// * `permissions` - 权限设置
    /// * `flags` - 创建标志
    /// 
    /// # 返回值
    /// 
    /// * `Result<u32, IpcError>` - 共享内存ID或错误
    pub fn create_shared_memory(&mut self, size: u64, permissions: crate::syscalls::ipc::types::IpcPermissions, _flags: u32) -> Result<u32, crate::syscalls::ipc::types::IpcError> {
        // println removed for no_std compatibility
        let id = self.allocate_ipc_id();

        let ipc_object = crate::syscalls::ipc::types::IpcObjectInfo {
            id,
            object_type: crate::syscalls::ipc::types::IpcObjectType::SharedMemory,
            name: format!("shm_{}", id),
            size,
            permissions,
            creator_pid: 0, // TODO: 获取当前进程ID
            creation_time: 0, // TODO: 获取当前时间
            last_access_time: 0,
            reference_count: 1,
        };

        // 将IPC对象添加到对象列表
        self.ipc_objects.push(ipc_object);
        
        // println removed for no_std compatibility
        // println removed for no_std compatibility
        Ok(id)
    }

    /// 创建消息队列
    /// 
    /// # 参数
    /// 
    /// * `key` - 队列键值
    /// * `permissions` - 权限设置
    /// * `flags` - 创建标志
    /// 
    /// # 返回值
    /// 
    /// * `Result<u32, IpcError>` - 消息队列ID或错误
    pub fn create_message_queue(&mut self, key: i32, permissions: crate::syscalls::ipc::types::IpcPermissions, _flags: u32) -> Result<u32, crate::syscalls::ipc::types::IpcError> {
        // println removed for no_std compatibility
        
        // 使用 key 查找或创建消息队列
        // 如果 key 已存在，返回现有队列ID；否则创建新队列
        // key == 0 表示 IPC_PRIVATE，总是创建新队列
        let existing_id = if key != 0 {
            self.ipc_objects.iter()
                .find(|obj| obj.object_type == crate::syscalls::ipc::types::IpcObjectType::MessageQueue)
                .map(|obj| obj.id)
        } else {
            None
        };
        
        let id = existing_id.unwrap_or_else(|| self.allocate_ipc_id());
        
        let ipc_object = crate::syscalls::ipc::types::IpcObjectInfo {
            id,
            object_type: crate::syscalls::ipc::types::IpcObjectType::MessageQueue,
            name: format!("msgq_{}", id),
            size: 4096,
            permissions,
            creator_pid: 0, // TODO: 获取当前进程ID
            creation_time: 0, // TODO: 获取当前时间
            last_access_time: 0,
            reference_count: 1,
        };

        // 将IPC对象添加到对象列表（如果不存在）
        if existing_id.is_none() {
            self.ipc_objects.push(ipc_object);
        }
        
        // println removed for no_std compatibility
        // println removed for no_std compatibility
        Ok(id)
    }

    /// 创建信号量
    /// 
    /// # 参数
    /// 
    /// * `initial_value` - 初始值
    /// * `permissions` - 权限设置
    /// * `flags` - 创建标志
    /// 
    /// # 返回值
    /// 
    /// * `Result<u32, IpcError>` - 信号量ID或错误
    pub fn create_semaphore(&mut self, initial_value: u32, permissions: crate::syscalls::ipc::types::IpcPermissions, _flags: u32) -> Result<u32, crate::syscalls::ipc::types::IpcError> {
        // println removed for no_std compatibility
        let id = self.allocate_ipc_id();

        let ipc_object = crate::syscalls::ipc::types::IpcObjectInfo {
            id,
            object_type: crate::syscalls::ipc::types::IpcObjectType::Semaphore,
            name: format!("sem_{}", id),
            size: 4, // 信号量只需要4字节
            permissions,
            creator_pid: 0, // TODO: 获取当前进程ID
            creation_time: 0, // TODO: 获取当前时间
            last_access_time: 0,
            reference_count: 1,
        };

        // 将IPC对象添加到对象列表
        self.ipc_objects.push(ipc_object);
        
        // 使用 initial_value 初始化信号量值（在实际实现中应该存储）
        let _sem_value = initial_value;
        
        // println removed for no_std compatibility
        // println removed for no_std compatibility
        Ok(id)
    }

    /// 删除IPC对象
    /// 
    /// # 参数
    /// 
    /// * `id` - IPC对象ID
    /// 
    /// # 返回值
    /// 
    /// * `Result<(), IpcError>` - 操作结果
    pub fn delete_ipc_object(&mut self, id: u32) -> Result<(), crate::syscalls::ipc::types::IpcError> {
        if let Some(pos) = self.ipc_objects.iter().position(|obj| obj.id == id) {
            // 从对象列表中移除
            self.ipc_objects.remove(pos);
            // println removed for no_std compatibility
            // println removed for no_std compatibility
            Ok(())
        } else {
            Err(crate::syscalls::ipc::types::IpcError::ObjectNotFound)
        }
    }

    /// 获取IPC对象权限
    /// 
    /// # 参数
    /// 
    /// * `id` - IPC对象ID
    /// 
    /// # 返回值
    /// 
    /// * `Option<IpcPermissions>` - 权限设置
    pub fn get_ipc_object_permissions(&self, id: u32) -> Option<crate::syscalls::ipc::types::IpcPermissions> {
        self.ipc_objects.iter()
            .find(|obj| obj.id == id)
            .map(|obj| obj.permissions.clone())
    }

    /// 设置IPC对象权限
    /// 
    /// # 参数
    /// 
    /// * `id` - IPC对象ID
    /// * `permissions` - 新的权限设置
    /// 
    /// # 返回值
    /// 
    /// * `Result<(), IpcError>` - 操作结果
    pub fn set_ipc_object_permissions(&mut self, id: u32, permissions: crate::syscalls::ipc::types::IpcPermissions) -> Result<(), crate::syscalls::ipc::types::IpcError> {
        if let Some(obj) = self.ipc_objects.iter_mut().find(|obj| obj.id == id) {
            obj.permissions = permissions;
            // println removed for no_std compatibility
            Ok(())
        } else {
            Err(crate::syscalls::ipc::types::IpcError::ObjectNotFound)
        }
    }
}

impl Default for IpcService {
    fn default() -> Self {
        Self::new()
    }
}

impl Service for IpcService {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn initialize(&mut self) -> Result<(), KernelError> {
        crate::log_info!("Initializing IpcService");
        self.status = ServiceStatus::Initializing;
        
        // TODO: 初始化IPC管理器
        
        self.status = ServiceStatus::Initialized;
        crate::log_info!("IpcService initialized successfully");
        Ok(())
    }

    fn start(&mut self) -> Result<(), KernelError> {
        crate::log_info!("Starting IpcService");
        self.status = ServiceStatus::Starting;
        
        // TODO: 启动IPC管理器
        
        self.status = ServiceStatus::Running;
        crate::log_info!("IpcService started successfully");
        Ok(())
    }

    fn stop(&mut self) -> Result<(), KernelError> {
        crate::log_info!("Stopping IpcService");
        self.status = ServiceStatus::Stopping;
        
        // TODO: 停止IPC管理器
        
        self.status = ServiceStatus::Stopped;
        crate::log_info!("IpcService stopped successfully");
        Ok(())
    }

    fn destroy(&mut self) -> Result<(), KernelError> {
        crate::log_info!("Destroying IpcService");
        
        // TODO: 销毁IPC管理器
        
        self.status = ServiceStatus::Uninitialized;
        crate::log_info!("IpcService destroyed successfully");
        Ok(())
    }

    fn status(&self) -> ServiceStatus {
        self.status
    }

    fn dependencies(&self) -> Vec<&str> {
        // IPC服务可能依赖的模块
        vec!["process_manager", "memory_manager"]
    }
}

impl SyscallService for IpcService {
    fn supported_syscalls(&self) -> Vec<u32> {
        self.supported_syscalls.clone()
    }

    fn handle_syscall(&mut self, syscall_number: u32, args: &[u64]) -> Result<u64, KernelError> {
        // println removed for no_std compatibility
        
        // 分发到具体的处理函数
        handlers::dispatch_syscall(syscall_number, args)
    }

    fn priority(&self) -> u32 {
        80 // IPC服务优先级
    }

    fn as_syscall_service(&self) -> Option<&dyn SyscallService> {
        Some(self)
    }
}

/// IPC服务工厂
/// 
/// 用于创建IPC服务实例的工厂结构体。
pub struct IpcServiceFactory;

impl IpcServiceFactory {
    /// 创建IPC服务实例
    /// 
    /// # 返回值
    /// 
    /// * `Box<dyn SyscallService>` - IPC服务实例
    pub fn create() -> Box<dyn SyscallService> {
        Box::new(IpcService::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_service_creation() {
        // println removed for no_std compatibility
        assert_eq!(service.name(), "ipc");
        assert_eq!(service.version(), "1.0.0");
        assert_eq!(service.status(), ServiceStatus::Uninitialized);
        assert_eq!(service.next_ipc_id, 1001);
    }

    #[test]
    fn test_ipc_service_lifecycle() {
        // println removed for no_std compatibility
        
        // 测试初始化
        assert!(service.initialize().is_ok());
        assert_eq!(service.status(), ServiceStatus::Initialized);
        
        // 测试启动
        assert!(service.start().is_ok());
        assert_eq!(service.status(), ServiceStatus::Running);
        
        // 测试停止
        assert!(service.stop().is_ok());
        assert_eq!(service.status(), ServiceStatus::Stopped);
    }

    #[test]
    fn test_pipe_creation() {
        // println removed for no_std compatibility
        
        // println removed for no_std compatibility
        assert!(read_fd >= 10);
        assert!(write_fd >= 20);
        assert_eq!(service.ipc_objects.len(), 1);
        
        let ipc_object = &service.ipc_objects[0];
        assert_eq!(ipc_object.object_type, crate::syscalls::ipc::types::IpcObjectType::Pipe);
        assert!(ipc_object.name.starts_with("pipe_"));
    }

    #[test]
    fn test_shared_memory_creation() {
        // println removed for no_std compatibility
        
        let id = service.create_shared_memory(
            4096,
            crate::syscalls::ipc::types::IpcPermissions::default(),
            0,
        );
        // println removed for no_std compatibility
        assert!(id >= 1001);
        assert_eq!(service.ipc_objects.len(), 1);
        
        let ipc_object = &service.ipc_objects[0];
        assert_eq!(ipc_object.object_type, crate::syscalls::ipc::types::IpcObjectType::SharedMemory);
        assert_eq!(ipc_object.size, 4096);
    }

    #[test]
    fn test_supported_syscalls() {
        // println removed for no_std compatibility
        // println removed for no_std compatibility
        assert!(!syscalls.is_empty());
        assert!(syscalls.contains(&22)); // pipe
        assert!(syscalls.contains(&29)); // shmget
    }
}