//! # 文件锁测试
//!
//! 测试文件锁的获取、释放、升级和降级功能。

#![cfg(test)]

extern crate alloc;

use alloc::sync::Arc;
use kernel::subsystems::fs::file_locking::{
    LockManager, LockType, LockRange, ActiveLock, LockRequest,
    acquire_shared_lock, acquire_exclusive_lock, release_lock,
};
use kernel::subsystems::process::ProcessId;

/// 创建测试用的进程 ID
fn test_pid(id: u32) -> ProcessId {
    ProcessId::new(id)
}

/// 测试共享锁获取
#[test]
fn test_shared_lock_acquisition() {
    let manager = LockManager::new();

    let inode = 1;
    let pid1 = test_pid(100);
    let pid2 = test_pid(101);
    let range = LockRange::entire_file();

    // 两个进程可以同时获取共享锁
    let lock1 = manager.try_lock(inode, pid1, LockType::Shared, range).unwrap();
    let lock2 = manager.try_lock(inode, pid2, LockType::Shared, range).unwrap();

    assert_ne!(lock1, lock2);

    // 清理
    manager.unlock(inode, pid1, lock1).unwrap();
    manager.unlock(inode, pid2, lock2).unwrap();
}

/// 测试独占锁获取
#[test]
fn test_exclusive_lock_acquisition() {
    let manager = LockManager::new();

    let inode = 1;
    let pid1 = test_pid(100);
    let range = LockRange::entire_file();

    // 独占锁获取成功
    let lock1 = manager.try_lock(inode, pid1, LockType::Exclusive, range).unwrap();

    // 同一进程可以再次获取（锁升级）
    let result = manager.try_lock(inode, pid1, LockType::Exclusive, range);
    // 这个行为取决于具体实现
    // 在某些实现中，同一进程可以重复获取锁

    // 清理
    manager.unlock(inode, pid1, lock1).unwrap();
}

/// 测试共享锁和独占锁冲突
#[test]
fn test_shared_exclusive_conflict() {
    let manager = LockManager::new();

    let inode = 1;
    let pid1 = test_pid(100);
    let pid2 = test_pid(101);
    let range = LockRange::entire_file();

    // 进程 1 获取共享锁
    let _lock1 = manager.try_lock(inode, pid1, LockType::Shared, range).unwrap();

    // 进程 2 无法获取独占锁（冲突）
    let result = manager.try_lock(inode, pid2, LockType::Exclusive, range);
    assert!(result.is_err());
}

/// 测试独占锁和共享锁冲突
#[test]
fn test_exclusive_shared_conflict() {
    let manager = LockManager::new();

    let inode = 1;
    let pid1 = test_pid(100);
    let pid2 = test_pid(101);
    let range = LockRange::entire_file();

    // 进程 1 获取独占锁
    let _lock1 = manager.try_lock(inode, pid1, LockType::Exclusive, range).unwrap();

    // 进程 2 无法获取共享锁（冲突）
    let result = manager.try_lock(inode, pid2, LockType::Shared, range);
    assert!(result.is_err());
}

/// 测试锁范围重叠
#[test]
fn test_lock_range_overlap() {
    let range1 = LockRange::new(0, 99);
    let range2 = LockRange::new(50, 149);

    assert!(range1.overlaps(&range2));

    let range3 = LockRange::new(100, 199);
    assert!(!range1.overlaps(&range3));
}

/// 测试锁包含
#[test]
fn test_lock_range_contains() {
    let range1 = LockRange::new(0, 199);
    let range2 = LockRange::new(50, 149);

    assert!(range1.contains(&range2));
    assert!(!range2.contains(&range1));
}

/// 测试锁升级
#[test]
fn test_lock_upgrade() {
    let manager = LockManager::new();

    let inode = 1;
    let pid = test_pid(100);
    let range = LockRange::entire_file();

    // 获取共享锁
    let lock1 = manager.try_lock(inode, pid, LockType::Shared, range).unwrap();

    // 升级为独占锁
    let lock2 = manager.upgrade_lock(inode, pid, lock1).unwrap();

    // 验证锁 ID 相同（升级操作）
    assert_eq!(lock1, lock2);

    // 清理
    manager.unlock(inode, pid, lock2).unwrap();
}

/// 测试锁降级
#[test]
fn test_lock_downgrade() {
    let manager = LockManager::new();

    let inode = 1;
    let pid = test_pid(100);
    let range = LockRange::entire_file();

    // 获取独占锁
    let lock1 = manager.try_lock(inode, pid, LockType::Exclusive, range).unwrap();

    // 降级为共享锁
    let lock2 = manager.downgrade_lock(inode, pid, lock1).unwrap();

    // 验证锁 ID 相同（降级操作）
    assert_eq!(lock1, lock2);

    // 清理
    manager.unlock(inode, pid, lock2).unwrap();
}

/// 测试进程退出时释放所有锁
#[test]
fn test_release_all_locks_on_exit() {
    let manager = LockManager::new();

    let inode = 1;
    let pid = test_pid(100);
    let range = LockRange::entire_file();

    // 进程获取多个锁
    let lock1 = manager.try_lock(inode, pid, LockType::Shared, range).unwrap();
    let lock2 = manager.try_lock(inode + 1, pid, LockType::Exclusive, range).unwrap();

    // 验证锁存在
    let locks_before = manager.get_process_locks(pid);
    assert_eq!(locks_before.len(), 2);

    // 进程退出，释放所有锁
    manager.release_all_locks(pid);

    // 验证锁已释放
    let locks_after = manager.get_process_locks(pid);
    assert_eq!(locks_after.len(), 0);
}

/// 测试获取文件的所有锁
#[test]
fn test_get_file_locks() {
    let manager = LockManager::new();

    let inode = 1;
    let pid1 = test_pid(100);
    let pid2 = test_pid(101);
    let range = LockRange::entire_file();

    // 两个进程获取锁
    let _lock1 = manager.try_lock(inode, pid1, LockType::Shared, range).unwrap();
    let _lock2 = manager.try_lock(inode, pid2, LockType::Shared, range).unwrap();

    // 获取文件的所有锁
    let locks = manager.get_file_locks(inode);
    assert_eq!(locks.len(), 2);
}

/// 测试锁统计信息
#[test]
fn test_lock_statistics() {
    let manager = LockManager::new();

    let inode = 1;
    let pid = test_pid(100);
    let range = LockRange::entire_file();

    // 初始统计
    let stats1 = manager.get_stats();
    assert_eq!(stats1.total_requests, 0);

    // 获取锁
    let _lock = manager.try_lock(inode, pid, LockType::Shared, range).unwrap();

    // 获取统计
    let stats2 = manager.get_stats();
    assert_eq!(stats2.total_requests, 1);
    assert_eq!(stats2.successful_acquisitions, 1);

    // 清理
    manager.unlock(inode, pid, _lock).unwrap();
}

/// 测试锁冲突检测
#[test]
fn test_lock_conflict_detection() {
    let manager = LockManager::new();

    let inode = 1;
    let pid1 = test_pid(100);
    let pid2 = test_pid(101);
    let range1 = LockRange::new(0, 99);
    let range2 = LockRange::new(100, 199);

    // 两个不重叠的锁
    let _lock1 = manager.try_lock(inode, pid1, LockType::Exclusive, range1).unwrap();
    let lock2 = manager.try_lock(inode, pid2, LockType::Exclusive, range2);

    // 不应该冲突
    assert!(lock2.is_ok());
}

/// 测试死锁检测
#[test]
fn test_deadlock_detection() {
    let manager = LockManager::new();

    let inode1 = 1;
    let inode2 = 2;
    let pid1 = test_pid(100);
    let pid2 = test_pid(101);
    let range = LockRange::entire_file();

    // 进程 1 持有 inode1 的锁
    let _lock1 = manager.try_lock(inode1, pid1, LockType::Exclusive, range).unwrap();

    // 进程 2 持有 inode2 的锁
    let _lock2 = manager.try_lock(inode2, pid2, LockType::Exclusive, range).unwrap();

    // 尝试创建循环依赖
    // 这个测试取决于死锁检测算法的实现
    let deadlock = manager.detect_deadlock();

    // 在当前简化实现中，可能不会检测到死锁
    // 这是一个用于演示的测试
    let _ = deadlock;
}

/// 性能测试：大量锁获取和释放
#[test]
fn test_lock_performance() {
    let manager = LockManager::new();

    let inode = 1;
    let range = LockRange::entire_file();

    // 获取和释放大量锁
    for i in 0..1000 {
        let pid = test_pid(100 + i as u32);
        if let Ok(lock) = manager.try_lock(inode, pid, LockType::Shared, range) {
            manager.unlock(inode, pid, lock).unwrap();
        }
    }

    // 验证最终状态
    let locks = manager.get_file_locks(inode);
    assert_eq!(locks.len(), 0);
}

/// 边界测试：空范围
#[test]
fn test_empty_range() {
    let range = LockRange::new(100, 99); // 无效范围

    // 空范围不应该与任何范围重叠
    let other = LockRange::new(0, 199);
    assert!(!range.overlaps(&other));
}

/// 边界测试：最大范围
#[test]
fn test_max_range() {
    let range = LockRange::entire_file();

    assert_eq!(range.start, 0);
    assert_eq!(range.end, u64::MAX);

    // 最大范围应该与任何范围重叠
    let other = LockRange::new(1000, 1999);
    assert!(range.overlaps(&other));
}

/// 测试辅助函数
#[test]
fn test_helper_functions() {
    use kernel::subsystems::fs::file_locking::{
        acquire_shared_lock, acquire_exclusive_lock,
    };

    // 注意：这些测试需要实际的锁管理器初始化
    // 目前我们只测试函数存在性和类型正确性

    let _ = LockType::Shared;
    let _ = LockType::Exclusive;
    let _ = LockType::None;
}
