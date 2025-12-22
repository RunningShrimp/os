//! C标准库统一接口定义
//!
//! 定义了所有C标准库实现的统一接口，确保不同实现版本之间的一致性。
//! 这个接口为内存管理、字符串操作、I/O操作等核心C库功能提供了标准化的API。

use core::ffi::{c_char, c_int, c_void, c_uint};

pub type SizeT = usize;
#[allow(non_camel_case_types)]
pub type size_t = SizeT;
/// Signed size type
pub type SsizeT = isize;
#[allow(non_camel_case_types)]
pub type ssize_t = SsizeT;

/// 长整型类型
pub type CLong = isize;
#[allow(non_camel_case_types)]
pub type c_long = CLong;
/// 无符号长整型类型
pub type CUlong = usize;
#[allow(non_camel_case_types)]
pub type c_ulong = CUlong;
/// 短整型类型
pub type CShort = i16;
#[allow(non_camel_case_types)]
pub type c_short = CShort;
/// 无符号短整型类型
pub type CUshort = u16;
#[allow(non_camel_case_types)]
pub type c_ushort = CUshort;
/// time_t类型
pub type TimeT = i64;
#[allow(non_camel_case_types)]
pub type time_t = TimeT;
/// suseconds_t类型（微秒）
pub type SusecondsT = i64;
#[allow(non_camel_case_types)]
pub type suseconds_t = SusecondsT;
/// time_t类型（用于sysinfo）
pub type CLonglong = i64;
#[allow(non_camel_case_types)]
pub type c_longlong = CLonglong;

/// div_t结构体（整数除法结果）
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DivT {
    pub quot: c_int,
    pub rem: c_int,
}

/// ldiv_t结构体（长整数除法结果）
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LDivT {
    pub quot: CLong,
    pub rem: CLong,
}

/// C标准库核心接口
///
/// 这个trait定义了所有C标准库实现必须遵循的统一接口。
/// 不同的实现版本（Minimal、Simple、Full）都需要实现这个trait。
pub trait CLibInterface {
    // === 内存管理函数 ===

    /// 分配指定大小的内存
    ///
    /// # 参数
    /// * `size` - 要分配的内存字节数
    ///
    /// # 返回值
    /// * 成功时返回指向分配内存的指针
    /// * 失败时返回空指针，并设置errno为ENOMEM
    fn malloc(&self, size: size_t) -> *mut c_void;

    /// 释放之前分配的内存
    ///
    /// # 参数
    /// * `ptr` - 要释放的内存指针，如果是空指针则什么都不做
    fn free(&self, ptr: *mut c_void);

    /// 重新分配内存大小
    ///
    /// # 参数
    /// * `ptr` - 原始内存指针，如果是空指针则等同于malloc
    /// * `size` - 新的内存大小
    ///
    /// # 返回值
    /// * 成功时返回指向新内存的指针
    /// * 失败时返回空指针，原始内存保持不变
    fn realloc(&self, ptr: *mut c_void, size: size_t) -> *mut c_void;

    /// 分配并清零内存
    ///
    /// # 参数
    /// * `nmemb` - 元素数量
    /// * `size` - 每个元素的大小
    ///
    /// # 返回值
    /// * 成功时返回指向分配内存的指针，内存已清零
    /// * 失败时返回空指针
    fn calloc(&self, nmemb: size_t, size: size_t) -> *mut c_void;

    // === 字符串操作函数 ===

    /// 计算字符串长度（不包括终止空字符）
    ///
    /// # 参数
    /// * `s` - 以null结尾的C字符串指针
    ///
    /// # 返回值
    /// * 字符串长度，如果s为空指针则返回0
    fn strlen(&self, s: *const c_char) -> size_t;

    /// 复制字符串
    ///
    /// # 参数
    /// * `dest` - 目标缓冲区
    /// * `src` - 源字符串
    ///
    /// # 返回值
    /// * 返回目标缓冲区指针
    /// # 安全性
    /// * 调用者必须确保目标缓冲区足够大
    fn strcpy(&self, dest: *mut c_char, src: *const c_char) -> *mut c_char;

    /// 复制字符串（限制长度）
    ///
    /// # 参数
    /// * `dest` - 目标缓冲区
    /// * `src` - 源字符串
    /// * `n` - 最大复制字符数
    ///
    /// # 返回值
    /// * 返回目标缓冲区指针
    fn strncpy(&self, dest: *mut c_char, src: *const c_char, n: size_t) -> *mut c_char;

    /// 连接字符串
    ///
    /// # 参数
    /// * `dest` - 目标字符串，必须有足够空间
    /// * `src` - 要连接的源字符串
    ///
    /// # 返回值
    /// * 返回目标字符串指针
    fn strcat(&self, dest: *mut c_char, src: *const c_char) -> *mut c_char;

    /// 比较字符串
    ///
    /// # 参数
    /// * `s1` - 第一个字符串
    /// * `s2` - 第二个字符串
    ///
    /// # 返回值
    /// * <0: s1 < s2
    /// * =0: s1 == s2
    /// * >0: s1 > s2
    fn strcmp(&self, s1: *const c_char, s2: *const c_char) -> c_int;

    /// 比较字符串（限制长度）
    ///
    /// # 参数
    /// * `s1` - 第一个字符串
    /// * `s2` - 第二个字符串
    /// * `n` - 最大比较字符数
    ///
    /// # 返回值
    /// * <0: s1 < s2
    /// * =0: s1 == s2
    /// * >0: s1 > s2
    fn strncmp(&self, s1: *const c_char, s2: *const c_char, n: size_t) -> c_int;

    // === 内存操作函数 ===

    /// 设置内存
    ///
    /// # 参数
    /// * `s` - 内存区域指针
    /// * `c` - 要设置的字符
    /// * `n` - 要设置的字节数
    ///
    /// # 返回值
    /// * 返回内存区域指针
    fn memset(&self, s: *mut c_void, c: c_int, n: size_t) -> *mut c_void;

    /// 复制内存
    ///
    /// # 参数
    /// * `dest` - 目标内存区域
    /// * `src` - 源内存区域
    /// * `n` - 要复制的字节数
    ///
    /// # 返回值
    /// * 返回目标内存区域指针
    fn memcpy(&self, dest: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;

    /// 移动内存（支持重叠区域）
    ///
    /// # 参数
    /// * `dest` - 目标内存区域
    /// * `src` - 源内存区域
    /// * `n` - 要移动的字节数
    ///
    /// # 返回值
    /// * 返回目标内存区域指针
    fn memmove(&self, dest: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;

    // === I/O操作函数 ===


    /// 输出字符串并换行
    ///
    /// # 参数
    /// * `s` - 要输出的字符串
    ///
    /// # 返回值
    /// * 成功时返回非负值（包括换行符）
    /// * 失败时返回EOF
    fn puts(&self, s: *const c_char) -> c_int;

    /// 输出单个字符
    ///
    /// # 参数
    /// * `c` - 要输出的字符
    ///
    /// # 返回值
    /// * 成功时返回输出的字符
    /// * 失败时返回EOF
    fn putchar(&self, c: c_int) -> c_int;

    // === 文件I/O操作 ===

    /// 打开文件
    ///
    /// # 参数
    /// * `path` - 文件路径
    /// * `mode` - 打开模式 ("r", "w", "a", "r+", "w+", "a+")
    ///
    /// # 返回值
    /// * 成功时返回FILE指针
    /// * 失败时返回NULL并设置errno
    fn fopen(&self, path: *const c_char, mode: *const c_char) -> *mut c_void;

    /// 关闭文件
    ///
    /// # 参数
    /// * `file` - 文件指针
    ///
    /// # 返回值
    /// * 成功时返回0
    /// * 失败时返回EOF
    fn fclose(&self, file: *mut c_void) -> c_int;

    /// 从文件读取数据
    ///
    /// # 参数
    /// * `ptr` - 数据缓冲区
    /// * `size` - 每个元素的大小
    /// * `nmemb` - 元素数量
    /// * `file` - 文件指针
    ///
    /// # 返回值
    /// * 成功时返回读取的元素数量
    /// * 失败或到达文件末尾时返回小于nmemb的值
    fn fread(&self, ptr: *mut c_void, size: size_t, nmemb: size_t, file: *mut c_void) -> size_t;

    /// 向文件写入数据
    ///
    /// # 参数
    /// * `ptr` - 数据缓冲区
    /// * `size` - 每个元素的大小
    /// * `nmemb` - 元素数量
    /// * `file` - 文件指针
    ///
    /// # 返回值
    /// * 成功时返回写入的元素数量
    /// * 失败时返回小于nmemb的值
    fn fwrite(&self, ptr: *const c_void, size: size_t, nmemb: size_t, file: *mut c_void) -> size_t;

    /// 刷新文件缓冲区
    ///
    /// # 参数
    /// * `file` - 文件指针
    ///
    /// # 返回值
    /// * 成功时返回0
    /// * 失败时返回EOF
    fn fflush(&self, file: *mut c_void) -> c_int;

    /// 获取文件当前位置
    ///
    /// # 参数
    /// * `file` - 文件指针
    ///
    /// # 返回值
    /// * 成功时返回当前位置的偏移量
    /// * 失败时返回-1L并设置errno
    fn ftell(&self, file: *mut c_void) -> c_long;

    /// 设置文件位置
    ///
    /// # 参数
    /// * `file` - 文件指针
    /// * `offset` - 偏移量
    /// * `whence` - 起始位置 (SEEK_SET=0, SEEK_CUR=1, SEEK_END=2)
    ///
    /// # 返回值
    /// * 成功时返回0
    /// * 失败时返回-1并设置errno
    fn fseek(&self, file: *mut c_void, offset: c_long, whence: c_int) -> c_int;

    /// 检查文件结束
    ///
    /// # 参数
    /// * `file` - 文件指针
    ///
    /// # 返回值
    /// * 如果到达文件末尾返回非零值
    /// * 否则返回0
    fn feof(&self, file: *mut c_void) -> c_int;

    /// 检查文件错误
    ///
    /// # 参数
    /// * `file` - 文件指针
    ///
    /// # 返回值
    /// * 如果发生错误返回非零值
    /// * 否则返回0
    fn ferror(&self, file: *mut c_void) -> c_int;

    /// 清除文件错误标志
    ///
    /// # 参数
    /// * `file` - 文件指针
    fn clearerr(&self, file: *mut c_void);

    // === 增强格式化函数 ===



    /// 输入单个字符
    ///
    /// # 返回值
    /// * 成功时返回读取的字符
    /// * 失败或到达文件末尾时返回EOF
    fn getchar(&self) -> c_int;

    /// 读取单个字符
    ///
    /// # 返回值
    /// * 成功时返回读取的字符（无符号扩展）
    /// * 失败或文件结束时返回EOF
    // 重复定义移除，统一使用带接收者的方法

    // === 系统调用函数 ===

    /// 退出当前进程
    ///
    /// # 参数
    /// * `status` - 退出状态码
    fn exit(&self, status: c_int) -> !;

    /// 获取当前进程ID
    ///
    /// # 返回值
    /// * 当前进程的ID
    fn getpid(&self) -> c_int;

    /// 睡眠指定秒数
    ///
    /// # 参数
    /// * `seconds` - 要睡眠的秒数
    ///
    /// # 返回值
    /// * 实际睡眠的秒数，0表示成功
    fn sleep(&self, seconds: c_uint) -> c_uint;

    /// 获取当前时间
    ///
    /// # 参数
    /// * `timer` - 如果不为空，存储当前时间戳
    ///
    /// # 返回值
    /// * 自1970-01-01 00:00:00 UTC以来的秒数
    fn time(&self, timer: *mut c_void) -> c_int;

    // === 扩展字符串函数 ===

    /// 查找字符在字符串中的首次出现
    ///
    /// # 参数
    /// * `s` - 要搜索的字符串
    /// * `c` - 要查找的字符
    ///
    /// # 返回值
    /// * 找到时返回指向该字符的指针
    /// * 未找到时返回NULL
    fn strchr(&self, s: *const c_char, c: c_int) -> *mut c_char;

    /// 查找子字符串
    ///
    /// # 参数
    /// * `haystack` - 要搜索的字符串
    /// * `needle` - 要查找的子字符串
    ///
    /// # 返回值
    /// * 找到时返回指向子字符串首次出现的指针
    /// * 未找到时返回NULL
    fn strstr(&self, haystack: *const c_char, needle: *const c_char) -> *mut c_char;

    /// 复制字符串（动态分配）
    ///
    /// # 参数
    /// * `s` - 要复制的字符串
    ///
    /// # 返回值
    /// * 成功时返回指向新分配字符串的指针
    /// * 失败时返回NULL
    fn strdup(&self, s: *const c_char) -> *mut c_char;

    // === 数学函数 ===

    /// 计算绝对值（整数）
    ///
    /// # 参数
    /// * `x` - 整数值
    ///
    /// # 返回值
    /// * 返回x的绝对值
    fn abs(&self, x: c_int) -> c_int;

    /// 计算绝对值（长整数）
    ///
    /// # 参数
    /// * `x` - 长整数值
    ///
    /// # 返回值
    /// * 返回x的绝对值
    fn labs(&self, x: c_long) -> c_long;

    // === 字符串转换函数 ===

    /// 将字符串转换为长整数
    ///
    /// # 参数
    /// * `nptr` - 要转换的字符串
    /// * `endptr` - 如果不为空，存储第一个无效字符的位置
    /// * `base` - 数字基数（2-36，0表示自动检测）
    ///
    /// # 返回值
    /// * 转换后的长整数值
    fn strtol(&self, nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;

    /// 将字符串转换为浮点数
    ///
    /// # 参数
    /// * `nptr` - 要转换的字符串
    ///
    /// # 返回值
    /// * 转换后的浮点数值
    fn atof(&self, nptr: *const c_char) -> f64;

    /// 将字符串转换为整数
    ///
    /// # 参数
    /// * `nptr` - 要转换的字符串
    ///
    /// # 返回值
    /// * 转换后的整数值
    fn atoi(&self, nptr: *const c_char) -> c_int;

    /// 将字符串转换为双精度浮点数
    ///
    /// # 参数
    /// * `nptr` - 要转换的字符串
    /// * `endptr` - 如果不为空，存储第一个无效字符的位置
    ///
    /// # 返回值
    /// * 转换后的双精度浮点数值
    fn strtod(&self, nptr: *const c_char, endptr: *mut *mut c_char) -> f64;

    // === 随机数函数 ===

    /// 生成随机数
    ///
    /// # 返回值
    /// * 返回0到RAND_MAX之间的随机整数
    fn rand(&self) -> c_int;

    /// 设置随机数种子
    ///
    /// # 参数
    /// * `seed` - 随机数种子
    fn srand(&self, seed: c_uint);

    // === 程序控制函数 ===

    /// 异常终止程序
    ///
    /// # 安全性
    /// 这个函数不会返回，会立即终止程序
    fn abort(&self) -> !;

    /// 断言宏（运行时检查）
    ///
    /// # 参数
    /// * `condition` - 要检查的条件
    ///
    /// # 安全性
    /// 如果condition为false，会调用abort终止程序
    fn assert(&self, condition: bool);

    // === 错误处理函数 ===

    /// 打印错误消息
    ///
    /// # 参数
    /// * `s` - 错误消息前缀
    fn perror(&self, s: *const c_char);

    /// 获取错误消息字符串
    ///
    /// # 参数
    /// * `errnum` - 错误号
    ///
    /// # 返回值
    /// * 返回指向错误消息字符串的指针
    fn strerror(&self, errnum: c_int) -> *const c_char;

    /// 获取错误类型
    ///
    /// # 返回值
    /// * 返回错误类型代码
    fn error_type(&self) -> c_int;

    /// 获取错误代码
    ///
    /// # 返回值
    /// * 返回具体错误代码
    fn error_code(&self) -> c_int;

    // === 环境变量函数 ===

    /// 获取环境变量值
    ///
    /// # 参数
    /// * `name` - 环境变量名
    ///
    /// # 返回值
    /// * 找到时返回指向值的指针
    /// * 未找到时返回NULL
    fn getenv(&self, name: *const c_char) -> *mut c_char;

    /// 设置环境变量
    ///
    /// # 参数
    /// * `name` - 环境变量名
    /// * `value` - 环境变量值
    /// * `overwrite` - 如果变量已存在，是否覆盖
    ///
    /// # 返回值
    /// * 成功时返回0
    /// * 失败时返回-1
    fn setenv(&self, name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;

    /// 删除环境变量
    ///
    /// # 参数
    /// * `name` - 环境变量名
    ///
    /// # 返回值
    /// * 成功时返回0
    /// * 失败时返回-1
    fn unsetenv(&self, name: *const c_char) -> c_int;

    // === 排序和搜索函数 ===

    /// 快速排序
    ///
    /// # 参数
    /// * `base` - 要排序的数组起始地址
    /// * `nmemb` - 元素数量
    /// * `size` - 每个元素的大小
    /// * `compar` - 比较函数指针
    fn qsort(&self, base: *mut c_void, nmemb: size_t, size: size_t, compar: extern "C" fn(*const c_void, *const c_void) -> c_int);

    /// 二分搜索
    ///
    /// # 参数
    /// * `key` - 要查找的键
    /// * `base` - 已排序数组的起始地址
    /// * `nmemb` - 元素数量
    /// * `size` - 每个元素的大小
    /// * `compar` - 比较函数指针
    ///
    /// # 返回值
    /// * 找到时返回指向匹配元素的指针
    /// * 未找到时返回NULL
    fn bsearch(&self, key: *const c_void, base: *const c_void, nmemb: size_t, size: size_t, compar: extern "C" fn(*const c_void, *const c_void) -> c_int) -> *mut c_void;

    // === 除法函数 ===

    /// 整数除法
    ///
    /// # 参数
    /// * `numer` - 被除数
    /// * `denom` - 除数
    ///
    /// # 返回值
    /// * 返回包含商和余数的div_t结构
    fn div(&self, numer: c_int, denom: c_int) -> DivT;

    /// 长整数除法
    ///
    /// # 参数
    /// * `numer` - 被除数
    /// * `denom` - 除数
    ///
    /// # 返回值
    /// * 返回包含商和余数的ldiv_t结构
    fn ldiv(&self, numer: c_long, denom: c_long) -> LDivT;

    /// 初始化C库实例
    ///
    /// # 返回值
    /// * 成功时返回Ok(())
    /// * 失败时返回错误信息
    fn initialize(&self) -> CLibResult<()>;

    /// 获取C库统计信息
    ///
    /// # 返回值
    /// * 返回包含统计信息的结构
    fn get_stats(&self) -> CLibStats;
}

/// C库实现类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImplementationType {
    /// 最小化实现 - 仅提供最核心的功能
    Minimal,
    /// 简化实现 - 提供常用的C库功能
    Simple,
    /// 完整实现 - 提供完整的POSIX兼容C库
    Full,
    /// 统一实现 - 提供完整的C库功能，使用统一内存管理
    Unified,
}

/// C库统计信息
#[derive(Debug, Clone, Default)]
pub struct CLibStats {
    /// 当前分配的字节数
    pub memory_allocated: u64,
    /// 峰值分配字节数
    pub memory_peak: u64,
    /// 总分配次数
    pub allocations_total: u64,
    /// 总释放次数
    pub deallocations_total: u64,
    /// 当前活跃分配数量
    pub allocations_active: u64,
    /// 内存池命中率（百分比）
    pub pool_hit_rate: f64,
    /// 内存分配统计
    pub memory_stats: MemoryStats,
    /// 函数调用统计
    pub function_calls: FunctionCallStats,
    /// 错误统计
    pub error_stats: ErrorStats,
}

/// 内存使用统计
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// 总分配次数
    pub total_allocations: u64,
    /// 当前分配的字节数
    pub allocated_bytes: u64,
    /// 峰值分配字节数
    pub peak_allocated_bytes: u64,
    /// 活跃分配数量
    pub active_allocations: u64,
    /// 内存碎片比例（百分比）
    pub fragmentation_ratio: f64,
}

/// 函数调用统计
#[derive(Debug, Clone, Default)]
pub struct FunctionCallStats {
    /// malloc调用次数
    pub malloc_calls: u64,
    /// free调用次数
    pub free_calls: u64,
    /// printf调用次数
    pub printf_calls: u64,
    /// 字符串操作调用次数
    pub string_calls: u64,
}

/// 错误统计
#[derive(Debug, Clone, Default)]
pub struct ErrorStats {
    /// 内存分配失败次数
    pub malloc_failures: u64,
    /// 字符串操作错误次数
    pub string_errors: u64,
    /// I/O错误次数
    pub io_errors: u64,
}

/// C库操作结果
pub type CLibResult<T> = Result<T, CLibError>;

/// C库错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CLibError {
    /// 内存不足
    OutOfMemory,
    /// 无效参数
    InvalidParameter(&'static str),
    /// 无效指针
    InvalidPointer,
    /// 缓冲区溢出
    BufferOverflow,
    /// I/O错误
    IOError(c_int),
    /// 系统调用错误
    SystemError(c_int),
    /// 未初始化
    Uninitialized,
    /// 功能未实现
    NotImplemented,
    /// 其他错误
    Other(&'static str),
}

impl core::fmt::Display for CLibError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CLibError::OutOfMemory => write!(f, "内存不足"),
            CLibError::InvalidParameter(msg) => write!(f, "无效参数: {}", msg),
            CLibError::InvalidPointer => write!(f, "无效指针"),
            CLibError::BufferOverflow => write!(f, "缓冲区溢出"),
            CLibError::IOError(code) => write!(f, "I/O错误 (错误码: {})", code),
            CLibError::SystemError(code) => write!(f, "系统错误 (错误码: {})", code),
            CLibError::Uninitialized => write!(f, "C库未初始化"),
            CLibError::NotImplemented => write!(f, "功能未实现"),
            CLibError::Other(msg) => write!(f, "其他错误: {}", msg),
        }
    }
}

/// 全局C库接口实例
///
/// 这是当前活跃的C库实现的全局实例。
/// 在系统初始化时根据配置选择合适的实现。
static mut GLOBAL_CLIB: Option<&'static dyn CLibInterface> = None;
static mut CLIB_INITIALIZED: bool = false;

/// 初始化全局C库接口
///
/// # 参数
/// * `impl_` - 要使用的C库实现
///
/// # 安全性
/// 这个函数只能在系统初始化时调用一次
pub unsafe fn initialize_c_lib(impl_: &'static dyn CLibInterface) {
    if CLIB_INITIALIZED {
        crate::println!("[libc] 警告：C库已经初始化，跳过重复初始化");
        return;
    }

    GLOBAL_CLIB = Some(impl_);
    CLIB_INITIALIZED = true;
    crate::println!("[libc] C库接口初始化完成");
}

/// 获取全局C库接口
///
/// # 返回值
/// * 返回当前活跃的C库实现引用
///
/// # 安全性
/// 必须在C库初始化后调用
pub unsafe fn get_c_lib() -> &'static dyn CLibInterface {
    if let Some(lib) = GLOBAL_CLIB {
        lib
    } else {
        // 在开发阶段，如果未初始化则panic
        panic!("C库未初始化！请确保在系统启动时调用initialize_c_lib()");
    }
}

/// 检查C库是否已初始化
pub fn is_c_lib_initialized() -> bool {
    unsafe { CLIB_INITIALIZED }
}

/// C库宏定义
///
/// 提供常用的C库宏和常量定义
pub mod macros {
    use core::ffi::{c_void, c_int};
    
    /// NULL指针定义
    pub const NULL: *mut c_void = core::ptr::null_mut();

    /// 标准文件描述符
    pub const STDIN_FILENO: c_int = 0;
    pub const STDOUT_FILENO: c_int = 1;
    pub const STDERR_FILENO: c_int = 2;

    /// 文件访问模式
    pub const O_RDONLY: c_int = 0o00000;
    pub const O_WRONLY: c_int = 0o00001;
    pub const O_RDWR: c_int = 0o00002;
    pub const O_CREAT: c_int = 0o00100;
    pub const O_EXCL: c_int = 0o00200;
    pub const O_TRUNC: c_int = 0o01000;
    pub const O_APPEND: c_int = 0o02000;
    pub const O_NONBLOCK: c_int = 0o04000;

    /// lseek常量
    pub const SEEK_SET: c_int = 0;
    pub const SEEK_CUR: c_int = 1;
    pub const SEEK_END: c_int = 2;

    /// 退出码
    pub const EXIT_SUCCESS: c_int = 0;
    pub const EXIT_FAILURE: c_int = 1;

    /// EOF定义
    pub const EOF: c_int = -1;
}
