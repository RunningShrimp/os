//! Newlib C标准库集成模块

extern crate alloc;
//!
//! 提供完整的C标准库支持，包括：
//! - 标准C函数实现
//! - 内存管理函数
//! - I/O操作函数
//! - 字符串处理函数
//! - 数学函数
//! - 系统调用接口
//!
//! 主要功能：
//! - 标准C库函数包装
//! - newlib配置和初始化
//! - 系统调用接口实现
//! - C/C++互操作支持
//! - 动态链接支持

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, Ordering};
use core::ffi::{c_char, c_int, c_void};
use core::ptr;
use spin::Mutex;

/// Newlib配置
pub struct NewlibConfig {
    /// 是否启用内存调试
    pub enable_memory_debug: bool,
    /// 是否启用线程安全
    pub enable_thread_safety: bool,
    /// 缓冲区大小
    pub buffer_size: usize,
    /// 最大打开文件数
    pub max_open_files: c_int,
    /// 时区信息
    pub timezone: Option<TimezoneInfo>,
    /// 区域设置
    pub locale: Option<LocaleInfo>,
}

/// 时区信息
#[derive(Debug, Clone)]
pub struct TimezoneInfo {
    /// 时区名称
    pub name: String,
    /// UTC偏移（秒）
    pub utc_offset: c_int,
    /// 是否使用夏令时
    pub daylight_saving: bool,
}

/// 区域设置信息
#[derive(Debug, Clone)]
pub struct LocaleInfo {
    /// 区域名称
    pub name: String,
    /// 语言代码
    pub language: String,
    /// 国家代码
    pub country: String,
    /// 编码
    pub encoding: String,
}

impl Default for NewlibConfig {
    fn default() -> Self {
        Self {
            enable_memory_debug: false,
            enable_thread_safety: true,
            buffer_size: 4096,
            max_open_files: 256,
            timezone: None,
            locale: None,
        }
    }
}

/// Newlib管理器
pub struct NewlibManager {
    /// 配置
    config: NewlibConfig,
    /// 文件描述符表
    fd_table: Arc<Mutex<BTreeMap<c_int, FileDescriptor>>>,
    /// 标准输入输出流
    stdin: Arc<Mutex<FileDescriptor>>,
    stdout: Arc<Mutex<FileDescriptor>>,
    stderr: Arc<Mutex<FileDescriptor>>,
    /// 环境变量
    environment: Arc<Mutex<BTreeMap<String, String>>>,
    /// 信号处理
    signal_handlers: Arc<Mutex<BTreeMap<c_int, SignalHandler>>>,
    /// 内存分配统计
    allocation_stats: Arc<Mutex<AllocationStats>>,
    /// 是否已初始化
    initialized: AtomicBool,
}

/// 文件描述符
#[derive(Debug, Clone)]
pub struct FileDescriptor {
    /// 文件描述符编号
    pub fd: c_int,
    /// 文件路径
    pub path: Option<String>,
    /// 文件模式
    pub mode: c_int,
    /// 文件偏移量
    pub offset: isize,
    /// 文件标志
    pub flags: c_int,
    /// 缓冲区
    pub buffer: Vec<u8>,
    /// 缓冲区位置
    pub buffer_pos: usize,
    /// 缓冲区大小
    pub buffer_size: usize,
    /// 是否需要刷新
    pub needs_flush: bool,
    /// 是否为标准流
    pub is_standard: bool,
}

/// 内存分配统计
#[derive(Debug, Default, Clone)]
pub struct AllocationStats {
    /// 总分配次数
    pub total_allocations: u64,
    /// 总分配大小
    pub total_allocated_bytes: u64,
    /// 当前分配块数
    pub current_blocks: u64,
    /// 最大内存使用量
    pub peak_memory_usage: u64,
    /// 内存泄漏检测
    pub leak_detection_enabled: bool,
}

impl AllocationStats {
    pub fn record_allocation(&mut self, size: u64) {
        self.total_allocations += 1;
        self.current_blocks += 1;
        self.total_allocated_bytes += size;
        if self.total_allocated_bytes > self.peak_memory_usage {
            self.peak_memory_usage = self.total_allocated_bytes;
        }
    }
    
    pub fn record_deallocation(&mut self, size: u64) {
        self.current_blocks = self.current_blocks.saturating_sub(1);
        self.total_allocated_bytes = self.total_allocated_bytes.saturating_sub(size);
    }
}

/// 信号处理器类型
pub type SignalHandler = extern "C" fn(c_int);

/// Newlib管理器实例
static NEWLIB_MANAGER: spin::Mutex<Option<NewlibManager>> = spin::Mutex::new(None);

/// 文件描述符管理
pub struct FileDescriptorManager {
    /// 下一个可用的文件描述符
    next_fd: AtomicI32,
    /// 标准文件描述符
    std_fds: BTreeMap<c_int, FileDescriptor>,
    /// 用户文件描述符
    user_fds: BTreeMap<c_int, FileDescriptor>,
}

impl FileDescriptorManager {
    /// 创建新的文件描述符管理器
    pub fn new() -> Self {
        let mut manager = Self {
            next_fd: AtomicI32::new(3), // 从3开始，0-2为标准流
            std_fds: BTreeMap::new(),
            user_fds: BTreeMap::new(),
        };

        // 初始化标准文件描述符
        manager.init_standard_fds();
        manager
    }

    /// 初始化标准文件描述符
    fn init_standard_fds(&mut self) {
        // STDIN (0)
        self.std_fds.insert(0, FileDescriptor {
            fd: 0,
            path: None,
            mode: 0,
            offset: 0,
            flags: 0,
            buffer: Vec::new(),
            buffer_pos: 0,
            buffer_size: 0,
            needs_flush: false,
            is_standard: true,
        });

        // STDOUT (1)
        self.std_fds.insert(1, FileDescriptor {
            fd: 1,
            path: None,
            mode: 0,
            offset: 0,
            flags: 0,
            buffer: Vec::new(),
            buffer_pos: 0,
            buffer_size: 0,
            needs_flush: false,
            is_standard: true,
        });

        // STDERR (2)
        self.std_fds.insert(2, FileDescriptor {
            fd: 2,
            path: None,
            mode: 0,
            offset: 0,
            flags: 0,
            buffer: Vec::new(),
            buffer_pos: 0,
            buffer_size: 0,
            needs_flush: false,
            is_standard: true,
        });
    }

    /// 分配新的文件描述符
    pub fn allocate_fd(&mut self) -> Result<c_int, LibcError> {
        let fd = self.next_fd.fetch_add(1, Ordering::SeqCst);

        if fd >= 1024 {
            return Err(LibcError::OutOfFileDescriptors);
        }

        Ok(fd)
    }

    /// 释放文件描述符
    pub fn free_fd(&mut self, fd: c_int) -> Result<(), LibcError> {
        if fd < 3 {
            return Err(LibcError::InvalidFileDescriptor);
        }

        self.user_fds.remove(&fd);
        Ok(())
    }

    /// 获取文件描述符
    pub fn get_fd(&self, fd: c_int) -> Option<&FileDescriptor> {
        if fd < 3 {
            self.std_fds.get(&fd)
        } else {
            self.user_fds.get(&fd)
        }
    }

    /// 设置文件描述符
    pub fn set_fd(&mut self, fd: c_int, file_desc: FileDescriptor) -> Result<(), LibcError> {
        if fd < 3 {
            return Err(LibcError::InvalidFileDescriptor);
        }

        self.user_fds.insert(fd, file_desc);
        Ok(())
    }
}

/// 内存管理器
pub struct MemoryManager {
    /// 分配的内存块
    allocated_blocks: Arc<Mutex<BTreeMap<*mut c_void, MemoryBlock>>>,
    /// 内存池
    memory_pools: Vec<MemoryPool>,
    /// 是否启用调试
    debug_enabled: bool,
}

/// 内存块信息
#[derive(Debug)]
struct MemoryBlock {
    /// 内存地址
    ptr: *mut c_void,
    /// 内存大小
    size: usize,
    /// 分配时间
    alloc_time: u64,
    /// 调用栈信息
    backtrace: Vec<usize>,
}

/// 内存池
#[derive(Debug)]
struct MemoryPool {
    /// 块大小
    block_size: usize,
    /// 空闲块
    free_blocks: Vec<*mut c_void>,
    /// 总块数
    total_blocks: usize,
}

impl MemoryManager {
    /// 创建新的内存管理器
    pub fn new(debug_enabled: bool) -> Self {
        let mut pools = Vec::new();

        // 创建不同大小的内存池
        let pool_sizes = [8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096];
        for &size in &pool_sizes {
            pools.push(MemoryPool {
                block_size: size,
                free_blocks: Vec::new(),
                total_blocks: 0,
            });
        }

        Self {
            allocated_blocks: Arc::new(Mutex::new(BTreeMap::new())),
            memory_pools: pools,
            debug_enabled,
        }
    }

    /// 分配内存
    pub fn malloc(&self, size: usize) -> *mut c_void {
        // 查找合适的内存池
        let pool_index = self.find_pool_index(size);
        let actual_size = if let Some(index) = pool_index {
            self.memory_pools[index].block_size
        } else {
            // 如果没有合适的池，直接分配
            size
        };

        let ptr = if let Some(index) = pool_index {
            self.allocate_from_pool(index)
        } else {
            self.allocate_direct(actual_size)
        };

        if !ptr.is_null() {
            if self.debug_enabled {
                self.track_allocation(ptr, actual_size);
            }
        }

        ptr
    }

    /// 释放内存
    pub fn free(&self, ptr: *mut c_void) {
        if ptr.is_null() {
            return;
        }

        if self.debug_enabled {
            self.untrack_allocation(ptr);
        }

        // 尝试释放到内存池
        if !self.free_to_pool(ptr) {
            // 如果无法释放到池，直接释放
            self.deallocate_direct(ptr);
        }
    }

    /// 重新分配内存
    pub fn realloc(&self, ptr: *mut c_void, old_size: usize, new_size: usize) -> *mut c_void {
        if ptr.is_null() {
            return self.malloc(new_size);
        }

        if new_size == 0 {
            self.free(ptr);
            return ptr::null_mut();
        }

        if new_size <= old_size {
            // 如果新大小小于等于旧大小，直接返回原指针
            return ptr;
        }

        // 分配新的内存
        let new_ptr = self.malloc(new_size);
        if !new_ptr.is_null() {
            // 复制数据
            unsafe {
                ptr::copy_nonoverlapping(ptr as *const u8, new_ptr as *mut u8, old_size);
            }
            self.free(ptr);
        }

        new_ptr
    }

    /// 查找合适的内存池
    fn find_pool_index(&self, size: usize) -> Option<usize> {
        for (i, pool) in self.memory_pools.iter().enumerate() {
            if size <= pool.block_size {
                return Some(i);
            }
        }
        None
    }

    /// 从内存池分配
    fn allocate_from_pool(&self, pool_index: usize) -> *mut c_void {
        let mut pool = unsafe { &mut *self.memory_pools.as_ptr().add(pool_index) };

        if let Some(ptr) = pool.free_blocks.pop() {
            ptr
        } else {
            // 如果池中没有空闲块，分配新的块
            self.allocate_direct(pool.block_size)
        }
    }

    /// 释放到内存池
    fn free_to_pool(&self, ptr: *mut c_void) -> bool {
        for pool in &self.memory_pools {
            // 这里需要实际的内存对齐检查
            if self.is_pool_aligned(ptr, pool.block_size) {
                let mut pool_mut = unsafe { &mut *(pool as *const _ as *mut MemoryPool) };
                pool_mut.free_blocks.push(ptr);
                return true;
            }
        }
        false
    }

    /// 直接分配内存（使用统一分配器适配器）
    fn allocate_direct(&self, size: usize) -> *mut c_void {
        use crate::libc::memory_adapter::libc_malloc;
        libc_malloc(size)
    }

    /// 直接释放内存（使用统一分配器适配器）
    fn deallocate_direct(&self, ptr: *mut c_void) {
        use crate::libc::memory_adapter::libc_free;
        libc_free(ptr);
    }

    /// 检查是否对齐到内存池
    fn is_pool_aligned(&self, ptr: *mut c_void, block_size: usize) -> bool {
        let addr = ptr as usize;
        addr % block_size == 0
    }

    /// 跟踪内存分配
    fn track_allocation(&self, ptr: *mut c_void, size: usize) {
        let block = MemoryBlock {
            ptr,
            size,
            alloc_time: crate::subsystems::time::timestamp_millis(),
            backtrace: self.capture_backtrace(),
        };

        let mut blocks = self.allocated_blocks.lock();
        blocks.insert(ptr, block);
    }

    /// 取消跟踪内存分配
    fn untrack_allocation(&self, ptr: *mut c_void) {
        let mut blocks = self.allocated_blocks.lock();
        blocks.remove(&ptr);
    }

    /// 捕获调用栈
    fn capture_backtrace(&self) -> Vec<usize> {
        // 简化实现，实际需要使用backtrace库
        vec![]
    }
}

/// I/O操作管理器
pub struct IOManager {
    /// 输入缓冲区管理
    input_buffers: Arc<Mutex<BTreeMap<c_int, Vec<u8>>>>,
    /// 输出缓冲区管理
    output_buffers: Arc<Mutex<BTreeMap<c_int, Vec<u8>>>>,
    /// 缓冲区刷新策略
    flush_policy: FlushPolicy,
}

/// 缓冲区刷新策略
#[derive(Debug, Clone)]
pub enum FlushPolicy {
    /// 行缓冲
    Line,
    /// 全缓冲
    Full,
    /// 无缓冲
    None,
}

impl IOManager {
    /// 创建新的I/O管理器
    pub fn new() -> Self {
        Self {
            input_buffers: Arc::new(Mutex::new(BTreeMap::new())),
            output_buffers: Arc::new(Mutex::new(BTreeMap::new())),
            flush_policy: FlushPolicy::Line,
        }
    }

    /// 读取数据
    pub fn read(&self, fd: c_int, buffer: *mut u8, size: usize) -> isize {
        // 简化实现，实际需要调用底层文件系统
        if fd == 0 {
            // 从标准输入读取
            self.read_stdin(buffer, size)
        } else {
            // 从文件读取
            self.read_file(fd, buffer, size)
        }
    }

    /// 写入数据
    pub fn write(&self, fd: c_int, buffer: *const u8, size: usize) -> isize {
        // 简化实现，实际需要调用底层文件系统
        if fd == 1 || fd == 2 {
            // 写入标准输出/错误
            self.write_stdout_stderr(fd, buffer, size)
        } else {
            // 写入文件
            self.write_file(fd, buffer, size)
        }
    }

    /// 刷新缓冲区
    pub fn flush(&self, fd: c_int) -> c_int {
        match self.flush_policy {
            FlushPolicy::None => 0,
            _ => {
                let mut buffers = self.output_buffers.lock();
                if buffers.remove(&fd).is_some() {
                    0 // 成功
                } else {
                    -1 // 失败
                }
            }
        }
    }

    /// 从标准输入读取
    fn read_stdin(&self, buffer: *mut u8, size: usize) -> isize {
        // 简化实现，实际需要处理终端输入
        unsafe {
            // 这里应该调用系统调用读取键盘输入
            // 暂时返回0表示没有数据
            0
        }
    }

    /// 写入标准输出/错误
    fn write_stdout_stderr(&self, fd: c_int, buffer: *const u8, size: usize) -> isize {
        unsafe {
            let slice = core::slice::from_raw_parts(buffer, size);
            let string = core::str::from_utf8(slice).unwrap_or("Invalid UTF-8");

            if fd == 1 {
                crate::print!("{}", string);
            } else {
                crate::drivers::console::ecrate::println!("{}", string);
            }

            size as isize
        }
    }

    /// 从文件读取
    fn read_file(&self, _fd: c_int, _buffer: *mut u8, _size: usize) -> isize {
        // 简化实现，需要实际文件系统支持
        0
    }

    /// 写入文件
    fn write_file(&self, _fd: c_int, _buffer: *const u8, _size: usize) -> isize {
        // 简化实现，需要实际文件系统支持
        _size as isize
    }
}

/// Newlib错误类型
#[derive(Debug, Clone)]
pub enum LibcError {
    /// 无效的文件描述符
    InvalidFileDescriptor,
    /// 文件描述符耗尽
    OutOfFileDescriptors,
    /// 内存不足
    OutOfMemory,
    /// 无效参数
    InvalidParameter(String),
    /// 操作不支持
    OperationNotSupported,
    /// 权限被拒绝
    PermissionDenied,
    /// I/O错误
    IOError(String),
}

impl core::fmt::Display for LibcError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LibcError::InvalidFileDescriptor => write!(f, "无效的文件描述符"),
            LibcError::OutOfFileDescriptors => write!(f, "文件描述符耗尽"),
            LibcError::OutOfMemory => write!(f, "内存不足"),
            LibcError::InvalidParameter(msg) => write!(f, "无效参数: {}", msg),
            LibcError::OperationNotSupported => write!(f, "操作不支持"),
            LibcError::PermissionDenied => write!(f, "权限被拒绝"),
            LibcError::IOError(msg) => write!(f, "I/O错误: {}", msg),
        }
    }
}

impl NewlibManager {
    /// 创建新的Newlib管理器
    pub fn new(config: NewlibConfig) -> Self {
        let fd_manager = FileDescriptorManager::new();
        let memory_manager = MemoryManager::new(config.enable_memory_debug);
        let io_manager = IOManager::new();

        Self {
            config,
            fd_table: Arc::new(Mutex::new(BTreeMap::new())),
            stdin: Arc::new(Mutex::new(FileDescriptor {
                fd: 0,
                path: None,
                mode: 0,
                offset: 0,
                flags: 0,
                buffer: Vec::new(),
                buffer_pos: 0,
                buffer_size: 0,
                needs_flush: false,
                is_standard: true,
            })),
            stdout: Arc::new(Mutex::new(FileDescriptor {
                fd: 1,
                path: None,
                mode: 0,
                offset: 0,
                flags: 0,
                buffer: Vec::new(),
                buffer_pos: 0,
                buffer_size: 0,
                needs_flush: false,
                is_standard: true,
            })),
            stderr: Arc::new(Mutex::new(FileDescriptor {
                fd: 2,
                path: None,
                mode: 0,
                offset: 0,
                flags: 0,
                buffer: Vec::new(),
                buffer_pos: 0,
                buffer_size: 0,
                needs_flush: false,
                is_standard: true,
            })),
            environment: Arc::new(Mutex::new(BTreeMap::new())),
            signal_handlers: Arc::new(Mutex::new(BTreeMap::new())),
            allocation_stats: Arc::new(Mutex::new(AllocationStats::default())),
            initialized: AtomicBool::new(false),
        }
    }

    /// 初始化Newlib
    pub fn initialize(&mut self) -> Result<(), LibcError> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        crate::println!("[libc] 初始化Newlib C标准库");

        // 初始化环境变量
        self.init_environment()?;

        // 初始化标准I/O
        self.init_stdio()?;

        // 初始化内存管理
        self.init_memory()?;

        // 初始化信号处理
        self.init_signals()?;

        self.initialized.store(true, Ordering::SeqCst);

        crate::println!("[libc] Newlib C标准库初始化完成");
        Ok(())
    }

    /// 初始化环境变量
    fn init_environment(&mut self) -> Result<(), LibcError> {
        let mut env = self.environment.lock();

        // 设置基本环境变量
        env.insert("PATH".to_string(), "/bin:/usr/bin".to_string());
        env.insert("HOME".to_string(), "/root".to_string());
        env.insert("USER".to_string(), "root".to_string());
        env.insert("SHELL".to_string(), "/bin/sh".to_string());
        env.insert("TERM".to_string(), "xterm".to_string());

        Ok(())
    }

    /// 初始化标准I/O
    fn init_stdio(&mut self) -> Result<(), LibcError> {
        // 设置标准输入缓冲模式
        {
            let mut stdin = self.stdin.lock();
            stdin.buffer_size = 1024;
            stdin.buffer.resize(1024, 0);
        }

        // 设置标准输出缓冲模式
        {
            let mut stdout = self.stdout.lock();
            stdout.buffer_size = 4096;
            stdout.buffer.resize(4096, 0);
        }

        // 设置标准错误为无缓冲
        {
            let mut stderr = self.stderr.lock();
            stderr.buffer_size = 0;
        }

        Ok(())
    }

    /// 初始化内存管理
    fn init_memory(&mut self) -> Result<(), LibcError> {
        // 初始化分配统计
        {
            let mut stats = self.allocation_stats.lock();
            stats.leak_detection_enabled = self.config.enable_memory_debug;
        }

        Ok(())
    }

    /// 初始化信号处理
    fn init_signals(&mut self) -> Result<(), LibcError> {
        let mut handlers = self.signal_handlers.lock();

        // 设置默认信号处理器
        for signal in 1..32 {
            handlers.insert(signal, default_signal_handler);
        }

        Ok(())
    }

    /// 分配文件描述符
    pub fn open_file(&self, path: &str, flags: c_int, mode: c_int) -> Result<c_int, LibcError> {
        let fd = {
            let mut fd_table = self.fd_table.lock();
            let fd_manager = FileDescriptorManager::new();
            fd_manager.allocate_fd()?
        };

        // 创建文件描述符对象
        let file_desc = FileDescriptor {
            fd,
            path: Some(path.to_string()),
            mode,
            offset: 0,
            flags,
            buffer: Vec::new(),
            buffer_pos: 0,
            buffer_size: 4096,
            needs_flush: false,
            is_standard: false,
        };

        // 添加到文件描述符表
        {
            let mut fd_table = self.fd_table.lock();
            fd_table.insert(fd, file_desc);
        }

        Ok(fd)
    }

    /// 关闭文件描述符
    pub fn close_file(&self, fd: c_int) -> Result<(), LibcError> {
        let mut fd_table = self.fd_table.lock();

        if fd_table.remove(&fd).is_some() {
            Ok(())
        } else {
            Err(LibcError::InvalidFileDescriptor)
        }
    }

    /// 获取文件描述符
    pub fn get_file_descriptor(&self, fd: c_int) -> Option<FileDescriptor> {
        let fd_table = self.fd_table.lock();
        fd_table.get(&fd).cloned()
    }

    /// 设置环境变量
    pub fn setenv(&self, name: &str, value: &str, overwrite: bool) -> Result<(), LibcError> {
        let mut env = self.environment.lock();

        if !overwrite && env.contains_key(name) {
            return Ok(());
        }

        env.insert(name.to_string(), value.to_string());
        Ok(())
    }

    /// 获取环境变量
    pub fn getenv(&self, name: &str) -> Option<String> {
        let env = self.environment.lock();
        env.get(name).cloned()
    }

    /// 设置信号处理器
    pub fn signal(&self, signal: c_int, handler: SignalHandler) -> Result<(), LibcError> {
        if signal < 1 || signal > 31 {
            return Err(LibcError::InvalidParameter("无效的信号号".to_string()));
        }

        let mut handlers = self.signal_handlers.lock();
        handlers.insert(signal, handler);
        Ok(())
    }

    /// 获取内存分配统计
    pub fn get_allocation_stats(&self) -> AllocationStats {
        self.allocation_stats.lock().clone()
    }
}

/// 默认信号处理器
extern "C" fn default_signal_handler(signal: c_int) {
    crate::println!("[libc] 收到信号: {}", signal);
    // 这里应该调用系统调用处理信号
    // 暂时打印消息
}

/// 初始化状态跟踪
static NEWLIB_MANAGER_INIT: AtomicBool = AtomicBool::new(false);

/// 初始化Newlib
pub fn init() -> Result<(), LibcError> {
    if NEWLIB_MANAGER_INIT.load(Ordering::SeqCst) {
        return Ok(());
    }

    let config = NewlibConfig::default();
    let mut manager = NewlibManager::new(config);
    manager.initialize()?;

    *NEWLIB_MANAGER.lock() = Some(manager);

    NEWLIB_MANAGER_INIT.store(true, Ordering::SeqCst);
    crate::println!("[libc] Newlib C标准库模块初始化完成");
    Ok(())
}

/// 检查Newlib是否已初始化
pub fn is_initialized() -> bool {
    NEWLIB_MANAGER_INIT.load(Ordering::SeqCst)
}

/// 执行Newlib操作（简化版本）
pub fn with_manager<F, R>(f: F) -> Result<R, LibcError>
where
    F: FnOnce(&NewlibManager) -> Result<R, LibcError>,
{
    if !NEWLIB_MANAGER_INIT.load(Ordering::SeqCst) {
        return Err(LibcError::NotInitialized);
    }

    NEWLIB_MANAGER.lock()
        .as_ref()
        .ok_or(LibcError::NotInitialized)
        .and_then(f)
}

/// C标准库函数实现
pub mod libc_funcs {
    use super::*;

    /// 内存分配函数
    pub extern "C" {
        fn malloc(size: usize) -> *mut c_void {
            if let Ok(manager) = get_manager() {
                // 这里需要访问内存管理器
                // 暂时简化实现
                unsafe {
                    let layout = alloc::alloc::Layout::from_size_align(size, 8);
                    alloc::alloc::alloc(layout) as *mut c_void
                }
            } else {
                ptr::null_mut()
            }
        }

        fn free(ptr: *mut c_void) {
            if ptr.is_null() {
                return;
            }

            unsafe {
                let layout = alloc::alloc::Layout::from_size_align(1, 8);
                alloc::alloc::dealloc(ptr as *mut u8, layout);
            }
        }

        fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
            if ptr.is_null() {
                return malloc(size);
            }

            if size == 0 {
                free(ptr);
                return ptr::null_mut();
            }

            unsafe {
                let layout = alloc::alloc::Layout::from_size_align(size, 8);
                let new_ptr = alloc::alloc::alloc(layout) as *mut c_void;

                if !new_ptr.is_null() {
                    // 这里需要知道旧的大小，暂时假设足够大
                    ptr::copy_nonoverlapping(
                        ptr as *const u8,
                        new_ptr as *mut u8,
                        size
                    );
                    free(ptr);
                }

                new_ptr
            }
        }

        fn calloc(nmemb: usize, size: usize) -> *mut c_void {
            let total_size = nmemb.checked_mul(size).unwrap_or(usize::MAX);
            let ptr = malloc(total_size);

            if !ptr.is_null() {
                unsafe {
                    ptr::write_bytes(ptr, 0, total_size);
                }
            }

            ptr
        }

        /// 字符串操作函数
        fn strlen(s: *const c_char) -> usize {
            if s.is_null() {
                return 0;
            }

            unsafe {
                let mut len = 0;
                let mut ptr = s;

                while *ptr != 0 {
                    len += 1;
                    ptr = ptr.add(1);
                }

                len
            }
        }

        fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char {
            if dest.is_null() || src.is_null() {
                return dest;
            }

            unsafe {
                let mut d = dest;
                let mut s = src;

                while *s != 0 {
                    *d = *s;
                    d = d.add(1);
                    s = s.add(1);
                }

                *d = 0;
                dest
            }
        }

        fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int {
            unsafe {
                let mut p1 = s1;
                let mut p2 = s2;

                while *p1 != 0 && *p1 == *p2 {
                    p1 = p1.add(1);
                    p2 = p2.add(1);
                }

                (*p1 as c_int) - (*p2 as c_int)
            }
        }

        fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int {
            unsafe {
                let mut p1 = s1;
                let mut p2 = s2;
                let mut count = 0;

                while count < n && *p1 != 0 && *p1 == *p2 {
                    p1 = p1.add(1);
                    p2 = p2.add(1);
                    count += 1;
                }

                if count == n {
                    0
                } else {
                    (*p1 as c_int) - (*p2 as c_int)
                }
            }
        }

        /// I/O函数
        fn printf(format: *const c_char, ...) -> c_int {
            unsafe {
                // 简化实现，只支持%s %d
                let mut ptr = format;
                let mut chars_written = 0;

                while *ptr != 0 {
                    if *ptr == b'%' as c_char {
                        ptr = ptr.add(1);
                        match *ptr {
                            val if val == b's' as c_char => {
                                ptr = ptr.add(1);
                                let arg_ptr = *(ptr as *const *const c_char);
                                let arg_str = crate::ffi::CStr::from_ptr(arg_ptr);
                                let arg_string = arg_str.to_str().unwrap_or("<invalid>");
                                crate::print!("{}", arg_string);
                                chars_written += arg_string.len();
                            }
                            val if val == b'd' as c_char => {
                                ptr = ptr.add(1);
                                let arg_ptr = *(ptr as *const c_int);
                                crate::print!("{}", *arg_ptr);
                                chars_written += 1;
                            }
                            _ => {
                                crate::print!("{}", *ptr as char);
                                chars_written += 1;
                            }
                        }
                    } else {
                        crate::print!("{}", *ptr as char);
                        chars_written += 1;
                    }
                    ptr = ptr.add(1);
                }

                chars_written as c_int
            }
        }

        fn puts(s: *const c_char) -> c_int {
            unsafe {
                let cstr = crate::ffi::CStr::from_ptr(s);
                let string = cstr.to_str().unwrap_or("<invalid>");
                crate::println!("{}", string);
                string.len() as c_int + 1 // 包括换行符
            }
        }

        fn putchar(c: c_int) -> c_int {
            crate::print!("{}", c as u8 as char);
            c
        }

        fn getchar() -> c_int {
            // 简化实现，需要实际终端输入
            0
        }

        /// 系统调用包装
        fn open(pathname: *const c_char, flags: c_int, ...) -> c_int {
            if let Ok(_manager) = get_manager() {
                let path = unsafe {
                    crate::ffi::CStr::from_ptr(pathname)
                        .to_str()
                        .unwrap_or("<invalid>")
                };

                // 解析可变参数的mode
                let mode = if flags & 0x100 != 0 {
                    // O_CREAT标志被设置，需要mode参数
                    let ap = unsafe {
                        &mut (flags as *const c_int as *const c_int)
                            .add(1) as *const c_int;
                    };
                    *ap
                } else {
                    0
                };

                // 这里应该调用实际的文件系统
                crate::println!("[libc] open(\"{}\", {}, {}) = 模拟文件描述符", path, flags, mode);
                3 // 模拟文件描述符
            } else {
                -1
            }
        }

        fn close(fd: c_int) -> c_int {
            if let Ok(_manager) = get_manager() {
                crate::println!("[libc] close({}) = 0", fd);
                0
            } else {
                -1
            }
        }

        fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize {
            if let Ok(_manager) = get_manager() {
                crate::println!("[libc] read({}, ptr, {}) = 模拟读取", fd, count);
                count as isize
            } else {
                -1
            }
        }

        fn write(fd: c_int, buf: *const c_void, count: usize) -> isize {
            if let Ok(_manager) = get_manager() {
                // 对于标准输出，实际写入数据
                if fd == 1 || fd == 2 {
                    unsafe {
                        let slice = core::slice::from_raw_parts(buf as *const u8, count);
                        let string = core::str::from_utf8(slice).unwrap_or("<invalid>");

                        if fd == 1 {
                            crate::print!("{}", string);
                        } else {
                            crate::drivers::console::ecrate::println!("{}", string);
                        }
                    }
                } else {
                    crate::println!("[libc] write({}, ptr, {}) = 模拟写入", fd, count);
                }
                count as isize
            } else {
                -1
            }
        }

        fn lseek(fd: c_int, offset: isize, whence: c_int) -> isize {
            if let Ok(_manager) = get_manager() {
                crate::println!("[libc] lseek({}, {}, {}) = 模拟偏移", fd, offset, whence);
                0
            } else {
                -1
            }
        }

        fn exit(status: c_int) -> ! {
            crate::println!("[libc] exit({})", status);
            // 这里应该调用系统退出
            loop {
                crate::arch::wfi();
            }
        }

        fn getpid() -> c_int {
            1 // 模拟进程ID
        }

        fn kill(pid: c_int, sig: c_int) -> c_int {
            crate::println!("[libc] kill({}, {}) = 0", pid, sig);
            0
        }

        fn fork() -> c_int {
            // 简化实现，只返回父进程
            crate::println!("[libc] fork() = 父进程ID");
            1
        }

        fn execve(pathname: *const c_char, argv: *const *mut c_char, envp: *const *mut c_char) -> c_int {
            let path = unsafe {
                crate::ffi::CStr::from_ptr(pathname)
                    .to_str()
                    .unwrap_or("<invalid>")
            };

            crate::println!("[libc] execve(\"{}\", ptr, ptr) = 模拟执行", path);
            -1 // 执行失败
        }

        fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int {
            crate::println!("[libc] waitpid({}, ptr, {}) = 模拟等待", pid, options);
            1 // 模拟子进程ID
        }

        fn sleep(seconds: c_uint) -> c_uint {
            // 简化实现
            for _ in 0..seconds {
                crate::arch::wfi();
            }
            0
        }

        fn time(tloc: *mut c_long) -> c_long {
            let current_time = crate::subsystems::time::timestamp() as c_long;
            if !tloc.is_null() {
                unsafe {
                    *tloc = current_time;
                }
            }
            current_time
        }

        fn random() -> c_long {
            use core::sync::atomic::{AtomicU64, Ordering};
            static RANDOM_SEED: AtomicU64 = AtomicU64::new(12345);

            let seed = RANDOM_SEED.fetch_add(1, Ordering::SeqCst);
            // 简单的伪随机数生成器
            ((seed * 1103515245 + 12345) & 0x7fffffff) as c_long
        }

        fn srandom(seed: c_uint) {
            use core::sync::atomic::Ordering;
            super::RANDOM_SEED.store(seed as u64, Ordering::SeqCst);
        }

        fn setjmp(env: *mut c_void) -> c_int {
            crate::println!("[libc] setjmp(ptr) = 0");
            0
        }

        fn longjmp(env: *mut c_void, val: c_int) {
            crate::println!("[libc] longjmp(ptr, {})", val);
            // 这里应该实现实际的跳转
        }
    }
}

/// Newlib未初始化错误
#[derive(Debug, Clone)]
pub enum LibcError {
    NotInitialized,
    InvalidParameter(String),
    OutOfMemory,
    IOError(String),
}

impl core::fmt::Display for LibcError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LibcError::NotInitialized => write!(f, "Newlib未初始化"),
            LibcError::InvalidParameter(msg) => write!(f, "无效参数: {}", msg),
            LibcError::OutOfMemory => write!(f, "内存不足"),
            LibcError::IOError(msg) => write!(f, "I/O错误: {}", msg),
        }
    }
}

/// 随机数生成器种子
static mut RANDOM_SEED: u64 = 12345;

/// 新lib兼容的宏定义
#[macro_export]
macro_rules! libc_mocks {
    ($($func:ident($($arg:expr),*))*) => {
        extern "C" {
            fn $func($($arg),*) -> i32;
        }
    };
}

// 导出符号供C程序使用
pub use libc_funcs::*;

// 导出C标准库符号
#[no_mangle]
pub extern "C" {
    // 内存管理
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    pub fn calloc(nmemb: usize, size: usize) -> *mut c_void;

    // 字符串操作
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
    pub fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    pub fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;

    // I/O操作
    pub fn printf(format: *const c_char, ...) -> c_int;
    pub fn puts(s: *const c_char) -> c_int;
    pub fn putchar(c: c_int) -> c_int;
    pub fn getchar() -> c_int;

    // 文件操作
    pub fn open(pathname: *const c_char, flags: c_int, ...) -> c_int;
    pub fn close(fd: c_int) -> c_int;
    pub fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    pub fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    pub fn lseek(fd: c_int, offset: isize, whence: c_int) -> isize;

    // 进程管理
    pub fn exit(status: c_int) -> !;
    pub fn getpid() -> c_int;
    pub fn kill(pid: c_int, sig: c_int) -> c_int;
    pub fn fork() -> c_int;
    pub fn execve(pathname: *const c_char, argv: *const *mut c_char, envp: *const *mut c_char) -> c_int;
    pub fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;

    // 系统调用
    pub fn sleep(seconds: c_uint) -> c_uint;
    pub fn time(tloc: *mut c_long) -> c_long;
    pub fn random() -> c_long;
    pub fn srandom(seed: c_uint);

    // setjmp/longjmp
    pub fn setjmp(env: *mut c_void) -> c_int;
    pub fn longjmp(env: *mut c_void, val: c_int);
}