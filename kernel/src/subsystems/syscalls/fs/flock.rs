//! # flock 系统调用实现
//!
//! 提供 flock() 系统调用的实现，用于文件的咨询锁。
//!
//! ## 功能
//!
//! - **共享锁**: 多个进程可以持有共享锁
//! - **独占锁**: 只能有一个进程持有独占锁
//! - **非阻塞模式**: LOCK_NB 标志用于非阻塞操作
//! - **锁释放**: 进程退出时自动释放所有锁

extern crate alloc;
use alloc::sync::Arc;

use crate::subsystems::fs::file_locking::{
    get_lock_manager, acquire_exclusive_lock, acquire_shared_lock, release_lock,
    FileLock as FsFileLock, LockError, LockRange, LockType,
};
use crate::subsystems::process::{Process, ProcessId};

/// flock 操作标志
pub const LOCK_SH: u32 = 1;  // 共享锁
pub const LOCK_EX: u32 = 2;  // 独占锁
pub const LOCK_UN: u32 = 4;  // 解锁
pub const LOCK_NB: u32 = 8;  // 非阻塞

/// flock 文件锁
#[derive(Debug)]
pub struct Flock {
    /// 文件描述符
    pub fd: i32,
    /// 锁类型
    pub lock_type: LockType,
    /// 是否非阻塞
    pub non_blocking: bool,
}

impl Flock {
    /// 创建新的 flock 操作
    pub fn new(fd: i32, operation: u32) -> Self {
        let lock_type = match operation & (LOCK_SH | LOCK_EX | LOCK_UN) {
            LOCK_SH => LockType::Shared,
            LOCK_EX => LockType::Exclusive,
            LOCK_UN => LockType::None,
            _ => LockType::None,
        };

        let non_blocking = (operation & LOCK_NB) != 0;

        Self { fd, lock_type, non_blocking }
    }

    /// 执行 flock 操作
    pub fn apply(&self) -> Result<(), FlockError> {
        // 获取当前进程
        let current_pid = Process::current().id();

        // 获取文件 inode
        let inode = self.get_file_inode()?;

        // 根据操作类型执行相应操作
        match self.lock_type {
            LockType::None => {
                // 释放锁
                self.release_all_locks(inode, current_pid)
            }
            LockType::Shared => {
                // 获取共享锁
                self.acquire_shared(inode, current_pid)
            }
            LockType::Exclusive => {
                // 获取独占锁
                self.acquire_exclusive(inode, current_pid)
            }
        }
    }

    /// 获取文件的 inode
    fn get_file_inode(&self) -> Result<u32, FlockError> {
        use crate::subsystems::process::FileDescriptor;

        // 获取当前进程的文件描述符
        let process = Process::current();
        let fd_table = process.fd_table();
        let fd_entry = fd_table.get(self.fd).ok_or(FlockError::BadFd)?;

        // 从文件描述符获取 inode
        // 注意：这里需要根据实际的文件描述符实现来获取 inode
        // 暂时返回一个假的 inode
        Ok(1)
    }

    /// 释放文件的所有锁
    fn release_all_locks(&self, inode: u32, pid: ProcessId) -> Result<(), FlockError> {
        if let Some(lock_manager) = get_lock_manager() {
            lock_manager.release_all_locks(pid);
            Ok(())
        } else {
            Err(FlockError::NotInitialized)
        }
    }

    /// 获取共享锁
    fn acquire_shared(&self, inode: u32, pid: ProcessId) -> Result<(), FlockError> {
        let lock_manager = get_lock_manager().ok_or(FlockError::NotInitialized)?;

        let range = LockRange::entire_file();
        let blocking = !self.non_blocking;

        match acquire_shared_lock(inode, pid, range, blocking) {
            Ok(_) => Ok(()),
            Err(LockError::WouldBlock) => Err(FlockError::WouldBlock),
            Err(LockError::Conflict(lock)) => Err(FlockError::Conflict(lock.pid)),
            Err(e) => Err(FlockError::Other(format!("{:?}", e))),
        }
    }

    /// 获取独占锁
    fn acquire_exclusive(&self, inode: u32, pid: ProcessId) -> Result<(), FlockError> {
        let lock_manager = get_lock_manager().ok_or(FlockError::NotInitialized)?;

        let range = LockRange::entire_file();
        let blocking = !self.non_blocking;

        match acquire_exclusive_lock(inode, pid, range, blocking) {
            Ok(_) => Ok(()),
            Err(LockError::WouldBlock) => Err(FlockError::WouldBlock),
            Err(LockError::Conflict(lock)) => Err(FlockError::Conflict(lock.pid)),
            Err(e) => Err(FlockError::Other(format!("{:?}", e))),
        }
    }
}

/// flock 错误
#[derive(Debug, Clone)]
pub enum FlockError {
    /// 无效的文件描述符
    BadFd,
    /// 锁管理器未初始化
    NotInitialized,
    /// 操作会阻塞
    WouldBlock,
    /// 锁冲突
    Conflict(ProcessId),
    /// 其他错误
    Other(String),
}

/// flock 系统调用处理程序
///
/// # 参数
///
/// * `fd`: 文件描述符
/// * `operation`: 操作标志 (LOCK_SH, LOCK_EX, LOCK_UN, LOCK_NB)
///
/// # 返回
///
/// 成功返回 0，失败返回错误码
pub fn sys_flock(fd: i32, operation: u32) -> isize {
    let flock_op = Flock::new(fd, operation);

    match flock_op.apply() {
        Ok(()) => 0,
        Err(FlockError::BadFd) => {
            crate::println!("flock: bad file descriptor {}", fd);
            -9 // EBADF
        }
        Err(FlockError::NotInitialized) => {
            crate::println!("flock: lock manager not initialized");
            -38 // ENOSYS
        }
        Err(FlockError::WouldBlock) => {
            // 返回 EWOULDBLOCK
            -11
        }
        Err(FlockError::Conflict(pid)) => {
            crate::println!("flock: lock conflict with process {}", pid);
            -11 // EWOULDBLOCK
        }
        Err(FlockError::Other(msg)) => {
            crate::println!("flock: error: {}", msg);
            -5 // EIO
        }
    }
}

/// fcntl 文件锁 (POSIX 记录锁)
#[derive(Debug, Clone)]
pub struct FlockStruct {
    /// 锁类型: F_RDLCK, F_WRLCK, F_UNLCK
    pub l_type: i16,
    /// 起始偏移量
    pub l_start: i64,
    /// 锁长度 (0 表示到文件末尾)
    pub l_len: i64,
    /// 锁的起始位置: SEEK_SET, SEEK_CUR, SEEK_END
    pub l_whence: i16,
    /// 进程 ID (用于返回信息)
    pub l_pid: ProcessId,
}

/// fcntl 锁命令
pub const F_GETLK: u32 = 5;   // 测试锁
pub const F_SETLK: u32 = 6;   // 设置锁（非阻塞）
pub const F_SETLKW: u32 = 7;  // 设置锁（阻塞）

/// 锁类型
pub const F_RDLCK: i16 = 0;   // 读锁（共享）
pub const F_WRLCK: i16 = 1;   // 写锁（独占）
pub const F_UNLCK: i16 = 2;   // 解锁

/// fcntl 文件锁系统调用
///
/// # 参数
///
/// * `fd`: 文件描述符
/// * `cmd`: 命令 (F_GETLK, F_SETLK, F_SETLKW)
/// * `flock`: 锁信息结构
///
/// # 返回
///
/// 成功返回 0，失败返回错误码
pub fn sys_fcntl_lock(fd: i32, cmd: u32, flock: &mut FlockStruct) -> isize {
    // 获取当前进程
    let current_pid = Process::current().id();

    // 获取文件 inode
    let inode = match get_file_inode_for_fd(fd) {
        Ok(inode) => inode,
        Err(_) => return -9, // EBADF
    };

    let lock_manager = match get_lock_manager() {
        Some(lm) => lm,
        None => return -38, // ENOSYS
    };

    match cmd {
        F_GETLK => {
            // 测试锁：检查是否存在冲突的锁
            let range = LockRange::new(
                flock.l_start as u64,
                if flock.l_len == 0 {
                    u64::MAX
                } else {
                    (flock.l_start + flock.l_len - 1) as u64
                }
            );

            let lock_type = match flock.l_type {
                F_RDLCK => LockType::Shared,
                F_WRLCK => LockType::Exclusive,
                F_UNLCK => return 0, // 解锁不需要测试
                _ => return -22, // EINVAL
            };

            // 查找冲突的锁
            let file_locks = lock_manager.get_file_locks(inode);
            for lock in file_locks {
                if lock.range.overlaps(&range) {
                    match (lock.lock_type, lock_type) {
                        (LockType::Exclusive, _) | (_, LockType::Exclusive) => {
                            // 找到冲突的锁
                            flock.l_type = match lock.lock_type {
                                LockType::Shared => F_RDLCK,
                                LockType::Exclusive => F_WRLCK,
                                LockType::None => F_UNLCK,
                            };
                            flock.l_start = lock.range.start as i64;
                            flock.l_len = if lock.range.end == u64::MAX {
                                0
                            } else {
                                (lock.range.end - lock.range.start + 1) as i64
                            };
                            flock.l_pid = lock.pid;
                            return 0;
                        }
                        (LockType::Shared, LockType::Shared) => {
                            // 共享锁不冲突
                            continue;
                        }
                        _ => {}
                    }
                }
            }

            // 没有冲突的锁
            flock.l_type = F_UNLCK;
            0
        }

        F_SETLK | F_SETLKW => {
            // 设置锁
            let blocking = cmd == F_SETLKW;

            let lock_type = match flock.l_type {
                F_RDLCK => LockType::Shared,
                F_WRLCK => LockType::Exclusive,
                F_UNLCK => {
                    // 释放锁
                    // 注意：这里简化了实现，实际需要跟踪每个进程持有的锁
                    return 0;
                }
                _ => return -22, // EINVAL
            };

            let range = LockRange::new(
                flock.l_start as u64,
                if flock.l_len == 0 {
                    u64::MAX
                } else {
                    (flock.l_start + flock.l_len - 1) as u64
                }
            );

            match lock_type {
                LockType::Shared => {
                    match acquire_shared_lock(inode, current_pid, range, blocking) {
                        Ok(_) => 0,
                        Err(LockError::WouldBlock) => -11, // EWOULDBLOCK
                        Err(_) => -5, // EIO
                    }
                }
                LockType::Exclusive => {
                    match acquire_exclusive_lock(inode, current_pid, range, blocking) {
                        Ok(_) => 0,
                        Err(LockError::WouldBlock) => -11, // EWOULDBLOCK
                        Err(_) => -5, // EIO
                    }
                }
                LockType::None => 0,
            }
        }

        _ => -22, // EINVAL
    }
}

/// 获取文件描述符对应的 inode
fn get_file_inode_for_fd(fd: i32) -> Result<u32, ()> {
    use crate::subsystems::process::FileDescriptor;

    let process = Process::current();
    let fd_table = process.fd_table();
    let _fd_entry = fd_table.get(fd).ok_or(())?;

    // 这里需要根据实际的文件描述符实现来获取 inode
    // 暂时返回一个假的 inode
    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flock_creation() {
        let flock = Flock::new(3, LOCK_SH | LOCK_NB);
        assert_eq!(flock.fd, 3);
        assert!(flock.non_blocking);
    }

    #[test]
    fn test_lock_type_parsing() {
        let flock_shared = Flock::new(0, LOCK_SH);
        assert!(matches!(flock_shared.lock_type, LockType::Shared));

        let flock_exclusive = Flock::new(0, LOCK_EX);
        assert!(matches!(flock_exclusive.lock_type, LockType::Exclusive));

        let flock_unlock = Flock::new(0, LOCK_UN);
        assert!(matches!(flock_unlock.lock_type, LockType::None));
    }
}
