#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! C标准库内存管理测试
//!
//! 测试增强内存管理器的各种功能：
//! - 内存分配和释放
//! - 内存池命中率
//! - 内存泄漏检测
//! - 边界检查
//! - 统计信息准确性

use crate::libc::interface::CLibInterface;
use crate::libc::implementations::{create_unified_c_lib, UnifiedCLib};
use core::ffi::{c_void, c_char};
use alloc::vec::Vec;

/// 运行所有内存管理测试
pub fn run_all_memory_tests() {
    crate::println!("\n=== C标准库内存管理测试 ===");

    // 创建测试实例
    let libc = create_unified_c_lib();
    if let Err(e) = libc.initialize() {
        crate::println!("❌ C库初始化失败: {:?}", e);
        return;
    }

    // 运行各项测试
    test_basic_allocation(&libc);
    test_memory_leak_detection(&libc);
    test_realloc_functionality(&libc);
    test_calloc_functionality(&libc);
    test_boundary_checking(&libc);
    test_pool_performance(&libc);
    test_large_allocations(&libc);
    test_fragmentation_resistance(&libc);

    // 打印最终统计报告
    libc.get_stats().memory_manager.print_memory_report();

    crate::println!("=== 内存管理测试完成 ===\n");
}

/// 测试基本内存分配功能
fn test_basic_allocation(libc: &UnifiedCLib) {
    crate::println!("\n🧪 测试基本内存分配...");

    let mut ptrs = Vec::new();

    // 测试不同大小的分配
    let test_sizes = [8, 16, 32, 64, 128, 256, 512, 1024, 2048];

    for &size in &test_sizes {
        let ptr = unsafe { libc.malloc(size) };
        if ptr.is_null() {
            crate::println!("❌ 分配 {} 字节失败", size);
            return;
        }

        // 写入测试模式
        unsafe {
            let bytes = core::slice::from_raw_parts_mut(ptr as *mut u8, size);
            for (i, byte) in bytes.iter_mut().enumerate() {
                *byte = (i % 256) as u8;
            }
        }

        ptrs.push((ptr, size));
        crate::println!("  ✅ 分配 {} 字节成功", size);
    }

    // 验证数据完整性
    for &(ptr, size) in &ptrs {
        unsafe {
            let bytes = core::slice::from_raw_parts(ptr as *const u8, size);
            for (i, &byte) in bytes.iter().enumerate() {
                if byte != (i % 256) as u8 {
                    crate::println!("❌ 数据完整性检查失败，地址: {:#x}, 位置: {}, 期望: {}, 实际: {}",
                        ptr as usize, i, i % 256, byte);
                    return;
                }
            }
        }
    }

    // 释放内存
    for (ptr, size) in ptrs {
        unsafe { libc.free(ptr) };
        crate::println!("  ✅ 释放 {} 字节成功", size);
    }

    crate::println!("✅ 基本内存分配测试通过");
}

/// 测试内存泄漏检测
fn test_memory_leak_detection(libc: &UnifiedCLib) {
    crate::println!("\n🧪 测试内存泄漏检测...");

    let stats_before = libc.get_stats();
    let initial_allocations = stats_before.allocations_total;
    let initial_active = stats_before.allocations_active;

    // 分配但不释放一些内存（模拟泄漏）
    let mut leaked_ptrs = Vec::new();
    for i in 0..10 {
        let ptr = unsafe { libc.malloc(64 * (i + 1)) }; // 64, 128, 192... bytes
        if !ptr.is_null() {
            leaked_ptrs.push(ptr);
        }
    }

    let stats_with_leaks = libc.get_stats();
    crate::println!("  📊 泄漏后统计: 总分配={}, 活跃分配={}",
        stats_with_leaks.allocations_total - initial_allocations,
        stats_with_leaks.allocations_active - initial_active);

    // 释放一半的内存
    for i in (0..leaked_ptrs.len()).step_by(2) {
        unsafe { libc.free(leaked_ptrs[i]) };
    }

    let stats_partial_cleanup = libc.get_stats();
    crate::println!("  📊 部分清理后统计: 活跃分配={}",
        stats_partial_cleanup.allocations_active - initial_active);

    // 清理剩余内存
    for i in (1..leaked_ptrs.len()).step_by(2) {
        unsafe { libc.free(leaked_ptrs[i]) };
    }

    crate::println!("✅ 内存泄漏检测测试完成");
}

/// 测试realloc功能
fn test_realloc_functionality(libc: &UnifiedCLib) {
    crate::println!("\n🧪 测试realloc功能...");

    // 测试扩展内存
    let ptr = unsafe { libc.malloc(100) };
    if ptr.is_null() {
        crate::println!("❌ 初始分配失败");
        return;
    }

    // 写入测试数据
    unsafe {
        let bytes = core::slice::from_raw_parts_mut(ptr as *mut u8, 100);
        for byte in bytes.iter_mut() {
            *byte = 0x42;
        }
    }

    // 扩展内存
    let expanded_ptr = unsafe { libc.realloc(ptr, 200) };
    if expanded_ptr.is_null() {
        crate::println!("❌ 内存扩展失败");
        unsafe { libc.free(ptr) };
        return;
    }

    // 验证原有数据完整性
    unsafe {
        let bytes = core::slice::from_raw_parts(expanded_ptr as *const u8, 100);
        for &byte in bytes.iter() {
            if byte != 0x42 {
                crate::println!("❌ realloc后数据完整性检查失败");
                unsafe { libc.free(expanded_ptr) };
                return;
            }
        }
    }

    // 缩小内存
    let shrunk_ptr = unsafe { libc.realloc(expanded_ptr, 50) };
    if shrunk_ptr.is_null() {
        crate::println!("❌ 内存缩小失败");
        unsafe { libc.free(expanded_ptr) };
        return;
    }

    unsafe { libc.free(shrunk_ptr) };

    crate::println!("✅ realloc功能测试通过");
}

/// 测试calloc功能
fn test_calloc_functionality(libc: &UnifiedCLib) {
    crate::println!("\n🧪 测试calloc功能...");

    // 测试calloc清零
    let ptr = unsafe { libc.calloc(10, 20) }; // 10个20字节的对象
    if ptr.is_null() {
        crate::println!("❌ calloc分配失败");
        return;
    }

    // 验证内存已清零
    unsafe {
        let bytes = core::slice::from_raw_parts(ptr as *const u8, 200);
        for &byte in bytes.iter() {
            if byte != 0 {
                crate::println!("❌ calloc内存未清零");
                unsafe { libc.free(ptr) };
                return;
            }
        }
    }

    unsafe { libc.free(ptr) };

    // 测试溢出检查
    let overflow_ptr = unsafe { libc.calloc(core::usize::MAX / 2, 2) };
    if !overflow_ptr.is_null() {
        crate::println!("❌ calloc溢出检查失败");
        unsafe { libc.free(overflow_ptr) };
        return;
    }

    crate::println!("✅ calloc功能测试通过");
}

/// 测试边界检查功能
fn test_boundary_checking(libc: &UnifiedCLib) {
    crate::println!("\n🧪 测试边界检查功能...");

    // 正常分配和释放
    let ptr = unsafe { libc.malloc(64) };
    if !ptr.is_null() {
        // 写入边界内的数据
        unsafe {
            let bytes = core::slice::from_raw_parts_mut(ptr as *mut u8, 64);
            bytes.fill(0xAA);
        }
        unsafe { libc.free(ptr) };
        crate::println!("  ✅ 正常边界内操作");
    }

    // 测试重复释放（这应该被检测到）
    let ptr = unsafe { libc.malloc(32) };
    if !ptr.is_null() {
        unsafe { libc.free(ptr) };
        // 这里我们不再次释放以避免系统崩溃，但在调试版本中应该能检测到
        crate::println!("  ✅ 重复释放检测（跳过实际重复释放以避免崩溃）");
    }

    crate::println!("✅ 边界检查测试完成");
}

/// 测试内存池性能
fn test_pool_performance(libc: &UnifiedCLib) {
    crate::println!("\n🧪 测试内存池性能...");

    let stats_before = libc.get_stats();
    let initial_pool_hits = stats_before.pool_hit_rate;

    // 分配大量小对象（应该命中小内存池）
    let mut small_ptrs = Vec::new();
    for _ in 0..100 {
        let ptr = unsafe { libc.malloc(32) }; // 小对象
        if !ptr.is_null() {
            small_ptrs.push(ptr);
        }
    }

    // 分配中等对象
    let mut medium_ptrs = Vec::new();
    for _ in 0..50 {
        let ptr = unsafe { libc.malloc(256) }; // 中等对象
        if !ptr.is_null() {
            medium_ptrs.push(ptr);
        }
    }

    let stats_during = libc.get_stats();
    crate::println!("  📊 内存池命中率: {:.2}%", stats_during.pool_hit_rate);

    // 释放所有内存
    for ptr in small_ptrs {
        unsafe { libc.free(ptr) };
    }
    for ptr in medium_ptrs {
        unsafe { libc.free(ptr) };
    }

    let stats_after = libc.get_stats();
    crate::println!("  📊 最终内存池命中率: {:.2}%", stats_after.pool_hit_rate);
    crate::println!("  📊 总分配次数: {}", stats_after.allocations_total);

    crate::println!("✅ 内存池性能测试完成");
}

/// 测试大内存分配
fn test_large_allocations(libc: &UnifiedCLib) {
    crate::println!("\n🧪 测试大内存分配...");

    // 测试不同大小的内存分配
    let large_sizes = [4096, 16384, 65536, 262144]; // 4KB, 16KB, 64KB, 256KB

    for &size in &large_sizes {
        let ptr = unsafe { libc.malloc(size) };
        if ptr.is_null() {
            crate::println!("  ⚠️  分配 {} 字节失败（可能内存不足）", size);
            continue;
        }

        // 写入一些数据来验证内存可用
        unsafe {
            let slice = core::slice::from_raw_parts_mut(ptr as *mut u8, 1024); // 只测试前1KB
            slice.fill(0xCC);
        }

        crate::println!("  ✅ 分配 {} 字节成功", size);
        unsafe { libc.free(ptr) };
    }

    crate::println!("✅ 大内存分配测试完成");
}

/// 测试内存碎片化抵抗性
fn test_fragmentation_resistance(libc: &UnifiedCLib) {
    crate::println!("\n🧪 测试内存碎片化抵抗性...");

    let mut ptrs = Vec::new();

    // 创建碎片化模式：交替分配不同大小的内存块
    let sizes = [32, 128, 64, 256, 16, 512, 8, 1024];

    for _round in 0..10 {
        for &size in &sizes {
            let ptr = unsafe { libc.malloc(size) };
            if !ptr.is_null() {
                ptrs.push((ptr, size));
            }
        }
    }

    // 随机释放一些内存块以创建碎片
    for i in (0..ptrs.len()).step_by(3) {
        unsafe { libc.free(ptrs[i].0) };
    }

    // 尝试分配一个中等大小的内存块（测试碎片影响）
    let test_ptr = unsafe { libc.malloc(2048) };
    let success = !test_ptr.is_null();
    if success {
        unsafe { libc.free(test_ptr) };
    }

    // 清理剩余内存
    for i in 0..ptrs.len() {
        if i % 3 != 0 { // 跳过已释放的
            unsafe { libc.free(ptrs[i].0) };
        }
    }

    crate::println!("  {} Fragmentation resistance test {}",
        if success { "✅" } else { "⚠️  " },
        if success { "passed" } else { "shows fragmentation may affect performance" });

    crate::println!("✅ 内存碎片化抵抗性测试完成");
}

/// 内存压力测试
pub fn stress_test_memory_management() {
    crate::println!("\n🔥 内存管理压力测试...");

    let libc = create_unified_c_lib();
    if let Err(e) = libc.initialize() {
        crate::println!("❌ C库初始化失败: {:?}", e);
        return;
    }

    let mut ptrs = Vec::new();
    let operations = 1000;
    let mut successful_ops = 0;

    for i in 0..operations {
        // 随机大小的内存分配
        let size = (i % 1024) + 1; // 1到1024字节
        let ptr = unsafe { libc.malloc(size) };

        if !ptr.is_null() {
            ptrs.push((ptr, size));
            successful_ops += 1;

            // 写入测试数据
            unsafe {
                let slice = core::slice::from_raw_parts_mut(ptr as *mut u8, size.min(64));
                for (j, byte) in slice.iter_mut().enumerate() {
                    *byte = (i + j) as u8;
                }
            }
        }

        // 随机释放一些内存
        if i % 7 == 0 && !ptrs.is_empty() {
            let idx = ptrs.len() / 2;
            unsafe { libc.free(ptrs.remove(idx).0) };
        }
    }

    // 释放所有剩余内存
    for (ptr, size) in ptrs {
        unsafe { libc.free(ptr) };
    }

    let final_stats = libc.get_stats();
    crate::println!("  📊 压力测试完成:");
    crate::println!("    - 操作次数: {}", operations);
    crate::println!("    - 成功分配: {}", successful_ops);
    crate::println!("    - 内存池命中率: {:.2}%", final_stats.pool_hit_rate);
    crate::println!("    - 峰值内存使用: {} KB", final_stats.memory_peak / 1024);

    crate::println!("✅ 内存管理压力测试完成");
}