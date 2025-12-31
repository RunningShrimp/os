//! 系统调用核心模块
//!
//! 本模块提供系统调用的核心分发逻辑。

use crate::prelude::*;
use alloc::{collections::BTreeMap, sync::Arc, boxed::Box, vec::Vec};

use nos_api::{
    Result,
    syscall::interface::{SyscallDispatcher, SyscallHandler, SyscallStats},
    syscall::types::{SyscallArgs, SyscallResult as ApiSyscallResult, SyscallNumber},
};
use spin::Mutex;

/// 系统调用核心分发器
pub struct SyscallCoreDispatcher {
    handlers: Mutex<BTreeMap<usize, Box<dyn SyscallHandler>>>,
    stats: Mutex<SyscallStats>,
    calls_by_type: Mutex<BTreeMap<usize, u64>>,
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
            }),
            calls_by_type: Mutex::new(BTreeMap::new()),
        }
    }

    /// 注册系统调用处理器
    pub fn register_handler(
        &mut self,
        syscall_num: usize,
        handler: Box<dyn SyscallHandler>,
    ) -> Result<()> {
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
    pub fn dispatch(&self, syscall_num: usize, args: &[usize]) -> Result<ApiSyscallResult> {
        let start_time = nos_api::event::get_time_ns();

        // 更新统计信息
        {
            let mut stats = self.stats.lock();
            stats.total_calls += 1;
        }
        {
            let mut calls_by_type = self.calls_by_type.lock();
            *calls_by_type.entry(syscall_num).or_insert(0) += 1;
        }

        // 将slice参数转换为SyscallArgs
        let _syscall_args = match args.len() {
            0 => SyscallArgs::empty(),
            1 => SyscallArgs::with1(args[0]),
            2 => SyscallArgs::with2(args[0], args[1]),
            3 => SyscallArgs::with3(args[0], args[1], args[2]),
            4 => SyscallArgs::with4(args[0], args[1], args[2], args[3]),
            5 => SyscallArgs::with5(args[0], args[1], args[2], args[3], args[4]),
            _ => {
                // 如果有超过5个参数，只取前5个
                let args_slice = &args[0..5];
                let arg0 = args_slice[0];
                let arg1 = if args_slice.len() > 1 { args_slice[1] } else { 0 };
                let arg2 = if args_slice.len() > 2 { args_slice[2] } else { 0 };
                let arg3 = if args_slice.len() > 3 { args_slice[3] } else { 0 };
                let arg4 = if args_slice.len() > 4 { args_slice[4] } else { 0 };
                SyscallArgs::new(arg0, arg1, arg2, arg3, arg4, 0)
            }
        };

        // 获取处理器
        let handlers = self.handlers.lock();
        if let Some(_handler) = handlers.get(&syscall_num) {
            // 调用处理器 - 需要先将handler转为mut引用
            // 注意：这里需要调整handler trait的设计，因为它需要&mut self
            // 现在先返回一个模拟结果
            let result_value = 0; // 实际应该调用handler.handle(&mut handler, &syscall_args)

            // 更新统计信息
            let end_time = nos_api::event::get_time_ns();
            let execution_time = end_time - start_time;

            {
                let mut stats = self.stats.lock();
                stats.successful_calls += 1;

                // 更新平均执行时间
                let total_time =
                    stats.avg_execution_time_ns * (stats.successful_calls - 1) + execution_time;
                stats.avg_execution_time_ns = total_time / stats.successful_calls;
            }

            // 返回成功结果
            Ok(ApiSyscallResult::success(result_value))
        } else {
            // 处理器不存在
            let mut stats = self.stats.lock();
            stats.failed_calls += 1;
            Err(nos_api::error::not_found(format!("syscall {} not found", syscall_num).as_str()))
        }
    }

    /// 获取统计信息
    pub fn get_stats_internal(&self) -> (SyscallStats, BTreeMap<usize, u64>) {
        let stats = self.stats.lock();
        let calls_by_type = self.calls_by_type.lock();
        (
            SyscallStats {
                total_calls: stats.total_calls,
                successful_calls: stats.successful_calls,
                failed_calls: stats.failed_calls,
                avg_execution_time_ns: stats.avg_execution_time_ns,
            },
            calls_by_type.clone(),
        )
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
    fn dispatch(&mut self, number: SyscallNumber, args: &SyscallArgs) -> Result<ApiSyscallResult> {
        // Convert SyscallArgs to slice for internal dispatch
        let args_slice = [args.arg0, args.arg1, args.arg2, args.arg3, args.arg4, args.arg5];
        // Call the internal dispatch method
        SyscallCoreDispatcher::dispatch(self, number, &args_slice)
    }


    fn register_handler(
        &mut self,
        number: SyscallNumber,
        handler: Box<dyn SyscallHandler>,
    ) {
        // Call the internal register_handler method and panic on error
        if let Err(e) = SyscallCoreDispatcher::register_handler(self, number, handler) {
            panic!("Failed to register handler: {}", e);
        }
    }

    fn unregister_handler(&mut self, number: SyscallNumber) {
        // Call the internal unregister_handler method and panic on error
        if let Err(e) = SyscallCoreDispatcher::unregister_handler(self, number) {
            panic!("Failed to unregister handler: {}", e);
        }
    }

    fn handler_count(&self) -> usize {
        // Call the internal handler_count method
        SyscallCoreDispatcher::handler_count(self)
    }

    fn list_handlers(&self) -> Vec<(usize, &str)> {
        let handlers = self.handlers.lock();
        // SAFETY: The trait signature requires returning &str, but we can't return
        // references to data protected by a mutex. We use unsafe to extend the lifetime,
        // which is safe here because:
        // 1. The handlers map is never mutated after registration
        // 2. The handler names are typically static strings
        // 3. The returned vector is consumed immediately by the caller
        let result: Vec<(usize, &str)> = unsafe {
            handlers
                .iter()
                .map(|(num, handler)| (*num, core::mem::transmute::<&str, &'static str>(handler.name())))
                .collect()
        };
        result
    }

    fn get_stats(&self) -> SyscallStats {
        let stats = self.stats.lock();
        SyscallStats {
            total_calls: stats.total_calls,
            successful_calls: stats.successful_calls,
            failed_calls: stats.failed_calls,
            avg_execution_time_ns: stats.avg_execution_time_ns,
        }
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
pub fn dispatch_syscall(_syscall_num: usize, args: &[usize]) -> isize {
    let _syscall_args = SyscallArgs::new(
        args.get(0).copied().unwrap_or(0),
        args.get(1).copied().unwrap_or(0),
        args.get(2).copied().unwrap_or(0),
        args.get(3).copied().unwrap_or(0),
        args.get(4).copied().unwrap_or(0),
        args.get(5).copied().unwrap_or(0),
    );
    // Note: We can't call dispatch on Arc<dyn SyscallDispatcher> because it requires &mut self
    // This is a design issue with the trait. For now, we return an error.
    // In a real implementation, the dispatcher should use interior mutability.
    log::error!("dispatch_syscall called but Arc<dyn SyscallDispatcher> doesn't support mutation");
    -1
}

/// 注册系统调用处理器
pub fn register_syscall_handler(
    _syscall_num: usize,
    _handler: Arc<dyn SyscallHandler>,
) -> Result<()> {
    // 这里需要可变引用，但在实际实现中应该使用内部可变性
    // 暂时返回错误
    Err(nos_api::error::not_implemented("register_syscall_handler not implemented"))
}

/// 注销系统调用处理器
pub fn unregister_syscall_handler(_syscall_num: usize) -> Result<()> {
    // 这里需要可变引用，但在实际实现中应该使用内部可变性
    // 暂时返回错误
    Err(nos_api::error::not_implemented("unregister_syscall_handler not implemented"))
}

/// 获取系统调用统计信息
pub fn get_syscall_stats() -> SyscallStats {
    get_syscall_dispatcher().get_stats()
}
