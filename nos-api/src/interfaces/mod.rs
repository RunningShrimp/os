//! 统一接口定义模块
//!
//! 本模块提供系统调用和服务的抽象接口，用于解决循环依赖问题。
//! 通过定义清晰的接口边界，实现模块间的解耦。

use crate::error::Result;
// Export types that work in both alloc and no-alloc environments
#[cfg(feature = "alloc")]
pub use alloc::boxed::Box;
#[cfg(feature = "alloc")]
pub use alloc::vec::Vec;
#[cfg(feature = "alloc")]
pub use alloc::string::String;
#[cfg(feature = "alloc")]
pub use alloc::sync::Arc;

// Fallback types for no-alloc environment
#[cfg(not(feature = "alloc"))]
pub type Vec<T> = &'static [T];
#[cfg(not(feature = "alloc"))]
pub type String = &'static str;
#[cfg(not(feature = "alloc"))]
pub struct Arc<T: ?Sized> {
    inner: *const T,
}
#[cfg(not(feature = "alloc"))]
pub struct Box<T: ?Sized> {
    inner: *mut T,
}

#[cfg(not(feature = "alloc"))]
impl<T: ?Sized> Arc<T> {
    pub fn new(value: &'static T) -> Self {
        Self { inner: value }
    }
    
    pub fn as_ref(&self) -> &'static T {
        unsafe { &*self.inner }
    }
    
    /// Get the inner pointer (for internal use)
    pub fn as_ptr(&self) -> *const T {
        self.inner
    }
}

#[cfg(not(feature = "alloc"))]
impl<T: ?Sized> core::ops::Deref for Arc<T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner }
    }
}

#[cfg(not(feature = "alloc"))]
impl<T: ?Sized> Clone for Arc<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner }
    }
}

#[cfg(not(feature = "alloc"))]
unsafe impl<T: ?Sized + Send + Sync> Send for Arc<T> {}

#[cfg(not(feature = "alloc"))]
unsafe impl<T: ?Sized + Send + Sync> Sync for Arc<T> {}

#[cfg(not(feature = "alloc"))]
impl<T: ?Sized> From<Box<T>> for Arc<T> {
    fn from(box_value: Box<T>) -> Self {
        Self { inner: box_value.inner }
    }
}

#[cfg(not(feature = "alloc"))]
impl<T: ?Sized + 'static> Arc<T> {
    pub fn downcast<U: 'static>(self) -> Result<Arc<U>> {
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<U>() {
            // This is safe because we've checked the types are the same
            Ok(Arc::<U> { inner: self.inner as *const U })
        } else {
            Err(crate::error::Error::ServiceError("Failed to downcast service instance"))
        }
    }
}

#[cfg(not(feature = "alloc"))]
impl<T: ?Sized> Box<T> {
    pub fn new(value: T) -> Self 
    where
        T: Sized,
    {
        // In no-alloc environment, we leak the value to create a "box"
        let mut boxed = core::mem::ManuallyDrop::new(value);
        Self { 
            inner: &raw mut *boxed as *mut T 
        }
    }
    
    pub fn as_ref(&self) -> &T {
        unsafe { &*self.inner }
    }
    
    pub fn as_mut(&mut self) -> &mut T 
    where
        T: Sized,
    {
        unsafe { &mut *self.inner }
    }
    
    /// Get the inner pointer (for internal use)
    pub fn as_ptr(&self) -> *mut T {
        self.inner
    }
}

#[cfg(not(feature = "alloc"))]
impl<T: ?Sized> Drop for Box<T> {
    fn drop(&mut self) {
        // In no-alloc environment, we don't actually free the memory
        // This is a limitation of the no-alloc environment
    }
}

/// 系统调用分发器接口
pub trait InterfaceSyscallDispatcher: Send + Sync {
    /// 分发系统调用
    fn dispatch(&self, syscall_num: usize, args: &[usize]) -> isize;
    
    /// 获取系统调用统计信息
    fn get_stats(&self) -> SyscallStats;
    
    /// 注册系统调用处理器
    fn register_handler(&mut self, syscall_num: usize, handler: Arc<dyn InterfaceSyscallHandler>) -> Result<()>;
    
    /// 注销系统调用处理器
    fn unregister_handler(&mut self, syscall_num: usize) -> Result<()>;
}

/// 系统调用处理器接口
pub trait InterfaceSyscallHandler: Send + Sync {
    /// 处理系统调用
    fn handle(&self, args: &[usize]) -> isize;
    
    /// 获取处理器名称
    fn name(&self) -> &str;
    
    /// 获取系统调用号
    fn syscall_number(&self) -> usize;
}

/// 系统调用统计信息
#[derive(Debug, Clone)]
pub struct SyscallStats {
    /// 总调用次数
    pub total_calls: u64,
    /// 成功调用次数
    pub successful_calls: u64,
    /// 失败调用次数
    pub failed_calls: u64,
    /// 平均执行时间（纳秒）
    pub avg_execution_time_ns: u64,
    /// 各系统调用统计
    pub calls_by_type: crate::collections::BTreeMap<usize, u64>,
}

/// 服务管理器接口
pub trait InterfaceServiceManager: Send + Sync {
    /// 注册服务
    fn register_service(&mut self, service: Arc<dyn InterfaceService>) -> Result<()>;
    
    /// 注销服务
    fn unregister_service(&mut self, service_id: &str) -> Result<()>;
    
    /// 获取服务
    fn get_service(&self, service_id: &str) -> Option<Arc<dyn InterfaceService>>;
    
    /// 列出所有服务
    fn list_services(&self) -> Vec<InterfaceServiceInfo>;
    
    /// 获取服务统计信息
    fn get_stats(&self) -> InterfaceServiceStats;
}

/// 服务接口
pub trait InterfaceService: Send + Sync {
    /// 获取服务ID
    fn service_id(&self) -> &str;
    
    /// 获取服务名称
    fn name(&self) -> &str;
    
    /// 获取服务版本
    fn version(&self) -> &str;
    
    /// 初始化服务
    fn initialize(&self) -> Result<()>;
    
    /// 启动服务
    fn start(&self) -> Result<()>;
    
    /// 停止服务
    fn stop(&self) -> Result<()>;
    
    /// 清理服务
    fn cleanup(&self) -> Result<()>;

    
    /// 处理服务请求
    fn handle_request(&self, request: &InterfaceServiceRequest) -> Result<InterfaceServiceResponse>;
    
    /// 获取服务状态
    fn status(&self) -> InterfaceServiceStatus;
}

/// 服务信息
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct InterfaceServiceInfo {
    /// 服务ID
    pub id: String,
    /// 服务名称
    pub name: String,
    /// 服务版本
    pub version: String,
    /// 服务状态
    pub status: InterfaceServiceStatus,
    /// 服务描述
    pub description: String,
    /// 服务依赖
    pub dependencies: Vec<String>,
    /// 注册时间
    pub registration_time: u64,
}

#[cfg(not(feature = "alloc"))]
#[derive(Debug, Clone)]
pub struct InterfaceServiceInfo {
    /// 服务ID
    pub id: &'static str,
    /// 服务名称
    pub name: &'static str,
    /// 服务版本
    pub version: &'static str,
    /// 服务状态
    pub status: InterfaceServiceStatus,
    /// 服务描述
    pub description: &'static str,
    /// 服务依赖
    pub dependencies: &'static [&'static str],
    /// 注册时间
    pub registration_time: u64,
}

/// 服务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceServiceStatus {
    /// 未初始化
    Uninitialized,
    /// 已初始化
    Initialized,
    /// 正在启动
    Starting,
    /// 运行中
    Running,
    /// 正在停止
    Stopping,
    /// 已停止
    Stopped,
    /// 错误状态
    Error,
}

/// 服务请求
#[derive(Debug, Clone)]
pub struct InterfaceServiceRequest {
    /// 请求ID
    pub id: String,
    /// 请求类型
    pub request_type: String,
    /// 请求数据
    pub data: Vec<u8>,
    /// 请求时间戳
    pub timestamp: u64,
}

/// 服务响应
#[derive(Debug, Clone)]
pub struct InterfaceServiceResponse {
    /// 请求ID
    pub request_id: String,
    /// 响应状态
    pub status: InterfaceResponseStatus,
    /// 响应数据
    pub data: Vec<u8>,
    /// 响应时间戳
    pub timestamp: u64,
}

/// 响应状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceResponseStatus {
    /// 成功
    Success,
    /// 失败
    Failure,
    /// 超时
    Timeout,
}

/// 服务统计信息
#[derive(Debug, Clone)]
pub struct InterfaceServiceStats {
    /// 注册的服务数量
    pub registered_services: usize,
    /// 运行中的服务数量
    pub running_services: usize,
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 平均响应时间（纳秒）
    pub avg_response_time_ns: u64,
}

/// 事件发布器接口
pub trait InterfaceEventPublisher: Send + Sync {
    /// 发布事件
    fn publish(&self, event: Arc<crate::event::BasicEvent>) -> Result<()>;
    
    /// 批量发布事件
    fn publish_batch(&self, events: Vec<Arc<crate::event::BasicEvent>>) -> Result<()>;
}

/// 事件订阅器接口
pub trait InterfaceEventSubscriber: Send + Sync {
    /// 订阅事件
    fn subscribe(&mut self, event_type: &str, handler: Arc<dyn crate::core::EventHandler<Event = crate::event::BasicEvent>>) -> Result<()>;
    
    /// 取消订阅
    fn unsubscribe(&mut self, event_type: &str, handler_id: &str) -> Result<()>;
}

/// 上下文管理器接口
pub trait InterfaceContextManager: Send + Sync {
    /// 创建上下文
    fn create_context(&self, context_id: &str) -> Result<InterfaceContext>;
    
    /// 获取上下文
    fn get_context(&self, context_id: &str) -> Option<InterfaceContext>;
    
    /// 更新上下文
    fn update_context(&self, context_id: &str, context: InterfaceContext) -> Result<()>;
    
    /// 删除上下文
    fn delete_context(&self, context_id: &str) -> Result<()>;
    
    /// 列出所有上下文
    fn list_contexts(&self) -> Vec<String>;
}

/// 上下文
#[derive(Debug, Clone)]
pub struct InterfaceContext {
    /// 上下文ID
    pub id: String,
    /// 上下文类型
    pub context_type: InterfaceContextType,
    /// 上下文数据
    pub data: Vec<u8>,
    /// 创建时间
    pub creation_time: u64,
    /// 最后访问时间
    pub last_access_time: u64,
}

/// 上下文类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceContextType {
    /// 进程上下文
    Process,
    /// 线程上下文
    Thread,
    /// 系统调用上下文
    Syscall,
    /// 中断上下文
    Interrupt,
    /// 用户上下文
    User,
}
