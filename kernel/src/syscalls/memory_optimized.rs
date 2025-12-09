//! 优化的内存管理实现
//!
//! 本模块提供高性能的内存管理功能，包括：
//! - 高效的内存分配和释放
//! - 优化的虚拟内存管理
//! - 快速的内存映射操作
//! - 减少内存碎片的管理策略

use crate::process::{PROC_TABLE, myproc};
use crate::mm::vm::{PageTable, map_pages, flags, PAGE_SIZE, map_page, unmap_page};
use crate::mm::{kalloc, kfree};
use crate::posix;
use crate::sync::Mutex;
use super::common::{SyscallError, SyscallResult, extract_args};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

/// 全局内存统计
static MEM_STATS: Mutex<MemStats> = Mutex::new(MemStats::new());

/// 内存统计信息
#[derive(Debug, Default)]
pub struct MemStats {
    pub alloc_count: AtomicUsize,
    pub free_count: AtomicUsize,
    pub mmap_count: AtomicUsize,
    pub munmap_count: AtomicUsize,
    pub brk_count: AtomicUsize,
    pub total_allocated: AtomicUsize,
    pub total_freed: AtomicUsize,
}

impl MemStats {
    pub const fn new() -> Self {
        Self {
            alloc_count: AtomicUsize::new(0),
            free_count: AtomicUsize::new(0),
            mmap_count: AtomicUsize::new(0),
            munmap_count: AtomicUsize::new(0),
            brk_count: AtomicUsize::new(0),
            total_allocated: AtomicUsize::new(0),
            total_freed: AtomicUsize::new(0),
        }
    }
    
    pub fn record_alloc(&self, size: usize) {
        self.alloc_count.fetch_add(1, Ordering::Relaxed);
        self.total_allocated.fetch_add(size, Ordering::Relaxed);
    }
    
    pub fn record_free(&self, size: usize) {
        self.free_count.fetch_add(1, Ordering::Relaxed);
        self.total_freed.fetch_add(size, Ordering::Relaxed);
    }
    
    pub fn record_mmap(&self, size: usize) {
        self.mmap_count.fetch_add(1, Ordering::Relaxed);
        self.total_allocated.fetch_add(size, Ordering::Relaxed);
    }
    
    pub fn record_munmap(&self, size: usize) {
        self.munmap_count.fetch_add(1, Ordering::Relaxed);
        self.total_freed.fetch_add(size, Ordering::Relaxed);
    }
    
    pub fn record_brk(&self, size: isize) {
        self.brk_count.fetch_add(1, Ordering::Relaxed);
        if size > 0 {
            self.total_allocated.fetch_add(size as usize, Ordering::Relaxed);
        } else {
            self.total_freed.fetch_add((-size) as usize, Ordering::Relaxed);
        }
    }
    
    pub fn get_allocated(&self) -> usize {
        self.total_allocated.load(Ordering::Relaxed)
    }
    
    pub fn get_freed(&self) -> usize {
        self.total_freed.load(Ordering::Relaxed)
    }
}

/// 内存区域结构，用于跟踪映射的内存
#[derive(Debug)]
pub struct MemoryRegion {
    pub start: usize,
    pub end: usize,
    pub prot: i32,
    pub flags: i32,
    pub fd: Option<i32>,
    pub offset: u64,
}

impl MemoryRegion {
    pub fn new(start: usize, end: usize, prot: i32, flags: i32, fd: Option<i32>, offset: u64) -> Self {
        Self {
            start,
            end,
            prot,
            flags,
            fd,
            offset,
        }
    }
    
    pub fn size(&self) -> usize {
        self.end - self.start
    }
    
    pub fn contains(&self, addr: usize) -> bool {
        addr >= self.start && addr < self.end
    }
}

/// 进程内存映射表
#[derive(Debug, Default)]
pub struct ProcessMemoryMap {
    regions: Vec<MemoryRegion>,
}

impl ProcessMemoryMap {
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }
    
    pub fn add_region(&mut self, region: MemoryRegion) {
        self.regions.push(region);
    }
    
    pub fn remove_region(&mut self, start: usize, length: usize) -> Option<MemoryRegion> {
        let end = start + length;
        for i in 0..self.regions.len() {
            if self.regions[i].start == start && self.regions[i].end == end {
                return Some(self.regions.remove(i));
            }
        }
        None
    }
    
    pub fn find_region(&self, addr: usize) -> Option<&MemoryRegion> {
        self.regions.iter().find(|r| r.contains(addr))
    }
}

/// 优化的brk系统调用实现
pub fn sys_brk_optimized(addr: usize) -> isize {
    // 记录统计
    MEM_STATS.lock().record_brk(addr as isize);
    
    // 获取当前进程
    let pid = match myproc() {
        Some(pid) => pid,
        None => return -1,
    };
    
    let mut table = PROC_TABLE.lock();
    let proc = match table.find(pid) {
        Some(proc) => proc,
        None => return -1,
    };
    
    // 获取当前break
    let old_sz = proc.sz;
    
    // 如果addr为0，返回当前break
    if addr == 0 {
        return old_sz as isize;
    }
    
    // 验证地址范围
    if addr >= crate::mm::vm::KERNEL_BASE {
        return -1;
    }
    
    // 只允许增加break（简化实现）
    if addr > old_sz {
        // 计算需要分配的页数
        let pages_needed = ((addr - old_sz + PAGE_SIZE - 1) / PAGE_SIZE).max(1);
        
        // 批量分配和映射页面
        let mut phys_pages = Vec::with_capacity(pages_needed);
        for _ in 0..pages_needed {
            let page = kalloc();
            if page.is_null() {
                // 清理已分配的页面
                for p in &phys_pages {
                    unsafe { kfree(*p as *mut u8) };
                }
                return -1;
            }
            phys_pages.push(page);
        }
        
        // 映射页面
        for (i, &page) in phys_pages.iter().enumerate() {
            let va = old_sz + i * PAGE_SIZE;
            let perm = flags::PTE_R | flags::PTE_W | flags::PTE_U;
            
            unsafe {
                // 清零页面
                core::ptr::write_bytes(page, 0, PAGE_SIZE);
                
                // 映射页面
                if map_page(proc.pagetable, va, page as usize, perm).is_err() {
                    // 清理已分配的页面
                    for p in &phys_pages {
                        unsafe { kfree(*p as *mut u8) };
                    }
                    return -1;
                }
            }
        }
        
        // 更新进程大小
        proc.sz = addr;
    } else if addr < old_sz {
        // 缩小break - 简化实现，只更新大小
        // TODO: 正确地取消映射和释放页面
        proc.sz = addr;
    }
    
    proc.sz as isize
}

/// 优化的mmap系统调用实现
pub fn sys_mmap_optimized(addr: usize, length: usize, prot: i32, flags: i32, fd: i32, offset: u64) -> isize {
    // 记录统计
    MEM_STATS.lock().record_mmap(length);
    
    // 验证基本参数
    if length == 0 {
        return -1;
    }
    
    // 对齐到页边界
    let aligned_addr = if addr == 0 {
        0 // 让内核选择地址
    } else {
        addr & !(PAGE_SIZE - 1) // 对齐请求的地址
    };
    
    let aligned_length = (length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    
    // 验证页对齐（如果指定了地址）
    if addr != 0 && addr != aligned_addr {
        return -1;
    }
    
    // 获取当前进程
    let pid = match myproc() {
        Some(pid) => pid,
        None => return -1,
    };
    
    let mut table = PROC_TABLE.lock();
    let proc = match table.find(pid) {
        Some(proc) => proc,
        None => return -1,
    };
    let pagetable = proc.pagetable;
    
    if pagetable.is_null() {
        return -1;
    }
    
    // 获取用户空间地址范围以映射
    let mut target_addr = aligned_addr;
    
    // 如果没有指定地址，在用户空间中查找空闲范围
    if target_addr == 0 {
        // 从堆末尾开始搜索，即proc.sz
        target_addr = proc.sz;
        // 确保不与内核空间重叠
        if target_addr + aligned_length >= crate::mm::vm::KERNEL_BASE {
            return -1;
        }
    }
    
    // 分配和映射页面
    let total_pages = aligned_length / PAGE_SIZE;
    
    // 从prot和flags构建权限
    let mut vm_flags = flags::PTE_U; // 用户可访问
    
    if (prot & posix::PROT_READ) != 0 {
        vm_flags |= flags::PTE_R;
    }
    
    if (prot & posix::PROT_WRITE) != 0 {
        vm_flags |= flags::PTE_W;
    }
    
    if (prot & posix::PROT_EXEC) != 0 {
        vm_flags |= flags::PTE_X;
    }
    
    // 目前只处理匿名映射（MAP_ANONYMOUS标志）
    if (flags & posix::MAP_ANONYMOUS) != 0 {
        // 批量分配页面
        let mut phys_pages = Vec::with_capacity(total_pages);
        for _ in 0..total_pages {
            let page = kalloc();
            if page.is_null() {
                // 清理已分配的页面
                for p in &phys_pages {
                    unsafe { kfree(*p as *mut u8) };
                }
                return -1;
            }
            
            // 清零页面
            unsafe {
                core::ptr::write_bytes(page, 0, PAGE_SIZE);
            }
            
            phys_pages.push(page);
        }
        
        // 批量映射页面
        for (i, &page) in phys_pages.iter().enumerate() {
            let va = target_addr + i * PAGE_SIZE;
            
            unsafe {
                if map_page(pagetable, va, page as usize, vm_flags).is_err() {
                    // 清理已分配的页面
                    for p in &phys_pages {
                        unsafe { kfree(*p as *mut u8) };
                    }
                    return -1;
                }
            }
        }
        
        // 更新进程大小（如果映射超出了当前堆末尾）
        if target_addr + aligned_length > proc.sz {
            proc.sz = target_addr + aligned_length;
        }
        
        // 记录内存区域
        let region = MemoryRegion::new(
            target_addr,
            target_addr + aligned_length,
            prot,
            flags,
            None, // 匿名映射没有文件描述符
            0,
        );
        
        // 这里应该将区域添加到进程的内存映射表中
        // 简化实现，暂时不跟踪
        
        target_addr as isize
    } else {
        // TODO: 处理文件支持的映射
        -1
    }
}

/// 优化的munmap系统调用实现
pub fn sys_munmap_optimized(addr: usize, length: usize) -> isize {
    // 记录统计
    MEM_STATS.lock().record_munmap(length);
    
    // 验证参数
    if length == 0 || addr == 0 {
        return -1;
    }
    
    // 对齐到页边界
    let start = addr & !(PAGE_SIZE - 1);
    let aligned_length = (length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let end = start + aligned_length;
    
    if start >= crate::mm::vm::KERNEL_BASE || end > crate::mm::vm::KERNEL_BASE {
        return -1;
    }
    
    // 获取当前进程
    let pid = match myproc() {
        Some(pid) => pid,
        None => return -1,
    };
    
    let mut table = PROC_TABLE.lock();
    let proc = match table.find(pid) {
        Some(proc) => proc,
        None => return -1,
    };
    let pagetable = proc.pagetable;
    
    if pagetable.is_null() {
        return -1;
    }
    
    // 对范围内的每个页面，取消映射并释放物理内存
    let mut current = start;
    let mut unmapped_count = 0;
    
    while current < end {
        // 尝试取消映射页面并获取物理地址
        if let Some(pa) = unsafe { unmap_page(pagetable, current) } {
            // 释放物理页面
            unsafe { kfree(pa as *mut u8) };
            unmapped_count += 1;
        }
        
        current += PAGE_SIZE;
    }
    
    // 刷新TLB以确保更改生效
    unsafe {
        for i in 0..unmapped_count {
            flush_tlb_page(start + i * PAGE_SIZE);
        }
    }
    
    0
}

/// 获取内存统计信息
pub fn get_mem_stats() -> MemStats {
    let stats = MEM_STATS.lock();
    MemStats {
        alloc_count: AtomicUsize::new(stats.alloc_count.load(Ordering::Relaxed)),
        free_count: AtomicUsize::new(stats.free_count.load(Ordering::Relaxed)),
        mmap_count: AtomicUsize::new(stats.mmap_count.load(Ordering::Relaxed)),
        munmap_count: AtomicUsize::new(stats.munmap_count.load(Ordering::Relaxed)),
        brk_count: AtomicUsize::new(stats.brk_count.load(Ordering::Relaxed)),
        total_allocated: AtomicUsize::new(stats.total_allocated.load(Ordering::Relaxed)),
        total_freed: AtomicUsize::new(stats.total_freed.load(Ordering::Relaxed)),
    }
}

/// 系统调用分发函数
pub fn dispatch_optimized(syscall_id: u32, args: &[u64]) -> SyscallResult {
    match syscall_id {
        0x3000 => {
            // brk
            let args = extract_args(args, 1)?;
            let addr = args[0] as usize;
            Ok(sys_brk_optimized(addr) as u64)
        }
        0x3001 => {
            // mmap
            let args = extract_args(args, 6)?;
            let addr = args[0] as usize;
            let length = args[1] as usize;
            let prot = args[2] as i32;
            let flags = args[3] as i32;
            let fd = args[4] as i32;
            let offset = args[5] as u64;
            Ok(sys_mmap_optimized(addr, length, prot, flags, fd, offset) as u64)
        }
        0x3002 => {
            // munmap
            let args = extract_args(args, 2)?;
            let addr = args[0] as usize;
            let length = args[1] as usize;
            Ok(sys_munmap_optimized(addr, length) as u64)
        }
        _ => Err(SyscallError::NotSupported),
    }
}