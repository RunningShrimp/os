//! C标准库统一错误处理机制
//!
//! 提供统一的errno错误码管理和错误处理功能，确保所有C库实现使用一致的错误处理机制。

use core::ffi::c_int;
use core::sync::atomic::{AtomicI32, Ordering};
use heapless::Vec;
use crate::sync::Mutex;

/// 全局errno变量
///
/// 使用原子变量确保线程安全的errno访问
static mut ERRNO: c_int = 0;
static ERRNO_ATOMIC: AtomicI32 = AtomicI32::new(0);
static ERROR_STATS: Mutex<Option<ErrorStats>> = Mutex::new(None);

/// POSIX错误码定义
///
/// 这些错误码与标准POSIX系统保持一致
pub mod errno {
    use super::c_int;

    /// 操作不被允许
    pub const EPERM: c_int = 1;
    /// 没有这样的文件或目录
    pub const ENOENT: c_int = 2;
    /// 没有这样的进程
    pub const ESRCH: c_int = 3;
    /// 中断的系统调用
    pub const EINTR: c_int = 4;
    /// I/O错误
    pub const EIO: c_int = 5;
    /// 没有这样的设备或地址
    pub const ENXIO: c_int = 6;
    /// 参数列表过长
    pub const E2BIG: c_int = 7;
    /// 可执行文件格式错误
    pub const ENOEXEC: c_int = 8;
    /// 文件描述符错误
    pub const EBADF: c_int = 9;
    /// 没有子进程
    pub const ECHILD: c_int = 10;
    /// 再试一次
    pub const EAGAIN: c_int = 11;
    /// 没有足够的内存
    pub const ENOMEM: c_int = 12;
    /// 权限不够
    pub const EACCES: c_int = 13;
    /// 错误的地址
    pub const EFAULT: c_int = 14;
    /// 需要块设备
    pub const ENOTBLK: c_int = 15;
    /// 设备或资源忙
    pub const EBUSY: c_int = 16;
    /// 文件已存在
    pub const EEXIST: c_int = 17;
    /// 跨设备链接
    pub const EXDEV: c_int = 18;
    /// 没有这样的设备
    pub const ENODEV: c_int = 19;
    /// 不是目录
    pub const ENOTDIR: c_int = 20;
    /// 是目录
    pub const EISDIR: c_int = 21;
    /// 无效的参数
    pub const EINVAL: c_int = 22;
    /// 文件表溢出
    pub const ENFILE: c_int = 23;
    /// 打开的文件过多
    pub const EMFILE: c_int = 24;
    /// 设备不合适的ioctl调用
    pub const ENOTTY: c_int = 25;
    /// 文本文件忙
    pub const ETXTBSY: c_int = 26;
    /// 文件过大
    pub const EFBIG: c_int = 27;
    /// 设备上没有空间
    pub const ENOSPC: c_int = 28;
    /// 非法寻址
    pub const ESPIPE: c_int = 29;
    /// 只读文件系统
    pub const EROFS: c_int = 30;
    /// 链接过多
    pub const EMLINK: c_int = 31;
    /// 断开的管道
    pub const EPIPE: c_int = 32;
    /// 数学参数超出域
    pub const EDOM: c_int = 33;
    /// 结果超出范围
    pub const ERANGE: c_int = 34;
    /// 资源死锁
    pub const EDEADLK: c_int = 35;
    /// 文件名过长
    pub const ENAMETOOLONG: c_int = 36;
    /// 没有记录锁可用
    pub const ENOLCK: c_int = 37;
    /// 功能未实现
    pub const ENOSYS: c_int = 38;
    /// 目录非空
    pub const ENOTEMPTY: c_int = 39;
    /// 符号链接层次过多
    pub const ELOOP: c_int = 40;
    /// 没有期望的消息类型
    pub const ENOMSG: c_int = 42;
    /// 标识符被删除
    pub const EIDRM: c_int = 43;
    /// 通道号超出范围
    pub const ECHRNG: c_int = 44;
    /// 级别2不同步
    pub const EL2NSYNC: c_int = 45;
    /// 级别3停止
    pub const EL3HLT: c_int = 46;
    /// 级别3重置
    pub const EL3RST: c_int = 47;
    /// 链接号超出范围
    pub const ELNRNG: c_int = 48;
    /// 协议驱动程序未连接
    pub const EUNATCH: c_int = 49;
    /// 没有CSI结构可用
    pub const ENOCSI: c_int = 50;
    /// 级别2停止
    pub const EL2HLT: c_int = 51;
    /// 交换超出
    pub const EBADE: c_int = 52;
    /// 无效的请求代码
    pub const EBADR: c_int = 53;
    /// 无效的交换
    pub const EXFULL: c_int = 54;
    /// 无效的请求描述符
    pub const ENOANO: c_int = 55;
    /// 无效的请求参数
    pub const EBADRQC: c_int = 56;
    /// 无效的请求序列
    pub const EBADSLT: c_int = 57;
    /// 超时
    pub const ETIME: c_int = 62;
    /// 远程I/O错误
    pub const EREMOTE: c_int = 71;
    /// 协议错误
    pub const EPROTO: c_int = 71;
    /// 多次跳跃被尝试
    pub const EMULTIHOP: c_int = 72;
    /// 远程I/O
    pub const EDOTDOT: c_int = 73;
    /// 消息过大
    pub const EMSGSIZE: c_int = 90;
    /// 协议不可用
    pub const EPROTONOSUPPORT: c_int = 93;
    /// 套接字类型不支持
    pub const ESOCKTNOSUPPORT: c_int = 94;
    /// 操作不支持
    pub const ENOTSUP: c_int = 95;
    /// 操作不支持
    pub const EOPNOTSUPP: c_int = 95;
    /// 协议族不支持
    pub const EPFNOSUPPORT: c_int = 96;
    /// 地址族不支持
    pub const EAFNOSUPPORT: c_int = 97;
    /// 地址已在使用
    pub const EADDRINUSE: c_int = 98;
    /// 地址不可用
    pub const EADDRNOTAVAIL: c_int = 99;
    /// 网络不可达
    pub const ENETUNREACH: c_int = 101;
    /// 网络已重置
    pub const ENETRESET: c_int = 102;
    /// 连接被拒绝
    pub const ECONNREFUSED: c_int = 111;
    /// 连接超时
    pub const ETIMEDOUT: c_int = 110;
    /// 无法连接到主机
    pub const EHOSTUNREACH: c_int = 113;
    /// 连接重置
    pub const ECONNRESET: c_int = 104;
    /// 没有到主机的路由
    pub const EHOSTDOWN: c_int = 112;
    /// 请求的操作被取消
    pub const ECANCELED: c_int = 125;
}

/// 获取errno指针（线程安全）
///
/// 这是newlib和其他C库实现通常使用的函数
/// # 返回值
/// * 返回当前errno值的指针
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __errno() -> *mut c_int {
    let ptr = core::ptr::addr_of_mut!(ERRNO);
    ptr as *mut c_int
}

/// 获取当前errno值
///
/// # 返回值
/// * 当前的errno错误码
pub fn get_errno() -> c_int {
    unsafe { ERRNO }
}

/// 设置errno值
///
/// # 参数
/// * `error_code` - 要设置的errno错误码
pub fn set_errno(error_code: c_int) {
    unsafe {
        ERRNO = error_code;
        ERRNO_ATOMIC.store(error_code, Ordering::SeqCst);
    }
}

/// 原子性地设置errno值
///
/// # 参数
/// * `error_code` - 要设置的errno错误码
pub fn set_errno_atomic(error_code: c_int) {
    unsafe {
        ERRNO = error_code;
        ERRNO_ATOMIC.store(error_code, Ordering::SeqCst);
    }
}

/// 原子性地获取errno值
///
/// # 返回值
/// * 当前的errno错误码
pub fn get_errno_atomic() -> c_int {
    ERRNO_ATOMIC.load(Ordering::SeqCst)
}

/// 清除errno（设置为0）
pub fn clear_errno() {
    set_errno(0);
}

/// 检查是否有错误
///
/// # 返回值
/// * true如果errno不为0，false如果errno为0
pub fn has_error() -> bool {
    get_errno() != 0
}

/// 错误码到字符串的映射
///
/// # 参数
/// * `error_code` - errno错误码
///
/// # 返回值
/// * 错误码的描述字符串
pub fn strerror(error_code: c_int) -> &'static str {
    match error_code {
        errno::EPERM => "Operation not permitted",
        errno::ENOENT => "No such file or directory",
        errno::ESRCH => "No such process",
        errno::EINTR => "Interrupted system call",
        errno::EIO => "Input/output error",
        errno::ENXIO => "No such device or address",
        errno::E2BIG => "Argument list too long",
        errno::ENOEXEC => "Exec format error",
        errno::EBADF => "Bad file descriptor",
        errno::ECHILD => "No child processes",
        errno::EAGAIN => "Resource temporarily unavailable",
        errno::ENOMEM => "Not enough memory",
        errno::EACCES => "Permission denied",
        errno::EFAULT => "Bad address",
        errno::ENOTBLK => "Block device required",
        errno::EBUSY => "Device or resource busy",
        errno::EEXIST => "File exists",
        errno::EXDEV => "Invalid cross-device link",
        errno::ENODEV => "No such device",
        errno::ENOTDIR => "Not a directory",
        errno::EISDIR => "Is a directory",
        errno::EINVAL => "Invalid argument",
        errno::ENFILE => "Too many open files in system",
        errno::EMFILE => "Too many open files",
        errno::ENOTTY => "Inappropriate ioctl for device",
        errno::ETXTBSY => "Text file busy",
        errno::EFBIG => "File too large",
        errno::ENOSPC => "No space left on device",
        errno::ESPIPE => "Illegal seek",
        errno::EROFS => "Read-only file system",
        errno::EMLINK => "Too many links",
        errno::EPIPE => "Broken pipe",
        errno::EDOM => "Mathematics argument out of domain of function",
        errno::ERANGE => "Result too large",
        errno::EDEADLK => "Resource deadlock avoided",
        errno::ENAMETOOLONG => "File name too long",
        errno::ENOLCK => "No locks available",
        errno::ENOSYS => "Function not implemented",
        errno::ENOTEMPTY => "Directory not empty",
        errno::ELOOP => "Too many levels of symbolic links",
        errno::ENOMSG => "No message of desired type",
        errno::EIDRM => "Identifier removed",
        errno::ECHRNG => "Channel number out of range",
        errno::EL2NSYNC => "Level 2 not synchronized",
        errno::EL3HLT => "Level 3 halted",
        errno::EL3RST => "Level 3 reset",
        errno::ELNRNG => "Link number out of range",
        errno::EUNATCH => "Protocol driver not attached",
        errno::ENOCSI => "No CSI structure available",
        errno::EL2HLT => "Level 2 halted",
        errno::ETIME => "Timer expired",
        errno::EREMOTE => "Remote I/O error",
        errno::EPROTO => "Protocol error",
        errno::EMULTIHOP => "Multihop attempted",
        errno::EDOTDOT => "RFS specific error",
        errno::EMSGSIZE => "Message too long",
        errno::EPROTONOSUPPORT => "Protocol not supported",
        errno::ESOCKTNOSUPPORT => "Socket type not supported",
        errno::ENOTSUP | errno::EOPNOTSUPP => "Operation not supported",
        errno::EPFNOSUPPORT => "Protocol family not supported",
        errno::EAFNOSUPPORT => "Address family not supported",
        errno::EADDRINUSE => "Address already in use",
        errno::EADDRNOTAVAIL => "Cannot assign requested address",
        errno::ENETUNREACH => "Network is unreachable",
        errno::ENETRESET => "Network dropped connection on reset",
        errno::ECONNREFUSED => "Connection refused",
        errno::ETIMEDOUT => "Connection timed out",
        errno::EHOSTDOWN => "Host is down",
        errno::EHOSTUNREACH => "No route to host",
        errno::ECONNRESET => "Connection reset by peer",
        errno::ECANCELED => "Operation canceled",
        _ => "Unknown error",
    }
}

/// 错误处理宏
///
/// 提供常用的错误处理宏定义
#[macro_export]
macro_rules! libc_error {
    ($error_code:expr) => {
        $crate::libc::error::set_errno($error_code);
        -1
    };
}

/// 成功返回宏
#[macro_export]
macro_rules! libc_success {
    () => {
        0
    };
    ($value:expr) => {
        $value
    };
}

/// 检查错误并返回的宏
#[macro_export]
macro_rules! libc_check {
    ($result:expr, $error_code:expr) => {
        if $result < 0 {
            $crate::libc::error::set_errno($error_code);
            -1
        } else {
            $result
        }
    };
}

/// 错误统计结构
#[derive(Debug)]
pub struct ErrorStats {
    /// 各类错误的发生次数
    pub error_counts: [u64; 128],
    /// 总错误次数
    pub total_errors: u64,
    /// 最近一次错误码
    pub last_error: c_int,
    /// 错误历史（最多保存最近的100个错误）
    pub error_history: Vec<c_int, 100>,
}

impl Default for ErrorStats {
    fn default() -> Self {
        Self {
            error_counts: [0u64; 128],
            total_errors: 0,
            last_error: 0,
            error_history: Vec::new(),
        }
    }
}

impl ErrorStats {
    pub fn new() -> Self {
        Self::default()
    }
}

/// 记录错误统计
///
/// # 参数
/// * `error_code` - 发生的错误码
pub fn record_error(error_code: c_int) {
    unsafe {
        ERRNO = error_code;
        ERRNO_ATOMIC.store(error_code, Ordering::SeqCst);
    }
    let mut stats = ERROR_STATS.lock();
    let s = stats.get_or_insert_with(ErrorStats::new);
    s.last_error = error_code;
    s.total_errors = s.total_errors.saturating_add(1);
    let idx = (error_code as usize) % s.error_counts.len();
    s.error_counts[idx] = s.error_counts[idx].saturating_add(1);
    if s.error_history.len() < s.error_history.capacity() {
        let _ = s.error_history.push(error_code);
    }

    // 可以在调试模式下打印错误信息
    #[cfg(debug_assertions)]
    {
        crate::println!("[libc] errno {} ({})", error_code, strerror(error_code));
    }
}

/// 根据条件设置errno
///
/// # 参数
/// * `condition` - 条件表达式
/// * `error_code` - 如果条件为假则设置的错误码
///
/// # 返回值
/// * true如果条件为真，false如果条件为假
pub fn check_errno(condition: bool, error_code: c_int) -> bool {
    if !condition {
        set_errno(error_code);
        false
    } else {
        true
    }
}

/// 初始化错误处理系统
pub fn init_error_handling() {
    unsafe {
        ERRNO = 0;
        ERRNO_ATOMIC.store(0, Ordering::SeqCst);
    }
    let mut stats = ERROR_STATS.lock();
    if stats.is_none() {
        *stats = Some(ErrorStats::new());
    }
    crate::println!("[libc] 错误处理系统初始化完成");
}

pub fn error_stats_string() -> alloc::string::String {
    let mut guard = ERROR_STATS.lock();
    let st = guard.get_or_insert_with(ErrorStats::new);
    let mut out = alloc::string::String::new();
    out.push_str("# Error Stats\n");
    out.push_str(&alloc::format!("total_errors: {}\n", st.total_errors));
    out.push_str(&alloc::format!("last_error: {} ({})\n", st.last_error, strerror(st.last_error)));
    out.push_str("recent: [");
    for (i, e) in st.error_history.iter().enumerate() {
        out.push_str(&alloc::format!("{}{}", e, if i + 1 < st.error_history.len() { "," } else { "" }));
    }
    out.push_str("]\n");
    out
}

pub fn error_stats_json() -> alloc::string::String {
    let mut guard = ERROR_STATS.lock();
    let st = guard.get_or_insert_with(ErrorStats::new);
    let mut out = alloc::string::String::new();
    out.push_str("{\n");
    out.push_str(&alloc::format!("  \"total_errors\": {},\n", st.total_errors));
    out.push_str(&alloc::format!("  \"last_error\": {},\n", st.last_error));
    out.push_str(&alloc::format!("  \"last_error_str\": \"{}\",\n", strerror(st.last_error)));
    out.push_str("  \"recent\": [");
    for (i, e) in st.error_history.iter().enumerate() {
        out.push_str(&alloc::format!("{}{}", e, if i + 1 < st.error_history.len() { "," } else { "" }));
    }
    out.push_str(" ]\n}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_errno_operations() {
        clear_errno();
        assert_eq!(get_errno(), 0);
        assert!(!has_error());

        set_errno(errno::ENOMEM);
        assert_eq!(get_errno(), errno::ENOMEM);
        assert!(has_error());

        clear_errno();
        assert_eq!(get_errno(), 0);
    }

    #[test]
    fn test_strerror() {
        assert!(!strerror(errno::ENOMEM).is_empty());
        assert!(!strerror(errno::ENOENT).is_empty());
        assert_eq!(strerror(999), "Unknown error");
    }

    #[test]
    fn test_errno_atomic() {
        set_errno_atomic(errno::EIO);
        assert_eq!(get_errno_atomic(), errno::EIO);
        assert_eq!(get_errno(), errno::EIO);
    }

    #[test]
    fn test_check_errno() {
        assert!(check_errno(true, errno::EINVAL));
        assert_eq!(get_errno(), errno::EINVAL);

        assert!(!check_errno(false, errno::ENOMEM));
        assert_eq!(get_errno(), errno::ENOMEM);
    }
}
