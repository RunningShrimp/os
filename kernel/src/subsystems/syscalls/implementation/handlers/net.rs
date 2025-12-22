//! 网络系统调用辅助函数
//!
//! 本模块包含网络系统调用的辅助函数，为服务层提供基础支持。
//! 所有核心逻辑已移至 NetworkService 内，不再有独立的全局处理函数。

use nos_nos_error_handling::unified::KernelError;
use crate::syscalls::net::types::*;

/// 从用户空间复制地址结构
/// TODO: 实现真正的用户空间拷贝机制，目前为占位符
pub fn copyin_from_user<T>(user_addr: u64, _kernel_addr: &mut T) -> Result<(), KernelError> {
    // TODO: 实现用户空间到内核空间的数据拷贝
    // 需要检查地址有效性、权限等
    if user_addr == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // 临时实现：仅校验地址非零即可
    crate::log_debug!("copyin_from_user: addr={:#x} (stub ok)", user_addr);
    Ok(())
}

/// 向用户空间复制数据
/// TODO: 实现真正的用户空间拷贝机制，目前为占位符
pub fn copyout_to_user<T>(_kernel_addr: &T, user_addr: u64) -> Result<(), KernelError> {
    // TODO: 实现内核空间到用户空间的数据拷贝
    if user_addr == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // 临时实现：仅校验地址非零即可
    crate::log_debug!("copyout_to_user: addr={:#x} (stub ok)", user_addr);
    Ok(())
}

/// 从用户空间复制地址长度
/// TODO: 实现真正的地址长度处理
pub fn copyin_addrlen(user_addrlen_ptr: u64) -> Result<u32, KernelError> {
    // TODO: 从用户空间读取地址长度
    if user_addrlen_ptr == 0 {
        return Ok(0);
    }

    // 临时实现 - 返回典型 IPv4 sockaddr 长度
    crate::log_debug!("copyin_addrlen: ptr={:#x} (stub -> 16)", user_addrlen_ptr);
    Ok(16)
}

/// 向用户空间写回地址长度
/// TODO: 实现真正的地址长度处理
pub fn copyout_addrlen(addrlen: u32, user_addrlen_ptr: u64) -> Result<(), KernelError> {
    // TODO: 向用户空间写入地址长度
    if user_addrlen_ptr == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // 临时实现：仅记录长度
    crate::log_debug!("copyout_addrlen: addrlen={}, ptr={:#x} (stub ok)", addrlen, user_addrlen_ptr);
    Ok(())
}

/// 验证网络地址结构
/// TODO: 实现完整的地址验证，目前为占位符
pub fn validate_network_address(_addr_ptr: u64, _addrlen: u32) -> Result<NetworkAddress, KernelError> {
    // TODO: 验证地址结构完整性、长度边界等
    // 临时实现 - 返回一个默认地址
    crate::log_debug!("validate_network_address: addr_ptr={:#x}, addrlen={} (stub default)", _addr_ptr, _addrlen);
    Ok(NetworkAddress::ipv4([0u8; 4], 0u16))
}

/// 获取网络系统调用号映射
///
/// 返回网络模块支持的系统调用号列表。
///
/// # 返回值
///
/// * `Vec<u32>` - 系统调用号列表
pub fn get_supported_syscalls() -> alloc::vec::Vec<u32> {
    alloc::vec![
        // Linux系统调用号（x86_64）
        41,  // socket
        49,  // bind
        50,  // listen
        43,  // accept
        42,  // connect
        44,  // send
        45,  // recv
        46,  // sendto
        47,  // recvfrom
        48,  // shutdown
        54,  // getsockopt
        55,  // setsockopt
    ]
}

/// 系统调用分发函数
///
/// 已废弃：所有系统调用现在由 NetworkService 处理。
/// 此函数仅为向后兼容而保留。
///
/// # 参数
///
/// * `syscall_number` - 系统调用号
/// * `args` - 系统调用参数
///
/// # 返回值
///
/// * `Ok(u64)` - 系统调用执行结果
/// * `Err(KernelError)` - 系统调用执行失败
#[deprecated(note = "Use NetworkService.handle_syscall instead")]
pub fn dispatch_syscall(syscall_number: u32, args: &[u64]) -> Result<u64, KernelError> {
    crate::log_debug!("deprecated dispatch_syscall called: {}, forwarding to service would be needed", syscall_number);
    // 所有逻辑已移至 NetworkService，不再有独立的全局处理函数
    match syscall_number {
        41 | 49 | 50 | 43 | 42 | 44 | 45 | 46 | 47 | 48 | 54 | 55 => {
            // 这些调用现在需要通过 NetworkService 的实例来处理
            Err(KernelError::NotSupported)
        }
        _ => Err(KernelError::UnsupportedSyscall),
    }
}