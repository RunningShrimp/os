//! 系统调用分发器模块
//! 
//! 本模块实现了系统调用的路由和分发功能，包括：
//! - SyscallDispatcher: 核心分发器
//! - 系统调用路由逻辑
//! - 性能监控和统计
//! - 错误处理和日志记录
//! 
//! 分发器是系统调用处理的核心组件，负责将系统调用请求路由到相应的服务。

use crate::error_handling::unified::KernelError;
use crate::syscalls::services::traits::*;
use crate::syscalls::services::registry::ServiceRegistry;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

/// 系统调用分发器
/// 
/// 负责将系统调用请求分发到相应的服务处理器。
/// 提供高性能的路由机制和完善的错误处理。
pub struct SyscallDispatcher {
    /// 服务注册表引用
    registry: Arc<ServiceRegistry>,
    
    /// 系统调用缓存
    /// 
    /// 缓存系统调用号到服务的映射，提高查找性能
    syscall_cache: Arc<Mutex<BTreeMap<u32, CachedServiceInfo>>>,
    
    /// 分发统计信息
    stats: Arc<Mutex<DispatchStats>>,
    
    /// 分发器配置
    config: DispatcherConfig,
}

/// 缓存的服务信息
/// 
/// 包含系统调用处理服务的缓存信息。
#[derive(Debug, Clone)]
pub struct CachedServiceInfo {
    /// 服务名称
    pub service_name: String,
    /// 缓存时间戳
    pub cache_timestamp: u64,
    /// 访问次数
    pub access_count: u64,
    /// 最后访问时间
    pub last_access: u64,
}

/// 分发统计信息
/// 
/// 记录分发的性能和使用统计。
#[derive(Debug, Default)]
#[derive(Clone)]
pub struct DispatchStats {
    /// 总分发次数
    pub total_dispatches: u64,
    /// 成功分发次数
    pub successful_dispatches: u64,
    /// 失败分发次数
    pub failed_dispatches: u64,
    /// 缓存命中次数
    pub cache_hits: u64,
    /// 缓存未命中次数
    pub cache_misses: u64,
    /// 平均分发时间（纳秒）
    pub avg_dispatch_time_ns: u64,
    /// 各系统调用的分发次数
    pub syscall_counts: BTreeMap<u32, u64>,
}

/// 分发器配置
/// 
/// 配置分发器的行为参数。
#[derive(Debug, Clone)]
pub struct DispatcherConfig {
    /// 是否启用缓存
    pub enable_cache: bool,
    /// 缓存大小限制
    pub cache_size_limit: usize,
    /// 缓存过期时间（秒）
    pub cache_ttl_seconds: u64,
    /// 是否启用性能统计
    pub enable_stats: bool,
    /// 是否启用详细日志
    pub enable_verbose_logging: bool,
    /// 最大重试次数
    pub max_retries: u32,
}

impl Default for DispatcherConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            cache_size_limit: 1000,
            cache_ttl_seconds: 300, // 5分钟
            enable_stats: true,
            enable_verbose_logging: false,
            max_retries: 3,
        }
    }
}

/// 分发结果
/// 
/// 包含系统调用分发的结果信息。
#[derive(Debug)]
pub struct DispatchResult {
    /// 是否成功
    pub success: bool,
    /// 返回值
    pub return_value: u64,
    /// 错误信息（如果有）
    pub error: Option<KernelError>,
    /// 分发时间（纳秒）
    pub dispatch_time_ns: u64,
    /// 处理的服务名称
    pub service_name: String,
}

impl SyscallDispatcher {
    /// 创建新的系统调用分发器
    /// 
    /// # 参数
    /// 
    /// * `registry` - 服务注册表引用
    /// * `config` - 分发器配置
    /// 
    /// # 返回值
    /// 
    /// * `Self` - 新的分发器实例
    pub fn new(registry: Arc<ServiceRegistry>, config: DispatcherConfig) -> Self {
        Self {
            registry,
            syscall_cache: Arc::new(Mutex::new(BTreeMap::new())),
            stats: Arc::new(Mutex::new(DispatchStats::default())),
            config,
        }
    }
    
    /// 使用默认配置创建分发器
    /// 
    /// # 参数
    /// 
    /// * `registry` - 服务注册表引用
    /// 
    /// # 返回值
    /// 
    /// * `Self` - 新的分发器实例
    pub fn with_default_config(registry: Arc<ServiceRegistry>) -> Self {
        Self::new(registry, DispatcherConfig::default())
    }
    
    /// 分发系统调用
    /// 
    /// 将系统调用请求分发到相应的服务处理器。
    /// 
    /// # 参数
    /// 
    /// * `syscall_number` - 系统调用号
    /// * `args` - 系统调用参数
    /// 
    /// # 返回值
    /// 
    /// * `Ok(DispatchResult)` - 分发结果
    /// * `Err(KernelError)` - 分发失败
    pub fn dispatch(&self, syscall_number: u32, args: &[u64]) -> Result<DispatchResult, KernelError> {
        let start_time = self.get_current_time_ns();
        
        // 更新统计信息
        if self.config.enable_stats {
            self.update_dispatch_stats(syscall_number);
        }
        
        // 查找处理服务
        let service_name = self.find_service_for_syscall(syscall_number)?;
        
        // 执行系统调用
        let result = self.execute_syscall(&service_name, syscall_number, args, start_time);
        
        // 记录详细日志
        if self.config.enable_verbose_logging {
            if let Ok(ref dispatch_result) = result {
                self.log_dispatch_details(&service_name, syscall_number, args, dispatch_result);
            }
        }
        
        result
    }
    
    /// 批量分发系统调用
    /// 
    /// 批量处理多个系统调用，提高效率。
    /// 
    /// # 参数
    /// 
    /// * `requests` - 系统调用请求列表
    /// 
    /// # 返回值
    /// 
    /// * `Vec<Result<DispatchResult, KernelError>>` - 分发结果列表
    pub fn batch_dispatch(
        &self,
        requests: &[(u32, Vec<u64>)],
    ) -> Vec<Result<DispatchResult, KernelError>> {
        requests
            .iter()
            .map(|(syscall_num, args)| self.dispatch(*syscall_num, args))
            .collect()
    }
    
    /// 查找系统调用的处理服务
    /// 
    /// # 参数
    /// 
    /// * `syscall_number` - 系统调用号
    /// 
    /// # 返回值
    /// 
    /// * `Ok(String)` - 服务名称
    /// * `Err(KernelError)` - 查找失败
    fn find_service_for_syscall(&self, syscall_number: u32) -> Result<String, KernelError> {
        // 首先检查缓存
        if self.config.enable_cache {
            if let Some(cached_info) = self.check_cache(syscall_number) {
                return Ok(cached_info.service_name);
            }
        }
        
        // 从注册表查找
        let service_name = self.registry.get_syscall_service(syscall_number)?
            .ok_or_else(|| KernelError::SyscallNotSupported)?;
        
        // 更新缓存
        if self.config.enable_cache {
            self.update_cache(syscall_number, &service_name);
        }
        
        Ok(service_name)
    }
    
    /// 执行系统调用
    /// 
    /// # 参数
    /// 
    /// * `service_name` - 服务名称
    /// * `syscall_number` - 系统调用号
    /// * `args` - 系统调用参数
    /// * `start_time` - 开始时间
    /// 
    /// # 返回值
    /// 
    /// * `Ok(DispatchResult)` - 执行结果
    /// * `Err(KernelError)` - 执行失败
    fn execute_syscall(
        &self,
        service_name: &str,
        syscall_number: u32,
        args: &[u64],
        start_time: u64,
    ) -> Result<DispatchResult, KernelError> {
        let mut retries = 0;
        
        while retries <= self.config.max_retries {
            match self.try_execute_syscall(service_name, syscall_number, args, start_time) {
                Ok(result) => return Ok(result),
                Err(KernelError::ServiceUnavailable(_)) if retries < self.config.max_retries => {
                    retries += 1;
                    // 简单的退避策略
                    self.sleep_ns(1000 * retries as u64);
                }
                Err(e) => return Ok(DispatchResult {
                    success: false,
                    return_value: 0,
                    error: Some(e),
                    dispatch_time_ns: self.get_current_time_ns() - start_time,
                    service_name: service_name.to_string(),
                }),
            }
        }
        
        Err(KernelError::MaxRetriesExceeded(syscall_number))
    }
    
    /// 尝试执行系统调用
    ///
    /// 单次执行尝试，不包含重试逻辑。
    fn try_execute_syscall(
        &self,
        service_name: &str,
        syscall_number: u32,
        args: &[u64],
        start_time: u64,
    ) -> Result<DispatchResult, KernelError> {
        // 从注册表获取服务实例
        let mut service_ref = self.registry.get_service_mut_ref(service_name)?
            .ok_or_else(|| KernelError::ServiceNotFound)?;

        // 尝试将服务转换为 SyscallService
        let syscall_service = service_ref.as_any_mut()
            .downcast_mut::<dyn SyscallService>();

        if let Some(syscall_service) = syscall_service {
            // 执行系统调用
            let result = syscall_service.handle_syscall(syscall_number, args);
            
            let end_time = self.get_current_time_ns();
            let dispatch_time = end_time - start_time;

            match result {
                Ok(return_value) => {
                    // 更新成功统计
                    let mut stats = self.stats.lock();
                    stats.successful_dispatches += 1;
                    
                    Ok(DispatchResult {
                        success: true,
                        return_value,
                        error: None,
                        dispatch_time_ns: dispatch_time,
                        service_name: service_name.to_string(),
                    })
                },
                Err(error) => {
                    // 更新失败统计
                    let mut stats = self.stats.lock();
                    stats.failed_dispatches += 1;
                    
                    Ok(DispatchResult {
                        success: false,
                        return_value: 0,
                        error: Some(error),
                        dispatch_time_ns: dispatch_time,
                        service_name: service_name.to_string(),
                    })
                }
            }
        } else {
            // 服务不是系统调用服务
            Err(KernelError::ServiceNotSyscallService)
        }
    }
    
    /// 检查缓存
    /// 
    /// # 参数
    /// 
    /// * `syscall_number` - 系统调用号
    /// 
    /// # 返回值
    /// 
    /// * `Option<CachedServiceInfo>` - 缓存信息（如果存在且有效）
    fn check_cache(&self, syscall_number: u32) -> Option<CachedServiceInfo> {
        let mut cache = self.syscall_cache.lock();
        
        if let Some(cached_info) = cache.get(&syscall_number).cloned() {
            let current_time = self.get_current_time_ns() / 1_000_000_000; // 转换为秒

            // 检查缓存是否过期
            if current_time - cached_info.cache_timestamp <= self.config.cache_ttl_seconds {
                // 释放不可变借用，然后进行更新
                drop(cache);

                // 更新访问统计
                {
                    let mut cache = self.syscall_cache.lock();
                    let mut updated_info = cached_info.clone();
                    updated_info.access_count += 1;
                    updated_info.last_access = current_time;
                    cache.insert(syscall_number, updated_info);
                }

                // 更新缓存命中统计
                if self.config.enable_stats {
                    let mut stats = self.stats.lock();
                    stats.cache_hits += 1;
                }

                return Some(cached_info);
            } else {
                // 缓存过期，移除
                cache.remove(&syscall_number);
            }
        }
        
        // 更新缓存未命中统计
        if self.config.enable_stats {
            let mut stats = self.stats.lock();
            stats.cache_misses += 1;
        }
        
        None
    }
    
    /// 更新缓存
    /// 
    /// # 参数
    /// 
    /// * `syscall_number` - 系统调用号
    /// * `service_name` - 服务名称
    fn update_cache(&self, syscall_number: u32, service_name: &str) {
        let mut cache = self.syscall_cache.lock();
        
        // 检查缓存大小限制
        if cache.len() >= self.config.cache_size_limit {
            self.evict_cache_entries(&mut cache);
        }
        
        let current_time = self.get_current_time_ns() / 1_000_000_000;
        
        let cached_info = CachedServiceInfo {
            service_name: service_name.to_string(),
            cache_timestamp: current_time,
            access_count: 1,
            last_access: current_time,
        };
        
        cache.insert(syscall_number, cached_info);
    }
    
    /// 缓存淘汰
    /// 
    /// 当缓存达到大小限制时，淘汰最旧的条目。
    /// 
    /// # 参数
    /// 
    /// * `cache` - 缓存映射的可变引用
    fn evict_cache_entries(&self, cache: &mut BTreeMap<u32, CachedServiceInfo>) {
        if cache.is_empty() {
            return;
        }
        
        // 找到最旧的条目
        let mut oldest_key = *cache.keys().next().unwrap();
        let mut oldest_time = u64::MAX;
        
        for (key, info) in cache.iter() {
            if info.cache_timestamp < oldest_time {
                oldest_time = info.cache_timestamp;
                oldest_key = *key;
            }
        }
        
        cache.remove(&oldest_key);
    }
    
    /// 更新分发统计
    /// 
    /// # 参数
    /// 
    /// * `syscall_number` - 系统调用号
    fn update_dispatch_stats(&self, syscall_number: u32) {
        let mut stats = self.stats.lock();
        stats.total_dispatches += 1;
        
        // 更新特定系统调用的计数
        *stats.syscall_counts.entry(syscall_number).or_insert(0) += 1;
    }
    
    /// 记录分发详情
    /// 
    /// # 参数
    /// 
    /// * `service_name` - 服务名称
    /// * `syscall_number` - 系统调用号
    /// * `args` - 系统调用参数
    /// * `result` - 分发结果
    fn log_dispatch_details(
        &self,
        service_name: &str,
        syscall_number: u32,
        args: &[u64],
        result: &DispatchResult,
    ) {
        // 这里应该实现实际的日志记录
        // 暂时使用简单的格式化输出
        let log_message = format!(
            "Syscall {} dispatched to service {} with args {:?}: success={}, value={}, time={}ns",
            syscall_number,
            service_name,
            args,
            result.success,
            result.return_value,
            result.dispatch_time_ns
        );
        
        // 在实际实现中，这里应该调用日志系统
        // println!("{}", log_message);
    }
    
    /// 获取当前时间（纳秒）
    fn get_current_time_ns(&self) -> u64 {
        // 这里应该实现真实的时间获取
        // 暂时返回固定值
        0
    }
    
    /// 睡眠指定纳秒数
    /// 
    /// # 参数
    /// 
    /// * `duration_ns` - 睡眠时间（纳秒）
    fn sleep_ns(&self, duration_ns: u64) {
        // 这里应该实现真实的睡眠功能
        // 暂时为空实现
    }
    
    /// 获取分发统计信息
    /// 
    /// # 返回值
    /// 
    /// * `DispatchStats` - 当前统计信息
    pub fn get_stats(&self) -> DispatchStats {
        self.stats.lock().clone()
    }
    
    /// 重置统计信息
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = DispatchStats::default();
    }
    
    /// 清空缓存
    pub fn clear_cache(&self) {
        let mut cache = self.syscall_cache.lock();
        cache.clear();
    }
    
    /// 获取缓存大小
    /// 
    /// # 返回值
    /// 
    /// * `usize` - 当前缓存条目数量
    pub fn cache_size(&self) -> usize {
        self.syscall_cache.lock().len()
    }
    
    /// 预热缓存
    /// 
    /// 为常用的系统调用预热缓存。
    /// 
    /// # 参数
    /// 
    /// * `syscall_numbers` - 要预热的系统调用号列表
    pub fn warmup_cache(&self, syscall_numbers: &[u32]) {
        for &syscall_num in syscall_numbers {
            let _ = self.find_service_for_syscall(syscall_num);
        }
    }
}

/// 系统调用分发错误类型
/// 
/// 定义分发过程中可能出现的错误。
#[derive(Debug, Clone)]
pub enum DispatcherError {
    /// 系统调用不支持
    SyscallNotSupported(u32),
    /// 服务不可用
    ServiceUnavailable(String),
    /// 超过最大重试次数
    MaxRetriesExceeded(u32),
    /// 缓存错误
    CacheError(String),
    /// 参数无效
    InvalidParameters(String),
}

impl core::fmt::Display for DispatcherError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DispatcherError::SyscallNotSupported(num) => {
                write!(f, "Syscall {} is not supported", num)
            }
            DispatcherError::ServiceUnavailable(name) => {
                write!(f, "Service '{}' is unavailable", name)
            }
            DispatcherError::MaxRetriesExceeded(num) => {
                write!(f, "Max retries exceeded for syscall {}", num)
            }
            DispatcherError::CacheError(msg) => {
                write!(f, "Cache error: {}", msg)
            }
            DispatcherError::InvalidParameters(msg) => {
                write!(f, "Invalid parameters: {}", msg)
            }
        }
    }
}