// Memory module error definitions
// These error types follow the MM_MODULE_API_BOUNDARIES_DESIGN.md

/// Memory allocation error
/// 
/// Memory allocation process中可能出现的错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocError {
    /// 内存不足
    OutOfMemory,
    /// 无效对齐
    InvalidAlignment,
    /// 无效大小
    InvalidSize,
    /// 分配器损坏
    CorruptedAllocator,
    /// 内存碎片过多
    TooFragmented,
}

/// Virtual memory error
/// 
/// Virtual memory操作过程中可能出现的错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    /// 无效地址
    InvalidAddress,
    /// 无效大小
    InvalidSize,
    /// 无效保护
    InvalidProtection,
    /// 映射不存在
    MappingNotFound,
    /// 权限被拒绝
    PermissionDenied,
    /// 地址已映射
    AddressAlreadyMapped,
    /// 页表错误
    PageTableError,
    /// TLB错误
    TLBError,
}

/// Physical memory error
/// 
/// Physical memory操作过程中可能出现的错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhysicalError {
    /// 内存不足
    OutOfMemory,
    /// 无效页面
    InvalidPage,
    /// 页面已分配
    PageAlreadyAllocated,
    /// 无效范围
    InvalidRange,
    /// 物理内存损坏
    CorruptedMemory,
}