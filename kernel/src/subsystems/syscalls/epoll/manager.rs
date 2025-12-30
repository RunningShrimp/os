//! GLib epoll manager trait and implementation

use super::*;
use crate::syscalls::common::SyscallError;

/// Helper function to convert errno to SyscallError
fn errno_to_syscall_error(errno: i32) -> SyscallError {
    match errno {
        1 => SyscallError::InvalidArgument,
        2 => SyscallError::BadFileDescriptor,
        9 => SyscallError::BadFileDescriptor,
        11 => SyscallError::WouldBlock,
        12 => SyscallError::WouldBlock,
        13 => SyscallError::PermissionDenied,
        14 => SyscallError::InvalidArgument,
        16 => SyscallError::WouldBlock,
        22 => SyscallError::InvalidArgument,
        32 => SyscallError::WouldBlock,
        110 => SyscallError::WouldBlock,
        _ => SyscallError::NotImplemented,
    }
}

/// GLib事件循环管理器特征
pub trait GLibEpollManager {
    /// 创建新的epoll实例
    fn create_epoll_instance(&mut self) -> Result<c_int, EpollError>;

    /// 添加事件源
    fn add_event_source(&mut self, epfd: c_int, fd: c_int, events: u32) -> Result<(), SyscallError>;

    /// 移除事件源
    fn remove_event_source(&mut self, epfd: c_int, fd: c_int) -> Result<(), SyscallError>;

    /// 等待事件
    fn wait_events(
        &mut self,
        epfd: c_int,
        events: &mut [instance::EpollEvent],
        timeout: c_int,
    ) -> Result<usize, SyscallError>;

    /// 关闭epoll实例
    fn close_epoll_instance(&mut self, epfd: c_int) -> Result<(), SyscallError>;

    /// 获取实例统计
    fn get_instance_stats(&self, epfd: c_int) -> Result<GLibEpollInstance, c_int>;
}

// Note: GLibEpollManager is a trait, not a concrete type, so Default is not implemented

impl GLibEpollManager for () {
    fn create_epoll_instance(&mut self) -> Result<c_int, EpollError> {
        let result = super::instance::sys_glib_epoll_create();
        if result > 0 {
            Ok(result)
        } else {
            Err(EpollError::DeviceError)
        }
    }

    fn add_event_source(&mut self, epfd: c_int, fd: c_int, events: u32) -> Result<(), SyscallError> {
        let result = super::instance::sys_glib_epoll_add_source(epfd, fd, events);
        if result == 0 {
            Ok(())
        } else {
            Err(errno_to_syscall_error(result))
        }
    }

    fn remove_event_source(&mut self, epfd: c_int, fd: c_int) -> Result<(), SyscallError> {
        let result = super::instance::sys_glib_epoll_remove_source(epfd, fd);
        if result == 0 {
            Ok(())
        } else {
            Err(errno_to_syscall_error(result))
        }
    }

    fn wait_events(
        &mut self,
        epfd: c_int,
        events: &mut [instance::EpollEvent],
        timeout: c_int,
    ) -> Result<usize, SyscallError> {
        let maxevents = events.len() as c_int;
        let result = super::instance::sys_glib_epoll_wait(
            epfd,
            events.as_mut_ptr(),
            maxevents,
            timeout,
        );
        if result >= 0 {
            Ok(result as usize)
        } else {
            Err(errno_to_syscall_error(result))
        }
    }

    fn close_epoll_instance(&mut self, epfd: c_int) -> Result<(), SyscallError> {
        let result = super::instance::sys_glib_epoll_close(epfd);
        if result == 0 {
            Ok(())
        } else {
            Err(errno_to_syscall_error(result))
        }
    }

    fn get_instance_stats(&self, epfd: c_int) -> Result<GLibEpollInstance, c_int> {
        let mut instance = GLibEpollInstance {
            epfd: 0,
            source_count: AtomicUsize::new(0),
            max_sources: 0,
            created_timestamp: 0,
            total_waits: AtomicUsize::new(0),
            total_events: AtomicUsize::new(0),
        };

        let result = super::instance::sys_glib_epoll_stats(
            epfd,
            &mut instance as *mut GLibEpollInstance,
        );

        if result == 0 {
            Ok(instance)
        } else {
            Err(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoll_creation() {
        // 测试epoll实例创建
        let epfd = super::instance::sys_glib_epoll_create();
        assert!(epfd > 0);

        // 清理
        super::instance::sys_glib_epoll_close(epfd);
    }

    #[test]
    fn test_event_source_management() {
        // 创建epoll实例
        let epfd = super::instance::sys_glib_epoll_create();
        assert!(epfd > 0);

        // 添加事件源
        let fd = 1; // 假设的文件描述符
        let events = EPOLLIN | EPOLLOUT;
        let result = super::instance::sys_glib_epoll_add_source(epfd, fd, events);
        assert_eq!(result, 0);

        // 移除事件源
        let result = super::instance::sys_glib_epoll_remove_source(epfd, fd);
        assert_eq!(result, 0);

        // 清理
        super::instance::sys_glib_epoll_close(epfd);
    }

    #[test]
    fn test_epoll_stats() {
        // 创建epoll实例
        let epfd = super::instance::sys_glib_epoll_create();
        assert!(epfd > 0);

        // 获取统计信息
        let mut stats = GLibEpollInstance {
            epfd: 0,
            source_count: AtomicUsize::new(0),
            max_sources: 0,
            created_timestamp: 0,
            total_waits: AtomicUsize::new(0),
            total_events: AtomicUsize::new(0),
        };

        let result =
            super::instance::sys_glib_epoll_stats(epfd, &mut stats as *mut GLibEpollInstance);
        assert_eq!(result, 0);
        assert_eq!(stats.epfd, epfd);
        assert_eq!(stats.source_count.load(Ordering::SeqCst), 0);

        // 清理
        super::instance::sys_glib_epoll_close(epfd);
    }
}
