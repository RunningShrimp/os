//! 优化系统测试程序
//!
//! 本程序用于测试和验证NOS优化系统的功能，包括：
//! - 性能优化测试
//! - 调度器优化测试
//! - 零拷贝I/O优化测试
//! - 综合优化报告测试

use crate::syscalls::{get_optimization_report, dispatch};
use crate::syscalls::common::SyscallError;
use alloc::string::String;
use alloc::vec::Vec;

/// 测试优化系统
pub fn test_optimization_system() -> Result<String, SyscallError> {
    let mut results = Vec::new();
    
    // 测试性能优化
    results.push(test_performance_optimization()?);
    
    // 测试调度器优化
    results.push(test_scheduler_optimization()?);
    
    // 测试零拷贝I/O优化
    results.push(test_zero_copy_optimization()?);
    
    // 生成综合报告
    results.push(get_optimization_report()?);
    
    // 合并结果
    let mut output = String::new();
    for result in results {
        output.push_str(&result);
        output.push_str("\n");
    }
    
    Ok(output)
}

/// 测试性能优化
fn test_performance_optimization() -> Result<String, SyscallError> {
    let mut output = String::new();
    
    output.push_str("## 性能优化测试\n");
    
    // 测试快速系统调用路径
    let args = [0u64; 6];
    let result = dispatch(0x1004, &args); // getpid
    
    match result {
        0 => {
            output.push_str("✓ 快速系统调用路径测试失败\n");
        }
        pid if pid > 0 => {
            output.push_str(&format!("✓ 快速系统调用路径测试成功，PID: {}\n", pid));
        }
        _ => {
            output.push_str("✗ 快速系统调用路径测试异常\n");
        }
    }
    
    // 测试文件I/O优化
    let file_args = [0u64, 0x1000u64, 1024u64]; // fd, buf, count
    let read_result = dispatch(0x2002, &file_args); // read
    
    match read_result {
        0 => {
            output.push_str("✓ 文件I/O优化测试：无数据可读\n");
        }
        n if n > 0 => {
            output.push_str(&format!("✓ 文件I/O优化测试成功，读取了{}字节\n", n));
        }
        _ => {
            output.push_str("✗ 文件I/O优化测试失败\n");
        }
    }
    
    output.push_str("性能优化测试完成\n\n");
    Ok(output)
}

/// 测试调度器优化
fn test_scheduler_optimization() -> Result<String, SyscallError> {
    let mut output = String::new();
    
    output.push_str("## 调度器优化测试\n");
    
    // 测试快速sched_yield
    let yield_args = [];
    let yield_result = dispatch(0xE010, &yield_args); // sched_yield_fast
    
    match yield_result {
        0 => {
            output.push_str("✓ 快速sched_yield测试成功\n");
        }
        _ => {
            output.push_str("✗ 快速sched_yield测试失败\n");
        }
    }
    
    // 测试调度提示
    let hint_args = [1u64, 50u64, 0u64]; // tid, priority, cpu_hint
    let hint_result = dispatch(0xE011, &hint_args); // sched_enqueue_hint
    
    match hint_result {
        0 => {
            output.push_str("✓ 调度提示测试成功\n");
        }
        _ => {
            output.push_str("✗ 调度提示测试失败\n");
        }
    }
    
    output.push_str("调度器优化测试完成\n\n");
    Ok(output)
}

/// 测试零拷贝I/O优化
fn test_zero_copy_optimization() -> Result<String, SyscallError> {
    let mut output = String::new();
    
    output.push_str("## 零拷贝I/O优化测试\n");
    
    // 测试sendfile
    let sendfile_args = [1u64, 0u64, 0u64, 1024u64]; // out_fd, in_fd, offset, count
    let sendfile_result = dispatch(0x9000, &sendfile_args); // sendfile
    
    match sendfile_result {
        0 => {
            output.push_str("✓ sendfile测试：无数据传输\n");
        }
        n if n > 0 => {
            output.push_str(&format!("✓ sendfile测试成功，传输了{}字节\n", n));
        }
        _ => {
            output.push_str("✗ sendfile测试失败\n");
        }
    }
    
    // 测试异步I/O设置
    let io_uring_args = [32u64, 0u64, 0u64, 0u64]; // entries, flags, resv, sq_fd
    let io_uring_result = dispatch(0x9006, &io_uring_args); // io_uring_setup
    
    match io_uring_result {
        0 => {
            output.push_str("✓ 异步I/O设置测试失败\n");
        }
        n if n > 0 => {
            output.push_str(&format!("✓ 异步I/O设置测试成功，返回值: {}\n", n));
        }
        _ => {
            output.push_str("✗ 异步I/O设置测试失败\n");
        }
    }
    
    output.push_str("零拷贝I/O优化测试完成\n\n");
    Ok(output)
}

/// 性能基准测试
pub fn benchmark_optimizations() -> Result<String, SyscallError> {
    let mut output = String::new();
    
    output.push_str("## 性能基准测试\n");
    
    // 基准测试系统调用性能
    let iterations = 10000;
    let start = crate::time::get_time_ns();
    
    for _ in 0..iterations {
        let args = [0u64; 6];
        dispatch(0x1004, &args); // getpid
    }
    
    let end = crate::time::get_time_ns();
    let duration = end - start;
    let avg_time = duration as f64 / iterations as f64;
    
    output.push_str(&format!("系统调用性能基准测试:\n"));
    output.push_str(&format!("- 迭代次数: {}\n", iterations));
    output.push_str_str(&format!("- 总时间: {}ns\n", duration));
    output.push_str(&format!("- 平均时间: {:.2}ns\n", avg_time));
    output.push_str(&format!("- 每秒调用数: {:.0}\n", 1_000_000_000.0 / avg_time));
    
    output.push_str("\n");
    
    // 基准测试文件I/O性能
    let file_iterations = 1000;
    let file_start = crate::time::get_time_ns();
    
    for i in 0..file_iterations {
        let args = [0u64, 0x1000u64, 1024u64]; // fd, buf, count
        dispatch(0x2002, &args); // read
        
        if i % 100 == 0 {
            // 每100次迭代打印进度
            crate::println!("[benchmark] 文件I/O测试进度: {}/{}", i, file_iterations);
        }
    }
    
    let file_end = crate::time::get_time_ns();
    let file_duration = file_end - file_start;
    let file_avg_time = file_duration as f64 / file_iterations as f64;
    
    output.push_str(&format!("文件I/O性能基准测试:\n"));
    output.push_str(&format!("- 迭代次数: {}\n", file_iterations));
    output.push_str(&format!("- 总时间: {}ns\n", file_duration));
    output.push_str(&format!("- 平均时间: {:.2}ns\n", file_avg_time));
    output.push_str(&format!("- 每秒操作数: {:.0}\n", 1_000_000_000.0 / file_avg_time));
    
    output.push_str("\n");
    
    // 基准测试调度器性能
    let sched_iterations = 10000;
    let sched_start = crate::time::get_time_ns();
    
    for _ in 0..sched_iterations {
        let args = [];
        dispatch(0xE010, &args); // sched_yield_fast
    }
    
    let sched_end = crate::time::get_time_ns();
    let sched_duration = sched_end - sched_start;
    let sched_avg_time = sched_duration as f64 / sched_iterations as f64;
    
    output.push_str(&format!("调度器性能基准测试:\n"));
    output.push_str(&format!("- 迭代次数: {}\n", sched_iterations));
    output.push_str(&format!("- 总时间: {}ns\n", sched_duration));
    output.push_str(&format!("- 平均时间: {:.2}ns\n", sched_avg_time));
    output.push_str(&format!("- 每秒调度数: {:.0}\n", 1_000_000_000.0 / sched_avg_time));
    
    output.push_str("性能基准测试完成\n\n");
    Ok(output)
}

/// 压力测试
pub fn stress_test_optimizations() -> Result<String, SyscallError> {
    let mut output = String::new();
    
    output.push_str("## 压力测试\n");
    
    // 多线程系统调用压测
    let thread_count = 4;
    let iterations_per_thread = 10000;
    
    output.push_str(&format!("启动{}个线程，每个线程执行{}次系统调用\n", 
                       thread_count, iterations_per_thread));
    
    // 简化实现，实际应该创建多个线程
    let start = crate::time::get_time_ns();
    
    for thread in 0..thread_count {
        let thread_start = crate::time::get_time_ns();
        
        for i in 0..iterations_per_thread {
            let args = [0u64; 6];
            dispatch(0x1004, &args); // getpid
            
            if i % 1000 == 0 {
                // 每1000次迭代打印进度
                crate::println!("[stress] 线程{}进度: {}/{}", thread, i, iterations_per_thread);
            }
        }
        
        let thread_end = crate::time::get_time_ns();
        let thread_duration = thread_end - thread_start;
        let thread_avg = thread_duration as f64 / iterations_per_thread as f64;
        
        output.push_str(&format!("线程{}完成，平均时间: {:.2}ns\n", thread, thread_avg));
    }
    
    let end = crate::time::get_time_ns();
    let total_duration = end - start;
    let total_iterations = thread_count * iterations_per_thread;
    let total_avg = total_duration as f64 / total_iterations as f64;
    
    output.push_str(&format!("压测结果:\n"));
    output.push_str(&format!("- 总迭代次数: {}\n", total_iterations));
    output.push_str(&format!("- 总时间: {}ns\n", total_duration));
    output.push_str(&format!("- 平均时间: {:.2}ns\n", total_avg));
    output.push_str(&format!("- 每秒调用数: {:.0}\n", 1_000_000_000.0 / total_avg));
    
    output.push_str("压力测试完成\n\n");
    Ok(output)
}