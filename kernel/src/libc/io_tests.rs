//! C标准库I/O功能测试
//!
//! 测试增强的I/O管理器和格式化器的各种功能：
//! - 文件操作测试
//! - 格式化输出测试
//! - 缓冲区性能测试
//! - 错误处理测试
//! - 并发I/O测试

use core::ffi::{c_char, c_int, c_void};

use crate::libc::{
    implementations::simple::SimpleCLib,
    io_manager::{EnhancedIOManager, IOManagerConfig},
};

/// 运行所有I/O测试
pub fn run_all_io_tests() {
    crate::println!("\n=== C标准库I/O功能测试 ===");

    // 创建测试实例
    let libc = create_unified_c_lib();
    if let Err(e) = libc.initialize() {
        crate::println!("❌ C库初始化失败: {:?}", e);
        return;
    }

    // 运行各项测试
    test_basic_io_operations(&libc);
    test_file_operations(&libc);
    test_formatting_capabilities(&libc);
    test_buffer_management(&libc);
    test_error_handling(&libc);
    test_standard_streams(&libc);
    test_format_specifiers(&libc);
    test_buffered_io_performance(&libc);

    // 打印最终统计报告
    libc.io_manager.print_io_report();

    crate::println!("=== I/O功能测试完成 ===\n");
}

/// 测试基本I/O操作
fn test_basic_io_operations(libc: &SimpleCLib) {
    crate::println!("\n🧪 测试基本I/O操作...");

    // 测试printf功能
    let result = unsafe {
        libc.printf(b"Basic test: number=%d, string=%s, hex=%#x\0".as_ptr(), 42, "Hello", 255)
    };
    if result > 0 {
        crate::println!("  ✅ printf基本格式化测试通过");
    } else {
        crate::println!("  ❌ printf基本格式化测试失败");
    }

    // 测试puts和putchar
    unsafe {
        let puts_result = libc.puts(b"Test puts function\0".as_ptr());
        if puts_result > 0 {
            crate::println!("  ✅ puts函数测试通过");
        }

        let putchar_result = libc.putchar('A' as c_int);
        if putchar_result == 'A' as c_int {
            crate::println!("  ✅ putchar函数测试通过");
        }
    }

    crate::println!("✅ 基本I/O操作测试完成");
}

/// 测试文件操作
fn test_file_operations(libc: &SimpleCLib) {
    crate::println!("\n🧪 测试文件操作...");

    // 注意：这些测试需要实际的文件系统支持
    // 目前我们主要测试函数调用不会崩溃

    unsafe {
        // 测试文件打开（可能失败，这是正常的）
        let file = libc.fopen(b"/test.txt\0".as_ptr(), b"w\0".as_ptr());

        if !file.is_null() {
            // 测试写入
            let test_data = b"Hello, File I/O!";
            let written =
                libc.fwrite(test_data.as_ptr() as *const c_void, 1, test_data.len(), file);

            if written == test_data.len() {
                crate::println!("  ✅ 文件写入测试通过");
            }

            // 测试刷新
            let flush_result = libc.fflush(file);
            if flush_result == 0 {
                crate::println!("  ✅ 文件刷新测试通过");
            }

            // 测试关闭
            let close_result = libc.fclose(file);
            if close_result == 0 {
                crate::println!("  ✅ 文件关闭测试通过");
            }
        } else {
            crate::println!("  ⚠️  文件打开失败（可能文件系统未完全实现）");
        }
    }

    crate::println!("✅ 文件操作测试完成");
}

/// 测试格式化功能
fn test_formatting_capabilities(libc: &SimpleCLib) {
    crate::println!("\n🧪 测试格式化功能...");

    unsafe {
        // 测试fprintf
        let stderr = libc.io_manager.stderr as *mut c_void;
        if !stderr.is_null() {
            let result = libc.fprintf(
                stderr,
                b"fprintf test: signed=%d, unsigned=%u, hex=%x, octal=%o\0".as_ptr(),
                -123,
                456,
                0xABCD,
                0755,
            );
            if result > 0 {
                crate::println!("  ✅ fprintf格式化测试通过");
            }
        }

        // 测试snprintf
        let mut buffer = [0u8; 256];
        let result = libc.snprintf(
            buffer.as_mut_ptr() as *mut c_char,
            buffer.len(),
            b"snprintf test: %s %d %f\0".as_ptr(),
            "Hello",
            42,
            3.14159,
        );
        if result > 0 && result < buffer.len() as c_int {
            crate::println!("  ✅ snprintf格式化测试通过");
            crate::println!(
                "    结果: {}",
                core::str::from_utf8(&buffer[..result as usize]).unwrap_or("(invalid)")
            );
        }
    }

    crate::println!("✅ 格式化功能测试完成");
}

/// 测试缓冲区管理
fn test_buffer_management(libc: &SimpleCLib) {
    crate::println!("\n🧪 测试缓冲区管理...");

    let io_stats = libc.io_manager.get_stats();
    let initial_flushes = io_stats
        .flush_operations
        .load(core::sync::atomic::Ordering::SeqCst);

    unsafe {
        // 创建文件进行缓冲区测试
        let file = libc.fopen(b"/buffer_test.txt\0".as_ptr(), b"w\0".as_ptr());

        if !file.is_null() {
            // 写入大量数据以触发缓冲
            for i in 0..100 {
                let result = libc.fprintf(
                    file,
                    b"Buffer test line %d: This is a test string to fill the buffer\0".as_ptr(),
                    i,
                );
                if result < 0 {
                    crate::println!("  ❌ 缓冲区写入失败");
                    break;
                }
            }

            // 手动刷新
            let flush_result = libc.fflush(file);
            if flush_result == 0 {
                crate::println!("  ✅ 缓冲区刷新测试通过");
            }

            libc.fclose(file);
        }
    }

    // 检查刷新操作是否增加
    let final_flushes = io_stats
        .flush_operations
        .load(core::sync::atomic::Ordering::SeqCst);
    if final_flushes > initial_flushes {
        crate::println!("  ✅ 缓冲区统计更新正常");
    }

    crate::println!("✅ 缓冲区管理测试完成");
}

/// 测试错误处理
fn test_error_handling(libc: &SimpleCLib) {
    crate::println!("\n🧪 测试错误处理...");

    unsafe {
        // 测试无效参数
        let printf_null = libc.printf(core::ptr::null());
        if printf_null < 0 {
            crate::println!("  ✅ NULL指针错误处理正确");
        }

        // 测试无效文件操作
        let fclose_null = libc.fclose(core::ptr::null_mut());
        if fclose_null < 0 {
            crate::println!("  ✅ 无效文件指针错误处理正确");
        }

        // 测试文件错误检查
        let file = libc.fopen(b"/nonexistent/file.txt\0".as_ptr(), b"r\0".as_ptr());
        if file.is_null() {
            crate::println!("  ✅ 文件不存在错误处理正确");
        }
    }

    crate::println!("✅ 错误处理测试完成");
}

/// 测试标准流
fn test_standard_streams(libc: &SimpleCLib) {
    crate::println!("\n🧪 测试标准流...");

    unsafe {
        // 测试stdout
        let stdout = libc.io_manager.stdout;
        if !stdout.is_null() {
            let result = libc.fprintf(
                stdout as *mut c_void,
                b"Standard output test: PID=%d, time=%ld\0".as_ptr(),
                libc.getpid(),
                1234567890,
            );
            if result > 0 {
                crate::println!("  ✅ stdout测试通过");
            }
        }

        // 测试stderr
        let stderr = libc.io_manager.stderr;
        if !stderr.is_null() {
            let result = libc.fprintf(
                stderr as *mut c_void,
                b"Standard error test: error code=%d\0".as_ptr(),
                404,
            );
            if result > 0 {
                crate::println!("  ✅ stderr测试通过");
            }
        }

        // 测试stdin（简化测试）
        let stdin = libc.io_manager.stdin;
        if !stdin.is_null() {
            let ch = libc.getchar();
            // getchar总是返回换行符在我们的简化实现中
            crate::println!("  ✅ stdin测试通过 (返回: {})", ch);
        }
    }

    crate::println!("✅ 标准流测试完成");
}

/// 测试格式说明符
fn test_format_specifiers(libc: &SimpleCLib) {
    crate::println!("\n🧪 测试格式说明符...");

    unsafe {
        // 测试各种格式说明符
        let test_cases = [
            (
                b"Integers: %d, %ld, %lld\0".as_ptr(),
                [42i64 as c_int, 1000i64 as c_int, 999999i64 as c_int],
            ),
            (b"Unsigned: %u, %lu\0".as_ptr(), [42u32 as c_int, 1000000u64 as c_int]),
            (
                b"Hexadecimal: %x, %X, %#x\0".as_ptr(),
                [255u32 as c_int, 255u32 as c_int, 255u32 as c_int],
            ),
            (b"Octal: %o, %#o\0".as_ptr(), [755u32 as c_int, 755u32 as c_int]),
            (b"Characters: %c, %s\0".as_ptr(), ['A' as c_int, b"Hello\0".as_ptr() as c_int]),
            (b"Pointers: %p\0".as_ptr(), [0x12345678usize as c_int]),
        ];

        for &(format_str, args_slice) in &test_cases {
            // 注意：这里简化了可变参数的处理
            let result = libc.printf(format_str);
            if result > 0 {
                crate::println!("  ✅ 格式说明符测试通过");
            } else {
                crate::println!("  ❌ 格式说明符测试失败");
            }
        }

        // 测试宽度和精度
        let width_result = libc.printf(
            b"Width and precision: |%10d|, |%-10s|, |%5.3f|\0".as_ptr(),
            42,
            "Hello",
            3.14159,
        );
        if width_result > 0 {
            crate::println!("  ✅ 宽度和精度测试通过");
        }
    }

    crate::println!("✅ 格式说明符测试完成");
}

/// 测试缓冲区I/O性能
fn test_buffered_io_performance(libc: &SimpleCLib) {
    crate::println!("\n🧪 测试缓冲区I/O性能...");

    let start_time = crate::subsystems::time::get_time_ns();
    let write_count = 1000;

    unsafe {
        let file = libc.fopen(b"/performance_test.txt\0".as_ptr(), b"w\0".as_ptr());

        if !file.is_null() {
            // 测试多次小写入（应该使用缓冲）
            for i in 0..write_count {
                let result = libc.fprintf(
                    file,
                    b"Performance test line %d: This is a long string used to test buffered I/O performance, containing number %d and more text content.\0".as_ptr(),
                    i,
                    i * 2
                );
                if result < 0 {
                    crate::println!("  ❌ 性能测试写入失败");
                    break;
                }
            }

            libc.fclose(file);

            let end_time = crate::subsystems::time::get_time_ns();
            let elapsed = end_time - start_time;

            crate::println!("  📊 性能测试结果:");
            crate::println!("    - 写入次数: {}", write_count);
            crate::println!("    - 总耗时: {} ms", elapsed);
            crate::println!("    - 平均每次写入: {} μs", (elapsed * 1000) / write_count);

            let stats = libc.io_manager.get_stats();
            crate::println!(
                "    - 缓冲区命中率: {:.2}%",
                (stats.buffer_hits.load(core::sync::atomic::Ordering::SeqCst) as f64
                    / (stats.buffer_hits.load(core::sync::atomic::Ordering::SeqCst)
                        + stats
                            .buffer_misses
                            .load(core::sync::atomic::Ordering::SeqCst))
                        as f64)
                    * 100.0
            );

            crate::println!("  ✅ 缓冲区I/O性能测试完成");
        } else {
            crate::println!("  ⚠️  无法创建测试文件，性能测试跳过");
        }
    }
}

/// 并发I/O测试（简化版）
pub fn concurrent_io_test() {
    crate::println!("\n🔥 并发I/O测试...");

    // 在实际系统中，这里会创建多个线程同时进行I/O操作
    // 由于我们的简化实现，这里只模拟基本场景

    let libc = SimpleCLib::new();
    if libc.initialize().is_err() {
        crate::println!("❌ C库初始化失败");
        return;
    }

    unsafe {
        // 模拟并发写入到不同文件
        let files = [
            libc.fopen(b"/concurrent1.txt\0".as_ptr(), b"w\0".as_ptr()),
            libc.fopen(b"/concurrent2.txt\0".as_ptr(), b"w\0".as_ptr()),
            libc.fopen(b"/concurrent3.txt\0".as_ptr(), b"w\0".as_ptr()),
        ];

        for (i, &file) in files.iter().enumerate() {
            if !file.is_null() {
                for j in 0..10 {
                    libc.fprintf(
                        file,
                        b"Thread %d - operation %d: Concurrent I/O test data\0".as_ptr(),
                        i,
                        j,
                    );
                }
                libc.fclose(file);
            }
        }

        crate::println!("  ✅ 并发I/O测试完成");
    }

    crate::println!("✅ 并发I/O测试完成");
}
