//! # NVDIMM 设备驱动
//!
//! 提供 Intel Optane 和 NVDIMM 设备支持。
//!
//! ## 功能
//!
//! - ACPI NFIT 表解析
//! - 持久化内存区域发现
//! - Namespace 管理和配置
//! - Label 和 metadata 处理
//! - PMEM 块设备接口
//!
//! ## 架构
//!
//! ```
//! NVDIMM 驱动
//!     ├── ACPI NFIT 解析
//!     ├── 区域发现和管理
//!     ├── Namespace 操作
//!     ├── Label 存储
//!     └── 块设备接口
//! ```

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use nos_api::Error;

use crate::subsystems::sync::Mutex as AdvancedMutex;

/// NFIT 表签名
pub const NFIT_SIGNATURE: u64 = 0x5449464E20415449; // "TIFN ATI" (little-endian)

/// NVDIMM 类型
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvdimmType {
    /// DDR4 DIMM
    Ddr4 = 0,
    /// DDR3 DIMM
    Ddr3 = 1,
    /// DDR2 DIMM
    Ddr2 = 2,
    /// DDR1 DIMM
    Ddr1 = 3,
    /// LPDDR4 DIMM
    Lpddr4 = 4,
    /// LPDDR3 DIMM
    Lpddr3 = 5,
    /// LPDDR2 DIMM
    Lpddr2 = 6,
    /// DDR4E DIMM
    Ddr4e = 7,
    /// Unknown
    Unknown = 0xFF,
}

impl From<u8> for NvdimmType {
    fn from(value: u8) -> Self {
        match value {
            0 => NvdimmType::Ddr4,
            1 => NvdimmType::Ddr3,
            2 => NvdimmType::Ddr2,
            3 => NvdimmType::Ddr1,
            4 => NvdimmType::Lpddr4,
            5 => NvdimmType::Lpddr3,
            6 => NvdimmType::Lpddr2,
            7 => NvdimmType::Ddr4e,
            _ => NvdimmType::Unknown,
        }
    }
}

/// 内存区域类型
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
    /// PMEM 区域（直接访问持久化内存）
    Pmem = 0,
    /// BLK 区域（块模式访问）
    Blk = 1,
    /// 未知
    Unknown = 0xFF,
}

impl From<u8> for RegionType {
    fn from(value: u8) -> Self {
        match value {
            0 => RegionType::Pmem,
            1 => RegionType::Blk,
            _ => RegionType::Unknown,
        }
    }
}

/// 地址范围类型
#[derive(Debug, Clone, Copy)]
pub enum AddressRangeType {
    /// 生产者范围
    Producer,
    /// 消费者范围
    Consumer,
}

/// 地址范围结构
#[derive(Debug, Clone)]
pub struct AddressRange {
    /// 范围类型
    pub range_type: AddressRangeType,
    /// 物理地址基址
    pub base: u64,
    /// 长度（字节）
    pub length: u64,
    /// 内存映射属性
    pub attributes: u64,
}

/// 插槽范围结构
#[derive(Debug, Clone)]
pub struct InterleaveStructure {
    /// 插槽 ID
    pub id: u16,
    /// 起始插槽线
    pub start_line: u16,
    /// 插槽线数量
    pub line_count: u16,
    /// 插槽线大小
    pub line_size: u32,
    /// 相关区域索引
    pub region_index: u16,
}

/// SMBIOS 条目
#[derive(Debug, Clone)]
pub struct SmbiosEntry {
    /// SMBIOS 类型
    pub smbios_type: u8,
    /// 数据偏移
    pub data_offset: u32,
    /// 数据长度
    pub data_length: u32,
}

/// 控制区域结构
#[derive(Debug, Clone)]
pub struct ControlRegion {
    /// 区域 ID
    pub id: u16,
    /// CID 插槽基址
    pub cid_base: u64,
    /// CID 插槽大小
    pub cid_size: u32,
    /// 控制区域大小
    pub region_size: u32,
    /// 能力标志
    pub capabilities: u64,
    /// 相关区域索引
    pub region_index: u16,
}

/// PMEM 区域
#[derive(Debug, Clone)]
pub struct PmemRegion {
    /// 区域索引
    pub index: u16,
    /// 区域类型
    pub region_type: RegionType,
    /// 物理地址基址
    pub base: u64,
    /// 长度（字节）
    pub length: u64,
    /// 相关控制区域索引
    pub control_region_index: u16,
    /// 相关插槽结构索引
    pub interleave_index: u16,
}

/// BLK 区域
#[derive(Debug, Clone)]
pub struct BlkRegion {
    /// 区域索引
    pub index: u16,
    /// 物理地址基址
    pub base: u64,
    /// 长度（字节）
    pub length: u64,
    /// 相关控制区域索引
    pub control_region_index: u16,
    /// 相关插槽结构索引
    pub interleave_index: u16,
    /// 坏块管理元数据基址
    pub bbm_base: u64,
    /// 坏块管理元数据大小
    pub bbm_size: u64,
}

/// NFIT 内存设备槽
#[derive(Debug, Clone)]
pub struct MemoryDeviceSlot {
    /// 插槽索引
    pub index: u16,
    /// 插槽 ID
    pub slot_id: u32,
    /// 相关 SMBIOS 条目索引
    pub smbios_index: u32,
    /// 相关区域索引
    pub region_indices: Vec<u16>,
    /// 插槽数量
    pub slot_count: u16,
}

/// NFIT 表头部
#[derive(Debug, Clone)]
#[repr(C)]
pub struct NfitHeader {
    /// 签名
    pub signature: u64,
    /// 长度
    pub length: u32,
    /// 修订
    pub revision: u8,
    /// 校验和
    pub checksum: u8,
    /// OEM ID
    pub oem_id: [u8; 6],
    /// OEM 表 ID
    pub oem_table_id: [u8; 8],
    /// OEM 修订
    pub oem_revision: u32,
    /// 创建者 ID
    pub creator_id: u32,
    /// 创建者修订
    pub creator_revision: u32,
}

/// NFIT 子结构类型
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NfitSubtype {
    /// 系统物理地址范围
    SystemAddress = 0,
    /// 内存设备到系统地址映射
    MemoryMap = 1,
    /// 插槽结构
    Interleave = 2,
    /// SMBIOS 条目
    Smbios = 3,
    /// 控制区域
    ControlRegion = 4,
    /// BLK 数据窗口区域
    BlkDataWindow = 5,
    /// 传输能力
    TransportCapabilities = 6,
    /// 保留
    Reserved = 7,
}

impl From<u16> for NfitSubtype {
    fn from(value: u16) -> Self {
        match value {
            0 => NfitSubtype::SystemAddress,
            1 => NfitSubtype::MemoryMap,
            2 => NfitSubtype::Interleave,
            3 => NfitSubtype::Smbios,
            4 => NfitSubtype::ControlRegion,
            5 => NfitSubtype::BlkDataWindow,
            6 => NfitSubtype::TransportCapabilities,
            _ => NfitSubtype::Reserved,
        }
    }
}

/// NFIT 子结构头部
#[derive(Debug, Clone)]
#[repr(C)]
pub struct NfitSubHeader {
    /// 类型
    pub subtype: u16,
    /// 长度
    pub length: u16,
}

/// NFIT 表
#[derive(Debug, Clone)]
pub struct NfitTable {
    /// 表头部
    pub header: NfitHeader,
    /// 系统地址范围
    pub address_ranges: Vec<AddressRange>,
    /// 插槽结构
    pub interleave_structures: Vec<InterleaveStructure>,
    /// SMBIOS 条目
    pub smbios_entries: Vec<SmbiosEntry>,
    /// 控制区域
    pub control_regions: Vec<ControlRegion>,
    /// PMEM 区域
    pub pmem_regions: Vec<PmemRegion>,
    /// BLK 区域
    pub blk_regions: Vec<BlkRegion>,
    /// 内存设备槽
    pub device_slots: Vec<MemoryDeviceSlot>,
}

/// Namespace 标签
#[derive(Debug, Clone)]
#[repr(C)]
pub struct NamespaceLabel {
    /// UUID
    pub uuid: [u8; 16],
    /// 名称
    pub name: [u8; 64],
    /// 标签版本
    pub version: u32,
    /// 标志
    pub flags: u32,
    /// 大小（字节）
    pub size: u64,
    /// 偏移（字节）
    pub offset: u64,
    /// 对齐（字节）
    pub alignment: u64,
    /// 槽索引
    pub slot: u16,
    /// 区域 ID
    pub region_id: u16,
    /// 类型
    pub type_guid: [u8; 16],
    /// 检查和
    pub checksum: u64,
}

/// Namespace 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceType {
    /// PMEM 命名空间
    Pmem,
    /// BLK 命名空间
    Blk,
    /// 未知
    Unknown,
}

/// Namespace 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceState {
    /// 活动
    Active,
    /// 已禁用
    Disabled,
    /// 错误
    Error,
}

/// Namespace
#[derive(Debug, Clone)]
pub struct Namespace {
    /// UUID
    pub uuid: [u8; 16],
    /// 名称
    pub name: String,
    /// 类型
    pub ns_type: NamespaceType,
    /// 状态
    pub state: NamespaceState,
    /// 物理地址基址
    pub base: u64,
    /// 大小（字节）
    pub size: u64,
    /// 对齐（字节）
    pub alignment: u64,
    /// 所属区域 ID
    pub region_id: u16,
    /// 块大小（字节）
    pub block_size: u64,
}

/// NVDIMM 设备
#[derive(Debug, Clone)]
pub struct NvdimmDevice {
    /// 设备 ID
    pub id: u32,
    /// 设备类型
    pub device_type: NvdimmType,
    /// 串行号
    pub serial_number: u32,
    /// 制造商 ID
    pub manufacturer_id: u32,
    /// 制造日期
    pub manufacture_date: u16,
    /// 容量（字节）
    pub capacity: u64,
    /// 相关区域索引
    pub region_indices: Vec<u16>,
    /// 是否在线
    pub online: bool,
}

/// NVDIMM 区域
#[derive(Debug)]
pub struct NvdimmRegion {
    /// 区域 ID
    pub id: u16,
    /// 区域类型
    pub region_type: RegionType,
    /// 物理地址基址
    pub base: u64,
    /// 长度（字节）
    pub length: u64,
    /// 控制区域索引
    pub control_index: u16,
    /// 插槽结构索引
    pub interleave_index: u16,
    /// 所属 NVDIMM 设备
    pub devices: Vec<Arc<NvdimmDevice>>,
    /// Namespace 映射
    pub namespaces: Mutex<BTreeMap<u16, Arc<Namespace>>>,
    /// 是否在线
    pub online: bool,
}

/// NVDIMM 驱动状态
#[derive(Debug)]
pub struct NvdimmDriver {
    /// NFIT 表
    pub nfit: Option<NfitTable>,
    /// NVDIMM 设备列表
    pub devices: Vec<Arc<NvdimmDevice>>,
    /// NVDIMM 区域列表
    pub regions: Vec<Arc<NvdimmRegion>>,
    /// 当前设备 ID
    pub next_device_id: AtomicU64,
    /// 是否已初始化
    pub initialized: bool,
}

/// NVDIMM 驱动全局实例
static NVDIMM_DRIVER: AdvancedMutex<Option<NvdimmDriver>> = AdvancedMutex::new(None);

impl NvdimmDriver {
    /// 创建新的 NVDIMM 驱动实例
    pub fn new() -> Self {
        Self {
            nfit: None,
            devices: Vec::new(),
            regions: Vec::new(),
            next_device_id: AtomicU64::new(1),
            initialized: false,
        }
    }

    /// 初始化 NVDIMM 驱动
    pub fn init(&mut self) -> Result<(), Error> {
        crate::println!("[nvdimm] Initializing NVDIMM driver...");

        // 解析 ACPI NFIT 表
        self.parse_nfit_table()?;

        // 发现 NVDIMM 设备
        self.discover_devices()?;

        // 创建和管理区域
        self.setup_regions()?;

        // 读取命名空间标签
        self.load_namespaces()?;

        self.initialized = true;
        crate::println!("[nvdimm] NVDIMM driver initialized successfully");
        crate::println!("[nvdimm]   - {} devices", self.devices.len());
        crate::println!("[nvdimm]   - {} regions", self.regions.len());

        Ok(())
    }

    /// 解析 ACPI NFIT 表
    fn parse_nfit_table(&mut self) -> Result<(), Error> {
        crate::println!("[nvdimm] Parsing ACPI NFIT table...");

        // TODO: 实际环境中从 ACPI 表中读取
        // 这里创建模拟的 NFIT 表
        let mut nfit = NfitTable {
            header: NfitHeader {
                signature: NFIT_SIGNATURE,
                length: 0,
                revision: 1,
                checksum: 0,
                oem_id: [0; 6],
                oem_table_id: [0; 8],
                oem_revision: 0,
                creator_id: 0,
                creator_revision: 0,
            },
            address_ranges: Vec::new(),
            interleave_structures: Vec::new(),
            smbios_entries: Vec::new(),
            control_regions: Vec::new(),
            pmem_regions: Vec::new(),
            blk_regions: Vec::new(),
            device_slots: Vec::new(),
        };

        // 添加模拟的系统地址范围
        nfit.address_ranges.push(AddressRange {
            range_type: AddressRangeType::Producer,
            base: 0x1000_0000_0000,
            length: 0x10_0000_0000, // 64 GB
            attributes: 0,
        });

        // 添加模拟的 PMEM 区域
        nfit.pmem_regions.push(PmemRegion {
            index: 0,
            region_type: RegionType::Pmem,
            base: 0x1000_0000_0000,
            length: 0x10_0000_0000,
            control_region_index: 0,
            interleave_index: 0,
        });

        self.nfit = Some(nfit);

        crate::println!("[nvdimm]   - Found {} address ranges", self.nfit.as_ref().unwrap().address_ranges.len());
        crate::println!("[nvdimm]   - Found {} PMEM regions", self.nfit.as_ref().unwrap().pmem_regions.len());

        Ok(())
    }

    /// 发现 NVDIMM 设备
    fn discover_devices(&mut self) -> Result<(), Error> {
        crate::println!("[nvdimm] Discovering NVDIMM devices...");

        let nfit = self.nfit.as_ref().ok_or_else(|| Error::InvalidArgument("NFIT not initialized".into()))?;

        // 根据区域信息创建设备
        for region in &nfit.pmem_regions {
            let device = Arc::new(NvdimmDevice {
                id: self.next_device_id.fetch_add(1, Ordering::SeqCst) as u32,
                device_type: NvdimmType::Ddr4,
                serial_number: 0x12345678,
                manufacturer_id: 0x8086,
                manufacture_date: 0x2401,
                capacity: region.length,
                region_indices: vec![region.index],
                online: true,
            });
            self.devices.push(device);
        }

        Ok(())
    }

    /// 设置 NVDIMM 区域
    fn setup_regions(&mut self) -> Result<(), Error> {
        crate::println!("[nvdimm] Setting up NVDIMM regions...");

        let nfit = self.nfit.as_ref().ok_or_else(|| Error::InvalidArgument("NFIT not initialized".into()))?;

        // 为每个 PMEM 区域创建区域对象
        for region in &nfit.pmem_regions {
            let devices: Vec<Arc<NvdimmDevice>> = self.devices
                .iter()
                .filter(|d| d.region_indices.contains(&region.index))
                .cloned()
                .collect();

            let nvdimm_region = Arc::new(NvdimmRegion {
                id: region.index,
                region_type: region.region_type,
                base: region.base,
                length: region.length,
                control_index: region.control_region_index,
                interleave_index: region.interleave_index,
                devices,
                namespaces: Mutex::new(BTreeMap::new()),
                online: true,
            });

            self.regions.push(nvdimm_region);
        }

        Ok(())
    }

    /// 加载命名空间
    fn load_namespaces(&mut self) -> Result<(), Error> {
        crate::println!("[nvdimm] Loading namespaces...");

        // TODO: 从标签存储区域读取命名空间信息
        // 这里创建一个默认的命名空间

        Ok(())
    }

    /// 创建命名空间
    pub fn create_namespace(
        &self,
        region_id: u16,
        name: &str,
        size: u64,
        alignment: u64,
    ) -> Result<Arc<Namespace>, Error> {
        let region = self.regions
            .iter()
            .find(|r| r.id == region_id)
            .ok_or_else(|| Error::NotFound("device not found".into()))?;

        if !region.online {
            return Err(Error::InvalidState("device offline".into()));
        }

        // 检查空间是否足够
        let mut used_space = 0u64;
        let namespaces = region.namespaces.lock();
        for ns in namespaces.values() {
            used_space += ns.size;
        }
        drop(namespaces);

        if used_space + size > region.length {
            return Err(Error::InvalidArgument("invalid argument".to_string())); // 使用有效的错误类型
        }

        // 创建新命名空间
        let ns_id = region.namespaces.lock().len() as u16;
        let offset = used_space;

        let namespace = Arc::new(Namespace {
            uuid: [0u8; 16], // TODO: 生成真实 UUID
            name: name.to_string(),
            ns_type: NamespaceType::Pmem,
            state: NamespaceState::Active,
            base: region.base + offset,
            size,
            alignment,
            region_id,
            block_size: 4096,
        });

        region.namespaces.lock().insert(ns_id, namespace.clone());

        crate::println!("[nvdimm] Created namespace '{}' on region {}: size=0x{:x}", name, region_id, size);

        Ok(namespace)
    }

    /// 删除命名空间
    pub fn delete_namespace(&self, region_id: u16, ns_id: u16) -> Result<(), Error> {
        let region = self.regions
            .iter()
            .find(|r| r.id == region_id)
            .ok_or_else(|| Error::NotFound("device not found".into()))?;

        region.namespaces.lock()
            .remove(&ns_id)
            .ok_or_else(|| Error::NotFound("device not found".into()))?;

        crate::println!("[nvdimm] Deleted namespace {} on region {}", ns_id, region_id);

        Ok(())
    }

    /// 获取命名空间
    pub fn get_namespace(&self, region_id: u16, ns_id: u16) -> Result<Arc<Namespace>, Error> {
        let region = self.regions
            .iter()
            .find(|r| r.id == region_id)
            .ok_or_else(|| Error::NotFound("device not found".into()))?;

        region.namespaces.lock()
            .get(&ns_id)
            .cloned()
            .ok_or_else(|| Error::NotFound("namespace not found".into()))
    }

    /// 获取设备列表
    pub fn list_devices(&self) -> Vec<Arc<NvdimmDevice>> {
        self.devices.clone()
    }

    /// 获取区域列表
    pub fn list_regions(&self) -> Vec<Arc<NvdimmRegion>> {
        self.regions.clone()
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> NvdimmStats {
        let total_capacity = self.devices.iter().map(|d| d.capacity).sum();
        let total_regions = self.regions.len();
        let total_namespaces = self.regions.iter()
            .map(|r| r.namespaces.lock().len())
            .sum();

        NvdimmStats {
            device_count: self.devices.len(),
            region_count: total_regions,
            namespace_count: total_namespaces,
            total_capacity,
            used_capacity: 0, // TODO: 计算已使用容量
        }
    }
}

/// NVDIMM 统计信息
#[derive(Debug, Clone)]
pub struct NvdimmStats {
    pub device_count: usize,
    pub region_count: usize,
    pub namespace_count: usize,
    pub total_capacity: u64,
    pub used_capacity: u64,
}

/// 初始化 NVDIMM 驱动
pub fn init() -> Result<(), Error> {
    let mut driver_guard = NVDIMM_DRIVER.lock();
    let mut driver = NvdimmDriver::new();
    driver.init()?;
    *driver_guard = Some(driver);
    Ok(())
}

/// 关闭 NVDIMM 驱动
pub fn shutdown() -> Result<(), Error> {
    crate::println!("[nvdimm] Shutting down NVDIMM driver");
    *NVDIMM_DRIVER.lock() = None;
    Ok(())
}

/// 获取 NVDIMM 驱动实例
pub fn driver() -> Result<&'static AdvancedMutex<Option<NvdimmDriver>>, Error> {
    if NVDIMM_DRIVER.lock().is_some() {
        Ok(&NVDIMM_DRIVER)
    } else {
        Err(Error::InvalidState("driver not initialized".into()))
    }
}

/// 获取 NVDIMM 统计信息
pub fn get_stats() -> Result<NvdimmStats, Error> {
    let driver_guard = NVDIMM_DRIVER.lock();
    let driver = driver_guard.as_ref().ok_or_else(|| Error::InvalidState("NVDIMM driver not initialized".into()))?;
    Ok(driver.get_stats())
}

/// 创建命名空间（便捷函数）
pub fn create_namespace(
    region_id: u16,
    name: &str,
    size: u64,
    alignment: u64,
) -> Result<Arc<Namespace>, Error> {
    let driver_guard = NVDIMM_DRIVER.lock();
    let driver = driver_guard.as_ref().ok_or_else(|| Error::InvalidState("NVDIMM driver not initialized".into()))?;
    driver.create_namespace(region_id, name, size, alignment)
}
