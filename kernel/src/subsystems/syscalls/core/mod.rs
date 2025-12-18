//! 系统调用核心模块
//!
//! 本模块提供系统调用的核心分发逻辑。

use nos_api::{Result, interfaces::SyscallDispatcher, interfaces::SyscallHandler, interfaces::SyscallStats};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::Mutex;

/// 系统调用核心分发器
pub struct SyscallCoreDispatcher {
    handlers: Mutex<BTreeMap<usize, Arc<dyn SyscallHandler>>>,
    stats: Mutex<SyscallStats>,
}

impl SyscallCoreDispatcher {
    /// 创建新的系统调用核心分发器
    pub fn new() -> Self {
        Self {
            handlers: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(SyscallStats {
                total_calls: 0,
                successful_calls: 0,
                failed_calls: 0,
                avg_execution_time_ns: 0,
                calls_by_type: BTreeMap::new(),
            }),
        }
    }
    
    /// 注册系统调用处理器
    pub fn register_handler(&self, syscall_num: usize, handler: Arc<dyn SyscallHandler>) -> Result<()> {
        let mut handlers = self.handlers.lock();
        handlers.insert(syscall_num, handler);
        Ok(())
    }
    
    /// 注销系统调用处理器
    pub fn unregister_handler(&self, syscall_num: usize) -> Result<()> {
        let mut handlers = self.handlers.lock();
        handlers.remove(&syscall_num);
        Ok(())
    }
    
    /// 分发系统调用
    pub fn dispatch(&self, syscall_num: usize, args: &[usize]) -> isize {
        let start_time = nos_api::event::get_time_ns();
        
        // 更新统计信息
        {
            let mut stats = self.stats.lock();
            stats.total_calls += 1;
            stats.calls_by_type.insert(syscall_num, stats.calls_by_type.get(&syscall_num).unwrap_or(&0) + 1);
        }
        
        // 获取处理器
        let handlers = self.handlers.lock();
        if let Some(handler) = handlers.get(&syscall_num) {
            // 调用处理器
            let result = handler.handle(args);
            
            // 更新统计信息
            let end_time = nos_api::event::get_time_ns();
            let execution_time = end_time - start_time;
            
            {
                let mut stats = self.stats.lock();
                stats.successful_calls += 1;
                
                // 更新平均执行时间
                let total_time = stats.avg_execution_time_ns * (stats.successful_calls - 1) + execution_time;
                stats.avg_execution_time_ns = total_time / stats.successful_calls;
            }
            
            result
        } else {
            // 处理器未找到
            {
                let mut stats = self.stats.lock();
                stats.failed_calls += 1;
            }
            
            -1 // 错误码
        }
    }
    
    /// 获取统计信息
    pub fn get_stats(&self) -> SyscallStats {
        let stats = self.stats.lock();
        SyscallStats {
            total_calls: stats.total_calls,
            successful_calls: stats.successful_calls,
            failed_calls: stats.failed_calls,
            avg_execution_time_ns: stats.avg_execution_time_ns,
            calls_by_type: stats.calls_by_type.clone(),
        }
    }
    
    /// 列出所有已注册的系统调用
    pub fn list_syscalls(&self) -> Vec<usize> {
        let handlers = self.handlers.lock();
        handlers.keys().cloned().collect()
    }
    
    /// 获取处理器数量
    pub fn handler_count(&self) -> usize {
        let handlers = self.handlers.lock();
        handlers.len()
    }
}

impl SyscallDispatcher for SyscallCoreDispatcher {
    fn dispatch(&self, syscall_num: usize, args: &[usize]) -> isize {
        self.dispatch(syscall_num, args)
    }
    
    fn get_stats(&self) -> SyscallStats {
        self.get_stats()
    }
    
    fn register_handler(&mut self, syscall_num: usize, handler: Arc<dyn SyscallHandler>) -> Result<()> {
        self.register_handler(syscall_num, handler)
    }
    
    fn unregister_handler(&mut self, syscall_num: usize) -> Result<()> {
        self.unregister_handler(syscall_num)
    }
    
    fn handler_count(&self) -> usize {
        self.handler_count()
    }
    
    fn list_handlers(&self) -> Vec<&str> {
        let handlers = self.handlers.lock();
        handlers.values().map(|h| h.name()).collect()
    }
}

/// 全局系统调用分发器
static mut GLOBAL_SYSCALL_DISPATCHER: Option<Arc<dyn SyscallDispatcher>> = None;
static SYSCALL_DISPATCHER_INIT: Mutex<bool> = Mutex::new(false);

/// 初始化全局系统调用分发器
pub fn init_syscall_dispatcher() -> Result<()> {
    let mut is_init = SYSCALL_DISPATCHER_INIT.lock();
    if *is_init {
        return Ok(());
    }
    
    unsafe {
        GLOBAL_SYSCALL_DISPATCHER = Some(Arc::new(SyscallCoreDispatcher::new()));
    }
    *is_init = true;
    Ok(())
}

/// 获取全局系统调用分发器
pub fn get_syscall_dispatcher() -> Arc<dyn SyscallDispatcher> {
    unsafe {
        GLOBAL_SYSCALL_DISPATCHER
            .as_ref()
            .expect("Syscall dispatcher not initialized")
            .clone()
    }
}

/// 分发系统调用
pub fn dispatch_syscall(syscall_num: usize, args: &[usize]) -> isize {
    get_syscall_dispatcher().dispatch(syscall_num, args)
}

/// 注册系统调用处理器
pub fn register_syscall_handler(syscall_num: usize, handler: Arc<dyn SyscallHandler>) -> Result<()> {
    // 这里需要可变引用，但在实际实现中应该使用内部可变性
    // 暂时返回错误
    Err(nos_api::error::not_implemented("register_syscall_handler not implemented"))
}

/// 注销系统调用处理器
pub fn unregister_syscall_handler(syscall_num: usize) -> Result<()> {
    // 这里需要可变引用，但在实际实现中应该使用内部可变性
    // 暂时返回错误
    Err(nos_api::error::not_implemented("unregister_syscall_handler not implemented"))
}

/// 获取系统调用统计信息
pub fn get_syscall_stats() -> SyscallStats {
    get_syscall_dispatcher().get_stats()
}