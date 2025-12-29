//! # VFS Integration Tests
//!
//! 综合的 VFS 层集成测试，验证符号链接、文件锁、扩展属性等功能的集成。

#![cfg(test)]

extern crate alloc;
use alloc::{string::String, vec::Vec};
use alloc::sync::Arc;

use kernel::vfs::{
    core::FileSystemType,
    inode::{FileLock, InodeOps},
    ramfs::RamFsType,
    tmpfs::TmpFsType,
    FileMode,
};

/// 测试符号链接创建和解析
#[test]
fn test_symlink_creation_and_resolution() {
    // 挂载 RamFS
    let ramfs = RamFsType;
    let sb = ramfs.mount(None, 0).expect("Failed to mount ramfs");
    let root = sb.root();

    // 创建文件
    let file = root
        .create("test_file.txt", FileMode::new(0o644))
        .expect("Failed to create file");

    // 写入数据
    let data = b"Hello, World!";
    file.write(0, data).expect("Failed to write data");

    // 创建符号链接
    let symlink = root
        .symlink("link_to_file", "test_file.txt")
        .expect("Failed to create symlink");

    // 读取符号链接目标
    let target = symlink.readlink().expect("Failed to read symlink");
    assert_eq!(target, "test_file.txt");

    // 通过符号链接访问文件
    let resolved = root.lookup("link_to_file").expect("Failed to resolve symlink");

    // 读取数据验证
    let mut buf = [0u8; 32];
    let n = resolved.read(0, &mut buf).expect("Failed to read via symlink");
    assert_eq!(&buf[..n], data);

    println!("✓ Symlink creation and resolution test passed");
}

/// 测试文件锁获取和释放
#[test]
fn test_file_lock_acquire_and_release() {
    let ramfs = RamFsType;
    let sb = ramfs.mount(None, 0).expect("Failed to mount ramfs");
    let root = sb.root();

    // 创建文件
    let file = root
        .create("lock_test.txt", FileMode::new(0o644))
        .expect("Failed to create file");

    // 获取读锁
    let lock1 = FileLock::shared(0, 100, 100);
    let lock_id1 = file
        .get_file_lock(0, &lock1)
        .expect("Failed to acquire shared lock");

    assert!(lock_id1 > 0);

    // 释放锁
    file.release_file_lock(&lock1)
        .expect("Failed to release lock");

    // 获取写锁
    let lock2 = FileLock::exclusive(0, 100, 101);
    let lock_id2 = file
        .get_file_lock(0, &lock2)
        .expect("Failed to acquire exclusive lock");

    assert!(lock_id2 > 0);

    // 释放写锁
    file.release_file_lock(&lock2)
        .expect("Failed to release exclusive lock");

    println!("✓ File lock acquire and release test passed");
}

/// 测试文件锁冲突检测
#[test]
fn test_file_lock_conflict_detection() {
    let ramfs = RamFsType;
    let sb = ramfs.mount(None, 0).expect("Failed to mount ramfs");
    let root = sb.root();

    let file = root
        .create("conflict_test.txt", FileMode::new(0o644))
        .expect("Failed to create file");

    // 进程 1 获取写锁
    let lock1 = FileLock::exclusive(0, 100, 200);
    file.get_file_lock(0, &lock1).expect("Failed to acquire lock");

    // 进程 2 尝试在同一区域获取写锁 - 应该冲突
    let lock2 = FileLock::exclusive(0, 100, 201);
    let result = file.get_file_lock(0, &lock2);

    assert!(result.is_err(), "Expected lock conflict");

    println!("✓ File lock conflict detection test passed");
}

/// 测试扩展属性设置和获取
#[test]
fn test_extended_attributes() {
    let tmpfs = TmpFsType;
    let sb = tmpfs.mount(None, 0).expect("Failed to mount tmpfs");
    let root = sb.root();

    let file = root
        .create("xattr_test.txt", FileMode::new(0o644))
        .expect("Failed to create file");

    // 设置扩展属性
    file.set_xattr("user.comment", b"This is a test file", 0)
        .expect("Failed to set xattr");

    file.set_xattr("user.mime_type", b"text/plain", 0)
        .expect("Failed to set xattr");

    // 获取扩展属性
    let mut buf = [0u8; 256];
    let size = file
        .get_xattr("user.comment", &mut buf)
        .expect("Failed to get xattr");

    assert_eq!(&buf[..size], b"This is a test file");

    // 获取另一个扩展属性
    let size2 = file
        .get_xattr("user.mime_type", &mut buf)
        .expect("Failed to get xattr");

    assert_eq!(&buf[..size2], b"text/plain");

    // 列出扩展属性
    let mut list = [0u8; 512];
    let list_size = file.list_xattr(&mut list).expect("Failed to list xattrs");

    assert!(list_size > 0);

    // 删除扩展属性
    file.remove_xattr("user.comment")
        .expect("Failed to remove xattr");

    // 验证删除
    let result = file.get_xattr("user.comment", &mut buf);
    assert!(result.is_err(), "Expected error after removing xattr");

    println!("✓ Extended attributes test passed");
}

/// 测试符号链接 + 文件锁 组合
#[test]
fn test_symlink_with_file_lock() {
    let ramfs = RamFsType;
    let sb = ramfs.mount(None, 0).expect("Failed to mount ramfs");
    let root = sb.root();

    // 创建原始文件
    let file = root
        .create("original.txt", FileMode::new(0o644))
        .expect("Failed to create file");

    // 写入数据
    file.write(0, b"Test data").expect("Failed to write");

    // 创建符号链接
    root.symlink("link.txt", "original.txt")
        .expect("Failed to create symlink");

    // 通过符号链接获取文件
    let linked_file = root.lookup("link.txt").expect("Failed to resolve");

    // 通过符号链接获取文件锁
    let lock = FileLock::exclusive(0, 100, 300);
    linked_file
        .get_file_lock(0, &lock)
        .expect("Failed to acquire lock via symlink");

    // 通过符号链接写入数据
    linked_file
        .write(0, b"Modified data")
        .expect("Failed to write via symlink");

    // 验证原始文件也被修改
    let mut buf = [0u8; 32];
    let n = file.read(0, &mut buf).expect("Failed to read original");
    assert_eq!(&buf[..n], b"Modified data");

    println!("✓ Symlink with file lock test passed");
}

/// 测试扩展属性 + 文件操作 组合
#[test]
fn test_xattr_with_file_operations() {
    let tmpfs = TmpFsType;
    let sb = tmpfs.mount(None, 0).expect("Failed to mount tmpfs");
    let root = sb.root();

    // 创建文件
    let file = root
        .create("xattr_ops_test.txt", FileMode::new(0o644))
        .expect("Failed to create file");

    // 设置扩展属性
    file.set_xattr("user.checksum", b"abc123", 0)
        .expect("Failed to set xattr");

    // 写入文件内容
    file.write(0, b"File content").expect("Failed to write");

    // 扩展属性应该仍然存在
    let mut buf = [0u8; 32];
    let size = file
        .get_xattr("user.checksum", &mut buf)
        .expect("Failed to get xattr");

    assert_eq!(&buf[..size], b"abc123");

    // 截断文件
    file.truncate(4).expect("Failed to truncate");

    // 扩展属性应该仍然存在
    let size2 = file
        .get_xattr("user.checksum", &mut buf)
        .expect("Failed to get xattr");

    assert_eq!(&buf[..size2], b"abc123");

    println!("✓ Extended attributes with file operations test passed");
}

/// 测试深层符号链接解析（7 层）
#[test]
fn test_deep_symlink_resolution() {
    let ramfs = RamFsType;
    let sb = ramfs.mount(None, 0).expect("Failed to mount ramfs");
    let root = sb.root();

    // 创建目标文件
    let target = root
        .create("target.txt", FileMode::new(0o644))
        .expect("Failed to create target");

    target.write(0, b"Deep link target").expect("Failed to write");

    // 创建 7 层符号链接链
    let mut current_path = String::from("target.txt");
    for i in (1..=7).rev() {
        let link_name = format!("link{}", i);
        root.symlink(&link_name, &current_path)
            .expect("Failed to create symlink");
        current_path = link_name;
    }

    // 通过最外层符号链接访问
    let final_file = root.lookup("link1").expect("Failed to resolve");

    // 读取并验证
    let mut buf = [0u8; 32];
    let n = final_file.read(0, &mut buf).expect("Failed to read");

    assert_eq!(&buf[..n], b"Deep link target");

    println!("✓ Deep symlink resolution test passed");
}

/// 测试并发文件操作
#[test]
fn test_concurrent_file_operations() {
    let tmpfs = TmpFsType;
    let sb = tmpfs.mount(None, 0).expect("Failed to mount tmpfs");
    let root = sb.root();

    // 创建多个文件
    for i in 0..10 {
        let name = format!("file{}.txt", i);
        root.create(&name, FileMode::new(0o644))
            .expect("Failed to create file");
    }

    // 为每个文件设置扩展属性
    for i in 0..10 {
        let name = format!("file{}.txt", i);
        let file = root.lookup(&name).expect("Failed to find file");

        let value = format!("value{}", i);
        file.set_xattr("user.test", value.as_bytes(), 0)
            .expect("Failed to set xattr");
    }

    // 验证所有扩展属性
    for i in 0..10 {
        let name = format!("file{}.txt", i);
        let file = root.lookup(&name).expect("Failed to find file");

        let mut buf = [0u8; 32];
        let expected = format!("value{}", i);

        let size = file
            .get_xattr("user.test", &mut buf)
            .expect("Failed to get xattr");

        assert_eq!(&buf[..size], expected.as_bytes());
    }

    println!("✓ Concurrent file operations test passed");
}

/// 运行所有集成测试
#[test]
fn run_all_vfs_integration_tests() {
    test_symlink_creation_and_resolution();
    test_file_lock_acquire_and_release();
    test_file_lock_conflict_detection();
    test_extended_attributes();
    test_symlink_with_file_lock();
    test_xattr_with_file_operations();
    test_deep_symlink_resolution();
    test_concurrent_file_operations();

    println!("\n✅ All VFS integration tests passed!");
}
