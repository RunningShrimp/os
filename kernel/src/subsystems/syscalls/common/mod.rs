//! 系统调用通用模块
//!
//! 本模块提供系统调用的通用功能和常量定义。

/// 系统调用错误码
pub mod error_codes {
    /// 成功
    pub const SUCCESS: isize = 0;
    /// 一般错误
    pub const ERROR: isize = -1;
    /// 参数无效
    pub const EINVAL: isize = -2;
    /// 权限被拒绝
    pub const EPERM: isize = -3;
    /// 文件不存在
    pub const ENOENT: isize = -4;
    /// 内存不足
    pub const ENOMEM: isize = -5;
    /// 资源忙
    pub const EBUSY: isize = -6;
    /// 资源不存在
    pub const ENOENT: isize = -7;
    /// 操作不支持
    pub const ENOTSUP: isize = -8;
}

/// 系统调用号范围
pub mod syscall_ranges {
    /// 文件系统系统调用范围
    pub const FS_RANGE_START: usize = 100;
    pub const FS_RANGE_END: usize = 199;
    
    /// 进程管理系统调用范围
    pub const PROCESS_RANGE_START: usize = 200;
    pub const PROCESS_RANGE_END: usize = 299;
    
    /// 网络系统调用范围
    pub const NETWORK_RANGE_START: usize = 300;
    pub const NETWORK_RANGE_END: usize = 399;
    
    /// IPC系统调用范围
    pub const IPC_RANGE_START: usize = 400;
    pub const IPC_RANGE_END: usize = 499;
}

/// 系统调用标志
pub mod syscall_flags {
    /// 阻塞标志
    pub const BLOCKING: usize = 0x01;
    /// 非阻塞标志
    pub const NONBLOCKING: usize = 0x02;
    /// 异步标志
    pub const ASYNC: usize = 0x04;
    /// 强制标志
    pub const FORCE: usize = 0x08;
    /// 只读标志
    pub const READONLY: usize = 0x10;
    /// 只写标志
    pub const WRITEONLY: usize = 0x20;
    /// 读写标志
    pub const READWRITE: usize = 0x30;
}

/// 系统调用参数验证
pub fn validate_syscall_args(syscall_num: usize, args: &[usize]) -> bool {
    // 基本验证
    if args.len() > 6 {
        return false;
    }
    
    // 根据系统调用范围进行特定验证
    match syscall_num {
        syscall_ranges::FS_RANGE_START..=syscall_ranges::FS_RANGE_END => {
            validate_fs_args(args)
        }
        syscall_ranges::PROCESS_RANGE_START..=syscall_ranges::PROCESS_RANGE_END => {
            validate_process_args(args)
        }
        syscall_ranges::NETWORK_RANGE_START..=syscall_ranges::NETWORK_RANGE_END => {
            validate_network_args(args)
        }
        syscall_ranges::IPC_RANGE_START..=syscall_ranges::IPC_RANGE_END => {
            validate_ipc_args(args)
        }
        _ => false,
    }
}

/// 验证文件系统系统调用参数
fn validate_fs_args(args: &[usize]) -> bool {
    // 基本验证
    if args.is_empty() {
        return false;
    }
    
    // 根据具体的文件系统操作进行验证
    match args[0] {
        0 => args.len() >= 3, // open
        1 => args.len() >= 2, // close
        2 => args.len() >= 4, // read
        3 => args.len() >= 4, // write
        4 => args.len() >= 3, // lseek
        5 => args.len() >= 2, // stat
        _ => false,
    }
}

/// 验证进程管理系统调用参数
fn validate_process_args(args: &[usize]) -> bool {
    // 基本验证
    if args.is_empty() {
        return false;
    }
    
    // 根据具体的进程管理操作进行验证
    match args[0] {
        0 => args.len() >= 1, // fork
        1 => args.len() >= 3, // exec
        2 => args.len() >= 2, // exit
        3 => args.len() >= 3, // wait
        4 => args.len() >= 2, // kill
        5 => args.len() >= 1, // getpid
        _ => false,
    }
}

/// 验证网络系统调用参数
fn validate_network_args(args: &[usize]) -> bool {
    // 基本验证
    if args.is_empty() {
        return false;
    }
    
    // 根据具体的网络操作进行验证
    match args[0] {
        0 => args.len() >= 3, // socket
        1 => args.len() >= 3, // bind
        2 => args.len() >= 3, // connect
        3 => args.len() >= 2, // listen
        4 => args.len() >= 3, // accept
        5 => args.len() >= 4, // send
        6 => args.len() >= 4, // recv
        _ => false,
    }
}

/// 验证IPC系统调用参数
fn validate_ipc_args(args: &[usize]) -> bool {
    // 基本验证
    if args.is_empty() {
        return false;
    }
    
    // 根据具体的IPC操作进行验证
    match args[0] {
        0 => args.len() >= 2, // pipe
        1 => args.len() >= 3, // msgget
        2 => args.len() >= 4, // msgsnd
        3 => args.len() >= 4, // msgrcv
        4 => args.len() >= 3, // semget
        5 => args.len() >= 3, // semop
        6 => args.len() >= 3, // shmget
        7 => args.len() >= 3, // shmat
        _ => false,
    }
}

/// 系统调用结果处理
pub fn handle_syscall_result(result: Result<isize>) -> isize {
    match result {
        Ok(value) => value,
        Err(e) => {
            // 根据错误类型返回相应的错误码
            match e {
                nos_api::Error::InvalidArgument(_) => error_codes::EINVAL,
                nos_api::Error::PermissionDenied(_) => error_codes::EPERM,
                nos_api::Error::NotFound(_) => error_codes::ENOENT,
                nos_api::Error::OutOfMemory => error_codes::ENOMEM,
                nos_api::Error::Busy(_) => error_codes::EBUSY,
                nos_api::Error::NotImplemented(_) => error_codes::ENOTSUP,
                _ => error_codes::ERROR,
            }
        }
    }
}