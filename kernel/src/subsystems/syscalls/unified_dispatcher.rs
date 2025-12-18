//! 统一系统调用分发器
//!
//! 本模块提供统一的系统调用分发机制，解决模块间耦合问题：
//! - 解耦系统调用处理逻辑
//! - 统一的性能监控和缓存
//! - 可插拔的优化策略
//! - 标准化的错误处理

use crate::syscalls::common::{SyscallError, SyscallResult};
use nos_nos_error_handling::unified_engine::PerformanceMonitor;
use crate::syscalls::optimization_core::{
    UnifiedCache, UnifiedSyscallStats, CacheConfig, EvictionPolicy, OptimizationConfig
};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

/// 系统调用处理器特征
/// 
/// 所有系统调用处理器都必须实现此特征，以实现解耦
pub trait SyscallHandler: Send + Sync {
    /// 处理系统调用
    fn handle(&self, syscall_num: u32, args: &[u64]) -> SyscallResult;
    
    /// 获取处理器名称
    fn name(&self) -> &str;
    
    /// 获取支持的系统调用范围
    fn supported_range(&self) -> (u32, u32);
    
    /// 获取处理器优先级
    fn priority(&self) -> u8;
    
    /// 判断是否为纯函数（可缓存）
    fn is_pure(&self, syscall_num: u32) -> bool {
        false
    }
    
    /// 获取预估执行时间（纳秒）
    fn estimated_time_ns(&self, syscall_num: u32) -> u64 {
        1000 // 默认1微秒
    }
}

/// 系统调用处理器注册信息
#[derive(Debug, Clone)]
pub struct HandlerRegistration {
    /// 处理器实例
    pub handler: Arc<dyn SyscallHandler>,
    /// 注册时间戳
    pub registration_time: u64,
    /// 是否启用
    pub enabled: bool,
}

/// 统一系统调用分发器
pub struct UnifiedDispatcher {
    /// 处理器注册表
    /// 键：系统调用号，值：处理器注册信息
    handlers: Arc<Mutex<BTreeMap<u32, HandlerRegistration>>>,
    
    /// 系统调用范围映射
    /// 用于快速查找处理器
    range_map: Arc<Mutex<Vec<(u32, u32, Arc<dyn SyscallHandler>)>>>,
    
    /// 结果缓存
    cache: Arc<Mutex<UnifiedCache<u64, SyscallResult>>>,
    
    /// 性能监控器
    performance_monitor: Arc<Mutex<Option<PerformanceMonitor>>>,
    
    /// 分发统计
    dispatch_stats: UnifiedSyscallStats,
    
    /// 配置
    config: DispatcherConfig,
}

/// 分发器配置
#[derive(Debug, Clone)]
pub struct DispatcherConfig {
    /// 是否启用缓存
    pub enable_cache: bool,
    /// 是否启用性能监控
    pub enable_performance_monitoring: bool,
    /// 是否启用自适应优化
    pub enable_adaptive_optimization: bool,
    /// 缓存配置
    pub cache_config: CacheConfig,
    /// 优化配置
    pub optimization_config: OptimizationConfig,
}

impl Default for DispatcherConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            enable_performance_monitoring: true,
            enable_adaptive_optimization: true,
            cache_config: CacheConfig::default(),
            optimization_config: OptimizationConfig::default(),
        }
    }
}

/// 分发结果
#[derive(Debug, Clone)]
pub struct DispatchResult {
    /// 是否成功
    pub success: bool,
    /// 返回值
    pub return_value: u64,
    /// 执行时间（纳秒）
    pub execution_time_ns: u64,
    /// 是否来自缓存
    pub from_cache: bool,
    /// 处理器名称
    pub handler_name: String,
    /// 错误信息（如果有）
    pub error: Option<SyscallError>,
}

impl UnifiedDispatcher {
    /// 创建新的统一分发器
    pub fn new(config: DispatcherConfig) -> Self {
        let cache = UnifiedCache::new(config.cache_config.clone());
        let performance_monitor = if config.enable_performance_monitoring {
            Some(PerformanceMonitor::new(config.optimization_config.clone()))
        } else {
            None
        };
        
        Self {
            handlers: Arc::new(Mutex::new(BTreeMap::new())),
            range_map: Arc::new(Mutex::new(Vec::new())),
            cache: Arc::new(Mutex::new(cache)),
            performance_monitor: Arc::new(Mutex::new(performance_monitor)),
            dispatch_stats: UnifiedSyscallStats::new(),
            config,
        }
    }
    
    /// 注册系统调用处理器
    pub fn register_handler(&self, handler: Arc<dyn SyscallHandler>) -> Result<(), SyscallError> {
        let (start, end) = handler.supported_range();
        
        // 验证范围有效性
        if start > end {
            return Err(SyscallError::InvalidArgument);
        }
        
        let registration = HandlerRegistration {
            handler: handler.clone(),
            registration_time: self.get_current_timestamp(),
            enabled: true,
        };
        
        // 注册到处理器表
        {
            let mut handlers = self.handlers.lock();
            for syscall_num in start..=end {
                handlers.insert(syscall_num, registration.clone());
            }
        }
        
        // 注册到范围映射
        {
            let mut range_map = self.range_map.lock();
            range_map.push((start, end, handler));
            // 按优先级排序
            range_map.sort_by(|a, b| b.2.priority().cmp(&a.2.priority()));
        }
        
        Ok(())
    }
    
    /// 注销系统调用处理器
    pub fn unregister_handler(&self, handler_name: &str) -> Result<(), SyscallError> {
        let mut handlers = self.handlers.lock();
        let mut range_map = self.range_map.lock();
        
        // 找到要注销的处理器
        let mut to_remove = Vec::new();
        for (syscall_num, registration) in handlers.iter() {
            if registration.handler.name() == handler_name {
                to_remove.push(*syscall_num);
            }
        }
        
        // 移除处理器
        for syscall_num in to_remove {
            handlers.remove(&syscall_num);
        }
        
        // 重建范围映射
        range_map.retain(|(_, _, h)| h.name() != handler_name);
        
        Ok(())
    }
    
    /// 分发系统调用
    pub fn dispatch(&self, syscall_num: u32, args: &[u64]) -> DispatchResult {
        let start_time = self.get_current_timestamp();
        
        // 检查缓存
        let cache_key = self.calculate_cache_key(syscall_num, args);
        let from_cache = if self.config.enable_cache {
            let mut cache = self.cache.lock();
            cache.get(&cache_key).map(|result| {
                self.record_dispatch_result(syscall_num, true, true, 0);
                result
            })
        } else {
            None
        };
        
        if let Some(cached_result) = from_cache {
            return match cached_result {
                Ok(value) => DispatchResult {
                    success: true,
                    return_value: value,
                    execution_time_ns: 0,
                    from_cache: true,
                    handler_name: "cache".to_string(),
                    error: None,
                },
                Err(error) => DispatchResult {
                    success: false,
                    return_value: 0,
                    execution_time_ns: 0,
                    from_cache: true,
                    handler_name: "cache".to_string(),
                    error: Some(error),
                },
            };
        }
        
        // 查找处理器
        let handler = self.find_handler(syscall_num);
        
        let result = if let Some(handler) = handler {
            // 执行系统调用
            let handler_start = self.get_current_timestamp();
            let result = handler.handle(syscall_num, args);
            let handler_end = self.get_current_timestamp();
            let execution_time = handler_end - handler_start;
            
            // 记录性能数据
            self.record_performance(syscall_num, execution_time, result.is_ok(), false);
            
            // 缓存结果
            if self.config.enable_cache && handler.is_pure(syscall_num) {
                let mut cache = self.cache.lock();
                cache.set(cache_key, result.clone(), Some(self.config.cache_config.default_ttl_ns));
            }
            
            result
        } else {
            Err(SyscallError::InvalidSyscall)
        };
        
        let end_time = self.get_current_timestamp();
        let total_execution_time = end_time - start_time;
        
        // 记录分发统计
        self.record_dispatch_result(syscall_num, result.is_ok(), false, total_execution_time);
        
        match result {
            Ok(value) => DispatchResult {
                success: true,
                return_value: value,
                execution_time_ns: total_execution_time,
                from_cache: false,
                handler_name: handler.map(|h| h.name().to_string()).unwrap_or_default(),
                error: None,
            },
            Err(error) => DispatchResult {
                success: false,
                return_value: 0,
                execution_time_ns: total_execution_time,
                from_cache: false,
                handler_name: handler.map(|h| h.name().to_string()).unwrap_or_default(),
                error: Some(error),
            },
        }
    }
    
    /// 批量分发系统调用
    pub fn batch_dispatch(&self, requests: &[(u32, Vec<u64>)]) -> Vec<DispatchResult> {
        requests
            .iter()
            .map(|(syscall_num, args)| self.dispatch(*syscall_num, args))
            .collect()
    }
    
    /// 查找处理器
    fn find_handler(&self, syscall_num: u32) -> Option<Arc<dyn SyscallHandler>> {
        let handlers = self.handlers.lock();
        handlers.get(&syscall_num).map(|reg| reg.handler.clone())
    }
    
    /// 计算缓存键
    fn calculate_cache_key(&self, syscall_num: u32, args: &[u64]) -> u64 {
        let mut key = syscall_num as u64;
        
        for (i, &arg) in args.iter().enumerate() {
            key ^= arg.rotate_left(i * 8);
        }
        
        key
    }
    
    /// 记录性能数据
    fn record_performance(&self, syscall_num: u32, execution_time_ns: u64, success: bool, from_cache: bool) {
        if let Some(ref monitor) = *self.performance_monitor.lock() {
            monitor.record_syscall_performance(syscall_num, execution_time_ns, success, from_cache);
        }
    }
    
    /// 记录分发结果
    fn record_dispatch_result(&self, syscall_num: u32, success: bool, from_cache: bool, execution_time_ns: u64) {
        self.dispatch_stats.record_call(execution_time_ns, success, from_cache);
    }
    
    /// 获取当前时间戳
    fn get_current_timestamp(&self) -> u64 {
        // 这里应该实现真实的时间戳获取
        // 暂时返回固定值
        0
    }
    
    /// 获取分发统计信息
    pub fn get_dispatch_stats(&self) -> crate::syscalls::optimization_core::SyscallStatsSnapshot {
        self.dispatch_stats.get_snapshot()
    }
    
    /// 获取缓存统计信息
    pub fn get_cache_stats(&self) -> Option<crate::syscalls::optimization_core::SyscallStatsSnapshot> {
        if self.config.enable_cache {
            let cache = self.cache.lock();
            Some(cache.get_stats())
        } else {
            None
        }
    }
    
    /// 获取性能监控统计
    pub fn get_performance_stats(&self) -> Option<crate::syscalls::optimization_core::SyscallStatsSnapshot> {
        let monitor_guard = self.performance_monitor.lock();
        monitor_guard.as_ref().map(|monitor| monitor.get_global_stats())
    }
    
    /// 获取所有已注册的处理器
    pub fn get_registered_handlers(&self) -> Vec<String> {
        let handlers = self.handlers.lock();
        let mut names = Vec::new();
        let mut seen = BTreeMap::new();
        
        for registration in handlers.values() {
            let name = registration.handler.name();
            if !seen.contains_key(&name.to_string()) {
                names.push(name.to_string());
                seen.insert(name.to_string(), true);
            }
        }
        
        names
    }
    
    /// 启用/禁用处理器
    pub fn set_handler_enabled(&self, handler_name: &str, enabled: bool) -> Result<(), SyscallError> {
        let mut handlers = self.handlers.lock();
        let mut found = false;
        
        for registration in handlers.values_mut() {
            if registration.handler.name() == handler_name {
                registration.enabled = enabled;
                found = true;
            }
        }
        
        if found {
            Ok(())
        } else {
            Err(SyscallError::NotFound)
        }
    }
    
    /// 清空缓存
    pub fn clear_cache(&self) {
        if self.config.enable_cache {
            let mut cache = self.cache.lock();
            cache.clear();
        }
    }
    
    /// 重置统计信息
    pub fn reset_stats(&self) {
        self.dispatch_stats.reset();
        
        if let Some(ref monitor) = *self.performance_monitor.lock() {
            monitor.reset_all_stats();
        }
        
        if self.config.enable_cache {
            let mut cache = self.cache.lock();
            cache.reset_stats();
        }
    }
}

/// 全局统一分发器实例
static GLOBAL_UNIFIED_DISPATCHER: Mutex<Option<UnifiedDispatcher>> = Mutex::new(None);

/// 初始化全局统一分发器
pub fn init_global_unified_dispatcher(config: DispatcherConfig) {
    let mut dispatcher_guard = GLOBAL_UNIFIED_DISPATCHER.lock();
    *dispatcher_guard = Some(UnifiedDispatcher::new(config));
}

/// 获取全局统一分发器
pub fn get_global_unified_dispatcher() -> Option<&'static Mutex<Option<UnifiedDispatcher>>> {
    Some(&GLOBAL_UNIFIED_DISPATCHER)
}

/// 便捷函数：分发系统调用
pub fn unified_dispatch(syscall_num: u32, args: &[u64]) -> DispatchResult {
    if let Some(dispatcher_guard) = get_global_unified_dispatcher() {
        let dispatcher = dispatcher_guard.lock();
        if let Some(ref dispatcher) = *dispatcher {
            dispatcher.dispatch(syscall_num, args)
        } else {
            DispatchResult {
                success: false,
                return_value: 0,
                execution_time_ns: 0,
                from_cache: false,
                handler_name: "none".to_string(),
                error: Some(SyscallError::InvalidSyscall),
            }
        }
    } else {
        DispatchResult {
            success: false,
            return_value: 0,
            execution_time_ns: 0,
            from_cache: false,
            handler_name: "none".to_string(),
            error: Some(SyscallError::InvalidSyscall),
        }
    }
}

/// 便捷函数：批量分发系统调用
pub fn unified_batch_dispatch(requests: &[(u32, Vec<u64>)]) -> Vec<DispatchResult> {
    if let Some(dispatcher_guard) = get_global_unified_dispatcher() {
        let dispatcher = dispatcher_guard.lock();
        if let Some(ref dispatcher) = *dispatcher {
            dispatcher.batch_dispatch(requests)
        } else {
            vec![]
        }
    } else {
        vec![]
    }
}
