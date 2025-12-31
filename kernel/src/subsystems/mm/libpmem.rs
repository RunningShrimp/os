//! # SNIA 编程模型 - PMDK 风格接口
//!
//! 提供类似 PMDK (Persistent Memory Development Kit) 的接口。
//!
//! ## 功能
//!
//! - libpmem: 持久化内存操作
//! - libpmemobj: 事务对象存储
//! - libpmemblk: 持久化块数组
//! - libpmemlog: 持久化日志
//!
//! ## 架构
//!
//! ```
//! PMDK 风格接口
//!     ├── libpmem (基础持久化操作)
//!     ├── libpmemobj (对象存储)
//!     ├── libpmemblk (块存储)
//!     └── libpmemlog (日志存储)
//! ```

#![allow(dead_code)]

use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use nos_api::Error;


/// PMEM 布局魔数
pub const PMEM_MAGIC: u64 = 0x504D454D_52464C00; // "PMEMRFL\0"

/// 默认映射大小（字节）
pub const DEFAULT_MAP_SIZE: usize = 1024 * 1024 * 1024; // 1 GB

/// 最大对象池数量
pub const MAX_POOLS: usize = 128;

/// PMEM 持久化内存池
#[derive(Debug)]
pub struct PmemPool {
    /// 池 ID
    pub id: u64,
    /// 池地址
    pub addr: u64,
    /// 池大小
    pub size: u64,
    /// 是否一致
    pub consistent: bool,
}

/// PMEMOBJ 对象类型 ID
pub type PMEMobjtype = u64;

/// PMEMOBJ 对象 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PMEMoid {
    /// 池 UUID
    pub pool_uuid_lo: u64,
    /// 对象偏移
    pub off: u64,
}

impl PMEMoid {
    /// 创建空对象 ID
    pub fn null() -> Self {
        Self {
            pool_uuid_lo: 0,
            off: 0,
        }
    }

    /// 检查是否为空
    pub fn is_null(&self) -> bool {
        self.off == 0
    }
}

/// PMEMOBJ 根节点
#[derive(Debug, Clone)]
#[repr(C)]
pub struct PMEMroot {
    /// 根对象偏移
    pub root_off: u64,
}

/// PMEMOBJ 池头部
#[derive(Debug, Clone)]
#[repr(C)]
pub struct PmemobjPoolHeader {
    /// 魔数
    pub magic: u64,
    /// 主要版本
    pub major: u32,
    /// 次要版本
    pub minor: u32,
    /// UUID
    pub uuid: [u8; 16],
    /// 创建时间戳
    pub create_ts: u64,
    /// 池大小
    pub pool_size: u64,
    /// 根节点
    pub root: PMEMroot,
}

/// PMEMBLK 块池头部
#[derive(Debug, Clone)]
#[repr(C)]
pub struct PmemblkPoolHeader {
    /// 魔数
    pub magic: u64,
    /// 块大小
    pub block_size: u64,
    /// 总块数
    pub total_blocks: u64,
}

/// PMEMLOG 日志池头部
#[derive(Debug, Clone)]
#[repr(C)]
pub struct PmemlogPoolHeader {
    /// 魔数
    pub magic: u64,
    /// 写入偏移
    pub write_offset: u64,
    /// 日志大小
    pub log_size: u64,
}

// ============================================================================
// libpmem: 持久化内存基础操作
// ============================================================================

/// 将内存区域刷新到持久化内存
///
/// # 参数
///
/// * `addr` - 起始地址
/// * `len` - 长度（字节）
///
/// # 安全性
///
/// 调用者必须确保地址范围有效。
pub unsafe fn pmem_flush(addr: u64, len: usize) {
    let ptr = addr as *const u8;

    // 使用 clflush 指令刷新每一行缓存
    for offset in (0..len).step_by(CACHE_LINE_SIZE) {
        unsafe {
            let line_ptr = ptr.add(offset) as *const u8;
            // GH-#846: 使用架构特定的 clflush 指令
            // See: https://github.com/npos/kernel/issues/846
            // asm!("clflush ($0)" : : "r"(line_ptr) : "memory");
            let _ = line_ptr;
        }
    }

    core::sync::atomic::fence(Ordering::Release);
}

/// 持久化内存区域（刷新 + 排空缓冲区）
///
/// # 参数
///
/// * `addr` - 起始地址
/// * `len` - 长度（字节）
pub unsafe fn pmem_persist(addr: u64, len: usize) { unsafe {
    pmem_flush(addr, len);
    pmem_drain();
}}

/// 排空持久化缓冲区
pub fn pmem_drain() {
    // GH-#847: 使用 sfence 指令
    // See: https://github.com/npos/kernel/issues/847
    core::sync::atomic::fence(Ordering::Release);
}

/// 持久化内存拷贝
///
/// # 参数
///
/// * `dest` - 目标地址
/// * `src` - 源地址
/// * `len` - 长度（字节）
///
/// # 返回
///
/// 返回目标地址
pub unsafe fn pmem_memcpy_persist(dest: u64, src: u64, len: usize) -> u64 {
    let src_ptr = src as *const u8;
    let dest_ptr = dest as *mut u8;

    for i in 0..len {
        unsafe {
            dest_ptr.add(i).write_volatile(src_ptr.add(i).read_volatile());
        }
    }

    unsafe { pmem_persist(dest, len); }

    dest
}

/// 持久化内存设置
///
/// # 参数
///
/// * `dest` - 目标地址
/// * `c` - 字符值
/// * `len` - 长度（字节）
///
/// # 返回
///
/// 返回目标地址
pub unsafe fn pmem_memset_persist(dest: u64, c: u8, len: usize) -> u64 {
    let dest_ptr = dest as *mut u8;

    for i in 0..len {
        unsafe {
            dest_ptr.add(i).write_volatile(c);
        }
    }

    unsafe { pmem_persist(dest, len); }

    dest
}

/// 映射持久化内存文件
///
/// # 参数
///
/// * `path` - 文件路径
/// * `len` - 映射长度
/// * `is_readonly` - 是否只读
///
/// # 返回
///
/// 返回映射地址
pub fn pmem_map_file(path: &str, _len: usize, _is_readonly: bool) -> Result<u64, Error> {
    // GH-#848: 实现文件映射
    // See: https://github.com/npos/kernel/issues/848
    // 这里简化处理，直接返回虚拟地址
    crate::println!("[libpmem] Mapping file: {}", path);

    let addr = 0x3000_0000_0000u64; // 模拟地址

    Ok(addr)
}

/// 取消映射持久化内存文件
///
/// # 参数
///
/// * `addr` - 映射地址
/// * `len` - 映射长度
pub fn pmem_unmap(addr: u64, _len: usize) -> Result<(), Error> {
    // GH-#849: 实现取消映射
    // See: https://github.com/npos/kernel/issues/849
    crate::println!("[libpmem] Unmapping address: 0x{:x}", addr);
    Ok(())
}

/// 检查地址是否来自持久化内存
///
/// # 参数
///
/// * `addr` - 要检查的地址
///
/// # 返回
///
/// 返回是否为持久化内存地址
pub fn pmem_is_pmem(addr: u64) -> bool {
    // GH-#850: 检查地址是否在 NVDIMM 区域
    // See: https://github.com/npos/kernel/issues/850
    addr >= 0x1000_0000_0000 && addr < 0x2000_0000_0000
}

// ============================================================================
// libpmemobj: 事务对象存储
// ============================================================================

/// PMEMOBJ 池
#[derive(Debug)]
pub struct PmemobjPool {
    /// 池头部
    pub header: PmemobjPoolHeader,
    /// 池地址
    pub addr: u64,
    /// 池大小
    pub size: u64,
    /// 下一个对象 ID
    pub next_oid: AtomicU64,
}

impl PmemobjPool {
    /// 创建新的对象池
    pub fn new(addr: u64, size: u64) -> Self {
        Self {
            header: PmemobjPoolHeader {
                magic: PMEM_MAGIC,
                major: 1,
                minor: 0,
                uuid: [0u8; 16], // GH-#851: 生成 UUID
                // See: https://github.com/npos/kernel/issues/851
                create_ts: 0,     // GH-#852: 获取时间戳
                // See: https://github.com/npos/kernel/issues/852
                pool_size: size,
                root: PMEMroot { root_off: 0 },
            },
            addr,
            size,
            next_oid: AtomicU64::new(1),
        }
    }

    /// 分配对象
    pub fn alloc(&self, _size: u64, _type_id: PMEMobjtype) -> Result<PMEMoid, Error> {
        let offset = self.next_oid.fetch_add(1, Ordering::SeqCst);

        // GH-#853: 实际分配对象空间
        // See: https://github.com/npos/kernel/issues/853
        let oid = PMEMoid {
            pool_uuid_lo: self.header.uuid[0] as u64,
            off: offset,
        };

        Ok(oid)
    }

    /// 释放对象
    pub fn free(&self, oid: PMEMoid) -> Result<(), Error> {
        if oid.is_null() {
            return Err(Error::InvalidArgument("invalid argument".to_string()));
        }

        // GH-#854: 实际释放对象空间
        // See: https://github.com/npos/kernel/issues/854
        Ok(())
    }

    /// 获取根对象
    pub fn root(&self) -> PMEMoid {
        PMEMoid {
            pool_uuid_lo: self.header.uuid[0] as u64,
            off: self.header.root.root_off,
        }
    }

    /// 设置根对象
    pub fn set_root(&mut self, oid: PMEMoid) {
        self.header.root.root_off = oid.off;

        // 持久化修改
        unsafe {
            let root_ptr = (self.addr + core::mem::offset_of!(PmemobjPoolHeader, root) as u64)
                as *const PMEMroot;
            pmem_persist(root_ptr as u64, core::mem::size_of::<PMEMroot>());
        }
    }
}

/// 打开或创建对象池
///
/// # 参数
///
/// * `path` - 池路径
/// * `layout` - 布局名称
/// * `poolsize` - 池大小
/// * `mode` - 创建模式
///
/// # 返回
///
/// 返回对象池指针
pub fn pmemobj_create(
    path: &str,
    layout: &str,
    poolsize: usize,
    _mode: u32,
) -> Result<*mut PmemobjPool, Error> {
    crate::println!("[libpmemobj] Creating pool: {} (layout: {})", path, layout);

    // GH-#855: 实际创建池文件
    // See: https://github.com/npos/kernel/issues/855
    let addr = 0x3100_0000_0000u64;

    let pool = PmemobjPool::new(addr, poolsize as u64);

    // 持久化头部
    unsafe {
        let header_ptr = addr as *mut PmemobjPoolHeader;
        *header_ptr = pool.header.clone();
        pmem_persist(
            addr,
            core::mem::size_of::<PmemobjPoolHeader>(),
        );
    }

    // 泄漏引用以获得稳定的指针
    let pool_ref = Arc::new(pool);
    let pool_ptr = Arc::into_raw(pool_ref) as *mut PmemobjPool;

    Ok(pool_ptr)
}

/// 打开现有对象池
///
/// # 参数
///
/// * `path` - 池路径
///
/// # 返回
///
/// 返回对象池指针
pub fn pmemobj_open(path: &str) -> Result<*mut PmemobjPool, Error> {
    crate::println!("[libpmemobj] Opening pool: {}", path);

    // GH-#856: 实际打开池文件
    // See: https://github.com/npos/kernel/issues/856
    Err(Error::NotImplemented("not implemented".to_string()))
}

/// 关闭对象池
///
/// # 参数
///
/// * `pop` - 对象池指针
pub fn pmemobj_close(pop: *mut PmemobjPool) -> Result<(), Error> {
    if pop.is_null() {
        return Err(Error::InvalidArgument("invalid argument".to_string()));
    }

    // 转换回 Arc 以释放
    unsafe {
        let _ = Arc::from_raw(pop);
    }

    crate::println!("[libpmemobj] Pool closed");

    Ok(())
}

/// 分配对象
///
/// # 参数
///
/// * `pop` - 对象池指针
/// * `size` - 对象大小
/// * `type_id` - 对象类型
///
/// # 返回
///
/// 返回对象 ID
pub fn pmemobj_alloc(pop: *mut PmemobjPool, size: usize, type_id: PMEMobjtype) -> Result<PMEMoid, Error> {
    if pop.is_null() {
        return Err(Error::InvalidArgument("invalid argument".to_string()));
    }

    unsafe {
        let pool = &*pop;
        pool.alloc(size as u64, type_id)
    }
}

/// 释放对象
///
/// # 参数
///
/// * `pop` - 对象池指针
/// * `oid` - 对象 ID
pub fn pmemobj_free(pop: *mut PmemobjPool, oid: PMEMoid) -> Result<(), Error> {
    if pop.is_null() {
        return Err(Error::InvalidArgument("invalid argument".to_string()));
    }

    unsafe {
        let pool = &*pop;
        pool.free(oid)
    }
}

// ============================================================================
// libpmemblk: 持久化块数组
// ============================================================================

/// PMEMBLK 池
#[derive(Debug)]
pub struct PmemblkPool {
    /// 池头部
    pub header: PmemblkPoolHeader,
    /// 池地址
    pub addr: u64,
    /// 池大小
    pub size: u64,
}

impl PmemblkPool {
    /// 创建新的块池
    pub fn new(addr: u64, size: u64, block_size: u64) -> Self {
        let total_blocks = (size - core::mem::size_of::<PmemblkPoolHeader>() as u64) / block_size;

        Self {
            header: PmemblkPoolHeader {
                magic: PMEM_MAGIC,
                block_size,
                total_blocks,
            },
            addr,
            size,
        }
    }

    /// 读取块
    pub fn read(&self, block_num: u64) -> Result<Vec<u8>, Error> {
        if block_num >= self.header.total_blocks {
            return Err(Error::InvalidArgument("invalid argument".to_string()));
        }

        let offset = core::mem::size_of::<PmemblkPoolHeader>() as u64
            + block_num * self.header.block_size;
        let addr = self.addr + offset;

        // GH-#857: 实际读取数据
        // See: https://github.com/npos/kernel/issues/857
        let mut data = vec![0u8; self.header.block_size as usize];

        unsafe {
            let ptr = addr as *const u8;
            for i in 0..data.len() {
                data[i] = ptr.add(i).read_volatile();
            }
        }

        Ok(data)
    }

    /// 写入块
    pub fn write(&self, block_num: u64, data: &[u8]) -> Result<(), Error> {
        if block_num >= self.header.total_blocks {
            return Err(Error::InvalidArgument("invalid argument".to_string()));
        }

        if data.len() != self.header.block_size as usize {
            return Err(Error::InvalidArgument("invalid argument".to_string()));
        }

        let offset = core::mem::size_of::<PmemblkPoolHeader>() as u64
            + block_num * self.header.block_size;
        let addr = self.addr + offset;

        unsafe {
            let ptr = addr as *mut u8;
            for i in 0..data.len() {
                ptr.add(i).write_volatile(data[i]);
            }

            // 持久化
            pmem_persist(addr, data.len());
        }

        Ok(())
    }
}

/// 创建块池
///
/// # 参数
///
/// * `path` - 池路径
/// * `block_size` - 块大小
/// * `poolsize` - 池大小
/// * `mode` - 创建模式
pub fn pmemblk_create(
    path: &str,
    block_size: usize,
    poolsize: usize,
    _mode: u32,
) -> Result<*mut PmemblkPool, Error> {
    crate::println!("[libpmemblk] Creating block pool: {}", path);

    let addr = 0x3200_0000_0000u64;

    let pool = PmemblkPool::new(addr, poolsize as u64, block_size as u64);

    // 持久化头部
    unsafe {
        let header_ptr = addr as *mut PmemblkPoolHeader;
        *header_ptr = pool.header.clone();
        pmem_persist(
            addr,
            core::mem::size_of::<PmemblkPoolHeader>(),
        );
    }

    let pool_ref = Arc::new(pool);
    let pool_ptr = Arc::into_raw(pool_ref) as *mut PmemblkPool;

    Ok(pool_ptr)
}

// ============================================================================
// libpmemlog: 持久化日志
// ============================================================================

/// PMEMLOG 池
#[derive(Debug)]
pub struct PmemlogPool {
    /// 池头部
    pub header: PmemlogPoolHeader,
    /// 池地址
    pub addr: u64,
    /// 池大小
    pub size: u64,
}

impl PmemlogPool {
    /// 创建新的日志池
    pub fn new(addr: u64, size: u64) -> Self {
        Self {
            header: PmemlogPoolHeader {
                magic: PMEM_MAGIC,
                write_offset: core::mem::size_of::<PmemlogPoolHeader>() as u64,
                log_size: size - core::mem::size_of::<PmemlogPoolHeader>() as u64,
            },
            addr,
            size,
        }
    }

    /// 追加日志
    pub fn append(&mut self, data: &[u8]) -> Result<(), Error> {
        let data_len = data.len() as u64;

        if self.header.write_offset + data_len > self.addr + self.size {
            // 日志已满，回绕
            self.header.write_offset = core::mem::size_of::<PmemlogPoolHeader>() as u64;
        }

        let write_addr = self.header.write_offset;

        unsafe {
            let ptr = write_addr as *mut u8;
            for i in 0..data.len() {
                ptr.add(i).write_volatile(data[i]);
            }

            // 持久化
            pmem_persist(write_addr, data.len());
        }

        self.header.write_offset += data_len;

        Ok(())
    }

    /// 读取日志
    pub fn read(&self, offset: u64, len: usize) -> Result<Vec<u8>, Error> {
        let read_addr = self.addr + offset;

        // GH-#858: 检查边界
        // See: https://github.com/npos/kernel/issues/858

        let mut data = vec![0u8; len];

        unsafe {
            let ptr = read_addr as *const u8;
            for i in 0..len {
                data[i] = ptr.add(i).read_volatile();
            }
        }

        Ok(data)
    }
}

/// 创建日志池
///
/// # 参数
///
/// * `path` - 池路径
/// * `poolsize` - 池大小
/// * `mode` - 创建模式
pub fn pmemlog_create(
    path: &str,
    poolsize: usize,
    _mode: u32,
) -> Result<*mut PmemlogPool, Error> {
    crate::println!("[libpmemlog] Creating log pool: {}", path);

    let addr = 0x3300_0000_0000u64;

    let pool = PmemlogPool::new(addr, poolsize as u64);

    // 持久化头部
    unsafe {
        let header_ptr = addr as *mut PmemlogPoolHeader;
        *header_ptr = pool.header.clone();
        pmem_persist(
            addr,
            core::mem::size_of::<PmemlogPoolHeader>(),
        );
    }

    let pool_ref = Arc::new(pool);
    let pool_ptr = Arc::into_raw(pool_ref) as *mut PmemlogPool;

    Ok(pool_ptr)
}

/// 追加日志
///
/// # 参数
///
/// * `plp` - 日志池指针
/// * `data` - 数据
pub fn pmemlog_append(plp: *mut PmemlogPool, data: &[u8]) -> Result<(), Error> {
    if plp.is_null() {
        return Err(Error::InvalidArgument("invalid argument".to_string()));
    }

    unsafe {
        let pool = &mut *plp;
        pool.append(data)
    }
}

/// 缓存行大小（常量）
const CACHE_LINE_SIZE: usize = 64;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pmem_is_pmem() {
        assert!(pmem_is_pmem(0x1000_0000_0000));
        assert!(!pmem_is_pmem(0x8000_0000));
    }

    #[test]
    fn test_oid_null() {
        let oid = PMEMoid::null();
        assert!(oid.is_null());
    }
}
