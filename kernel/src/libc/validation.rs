//! C标准库验证和测试工具
//!
//! 提供C库功能的快速验证和性能测试

extern crate alloc;
use alloc::vec::Vec;
use crate::libc::interface::CLibInterface;
use crate::libc::implementations::create_unified_c_lib;

/// 快速验证C库基本功能
pub fn quick_validation() -> Result<(), &'static str> {
    crate::println!("[libc_validation] 开始快速验证...");

    // 创建并初始化C库
    let libc = create_unified_c_lib();
    // UnifiedCLib needs initialization

    // 测试内存分配
    let ptr = unsafe { libc.malloc(1024) };
    if ptr.is_null() {
        return Err("内存分配失败");
    }

    // 测试内存释放
    unsafe { libc.free(ptr) };

    // 测试字符串操作
    let test_str = b"Hello, World!\0";
    let len = unsafe { libc.strlen(test_str.as_ptr() as *const core::ffi::c_char) };
    if len != 13 {
        return Err("字符串长度计算错误");
    }

    // 测试统计信息
    // SimpleCLib doesn't have get_stats method
    crate::println!("[libc_validation] 内存池命中率: N/A (SimpleCLib doesn't provide stats)");

    crate::println!("[libc_validation] ✅ 快速验证通过");
    Ok(())
}

/// 内存管理功能验证
pub fn memory_management_validation() -> Result<(), &'static str> {
    crate::println!("[mem_validation] 开始内存管理验证...");

    let libc = create_unified_c_lib();
    // UnifiedCLib needs initialization

    // 测试1: 基本分配和释放
    {
        let ptr = unsafe { libc.malloc(100) };
        if ptr.is_null() {
            return Err("基本分配失败");
        }
        unsafe { libc.free(ptr) };
    }

    // 测试2: realloc功能
    {
        let ptr = unsafe { libc.malloc(50) };
        if ptr.is_null() {
            return Err("realloc初始分配失败");
        }

        let new_ptr = unsafe { libc.realloc(ptr, 100) };
        if new_ptr.is_null() {
            unsafe { libc.free(ptr) };
            return Err("realloc扩展失败");
        }

        unsafe { libc.free(new_ptr) };
    }

    // 测试3: calloc清零
    {
        let ptr = unsafe { libc.calloc(10, 10) };
        if ptr.is_null() {
            return Err("calloc分配失败");
        }

        // 验证内存已清零
        unsafe {
            let bytes = core::slice::from_raw_parts(ptr as *const u8, 100);
            for &byte in bytes.iter() {
                if byte != 0 {
                    unsafe { libc.free(ptr) };
                    return Err("calloc内存未清零");
                }
            }
        }

        unsafe { libc.free(ptr) };
    }

    // 测试4: 边界情况
    {
        // 零字节分配
        let zero_ptr = unsafe { libc.malloc(0) };
        // 根据C标准，malloc(0)可以返回NULL或非NULL，我们都不应该崩溃
        unsafe { libc.free(zero_ptr) };

        // realloc NULL指针
        let realloc_null = unsafe { libc.realloc(core::ptr::null_mut(), 50) };
        if realloc_null.is_null() {
            return Err("realloc NULL失败");
        }
        unsafe { libc.free(realloc_null) };

        // realloc到零大小
        let ptr = unsafe { libc.malloc(100) };
        let zero_result = unsafe { libc.realloc(ptr, 0) };
        // realloc到0应该返回NULL并释放原内存
        if !zero_result.is_null() {
            unsafe { libc.free(zero_result) };
        }
    }

    // 打印最终统计
    // SimpleCLib doesn't have get_stats method
    // let stats = libc.get_stats();
    crate::println!("[mem_validation] 内存统计:");
    // SimpleCLib doesn't have get_stats method
    crate::println!("  - 总分配: N/A (SimpleCLib doesn't provide stats)");
    crate::println!("  - 活跃分配: N/A (SimpleCLib doesn't provide stats)");
    crate::println!("  - 内存池命中率: N/A (SimpleCLib doesn't provide stats)");

    crate::println!("[mem_validation] ✅ 内存管理验证通过");
    Ok(())
}

/// 性能基准测试
pub fn performance_benchmark() {
    crate::println!("[perf_benchmark] 开始性能基准测试...");

    let libc = create_unified_c_lib();
    if libc.initialize().is_err() {
        crate::println!("[perf_benchmark] ❌ C库初始化失败");
        return;
    }

    // 分配性能测试
    let allocations = 1000;
    let test_size = 256;

    let start_time = crate::subsystems::time::uptime_ms();
    let mut ptrs = Vec::new();

    // 分配阶段
    for _ in 0..allocations {
        let ptr = unsafe { libc.malloc(test_size) };
        if !ptr.is_null() {
            ptrs.push(ptr);
        }
    }

    let alloc_time = crate::subsystems::time::uptime_ms() - start_time;

    // 释放阶段
    let release_start = crate::subsystems::time::uptime_ms();
    for ptr in ptrs {
        unsafe { libc.free(ptr) };
    }

    let release_time = crate::subsystems::time::uptime_ms() - release_start;

    // 打印性能结果
    crate::println!("[perf_benchmark] 性能测试结果:");
    crate::println!("  - 分配操作: {} 次", allocations);
    crate::println!("  - 分配时间: {} ms", alloc_time);
    crate::println!("  - 平均分配时间: {} μs/op", (alloc_time * 1000) / allocations);
    crate::println!("  - 释放时间: {} ms", release_time);
    crate::println!("  - 平均释放时间: {} μs/op", (release_time * 1000) / allocations);

    // SimpleCLib doesn't provide stats; skip detailed metrics in this build

    crate::println!("[perf_benchmark] ✅ 性能基准测试完成");
}
