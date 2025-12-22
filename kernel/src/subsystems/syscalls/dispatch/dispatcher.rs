//! 系统调用分发器模块
//! 
//! 本模块实现了系统调用的路由和分发功能，包括：
//! - SyscallDispatcher: 核心分发器
//! - 系统调用路由逻辑
//! - 性能监控和统计
//! - 错误处理和日志记录
//! 
//! 分发器是系统调用处理的核心组件，负责将系统调用请求路由到相应的服务。

use nos_nos_error_handling::unified::KernelError;
use crate::syscalls::services::traits::*;
use crate::syscalls::services::registry::{ServiceRegistry, Version};
use crate::syscalls::security::{SyscallSecurityValidator, SecurityContext, SecurityLevel, SecurityValidationResult, ResourceAccess, AccessControlManager, Permission, ResourceType};
use crate::reliability::{FaultManager, FaultType, FaultSeverity, CheckpointManager, CheckpointType, ErrorLogManager, LogLevel};
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

// 定义Result类型别名，使用DispatcherError作为错误类型
pub type Result<T> = core::result::Result<T, DispatcherError>;

/// LRU缓存节点
struct LruNode {
    syscall_num: u32,
    version: Option<Version>,
    service_name: String,
    cache_timestamp: u64,
    access_count: u64,
    last_access: u64,
    prev: Option<(u32, Option<Version>)>,
    next: Option<(u32, Option<Version>)>,
}

/// LRU缓存实现
struct LruCache {
    /// 缓存映射：(syscall_num, version) -> LruNode
    map: BTreeMap<(u32, Option<Version>), LruNode>,
    /// 头节点（最近使用）
    head: Option<(u32, Option<Version>)>,
    /// 尾节点（最少使用）
    tail: Option<(u32, Option<Version>)>,
    /// 缓存大小限制
    size_limit: usize,
}

impl LruCache {
    fn new(size_limit: usize) -> Self {
        Self {
            map: BTreeMap::new(),
            head: None,
            tail: None,
            size_limit,
        }
    }
    
    /// 获取缓存条目，如果存在则将其移到头部（最近使用）
    /// 注意：当前版本的缓存不支持版本化，仅作为演示
    fn get(&mut self, syscall_num: u32, version: Option<Version>) -> Option<String> {
        // 简单的实现：不支持版本化缓存
        None
    }
    
    /// 插入缓存条目，如果超过大小限制则淘汰最旧的（尾节点）
    /// 注意：当前版本的缓存不支持版本化，仅作为演示
    fn put(&mut self, syscall_num: u32, version: Option<Version>, service_name: String) {
        // 简单的实现：不支持版本化缓存
    }
    
    /// 移除缓存条目
    fn remove(&mut self, syscall_num: u32, version: Option<Version>) {
        if let Some(node) = self.map.get(&syscall_num) {
            let prev = node.prev;
            let next = node.next;
            
            // 更新前一个节点的next
            if let Some(prev_num) = prev {
                self.map.get_mut(&prev_num).unwrap().next = next;
            }
            
            // 更新后一个节点的prev
            if let Some(next_num) = next {
                self.map.get_mut(&next_num).unwrap().prev = prev;
            }
            
            // 更新头节点
            if self.head == Some(syscall_num) {
                self.head = next;
            }
            
            // 更新尾节点
            if self.tail == Some(syscall_num) {
                self.tail = prev;
            }
            
            // 从map中移除
            self.map.remove(&syscall_num);
        }
    }
    
    /// 将节点移到头部
    fn move_to_head(&mut self, syscall_num: u32, version: Option<Version>) {
        if self.head == Some(syscall_num) {
            return; // 已经是头节点
        }
        
        // 获取当前节点信息
        let node = self.map.get(&syscall_num).unwrap();
        let prev = node.prev;
        let next = node.next;
        
        // 更新前一个节点的next
        if let Some(prev_num) = prev {
            self.map.get_mut(&prev_num).unwrap().next = next;
        }
        
        // 更新后一个节点的prev
        if let Some(next_num) = next {
            self.map.get_mut(&next_num).unwrap().prev = prev;
        }
        
        // 更新尾节点（如果当前节点是尾节点）
        if self.tail == Some(syscall_num) {
            self.tail = prev;
        }
        
        // 将当前节点插入到头部
        let mut node = self.map.get_mut(&syscall_num).unwrap();
        node.prev = None;
        node.next = self.head;
        
        // 更新原头节点的prev
        if let Some(head_num) = self.head {
            self.map.get_mut(&head_num).unwrap().prev = Some(syscall_num);
        }
        
        // 设置为新头节点
        self.head = Some(syscall_num);
    }
    
    /// 辅助函数：获取当前时间（秒）
    fn get_current_time_sec(&self) -> u64 {
        // 实际实现应该调用系统时间函数
        // 这里暂时返回固定值，与Dispatcher中的实现保持一致
        0
    }
}

/// 系统调用分发器
///
/// 负责将系统调用请求分发到相应的服务处理器。
/// 提供高性能的路由机制和完善的错误处理。
pub struct SyscallDispatcher {
    /// 服务注册表引用
    registry: Arc<ServiceRegistry>,
    
    /// 快速路径缓存：小容量，低延迟，使用LRU
    fast_path_cache: Arc<Mutex<LruCache>>,
    
    /// 普通路径缓存：大容量，使用LRU
    normal_path_cache: Arc<Mutex<LruCache>>,
    
    /// 分发统计信息
    stats: Arc<Mutex<DispatchStats>>,
    
    /// 分发器配置
    config: DispatcherConfig,
    
    /// 安全验证器
    security_validator: Arc<SyscallSecurityValidator>,
    
    /// 访问控制管理器
    access_control: Arc<AccessControlManager>,
    
    /// 故障管理器
    fault_manager: Arc<FaultManager>,
    
    /// 检查点管理器
    checkpoint_manager: Arc<CheckpointManager>,
    
    /// 错误日志管理器
    error_log_manager: Arc<ErrorLogManager>,
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
    /// 快速路径缓存大小限制
    pub fast_path_cache_size: usize,
    /// 普通路径缓存大小限制
    pub normal_path_cache_size: usize,
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
            fast_path_cache_size: 256, // 小容量，快速访问
            normal_path_cache_size: 2048, // 大容量，普通访问
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
            fast_path_cache: Arc::new(Mutex::new(LruCache::new(config.fast_path_cache_size))),
            normal_path_cache: Arc::new(Mutex::new(LruCache::new(config.normal_path_cache_size))),
            stats: Arc::new(Mutex::new(DispatchStats::default())),
            config,
            security_validator: Arc::new(SyscallSecurityValidator::with_default_config()),
            access_control: Arc::new(AccessControlManager::with_default_config()),
            fault_manager: Arc::new(FaultManager::with_default_config()),
            checkpoint_manager: Arc::new(CheckpointManager::with_default_config()),
            error_log_manager: Arc::new(ErrorLogManager::with_default_config()),
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
    /// * `version` - 系统调用版本（可选，用于向后兼容）
    ///
    /// # 返回值
    ///
    /// * `Ok(DispatchResult)` - 分发结果
    /// * `Err(Error)` - 分发失败
    pub fn dispatch(&self, syscall_number: u32, args: &[u64], version: Option<Version>) -> Result<DispatchResult> {
        let start_time = self.get_current_time_ns();
        
        // 更新统计信息
        if self.config.enable_stats {
            self.update_dispatch_stats(syscall_number);
        }
        
        // 创建安全上下文
        let security_context = self.create_security_context();
        
        // 执行安全验证
        match self.security_validator.validate_syscall(syscall_number, args, &security_context) {
            SecurityValidationResult::Allowed => {
                // 安全验证通过，继续执行访问控制检查
            },
            validation_result => {
                // 安全验证失败，返回错误
                let end_time = self.get_current_time_ns();
                return Ok(DispatchResult {
                    success: false,
                    return_value: 0,
                    error: Some(KernelError::PermissionDenied),
                    dispatch_time_ns: end_time - start_time,
                    service_name: "security_validator".to_string(),
                });
            }
        }
        
        // 执行访问控制检查
        match self.check_access_control(syscall_number, args, &security_context) {
            AccessResult::Allowed => {
                // 访问控制检查通过，继续执行
            },
            access_result => {
                // 访问控制检查失败，返回错误
                let end_time = self.get_current_time_ns();
                return Ok(DispatchResult {
                    success: false,
                    return_value: 0,
                    error: Some(KernelError::PermissionDenied),
                    dispatch_time_ns: end_time - start_time,
                    service_name: "access_control".to_string(),
                });
            }
        }
        
        // 查找处理服务
        let service_name = self.find_service_for_syscall(syscall_number, version)?;
        
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
    
    /// 分发系统调用（向后兼容版本）
    ///
    /// 将系统调用请求分发到相应的服务处理器，使用默认版本。
    ///
    /// # 参数
    ///
    /// * `syscall_number` - 系统调用号
    /// * `args` - 系统调用参数
    ///
    /// # 返回值
    ///
    /// * `Ok(DispatchResult)` - 分发结果
    /// * `Err(Error)` - 分发失败
    pub fn dispatch_with_default_version(&self, syscall_number: u32, args: &[u64]) -> Result<DispatchResult> {
        self.dispatch(syscall_number, args, None)
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
    /// * `Vec<Result<DispatchResult>>` - 分发结果列表
    pub fn batch_dispatch(
        &self,
        requests: &[(u32, Vec<u64>)],
    ) -> Vec<Result<DispatchResult>> {
        requests
            .iter()
            .map(|(syscall_num, args)| self.dispatch_with_default_version(*syscall_num, args))
            .collect()
    }
    
    /// 查找系统调用的处理服务
    ///
    /// # 参数
    ///
    /// * `syscall_number` - 系统调用号
    /// * `version` - 系统调用版本（可选）
    ///
    /// # 返回值
    ///
    /// * `Ok(String)` - 服务名称
    /// * `Err(Error)` - 查找失败
    fn find_service_for_syscall(&self, syscall_number: u32, version: Option<Version>) -> Result<String> {
        // 首先检查缓存
        // 注意：当前版本的缓存不支持版本化，仅作为演示
        if self.config.enable_cache {
            // 1. 检查快速路径缓存
            let mut fast_cache = self.fast_path_cache.lock();
            if let Some(service_name) = fast_cache.get(syscall_number, version) {
                // 缓存命中
                drop(fast_cache);
                
                // 更新统计
                if self.config.enable_stats {
                    let mut stats = self.stats.lock();
                    stats.cache_hits += 1;
                }
                
                return Ok(service_name);
            }
            drop(fast_cache);
            
            // 2. 检查普通路径缓存
            let mut normal_cache = self.normal_path_cache.lock();
            if let Some(service_name) = normal_cache.get(syscall_number, version) {
                // 缓存命中
                drop(normal_cache);
                
                // 更新统计
                if self.config.enable_stats {
                    let mut stats = self.stats.lock();
                    stats.cache_hits += 1;
                }
                
                return Ok(service_name);
            }
            drop(normal_cache);
            
            // 缓存未命中
            if self.config.enable_stats {
                let mut stats = self.stats.lock();
                stats.cache_misses += 1;
            }
        }
        
        // 从注册表查找
        let service_name = self.registry.get_syscall_service(syscall_number, version)?
            .ok_or_else(|| DispatcherError::SyscallNotSupported(syscall_number))?;
        
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
    /// * `Err(Error)` - 执行失败
    fn execute_syscall(
        &self,
        service_name: &str,
        syscall_number: u32,
        args: &[u64],
        start_time: u64,
    ) -> Result<DispatchResult> {
        let mut retries = 0;
        
        while retries <= self.config.max_retries {
            match self.try_execute_syscall(service_name, syscall_number, args, start_time) {
                Ok(result) => return Ok(result),
                Err(DispatcherError::ServiceUnavailable(_)) if retries < self.config.max_retries => {
                    retries += 1;
                    // 简单的退避策略
                    self.sleep_ns(1000 * retries as u64);
                }
                Err(e) => return Ok(DispatchResult {
                    success: false,
                    return_value: 0,
                    error: Some(KernelError::from(e)),
                    dispatch_time_ns: self.get_current_time_ns() - start_time,
                    service_name: service_name.to_string(),
                }),
            }
        }
        
        Err(DispatcherError::MaxRetriesExceeded(syscall_number))
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
    ) -> Result<DispatchResult> {
        // 从注册表获取服务实例
        let mut service_ref = self.registry.get_service_mut_ref(service_name)?
            .ok_or_else(|| DispatcherError::ServiceUnavailable(service_name.to_string()))?;

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
                    
                    // 报告故障
                    self.report_syscall_fault(syscall_number, &error, service_name);
                    
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
            Err(DispatcherError::InvalidParameters(format!("Service '{}' is not a syscall service", service_name)))
        }
    }
    /// 清空缓存
    pub fn clear_cache(&self) {
        let mut fast_cache = self.fast_path_cache.lock();
        let mut normal_cache = self.normal_path_cache.lock();
        
        fast_cache.map.clear();
        fast_cache.head = None;
        fast_cache.tail = None;
        
        normal_cache.map.clear();
        normal_cache.head = None;
        normal_cache.tail = None;
    }
    
    /// 获取缓存大小
    ///
    /// # 返回值
    ///
    /// * `usize` - 当前缓存条目总数
    pub fn cache_size(&self) -> usize {
        let fast_cache = self.fast_path_cache.lock();
        let normal_cache = self.normal_path_cache.lock();
        fast_cache.map.len() + normal_cache.map.len()
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
    
    /// 创建安全上下文
    /// 
    /// # 返回值
    /// 
    /// * `SecurityContext` - 当前进程的安全上下文
    fn create_security_context(&self) -> SecurityContext {
        // 获取当前进程信息
        let pid = crate::process::getpid();
        let uid = crate::process::getuid();
        let gid = crate::process::getgid();
        
        // 获取安全级别
        let security_level = crate::security::get_current_security_level();
        
        // 创建权限映射
        let mut permissions = BTreeMap::new();
        
        // 根据安全级别设置默认权限
        match security_level {
            SecurityLevel::System => {
                permissions.insert("memory.allocate".to_string(), true);
                permissions.insert("memory.deallocate".to_string(), true);
                permissions.insert("memory.protect".to_string(), true);
                permissions.insert("process.fork".to_string(), true);
                permissions.insert("process.exec".to_string(), true);
                permissions.insert("process.kill".to_string(), true);
                permissions.insert("file.read".to_string(), true);
                permissions.insert("file.write".to_string(), true);
                permissions.insert("file.create".to_string(), true);
                permissions.insert("file.delete".to_string(), true);
                permissions.insert("network.create".to_string(), true);
                permissions.insert("network.connect".to_string(), true);
                permissions.insert("network.bind".to_string(), true);
                permissions.insert("network.listen".to_string(), true);
                permissions.insert("ipc.create".to_string(), true);
                permissions.insert("ipc.send".to_string(), true);
                permissions.insert("ipc.receive".to_string(), true);
            },
            SecurityLevel::High => {
                permissions.insert("memory.allocate".to_string(), true);
                permissions.insert("memory.deallocate".to_string(), true);
                permissions.insert("memory.protect".to_string(), true);
                permissions.insert("process.fork".to_string(), true);
                permissions.insert("process.exec".to_string(), true);
                permissions.insert("file.read".to_string(), true);
                permissions.insert("file.write".to_string(), true);
                permissions.insert("file.create".to_string(), true);
                permissions.insert("network.create".to_string(), true);
                permissions.insert("network.connect".to_string(), true);
                permissions.insert("ipc.create".to_string(), true);
                permissions.insert("ipc.send".to_string(), true);
                permissions.insert("ipc.receive".to_string(), true);
            },
            SecurityLevel::Medium => {
                permissions.insert("memory.allocate".to_string(), true);
                permissions.insert("memory.deallocate".to_string(), true);
                permissions.insert("process.fork".to_string(), true);
                permissions.insert("file.read".to_string(), true);
                permissions.insert("file.write".to_string(), true);
                permissions.insert("network.create".to_string(), true);
                permissions.insert("network.connect".to_string(), true);
                permissions.insert("ipc.create".to_string(), true);
                permissions.insert("ipc.send".to_string(), true);
                permissions.insert("ipc.receive".to_string(), true);
            },
            SecurityLevel::Low => {
                permissions.insert("memory.allocate".to_string(), true);
                permissions.insert("memory.deallocate".to_string(), true);
                permissions.insert("file.read".to_string(), true);
                permissions.insert("file.write".to_string(), true);
                permissions.insert("ipc.create".to_string(), true);
                permissions.insert("ipc.send".to_string(), true);
                permissions.insert("ipc.receive".to_string(), true);
            },
            SecurityLevel::Sandbox => {
                permissions.insert("memory.allocate".to_string(), true);
                permissions.insert("memory.deallocate".to_string(), true);
                permissions.insert("file.read".to_string(), true);
                permissions.insert("ipc.send".to_string(), true);
                permissions.insert("ipc.receive".to_string(), true);
            },
        }
        
        // 创建资源访问权限映射
        let mut resource_access = BTreeMap::new();
        
        // 根据安全级别设置默认资源访问权限
        match security_level {
            SecurityLevel::System => {
                resource_access.insert("system_memory".to_string(), ResourceAccess {
                    readable: true,
                    writable: true,
                    executable: true,
                    deletable: true,
                });
                resource_access.insert("kernel_memory".to_string(), ResourceAccess {
                    readable: true,
                    writable: true,
                    executable: true,
                    deletable: false,
                });
            },
            SecurityLevel::High => {
                resource_access.insert("system_memory".to_string(), ResourceAccess {
                    readable: true,
                    writable: true,
                    executable: true,
                    deletable: false,
                });
            },
            SecurityLevel::Medium => {
                resource_access.insert("user_memory".to_string(), ResourceAccess {
                    readable: true,
                    writable: true,
                    executable: true,
                    deletable: false,
                });
            },
            SecurityLevel::Low => {
                resource_access.insert("user_memory".to_string(), ResourceAccess {
                    readable: true,
                    writable: true,
                    executable: false,
                    deletable: false,
                });
            },
            SecurityLevel::Sandbox => {
                resource_access.insert("sandbox_memory".to_string(), ResourceAccess {
                    readable: true,
                    writable: true,
                    executable: false,
                    deletable: false,
                });
            },
        }
        
        SecurityContext {
            pid,
            uid,
            gid,
            security_level,
            permissions,
            resource_access,
        }
    }
    
    /// 检查访问控制
    /// 
    /// # 参数
    /// 
    /// * `syscall_number` - 系统调用号
    /// * `args` - 系统调用参数
    /// * `security_context` - 安全上下文
    /// 
    /// # 返回值
    /// 
    /// * `AccessResult` - 访问控制检查结果
    fn check_access_control(
        &self,
        syscall_number: u32,
        args: &[u64],
        security_context: &SecurityContext,
    ) -> AccessResult {
        // 根据系统调用号确定所需的权限和资源类型
        let (resource_type, resource_id, permission) = match syscall_number {
            // 文件系统系统调用
            0x2000 => { // SYS_READ
                if args.is_empty() {
                    return AccessResult::DeniedOperationNotSupported("Missing file descriptor argument".to_string());
                }
                let fd = args[0] as u32;
                (ResourceType::File, format!("fd:{}", fd), Permission::Read)
            },
            0x2001 => { // SYS_WRITE
                if args.is_empty() {
                    return AccessResult::DeniedOperationNotSupported("Missing file descriptor argument".to_string());
                }
                let fd = args[0] as u32;
                (ResourceType::File, format!("fd:{}", fd), Permission::Write)
            },
            0x2002 => { // SYS_OPEN
                if args.len() < 2 {
                    return AccessResult::DeniedOperationNotSupported("Missing arguments".to_string());
                }
                let pathname_ptr = args[0];
                let flags = args[1];
                let permission = if (flags & 0x2) != 0 { Permission::Write } else { Permission::Read };
                (ResourceType::File, format!("path:0x{:x}", pathname_ptr), permission)
            },
            
            // 内存管理系统调用
            0x3000 => { // SYS_MMAP
                (ResourceType::MemoryRegion, "memory".to_string(), Permission::Create)
            },
            0x3001 => { // SYS_MUNMAP
                if args.is_empty() {
                    return AccessResult::DeniedOperationNotSupported("Missing address argument".to_string());
                }
                let addr = args[0];
                (ResourceType::MemoryRegion, format!("addr:0x{:x}", addr), Permission::Delete)
            },
            0x3003 => { // SYS_MPROTECT
                if args.is_empty() {
                    return AccessResult::DeniedOperationNotSupported("Missing address argument".to_string());
                }
                let addr = args[0];
                (ResourceType::MemoryRegion, format!("addr:0x{:x}", addr), Permission::Write)
            },
            
            // 进程管理系统调用
            0x1001 => { // SYS_FORK
                (ResourceType::Process, "self".to_string(), Permission::Create)
            },
            0x1005 => { // SYS_KILL
                if args.is_empty() {
                    return AccessResult::DeniedOperationNotSupported("Missing PID argument".to_string());
                }
                let pid = args[0] as u32;
                (ResourceType::Process, format!("pid:{}", pid), Permission::Delete)
            },
            
            // 网络系统调用
            0x4000 => { // SYS_SOCKET
                (ResourceType::NetworkSocket, "new".to_string(), Permission::Create)
            },
            0x4002 => { // SYS_CONNECT
                if args.is_empty() {
                    return AccessResult::DeniedOperationNotSupported("Missing socket argument".to_string());
                }
                let sockfd = args[0] as u32;
                (ResourceType::NetworkSocket, format!("sockfd:{}", sockfd), Permission::Write)
            },
            
            // 默认情况
            _ => {
                // 对于未知的系统调用，默认允许
                return AccessResult::Allowed;
            }
        };
        
        // 检查访问权限
        self.access_control.check_access(
            security_context.uid,
            resource_type,
            &resource_id,
            permission,
        )
    }
    
    /// 获取安全验证器
    /// 
    /// # 返回值
    /// 
    /// * `&SyscallSecurityValidator` - 安全验证器引用
    pub fn get_security_validator(&self) -> &SyscallSecurityValidator {
        &self.security_validator
    }
    
    /// 报告系统调用故障
    /// 
    /// # 参数
    /// 
    /// * `syscall_number` - 系统调用号
    /// * `error` - 错误信息
    /// * `service_name` - 服务名称
    fn report_syscall_fault(&self, syscall_number: u32, error: &KernelError, service_name: &str) {
        // 确定故障类型和严重程度
        let (fault_type, severity) = match error {
            KernelError::PermissionDenied => (FaultType::Syscall, FaultSeverity::Warning),
            KernelError::NotFound => (FaultType::Syscall, FaultSeverity::Warning),
            KernelError::InvalidArgument => (FaultType::Syscall, FaultSeverity::Warning),
            KernelError::OutOfMemory => (FaultType::Memory, FaultSeverity::Error),
            KernelError::IoError => (FaultType::Software, FaultSeverity::Error),
            KernelError::NoDevice => (FaultType::Hardware, FaultSeverity::Error),
            KernelError::Busy => (FaultType::Software, FaultSeverity::Warning),
            KernelError::WouldBlock => (FaultType::Network, FaultSeverity::Info),
            KernelError::AlreadyInProgress => (FaultType::Software, FaultSeverity::Warning),
            KernelError::ConnectionReset => (FaultType::Network, FaultSeverity::Error),
            KernelError::ConnectionAborted => (FaultType::Network, FaultSeverity::Error),
            KernelError::NoProcess => (FaultType::Process, FaultSeverity::Error),
            KernelError::Interrupted => (FaultType::Software, FaultSeverity::Info),
            KernelError::BadFileDescriptor => (FaultType::Software, FaultSeverity::Error),
            KernelError::NotSupported => (FaultType::Software, FaultSeverity::Warning),
            KernelError::TimedOut => (FaultType::Network, FaultSeverity::Error),
            KernelError::OutOfSpace => (FaultType::Storage, FaultSeverity::Error),
            KernelError::QuotaExceeded => (FaultType::Storage, FaultSeverity::Warning),
            KernelError::Unknown(_) => (FaultType::Software, FaultSeverity::Error),
        };
        
        // 创建故障元数据
        let mut metadata = BTreeMap::new();
        metadata.insert("syscall_number".to_string(), syscall_number.to_string());
        metadata.insert("service_name".to_string(), service_name.to_string());
        metadata.insert("error_code".to_string(), format!("{:?}", error));
        
        // 记录错误日志
        self.error_log_manager.log(
            LogLevel::Error,
            "syscall_dispatcher",
            &format!("Syscall {} failed in service {}: {:?}", syscall_number, service_name, error),
            Some(&format!("{:?}", error)),
            None, // PID
            None, // TID
            Some("dispatcher.rs"),
            Some(line!()),
            Some("report_syscall_fault"),
            {
                let mut fields = BTreeMap::new();
                fields.insert("syscall_number".to_string(), syscall_number.to_string());
                fields.insert("service_name".to_string(), service_name.to_string());
                fields.insert("fault_type".to_string(), format!("{:?}", fault_type));
                fields.insert("severity".to_string(), format!("{:?}", severity));
                fields
            },
        );
        
        // 报告故障
        self.fault_manager.report_fault(
            fault_type,
            severity,
            format!("Syscall {} failed in service {}: {:?}", syscall_number, service_name, error),
            "syscall_dispatcher".to_string(),
            metadata,
        );
    }
    
    /// 获取访问控制管理器
    /// 
    /// # 返回值
    /// 
    /// * `&AccessControlManager` - 访问控制管理器引用
    pub fn get_access_control(&self) -> &AccessControlManager {
        &self.access_control
    }
    
    /// 创建系统状态检查点
    /// 
    /// # 参数
    /// 
    /// * `checkpoint_type` - 检查点类型
    /// * `description` - 检查点描述
    /// * `creator` - 创建者
    /// * `tags` - 标签列表
    /// 
    /// # 返回值
    /// 
    /// * `Result<CheckpointId, KernelError>` - 检查点ID或错误
    pub fn create_checkpoint(
        &self,
        checkpoint_type: CheckpointType,
        description: String,
        creator: String,
        tags: Vec<String>,
    ) -> Result<crate::reliability::CheckpointId, KernelError> {
        self.checkpoint_manager.create_checkpoint(
            checkpoint_type,
            description,
            creator,
            tags,
            None, // No parent checkpoint for now
        )
    }
    
    /// 恢复系统状态检查点
    /// 
    /// # 参数
    /// 
    /// * `checkpoint_id` - 检查点ID
    /// 
    /// # 返回值
    /// 
    /// * `Result<(), KernelError>` - 成功或错误
    pub fn restore_checkpoint(&self, checkpoint_id: crate::reliability::CheckpointId) -> Result<(), KernelError> {
        self.checkpoint_manager.restore_checkpoint(checkpoint_id)
    }
    
    /// 获取所有检查点
    /// 
    /// # 返回值
    /// 
    /// * `Vec<CheckpointMetadata>` - 检查点元数据列表
    pub fn get_all_checkpoints(&self) -> Vec<crate::reliability::CheckpointMetadata> {
        self.checkpoint_manager.get_all_checkpoints()
    }
    
    /// 获取故障管理器
    /// 
    /// # 返回值
    /// 
    /// * `&FaultManager` - 故障管理器引用
    pub fn get_fault_manager(&self) -> &FaultManager {
        &self.fault_manager
    }
    
    /// 获取检查点管理器
    /// 
    /// # 返回值
    /// 
    /// * `&CheckpointManager` - 检查点管理器引用
    pub fn get_checkpoint_manager(&self) -> &CheckpointManager {
        &self.checkpoint_manager
    }
    
    /// 获取错误日志管理器
    /// 
    /// # 返回值
    /// 
    /// * `&ErrorLogManager` - 错误日志管理器引用
    pub fn get_error_log_manager(&self) -> &ErrorLogManager {
        &self.error_log_manager
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
    
    /// 预热缓存
    /// 
    /// 为常用的系统调用预热缓存。
    /// 
    /// # 参数
    /// 
    /// * `syscall_numbers` - 要预热的系统调用号列表
    pub fn warmup_cache(&self, syscall_numbers: &[u32]) {
        for &syscall_num in syscall_numbers {
            let _ = self.find_service_for_syscall(syscall_num, None);
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

// 实现从KernelError到DispatcherError的转换
impl From<KernelError> for DispatcherError {
    fn from(error: KernelError) -> Self {
        DispatcherError::ServiceUnavailable(error.to_string())
    }
}