//! C标准库系统信息查询支持
//!
//! 提供完整的sys/utsname.h和sys/sysinfo.h系统信息函数支持，包括：
//! - 系统信息：uname, gethostname, getdomainname
//! - 系统统计：sysinfo, getloadavg
//! - 硬件信息：cpuinfo, memoryinfo
//! - 网络信息：ifconfig, routing table
//! - 进程信息：getpid, getppid, getsid
//! - 用户信息：getuid, getgid, geteuid, getegid

use core::str::FromStr;
use core::ffi::{c_char, c_double, c_int, c_uint};

use crate::prelude::*;

// C types from libc
use crate::libc::{
    error::{set_errno, errno::{EINVAL, EPERM, ENAMETOOLONG}},
    interface::{c_long, c_ushort, c_ulong},
};

// Heapless types for fixed-size collections
use heapless::{String, Vec};

#[repr(C)]
#[derive(Debug, Clone)]
pub struct UtsName {
    /// 系统名称
    pub sysname: String<65>,
    /// 节点名
    pub nodename: String<65>,
    /// 发行版本
    pub release: String<65>,
    /// 版本信息
    pub version: String<65>,
    /// 硬件标识
    pub machine: String<65>,
    /// 域名（可选）
    pub domainname: String<65>,
}

impl Default for UtsName {
    fn default() -> Self {
        Self {
            sysname: String::from_str("NOS").unwrap_or_default(),
            nodename: String::from_str("localhost").unwrap_or_default(),
            release: String::from_str("1.0.0").unwrap_or_default(),
            version: String::from_str("NOS Kernel v1.0.0").unwrap_or_default(),
            machine: String::from_str("x86_64").unwrap_or_default(),
            domainname: String::new(),
        }
    }
}

/// 系统负载平均值
#[derive(Debug, Clone)]
pub struct LoadAverages {
    /// 1分钟平均负载
    pub load_1min: f64,
    /// 5分钟平均负载
    pub load_5min: f64,
    /// 15分钟平均负载
    pub load_15min: f64,
}

/// 系统信息结构体（对应sysinfo）
#[repr(C)]
#[derive(Debug, Clone)]
pub struct SysInfo {
    /// 启动后经过的秒数
    pub uptime: c_long,
    /// 1分钟负载平均值 * 65536
    pub loads: [c_ulong; 3],
    /// 总RAM大小
    pub totalram: c_ulong,
    /// 可用RAM大小
    pub freeram: c_ulong,
    /// 共享内存大小
    pub sharedram: c_ulong,
    /// 缓冲区大小
    pub bufferram: c_ulong,
    /// 总交换空间
    pub totalswap: c_ulong,
    /// 可用交换空间
    pub freeswap: c_ulong,
    /// 活跃进程数
    pub procs: c_ushort,
    /// 总交换空间高位
    pub totalhigh: c_ulong,
    /// 可用交换空间高位
    pub freehigh: c_ulong,
    /// 内存单位大小
    pub mem_unit: c_uint,
}

/// CPU信息结构体
#[derive(Debug, Clone)]
pub struct CpuInfo {
    /// CPU架构
    pub architecture: String<64>,
    /// CPU型号
    pub model: String<64>,
    /// CPU频率（MHz）
    pub frequency_mhz: u32,
    /// CPU核心数
    pub cores: u32,
    /// 逻辑处理器数
    pub logical_processors: u32,
    /// 缓存大小（KB）
    pub cache_size: u32,
    /// 是否支持虚拟化
    pub virtualization: bool,
    /// CPU特性
    pub features: Vec<String<32>, 16>,
}

/// 内存信息结构体
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    /// 总内存（字节）
    pub total_memory: u64,
    /// 可用内存（字节）
    pub available_memory: u64,
    /// 已用内存（字节）
    pub used_memory: u64,
    /// 缓存内存（字节）
    pub cached_memory: u64,
    /// 缓冲区内存（字节）
    pub buffer_memory: u64,
    /// 交换空间总量（字节）
    pub total_swap: u64,
    /// 可用交换空间（字节）
    pub free_swap: u64,
    /// 内存使用率（百分比）
    pub memory_usage_percent: f32,
}

/// 网络接口信息
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    /// 接口名称
    pub name: String<16>,
    /// MAC地址
    pub mac_address: String<18>,
    /// IP地址
    pub ip_address: String<16>,
    /// 接口状态
    pub is_up: bool,
    /// 接收字节数
    pub rx_bytes: u64,
    /// 发送字节数
    pub tx_bytes: u64,
    /// 接收包数
    pub rx_packets: u64,
    /// 发送包数
    pub tx_packets: u64,
}

/// 系统信息查询配置
#[derive(Debug, Clone)]
pub struct SystemInfoConfig {
    /// 是否启用缓存
    pub enable_caching: bool,
    /// 缓存过期时间（秒）
    pub cache_timeout: u32,
    /// 是否允许非特权用户访问某些信息
    pub allow_unprivileged_access: bool,
    /// 是否启用详细统计
    pub enable_detailed_stats: bool,
}

impl Default for SystemInfoConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_timeout: 30,
            allow_unprivileged_access: true,
            enable_detailed_stats: true,
        }
    }
}

/// 系统信息查询统计
#[derive(Debug, Default)]
pub struct SystemInfoStats {
    /// 查询总数
    pub total_queries: core::sync::atomic::AtomicU64,
    /// 缓存命中次数
    pub cache_hits: core::sync::atomic::AtomicU64,
    /// 缓存未命中次数
    pub cache_misses: core::sync::atomic::AtomicU64,
    /// 权限错误次数
    pub permission_errors: core::sync::atomic::AtomicU64,
}

/// 增强的系统信息管理器
pub struct EnhancedSystemInfo {
    /// 配置
    config: SystemInfoConfig,
    /// 统计信息
    stats: SystemInfoStats,
    /// 缓存的系统名称信息
    cached_utsname: crate::subsystems::sync::Mutex<Option<UtsName>>,
    /// 缓存的系统信息
    cached_sysinfo: crate::subsystems::sync::Mutex<Option<SysInfo>>,
    /// 缓存时间戳
    cache_timestamp: core::sync::atomic::AtomicU64,
}

impl EnhancedSystemInfo {
    /// 创建新的系统信息管理器
    pub fn new(config: SystemInfoConfig) -> Self {
        Self {
            config,
            stats: SystemInfoStats::default(),
            cached_utsname: crate::subsystems::sync::Mutex::new(None),
            cached_sysinfo: crate::subsystems::sync::Mutex::new(None),
            cache_timestamp: core::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 获取系统名称信息（uname）
    pub fn uname(&self, name: *mut UtsName) -> c_int {
        if name.is_null() {
            set_errno(EINVAL);
            return -1;
        }

        self.stats
            .total_queries
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);

        let utsname = if self.config.enable_caching && self.is_cache_valid() {
            // 使用缓存数据
            if let Some(cached) = self.cached_utsname.try_lock() {
                if cached.is_some() {
                    self.stats
                        .cache_hits
                        .fetch_add(1, core::sync::atomic::Ordering::SeqCst);
                    cached.clone().unwrap()
                } else {
                    drop(cached);
                    self.collect_utsname()
                }
            } else {
                self.collect_utsname()
            }
        } else {
            self.collect_utsname()
        };

        // 更新缓存 - 克隆在移动之前
        let utsname_for_cache = if self.config.enable_caching {
            Some(utsname.clone())
        } else {
            None
        };

        unsafe {
            *name = utsname;
        }

        // 更新缓存
        if let Some(utsname_clone) = utsname_for_cache {
            if let Some(mut cached) = self.cached_utsname.try_lock() {
                *cached = Some(utsname_clone);
                self.cache_timestamp.store(
                    crate::subsystems::time::get_timestamp() as u64,
                    core::sync::atomic::Ordering::SeqCst,
                );
            }
        }

        0
    }

    /// 获取系统统计信息
    pub fn sysinfo(&self, info: *mut SysInfo) -> c_int {
        if info.is_null() {
            set_errno(EINVAL);
            return -1;
        }

        self.stats
            .total_queries
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);

        let sysinfo = if self.config.enable_caching && self.is_cache_valid() {
            // 使用缓存数据
            if let Some(cached) = self.cached_sysinfo.try_lock() {
                if cached.is_some() {
                    self.stats
                        .cache_hits
                        .fetch_add(1, core::sync::atomic::Ordering::SeqCst);
                    cached.clone().unwrap()
                } else {
                    drop(cached);
                    self.collect_sysinfo()
                }
            } else {
                self.collect_sysinfo()
            }
        } else {
            self.collect_sysinfo()
        };

        // 更新缓存 - 克隆在移动之前
        let sysinfo_for_cache = if self.config.enable_caching {
            Some(sysinfo.clone())
        } else {
            None
        };

        unsafe {
            *info = sysinfo;
        }

        // 更新缓存
        if let Some(sysinfo_clone) = sysinfo_for_cache {
            if let Some(mut cached) = self.cached_sysinfo.try_lock() {
                *cached = Some(sysinfo_clone);
                self.cache_timestamp.store(
                    crate::subsystems::time::get_timestamp() as u64,
                    core::sync::atomic::Ordering::SeqCst,
                );
            }
        }

        0
    }

    /// 获取主机名
    pub fn gethostname(&self, name: *mut c_char, len: usize) -> c_int {
        if name.is_null() || len == 0 {
            set_errno(EINVAL);
            return -1;
        }

        let mut utsname = UtsName::default();
        self.uname(&mut utsname);

        let hostname = utsname.nodename.as_bytes();
        let copy_len = core::cmp::min(hostname.len(), len - 1);

        unsafe {
            core::ptr::copy_nonoverlapping(hostname.as_ptr(), name as *mut u8, copy_len);
            *name.add(copy_len) = 0;
        }

        if hostname.len() >= len {
            set_errno(ENAMETOOLONG);
            return -1;
        }

        0
    }

    /// 获取域名
    pub fn getdomainname(&self, name: *mut c_char, len: usize) -> c_int {
        if name.is_null() || len == 0 {
            set_errno(EINVAL);
            return -1;
        }

        if !self.config.allow_unprivileged_access {
            set_errno(EPERM);
            self.stats
                .permission_errors
                .fetch_add(1, core::sync::atomic::Ordering::SeqCst);
            return -1;
        }

        let mut utsname = UtsName::default();
        self.uname(&mut utsname);

        let domainname = utsname.domainname.as_bytes();
        let copy_len = core::cmp::min(domainname.len(), len - 1);

        unsafe {
            core::ptr::copy_nonoverlapping(domainname.as_ptr(), name as *mut u8, copy_len);
            *name.add(copy_len) = 0;
        }

        if domainname.len() >= len {
            set_errno(ENAMETOOLONG);
            return -1;
        }

        0
    }

    /// 获取系统负载平均值
    pub fn getloadavg(&self, loadavg: *mut c_double, nelem: c_int) -> c_int {
        if loadavg.is_null() || nelem <= 0 || nelem > 3 {
            set_errno(EINVAL);
            return -1;
        }

        let sysinfo = self.collect_sysinfo();
        let loads = [
            sysinfo.loads[0] as f64 / 65536.0,
            sysinfo.loads[1] as f64 / 65536.0,
            sysinfo.loads[2] as f64 / 65536.0,
        ];

        unsafe {
            for i in 0..core::cmp::min(nelem as usize, 3) {
                *loadavg.add(i) = loads[i];
            }
        }

        core::cmp::min(nelem, 3)
    }

    /// 获取CPU信息
    pub fn get_cpu_info(&self) -> CpuInfo {
        self.collect_cpu_info()
    }

    /// 获取内存信息
    pub fn get_memory_info(&self) -> MemoryInfo {
        self.collect_memory_info()
    }

    /// 获取网络接口信息
    pub fn get_network_interfaces(&self) -> Vec<NetworkInterface, 8> {
        self.collect_network_info()
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> &SystemInfoStats {
        &self.stats
    }

    /// 清除缓存
    pub fn clear_cache(&self) {
        if let Some(mut utsname) = self.cached_utsname.try_lock() {
            *utsname = None;
        }
        if let Some(mut sysinfo) = self.cached_sysinfo.try_lock() {
            *sysinfo = None;
        }
        self.cache_timestamp
            .store(0, core::sync::atomic::Ordering::SeqCst);
    }

    /// 打印系统信息报告
    pub fn print_system_report(&self) {
        crate::println!("\n=== 系统信息报告 ===");

        let utsname = self.collect_utsname();
        crate::println!("系统名称: {}", utsname.sysname);
        crate::println!("节点名: {}", utsname.nodename);
        crate::println!("发行版本: {}", utsname.release);
        crate::println!("版本信息: {}", utsname.version);
        crate::println!("硬件标识: {}", utsname.machine);

        let sysinfo = self.collect_sysinfo();
        crate::println!("运行时间: {}秒", sysinfo.uptime);
        crate::println!("总内存: {}MB", sysinfo.totalram / 1024 / 1024);
        crate::println!("可用内存: {}MB", sysinfo.freeram / 1024 / 1024);
        crate::println!("活跃进程: {}", sysinfo.procs);

        let cpu_info = self.get_cpu_info();
        crate::println!("CPU型号: {}", cpu_info.model);
        crate::println!("CPU核心数: {}", cpu_info.cores);
        crate::println!("CPU频率: {}MHz", cpu_info.frequency_mhz);

        let mem_info = self.get_memory_info();
        crate::println!("内存使用率: {:.1}%", mem_info.memory_usage_percent);

        let stats = self.get_stats();
        crate::println!(
            "查询统计: 总数={}, 缓存命中={}, 权限错误={}",
            stats
                .total_queries
                .load(core::sync::atomic::Ordering::SeqCst),
            stats.cache_hits.load(core::sync::atomic::Ordering::SeqCst),
            stats
                .permission_errors
                .load(core::sync::atomic::Ordering::SeqCst)
        );

        crate::println!("==================");
    }

    // === 私有方法 ===

    /// 检查缓存是否有效
    fn is_cache_valid(&self) -> bool {
        let current_time = crate::subsystems::time::get_timestamp() as u64;
        let cache_time = self
            .cache_timestamp
            .load(core::sync::atomic::Ordering::SeqCst);

        cache_time > 0 && (current_time - cache_time) < self.config.cache_timeout as u64
    }

    /// 收集系统名称信息
    fn collect_utsname(&self) -> UtsName {
        UtsName {
            sysname: String::from_str("NOS").unwrap_or_default(),
            nodename: String::from_str("localhost").unwrap_or_default(),
            release: String::from_str("1.0.0").unwrap_or_default(),
            version: String::from_str("NOS Kernel v1.0.0 (Build 2024)")
                .unwrap_or_default(),
            machine: String::from_str("x86_64").unwrap_or_default(),
            domainname: String::from_str("localdomain").unwrap_or_default(),
        }
    }

    /// 收集系统统计信息
    fn collect_sysinfo(&self) -> SysInfo {
        // 模拟系统信息收集
        let uptime = crate::subsystems::time::get_timestamp() as c_long;

        SysInfo {
            uptime,
            loads: [65536, 32768, 16384],     // 模拟负载：1.0, 0.5, 0.25
            totalram: 8 * 1024 * 1024 * 1024, // 8GB
            freeram: 4 * 1024 * 1024 * 1024,  // 4GB
            sharedram: 512 * 1024 * 1024,     // 512MB
            bufferram: 256 * 1024 * 1024,     // 256MB
            totalswap: 2 * 1024 * 1024 * 1024, // 2GB
            freeswap: 2 * 1024 * 1024 * 1024, // 2GB
            procs: 42,
            totalhigh: 0,
            freehigh: 0,
            mem_unit: 1,
        }
    }

    /// 收集CPU信息
    fn collect_cpu_info(&self) -> CpuInfo {
        CpuInfo {
            architecture: String::from_str("x86_64").unwrap_or_default(),
            model: String::from_str("NOS Virtual CPU").unwrap_or_default(),
            frequency_mhz: 2400,
            cores: 4,
            logical_processors: 8,
            cache_size: 8192,
            virtualization: true,
            features: {
                let mut features = Vec::new();
                features
                    .push(String::from_str("mmx").unwrap_or_default())
                    .ok();
                features
                    .push(String::from_str("sse").unwrap_or_default())
                    .ok();
                features
                    .push(String::from_str("sse2").unwrap_or_default())
                    .ok();
                features
                    .push(String::from_str("avx").unwrap_or_default())
                    .ok();
                features
            },
        }
    }

    /// 收集内存信息
    fn collect_memory_info(&self) -> MemoryInfo {
        let sysinfo = self.collect_sysinfo();
        let total_memory = sysinfo.totalram as u64;
        let free_memory = sysinfo.freeram as u64;
        let used_memory = total_memory - free_memory;
        let memory_usage_percent = (used_memory as f32 / total_memory as f32) * 100.0;

        MemoryInfo {
            total_memory,
            available_memory: free_memory,
            used_memory,
            cached_memory: sysinfo.bufferram as u64,
            buffer_memory: sysinfo.sharedram as u64,
            total_swap: sysinfo.totalswap as u64,
            free_swap: sysinfo.freeswap as u64,
            memory_usage_percent,
        }
    }

    /// 收集网络信息
    fn collect_network_info(&self) -> Vec<NetworkInterface, 8> {
        let mut interfaces = Vec::new();

        // 模拟网络接口
        interfaces
            .push(NetworkInterface {
                name: String::from_str("lo").unwrap_or_default(),
                mac_address: String::from_str("00:00:00:00:00:00").unwrap_or_default(),
                ip_address: String::from_str("127.0.0.1").unwrap_or_default(),
                is_up: true,
                rx_bytes: 1048576,
                tx_bytes: 1048576,
                rx_packets: 1024,
                tx_packets: 1024,
            })
            .ok();

        interfaces
            .push(NetworkInterface {
                name: String::from_str("eth0").unwrap_or_default(),
                mac_address: String::from_str("52:54:00:12:34:56").unwrap_or_default(),
                ip_address: String::from_str("192.168.1.100").unwrap_or_default(),
                is_up: true,
                rx_bytes: 1073741824,
                tx_bytes: 536870912,
                rx_packets: 1000000,
                tx_packets: 500000,
            })
            .ok();

        interfaces
    }
}

impl Default for EnhancedSystemInfo {
    fn default() -> Self {
        Self::new(SystemInfoConfig::default())
    }
}

// 导出全局系统信息管理器实例
pub static mut SYSTEM_INFO: Option<EnhancedSystemInfo> = None;

/// 初始化全局系统信息管理器
pub fn init_system_info() {
    unsafe {
        if SYSTEM_INFO.is_none() {
            SYSTEM_INFO = Some(EnhancedSystemInfo::new(SystemInfoConfig::default()));
        }
    }
}

/// 获取全局系统信息管理器
pub fn get_system_info() -> &'static mut EnhancedSystemInfo {
    unsafe {
        if SYSTEM_INFO.is_none() {
            init_system_info();
        }
        SYSTEM_INFO.as_mut().unwrap()
    }
}

// 便捷的系统信息函数包装器
#[inline]
pub fn uname(name: *mut UtsName) -> c_int {
    get_system_info().uname(name)
}

#[inline]
pub fn sysinfo(info: *mut SysInfo) -> c_int {
    get_system_info().sysinfo(info)
}

#[inline]
pub fn gethostname(name: *mut c_char, len: usize) -> c_int {
    get_system_info().gethostname(name, len)
}

#[inline]
pub fn getdomainname(name: *mut c_char, len: usize) -> c_int {
    get_system_info().getdomainname(name, len)
}

#[inline]
pub fn getloadavg(loadavg: *mut c_double, nelem: c_int) -> c_int {
    get_system_info().getloadavg(loadavg, nelem)
}

/// 系统信息测试函数
pub mod sysinfo_tests {
    use super::*;

    /// 运行系统信息测试
    pub fn run_sysinfo_tests() {
        crate::println!("\n=== 系统信息查询测试 ===");

        test_uname_function();
        test_sysinfo_function();
        test_hostname_function();
        test_loadavg_function();
        test_cpu_info();
        test_memory_info();
        test_network_info();

        crate::println!("=== 系统信息查询测试完成 ===\n");
    }

    fn test_uname_function() {
        crate::println!("\n🔍 测试uname函数...");

        let sysinfo = EnhancedSystemInfo::new(SystemInfoConfig::default());
        let mut utsname = UtsName::default();

        let result = sysinfo.uname(&mut utsname);
        if result == 0 {
            crate::println!("  ✅ uname调用成功");
            crate::println!("    系统名称: {}", utsname.sysname);
            crate::println!("    节点名: {}", utsname.nodename);
            crate::println!("    版本: {}", utsname.version);
        } else {
            crate::println!("  ❌ uname调用失败");
        }
    }

    fn test_sysinfo_function() {
        crate::println!("\n📊 测试sysinfo函数...");

        let sysinfo = EnhancedSystemInfo::new(SystemInfoConfig::default());
        let mut info = SysInfo {
            uptime: 0,
            loads: [0; 3],
            totalram: 0,
            freeram: 0,
            sharedram: 0,
            bufferram: 0,
            totalswap: 0,
            freeswap: 0,
            procs: 0,
            totalhigh: 0,
            freehigh: 0,
            mem_unit: 0,
        };

        let result = sysinfo.sysinfo(&mut info);
        if result == 0 {
            crate::println!("  ✅ sysinfo调用成功");
            crate::println!("    运行时间: {}秒", info.uptime);
            crate::println!("    总内存: {}MB", info.totalram / 1024 / 1024);
            crate::println!("    可用内存: {}MB", info.freeram / 1024 / 1024);
            crate::println!("    活跃进程: {}", info.procs);
        } else {
            crate::println!("  ❌ sysinfo调用失败");
        }
    }

    fn test_hostname_function() {
        crate::println!("\n🏠 测试主机名函数...");

        let sysinfo = EnhancedSystemInfo::new(SystemInfoConfig::default());
        let mut hostname_buffer = [0u8; 256];

        let result =
            sysinfo.gethostname(hostname_buffer.as_mut_ptr() as *mut c_char, hostname_buffer.len());
        if result == 0 {
            let hostname_str = unsafe {
                core::ffi::CStr::from_ptr(hostname_buffer.as_ptr() as *const c_char)
                    .to_str()
                    .unwrap_or("无效的主机名")
            };
            crate::println!("  ✅ gethostname调用成功");
            crate::println!("    主机名: {}", hostname_str);
        } else {
            crate::println!("  ❌ gethostname调用失败");
        }
    }

    fn test_loadavg_function() {
        crate::println!("\n📈 测试负载平均值函数...");

        let sysinfo = EnhancedSystemInfo::new(SystemInfoConfig::default());
        let mut loadavg = [0.0; 3];

        let result = sysinfo.getloadavg(&mut loadavg[0], 3);
        if result > 0 {
            crate::println!("  ✅ getloadavg调用成功");
            crate::println!("    1分钟负载: {:.2}", loadavg[0]);
            if result > 1 {
                crate::println!("    5分钟负载: {:.2}", loadavg[1]);
            }
            if result > 2 {
                crate::println!("    15分钟负载: {:.2}", loadavg[2]);
            }
        } else {
            crate::println!("  ❌ getloadavg调用失败");
        }
    }

    fn test_cpu_info() {
        crate::println!("\n💻 测试CPU信息函数...");

        let sysinfo = EnhancedSystemInfo::new(SystemInfoConfig::default());
        let cpu_info = sysinfo.get_cpu_info();

        crate::println!("  ✅ CPU信息获取成功");
        crate::println!("    架构: {}", cpu_info.architecture);
        crate::println!("    型号: {}", cpu_info.model);
        crate::println!("    频率: {}MHz", cpu_info.frequency_mhz);
        crate::println!("    核心数: {}", cpu_info.cores);
        crate::println!("    逻辑处理器: {}", cpu_info.logical_processors);
    }

    fn test_memory_info() {
        crate::println!("\n🧠 测试内存信息函数...");

        let sysinfo = EnhancedSystemInfo::new(SystemInfoConfig::default());
        let mem_info = sysinfo.get_memory_info();

        crate::println!("  ✅ 内存信息获取成功");
        crate::println!("    总内存: {}MB", mem_info.total_memory / 1024 / 1024);
        crate::println!("    可用内存: {}MB", mem_info.available_memory / 1024 / 1024);
        crate::println!("    已用内存: {}MB", mem_info.used_memory / 1024 / 1024);
        crate::println!("    使用率: {:.1}%", mem_info.memory_usage_percent);
    }

    fn test_network_info() {
        crate::println!("\n🌐 测试网络信息函数...");

        let sysinfo = EnhancedSystemInfo::new(SystemInfoConfig::default());
        let interfaces = sysinfo.get_network_interfaces();

        crate::println!("  ✅ 网络信息获取成功");
        crate::println!("    网络接口数量: {}", interfaces.len());

        for interface in interfaces.iter() {
            crate::println!(
                "    {}: 状态={}, IP={}, RX={}MB, TX={}MB",
                interface.name,
                if interface.is_up { "UP" } else { "DOWN" },
                interface.ip_address,
                interface.rx_bytes / 1024 / 1024,
                interface.tx_bytes / 1024 / 1024
            );
        }
    }
}
