//! 共享内存系统调用处理
//!
//! 实现共享内存相关系统调用

use crate::error::unified::KernelResult;
use crate::api::KernelError;

use super::utils::extract_args;

/// shmget系统调用处理函数
///
/// 创建或获取共享内存段。
///
/// # 参数
///
/// * `args` - 系统调用参数：[key, size, shmflg]
///
/// # 返回值
///
/// * `Ok(u64)` - 共享内存ID
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_shmget(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 3)?;
    let key = args[0] as i32;
    let size = args[1] as usize;
    let shmflg = args[2] as i32;

    // Use POSIX shmget implementation
    use crate::subsystems::posix::shm::shmget;
    let shmid = unsafe { shmget(key, size, shmflg) };

    if shmid < 0 {
        Err(KernelError::InvalidArgument)
    } else {
        Ok(shmid as u64)
    }
}

/// shmat系统调用处理函数
///
/// 将共享内存段附加到进程地址空间。
///
/// # 参数
///
/// * `args` - 系统调用参数：[shmid, shmaddr, shmflg]
///
/// # 返回值
///
/// * `Ok(u64)` - 附加地址
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_shmat(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 3)?;
    let shmid = args[0] as i32;
    let shmaddr = args[1] as *mut u8;
    let shmflg = args[2] as i32;

    // Use POSIX shmat implementation
    use crate::subsystems::posix::shm::shmat;
    let addr = unsafe { shmat(shmid, shmaddr, shmflg) };

    if addr.is_null() {
        Err(KernelError::InvalidArgument)
    } else {
        Ok(addr as usize as u64)
    }
}

/// shmdt系统调用处理函数
///
/// 从进程地址空间分离共享内存段。
///
/// # 参数
///
/// * `args` - 系统调用参数：[shmaddr]
///
/// # 返回值
///
/// * `Ok(u64)` - 0表示成功
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_shmdt(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 1)?;
    let shmaddr = args[0] as *mut u8;

    // Use POSIX shmdt implementation
    use crate::subsystems::posix::shm::shmdt;
    let result = unsafe { shmdt(shmaddr) };

    if result < 0 {
        Err(KernelError::InvalidArgument)
    } else {
        Ok(0)
    }
}

/// shmctl系统调用处理函数
///
/// 控制共享内存段。
///
/// # 参数
///
/// * `args` - 系统调用参数：[shmid, cmd, buf]
///
/// # 返回值
///
/// * `Ok(u64)` - 0表示成功或操作结果
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_shmctl(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 3)?;
    let shmid = args[0] as i32;
    let cmd = args[1] as i32;
    let buf = args[2] as *mut crate::posix::ShmidDs;

    // Use POSIX shmctl implementation
    use crate::subsystems::posix::shm::shmctl;
    let result = unsafe { shmctl(shmid, cmd, buf) };

    if result < 0 {
        Err(KernelError::InvalidArgument)
    } else {
        Ok(result as u64)
    }
}
