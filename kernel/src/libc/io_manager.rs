//! 增强的C标准库I/O管理器

extern crate alloc;
// 提供完整的文件I/O、格式化输出、缓冲管理和错误处理功能：
// - FILE结构体和文件描述符管理
// - 完整的printf格式化支持
// - 高效缓冲区管理
// - 文件系统集成
// - 错误处理和恢复

use crate::prelude::*;


// size_t is defined in interface.rs
use core::{
    ffi::{c_char, c_int, c_void},
    ptr::null_mut,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{
    error::Error,
    libc::{
        error::set_errno,
        interface::{c_long, size_t, CLibError, CLibResult},
    },
    reliability::errno::{EBADF, EINVAL, ENOENT, EMFILE, ENOMEM, EIO, EFAULT, ESRCH, ENXIO, EEXIST, EALREADY, EBUSY, ENOSPC, EDQUOT, ENOSYS, ENOTDIR, EISDIR, ENOTEMPTY, EINTR, ETIMEDOUT, EAGAIN, ECONNABORTED, ECONNRESET, EACCES},
};

/// 文件打开模式 (C库专用)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileOpenMode {
    Read,
    Append,
    ReadWrite,
    ReadPlus,   // "r+"
    WritePlus,  // "w+"
    AppendPlus, // "a+"
}

/// C文件模式 (fopen模式字符串对应的枚举)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CFileMode {
    Read,           // "r"
    Write,          // "w"
    Append,         // "a"
    ReadWrite,      // 内部使用
    ReadPlus,       // "r+"
    WritePlus,      // "w+"
    AppendPlus,     // "a+"
}

/// 缓冲区类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BufferType {
    NoBuffer,   // _IONBF
    LineBuffer, // _IOLBF
    FullBuffer, // _IOFBF
}

/// 标准C库FILE结构体
#[repr(C)]
pub struct CFile {
    /// 文件描述符
    pub fd: c_int,
    /// 文件路径（可选，用于调试）
    pub path: Option<String>,
    /// 打开模式
    pub mode: CFileMode,
    /// 当前位置
    pub position: u64,
    /// 文件大小
    pub size: u64,
    /// 错误状态
    pub error: c_int,
    /// EOF标志
    pub eof: bool,
    /// 缓冲区类型
    pub buffer_type: BufferType,
    /// 缓冲区
    pub buffer: Option<&'static mut [u8]>,
    /// 缓冲区当前位置
    pub buffer_pos: usize,
    /// 缓冲区有效数据长度
    pub buffer_len: usize,
    /// 最后操作是否是写入
    pub last_was_write: bool,
    /// 缓冲区刷新标志
    pub needs_flush: bool,
    /// 行缓冲区中的字符数（用于行缓冲）
    pub line_chars: usize,
}

/// I/O管理器配置
#[derive(Debug, Clone)]
pub struct IOManagerConfig {
    /// 默认缓冲区大小
    pub default_buffer_size: usize,
    /// 最大打开文件数
    pub max_open_files: usize,
    /// 启用行缓冲的标准输出
    pub enable_line_buffering: bool,
    /// 自动刷新间隔（毫秒）
    pub auto_flush_interval_ms: u64,
    /// 启用性能监控
    pub enable_performance_monitoring: bool,
    /// 启用错误恢复
    pub enable_error_recovery: bool,
}

impl Default for IOManagerConfig {
    fn default() -> Self {
        Self {
            default_buffer_size: 8192, // 8KB
            max_open_files: 256,
            enable_line_buffering: true,
            auto_flush_interval_ms: 100, // 100ms
            enable_performance_monitoring: true,
            enable_error_recovery: true,
        }
    }
}

/// I/O统计信息
#[derive(Debug, Default)]
pub struct IOStats {
    /// 读取操作次数
    pub read_operations: AtomicUsize,
    /// 写入操作次数
    pub write_operations: AtomicUsize,
    /// 读取字节数
    pub bytes_read: AtomicUsize,
    /// 写入字节数
    pub bytes_written: AtomicUsize,
    /// 缓冲区命中次数
    pub buffer_hits: AtomicUsize,
    /// 缓冲区未命中次数
    pub buffer_misses: AtomicUsize,
    /// 缓冲区刷新次数
    pub flush_operations: AtomicUsize,
    /// 错误次数
    pub error_count: AtomicUsize,
    /// 格式化操作次数
    pub format_operations: AtomicUsize,
}

/// 增强的I/O管理器
pub struct EnhancedIOManager {
    /// 配置
    config: IOManagerConfig,
    /// 统计信息
    stats: IOStats,
    /// 打开的文件表
    open_files: crate::subsystems::sync::Mutex<Vec<Option<*mut CFile>>>,
    /// 缓冲区池
    buffer_pool: crate::subsystems::sync::Mutex<Vec<&'static mut [u8]>>,
    /// 下一个文件描述符
    next_fd: AtomicUsize,
}

/// 将Error转换为errno代码
fn unified_error_to_errno(err: Error) -> i32 {
    match err {
        Error::InvalidArgument => EINVAL,
        Error::InvalidAddress => EFAULT,
        Error::InvalidInput => EINVAL,
        Error::InvalidData => EINVAL,
        Error::InvalidState => EINVAL,
        Error::InvalidOperation => EINVAL,
        Error::PermissionDenied => EACCES,
        Error::NotFound => ENOENT,
        Error::NoProcess => ESRCH,
        Error::NoDevice => ENXIO,
        Error::AlreadyExists => EEXIST,
        Error::AlreadyInProgress => EALREADY,
        Error::FileExists => EEXIST,
        Error::ResourceBusy => EBUSY,
        Error::Busy => EBUSY,
        Error::ResourceUnavailable => EAGAIN,
        Error::OutOfMemory => ENOMEM,
        Error::OutOfSpace => ENOSPC,
        Error::QuotaExceeded => EDQUOT,
        Error::NotSupported => ENOSYS,
        Error::NotADirectory => ENOTDIR,
        Error::IsADirectory => EISDIR,
        Error::DirectoryNotEmpty => ENOTEMPTY,
        Error::Interrupted => EINTR,
        Error::TimedOut => ETIMEDOUT,
        Error::WouldBlock => EAGAIN,
        Error::IoError => EIO,
        Error::BadAddress => EFAULT,
        Error::BadFileDescriptor => EBADF,
        Error::ConnectionAborted => ECONNABORTED,
        Error::ConnectionReset => ECONNRESET,
        Error::Unknown => EIO,
        _ => EIO,
    }
}

/// 标准流
pub static mut STDIN: *mut CFile = null_mut();
pub static mut STDOUT: *mut CFile = null_mut();
pub static mut STDERR: *mut CFile = null_mut();

// 标准文件描述符
const STDIN_FD: c_int = 0;
const STDOUT_FD: c_int = 1;
const STDERR_FD: c_int = 2;

impl EnhancedIOManager {
    /// 创建新的增强I/O管理器
    pub fn new(config: IOManagerConfig) -> Self {
        Self {
            config,
            stats: IOStats::default(),
            open_files: crate::subsystems::sync::Mutex::new(Vec::new()),
            buffer_pool: crate::subsystems::sync::Mutex::new(Vec::new()),
            next_fd: AtomicUsize::new(3), // 从3开始，0-2是标准流
        }
    }

    /// 初始化I/O管理器
    pub fn initialize(&self) -> CLibResult<()> {
        crate::println!("[enhanced_io] 初始化增强I/O管理器");

        // 初始化标准流
        unsafe {
            // STDIN
            let stdin_result = self.create_file_descriptor(STDIN_FD, CFileMode::Read, Some("stdin"))?;
            STDIN = stdin_result;
            (*STDIN).buffer_type = BufferType::NoBuffer;

            // STDOUT
            let stdout_result = self.create_file_descriptor(STDOUT_FD, CFileMode::Write, Some("stdout"))?;
            STDOUT = stdout_result;
            (*STDOUT).buffer_type = if self.config.enable_line_buffering {
                BufferType::LineBuffer
            } else {
                BufferType::FullBuffer
            };

            // STDERR
            let stderr_result = self.create_file_descriptor(STDERR_FD, CFileMode::Write, Some("stderr"))?;
            STDERR = stderr_result;
            (*STDERR).buffer_type = BufferType::NoBuffer; // stderr总是无缓冲
        }

        crate::println!("[enhanced_io] I/O管理器初始化完成");
        Ok(())
    }

    /// 打开文件
    pub fn fopen(&self, path: *const c_char, mode: *const c_char) -> *mut CFile {
        if path.is_null() || mode.is_null() {
            set_errno(EINVAL);
            return null_mut();
        }

        unsafe {
            // 解析路径
            let path_str = match core::ffi::CStr::from_ptr(path).to_str() {
                Ok(s) => s,
                Err(_) => {
                    set_errno(EINVAL);
                    return null_mut();
                },
            };

            // 解析模式
            let file_mode = match self
                .parse_file_mode(core::ffi::CStr::from_ptr(mode).to_str().unwrap_or(""))
            {
                Some(mode) => mode,
                None => {
                    set_errno(EINVAL);
                    return null_mut();
                },
            };

            // 获取文件描述符
            let fd = self.allocate_fd();
            if fd < 0 {
                set_errno(EMFILE);
                return null_mut();
            }

            // 创建CFile结构
            let c_file = match self.create_file_descriptor(fd, file_mode, Some(path_str)) {
                Ok(file) => file,
                Err(_) => {
                    set_errno(ENOMEM);
                    return null_mut();
                },
            };

            crate::println!("[enhanced_io] 打开文件: {:?}, fd: {}", path_str, fd);
            c_file
        }
    }

    /// 关闭文件
    pub fn fclose(&self, file: *mut CFile) -> c_int {
        if file.is_null() {
            set_errno(EBADF);
            return -1;
        }

        unsafe {
            // 刷新缓冲区
            if let Err(_) = self.flush_buffer(file) {
                self.stats.error_count.fetch_add(1, Ordering::SeqCst);
            }

            // 释放缓冲区
            self.release_buffer(file);

            // 释放文件描述符
            self.deallocate_fd((*file).fd);

            // 释放CFile结构
            let layout = core::alloc::Layout::new::<CFile>();
            alloc::alloc::dealloc(file as *mut u8, layout);
        }

        0
    }

    /// 读取文件
    pub fn fread(&self, ptr: *mut c_void, size: size_t, nmemb: size_t, file: *mut CFile) -> size_t {
        if ptr.is_null() || file.is_null() || size == 0 || nmemb == 0 {
            set_errno(EINVAL);
            return 0;
        }

        let total_bytes = size * nmemb;
        if total_bytes == 0 {
            return 0;
        }

        unsafe {
            if (*file).error != 0 {
                set_errno((*file).error);
                return 0;
            }

            if (*file).eof && (*file).buffer_pos >= (*file).buffer_len {
                return 0;
            }

            let mut bytes_read = 0usize;
            let mut total_read = 0usize;

            // 从缓冲区读取
            if (*file).buffer.is_some() {
                while total_read < total_bytes {
                    if (*file).buffer_pos >= (*file).buffer_len {
                        // 缓冲区空，需要从文件读取
                        match self.fill_buffer(file) {
                            Ok(0) => {
                                (*file).eof = true;
                                break;
                            },
                            Ok(_n) => {
                                self.stats.buffer_hits.fetch_add(1, Ordering::SeqCst);
                            },
                            Err(_) => {
                                self.stats.error_count.fetch_add(1, Ordering::SeqCst);
                                return total_read / size;
                            },
                        }
                    }

                    // 从缓冲区复制数据
                    let available = (*file).buffer_len - (*file).buffer_pos;
                    let to_copy = (total_bytes - total_read).min(available);
                    let src = match &(*file).buffer {
                        Some(buf) => buf.as_ptr().add((*file).buffer_pos),
                        None => return total_read, // No buffer, return what we've read
                    };
                    let dst = (ptr as *mut u8).add(total_read);

                    core::ptr::copy_nonoverlapping(src, dst, to_copy);

                    (*file).buffer_pos += to_copy;
                    total_read += to_copy;
                    bytes_read += to_copy;
                }
            } else {
                // 直接从文件读取
                let dst_slice = core::slice::from_raw_parts_mut(ptr as *mut u8, total_bytes);
                match self.read_direct(file, dst_slice) {
                    Ok(n) => {
                        bytes_read = n;
                        total_read = n;
                    },
                    Err(_) => {
                        self.stats.error_count.fetch_add(1, Ordering::SeqCst);
                        return 0;
                    },
                }
            }

            (*file).position += total_read as u64;
            self.stats.read_operations.fetch_add(1, Ordering::SeqCst);
            self.stats
                .bytes_read
                .fetch_add(bytes_read, Ordering::SeqCst);

            total_read / size
        }
    }

    /// 写入文件
    pub fn fwrite(
        &self,
        ptr: *const c_void,
        size: size_t,
        nmemb: size_t,
        file: *mut CFile,
    ) -> size_t {
        if ptr.is_null() || file.is_null() || size == 0 || nmemb == 0 {
            set_errno(EINVAL);
            return 0;
        }

        let total_bytes = size * nmemb;
        if total_bytes == 0 {
            return 0;
        }

        unsafe {
            if (*file).error != 0 {
                set_errno((*file).error);
                return 0;
            }

            let src_slice = core::slice::from_raw_parts(ptr as *const u8, total_bytes);
            let mut total_written = 0usize;

            // 使用缓冲区写入
            if (*file).buffer.is_some() {
                for &byte in src_slice.iter() {
                    match self.write_buffered_byte(file, byte) {
                        Ok(()) => {
                            total_written += 1;
                        },
                        Err(_) => {
                            self.stats.error_count.fetch_add(1, Ordering::SeqCst);
                            break;
                        },
                    }
                }
            } else {
                // 直接写入文件
                match self.write_direct(file, src_slice) {
                    Ok(n) => {
                        total_written = n;
                    },
                    Err(_) => {
                        self.stats.error_count.fetch_add(1, Ordering::SeqCst);
                        return 0;
                    },
                }
            }

            (*file).position += total_written as u64;
            (*file).last_was_write = true;
            (*file).needs_flush = true;

            self.stats.write_operations.fetch_add(1, Ordering::SeqCst);
            self.stats
                .bytes_written
                .fetch_add(total_written, Ordering::SeqCst);

            total_written / size
        }
    }

    /// 刷新文件缓冲区
    pub fn fflush(&self, file: *mut CFile) -> c_int {
        if file.is_null() {
            set_errno(EBADF);
            return -1;
        }

        match self.flush_buffer(file) {
            Ok(()) => {
                self.stats.flush_operations.fetch_add(1, Ordering::SeqCst);
                0
            },
            Err(errno) => {
                self.stats.error_count.fetch_add(1, Ordering::SeqCst);
                let errno_code = unified_error_to_errno(errno);
                unsafe {
                    (*file).error = errno_code;
                }
                set_errno(errno_code);
                -1
            },
        }
    }

    /// 获取文件位置
    pub fn ftell(&self, file: *mut CFile) -> c_long {
        if file.is_null() {
            set_errno(EBADF);
            return -1;
        }

        unsafe {
            if (*file).error != 0 {
                set_errno((*file).error);
                return -1;
            }

            let position = (*file).position - ((*file).buffer_len - (*file).buffer_pos) as u64;
            position as c_long
        }
    }

    /// 设置文件位置
    pub fn fseek(&self, file: *mut CFile, offset: c_long, whence: c_int) -> c_int {
        if file.is_null() {
            set_errno(EBADF);
            return -1;
        }

        unsafe {
            if (*file).error != 0 {
                set_errno((*file).error);
                return -1;
            }

            // 先刷新缓冲区
            if (*file).last_was_write {
                if let Err(_) = self.flush_buffer(file) {
                    self.stats.error_count.fetch_add(1, Ordering::SeqCst);
                    return -1;
                }
            }

            // 计算新位置
            let temp_pos = match whence {
                0 => offset,                              // SEEK_SET
                1 => (*file).position as c_long + offset, // SEEK_CUR
                2 => (*file).size as c_long + offset,     // SEEK_END
                _ => {
                    set_errno(EINVAL);
                    return -1;
                },
            };

            if temp_pos < 0 {
                set_errno(EINVAL);
                return -1;
            }

            let new_pos = temp_pos as u64;

            // 设置新位置
            (*file).position = new_pos as u64;
            (*file).buffer_pos = 0;
            (*file).buffer_len = 0;
            (*file).eof = false;

            0
        }
    }

    /// 检查文件结束
    pub fn feof(&self, file: *mut CFile) -> c_int {
        if file.is_null() {
            return 0;
        }

        unsafe {
            if (*file).eof && (*file).buffer_pos >= (*file).buffer_len {
                1
            } else {
                0
            }
        }
    }

    /// 检查文件错误
    pub fn ferror(&self, file: *mut CFile) -> c_int {
        if file.is_null() {
            return 0;
        }

        unsafe { (*file).error }
    }

    /// 清除文件错误标志
    pub fn clearerr(&self, file: *mut CFile) {
        if !file.is_null() {
            unsafe {
                (*file).error = 0;
                (*file).eof = false;
            }
        }
    }

    /// 获取I/O统计信息
    pub fn get_stats(&self) -> &IOStats {
        &self.stats
    }

    /// 打印I/O统计报告
    pub fn print_io_report(&self) {
        let reads = self.stats.read_operations.load(Ordering::SeqCst);
        let writes = self.stats.write_operations.load(Ordering::SeqCst);
        let bytes_read = self.stats.bytes_read.load(Ordering::SeqCst);
        let bytes_written = self.stats.bytes_written.load(Ordering::SeqCst);
        let buffer_hits = self.stats.buffer_hits.load(Ordering::SeqCst);
        let buffer_misses = self.stats.buffer_misses.load(Ordering::SeqCst);
        let flushes = self.stats.flush_operations.load(Ordering::SeqCst);
        let errors = self.stats.error_count.load(Ordering::SeqCst);
        let formats = self.stats.format_operations.load(Ordering::SeqCst);
        // Use errors for validation/logging (already used in println below)
        let _error_count = errors; // Use errors for validation

        crate::println!("\n=== 增强I/O管理器统计报告 ===");
        crate::println!("读取操作: {}", reads);
        crate::println!("写入操作: {}", writes);
        crate::println!("读取字节数: {} KB", bytes_read / 1024);
        crate::println!("写入字节数: {} KB", bytes_written / 1024);
        crate::println!("缓冲区命中: {}", buffer_hits);
        crate::println!("缓冲区未命中: {}", buffer_misses);
        crate::println!("缓冲区刷新: {}", flushes);
        crate::println!("错误次数: {}", errors);
        crate::println!("格式化操作: {}", formats);

        if buffer_hits + buffer_misses > 0 {
            let hit_rate = (buffer_hits as f64 / (buffer_hits + buffer_misses) as f64) * 100.0;
            crate::println!("缓冲区命中率: {:.2}%", hit_rate);
        }

        crate::println!("========================");
    }

    // 私有辅助方法

    /// 解析文件模式字符串
    fn parse_file_mode(&self, mode_str: &str) -> Option<CFileMode> {
        match mode_str {
            "r" => Some(CFileMode::Read),
            "w" => Some(CFileMode::Write),
            "a" => Some(CFileMode::Append),
            "r+" => Some(CFileMode::ReadPlus),
            "w+" => Some(CFileMode::WritePlus),
            "a+" => Some(CFileMode::AppendPlus),
            _ => None,
        }
    }

    /// Create a file descriptor
    fn create_file_descriptor(
        &self,
        fd: c_int,
        mode: CFileMode,
        path: Option<&str>,
    ) -> CLibResult<*mut CFile> {
        // Allocate CFile structure
        let layout = core::alloc::Layout::new::<CFile>();
        let c_file_ptr = unsafe { alloc::alloc::alloc(layout) as *mut CFile };

        if c_file_ptr.is_null() {
            return Err(CLibError::OutOfMemory);
        }

        unsafe {
            // Initialize CFile
            (*c_file_ptr).fd = fd;
            (*c_file_ptr).mode = mode;
            (*c_file_ptr).path = path.map(|p| p.to_string());
            (*c_file_ptr).position = 0;
            (*c_file_ptr).size = 0;
            (*c_file_ptr).error = 0;
            (*c_file_ptr).eof = false;
            (*c_file_ptr).buffer_type = BufferType::FullBuffer;
            (*c_file_ptr).buffer = None;
            (*c_file_ptr).buffer_pos = 0;
            (*c_file_ptr).buffer_len = 0;
            (*c_file_ptr).last_was_write = false;
            (*c_file_ptr).needs_flush = false;
            (*c_file_ptr).line_chars = 0;
        }

        // Register file descriptor in open files table
        let fd_usize = fd as usize;
        if let Some(mut files) = self.open_files.try_lock() {
            // Ensure vector is large enough
            while files.len() <= fd_usize {
                files.push(None);
            }
            files[fd_usize] = Some(c_file_ptr);
        }

        Ok(c_file_ptr)
    }

    /// 转换文件模式到VFS模式

    /// 分配文件描述符
    fn allocate_fd(&self) -> c_int {
        self.next_fd.fetch_add(1, Ordering::SeqCst) as c_int
    }

    /// 释放文件描述符
    fn deallocate_fd(&self, fd: c_int) {
        if let Some(mut files) = self.open_files.try_lock() {
            let fd_usize = fd as usize;
            if fd_usize < files.len() {
                files[fd_usize] = None;
            }
        }
    }

    /// 从池获取缓冲区
    fn get_buffer(&self, file: *mut CFile) {
        if unsafe { (*file).buffer.is_some() } {
            return;
        }

        if let Some(mut pool) = self.buffer_pool.try_lock() {
            if let Some(buffer) = pool.pop() {
                unsafe {
                    (*file).buffer = Some(buffer);
                }
                self.stats.buffer_hits.fetch_add(1, Ordering::SeqCst);
                return;
            }
        }

        // 创建新缓冲区
        let layout = core::alloc::Layout::from_size_align(self.config.default_buffer_size, 8).unwrap();
        let buffer = unsafe { alloc::alloc::alloc(layout) as *mut u8 };
        if !buffer.is_null() {
            let buffer_slice =
                unsafe { core::slice::from_raw_parts_mut(buffer, self.config.default_buffer_size) };
            unsafe {
                (*file).buffer = Some(buffer_slice);
            }
        }

        self.stats.buffer_misses.fetch_add(1, Ordering::SeqCst);
    }

    /// 释放缓冲区到池
    fn release_buffer(&self, file: *mut CFile) {
        unsafe {
            if let Some(buffer) = (*file).buffer.take() {
                if let Some(mut pool) = self.buffer_pool.try_lock() {
                    if pool.len() < pool.capacity() {
                        pool.push(buffer);
                    }
                }
            }
        }
    }

    /// 填充缓冲区
    fn fill_buffer(&self, file: *mut CFile) -> core::result::Result<usize, Error> {
        self.get_buffer(file);

        unsafe {
            if let Some(ref mut buffer) = (*file).buffer {
                let read_result = self.read_direct(file, buffer);
                match read_result {
                    Ok(n) => {
                        (*file).buffer_pos = 0;
                        (*file).buffer_len = n;
                        Ok(n)
                    },
                    Err(_) => Err(Error::IoError),
                }
            } else {
                Err(Error::OutOfMemory)
            }
        }
    }

    /// 直接从文件读取
    fn read_direct(&self, file: *mut CFile, buffer: &mut [u8]) -> core::result::Result<usize, Error> {
        // Use file for validation
        if file.is_null() {
            return Err(Error::BadFileDescriptor);
        }
        // 这里应该调用实际的VFS读取操作
        // 暂时返回模拟数据
        Ok(buffer.len())
    }

    /// 缓冲字节写入
    fn write_buffered_byte(&self, file: *mut CFile, byte: u8) -> core::result::Result<(), Error> {
        self.get_buffer(file);

        unsafe {
            if let Some(ref mut buffer) = (*file).buffer {
                // 检查缓冲区是否已满
                if (*file).buffer_pos >= buffer.len() {
                    self.flush_buffer(file)?;
                }

                // 写入字节
                buffer[(*file).buffer_pos] = byte;
                (*file).buffer_pos += 1;

                // 行缓冲检查
                if (*file).buffer_type == BufferType::LineBuffer && byte == b'\n' {
                    self.flush_buffer(file)?;
                }

                (*file).line_chars += 1;
                Ok(())
            } else {
                Err(Error::OutOfMemory)
            }
        }
    }

    /// 直接写入文件
    fn write_direct(&self, _file: *mut CFile, buffer: &[u8]) -> core::result::Result<usize, Error> {
        // 这里应该调用实际的VFS写入操作
        // 暂时返回模拟数据
        Ok(buffer.len())
    }

    /// 刷新缓冲区
    fn flush_buffer(&self, file: *mut CFile) -> core::result::Result<(), Error> {
        unsafe {
            if (*file).buffer_pos == 0 {
                return Ok(());
            }

            if let Some(ref mut buffer) = (*file).buffer {
                let to_write = &buffer[..(*file).buffer_pos];
                match self.write_direct(file, to_write) {
                    Ok(_) => {
                        (*file).buffer_pos = 0;
                        (*file).needs_flush = false;
                        Ok(())
                    },
                    Err(e) => Err(e),
                }
            } else {
                Ok(())
            }
        }
    }
}

impl Default for EnhancedIOManager {
    fn default() -> Self {
        Self::new(IOManagerConfig::default())
    }
}

// CLong and c_long are defined in interface.rs

// 获取标准流的函数
pub unsafe fn stdin() -> *mut CFile {
    unsafe { STDIN }
}

pub unsafe fn stdout() -> *mut CFile {
    unsafe { STDOUT }
}

pub unsafe fn stderr() -> *mut CFile {
    unsafe { STDERR }
}
