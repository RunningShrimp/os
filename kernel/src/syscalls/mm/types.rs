//! 内存管理模块类型定义
//! 
//! 本模块定义了内存管理相关的类型、枚举和结构体，包括：
//! - 内存区域和映射类型
//! - 内存保护标志
//! - 内存分配参数
//! - 虚拟内存管理类型

use alloc::string::String;
use alloc::vec::Vec;

/// 内存保护标志
/// 
/// 定义内存区域的访问权限。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryProtection {
    /// 可读
    pub readable: bool,
    /// 可写
    pub writable: bool,
    /// 可执行
    pub executable: bool,
}

impl MemoryProtection {
    /// 创建新的内存保护标志
    pub fn new() -> Self {
        Self {
            readable: true,
            writable: false,
            executable: false,
        }
    }

    /// 创建读写内存保护
    pub fn read_write() -> Self {
        Self {
            readable: true,
            writable: true,
            executable: false,
        }
    }

    /// 创建可执行内存保护
    pub fn read_execute() -> Self {
        Self {
            readable: true,
            writable: false,
            executable: true,
        }
    }

    /// 转换为页表标志
    pub fn to_page_flags(&self) -> u64 {
        let mut flags = 0u64;
        if self.readable { flags |= 0x1; }   // Present
        if self.writable { flags |= 0x2; }   // Read/Write
        if self.executable { flags |= 0x4; } // User/Supervisor
        flags
    }
}

impl Default for MemoryProtection {
    fn default() -> Self {
        Self::new()
    }
}

/// 内存映射标志
/// 
/// 定义内存映射的选项和属性。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryMapFlags {
    /// 共享映射
    Shared,
    /// 私有映射
    Private,
    /// 固定映射（不可换出）
    Fixed,
    /// 匿名映射
    Anonymous,
    /// 映射文件
    MapFile,
    /// 要求文件存在
    DenyWrite,
    /// 可执行
    Executable,
}

/// 内存区域类型
/// 
/// 定义内存区域的用途和类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// 代码段
    Code,
    /// 数据段
    Data,
    /// 堆
    Heap,
    /// 栈
    Stack,
    /// 内存映射文件
    MappedFile,
    /// 共享内存
    SharedMemory,
    /// 设备内存
    DeviceMemory,
    /// 保留区域
    Reserved,
}

/// 内存区域信息
/// 
/// 描述一个内存区域的属性。
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// 区域起始地址
    pub start_address: u64,
    /// 区域结束地址
    pub end_address: u64,
    /// 区域大小
    pub size: u64,
    /// 区域类型
    pub region_type: MemoryRegionType,
    /// 内存保护
    pub protection: MemoryProtection,
    /// 映射标志
    pub flags: Vec<MemoryMapFlags>,
    /// 文件偏移（如果映射文件）
    pub file_offset: u64,
    /// 文件描述符（如果映射文件）
    pub fd: Option<i32>,
    /// 区域名称
    pub name: String,
}

/// 内存映射参数
/// 
/// 包含创建内存映射所需的参数。
#[derive(Debug, Clone)]
pub struct MemoryMapParams {
    /// 映射地址（建议）
    pub address: u64,
    /// 映射大小
    pub size: u64,
    /// 内存保护
    pub protection: MemoryProtection,
    /// 映射标志
    pub flags: Vec<MemoryMapFlags>,
    /// 文件描述符
    pub fd: Option<i32>,
    /// 文件偏移
    pub offset: u64,
}

impl Default for MemoryMapParams {
    fn default() -> Self {
        Self {
            address: 0,
            size: 0,
            protection: MemoryProtection::default(),
            flags: Vec::new(),
            fd: None,
            offset: 0,
        }
    }
}

/// 内存统计信息
/// 
/// 包含内存使用的统计信息。
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// 总内存大小
    pub total_memory: u64,
    /// 可用内存大小
    pub free_memory: u64,
    /// 已用内存大小
    pub used_memory: u64,
    /// 缓存内存大小
    pub cached_memory: u64,
    /// 交换空间大小
    pub swap_total: u64,
    /// 已用交换空间
    pub swap_used: u64,
    /// 内存区域数量
    pub regions_count: u32,
    /// 最大连续内存块
    pub largest_free_block: u64,
}

/// 内存分配策略
/// 
/// 定义内存分配的策略和算法。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationStrategy {
    /// 首次适应
    FirstFit,
    /// 最佳适应
    BestFit,
    /// 最差适应
    WorstFit,
    /// 下次适应
    NextFit,
    /// 快速适应
    QuickFit,
}

/// 页面大小类型
/// 
/// 定义系统支持的页面大小。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSize {
    /// 4KB页面
    Size4K = 4096,
    /// 2MB页面
    Size2M = 2097152,
    /// 1GB页面
    Size1G = 1073741824,
}

/// 内存对齐要求
/// 
/// 定义内存对齐的约束。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryAlignment {
    /// 对齐边界
    pub boundary: u64,
    /// 对齐偏移
    pub offset: u64,
}

impl MemoryAlignment {
    /// 创建新的对齐要求
    pub fn new(boundary: u64, offset: u64) -> Self {
        Self { boundary, offset }
    }

    /// 创建页面对齐
    pub fn page_aligned() -> Self {
        Self::new(PageSize::Size4K as u64, 0)
    }

    /// 检查地址是否对齐
    pub fn is_aligned(&self, address: u64) -> bool {
        (address - self.offset) % self.boundary == 0
    }

    /// 对齐地址到边界
    pub fn align_up(&self, address: u64) -> u64 {
        let aligned = (address + self.boundary - 1) & !(self.boundary - 1);
        aligned + (self.offset % self.boundary)
    }

    /// 对齐地址到边界（向下）
    pub fn align_down(&self, address: u64) -> u64 {
        let aligned = address & !(self.boundary - 1);
        aligned + (self.offset % self.boundary)
    }
}

/// 内存管理错误类型
/// 
/// 定义内存管理模块特有的错误类型。
#[derive(Debug, Clone)]
pub enum MemoryError {
    /// 内存不足
    OutOfMemory,
    /// 无效地址
    InvalidAddress,
    /// 权限不足
    PermissionDenied,
    /// 无效参数
    InvalidArgument,
    /// 地址已在使用
    AddressInUse,
    /// 对齐错误
    AlignmentError,
    /// 页面错误
    PageFault,
    /// 保护错误
    ProtectionFault,
    /// 系统调用不支持
    UnsupportedSyscall,
}

impl MemoryError {
    /// 获取错误码
    pub fn error_code(&self) -> i32 {
        match self {
            MemoryError::OutOfMemory => -12,
            MemoryError::InvalidAddress => -14,
            MemoryError::PermissionDenied => -13,
            MemoryError::InvalidArgument => -22,
            MemoryError::AddressInUse => -16,
            MemoryError::AlignmentError => -22,
            MemoryError::PageFault => -14,
            MemoryError::ProtectionFault => -13,
            MemoryError::UnsupportedSyscall => -38,
        }
    }

    /// 获取错误描述
    pub fn error_message(&self) -> &str {
        match self {
            MemoryError::OutOfMemory => "Out of memory",
            MemoryError::InvalidAddress => "Invalid memory address",
            MemoryError::PermissionDenied => "Permission denied",
            MemoryError::InvalidArgument => "Invalid argument",
            MemoryError::AddressInUse => "Address already in use",
            MemoryError::AlignmentError => "Memory alignment error",
            MemoryError::PageFault => "Page fault",
            MemoryError::ProtectionFault => "Memory protection fault",
            MemoryError::UnsupportedSyscall => "Unsupported syscall",
        }
    }
}

/// 虚拟内存管理接口特征
/// 
/// 定义虚拟内存管理的基本操作接口。
pub trait VirtualMemoryManager: Send + Sync {
    /// 映射内存区域
    fn mmap(&mut self, params: MemoryMapParams) -> Result<u64, MemoryError>;
    
    /// 取消映射内存区域
    fn munmap(&mut self, address: u64, size: u64) -> Result<(), MemoryError>;
    
    /// 更改内存保护
    fn mprotect(&mut self, address: u64, size: u64, protection: MemoryProtection) -> Result<(), MemoryError>;
    
    /// 同步内存到文件
    fn msync(&mut self, address: u64, size: u64, flags: i32) -> Result<(), MemoryError>;
    
    /// 锁定内存
    fn mlock(&mut self, address: u64, size: u64) -> Result<(), MemoryError>;
    
    /// 解锁内存
    fn munlock(&mut self, address: u64, size: u64) -> Result<(), MemoryError>;
    
    /// 分配内存
    fn allocate(&mut self, size: u64, alignment: MemoryAlignment, strategy: AllocationStrategy) -> Result<u64, MemoryError>;
    
    /// 释放内存
    fn deallocate(&mut self, address: u64, size: u64) -> Result<(), MemoryError>;
    
    /// 获取内存统计
    fn get_stats(&self) -> MemoryStats;
    
    /// 列出内存区域
    fn list_regions(&self) -> Vec<MemoryRegion>;
    
    /// 查找内存区域
    fn find_region(&self, address: u64) -> Option<MemoryRegion>;
}