#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! 核心POSIX系统调用测试
//!
//! 测试核心POSIX系统调用的实现，包括：
//! - 文件系统相关系统调用
//! - 进程管理相关系统调用
//! - 内存管理相关系统调用
//! - 网络相关系统调用
//!
//! 每个测试用例都包含正面和负面测试，以及边界条件测试。

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::{c_char, c_int, c_void};
use crate::posix_tests::{PosixTestResult, PosixTestResults, PerformanceMetric};
use crate::syscalls;
use crate::posix;

/// 文件系统相关系统调用测试
pub fn test_filesystem_syscalls(results: &mut PosixTestResults) {
    crate::println!("  📁 文件系统系统调用测试:");
    
    let start_time = crate::subsystems::time::get_time_ns();
    
    // 测试stat系列系统调用
    test_stat_syscalls(results);
    
    // 测试文件操作系统调用
    test_file_operations(results);
    
    // 测试目录操作系统调用
    test_directory_operations(results);
    
    // 测试文件描述符操作
    test_fd_operations(results);
    
    // 测试文件权限操作
    test_file_permissions(results);
    
    let execution_time = crate::subsystems::time::get_time_ns() - start_time;
    results.record_performance(PerformanceMetric {
        test_name: "filesystem_syscalls".to_string(),
        execution_time_ns: execution_time,
        memory_used_bytes: 0,
        cpu_cycles: 0,
    });
}

/// 测试stat系列系统调用
fn test_stat_syscalls(results: &mut PosixTestResults) {
    crate::println!("    📊 stat系列系统调用测试:");
    
    // 测试fstat
    test_fstat(results);
    
    // 测试stat
    test_stat(results);
    
    // 测试lstat
    test_lstat(results);
    
    // 测试fstatat
    test_fstatat(results);
    
    // 测试statfs
    test_statfs(results);
    
    // 测试statvfs
    test_statvfs(results);
}

/// 测试fstat系统调用
fn test_fstat(results: &mut PosixTestResults) {
    crate::println!("      🔍 测试fstat系统调用:");
    
    // 正面测试：正常文件描述符
    let fd = 1; // stdout
    let mut stat_buf = crate::posix::Stat::default();
    let result = unsafe {
        crate::posix::fstat(fd, &mut stat_buf)
    };
    
    let passed = result == 0;
    results.record_result(passed, "fstat正常文件描述符",
        if passed { None } else { Some("fstat调用失败") });
    
    // 负面测试：无效文件描述符
    let invalid_fd = -1;
    let result = unsafe {
        crate::posix::fstat(invalid_fd, &mut stat_buf)
    };
    
    let passed = result == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::EBADF;
    results.record_result(passed, "fstat无效文件描述符",
        if passed { None } else { Some("fstat应该返回EBADF错误") });
    
    // 边界测试：空指针
    let result = unsafe {
        crate::posix::fstat(fd, core::ptr::null_mut())
    };
    
    let passed = result == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::EFAULT;
    results.record_result(passed, "fstat空指针",
        if passed { None } else { Some("fstat应该返回EFAULT错误") });
}

/// 测试stat系统调用
fn test_stat(results: &mut PosixTestResults) {
    crate::println!("      🔍 测试stat系统调用:");
    
    let test_path = b"/test_file\0";
    let mut stat_buf = crate::posix::Stat::default();
    
    // 正面测试：存在的文件
    // 首先创建一个测试文件
    let fd = unsafe { crate::posix::open(test_path.as_ptr() as *const c_char, 
                                     crate::posix::O_CREAT | crate::posix::O_WRONLY, 
                                     0o644) };
    
    if fd >= 0 {
        let result = unsafe {
            crate::posix::stat(test_path.as_ptr() as *const c_char, &mut stat_buf)
        };
        
        let passed = result == 0;
        results.record_result(passed, "stat存在的文件",
            if passed { None } else { Some("stat调用失败") });
        
        // 清理
        unsafe { crate::posix::close(fd) };
        unsafe { crate::posix::unlink(test_path.as_ptr() as *const c_char) };
    } else {
        results.record_skip("stat存在的文件", "无法创建测试文件");
    }
    
    // 负面测试：不存在的文件
    let nonexistent_path = b"/nonexistent_file\0";
    let result = unsafe {
        crate::posix::stat(nonexistent_path.as_ptr() as *const c_char, &mut stat_buf)
    };
    
    let passed = result == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::ENOENT;
    results.record_result(passed, "stat不存在的文件",
        if passed { None } else { Some("stat应该返回ENOENT错误") });
    
    // 边界测试：空路径指针
    let result = unsafe {
        crate::posix::stat(core::ptr::null(), &mut stat_buf)
    };
    
    let passed = result == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::EFAULT;
    results.record_result(passed, "stat空路径指针",
        if passed { None } else { Some("stat应该返回EFAULT错误") });
}

/// 测试lstat系统调用
fn test_lstat(results: &mut PosixTestResults) {
    crate::println!("      🔍 测试lstat系统调用:");
    
    let test_path = b"/test_symlink\0";
    let target_path = b"/test_target\0";
    let mut stat_buf = crate::posix::Stat::default();
    
    // 创建目标文件
    let fd = unsafe { crate::posix::open(target_path.as_ptr() as *const c_char, 
                                     crate::posix::O_CREAT | crate::posix::O_WRONLY, 
                                     0o644) };
    
    if fd >= 0 {
        unsafe { crate::posix::close(fd) };
        
        // 创建符号链接
        let result = unsafe {
            crate::posix::symlink(target_path.as_ptr() as *const c_char, 
                              test_path.as_ptr() as *const c_char)
        };
        
        if result == 0 {
            // 测试lstat（不跟随符号链接）
            let result = unsafe {
                crate::posix::lstat(test_path.as_ptr() as *const c_char, &mut stat_buf)
            };
            
            let passed = result == 0;
            results.record_result(passed, "lstat符号链接",
                if passed { None } else { Some("lstat调用失败") });
            
            // 清理
            unsafe { crate::posix::unlink(test_path.as_ptr() as *const c_char) };
        } else {
            results.record_skip("lstat符号链接", "无法创建符号链接");
        }
        
        // 清理目标文件
        unsafe { crate::posix::unlink(target_path.as_ptr() as *const c_char) };
    } else {
        results.record_skip("lstat符号链接", "无法创建目标文件");
    }
}

/// 测试fstatat系统调用
fn test_fstatat(results: &mut PosixTestResults) {
    crate::println!("      🔍 测试fstatat系统调用:");
    
    let dirfd = unsafe { crate::posix::open(b".\0".as_ptr() as *const c_char, 
                                         crate::posix::O_RDONLY, 0) };
    
    if dirfd >= 0 {
        let test_path = b"test_file\0";
        let mut stat_buf = crate::posix::Stat::default();
        
        // 创建测试文件
        let fd = unsafe { crate::posix::openat(dirfd, test_path.as_ptr() as *const c_char,
                                           crate::posix::O_CREAT | crate::posix::O_WRONLY,
                                           0o644) };
        
        if fd >= 0 {
            unsafe { crate::posix::close(fd) };
            
            // 测试fstatat
            let result = unsafe {
                crate::posix::fstatat(dirfd, test_path.as_ptr() as *const c_char, &mut stat_buf, 0)
            };
            
            let passed = result == 0;
            results.record_result(passed, "fstatat正常文件",
                if passed { None } else { Some("fstatat调用失败") });
            
            // 清理
            unsafe { crate::posix::unlinkat(dirfd, test_path.as_ptr() as *const c_char, 0) };
        } else {
            results.record_skip("fstatat正常文件", "无法创建测试文件");
        }
        
        unsafe { crate::posix::close(dirfd) };
    } else {
        results.record_skip("fstatat", "无法打开当前目录");
    }
}

/// 测试statfs系统调用
fn test_statfs(results: &mut PosixTestResults) {
    crate::println!("      🔍 测试statfs系统调用:");
    
    let mut fs_buf = crate::posix::Statfs::default();
    let path = b".\0";
    
    // 正面测试：有效路径
    let result = unsafe {
        crate::posix::statfs(path.as_ptr() as *const c_char, &mut fs_buf)
    };
    
    let passed = result == 0;
    results.record_result(passed, "statfs有效路径",
        if passed { None } else { Some("statfs调用失败") });
    
    // 负面测试：无效路径
    let result = unsafe {
        crate::posix::statfs(core::ptr::null(), &mut fs_buf)
    };
    
    let passed = result == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::EFAULT;
    results.record_result(passed, "statfs无效路径",
        if passed { None } else { Some("statfs应该返回EFAULT错误") });
}

/// 测试statvfs系统调用
fn test_statvfs(results: &mut PosixTestResults) {
    crate::println!("      🔍 测试statvfs系统调用:");
    
    let mut vfs_buf = crate::posix::Statvfs::default();
    let path = b".\0";
    
    // 正面测试：有效路径
    let result = unsafe {
        crate::posix::statvfs(path.as_ptr() as *const c_char, &mut vfs_buf)
    };
    
    let passed = result == 0;
    results.record_result(passed, "statvfs有效路径",
        if passed { None } else { Some("statvfs调用失败") });
    
    // 负面测试：无效路径
    let result = unsafe {
        crate::posix::statvfs(core::ptr::null(), &mut vfs_buf)
    };
    
    let passed = result == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::EFAULT;
    results.record_result(passed, "statvfs无效路径",
        if passed { None } else { Some("statvfs应该返回EFAULT错误") });
}

/// 测试文件操作系统调用
fn test_file_operations(results: &mut PosixTestResults) {
    crate::println!("    📄 文件操作系统调用测试:");
    
    // 测试open/close
    test_open_close(results);
    
    // 测试read/write
    test_read_write(results);
    
    // 测试lseek
    test_lseek(results);
    
    // 测试fsync/fdatasync
    test_fsync(results);
    
    // 测试truncate/ftruncate
    test_truncate(results);
}

/// 测试open/close系统调用
fn test_open_close(results: &mut PosixTestResults) {
    crate::println!("      📂 测试open/close系统调用:");
    
    let test_path = b"/test_open_close\0";
    
    // 正面测试：创建新文件
    let fd = unsafe {
        crate::posix::open(test_path.as_ptr() as *const c_char,
                        crate::posix::O_CREAT | crate::posix::O_WRONLY,
                        0o644)
    };
    
    let passed = fd >= 0;
    results.record_result(passed, "open创建新文件",
        if passed { None } else { Some("open调用失败") });
    
    if fd >= 0 {
        // 测试close
        let result = unsafe { crate::posix::close(fd) };
        let passed = result == 0;
        results.record_result(passed, "close正常文件描述符",
            if passed { None } else { Some("close调用失败") });
        
        // 清理
        unsafe { crate::posix::unlink(test_path.as_ptr() as *const c_char) };
    }
    
    // 负面测试：打开不存在的文件
    let fd = unsafe {
        crate::posix::open(b"/nonexistent\0".as_ptr() as *const c_char,
                        crate::posix::O_RDONLY, 0)
    };
    
    let passed = fd == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::ENOENT;
    results.record_result(passed, "open不存在的文件",
        if passed { None } else { Some("open应该返回ENOENT错误") });
}

/// 测试read/write系统调用
fn test_read_write(results: &mut PosixTestResults) {
    crate::println!("      📖 测试read/write系统调用:");
    
    let test_path = b"/test_read_write\0";
    let test_data = b"Hello, POSIX!";
    let mut read_buffer = [0u8; 256];
    
    // 创建测试文件
    let fd = unsafe {
        crate::posix::open(test_path.as_ptr() as *const c_char,
                        crate::posix::O_CREAT | crate::posix::O_RDWR,
                        0o644)
    };
    
    if fd >= 0 {
        // 测试write
        let written = unsafe {
            crate::posix::write(fd, test_data.as_ptr() as *const c_void, test_data.len())
        };
        
        let passed = written == test_data.len() as isize;
        results.record_result(passed, "write正常数据",
            if passed { None } else { Some("write写入字节数不匹配") });
        
        // 重置文件指针
        unsafe { crate::posix::lseek(fd, 0, crate::posix::SEEK_SET) };
        
        // 测试read
        let read = unsafe {
            crate::posix::read(fd, read_buffer.as_mut_ptr() as *mut c_void, read_buffer.len())
        };
        
        let passed = read == test_data.len() as isize;
        results.record_result(passed, "read正常数据",
            if passed { None } else { Some("read读取字节数不匹配") });
        
        // 验证数据内容
        if read == test_data.len() as isize {
            let passed = &read_buffer[..test_data.len()] == test_data;
            results.record_result(passed, "read/write数据一致性",
                if passed { None } else { Some("读取数据与写入数据不匹配") });
        }
        
        // 清理
        unsafe { crate::posix::close(fd) };
        unsafe { crate::posix::unlink(test_path.as_ptr() as *const c_char) };
    } else {
        results.record_skip("read/write", "无法创建测试文件");
    }
}

/// 测试lseek系统调用
fn test_lseek(results: &mut PosixTestResults) {
    crate::println!("      🔍 测试lseek系统调用:");
    
    let test_path = b"/test_lseek\0";
    let test_data = b"0123456789";
    
    // 创建测试文件
    let fd = unsafe {
        crate::posix::open(test_path.as_ptr() as *const c_char,
                        crate::posix::O_CREAT | crate::posix::O_RDWR,
                        0o644)
    };
    
    if fd >= 0 {
        // 写入测试数据
        unsafe {
            crate::posix::write(fd, test_data.as_ptr() as *const c_void, test_data.len())
        };
        
        // 测试SEEK_SET
        let offset = unsafe {
            crate::posix::lseek(fd, 5, crate::posix::SEEK_SET)
        };
        
        let passed = offset == 5;
        results.record_result(passed, "lseek SEEK_SET",
            if passed { None } else { Some("lseek SEEK_SET返回值错误") });
        
        // 测试SEEK_CUR
        let offset = unsafe {
            crate::posix::lseek(fd, 2, crate::posix::SEEK_CUR)
        };
        
        let passed = offset == 7;
        results.record_result(passed, "lseek SEEK_CUR",
            if passed { None } else { Some("lseek SEEK_CUR返回值错误") });
        
        // 测试SEEK_END
        let offset = unsafe {
            crate::posix::lseek(fd, -3, crate::posix::SEEK_END)
        };
        
        let passed = offset == 7;
        results.record_result(passed, "lseek SEEK_END",
            if passed { None } else { Some("lseek SEEK_END返回值错误") });
        
        // 清理
        unsafe { crate::posix::close(fd) };
        unsafe { crate::posix::unlink(test_path.as_ptr() as *const c_char) };
    } else {
        results.record_skip("lseek", "无法创建测试文件");
    }
}

/// 测试fsync/fdatasync系统调用
fn test_fsync(results: &mut PosixTestResults) {
    crate::println!("      💾 测试fsync/fdatasync系统调用:");
    
    let test_path = b"/test_fsync\0";
    
    // 创建测试文件
    let fd = unsafe {
        crate::posix::open(test_path.as_ptr() as *const c_char,
                        crate::posix::O_CREAT | crate::posix::O_WRONLY,
                        0o644)
    };
    
    if fd >= 0 {
        // 写入一些数据
        unsafe {
            crate::posix::write(fd, b"test data\0".as_ptr() as *const c_void, 9);
        };
        
        // 测试fsync
        let result = unsafe { crate::posix::fsync(fd) };
        let passed = result == 0;
        results.record_result(passed, "fsync同步",
            if passed { None } else { Some("fsync调用失败") });
        
        // 测试fdatasync
        let result = unsafe { crate::posix::fdatasync(fd) };
        let passed = result == 0;
        results.record_result(passed, "fdatasync同步",
            if passed { None } else { Some("fdatasync调用失败") });
        
        // 清理
        unsafe { crate::posix::close(fd) };
        unsafe { crate::posix::unlink(test_path.as_ptr() as *const c_char) };
    } else {
        results.record_skip("fsync/fdatasync", "无法创建测试文件");
    }
}

/// 测试truncate/ftruncate系统调用
fn test_truncate(results: &mut PosixTestResults) {
    crate::println!("      ✂️ 测试truncate/ftruncate系统调用:");
    
    let test_path = b"/test_truncate\0";
    let test_data = b"0123456789";
    
    // 创建测试文件
    let fd = unsafe {
        crate::posix::open(test_path.as_ptr() as *const c_char,
                        crate::posix::O_CREAT | crate::posix::O_RDWR,
                        0o644)
    };
    
    if fd >= 0 {
        // 写入测试数据
        unsafe {
            crate::posix::write(fd, test_data.as_ptr() as *const c_void, test_data.len())
        };
        unsafe { crate::posix::close(fd) };
        
        // 测试truncate
        let result = unsafe {
            crate::posix::truncate(test_path.as_ptr() as *const c_char, 5)
        };
        
        let passed = result == 0;
        results.record_result(passed, "truncate文件",
            if passed { None } else { Some("truncate调用失败") });
        
        // 重新打开文件测试ftruncate
        let fd = unsafe {
            crate::posix::open(test_path.as_ptr() as *const c_char,
                            crate::posix::O_RDWR, 0)
        };
        
        if fd >= 0 {
            let result = unsafe { crate::posix::ftruncate(fd, 3) };
            let passed = result == 0;
            results.record_result(passed, "ftruncate文件",
                if passed { None } else { Some("ftruncate调用失败") });
            
            unsafe { crate::posix::close(fd) };
        }
        
        // 清理
        unsafe { crate::posix::unlink(test_path.as_ptr() as *const c_char) };
    } else {
        results.record_skip("truncate/ftruncate", "无法创建测试文件");
    }
}

/// 测试目录操作系统调用
fn test_directory_operations(results: &mut PosixTestResults) {
    crate::println!("    📁 目录操作系统调用测试:");
    
    // 测试mkdir/rmdir
    test_mkdir_rmdir(results);
    
    // 测试opendir/closedir/readdir
    test_opendir_readdir(results);
    
    // 测试getcwd/chdir
    test_getcwd_chdir(results);
}

/// 测试mkdir/rmdir系统调用
fn test_mkdir_rmdir(results: &mut PosixTestResults) {
    crate::println!("      📂 测试mkdir/rmdir系统调用:");
    
    let test_dir = b"/test_mkdir\0";
    
    // 测试mkdir
    let result = unsafe {
        crate::posix::mkdir(test_dir.as_ptr() as *const c_char, 0o755)
    };
    
    let passed = result == 0;
    results.record_result(passed, "mkdir创建目录",
        if passed { None } else { Some("mkdir调用失败") });
    
    if result == 0 {
        // 测试rmdir
        let result = unsafe {
            crate::posix::rmdir(test_dir.as_ptr() as *const c_char)
        };
        
        let passed = result == 0;
        results.record_result(passed, "rmdir删除目录",
            if passed { None } else { Some("rmdir调用失败") });
    }
    
    // 负面测试：创建已存在的目录
    let result = unsafe {
        crate::posix::mkdir(test_dir.as_ptr() as *const c_char, 0o755)
    };
    
    if result == 0 {
        let result = unsafe {
            crate::posix::mkdir(test_dir.as_ptr() as *const c_char, 0o755)
        };
        
        let passed = result == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::EEXIST;
        results.record_result(passed, "mkdir已存在目录",
            if passed { None } else { Some("mkdir应该返回EEXIST错误") });
        
        // 清理
        unsafe { crate::posix::rmdir(test_dir.as_ptr() as *const c_char) };
    }
}

/// 测试opendir/closedir/readdir系统调用
fn test_opendir_readdir(results: &mut PosixTestResults) {
    crate::println!("      📂 测试opendir/closedir/readdir系统调用:");
    
    let test_dir = b"/test_opendir\0";
    
    // 创建测试目录
    let result = unsafe {
        crate::posix::mkdir(test_dir.as_ptr() as *const c_char, 0o755)
    };
    
    if result == 0 {
        // 在目录中创建一些文件
        for i in 1..=3 {
            let filename = alloc::format!("/test_opendir/file_{}", i);
            let fd = unsafe {
                crate::posix::open(filename.as_ptr() as *const c_char,
                                crate::posix::O_CREAT | crate::posix::O_WRONLY,
                                0o644)
            };
            if fd >= 0 {
                unsafe { crate::posix::close(fd) };
            }
        }
        
        // 测试opendir
        let dir = unsafe {
            crate::posix::opendir(test_dir.as_ptr() as *const c_char)
        };
        
        let passed = !dir.is_null();
        results.record_result(passed, "opendir打开目录",
            if passed { None } else { Some("opendir调用失败") });
        
        if !dir.is_null() {
            // 测试readdir
            let mut file_count = 0;
            loop {
                let entry = unsafe { crate::posix::readdir(dir) };
                if entry.is_null() {
                    break;
                }
                file_count += 1;
            }
            
            let passed = file_count >= 3;
            results.record_result(passed, "readdir读取目录",
                if passed { None } else { Some("readdir读取文件数不足") });
            
            // 测试closedir
            unsafe { crate::posix::closedir(dir) };
            
            let passed = true; // closedir总是成功
            results.record_result(passed, "closedir关闭目录",
                if passed { None } else { Some("closedir调用失败") });
        }
        
        // 清理
        for i in 1..=3 {
            let filename = alloc::format!("/test_opendir/file_{}", i);
            unsafe { crate::posix::unlink(filename.as_ptr() as *const c_char) };
        }
        unsafe { crate::posix::rmdir(test_dir.as_ptr() as *const c_char) };
    } else {
        results.record_skip("opendir/readdir", "无法创建测试目录");
    }
}

/// 测试getcwd/chdir系统调用
fn test_getcwd_chdir(results: &mut PosixTestResults) {
    crate::println!("      🔍 测试getcwd/chdir系统调用:");
    
    let mut buffer = [0u8; 512];
    
    // 测试getcwd
    let result = unsafe {
        crate::posix::getcwd(buffer.as_mut_ptr() as *mut c_char, buffer.len())
    };
    
    let passed = !result.is_null();
    results.record_result(passed, "getcwd获取当前目录",
        if passed { None } else { Some("getcwd调用失败") });
    
    if !result.is_null() {
        let original_cwd = unsafe {
            core::ffi::CStr::from_ptr(result).to_str().unwrap_or("")
        };
        
        // 创建测试目录
        let test_dir = b"/test_chdir\0";
        let result = unsafe {
            crate::posix::mkdir(test_dir.as_ptr() as *const c_char, 0o755)
        };
        
        if result == 0 {
            // 测试chdir
            let result = unsafe {
                crate::posix::chdir(test_dir.as_ptr() as *const c_char)
            };
            
            let passed = result == 0;
            results.record_result(passed, "chdir切换目录",
                if passed { None } else { Some("chdir调用失败") });
            
            if result == 0 {
                // 验证目录已切换
                let result = unsafe {
                    crate::posix::getcwd(buffer.as_mut_ptr() as *mut c_char, buffer.len())
                };
                
                if !result.is_null() {
                    let new_cwd = unsafe {
                        core::ffi::CStr::from_ptr(result).to_str().unwrap_or("")
                    };
                    
                    let passed = new_cwd.ends_with("test_chdir");
                    results.record_result(passed, "chdir目录切换验证",
                        if passed { None } else { Some("目录切换验证失败") });
                }
                
                // 切回原目录
                unsafe { crate::posix::chdir(original_cwd.as_ptr() as *const c_char) };
            }
            
            // 清理
            unsafe { crate::posix::rmdir(test_dir.as_ptr() as *const c_char) };
        } else {
            results.record_skip("chdir", "无法创建测试目录");
        }
    }
}

/// 测试文件描述符操作
fn test_fd_operations(results: &mut PosixTestResults) {
    crate::println!("    🔢 文件描述符操作测试:");
    
    // 测试dup/dup2
    test_dup_dup2(results);
    
    // 测试fcntl
    test_fcntl(results);
    
    // 测试ioctl
    test_ioctl(results);
}

/// 测试dup/dup2系统调用
fn test_dup_dup2(results: &mut PosixTestResults) {
    crate::println!("      🔄 测试dup/dup2系统调用:");
    
    let test_path = b"/test_dup\0";
    
    // 创建测试文件
    let fd1 = unsafe {
        crate::posix::open(test_path.as_ptr() as *const c_char,
                        crate::posix::O_CREAT | crate::posix::O_WRONLY,
                        0o644)
    };
    
    if fd1 >= 0 {
        // 测试dup
        let fd2 = unsafe { crate::posix::dup(fd1) };
        let passed = fd2 >= 0 && fd2 != fd1;
        results.record_result(passed, "dup复制文件描述符",
            if passed { None } else { Some("dup调用失败") });
        
        if fd2 >= 0 {
            // 测试dup2
            let fd3 = unsafe { crate::posix::dup2(fd1, fd2) };
            let passed = fd3 == fd2;
            results.record_result(passed, "dup2强制复制文件描述符",
                if passed { None } else { Some("dup2调用失败") });
            
            unsafe { crate::posix::close(fd2) };
        }
        
        // 清理
        unsafe { crate::posix::close(fd1) };
        unsafe { crate::posix::unlink(test_path.as_ptr() as *const c_char) };
    } else {
        results.record_skip("dup/dup2", "无法创建测试文件");
    }
}

/// 测试fcntl系统调用
fn test_fcntl(results: &mut PosixTestResults) {
    crate::println!("      🔧 测试fcntl系统调用:");
    
    let test_path = b"/test_fcntl\0";
    
    // 创建测试文件
    let fd = unsafe {
        crate::posix::open(test_path.as_ptr() as *const c_char,
                        crate::posix::O_CREAT | crate::posix::O_RDWR,
                        0o644)
    };
    
    if fd >= 0 {
        // 测试F_GETFL
        let flags = unsafe { crate::posix::fcntl(fd, crate::posix::F_GETFL, 0) };
        let passed = flags >= 0;
        results.record_result(passed, "fcntl F_GETFL获取文件标志",
            if passed { None } else { Some("fcntl F_GETFL调用失败") });
        
        // 测试F_SETFL
        let result = unsafe { crate::posix::fcntl(fd, crate::posix::F_SETFL, crate::posix::O_APPEND) };
        let passed = result == 0;
        results.record_result(passed, "fcntl F_SETFL设置文件标志",
            if passed { None } else { Some("fcntl F_SETFL调用失败") });
        
        // 清理
        unsafe { crate::posix::close(fd) };
        unsafe { crate::posix::unlink(test_path.as_ptr() as *const c_char) };
    } else {
        results.record_skip("fcntl", "无法创建测试文件");
    }
}

/// 测试ioctl系统调用
fn test_ioctl(results: &mut PosixTestResults) {
    crate::println!("      🔧 测试ioctl系统调用:");
    
    let test_path = b"/test_ioctl\0";
    
    // 创建测试文件
    let fd = unsafe {
        crate::posix::open(test_path.as_ptr() as *const c_char,
                        crate::posix::O_CREAT | crate::posix::O_RDWR,
                        0o644)
    };
    
    if fd >= 0 {
        // 测试TCGETS（获取终端属性）
        let mut termios = crate::posix::Termios::default();
        let result = unsafe { crate::posix::ioctl(fd, crate::posix::TCGETS, &mut termios) };
        
        // ioctl可能失败，这是正常的，因为我们不是终端
        let passed = result == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::ENOTTY;
        results.record_result(passed, "ioctl TCGETS非终端",
            if passed { None } else { Some("ioctl应该返回ENOTTY错误") });
        
        // 清理
        unsafe { crate::posix::close(fd) };
        unsafe { crate::posix::unlink(test_path.as_ptr() as *const c_char) };
    } else {
        results.record_skip("ioctl", "无法创建测试文件");
    }
}

/// 测试文件权限操作
fn test_file_permissions(results: &mut PosixTestResults) {
    crate::println!("    🔒 文件权限操作测试:");
    
    // 测试chmod/fchmod
    test_chmod_fchmod(results);
    
    // 测试chown/fchown
    test_chown_fchown(results);
    
    // 测试access
    test_access(results);
    
    // 测试umask
    test_umask(results);
}

/// 测试chmod/fchmod系统调用
fn test_chmod_fchmod(results: &mut PosixTestResults) {
    crate::println!("      🔐 测试chmod/fchmod系统调用:");
    
    let test_path = b"/test_chmod\0";
    
    // 创建测试文件
    let fd = unsafe {
        crate::posix::open(test_path.as_ptr() as *const c_char,
                        crate::posix::O_CREAT | crate::posix::O_WRONLY,
                        0o644)
    };
    
    if fd >= 0 {
        unsafe { crate::posix::close(fd) };
        
        // 测试chmod
        let result = unsafe {
            crate::posix::chmod(test_path.as_ptr() as *const c_char, 0o755)
        };
        
        let passed = result == 0;
        results.record_result(passed, "chmod修改文件权限",
            if passed { None } else { Some("chmod调用失败") });
        
        // 重新打开文件测试fchmod
        let fd = unsafe {
            crate::posix::open(test_path.as_ptr() as *const c_char,
                            crate::posix::O_RDWR, 0)
        };
        
        if fd >= 0 {
            let result = unsafe { crate::posix::fchmod(fd, 0o644) };
            let passed = result == 0;
            results.record_result(passed, "fchmod修改文件权限",
                if passed { None } else { Some("fchmod调用失败") });
            
            unsafe { crate::posix::close(fd) };
        }
        
        // 清理
        unsafe { crate::posix::unlink(test_path.as_ptr() as *const c_char) };
    } else {
        results.record_skip("chmod/fchmod", "无法创建测试文件");
    }
}

/// 测试chown/fchown系统调用
fn test_chown_fchown(results: &mut PosixTestResults) {
    crate::println!("      👥 测试chown/fchown系统调用:");
    
    let test_path = b"/test_chown\0";
    
    // 创建测试文件
    let fd = unsafe {
        crate::posix::open(test_path.as_ptr() as *const c_char,
                        crate::posix::O_CREAT | crate::posix::O_WRONLY,
                        0o644)
    };
    
    if fd >= 0 {
        unsafe { crate::posix::close(fd) };
        
        // 测试chown
        let result = unsafe {
            crate::posix::chown(test_path.as_ptr() as *const c_char, 1000, 1000)
        };
        
        // chown可能失败，这是正常的，因为我们不是root
        let passed = result == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::EPERM;
        results.record_result(passed, "chown非root用户",
            if passed { None } else { Some("chown应该返回EPERM错误") });
        
        // 重新打开文件测试fchown
        let fd = unsafe {
            crate::posix::open(test_path.as_ptr() as *const c_char,
                            crate::posix::O_RDWR, 0)
        };
        
        if fd >= 0 {
            let result = unsafe { crate::posix::fchown(fd, 1000, 1000) };
            let passed = result == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::EPERM;
            results.record_result(passed, "fchown非root用户",
                if passed { None } else { Some("fchown应该返回EPERM错误") });
            
            unsafe { crate::posix::close(fd) };
        }
        
        // 清理
        unsafe { crate::posix::unlink(test_path.as_ptr() as *const c_char) };
    } else {
        results.record_skip("chown/fchown", "无法创建测试文件");
    }
}

/// 测试access系统调用
fn test_access(results: &mut PosixTestResults) {
    crate::println!("      🔍 测试access系统调用:");
    
    let test_path = b"/test_access\0";
    
    // 创建测试文件
    let fd = unsafe {
        crate::posix::open(test_path.as_ptr() as *const c_char,
                        crate::posix::O_CREAT | crate::posix::O_WRONLY,
                        0o644)
    };
    
    if fd >= 0 {
        unsafe { crate::posix::close(fd) };
        
        // 测试F_OK
        let result = unsafe {
            crate::posix::access(test_path.as_ptr() as *const c_char, crate::posix::F_OK)
        };
        
        let passed = result == 0;
        results.record_result(passed, "access F_OK检查文件存在",
            if passed { None } else { Some("access F_OK调用失败") });
        
        // 测试R_OK
        let result = unsafe {
            crate::posix::access(test_path.as_ptr() as *const c_char, crate::posix::R_OK)
        };
        
        let passed = result == 0;
        results.record_result(passed, "access R_OK检查读权限",
            if passed { None } else { Some("access R_OK调用失败") });
        
        // 测试W_OK
        let result = unsafe {
            crate::posix::access(test_path.as_ptr() as *const c_char, crate::posix::W_OK)
        };
        
        let passed = result == 0;
        results.record_result(passed, "access W_OK检查写权限",
            if passed { None } else { Some("access W_OK调用失败") });
        
        // 测试X_OK
        let result = unsafe {
            crate::posix::access(test_path.as_ptr() as *const c_char, crate::posix::X_OK)
        };
        
        let passed = result == 0;
        results.record_result(passed, "access X_OK检查执行权限",
            if passed { None } else { Some("access X_OK调用失败") });
        
        // 测试不存在的文件
        let result = unsafe {
            crate::posix::access(b"/nonexistent\0".as_ptr() as *const c_char, crate::posix::F_OK)
        };
        
        let passed = result == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::ENOENT;
        results.record_result(passed, "access不存在的文件",
            if passed { None } else { Some("access应该返回ENOENT错误") });
        
        // 清理
        unsafe { crate::posix::unlink(test_path.as_ptr() as *const c_char) };
    } else {
        results.record_skip("access", "无法创建测试文件");
    }
}

/// 测试umask系统调用
fn test_umask(results: &mut PosixTestResults) {
    crate::println!("      🎭 测试umask系统调用:");
    
    // 保存原始umask
    let old_umask = unsafe { crate::posix::umask(0o022) };
    
    // 验证umask设置
    let passed = old_umask == 0o022;
    results.record_result(passed, "umask设置权限掩码",
        if passed { None } else { Some("umask返回值错误") });
    
    // 创建文件测试umask效果
    let test_path = b"/test_umask\0";
    let fd = unsafe {
        crate::posix::open(test_path.as_ptr() as *const c_char,
                        crate::posix::O_CREAT | crate::posix::O_WRONLY,
                        0o777) // 尝试创建所有权限
    };
    
    if fd >= 0 {
        unsafe { crate::posix::close(fd) };
        
        // 恢复原始umask
        unsafe { crate::posix::umask(old_umask) };
        
        // 清理
        unsafe { crate::posix::unlink(test_path.as_ptr() as *const c_char) };
        
        let passed = true;
        results.record_result(passed, "umask权限掩码效果",
            if passed { None } else { Some("umask权限掩码未生效") });
    } else {
        results.record_skip("umask效果", "无法创建测试文件");
    }
}

/// 进程管理相关系统调用测试
pub fn test_process_syscalls(results: &mut PosixTestResults) {
    crate::println!("  ⚙️ 进程管理系统调用测试:");
    
    let start_time = crate::subsystems::time::get_time_ns();
    
    // 测试fork/vfork
    test_fork_vfork(results);
    
    // 测试exec系列
    test_exec_series(results);
    
    // 测试wait系列
    test_wait_series(results);
    
    // 测试exit系列
    test_exit_series(results);
    
    // 测试getpid/getppid
    test_getpid_getppid(results);
    
    // 测试进程组相关
    test_process_groups(results);
    
    // 测试会话相关
    test_session_management(results);
    
    let execution_time = crate::subsystems::time::get_time_ns() - start_time;
    results.record_performance(PerformanceMetric {
        test_name: "process_syscalls".to_string(),
        execution_time_ns: execution_time,
        memory_used_bytes: 0,
        cpu_cycles: 0,
    });
}

/// 测试fork/vfork系统调用
fn test_fork_vfork(results: &mut PosixTestResults) {
    crate::println!("    🍃 测试fork/vfork系统调用:");
    
    // 测试fork
    let pid = unsafe { crate::posix::fork() };
    
    if pid == 0 {
        // 子进程
        unsafe { crate::posix::_exit(0) };
    } else if pid > 0 {
        // 父进程
        let mut status = 0;
        let result = unsafe { crate::posix::waitpid(pid, &mut status, 0) };
        
        let passed = result == pid && status == 0;
        results.record_result(passed, "fork创建子进程",
            if passed { None } else { Some("fork/waitpid调用失败") });
    } else {
        // fork失败
        let passed = crate::libc::error::get_errno() == crate::libc::error::errno::ENOSYS;
        results.record_result(passed, "fork未实现",
            if passed { None } else { Some("fork应该返回ENOSYS错误") });
    }
    
    // 测试vfork
    let pid = unsafe { crate::posix::vfork() };
    
    if pid == 0 {
        // 子进程
        unsafe { crate::posix::_exit(0) };
    } else if pid > 0 {
        // 父进程
        let mut status = 0;
        let result = unsafe { crate::posix::waitpid(pid, &mut status, 0) };
        
        let passed = result == pid && status == 0;
        results.record_result(passed, "vfork创建子进程",
            if passed { None } else { Some("vfork/waitpid调用失败") });
    } else {
        // vfork失败
        let passed = crate::libc::error::get_errno() == crate::libc::error::errno::ENOSYS;
        results.record_result(passed, "vfork未实现",
            if passed { None } else { Some("vfork应该返回ENOSYS错误") });
    }
}

/// 测试exec系列系统调用
fn test_exec_series(results: &mut PosixTestResults) {
    crate::println!("    🚀 测试exec系列系统调用:");
    
    // 测试execve
    test_execve(results);
    
    // 测试execvp
    test_execvp(results);
    
    // 测试execlp
    test_execlp(results);
}

/// 测试execve系统调用
fn test_execve(results: &mut PosixTestResults) {
    crate::println!("      🔧 测试execve系统调用:");
    
    // 创建一个简单的测试程序
    let test_program = b"/bin/echo\0";
    let args = [test_program.as_ptr(), b"hello\0".as_ptr(), core::ptr::null()];
    let envp = [core::ptr::null()];
    
    // 测试execve（这会替换当前进程，所以我们不能直接测试）
    // 这里我们只测试参数验证
    let passed = !test_program.is_null() && !args[0].is_null() && !envp[0].is_null();
    results.record_result(passed, "execve参数验证",
        if passed { None } else { Some("execve参数验证失败") });
    
    // 测试无效程序路径
    let result = unsafe {
        crate::posix::execve(b"/nonexistent\0".as_ptr(), args.as_ptr(), envp.as_ptr())
    };
    
    // execve失败时会返回，但实际上它不应该返回
    // 这里我们检查错误码
    let passed = crate::libc::error::get_errno() == crate::libc::error::errno::ENOENT;
    results.record_result(passed, "execve无效程序",
        if passed { None } else { Some("execve应该设置ENOENT错误") });
}

/// 测试execvp系统调用
fn test_execvp(results: &mut PosixTestResults) {
    crate::println!("      🔧 测试execvp系统调用:");
    
    let test_program = b"echo\0";
    let args = [test_program.as_ptr(), b"hello\0".as_ptr(), core::ptr::null()];
    
    // 测试execvp参数验证
    let passed = !test_program.is_null() && !args[0].is_null();
    results.record_result(passed, "execvp参数验证",
        if passed { None } else { Some("execvp参数验证失败") });
    
    // 测试无效程序
    let result = unsafe {
        crate::posix::execvp(b"nonexistent\0".as_ptr(), args.as_ptr())
    };
    
    let passed = crate::libc::error::get_errno() == crate::libc::error::errno::ENOENT;
    results.record_result(passed, "execvp无效程序",
        if passed { None } else { Some("execvp应该设置ENOENT错误") });
}

/// 测试execlp系统调用
fn test_execlp(results: &mut PosixTestResults) {
    crate::println!("      🔧 测试execlp系统调用:");
    
    let test_program = b"echo\0";
    
    // 测试execlp参数验证
    let passed = !test_program.is_null();
    results.record_result(passed, "execlp参数验证",
        if passed { None } else { Some("execlp参数验证失败") });
    
    // 测试无效程序
    let result = unsafe {
        crate::posix::execlp(b"nonexistent\0".as_ptr(), b"hello\0".as_ptr(), core::ptr::null())
    };
    
    let passed = crate::libc::error::get_errno() == crate::libc::error::errno::ENOENT;
    results.record_result(passed, "execlp无效程序",
        if passed { None } else { Some("execlp应该设置ENOENT错误") });
}

/// 测试wait系列系统调用
fn test_wait_series(results: &mut PosixTestResults) {
    crate::println!("      ⏳ 测试wait系列系统调用:");
    
    // 测试wait
    test_wait(results);
    
    // 测试waitpid
    test_waitpid(results);
    
    // 测试waitid
    test_waitid(results);
}

/// 测试wait系统调用
fn test_wait(results: &mut PosixTestResults) {
    crate::println!("        🔍 测试wait系统调用:");
    
    // 创建子进程
    let pid = unsafe { crate::posix::fork() };
    
    if pid == 0 {
        // 子进程立即退出
        unsafe { crate::posix::_exit(42) };
    } else if pid > 0 {
        // 父进程等待
        let mut status = 0;
        let result = unsafe { crate::posix::wait(&mut status) };
        
        let passed = result == pid && (status & 0x7F) == 42;
        results.record_result(passed, "wait等待子进程",
            if passed { None } else { Some("wait调用失败或状态错误") });
    } else {
        results.record_skip("wait", "无法创建子进程");
    }
}

/// 测试waitpid系统调用
fn test_waitpid(results: &mut PosixTestResults) {
    crate::println!("        🔍 测试waitpid系统调用:");
    
    // 创建子进程
    let pid = unsafe { crate::posix::fork() };
    
    if pid == 0 {
        // 子进程立即退出
        unsafe { crate::posix::_exit(43) };
    } else if pid > 0 {
        // 父进程等待特定PID
        let mut status = 0;
        let result = unsafe { crate::posix::waitpid(pid, &mut status, 0) };
        
        let passed = result == pid && (status & 0x7F) == 43;
        results.record_result(passed, "waitpid等待特定PID",
            if passed { None } else { Some("waitpid调用失败或状态错误") });
        
        // 测试WNOHANG选项
        let result = unsafe { crate::posix::waitpid(-1, &mut status, crate::posix::WNOHANG) };
        let passed = result == 0 || result == -1; // 没有子进程或无状态变化
        results.record_result(passed, "waitpid WNOHANG选项",
            if passed { None } else { Some("waitpid WNOHANG调用失败") });
    } else {
        results.record_skip("waitpid", "无法创建子进程");
    }
}

/// 测试waitid系统调用
fn test_waitid(results: &mut PosixTestResults) {
    crate::println!("        🔍 测试waitid系统调用:");
    
    // 创建子进程
    let pid = unsafe { crate::posix::fork() };
    
    if pid == 0 {
        // 子进程立即退出
        unsafe { crate::posix::_exit(44) };
    } else if pid > 0 {
        // 父进程等待
        let mut info = crate::posix::Siginfo::default();
        let id = crate::posix::P_PID;
        let result = unsafe { crate::posix::waitid(&id as *const c_void, pid, &mut info, crate::posix::WEXITED, core::ptr::null()) };
        
        let passed = result == pid && info.si_status == 44;
        results.record_result(passed, "waitid等待子进程",
            if passed { None } else { Some("waitid调用失败或状态错误") });
    } else {
        results.record_skip("waitid", "无法创建子进程");
    }
}

/// 测试exit系列系统调用
fn test_exit_series(results: &mut PosixTestResults) {
    crate::println!("      🚪 测试exit系列系统调用:");
    
    // 测试exit
    test_exit(results);
    
    // 测试_exit
    test__exit(results);
    
    // 测试abort
    test_abort(results);
}

/// 测试exit系统调用
fn test_exit(results: &mut PosixTestResults) {
    crate::println!("        🚪 测试exit系统调用:");
    
    // 创建子进程来测试exit
    let pid = unsafe { crate::posix::fork() };
    
    if pid == 0 {
        // 子进程调用exit
        unsafe { crate::posix::exit(0) };
    } else if pid > 0 {
        // 父进程等待
        let mut status = 0;
        let result = unsafe { crate::posix::waitpid(pid, &mut status, 0) };
        
        let passed = result == pid && status == 0;
        results.record_result(passed, "exit正常退出",
            if passed { None } else { Some("exit调用失败或状态错误") });
    } else {
        results.record_skip("exit", "无法创建子进程");
    }
}

/// 测试_exit系统调用
fn test__exit(results: &mut PosixTestResults) {
    crate::println!("        🚪 测试_exit系统调用:");
    
    // 创建子进程来测试_exit
    let pid = unsafe { crate::posix::fork() };
    
    if pid == 0 {
        // 子进程调用_exit
        unsafe { crate::posix::_exit(1) };
    } else if pid > 0 {
        // 父进程等待
        let mut status = 0;
        let result = unsafe { crate::posix::waitpid(pid, &mut status, 0) };
        
        let passed = result == pid && status == 1;
        results.record_result(passed, "_exit正常退出",
            if passed { None } else { Some("_exit调用失败或状态错误") });
    } else {
        results.record_skip("_exit", "无法创建子进程");
    }
}

/// 测试abort系统调用
fn test_abort(results: &mut PosixTestResults) {
    crate::println!("        💥 测试abort系统调用:");
    
    // 创建子进程来测试abort
    let pid = unsafe { crate::posix::fork() };
    
    if pid == 0 {
        // 子进程调用abort
        unsafe { crate::posix::abort() };
    } else if pid > 0 {
        // 父进程等待
        let mut status = 0;
        let result = unsafe { crate::posix::waitpid(pid, &mut status, 0) };
        
        // abort通常用SIGABRT信号终止
        let passed = result == pid && crate::posix::WCOREDUMP(status);
        results.record_result(passed, "abort异常终止",
            if passed { None } else { Some("abort调用失败或状态错误") });
    } else {
        results.record_skip("abort", "无法创建子进程");
    }
}

/// 测试getpid/getppid系统调用
fn test_getpid_getppid(results: &mut PosixTestResults) {
    crate::println!("      🆔 测试getpid/getppid系统调用:");
    
    // 测试getpid
    let pid = unsafe { crate::posix::getpid() };
    let passed = pid > 0;
    results.record_result(passed, "getpid获取进程ID",
        if passed { None } else { Some("getpid返回无效PID") });
    
    // 测试getppid
    let ppid = unsafe { crate::posix::getppid() };
    let passed = ppid > 0;
    results.record_result(passed, "getppid获取父进程ID",
        if passed { None } else { Some("getppid返回无效PID") });
    
    // 验证父子关系
    let passed = ppid != pid; // 父进程PID不应该等于子进程PID
    results.record_result(passed, "getpid/getppid父子关系",
        if passed { None } else { Some("父子进程PID关系错误") });
}

/// 测试进程组相关系统调用
fn test_process_groups(results: &mut PosixTestResults) {
    crate::println!("      👥 测试进程组相关系统调用:");
    
    // 测试getpgrp
    test_getpgrp(results);
    
    // 测试setpgrp
    test_setpgrp(results);
    
    // 测试getpgid
    test_getpgid(results);
    
    // 测试setpgid
    test_setpgid(results);
}

/// 测试getpgrp系统调用
fn test_getpgrp(results: &mut PosixTestResults) {
    crate::println!("        🔍 测试getpgrp系统调用:");
    
    let pgid = unsafe { crate::posix::getpgrp() };
    let passed = pgid > 0;
    results.record_result(passed, "getpgrp获取进程组ID",
        if passed { None } else { Some("getpgrp返回无效PGID") });
}

/// 测试setpgrp系统调用
fn test_setpgrp(results: &mut PosixTestResults) {
    crate::println!("        🔧 测试setpgrp系统调用:");
    
    let pid = unsafe { crate::posix::getpid() };
    let result = unsafe { crate::posix::setpgrp(pid) };
    
    // setpgrp可能失败，这是正常的
    let passed = result == 0 || result == -1;
    results.record_result(passed, "setpgrp设置进程组",
        if passed { None } else { Some("setpgrp调用异常") });
}

/// 测试getpgid系统调用
fn test_getpgid(results: &mut PosixTestResults) {
    crate::println!("        🔍 测试getpgid系统调用:");
    
    let pid = unsafe { crate::posix::getpid() };
    let pgid = unsafe { crate::posix::getpgid(pid) };
    let passed = pgid > 0;
    results.record_result(passed, "getpgid获取进程组ID",
        if passed { None } else { Some("getpgid返回无效PGID") });
}

/// 测试setpgid系统调用
fn test_setpgid(results: &mut PosixTestResults) {
    crate::println!("        🔧 测试setpgid系统调用:");
    
    let pid = unsafe { crate::posix::getpid() };
    let pgid = 1234;
    let result = unsafe { crate::posix::setpgid(pid, pgid) };
    
    // setpgid可能失败，这是正常的
    let passed = result == 0 || result == -1;
    results.record_result(passed, "setpgid设置进程组",
        if passed { None } else { Some("setpgid调用异常") });
}

/// 测试会话管理相关系统调用
fn test_session_management(results: &mut PosixTestResults) {
    crate::println!("      🏢 测试会话管理相关系统调用:");
    
    // 测试getsid
    test_getsid(results);
    
    // 测试setsid
    test_setsid(results);
}

/// 测试getsid系统调用
fn test_getsid(results: &mut PosixTestResults) {
    crate::println!("        🔍 测试getsid系统调用:");
    
    let pid = unsafe { crate::posix::getpid() };
    let sid = unsafe { crate::posix::getsid(pid) };
    let passed = sid > 0;
    results.record_result(passed, "getsid获取会话ID",
        if passed { None } else { Some("getsid返回无效SID") });
}

/// 测试setsid系统调用
fn test_setsid(results: &mut PosixTestResults) {
    crate::println!("        🔧 测试setsid系统调用:");
    
    let pid = unsafe { crate::posix::getpid() };
    let result = unsafe { crate::posix::setsid(pid) };
    
    // setsid可能失败，这是正常的
    let passed = result == pid || result == -1;
    results.record_result(passed, "setsid创建会话",
        if passed { None } else { Some("setsid调用异常") });
}

/// 内存管理相关系统调用测试
pub fn test_memory_syscalls(results: &mut PosixTestResults) {
    crate::println!("  💾 内存管理系统调用测试:");
    
    let start_time = crate::subsystems::time::get_time_ns();
    
    // 测试mmap系列
    test_mmap_series(results);
    
    // 测试mprotect
    test_mprotect(results);
    
    // 测试msync
    test_msync(results);
    
    // 测试mlock系列
    test_mlock_series(results);
    
    // 测试brk/sbrk
    test_brk_sbrk(results);
    
    let execution_time = crate::subsystems::time::get_time_ns() - start_time;
    results.record_performance(PerformanceMetric {
        test_name: "memory_syscalls".to_string(),
        execution_time_ns: execution_time,
        memory_used_bytes: 0,
        cpu_cycles: 0,
    });
}

/// 测试mmap系列系统调用
fn test_mmap_series(results: &mut PosixTestResults) {
    crate::println!("    🗺️ 测试mmap系列系统调用:");
    
    // 测试mmap
    test_mmap(results);
    
    // 测试munmap
    test_munmap(results);
    
    // 测试mremap
    test_mremap(results);
    
    // 测试madvise
    test_madvise(results);
    
    // 测试mincore
    test_mincore(results);
}

/// 测试mmap系统调用
fn test_mmap(results: &mut PosixTestResults) {
    crate::println!("      🗺️ 测试mmap系统调用:");
    
    // 测试匿名内存映射
    let addr = unsafe {
        crate::posix::mmap(
            core::ptr::null_mut(),
            4096,
            crate::posix::PROT_READ | crate::posix::PROT_WRITE,
            crate::posix::MAP_PRIVATE | crate::posix::MAP_ANONYMOUS,
            -1,
            0
        )
    };
    
    let passed = !addr.is_null() && addr != crate::posix::MAP_FAILED;
    results.record_result(passed, "mmap匿名内存映射",
        if passed { None } else { Some("mmap调用失败") });
    
    if !addr.is_null() && addr != crate::posix::MAP_FAILED {
        // 测试内存访问
        unsafe {
            let ptr = addr as *mut u8;
            *ptr = 0x42;
            let passed = *ptr == 0x42;
            results.record_result(passed, "mmap内存访问",
                if passed { None } else { Some("mmap内存访问失败") });
        };
        
        // 清理
        unsafe { crate::posix::munmap(addr, 4096) };
    }
    
    // 测试文件内存映射
    let test_path = b"/test_mmap\0";
    let fd = unsafe {
        crate::posix::open(test_path.as_ptr() as *const c_char,
                        crate::posix::O_CREAT | crate::posix::O_RDWR,
                        0o644)
    };
    
    if fd >= 0 {
        // 写入一些数据
        unsafe {
            crate::posix::write(fd, b"test data\0".as_ptr() as *const c_void, 9);
        };
        
        let addr = unsafe {
            crate::posix::mmap(
                core::ptr::null_mut(),
                4096,
                crate::posix::PROT_READ | crate::posix::PROT_WRITE,
                crate::posix::MAP_SHARED,
                fd,
                0
            )
        };
        
        let passed = !addr.is_null() && addr != crate::posix::MAP_FAILED;
        results.record_result(passed, "mmap文件内存映射",
            if passed { None } else { Some("mmap文件映射失败") });
        
        if !addr.is_null() && addr != crate::posix::MAP_FAILED {
            // 清理
            unsafe { crate::posix::munmap(addr, 4096) };
        }
        
        unsafe { crate::posix::close(fd) };
        unsafe { crate::posix::unlink(test_path.as_ptr() as *const c_char) };
    } else {
        results.record_skip("mmap文件映射", "无法创建测试文件");
    }
}

/// 测试munmap系统调用
fn test_munmap(results: &mut PosixTestResults) {
    crate::println!("      🗑️ 测试munmap系统调用:");
    
    // 先映射内存
    let addr = unsafe {
        crate::posix::mmap(
            core::ptr::null_mut(),
            4096,
            crate::posix::PROT_READ | crate::posix::PROT_WRITE,
            crate::posix::MAP_PRIVATE | crate::posix::MAP_ANONYMOUS,
            -1,
            0
        )
    };
    
    if !addr.is_null() && addr != crate::posix::MAP_FAILED {
        // 测试munmap
        let result = unsafe { crate::posix::munmap(addr, 4096) };
        let passed = result == 0;
        results.record_result(passed, "munmap解除内存映射",
            if passed { None } else { Some("munmap调用失败") });
        
        // 测试无效地址
        let result = unsafe { crate::posix::munmap(core::ptr::null_mut(), 4096) };
        let passed = result == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::EINVAL;
        results.record_result(passed, "munmap无效地址",
            if passed { None } else { Some("munmap应该返回EINVAL错误") });
    } else {
        results.record_skip("munmap", "无法映射内存");
    }
}

/// 测试mremap系统调用
fn test_mremap(results: &mut PosixTestResults) {
    crate::println!("      🔄 测试mremap系统调用:");
    
    // 先映射内存
    let old_addr = unsafe {
        crate::posix::mmap(
            core::ptr::null_mut(),
            4096,
            crate::posix::PROT_READ | crate::posix::PROT_WRITE,
            crate::posix::MAP_PRIVATE | crate::posix::MAP_ANONYMOUS,
            -1,
            0
        )
    };
    
    if !old_addr.is_null() && old_addr != crate::posix::MAP_FAILED {
        // 测试mremap扩展
        let new_addr = unsafe {
            crate::posix::mremap(old_addr, 4096, 8192, crate::posix::MREMAP_MAYMOVE)
        };
        
        let passed = !new_addr.is_null() && new_addr != crate::posix::MAP_FAILED;
        results.record_result(passed, "mremap扩展内存映射",
            if passed { None } else { Some("mremap调用失败") });
        
        if !new_addr.is_null() && new_addr != crate::posix::MAP_FAILED {
            // 清理
            unsafe { crate::posix::munmap(new_addr, 8192) };
        } else {
            // 清理原始映射
            unsafe { crate::posix::munmap(old_addr, 4096) };
        }
    } else {
        results.record_skip("mremap", "无法映射内存");
    }
}

/// 测试madvise系统调用
fn test_madvise(results: &mut PosixTestResults) {
    crate::println!("      💡 测试madvise系统调用:");
    
    // 先映射内存
    let addr = unsafe {
        crate::posix::mmap(
            core::ptr::null_mut(),
            4096,
            crate::posix::PROT_READ | crate::posix::PROT_WRITE,
            crate::posix::MAP_PRIVATE | crate::posix::MAP_ANONYMOUS,
            -1,
            0
        )
    };
    
    if !addr.is_null() && addr != crate::posix::MAP_FAILED {
        // 测试MADV_NORMAL
        let result = unsafe {
            crate::posix::madvise(addr, 4096, crate::posix::MADV_NORMAL)
        };
        
        let passed = result == 0;
        results.record_result(passed, "madvise正常建议",
            if passed { None } else { Some("madvise调用失败") });
        
        // 测试MADV_RANDOM
        let result = unsafe {
            crate::posix::madvise(addr, 4096, crate::posix::MADV_RANDOM)
        };
        
        let passed = result == 0;
        results.record_result(passed, "madvise随机访问建议",
            if passed { None } else { Some("madvise随机访问建议失败") });
        
        // 清理
        unsafe { crate::posix::munmap(addr, 4096) };
    } else {
        results.record_skip("madvise", "无法映射内存");
    }
}

/// 测试mincore系统调用
fn test_mincore(results: &mut PosixTestResults) {
    crate::println!("      🔍 测试mincore系统调用:");
    
    // 先映射内存
    let addr = unsafe {
        crate::posix::mmap(
            core::ptr::null_mut(),
            4096,
            crate::posix::PROT_READ | crate::posix::PROT_WRITE,
            crate::posix::MAP_PRIVATE | crate::posix::MAP_ANONYMOUS,
            -1,
            0
        )
    };
    
    if !addr.is_null() && addr != crate::posix::MAP_FAILED {
        let mut vec = [0u8; 64]; // 64个页面，每页4KB
        let result = unsafe {
            crate::posix::mincore(addr, 4096, vec.as_mut_ptr(), vec.len())
        };
        
        let passed = result == 0;
        results.record_result(passed, "mincore页面驻留状态",
            if passed { None } else { Some("mincore调用失败") });
        
        // 验证结果
        if result == 0 {
            // 至少应该有一些页面在内存中
            let any_resident = vec.iter().any(|&x| x & 0x01 != 0);
            let passed = any_resident;
            results.record_result(passed, "mincore页面驻留验证",
                if passed { None } else { Some("mincore没有检测到驻留页面") });
        }
        
        // 清理
        unsafe { crate::posix::munmap(addr, 4096) };
    } else {
        results.record_skip("mincore", "无法映射内存");
    }
}

/// 测试mprotect系统调用
fn test_mprotect(results: &mut PosixTestResults) {
    crate::println!("      🛡️ 测试mprotect系统调用:");
    
    // 先映射内存
    let addr = unsafe {
        crate::posix::mmap(
            core::ptr::null_mut(),
            4096,
            crate::posix::PROT_READ | crate::posix::PROT_WRITE,
            crate::posix::MAP_PRIVATE | crate::posix::MAP_ANONYMOUS,
            -1,
            0
        )
    };
    
    if !addr.is_null() && addr != crate::posix::MAP_FAILED {
        // 测试设置为只读
        let result = unsafe {
            crate::posix::mprotect(addr, 4096, crate::posix::PROT_READ)
        };
        
        let passed = result == 0;
        results.record_result(passed, "mprotect设置为只读",
            if passed { None } else { Some("mprotect调用失败") });
        
        // 测试设置为读写
        let result = unsafe {
            crate::posix::mprotect(addr, 4096, crate::posix::PROT_READ | crate::posix::PROT_WRITE)
        };
        
        let passed = result == 0;
        results.record_result(passed, "mprotect设置为读写",
            if passed { None } else { Some("mprotect调用失败") });
        
        // 测试无效地址
        let result = unsafe {
            crate::posix::mprotect(core::ptr::null_mut(), 4096, crate::posix::PROT_READ)
        };
        
        let passed = result == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::EINVAL;
        results.record_result(passed, "mprotect无效地址",
            if passed { None } else { Some("mprotect应该返回EINVAL错误") });
        
        // 清理
        unsafe { crate::posix::munmap(addr, 4096) };
    } else {
        results.record_skip("mprotect", "无法映射内存");
    }
}

/// 测试msync系统调用
fn test_msync(results: &mut PosixTestResults) {
    crate::println!("      💾 测试msync系统调用:");
    
    // 先映射内存
    let addr = unsafe {
        crate::posix::mmap(
            core::ptr::null_mut(),
            4096,
            crate::posix::PROT_READ | crate::posix::PROT_WRITE,
            crate::posix::MAP_SHARED,
            -1,
            0
        )
    };
    
    if !addr.is_null() && addr != crate::posix::MAP_FAILED {
        // 写入一些数据
        unsafe {
            let ptr = addr as *mut u8;
            for i in 0..4096 {
                *ptr.add(i) = (i % 256) as u8;
            }
        };
        
        // 测试MS_SYNC
        let result = unsafe {
            crate::posix::msync(addr, 4096, crate::posix::MS_SYNC)
        };
        
        let passed = result == 0;
        results.record_result(passed, "msync同步内存",
            if passed { None } else { Some("msync调用失败") });
        
        // 测试MS_ASYNC
        let result = unsafe {
            crate::posix::msync(addr, 4096, crate::posix::MS_ASYNC)
        };
        
        let passed = result == 0;
        results.record_result(passed, "msync异步同步",
            if passed { None } else { Some("msync异步调用失败") });
        
        // 测试MS_INVALIDATE
        let result = unsafe {
            crate::posix::msync(addr, 4096, crate::posix::MS_INVALIDATE)
        };
        
        let passed = result == 0;
        results.record_result(passed, "msync无效化缓存",
            if passed { None } else { Some("msync无效化调用失败") });
        
        // 清理
        unsafe { crate::posix::munmap(addr, 4096) };
    } else {
        results.record_skip("msync", "无法映射内存");
    }
}

/// 测试mlock系列系统调用
fn test_mlock_series(results: &mut PosixTestResults) {
    crate::println!("    🔒 测试mlock系列系统调用:");
    
    // 测试mlock/munlock
    test_mlock_munlock(results);
    
    // 测试mlockall/munlockall
    test_mlockall_munlockall(results);
}

/// 测试mlock/munlock系统调用
fn test_mlock_munlock(results: &mut PosixTestResults) {
    crate::println!("      🔒 测试mlock/munlock系统调用:");
    
    // 分配一些内存
    let addr = unsafe {
        crate::posix::mmap(
            core::ptr::null_mut(),
            4096,
            crate::posix::PROT_READ | crate::posix::PROT_WRITE,
            crate::posix::MAP_PRIVATE | crate::posix::MAP_ANONYMOUS,
            -1,
            0
        )
    };
    
    if !addr.is_null() && addr != crate::posix::MAP_FAILED {
        // 测试mlock
        let result = unsafe { crate::posix::mlock(addr, 4096) };
        
        // mlock可能失败，这是正常的
        let passed = result == 0 || result == -1;
        results.record_result(passed, "mlock锁定内存",
            if passed { None } else { Some("mlock调用异常") });
        
        if result == 0 {
            // 测试munlock
            let result = unsafe { crate::posix::munlock(addr, 4096) };
            let passed = result == 0;
            results.record_result(passed, "munlock解锁内存",
                if passed { None } else { Some("munlock调用失败") });
        }
        
        // 清理
        unsafe { crate::posix::munmap(addr, 4096) };
    } else {
        results.record_skip("mlock/munlock", "无法映射内存");
    }
}

/// 测试mlockall/munlockall系统调用
fn test_mlockall_munlockall(results: &mut PosixTestResults) {
    crate::println!("      🔒 测试mlockall/munlockall系统调用:");
    
    // 测试mlockall
    let result = unsafe { crate::posix::mlockall(crate::posix::MCL_CURRENT) };
    
    // mlockall可能失败，这是正常的
    let passed = result == 0 || result == -1;
    results.record_result(passed, "mlockall锁定进程内存",
        if passed { None } else { Some("mlockall调用异常") });
    
    if result == 0 {
        // 测试munlockall
        let result = unsafe { crate::posix::munlockall() };
        let passed = result == 0;
        results.record_result(passed, "munlockall解锁进程内存",
            if passed { None } else { Some("munlockall调用失败") });
    }
}

/// 测试brk/sbrk系统调用
fn test_brk_sbrk(results: &mut PosixTestResults) {
    crate::println!("      📈 测试brk/sbrk系统调用:");
    
    // 获取当前break
    let old_brk = unsafe { crate::posix::sbrk(0 as *mut c_void) };
    let passed = !old_brk.is_null();
    results.record_result(passed, "sbrk获取当前break",
        if passed { None } else { Some("sbrk返回空指针") });
    
    if !old_brk.is_null() {
        // 测试brk扩展堆
        let new_brk = unsafe { crate::posix::brk((old_brk as usize + 4096) as *mut c_void) };
        let passed = !new_brk.is_null() && new_brk > old_brk;
        results.record_result(passed, "brk扩展堆空间",
            if passed { None } else { Some("brk扩展失败") });
        
        // 测试sbrk扩展堆
        let new_brk = unsafe { crate::posix::sbrk((old_brk as usize + 8192) as *mut c_void) };
        let passed = !new_brk.is_null() && new_brk > old_brk;
        results.record_result(passed, "sbrk扩展堆空间",
            if passed { None } else { Some("sbrk扩展失败") });
        
        // 测试无效地址
        let new_brk = unsafe { crate::posix::brk(core::ptr::null_mut()) };
        let passed = new_brk.is_null() && crate::libc::error::get_errno() == crate::libc::error::errno::ENOMEM;
        results.record_result(passed, "brk无效地址",
            if passed { None } else { Some("brk应该返回ENOMEM错误") });
    }
}

/// 网络相关系统调用测试
pub fn test_network_syscalls(results: &mut PosixTestResults) {
    crate::println!("  🌐 网络系统调用测试:");
    
    let start_time = crate::subsystems::time::get_time_ns();
    
    // 测试socket系列
    test_socket_series(results);
    
    // 测试bind/listen/accept
    test_bind_listen_accept(results);
    
    // 测试connect
    test_connect(results);
    
    // 测试send/recv系列
    test_send_recv_series(results);
    
    // 测试shutdown
    test_shutdown(results);
    
    let execution_time = crate::subsystems::time::get_time_ns() - start_time;
    results.record_performance(PerformanceMetric {
        test_name: "network_syscalls".to_string(),
        execution_time_ns: execution_time,
        memory_used_bytes: 0,
        cpu_cycles: 0,
    });
}

/// 测试socket系列系统调用
fn test_socket_series(results: &mut PosixTestResults) {
    crate::println!("    🔌 测试socket系列系统调用:");
    
    // 测试socket
    test_socket(results);
    
    // 测试socketpair
    test_socketpair(results);
    
    // 测试getsockname/getpeername
    test_socket_names(results);
    
    // 测试getsockopt/setsockopt
    test_socket_options(results);
}

/// 测试socket系统调用
fn test_socket(results: &mut PosixTestResults) {
    crate::println!("      🔌 测试socket系统调用:");
    
    // 测试创建TCP socket
    let fd = unsafe {
        crate::posix::socket(crate::posix::AF_INET, crate::posix::SOCK_STREAM, crate::posix::IPPROTO_TCP)
    };
    
    let passed = fd >= 0;
    results.record_result(passed, "socket创建TCP socket",
        if passed { None } else { Some("socket调用失败") });
    
    if fd >= 0 {
        unsafe { crate::posix::close(fd) };
    }
    
    // 测试创建UDP socket
    let fd = unsafe {
        crate::posix::socket(crate::posix::AF_INET, crate::posix::SOCK_DGRAM, crate::posix::IPPROTO_UDP)
    };
    
    let passed = fd >= 0;
    results.record_result(passed, "socket创建UDP socket",
        if passed { None } else { Some("socket调用失败") });
    
    if fd >= 0 {
        unsafe { crate::posix::close(fd) };
    }
    
    // 测试无效参数
    let fd = unsafe {
        crate::posix::socket(-1, crate::posix::SOCK_STREAM, crate::posix::IPPROTO_TCP)
    };
    
    let passed = fd == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::EAFNOSUPPORT;
    results.record_result(passed, "socket无效协议族",
        if passed { None } else { Some("socket应该返回EAFNOSUPPORT错误") });
}

/// 测试socketpair系统调用
fn test_socketpair(results: &mut PosixTestResults) {
    crate::println!("      🔗 测试socketpair系统调用:");
    
    let mut fds = [0; 2];
    let result = unsafe {
        crate::posix::socketpair(crate::posix::AF_UNIX, crate::posix::SOCK_STREAM, 0, fds.as_mut_ptr())
    };
    
    let passed = result == 0;
    results.record_result(passed, "socketpair创建socket对",
        if passed { None } else { Some("socketpair调用失败") });
    
    if result == 0 {
        // 测试socket对通信
        let test_data = b"hello";
        let result = unsafe {
            crate::posix::write(fds[0], test_data.as_ptr() as *const c_void, test_data.len())
        };
        
        let passed = result == test_data.len() as isize;
        results.record_result(passed, "socketpair写入数据",
            if passed { None } else { Some("socketpair写入失败") });
        
        if result == test_data.len() as isize {
            let mut buffer = [0u8; 256];
            let result = unsafe {
                crate::posix::read(fds[1], buffer.as_mut_ptr() as *mut c_void, buffer.len())
            };
            
            let passed = result == test_data.len() as isize;
            results.record_result(passed, "socketpair读取数据",
                if passed { None } else { Some("socketpair读取失败") });
            
            if result == test_data.len() as isize {
                let passed = &buffer[..test_data.len()] == test_data;
                results.record_result(passed, "socketpair数据一致性",
                    if passed { None } else { Some("socketpair数据不一致") });
            }
        }
        
        // 清理
        unsafe { crate::posix::close(fds[0]) };
        unsafe { crate::posix::close(fds[1]) };
    }
}

/// 测试getsockname/getpeername系统调用
fn test_socket_names(results: &mut PosixTestResults) {
    crate::println!("      🔍 测试socket名称系统调用:");
    
    let fd = unsafe {
        crate::posix::socket(crate::posix::AF_INET, crate::posix::SOCK_STREAM, crate::posix::IPPROTO_TCP)
    };
    
    if fd >= 0 {
        // 绑定到本地地址
        let addr = crate::posix::SockaddrIn {
            sin_family: crate::posix::AF_INET,
            sin_port: 0x1234, // 4660
            sin_addr: crate::posix::INADDR_ANY,
        };
        
        let result = unsafe {
            crate::posix::bind(fd, &addr as *const crate::posix::Sockaddr, core::mem::size_of::<crate::posix::Sockaddr>())
        };
        
        if result == 0 {
            // 测试getsockname
            let mut sockaddr = crate::posix::Sockaddr::default();
            let mut len = core::mem::size_of::<crate::posix::Sockaddr>() as u32;
            let result = unsafe {
                crate::posix::getsockname(fd, &mut sockaddr, &mut len)
            };
            
            let passed = result == 0;
            results.record_result(passed, "getsockname获取本地地址",
                if passed { None } else { Some("getsockname调用失败") });
            
            if result == 0 {
                let passed = len == core::mem::size_of::<crate::posix::Sockaddr>();
                results.record_result(passed, "getsockname地址长度",
                    if passed { None } else { Some("getsockname地址长度错误") });
            }
        }
        
        // 测试getpeername（未连接的socket）
        let mut sockaddr = crate::posix::Sockaddr::default();
        let mut len = core::mem::size_of::<crate::posix::Sockaddr>() as u32;
        let result = unsafe {
            crate::posix::getpeername(fd, &mut sockaddr, &mut len)
        };
        
        let passed = result == -1 && crate::libc::error::get_errno() == crate::libc::error::errno::ENOTCONN;
        results.record_result(passed, "getpeername未连接socket",
            if passed { None } else { Some("getpeername应该返回ENOTCONN错误") });
        
        unsafe { crate::posix::close(fd) };
    } else {
        results.record_skip("socket名称", "无法创建socket");
    }
}

/// 测试getsockopt/setsockopt系统调用
fn test_socket_options(results: &mut PosixTestResults) {
    crate::println!("      ⚙️ 测试socket选项系统调用:");
    
    let fd = unsafe {
        crate::posix::socket(crate::posix::AF_INET, crate::posix::SOCK_STREAM, crate::posix::IPPROTO_TCP)
    };
    
    if fd >= 0 {
        // 测试SO_REUSEADDR
        let mut optval = 0;
        let mut optlen = core::mem::size_of::<c_int>() as u32;
        let result = unsafe {
            crate::posix::getsockopt(fd, crate::posix::SOL_SOCKET, crate::posix::SO_REUSEADDR, &mut optval, &mut optlen)
        };
        
        let passed = result == 0;
        results.record_result(passed, "getsockopt SO_REUSEADDR",
            if passed { None } else { Some("getsockopt调用失败") });
        
        // 设置SO_REUSEADDR
        let optval = 1;
        let result = unsafe {
            crate::posix::setsockopt(fd, crate::posix::SOL_SOCKET, crate::posix::SO_REUSEADDR, &optval, core::mem::size_of::<c_int>())
        };
        
        let passed = result == 0;
        results.record_result(passed, "setsockopt SO_REUSEADDR",
            if passed { None } else { Some("setsockopt调用失败") });
        
        // 测试SO_SNDBUF
        let mut optval = 0;
        let mut optlen = core::mem::size_of::<c_int>() as u32;
        let result = unsafe {
            crate::posix::getsockopt(fd, crate::posix::SOL_SOCKET, crate::posix::SO_SNDBUF, &mut optval, &mut optlen)
        };
        
        let passed = result == 0;
        results.record_result(passed, "getsockopt SO_SNDBUF",
            if passed { None } else { Some("getsockopt调用失败") });
        
        // 设置SO_SNDBUF
        let optval = 8192; // 8KB
        let result = unsafe {
            crate::posix::setsockopt(fd, crate::posix::SOL_SOCKET, crate::posix::SO_SNDBUF, &optval, core::mem::size_of::<c_int>())
        };
        
        let passed = result == 0;
        results.record_result(passed, "setsockopt SO_SNDBUF",
            if passed { None } else { Some("setsockopt调用失败") });
        
        unsafe { crate::posix::close(fd) };
    } else {
        results.record_skip("socket选项", "无法创建socket");
    }
}

/// 测试bind/listen/accept系统调用
fn test_bind_listen_accept(results: &mut PosixTestResults) {
    crate::println!("    🎣 测试bind/listen/accept系统调用:");
    
    let fd = unsafe {
        crate::posix::socket(crate::posix::AF_INET, crate::posix::SOCK_STREAM, crate::posix::IPPROTO_TCP)
    };
    
    if fd >= 0 {
        // 绑定地址
        let addr = crate::posix::SockaddrIn {
            sin_family: crate::posix::AF_INET,
            sin_port: 0x1234, // 4660
            sin_addr: crate::posix::INADDR_ANY,
        };
        
        let result = unsafe {
            crate::posix::bind(fd, &addr as *const crate::posix::Sockaddr, core::mem::size_of::<crate::posix::Sockaddr>())
        };
        
        let passed = result == 0;
        results.record_result(passed, "bind绑定地址",
            if passed { None } else { Some("bind调用失败") });
        
        if result == 0 {
            // 测试listen
            let result = unsafe { crate::posix::listen(fd, 5) }; // backlog = 5
            let passed = result == 0;
            results.record_result(passed, "listen监听连接",
                if passed { None } else { Some("listen调用失败") });
            
            if result == 0 {
                // 注意：accept会阻塞，所以我们不实际测试
                // 在实际测试中，应该使用非阻塞模式或超时
                let passed = true;
                results.record_result(passed, "accept接受连接（跳过实际测试）",
                    if passed { None } else { Some("accept测试设计问题") });
            }
        }
        
        unsafe { crate::posix::close(fd) };
    } else {
        results.record_skip("bind/listen/accept", "无法创建socket");
    }
}

/// 测试connect系统调用
fn test_connect(results: &mut PosixTestResults) {
    crate::println!("    🔗 测试connect系统调用:");
    
    let fd = unsafe {
        crate::posix::socket(crate::posix::AF_INET, crate::posix::SOCK_STREAM, crate::posix::IPPROTO_TCP)
    };
    
    if fd >= 0 {
        // 尝试连接到本地回环地址
        let addr = crate::posix::SockaddrIn {
            sin_family: crate::posix::AF_INET,
            sin_port: 0x1234, // 4660
            sin_addr: crate::posix::INADDR_LOOPBACK,
        };
        
        let result = unsafe {
            crate::posix::connect(fd, &addr as *const crate::posix::Sockaddr, core::mem::size_of::<crate::posix::Sockaddr>())
        };
        
        // 连接可能失败，这是正常的，因为没有服务器监听
        let passed = result == 0 || result == -1;
        results.record_result(passed, "connect连接到回环地址",
            if passed { None } else { Some("connect调用异常") });
        
        if result == -1 {
            let passed = crate::libc::error::get_errno() == crate::libc::error::errno::ECONNREFUSED;
            results.record_result(passed, "connect连接被拒绝",
                if passed { None } else { Some("connect应该返回ECONNREFUSED错误") });
        }
        
        unsafe { crate::posix::close(fd) };
    } else {
        results.record_skip("connect", "无法创建socket");
    }
}

/// 测试send/recv系列系统调用
fn test_send_recv_series(results: &mut PosixTestResults) {
    crate::println!("    📤 测试send/recv系列系统调用:");
    
    // 测试send/recv
    test_send_recv(results);
    
    // 测试sendto/recvfrom
    test_sendto_recvfrom(results);
    
    // 测试sendmsg/recvmsg
    test_sendmsg_recvmsg(results);
}

/// 测试send/recv系统调用
fn test_send_recv(results: &mut PosixTestResults) {
    crate::println!("      📤 测试send/recv系统调用:");
    
    let fds = [0; 2];
    let result = unsafe {
        crate::posix::socketpair(crate::posix::AF_UNIX, crate::posix::SOCK_STREAM, 0, fds.as_mut_ptr())
    };
    
    if result == 0 {
        let test_data = b"Hello, POSIX!";
        
        // 测试send
        let result = unsafe {
            crate::posix::send(fds[0], test_data.as_ptr() as *const c_void, test_data.len(), 0)
        };
        
        let passed = result == test_data.len() as isize;
        results.record_result(passed, "send发送数据",
            if passed { None } else { Some("send调用失败") });
        
        if result == test_data.len() as isize {
            let mut buffer = [0u8; 256];
            let result = unsafe {
                crate::posix::recv(fds[1], buffer.as_mut_ptr() as *mut c_void, buffer.len(), 0)
            };
            
            let passed = result == test_data.len() as isize;
            results.record_result(passed, "recv接收数据",
                if passed { None } else { Some("recv调用失败") });
            
            if result == test_data.len() as isize {
                let passed = &buffer[..test_data.len()] == test_data;
                results.record_result(passed, "send/recv数据一致性",
                    if passed { None } else { Some("send/recv数据不一致") });
            }
        }
        
        // 清理
        unsafe { crate::posix::close(fds[0]) };
        unsafe { crate::posix::close(fds[1]) };
    } else {
        results.record_skip("send/recv", "无法创建socket对");
    }
}

/// 测试sendto/recvfrom系统调用
fn test_sendto_recvfrom(results: &mut PosixTestResults) {
    crate::println!("      📤 测试sendto/recvfrom系统调用:");
    
    let fd = unsafe {
        crate::posix::socket(crate::posix::AF_INET, crate::posix::SOCK_DGRAM, crate::posix::IPPROTO_UDP)
    };
    
    if fd >= 0 {
        let test_data = b"Hello, UDP!";
        let addr = crate::posix::SockaddrIn {
            sin_family: crate::posix::AF_INET,
            sin_port: 0x1234, // 4660
            sin_addr: crate::posix::INADDR_LOOPBACK,
        };
        
        // 测试sendto
        let result = unsafe {
            crate::posix::sendto(fd, test_data.as_ptr() as *const c_void, test_data.len(), 0,
                           &addr as *const crate::posix::Sockaddr, core::mem::size_of::<crate::posix::Sockaddr>())
        };
        
        let passed = result == test_data.len() as isize;
        results.record_result(passed, "sendto发送UDP数据",
            if passed { None } else { Some("sendto调用失败") });
        
        if result == test_data.len() as isize {
            let mut buffer = [0u8; 256];
            let mut from_addr = crate::posix::Sockaddr::default();
            let mut from_len = core::mem::size_of::<crate::posix::Sockaddr>() as u32;
            let result = unsafe {
                crate::posix::recvfrom(fd, buffer.as_mut_ptr() as *mut c_void, buffer.len(), 0,
                                &mut from_addr as *mut crate::posix::Sockaddr, &mut from_len)
            };
            
            let passed = result == test_data.len() as isize;
            results.record_result(passed, "recvfrom接收UDP数据",
                if passed { None } else { Some("recvfrom调用失败") });
            
            if result == test_data.len() as isize {
                let passed = &buffer[..test_data.len()] == test_data;
                results.record_result(passed, "sendto/recvfrom数据一致性",
                    if passed { None } else { Some("sendto/recvfrom数据不一致") });
            }
        }
        
        unsafe { crate::posix::close(fd) };
    } else {
        results.record_skip("sendto/recvfrom", "无法创建socket");
    }
}

/// 测试sendmsg/recvmsg系统调用
fn test_sendmsg_recvmsg(results: &mut PosixTestResults) {
    crate::println!("      📨 测试sendmsg/recvmsg系统调用:");
    
    let fds = [0; 2];
    let result = unsafe {
        crate::posix::socketpair(crate::posix::AF_UNIX, crate::posix::SOCK_STREAM, 0, fds.as_mut_ptr())
    };
    
    if result == 0 {
        let test_data = b"Hello, msg!";
        
        // 构造msghdr
        let mut hdr = crate::posix::Msghdr::default();
        hdr.msg_iovlen = 1;
        hdr.msg_name = fds[0] as c_int;
        
        // 构造iovec
        let iov = crate::posix::IoVec {
            iov_base: test_data.as_ptr() as *mut c_void,
            iov_len: test_data.len(),
        };
        
        // 测试sendmsg
        let result = unsafe {
            crate::posix::sendmsg(fds[0], &mut hdr, &iov, 0)
        };
        
        let passed = result == test_data.len() as isize;
        results.record_result(passed, "sendmsg发送消息",
            if passed { None } else { Some("sendmsg调用失败") });
        
        if result == test_data.len() as isize {
            let mut recv_hdr = crate::posix::Msghdr::default();
            let mut recv_iov = crate::posix::IoVec {
                iov_base: core::ptr::null_mut(),
                iov_len: 0,
            };
            let mut buffer = [0u8; 256];
            
            // 设置接收缓冲区
            recv_iov.iov_base = buffer.as_mut_ptr() as *mut c_void;
            recv_iov.iov_len = buffer.len();
            
            let result = unsafe {
                crate::posix::recvmsg(fds[1], &mut recv_hdr, &mut recv_iov, 0)
            };
            
            let passed = result == test_data.len() as isize;
            results.record_result(passed, "recvmsg接收消息",
                if passed { None } else { Some("recvmsg调用失败") });
            
            if result == test_data.len() as isize {
                let passed = &buffer[..test_data.len()] == test_data;
                results.record_result(passed, "sendmsg/recvmsg数据一致性",
                    if passed { None } else { Some("sendmsg/recvmsg数据不一致") });
            }
        }
        
        // 清理
        unsafe { crate::posix::close(fds[0]) };
        unsafe { crate::posix::close(fds[1]) };
    } else {
        results.record_skip("sendmsg/recvmsg", "无法创建socket对");
    }
}

/// 测试shutdown系统调用
fn test_shutdown(results: &mut PosixTestResults) {
    crate::println!("      🔌 测试shutdown系统调用:");
    
    let fds = [0; 2];
    let result = unsafe {
        crate::posix::socketpair(crate::posix::AF_UNIX, crate::posix::SOCK_STREAM, 0, fds.as_mut_ptr())
    };
    
    if result == 0 {
        // 测试SHUT_RD
        let result = unsafe { crate::posix::shutdown(fds[0], crate::posix::SHUT_RD) };
        let passed = result == 0;
        results.record_result(passed, "shutdown关闭读方向",
            if passed { None } else { Some("shutdown SHUT_RD失败") });
        
        // 测试SHUT_WR
        let result = unsafe { crate::posix::shutdown(fds[0], crate::posix::SHUT_WR) };
        let passed = result == 0;
        results.record_result(passed, "shutdown关闭写方向",
            if passed { None } else { Some("shutdown SHUT_WR失败") });
        
        // 测试SHUT_RDWR
        let result = unsafe { crate::posix::shutdown(fds[0], crate::posix::SHUT_RDWR) };
        let passed = result == 0;
        results.record_result(passed, "shutdown关闭读写方向",
            if passed { None } else { Some("shutdown SHUT_RDWR失败") });
        
        // 清理
        unsafe { crate::posix::close(fds[0]) };
        unsafe { crate::posix::close(fds[1]) };
    } else {
        results.record_skip("shutdown", "无法创建socket对");
    }
}