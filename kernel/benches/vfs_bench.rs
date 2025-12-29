//! # VFS Performance Benchmarks
//!
//! 测试 VFS 层的性能特征，包括符号链接解析、文件锁操作和扩展属性访问。

#![cfg(feature = "benchmark")]

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

/// 基准测试：符号链接解析性能
pub fn benchmark_symlink_resolution(iterations: usize) {
    let ramfs = RamFsType;
    let sb = ramfs.mount(None, 0).expect("Failed to mount ramfs");
    let root = sb.root();

    // 创建目标文件
    let target = root
        .create("target.txt", FileMode::new(0o644))
        .expect("Failed to create target");

    target.write(0, b"Target data").expect("Failed to write");

    // 创建符号链接
    root.symlink("link.txt", "target.txt")
        .expect("Failed to create symlink");

    // 基准测试：重复解析符号链接
    let start = crate::subsystems::time::get_timestamp();

    for _ in 0..iterations {
        let file = root.lookup("link.txt").expect("Failed to resolve");
        let mut buf = [0u8; 32];
        file.read(0, &mut buf).expect("Failed to read");
    }

    let end = crate::subsystems::time::get_timestamp();
    let duration = end - start;
    let avg_time = duration / iterations as u64;

    crate::println!(
        "[Benchmark] Symlink resolution: {} iterations, {} total time, {} avg per operation",
        iterations,
        duration,
        avg_time
    );
}

/// 基准测试：文件锁获取/释放性能
pub fn benchmark_file_locks(iterations: usize) {
    let ramfs = RamFsType;
    let sb = ramfs.mount(None, 0).expect("Failed to mount ramfs");
    let root = sb.root();

    let file = root
        .create("lock_bench.txt", FileMode::new(0o644))
        .expect("Failed to create file");

    // 基准测试：重复获取和释放锁
    let start = crate::subsystems::time::get_timestamp();

    for i in 0..iterations {
        let lock = FileLock::exclusive(0, 100, i as u32);
        file.get_file_lock(0, &lock)
            .expect("Failed to acquire lock");
        file.release_file_lock(&lock)
            .expect("Failed to release lock");
    }

    let end = crate::subsystems::time::get_timestamp();
    let duration = end - start;
    let avg_time = duration / iterations as u64;

    crate::println!(
        "[Benchmark] File lock acquire/release: {} iterations, {} total time, {} avg per operation",
        iterations,
        duration,
        avg_time
    );
}

/// 基准测试：扩展属性读写性能
pub fn benchmark_xattr_operations(iterations: usize) {
    let tmpfs = TmpFsType;
    let sb = tmpfs.mount(None, 0).expect("Failed to mount tmpfs");
    let root = sb.root();

    let file = root
        .create("xattr_bench.txt", FileMode::new(0o644))
        .expect("Failed to create file");

    // 基准测试：重复设置和获取扩展属性
    let start = crate::subsystems::time::get_timestamp();

    for i in 0..iterations {
        let name = format!("user.attr{}", i);
        let value = format!("value{}", i);

        file.set_xattr(&name, value.as_bytes(), 0)
            .expect("Failed to set xattr");

        let mut buf = [0u8; 256];
        file.get_xattr(&name, &mut buf)
            .expect("Failed to get xattr");
    }

    let end = crate::subsystems::time::get_timestamp();
    let duration = end - start;
    let avg_time = duration / iterations as u64;

    crate::println!(
        "[Benchmark] XAttr set/get: {} iterations, {} total time, {} avg per operation",
        iterations,
        duration,
        avg_time
    );
}

/// 基准测试：并发文件操作吞吐量
pub fn benchmark_concurrent_operations(iterations: usize) {
    let tmpfs = TmpFsType;
    let sb = tmpfs.mount(None, 0).expect("Failed to mount tmpfs");
    let root = sb.root();

    // 创建多个文件
    let num_files = 10;
    for i in 0..num_files {
        let name = format!("file{}.txt", i);
        root.create(&name, FileMode::new(0o644))
            .expect("Failed to create file");
    }

    // 基准测试：并发操作
    let start = crate::subsystems::time::get_timestamp();

    for i in 0..iterations {
        let file_idx = i % num_files;
        let name = format!("file{}.txt", file_idx);

        if let Ok(file) = root.lookup(&name) {
            // 设置扩展属性
            let attr_name = format!("user.attr{}", i);
            file.set_xattr(&attr_name, b"test value", 0)
                .expect("Failed to set xattr");

            // 读写数据
            let mut buf = [0u8; 128];
            file.write(0, b"data").expect("Failed to write");
            file.read(0, &mut buf).expect("Failed to read");

            // 获取扩展属性
            file.get_xattr(&attr_name, &mut buf)
                .expect("Failed to get xattr");
        }
    }

    let end = crate::subsystems::time::get_timestamp();
    let duration = end - start;
    let throughput = iterations as u64 * 1000 / duration.max(1);

    crate::println!(
        "[Benchmark] Concurrent operations: {} iterations, {} total time, {} ops/sec",
        iterations,
        duration,
        throughput
    );
}

/// 基准测试：路径解析性能
pub fn benchmark_path_resolution(iterations: usize) {
    let ramfs = RamFsType;
    let sb = ramfs.mount(None, 0).expect("Failed to mount ramfs");
    let root = sb.root();

    // 创建深层目录结构
    let mut current = root.clone();
    for i in 0..10 {
        let name = format!("dir{}", i);
        let attr = current.getattr().expect("Failed to get attr");
        if attr.mode.is_dir() {
            if let Ok(new_dir) = current.mkdir(&name, FileMode::new(0o755)) {
                current = new_dir;
            }
        }
    }

    // 创建目标文件
    current
        .create("deep_file.txt", FileMode::new(0o644))
        .expect("Failed to create file");

    // 基准测试：重复查找深层路径
    let start = crate::subsystems::time::get_timestamp();

    for _ in 0..iterations {
        // 模拟路径查找（简化版）
        let mut lookup_root = root.clone();
        for i in 0..10 {
            let name = format!("dir{}", i);
            if let Ok(dir) = lookup_root.lookup(&name) {
                lookup_root = dir;
            }
        }
        let _ = lookup_root.lookup("deep_file.txt");
    }

    let end = crate::subsystems::time::get_timestamp();
    let duration = end - start;
    let avg_time = duration / iterations as u64;

    crate::println!(
        "[Benchmark] Path resolution (10 deep): {} iterations, {} total time, {} avg per lookup",
        iterations,
        duration,
        avg_time
    );
}

/// 基准测试：文件创建和删除性能
pub fn benchmark_file_create_delete(iterations: usize) {
    let tmpfs = TmpFsType;
    let sb = tmpfs.mount(None, 0).expect("Failed to mount tmpfs");
    let root = sb.root();

    // 基准测试：创建和删除文件
    let start = crate::subsystems::time::get_timestamp();

    for i in 0..iterations {
        let name = format!("temp{}.txt", i);

        // 创建文件
        let file = root
            .create(&name, FileMode::new(0o644))
            .expect("Failed to create file");

        // 写入数据
        file.write(0, b"test data").expect("Failed to write");

        // 删除文件
        root.unlink(&name).expect("Failed to delete file");
    }

    let end = crate::subsystems::time::get_timestamp();
    let duration = end - start;
    let avg_time = duration / iterations as u64;

    crate::println!(
        "[Benchmark] File create/delete: {} iterations, {} total time, {} avg per operation",
        iterations,
        duration,
        avg_time
    );
}

/// 运行所有基准测试
pub fn run_all_benchmarks() {
    crate::println!("\n=== VFS Performance Benchmarks ===\n");

    benchmark_symlink_resolution(1000);
    benchmark_file_locks(1000);
    benchmark_xattr_operations(1000);
    benchmark_concurrent_operations(1000);
    benchmark_path_resolution(1000);
    benchmark_file_create_delete(1000);

    crate::println!("\n✅ All benchmarks completed!");
}

/// 性能测试主入口
#[cfg(test)]
mod bench_tests {
    use super::*;

    #[test]
    fn bench_symlink_resolution() {
        benchmark_symlink_resolution(100);
    }

    #[test]
    fn bench_file_locks() {
        benchmark_file_locks(100);
    }

    #[test]
    fn bench_xattr_operations() {
        benchmark_xattr_operations(100);
    }

    #[test]
    fn bench_concurrent_operations() {
        benchmark_concurrent_operations(100);
    }
}
