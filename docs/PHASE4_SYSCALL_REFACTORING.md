# NOS内核系统调用模块重构设计文档

## 概述

本文档描述了NOS内核系统调用模块的重构设计，作为第4阶段架构重构的试点项目。重构目标是降低模块间耦合度，实现清晰的接口抽象，支持插件化扩展，并提高可测试性和可维护性。

## 当前问题分析

### 1. 高耦合度问题
- 系统调用模块包含287个`use crate::`依赖，是所有模块中最高的
- 直接依赖process、fs、mm、ipc、posix等多个核心模块
- 模块间通过硬编码的import语句耦合，难以独立测试和替换

### 2. 架构问题
- 单体设计：所有系统调用都在一个大模块中
- 职责不清：系统调用处理、参数验证、错误处理混合在一起
- 缺乏抽象：没有统一的系统调用接口定义

### 3. 可扩展性问题
- 添加新系统调用需要修改多个文件
- 难以支持动态系统调用注册
- 无法实现系统调用的热替换

## 重构设计

### 1. 分层架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                 应用接口层 (Application Interface)            │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐      │
│  │ Syscall API │ │ POSIX API  │ │ Compatibility│      │
│  └─────────────┘ └─────────────┘ └─────────────┘      │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                  服务层 (Service Layer)                   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐      │
│  │ Process SVC │ │ File SVC    │ │ Memory SVC  │      │
│  └─────────────┘ └─────────────┘ └─────────────┘      │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐      │
│  │ Signal SVC  │ │ Network SVC │ │ Security SVC│      │
│  └─────────────┘ └─────────────┘ └─────────────┘      │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                  抽象层 (Abstraction Layer)              │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐      │
│  │ Syscall Mgr│ │ Service Reg │ │ DI Container│      │
│  └─────────────┘ └─────────────┘ └─────────────┘      │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                   HAL层 (HAL Layer)                     │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐      │
│  │ Process HAL │ │ Memory HAL  │ │ I/O HAL     │      │
│  └─────────────┘ └─────────────┘ └─────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

### 2. 核心接口定义

#### 2.1 系统调用接口

```rust
// 系统调用服务接口
pub trait SyscallService: Send + Sync {
    fn service_name(&self) -> &str;
    fn service_version(&self) -> ServiceVersion;
    fn handle_syscall(&self, context: &SyscallContext) -> SyscallResult;
    fn get_supported_syscalls(&self) -> Vec<u32>;
    fn validate_args(&self, syscall_id: u32, args: &[u64]) -> Result<(), ValidationError>;
}

// 系统调用上下文
#[derive(Debug, Clone)]
pub struct SyscallContext {
    pub syscall_id: u32,
    pub args: Vec<u64>,
    pub caller_pid: Pid,
    pub caller_tid: Tid,
    pub caller_credentials: ProcessCredentials,
    pub timestamp: u64,
    pub flags: SyscallFlags,
}

// 系统调用结果
pub type SyscallResult = Result<SyscallResponse, SyscallError>;

#[derive(Debug, Clone)]
pub struct SyscallResponse {
    pub return_value: u64,
    pub metadata: ResponseMetadata,
    pub performance_metrics: Option<PerformanceMetrics>,
}

// 系统调用标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyscallFlags {
    pub fast_path: bool,
    pub blocking: bool,
    pub privileged: bool,
    pub async_capable: bool,
}
```

#### 2.2 服务注册接口

```rust
// 服务注册表接口
pub trait ServiceRegistry: Send + Sync {
    fn register_service(&mut self, service: Box<dyn SyscallService>) -> Result<ServiceId, RegistryError>;
    fn unregister_service(&mut self, service_id: ServiceId) -> Result<(), RegistryError>;
    fn find_service(&self, syscall_id: u32) -> Option<&dyn SyscallService>;
    fn list_services(&self) -> Vec<ServiceInfo>;
    fn get_service_by_id(&self, service_id: ServiceId) -> Option<&dyn SyscallService>;
}

// 服务信息
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub id: ServiceId,
    pub name: String,
    pub version: ServiceVersion,
    pub supported_syscalls: Vec<u32>,
    pub dependencies: Vec<ServiceId>,
    pub capabilities: ServiceCapabilities,
    pub resource_requirements: ResourceRequirements,
}
```

#### 2.3 依赖注入接口

```rust
// 依赖注入容器接口
pub trait DIContainer: Send + Sync {
    fn register<T: Service + 'static>(&mut self, service: T) -> Result<(), DIError>;
    fn resolve<T: Service + 'static>(&self) -> Result<T, DIError>;
    fn resolve_ref<T: Service + 'static>(&self) -> Result<&T, DIError>;
    fn has_service<T: Service + 'static>(&self) -> bool;
    fn list_services(&self) -> Vec<ServiceDescriptor>;
}

// 服务特征
pub trait Service: Send + Sync {
    fn service_id(&self) -> ServiceId;
    fn service_name(&self) -> &str;
    fn initialize(&mut self) -> Result<(), ServiceError>;
    fn shutdown(&mut self) -> Result<(), ServiceError>;
    fn health_check(&self) -> ServiceHealth;
}
```

### 3. 模块重构设计

#### 3.1 系统调用管理器

```rust
// 系统调用管理器
pub struct SyscallManager {
    registry: Box<dyn ServiceRegistry>,
    di_container: Box<dyn DIContainer>,
    performance_monitor: Box<dyn PerformanceMonitor>,
    security_manager: Box<dyn SecurityManager>,
    cache: Box<dyn SyscallCache>,
    config: SyscallManagerConfig,
}

impl SyscallManager {
    pub fn new(
        registry: Box<dyn ServiceRegistry>,
        di_container: Box<dyn DIContainer>,
    ) -> Self {
        Self {
            registry,
            di_container,
            performance_monitor: Box::new(DefaultPerformanceMonitor::new()),
            security_manager: Box::new(DefaultSecurityManager::new()),
            cache: Box::new(DefaultSyscallCache::new()),
            config: SyscallManagerConfig::default(),
        }
    }

    pub fn dispatch_syscall(&mut self, context: SyscallContext) -> SyscallResult {
        // 1. 性能监控开始
        let perf_id = self.performance_monitor.start_measurement(context.syscall_id);

        // 2. 安全检查
        self.security_manager.check_permissions(&context)?;

        // 3. 缓存检查
        if let Some(cached_result) = self.cache.get(&context) {
            self.performance_monitor.end_measurement(perf_id, &cached_result);
            return Ok(cached_result);
        }

        // 4. 服务查找
        let service = self.registry.find_service(context.syscall_id)
            .ok_or(SyscallError::UnsupportedSyscall)?;

        // 5. 参数验证
        service.validate_args(context.syscall_id, &context.args)?;

        // 6. 执行系统调用
        let result = service.handle_syscall(&context);

        // 7. 缓存结果（如果适用）
        if let Ok(ref response) = result {
            if self.config.cache_enabled && self.is_cacheable(context.syscall_id) {
                self.cache.put(context.clone(), response.clone());
            }
        }

        // 8. 性能监控结束
        self.performance_monitor.end_measurement(perf_id, &result);

        result
    }

    pub fn register_service(&mut self, service: Box<dyn SyscallService>) -> Result<ServiceId, RegistryError> {
        let service_id = self.registry.register_service(service)?;
        
        // 注册到DI容器
        if let Some(service) = self.registry.get_service_by_id(service_id) {
            self.di_container.register(service)?;
        }

        Ok(service_id)
    }

    fn is_cacheable(&self, syscall_id: u32) -> bool {
        // 定义哪些系统调用可以被缓存
        match syscall_id {
            SYS_GETPID | SYS_GETUID | SYS_GETGID | SYS_GETPPID | SYS_GETPGID => true,
            _ => false,
        }
    }
}
```

#### 3.2 进程服务实现

```rust
// 进程系统调用服务
pub struct ProcessSyscallService {
    process_manager: Arc<dyn ProcessManager>,
    security_manager: Arc<dyn SecurityManager>,
    config: ProcessServiceConfig,
}

impl ProcessSyscallService {
    pub fn new(
        process_manager: Arc<dyn ProcessManager>,
        security_manager: Arc<dyn SecurityManager>,
    ) -> Self {
        Self {
            process_manager,
            security_manager,
            config: ProcessServiceConfig::default(),
        }
    }
}

impl SyscallService for ProcessSyscallService {
    fn service_name(&self) -> &str {
        "process_syscall_service"
    }

    fn service_version(&self) -> ServiceVersion {
        ServiceVersion { major: 1, minor: 0, patch: 0 }
    }

    fn handle_syscall(&self, context: &SyscallContext) -> SyscallResult {
        match context.syscall_id {
            SYS_FORK => self.handle_fork(context),
            SYS_EXECVE => self.handle_execve(context),
            SYS_WAITPID => self.handle_waitpid(context),
            SYS_EXIT => self.handle_exit(context),
            SYS_GETPID => self.handle_getpid(context),
            SYS_GETPPID => self.handle_getppid(context),
            _ => Err(SyscallError::UnsupportedSyscall),
        }
    }

    fn get_supported_syscalls(&self) -> Vec<u32> {
        vec![
            SYS_FORK, SYS_EXECVE, SYS_WAITPID, SYS_EXIT,
            SYS_GETPID, SYS_GETPPID, SYS_SETUID, SYS_GETUID,
            SYS_SETGID, SYS_GETGID, SYS_SETSID, SYS_GETSID,
        ]
    }

    fn validate_args(&self, syscall_id: u32, args: &[u64]) -> Result<(), ValidationError> {
        match syscall_id {
            SYS_FORK => {
                if !args.is_empty() {
                    return Err(ValidationError::InvalidArgumentCount);
                }
            },
            SYS_EXECVE => {
                if args.len() != 3 {
                    return Err(ValidationError::InvalidArgumentCount);
                }
                // 验证指针有效性
                if args[0] == 0 || args[1] == 0 || args[2] == 0 {
                    return Err(ValidationError::InvalidPointer);
                }
            },
            _ => {
                // 其他系统调用的参数验证
            }
        }
        Ok(())
    }
}

impl ProcessSyscallService {
    fn handle_fork(&self, context: &SyscallContext) -> SyscallResult {
        // 安全检查
        self.security_manager.check_fork_permission(&context.caller_credentials)?;

        // 调用进程管理器
        match self.process_manager.fork_process(context.caller_pid) {
            Ok(child_pid) => {
                let response = if child_pid == context.caller_pid {
                    // 子进程返回0
                    SyscallResponse {
                        return_value: 0,
                        metadata: ResponseMetadata::default(),
                        performance_metrics: None,
                    }
                } else {
                    // 父进程返回子进程PID
                    SyscallResponse {
                        return_value: child_pid as u64,
                        metadata: ResponseMetadata::default(),
                        performance_metrics: None,
                    }
                };
                Ok(response)
            },
            Err(e) => Err(SyscallError::ProcessError(e)),
        }
    }

    fn handle_execve(&self, context: &SyscallContext) -> SyscallResult {
        let args = &context.args;
        let pathname_ptr = args[0] as usize;
        let argv_ptr = args[1] as usize;
        let envp_ptr = args[2] as usize;

        // 安全检查
        self.security_manager.check_exec_permission(&context.caller_credentials)?;

        // 从用户空间读取参数
        let pathname = self.read_string_from_user(pathname_ptr)?;
        let argv = self.read_argv_array_from_user(argv_ptr)?;
        let envp = self.read_argv_array_from_user(envp_ptr)?;

        // 调用进程管理器
        match self.process_manager.exec_process(
            context.caller_pid,
            &pathname,
            &argv,
            &envp,
        ) {
            Ok(_) => {
                // exec成功不会返回，这里只是为了类型检查
                Ok(SyscallResponse {
                    return_value: 0,
                    metadata: ResponseMetadata::default(),
                    performance_metrics: None,
                })
            },
            Err(e) => Err(SyscallError::ProcessError(e)),
        }
    }

    fn handle_getpid(&self, context: &SyscallContext) -> SyscallResult {
        // 快速路径：直接返回PID
        let response = SyscallResponse {
            return_value: context.caller_pid as u64,
            metadata: ResponseMetadata::default(),
            performance_metrics: None,
        };
        Ok(response)
    }

    // 辅助方法
    fn read_string_from_user(&self, ptr: usize) -> Result<String, SyscallError> {
        // 使用HAL抽象层读取用户空间字符串
        let memory_hal = self.di_container.resolve::<MemoryHAL>()
            .map_err(|_| SyscallError::ServiceUnavailable)?;
        
        memory_hal.read_user_string(ptr)
            .map_err(|_| SyscallError::BadAddress)
    }

    fn read_argv_array_from_user(&self, ptr: usize) -> Result<Vec<String>, SyscallError> {
        // 使用HAL抽象层读取用户空间参数数组
        let memory_hal = self.di_container.resolve::<MemoryHAL>()
            .map_err(|_| SyscallError::ServiceUnavailable)?;
        
        memory_hal.read_user_argv_array(ptr)
            .map_err(|_| SyscallError::BadAddress)
    }
}
```

#### 3.3 文件I/O服务实现

```rust
// 文件I/O系统调用服务
pub struct FileIOSyscallService {
    file_manager: Arc<dyn FileManager>,
    vfs_manager: Arc<dyn VfsManager>,
    security_manager: Arc<dyn SecurityManager>,
    io_hal: Arc<dyn IoHAL>,
    config: FileIOServiceConfig,
}

impl FileIOSyscallService {
    pub fn new(
        file_manager: Arc<dyn FileManager>,
        vfs_manager: Arc<dyn VfsManager>,
        security_manager: Arc<dyn SecurityManager>,
        io_hal: Arc<dyn IoHAL>,
    ) -> Self {
        Self {
            file_manager,
            vfs_manager,
            security_manager,
            io_hal,
            config: FileIOServiceConfig::default(),
        }
    }
}

impl SyscallService for FileIOSyscallService {
    fn service_name(&self) -> &str {
        "fileio_syscall_service"
    }

    fn service_version(&self) -> ServiceVersion {
        ServiceVersion { major: 1, minor: 0, patch: 0 }
    }

    fn handle_syscall(&self, context: &SyscallContext) -> SyscallResult {
        match context.syscall_id {
            SYS_OPEN => self.handle_open(context),
            SYS_READ => self.handle_read(context),
            SYS_WRITE => self.handle_write(context),
            SYS_CLOSE => self.handle_close(context),
            _ => Err(SyscallError::UnsupportedSyscall),
        }
    }

    fn get_supported_syscalls(&self) -> Vec<u32> {
        vec![
            SYS_OPEN, SYS_READ, SYS_WRITE, SYS_CLOSE,
            SYS_LSEEK, SYS_STAT, SYS_FSTAT, SYS_DUP,
            SYS_DUP2, SYS_FCNTL, SYS_POLL, SYS_SELECT,
        ]
    }

    fn validate_args(&self, syscall_id: u32, args: &[u64]) -> Result<(), ValidationError> {
        match syscall_id {
            SYS_READ => {
                if args.len() != 3 {
                    return Err(ValidationError::InvalidArgumentCount);
                }
                let fd = args[0] as i32;
                if fd < 0 {
                    return Err(ValidationError::InvalidFileDescriptor);
                }
            },
            SYS_WRITE => {
                if args.len() != 3 {
                    return Err(ValidationError::InvalidArgumentCount);
                }
                let fd = args[0] as i32;
                if fd < 0 {
                    return Err(ValidationError::InvalidFileDescriptor);
                }
            },
            _ => {
                // 其他系统调用的参数验证
            }
        }
        Ok(())
    }
}

impl FileIOSyscallService {
    fn handle_read(&self, context: &SyscallContext) -> SyscallResult {
        let args = &context.args;
        let fd = args[0] as i32;
        let buf_ptr = args[1] as usize;
        let count = args[2] as usize;

        // 快速路径：小缓冲区优化
        if self.config.fast_path_enabled && count <= 4096 && fd >= 0 && fd < 8 {
            return self.fast_path_read(context, fd, buf_ptr, count);
        }

        // 安全检查
        self.security_manager.check_file_access(
            &context.caller_credentials,
            fd,
            FileAccessMode::Read,
        )?;

        // 获取文件描述符
        let file_handle = self.file_manager.get_file_handle(fd)
            .ok_or(SyscallError::BadFileDescriptor)?;

        // 读取数据
        let mut buffer = vec![0u8; count];
        let bytes_read = self.io_hal.read_file(file_handle, &mut buffer)?;

        // 复制到用户空间
        let memory_hal = self.di_container.resolve::<MemoryHAL>()
            .map_err(|_| SyscallError::ServiceUnavailable)?;
        
        memory_hal.copy_to_user(buf_ptr, &buffer[..bytes_read])
            .map_err(|_| SyscallError::BadAddress)?;

        let response = SyscallResponse {
            return_value: bytes_read as u64,
            metadata: ResponseMetadata::default(),
            performance_metrics: Some(PerformanceMetrics {
                bytes_transferred: bytes_read as u64,
                operation_time: 0, // 由性能监控器填充
                cache_hits: 0,
            }),
        };
        Ok(response)
    }

    fn fast_path_read(&self, context: &SyscallContext, fd: i32, buf_ptr: usize, count: usize) -> SyscallResult {
        // 快速路径实现：使用栈分配的缓冲区，减少锁竞争
        let mut stack_buffer = [0u8; 4096];
        let read_count = count.min(4096);

        // 获取文件句柄（无锁快速查找）
        let file_handle = self.file_manager.get_file_handle_fast(fd)
            .ok_or(SyscallError::BadFileDescriptor)?;

        // 直接读取
        let bytes_read = self.io_hal.read_file_fast(file_handle, &mut stack_buffer[..read_count])?;

        // 复制到用户空间
        let memory_hal = self.di_container.resolve::<MemoryHAL>()
            .map_err(|_| SyscallError::ServiceUnavailable)?;
        
        memory_hal.copy_to_user_fast(buf_ptr, &stack_buffer[..bytes_read])
            .map_err(|_| SyscallError::BadAddress)?;

        let response = SyscallResponse {
            return_value: bytes_read as u64,
            metadata: ResponseMetadata {
                fast_path: true,
                ..Default::default()
            },
            performance_metrics: Some(PerformanceMetrics {
                bytes_transferred: bytes_read as u64,
                operation_time: 0,
                cache_hits: 1,
            }),
        };
        Ok(response)
    }
}
```

### 4. HAL抽象层设计

#### 4.1 进程HAL接口

```rust
// 进程管理HAL接口
pub trait ProcessHAL: Send + Sync {
    fn create_process(&self, config: ProcessConfig) -> Result<Pid, ProcessError>;
    fn terminate_process(&self, pid: Pid, exit_code: i32) -> Result<(), ProcessError>;
    fn get_process_info(&self, pid: Pid) -> Result<ProcessInfo, ProcessError>;
    fn get_current_process(&self) -> Result<Pid, ProcessError>;
    fn set_process_state(&self, pid: Pid, state: ProcessState) -> Result<(), ProcessError>;
    fn get_process_memory(&self, pid: Pid) -> Result<MemoryInfo, ProcessError>;
    fn schedule_process(&self, pid: Pid, priority: u8) -> Result<(), ProcessError>;
}

// 进程配置
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    pub name: String,
    pub executable_path: String,
    pub arguments: Vec<String>,
    pub environment: Vec<String>,
    pub working_directory: String,
    pub uid: u32,
    pub gid: u32,
    pub priority: u8,
    pub affinity: Option<CpuAffinity>,
    pub resource_limits: ResourceLimits,
}

// 进程信息
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: Pid,
    pub ppid: Pid,
    pub state: ProcessState,
    pub name: String,
    pub uid: u32,
    pub gid: u32,
    pub priority: u8,
    pub affinity: CpuAffinity,
    pub start_time: u64,
    pub cpu_time: u64,
    pub memory_usage: MemoryUsage,
    pub open_files: Vec<FileDescriptor>,
}
```

#### 4.2 内存HAL接口

```rust
// 内存管理HAL接口
pub trait MemoryHAL: Send + Sync {
    fn allocate_pages(&self, count: usize) -> Result<*mut u8, MemoryError>;
    fn free_pages(&self, ptr: *mut u8, count: usize) -> Result<(), MemoryError>;
    fn map_page(&self, vaddr: usize, paddr: usize, flags: PageFlags) -> Result<(), MemoryError>;
    fn unmap_page(&self, vaddr: usize) -> Result<usize, MemoryError>;
    fn protect_page(&self, vaddr: usize, flags: PageFlags) -> Result<(), MemoryError>;
    fn copy_to_user(&self, user_ptr: usize, data: &[u8]) -> Result<(), MemoryError>;
    fn copy_from_user(&self, user_ptr: usize, data: &mut [u8]) -> Result<(), MemoryError>;
    fn read_user_string(&self, ptr: usize) -> Result<String, MemoryError>;
    fn read_user_argv_array(&self, ptr: usize) -> Result<Vec<String>, MemoryError>;
    fn fast_copy_to_user(&self, user_ptr: usize, data: &[u8]) -> Result<(), MemoryError>;
    fn flush_tlb(&self, vaddr: usize);
    fn get_page_info(&self, vaddr: usize) -> Result<PageInfo, MemoryError>;
}

// 页面标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageFlags {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub user_accessible: bool,
    pub global: bool,
    pub cache_disabled: bool,
    pub write_through: bool,
}

// 页面信息
#[derive(Debug, Clone)]
pub struct PageInfo {
    pub virtual_address: usize,
    pub physical_address: usize,
    pub flags: PageFlags,
    pub size: usize,
    pub reference_count: u32,
    pub last_accessed: u64,
}
```

#### 4.3 I/O HAL接口

```rust
// I/O管理HAL接口
pub trait IoHAL: Send + Sync {
    fn open_file(&self, path: &str, flags: OpenFlags) -> Result<FileHandle, IoError>;
    fn close_file(&self, handle: FileHandle) -> Result<(), IoError>;
    fn read_file(&self, handle: FileHandle, buffer: &mut [u8]) -> Result<usize, IoError>;
    fn write_file(&self, handle: FileHandle, data: &[u8]) -> Result<usize, IoError>;
    fn seek_file(&self, handle: FileHandle, offset: isize, whence: SeekWhence) -> Result<u64, IoError>;
    fn stat_file(&self, path: &str) -> Result<FileStat, IoError>;
    fn fstat_file(&self, handle: FileHandle) -> Result<FileStat, IoError>;
    fn dup_file(&self, handle: FileHandle) -> Result<FileHandle, IoError>;
    fn dup2_file(&self, old_handle: FileHandle, new_handle: FileHandle) -> Result<(), IoError>;
    fn poll_files(&self, handles: &[FileHandle], timeout: u32) -> Result<Vec<PollEvent>, IoError>;
    
    // 快速路径方法
    fn read_file_fast(&self, handle: FileHandle, buffer: &mut [u8]) -> Result<usize, IoError>;
    fn write_file_fast(&self, handle: FileHandle, data: &[u8]) -> Result<usize, IoError>;
    fn get_file_handle_fast(&self, fd: i32) -> Option<FileHandle>;
}

// 文件句柄
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileHandle {
    pub id: u64,
    pub type: FileType,
    pub flags: FileFlags,
    pub position: u64,
}

// 文件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Regular,
    Directory,
    CharacterDevice,
    BlockDevice,
    Pipe,
    Socket,
    Symlink,
}
```

### 5. 插件化架构支持

#### 5.1 系统调用插件接口

```rust
// 系统调用插件接口
pub trait SyscallPlugin: Send + Sync {
    fn plugin_name(&self) -> &str;
    fn plugin_version(&self) -> PluginVersion;
    fn plugin_type(&self) -> PluginType;
    fn initialize(&mut self, context: &PluginContext) -> Result<(), PluginError>;
    fn shutdown(&mut self) -> Result<(), PluginError>;
    fn get_syscall_services(&self) -> Vec<Box<dyn SyscallService>>;
    fn get_dependencies(&self) -> Vec<PluginDependency>;
    fn get_capabilities(&self) -> PluginCapabilities;
}

// 插件上下文
#[derive(Debug)]
pub struct PluginContext {
    pub di_container: Arc<dyn DIContainer>,
    pub service_registry: Arc<dyn ServiceRegistry>,
    pub config_manager: Arc<dyn ConfigManager>,
    pub logger: Arc<dyn Logger>,
}

// 插件依赖
#[derive(Debug, Clone)]
pub struct PluginDependency {
    pub name: String,
    pub version_range: VersionRange,
    pub optional: bool,
}

// 插件能力
#[derive(Debug, Clone)]
pub struct PluginCapabilities {
    pub privileged_operations: bool,
    pub direct_hardware_access: bool,
    pub network_access: bool,
    pub file_system_access: bool,
    pub process_control: bool,
    pub memory_management: bool,
}
```

#### 5.2 插件管理器

```rust
// 插件管理器
pub struct PluginManager {
    plugins: HashMap<PluginId, Box<dyn SyscallPlugin>>,
    loaded_plugins: HashSet<PluginId>,
    plugin_loader: Box<dyn PluginLoader>,
    security_manager: Arc<dyn SecurityManager>,
    config: PluginManagerConfig,
}

impl PluginManager {
    pub fn new(
        plugin_loader: Box<dyn PluginLoader>,
        security_manager: Arc<dyn SecurityManager>,
    ) -> Self {
        Self {
            plugins: HashMap::new(),
            loaded_plugins: HashSet::new(),
            plugin_loader,
            security_manager,
            config: PluginManagerConfig::default(),
        }
    }

    pub fn load_plugin(&mut self, plugin_path: &str) -> Result<PluginId, PluginError> {
        // 1. 安全检查
        self.security_manager.validate_plugin(plugin_path)?;

        // 2. 加载插件
        let plugin = self.plugin_loader.load_plugin(plugin_path)?;

        // 3. 验证依赖
        self.validate_plugin_dependencies(&plugin)?;

        // 4. 初始化插件
        let context = PluginContext {
            di_container: self.get_di_container(),
            service_registry: self.get_service_registry(),
            config_manager: self.get_config_manager(),
            logger: self.get_logger(),
        };

        let mut plugin = plugin;
        plugin.initialize(&context)?;

        // 5. 注册服务
        let services = plugin.get_syscall_services();
        for service in services {
            self.register_syscall_service(service)?;
        }

        // 6. 记录插件
        let plugin_id = PluginId::new();
        self.plugins.insert(plugin_id, Box::new(plugin));
        self.loaded_plugins.insert(plugin_id);

        Ok(plugin_id)
    }

    pub fn unload_plugin(&mut self, plugin_id: PluginId) -> Result<(), PluginError> {
        if let Some(mut plugin) = self.plugins.remove(&plugin_id) {
            // 1. 停止服务
            let services = plugin.get_syscall_services();
            for service in services {
                self.unregister_syscall_service(&service)?;
            }

            // 2. 关闭插件
            plugin.shutdown()?;

            // 3. 更新状态
            self.loaded_plugins.remove(&plugin_id);

            Ok(())
        } else {
            Err(PluginError::NotFound)
        }
    }

    fn validate_plugin_dependencies(&self, plugin: &dyn SyscallPlugin) -> Result<(), PluginError> {
        let dependencies = plugin.get_dependencies();
        
        for dep in dependencies {
            if !dep.optional {
                let found = self.plugins.values().any(|p| {
                    p.plugin_name() == dep.name &&
                    dep.version_range.contains(&p.plugin_version())
                });
                
                if !found {
                    return Err(PluginError::DependencyMissing(dep.name.clone()));
                }
            }
        }
        
        Ok(())
    }
}
```

### 6. 性能优化设计

#### 6.1 快速路径优化

```rust
// 快速路径处理器
pub struct FastPathHandler {
    enabled: bool,
    fast_syscalls: HashSet<u32>,
    stack_buffers: Vec<[u8; 4096]>,
    buffer_pool: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl FastPathHandler {
    pub fn new() -> Self {
        Self {
            enabled: true,
            fast_syscalls: HashSet::from_iter([
                SYS_GETPID, SYS_GETPPID, SYS_GETUID, SYS_GETGID,
                SYS_READ, SYS_WRITE, SYS_CLOSE,
            ]),
            stack_buffers: vec![[0u8; 4096]; 8], // 8个栈缓冲区
            buffer_pool: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn handle_fast_syscall(&mut self, context: &SyscallContext) -> Option<SyscallResult> {
        if !self.enabled || !self.fast_syscalls.contains(&context.syscall_id) {
            return None;
        }

        match context.syscall_id {
            SYS_GETPID => Some(self.fast_getpid(context)),
            SYS_READ => self.fast_read(context),
            SYS_WRITE => self.fast_write(context),
            _ => None,
        }
    }

    fn fast_getpid(&self, context: &SyscallContext) -> SyscallResult {
        let response = SyscallResponse {
            return_value: context.caller_pid as u64,
            metadata: ResponseMetadata {
                fast_path: true,
                ..Default::default()
            },
            performance_metrics: Some(PerformanceMetrics {
                operation_time: 50, // 50ns
                cache_hits: 1,
                ..Default::default()
            }),
        };
        Ok(response)
    }

    fn fast_read(&mut self, context: &SyscallContext) -> Option<SyscallResult> {
        let args = &context.args;
        if args.len() != 3 {
            return None;
        }

        let fd = args[0] as i32;
        let buf_ptr = args[1] as usize;
        let count = args[2] as usize;

        // 快速路径条件检查
        if fd < 0 || fd >= 8 || buf_ptr == 0 || count == 0 || count > 4096 {
            return None;
        }

        // 使用栈缓冲区
        let buffer_index = (fd as usize) % self.stack_buffers.len();
        let stack_buffer = &mut self.stack_buffers[buffer_index];
        let read_count = count.min(4096);

        // 执行快速读取
        let bytes_read = self.execute_fast_read(fd, &mut stack_buffer[..read_count])?;

        // 快速复制到用户空间
        if self.fast_copy_to_user(buf_ptr, &stack_buffer[..bytes_read]).is_err() {
            return None;
        }

        let response = SyscallResponse {
            return_value: bytes_read as u64,
            metadata: ResponseMetadata {
                fast_path: true,
                ..Default::default()
            },
            performance_metrics: Some(PerformanceMetrics {
                bytes_transferred: bytes_read as u64,
                operation_time: 300, // 300ns
                cache_hits: 1,
            }),
        };
        Some(Ok(response))
    }
}
```

#### 6.2 缓存系统

```rust
// 系统调用缓存
pub trait SyscallCache: Send + Sync {
    fn get(&self, context: &SyscallContext) -> Option<SyscallResponse>;
    fn put(&mut self, context: SyscallContext, response: SyscallResponse);
    fn invalidate(&mut self, pattern: &CacheInvalidationPattern);
    fn clear(&mut self);
    fn get_stats(&self) -> CacheStats;
}

// 默认缓存实现
pub struct DefaultSyscallCache {
    entries: HashMap<CacheKey, CacheEntry>,
    lru_list: VecDeque<CacheKey>,
    max_entries: usize,
    stats: CacheStats,
}

impl DefaultSyscallCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            lru_list: VecDeque::new(),
            max_entries,
            stats: CacheStats::default(),
        }
    }
}

impl SyscallCache for DefaultSyscallCache {
    fn get(&self, context: &SyscallContext) -> Option<SyscallResponse> {
        let key = CacheKey::from_context(context);
        
        if let Some(entry) = self.entries.get(&key) {
            if entry.is_valid() {
                self.stats.hits += 1;
                return Some(entry.response.clone());
            }
        }
        
        self.stats.misses += 1;
        None
    }

    fn put(&mut self, context: SyscallContext, response: SyscallResponse) {
        let key = CacheKey::from_context(&context);
        
        // 检查是否应该缓存
        if !self.should_cache(&key, &response) {
            return;
        }

        let entry = CacheEntry {
            response,
            timestamp: self.get_current_time(),
            access_count: 1,
        };

        // 如果缓存已满，移除最旧的条目
        if self.entries.len() >= self.max_entries {
            if let Some(old_key) = self.lru_list.pop_front() {
                self.entries.remove(&old_key);
            }
        }

        self.entries.insert(key, entry);
        self.lru_list.push_back(key);
        self.stats.puts += 1;
    }

    fn should_cache(&self, key: &CacheKey, response: &SyscallResponse) -> bool {
        // 只缓存纯函数（无副作用的系统调用）
        match key.syscall_id {
            SYS_GETPID | SYS_GETUID | SYS_GETGID | SYS_GETPPID => true,
            _ => false,
        }
    }
}

// 缓存键
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CacheKey {
    pub syscall_id: u32,
    pub args_hash: u64,
    pub caller_uid: u32,
    pub caller_gid: u32,
}

impl CacheKey {
    pub fn from_context(context: &SyscallContext) -> Self {
        let args_hash = self.hash_args(&context.args);
        
        Self {
            syscall_id: context.syscall_id,
            args_hash,
            caller_uid: context.caller_credentials.uid,
            caller_gid: context.caller_credentials.gid,
        }
    }

    fn hash_args(args: &[u64]) -> u64 {
        // 使用简单的哈希算法
        let mut hash = 0u64;
        for &arg in args {
            hash = hash.wrapping_mul(31).wrapping_add(arg);
        }
        hash
    }
}
```

### 7. 安全性设计

#### 7.1 安全管理器

```rust
// 安全管理器接口
pub trait SecurityManager: Send + Sync {
    fn check_permissions(&self, context: &SyscallContext) -> Result<(), SecurityError>;
    fn check_file_access(&self, credentials: &ProcessCredentials, fd: i32, mode: FileAccessMode) -> Result<(), SecurityError>;
    fn check_fork_permission(&self, credentials: &ProcessCredentials) -> Result<(), SecurityError>;
    fn check_exec_permission(&self, credentials: &ProcessCredentials) -> Result<(), SecurityError>;
    fn validate_plugin(&self, plugin_path: &str) -> Result<(), SecurityError>;
    fn audit_syscall(&self, context: &SyscallContext, result: &SyscallResult);
}

// 默认安全管理器实现
pub struct DefaultSecurityManager {
    policy_engine: Box<dyn SecurityPolicyEngine>,
    audit_logger: Box<dyn AuditLogger>,
    capabilities: HashMap<u32, CapabilitySet>,
}

impl DefaultSecurityManager {
    pub fn new() -> Self {
        Self {
            policy_engine: Box::new(DefaultPolicyEngine::new()),
            audit_logger: Box::new(DefaultAuditLogger::new()),
            capabilities: HashMap::new(),
        }
    }
}

impl SecurityManager for DefaultSecurityManager {
    fn check_permissions(&self, context: &SyscallContext) -> Result<(), SecurityError> {
        // 1. 检查基本权限
        if !self.has_basic_permission(&context.caller_credentials, context.syscall_id) {
            return Err(SecurityError::PermissionDenied);
        }

        // 2. 检查特殊权限
        if self.is_privileged_syscall(context.syscall_id) {
            if !self.has_privilege(&context.caller_credentials, Privilege::Admin) {
                return Err(SecurityError::InsufficientPrivilege);
            }
        }

        // 3. 检查资源限制
        if let Some(limit) = self.get_resource_limit(&context.caller_credentials.uid, context.syscall_id) {
            if self.exceeds_limit(&context, limit) {
                return Err(SecurityError::ResourceLimitExceeded);
            }
        }

        Ok(())
    }

    fn audit_syscall(&self, context: &SyscallContext, result: &SyscallResult) {
        let audit_record = AuditRecord {
            timestamp: self.get_current_time(),
            pid: context.caller_pid,
            uid: context.caller_credentials.uid,
            syscall_id: context.syscall_id,
            args: context.args.clone(),
            result: result.clone(),
            success: result.is_ok(),
        };

        self.audit_logger.log_audit_record(audit_record);
    }
}
```

## 验收标准

### 1. 功能完整性
- [ ] 系统调用模块完全解耦，通过接口交互
- [ ] 支持动态服务注册和发现
- [ ] 实现完整的HAL抽象层
- [ ] 支持插件化扩展
- [ ] 提供完整的性能监控

### 2. 性能指标
- [ ] 系统调用延迟降低30%
- [ ] 快速路径命中率达到80%
- [ ] 缓存命中率达到60%
- [ ] 内存使用优化20%
- [ ] CPU使用率降低15%

### 3. 可维护性
- [ ] 模块间耦合度降低到<0.3
- [ ] 代码重复率降低到<10%
- [ ] 单元测试覆盖率达到90%
- [ ] 接口稳定性达到95%
- [ ] 文档完整性达到100%

### 4. 可扩展性
- [ ] 支持热插拔系统调用服务
- [ ] 支持插件动态加载/卸载
- [ ] 支持多架构HAL实现
- [ ] 支持运行时配置更新
- [ ] 支持版本兼容性检查

## 实施计划

### 阶段1：核心架构重构（1周）
1. 实现基础接口定义
2. 创建服务注册表和DI容器
3. 实现系统调用管理器
4. 创建HAL抽象层接口

### 阶段2：服务实现（1周）
1. 重构进程系统调用服务
2. 重构文件I/O系统调用服务
3. 重构内存管理系统调用服务
4. 重构信号处理系统调用服务

### 阶段3：性能优化（0.5周）
1. 实现快速路径处理
2. 实现系统调用缓存
3. 优化参数传递
4. 性能基准测试

### 阶段4：插件化支持（0.5周）
1. 实现插件管理器
2. 创建插件接口
3. 实现安全沙箱
4. 插件示例和文档

## 结论

通过这个重构设计，NOS内核系统调用模块将实现：

1. **低耦合架构**：模块间通过接口交互，依赖关系清晰
2. **高性能**：快速路径、缓存机制、批量操作
3. **高可扩展性**：插件化架构支持动态扩展
4. **高安全性**：统一的安全检查和审计机制
5. **高可维护性**：清晰的模块边界和标准化接口

这个重构将为NOS内核的长期发展提供坚实的基础，支持系统的持续演进和优化。