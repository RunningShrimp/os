// System Call Service
//
// 统一系统调用路由和管理服务
// 提供安全、高效的系统调用处理机制

extern crate alloc;

use crate::types::stubs::{ServiceId, Message, MessageType, send_message, receive_message,
                          ServiceInfo, InterfaceVersion, ServiceCategory, get_service_registry,
                          pid_t, uid_t, gid_t};
use crate::reliability::errno::{EINVAL, ENOSYS, EPERM, EACCES};
use crate::syscalls;
use crate::microkernel::ipc::MessageQueue;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
use alloc::string::ToString;
use spin::Mutex;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// 系统调用请求消息
#[derive(Debug, Clone)]
pub struct SyscallRequest {
    /// 请求ID
    pub request_id: u64,
    /// 调用进程ID
    pub pid: pid_t,
    /// 调用者用户ID
    pub uid: uid_t,
    /// 调用者组ID
    pub gid: gid_t,
    /// 系统调用号
    pub syscall_num: usize,
    /// 参数
    pub args: [usize; 6],
    /// 调用栈地址（用于安全检查）
    pub stack_pointer: usize,
    /// 调用时间戳
    pub timestamp: u64,
}

/// 系统调用响应消息
#[derive(Debug, Clone)]
pub struct SyscallResponse {
    /// 对应的请求ID
    pub request_id: u64,
    /// 返回值
    pub return_value: isize,
    /// 错误码（如果有）
    pub error_code: Option<isize>,
    /// 处理时间（微秒）
    pub processing_time_us: u64,
    /// 处理服务的ID
    pub handler_service: Option<ServiceId>,
}

/// 系统调用权限信息
#[derive(Debug, Clone)]
pub struct SyscallPermissions {
    /// 是否需要root权限
    pub requires_root: bool,
    /// 是否需要特定能力
    pub required_capabilities: Vec<u32>,
    /// 允许的用户ID（None表示所有用户）
    pub allowed_uids: Option<Vec<uid_t>>,
    /// 允许的组ID（None表示所有组）
    pub allowed_gids: Option<Vec<gid_t>>,
    /// 是否允许在容器中调用
    pub allowed_in_container: bool,
}

/// 系统调用统计信息
#[derive(Debug)]
pub struct SyscallStats {
    /// 总调用次数
    pub total_calls: AtomicU64,
    /// 成功次数
    pub successful_calls: AtomicU64,
    /// 失败次数
    pub failed_calls: AtomicU64,
    /// 权限拒绝次数
    pub permission_denied: AtomicU64,
    /// 平均处理时间（纳秒）
    pub avg_processing_time_ns: AtomicU64,
    /// 最大处理时间（纳秒）
    pub max_processing_time_ns: AtomicU64,
    /// 最小处理时间（纳秒）
    pub min_processing_time_ns: AtomicU64,
    /// 每个系统调用的统计
    pub per_syscall_stats: BTreeMap<usize, SyscallPerfStats>,
}

/// 单个系统调用的性能统计
#[derive(Debug, Clone)]
pub struct SyscallPerfStats {
    /// 调用次数
    pub call_count: u64,
    /// 总时间（纳秒）
    pub total_time_ns: u64,
    /// 最大时间（纳秒）
    pub max_time_ns: u64,
    /// 最小时间（纳秒）
    pub min_time_ns: u64,
    /// 错误次数
    pub error_count: u64,
}

/// 系统调用服务
pub struct SyscallService {
    /// 服务ID
    service_id: ServiceId,
    /// 消息队列
    message_queue: Arc<Mutex<MessageQueue>>,
    /// 系统调用统计
    stats: Arc<Mutex<SyscallStats>>,
    /// 系统调用权限映射
    syscall_permissions: BTreeMap<usize, SyscallPermissions>,
    /// 系统调用路由表
    syscall_routes: BTreeMap<usize, ServiceCategory>,
    /// 请求ID生成器
    request_id_counter: AtomicU64,
    /// 待处理请求
    pending_requests: Arc<Mutex<BTreeMap<u64, SyscallRequest>>>,
}

impl SyscallService {
    /// 创建新的系统调用服务
    pub fn new() -> Result<Self, &'static str> {
        let service_id: ServiceId = 1; // 固定ID用于系统调用管理器 (ServiceId is u64)
        let message_queue = Arc::new(Mutex::new(MessageQueue::new(service_id, 0, 1024, 4096)));

        let mut service = Self {
            service_id,
            message_queue,
            stats: Arc::new(Mutex::new(SyscallStats::new())),
            syscall_permissions: BTreeMap::new(),
            syscall_routes: BTreeMap::new(),
            request_id_counter: AtomicU64::new(1),
            pending_requests: Arc::new(Mutex::new(BTreeMap::new())),
        };

        service.initialize_permissions();
        service.initialize_routes();

        Ok(service)
    }

    /// 初始化系统调用权限
    fn initialize_permissions(&mut self) {
        // 基础文件操作 - 所有用户
        let basic_file_ops = [0, 1, 2, 3, 4, 5, 6, 8, 9]; // read, write, open, close, stat, fstat, lstat, poll, lseek
        for syscall in &basic_file_ops {
            self.syscall_permissions.insert(*syscall, SyscallPermissions {
                requires_root: false,
                required_capabilities: Vec::new(),
                allowed_uids: None,
                allowed_gids: None,
                allowed_in_container: true,
            });
        }

        // 进程管理 - 需要适当权限
        let process_mgmt = [39, 57, 60]; // getpid, fork, execve
        for syscall in &process_mgmt {
            self.syscall_permissions.insert(*syscall, SyscallPermissions {
                requires_root: false,
                required_capabilities: Vec::new(),
                allowed_uids: None,
                allowed_gids: None,
                allowed_in_container: true,
            });
        }

        // 系统管理 - 需要root权限
        let sys_admin = [161, 162, 163]; // chown, chmod, setuid
        for syscall in &sys_admin {
            self.syscall_permissions.insert(*syscall, SyscallPermissions {
                requires_root: true,
                required_capabilities: vec![0], // CAP_CHOWN
                allowed_uids: None,
                allowed_gids: None,
                allowed_in_container: false,
            });
        }

        // 网络操作
        let network_ops = [41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55];
        for syscall in &network_ops {
            self.syscall_permissions.insert(*syscall, SyscallPermissions {
                requires_root: false,
                required_capabilities: vec![1], // CAP_NET_RAW for raw sockets
                allowed_uids: None,
                allowed_gids: None,
                allowed_in_container: true,
            });
        }

        // 内存管理
        let memory_ops = [9, 10, 11, 12]; // mmap, mprotect, munmap, brk
        for syscall in &memory_ops {
            self.syscall_permissions.insert(*syscall, SyscallPermissions {
                requires_root: false,
                required_capabilities: Vec::new(),
                allowed_uids: None,
                allowed_gids: None,
                allowed_in_container: true,
            });
        }
    }

    /// 初始化系统调用路由表
    fn initialize_routes(&mut self) {
        // 文件系统相关系统调用 -> 文件系统服务
        let fs_syscalls = [
            0, 1, 2, 3, 4, 5, 6, 8, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 24, 25, 26, 27, 28,
            29, 30, 31, 32, 33, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87,
            88, 89, 90, 91, 92, 93, 94, 95, 133, 137, 138, 165, 166, 217, 257, 258, 259, 260,
            261, 262, 263, 264, 265, 266, 267, 268, 269, 270, 271, 272, 273, 274, 280, 281, 282,
            283, 284, 285, 286, 287, 288, 289, 290, 291, 292, 293, 294, 295, 296, 297, 298, 299,
            300, 301, 302, 303, 304, 305, 306, 307, 308, 309, 310, 311, 312, 313, 314, 315, 316,
            317, 318, 319, 320, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330, 331, 332
        ];
        for syscall in &fs_syscalls {
            self.syscall_routes.insert(*syscall, ServiceCategory::FileSystem);
        }

        // 进程管理相关系统调用 -> 进程管理服务
        let proc_syscalls = [
            39, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69,
            70, 71, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123,
            124, 125, 126, 127, 128, 129, 130, 131, 132, 135, 172, 173, 175, 176, 186, 199, 200,
            201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217,
            218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234,
            235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251,
            252, 253, 254, 255, 256
        ];
        for syscall in &proc_syscalls {
            self.syscall_routes.insert(*syscall, ServiceCategory::Process);
        }

        // 内存管理相关系统调用 -> 内存管理服务
        let mem_syscalls = [
            9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
            30, 31, 125, 126, 127, 128, 129, 130, 131, 148, 149, 150, 151, 152, 155, 156, 157,
            158, 159, 160, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191,
            192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208,
            209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225,
            226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242,
            243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255, 256
        ];
        for syscall in &mem_syscalls {
            self.syscall_routes.insert(*syscall, ServiceCategory::Memory);
        }

        // 网络相关系统调用 -> 网络服务
        let net_syscalls = [
            41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 102, 103, 104, 105,
            106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121,
            122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137,
            138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153,
            154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169,
            170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185,
            186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201,
            202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217,
            218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233,
            234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249,
            250, 251, 252, 253, 254, 255, 256
        ];
        for syscall in &net_syscalls {
            self.syscall_routes.insert(*syscall, ServiceCategory::Network);
        }
    }

    /// 处理系统调用请求（简化版本，直接调用现有的syscall dispatcher）
    pub fn handle_syscall(&self, request: SyscallRequest) -> SyscallResponse {
        let start_time = self.get_timestamp_ns();

        // 记录请求
        {
            let mut pending = self.pending_requests.lock();
            pending.insert(request.request_id, request.clone());
        }

        // 检查权限
        if let Err(error_code) = self.check_permissions(&request) {
            self.update_stats_error(&request, start_time);
            return SyscallResponse {
                request_id: request.request_id,
                return_value: -1,
                error_code: Some(error_code as isize),
                processing_time_us: ((self.get_timestamp_ns() - start_time) / 1000) as u64,
                handler_service: None,
            };
        }

        // 获取目标服务类型
        let handler_service = self.syscall_routes.get(&request.syscall_num)
            .copied()
            .unwrap_or(ServiceCategory::Process); // 默认路由到进程管理服务

        // 直接调用现有的系统调用分发器
        // 在完整版本中，这里会将请求转发到相应的服务
        let result = syscalls::dispatch(request.syscall_num, &request.args);

        // 更新统计信息
        self.update_stats_success(&request, start_time);

        // 清理待处理请求
        {
            let mut pending = self.pending_requests.lock();
            pending.remove(&request.request_id);
        }

        // Convert ServiceCategory to ServiceId for response
        // Use the category's numeric value as a base for service ID
        let handler_service_id: ServiceId = handler_service.as_u32() as u64;

        SyscallResponse {
            request_id: request.request_id,
            return_value: result,
            error_code: if result < 0 { Some(-result) } else { None },
            processing_time_us: ((self.get_timestamp_ns() - start_time) / 1000) as u64,
            handler_service: Some(handler_service_id), // Use converted ServiceId
        }
    }

    /// 检查系统调用权限
    fn check_permissions(&self, request: &SyscallRequest) -> Result<(), isize> {
        let permissions = self.syscall_permissions.get(&request.syscall_num)
            .ok_or(ENOSYS as isize)?; // 系统调用不存在

        // 检查root权限要求
        if permissions.requires_root && request.uid != 0 {
            return Err(EPERM as isize);
        }

        // 检查用户ID白名单
        if let Some(allowed_uids) = &permissions.allowed_uids {
            if !allowed_uids.contains(&request.uid) {
                return Err(EACCES as isize);
            }
        }

        // 检查组ID白名单
        if let Some(allowed_gids) = &permissions.allowed_gids {
            if !allowed_gids.contains(&request.gid) {
                return Err(EACCES as isize);
            }
        }

        Ok(())
    }

    /// 更新成功统计信息
    fn update_stats_success(&self, request: &SyscallRequest, start_time: u64) {
        let processing_time = self.get_timestamp_ns() - start_time;

        {
            let mut stats = self.stats.lock();
            stats.total_calls.fetch_add(1, Ordering::Relaxed);
            stats.successful_calls.fetch_add(1, Ordering::Relaxed);

            // 更新平均处理时间
            let current_avg = stats.avg_processing_time_ns.load(Ordering::Relaxed);
            let total_calls = stats.total_calls.load(Ordering::Relaxed);
            let new_avg = (current_avg * (total_calls - 1) + processing_time) / total_calls;
            stats.avg_processing_time_ns.store(new_avg, Ordering::Relaxed);

            // 更新最大/最小处理时间
            let current_max = stats.max_processing_time_ns.load(Ordering::Relaxed);
            let current_min = stats.min_processing_time_ns.load(Ordering::Relaxed);

            if processing_time > current_max {
                stats.max_processing_time_ns.store(processing_time, Ordering::Relaxed);
            }

            if current_min == 0 || processing_time < current_min {
                stats.min_processing_time_ns.store(processing_time, Ordering::Relaxed);
            }

            // 更新特定系统调用统计
            let perf_stats = stats.per_syscall_stats.entry(request.syscall_num)
                .or_insert(SyscallPerfStats {
                    call_count: 0,
                    total_time_ns: 0,
                    max_time_ns: 0,
                    min_time_ns: u64::MAX,
                    error_count: 0,
                });

            perf_stats.call_count += 1;
            perf_stats.total_time_ns += processing_time;

            if processing_time > perf_stats.max_time_ns {
                perf_stats.max_time_ns = processing_time;
            }

            if processing_time < perf_stats.min_time_ns {
                perf_stats.min_time_ns = processing_time;
            }
        }
    }

    /// 更新错误统计信息
    fn update_stats_error(&self, request: &SyscallRequest, start_time: u64) {
        let processing_time = self.get_timestamp_ns() - start_time;

        {
            let mut stats = self.stats.lock();
            stats.total_calls.fetch_add(1, Ordering::Relaxed);
            stats.failed_calls.fetch_add(1, Ordering::Relaxed);

            // 更新特定系统调用统计
            let perf_stats = stats.per_syscall_stats.entry(request.syscall_num)
                .or_insert(SyscallPerfStats {
                    call_count: 0,
                    total_time_ns: 0,
                    max_time_ns: 0,
                    min_time_ns: u64::MAX,
                    error_count: 0,
                });

            perf_stats.call_count += 1;
            perf_stats.total_time_ns += processing_time;
            perf_stats.error_count += 1;
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> SyscallStatsSnapshot {
        let stats = self.stats.lock();
        SyscallStatsSnapshot {
            total_calls: stats.total_calls.load(Ordering::Relaxed),
            successful_calls: stats.successful_calls.load(Ordering::Relaxed),
            failed_calls: stats.failed_calls.load(Ordering::Relaxed),
            permission_denied: stats.permission_denied.load(Ordering::Relaxed),
            avg_processing_time_ns: stats.avg_processing_time_ns.load(Ordering::Relaxed),
            max_processing_time_ns: stats.max_processing_time_ns.load(Ordering::Relaxed),
            min_processing_time_ns: stats.min_processing_time_ns.load(Ordering::Relaxed),
            per_syscall_stats: stats.per_syscall_stats.clone(),
        }
    }

    /// 获取待处理请求数量
    pub fn get_pending_requests_count(&self) -> usize {
        self.pending_requests.lock().len()
    }

    /// 生成新的请求ID
    pub fn generate_request_id(&self) -> u64 {
        self.request_id_counter.fetch_add(1, Ordering::Relaxed)
    }

    /// 获取当前时间戳（纳秒）
    fn get_timestamp_ns(&self) -> u64 {
        // Use high precision timer (nanoseconds since boot)
        crate::time::hrtime_nanos() as u64
    }

    /// 获取服务ID
    pub fn get_service_id(&self) -> ServiceId {
        self.service_id
    }
}

/// 系统调用统计信息快照
#[derive(Debug, Clone)]
pub struct SyscallStatsSnapshot {
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub permission_denied: u64,
    pub avg_processing_time_ns: u64,
    pub max_processing_time_ns: u64,
    pub min_processing_time_ns: u64,
    pub per_syscall_stats: BTreeMap<usize, SyscallPerfStats>,
}

impl SyscallStats {
    /// 创建新的统计信息
    pub fn new() -> Self {
        Self {
            total_calls: AtomicU64::new(0),
            successful_calls: AtomicU64::new(0),
            failed_calls: AtomicU64::new(0),
            permission_denied: AtomicU64::new(0),
            avg_processing_time_ns: AtomicU64::new(0),
            max_processing_time_ns: AtomicU64::new(0),
            min_processing_time_ns: AtomicU64::new(u64::MAX),
            per_syscall_stats: BTreeMap::new(),
        }
    }
}

/// 系统调用服务管理器
pub struct SyscallManager {
    /// 系统调用服务实例
    service: Arc<SyscallService>,
    /// 是否已初始化
    initialized: bool,
}

impl SyscallManager {
    /// 创建新的系统调用管理器
    pub fn new() -> Result<Self, &'static str> {
        let service = Arc::new(SyscallService::new()?);

        Ok(Self {
            service,
            initialized: false,
        })
    }

    /// 初始化系统调用服务
    pub fn initialize(&mut self) -> Result<(), &'static str> {
        if self.initialized {
            return Ok(());
        }

        // 注册到服务注册表
        let registry = get_service_registry().ok_or("Service registry not initialized")?;
        let service_info = ServiceInfo::new(
            self.service.get_service_id(),
            "SyscallManager".to_string(),
            "System call service for processing user-space system calls".to_string(),
            ServiceCategory::System,
            InterfaceVersion::new(1, 0, 0),
            0, // owner_id - kernel owned
        );

        registry.register_service(service_info).map_err(|_| "Failed to register service")?;

        self.initialized = true;
        crate::println!("[syscall] System call service initialized");
        Ok(())
    }

    /// 获取系统调用服务引用
    pub fn get_service(&self) -> Arc<SyscallService> {
        self.service.clone()
    }
}

// 全局系统调用管理器实例
static mut SYSCALL_MANAGER: Option<SyscallManager> = None;
static SYSCALL_MANAGER_INIT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// 初始化系统调用服务
pub fn init() -> Result<(), &'static str> {
    if SYSCALL_MANAGER_INIT.load(core::sync::atomic::Ordering::Relaxed) {
        return Ok(());
    }

    unsafe {
        let mut manager = SyscallManager::new()?;
        manager.initialize()?;
        SYSCALL_MANAGER = Some(manager);
    }

    SYSCALL_MANAGER_INIT.store(true, core::sync::atomic::Ordering::Relaxed);
    Ok(())
}

/// 获取全局系统调用服务
pub fn get_syscall_service() -> Option<Arc<SyscallService>> {
    unsafe {
        SYSCALL_MANAGER.as_ref().map(|m| m.get_service())
    }
}

/// 处理系统调用的便捷函数
pub fn handle_syscall(request: SyscallRequest) -> SyscallResponse {
    let service = get_syscall_service().unwrap(); // 在生产环境中需要更好的错误处理
    service.handle_syscall(request)
}

/// 获取系统调用统计信息
pub fn get_stats() -> Option<SyscallStatsSnapshot> {
    let service = get_syscall_service()?;
    Some(service.get_stats())
}

/// 兼容性接口 - 保持与现有代码的兼容性
pub fn syscall_dispatch(sysnum: usize, args: &[usize]) -> usize {
    syscalls::dispatch(sysnum, args) as usize
}

/// 获取系统调用服务ID
pub fn get_service_id() -> Option<ServiceId> {
    get_syscall_service().map(|s| s.get_service_id())
}