//! NVMe（非易失性内存Express）驱动程序
//!
//! 提供高性能的NVMe SSD支持，包括命名空间管理、
//! I/O队列管理和异步操作支持。
//!
//! 主要功能：
//! - NVMe设备发现和初始化
//! - 命名空间管理
//! - I/O队列管理
//! - 异步I/O操作
//! - 错误处理和恢复
//! - 电源管理
//! - 性能优化

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use spin::Mutex;

use crate::time;

/// NVMe寄存器基址类型
pub type NvmeRegisterBase = usize;

/// NVMe控制器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvmeControllerState {
    /// 未初始化
    Uninitialized,
    /// 正在初始化
    Initializing,
    /// 就绪
    Ready,
    /// 正在重置
    Resetting,
    /// 错误状态
    Error,
    /// 关闭
    Shutdown,
}

/// NVMe设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvmeDeviceType {
    /// SSD
    Ssd,
    /// HDD（通过NVMe桥接）
    Hdd,
    /// 混合驱动器
    Hybrid,
    /// 未知类型
    Unknown,
}

/// NVMe控制器配置
#[derive(Debug, Clone)]
pub struct NvmeControllerConfig {
    /// PCI配置空间
    pub pci_config: PciConfigSpace,
    /// 最大队列大小
    pub max_queue_size: u16,
    /// 最大I/O队列数
    pub max_io_queues: u16,
    /// 中断向量数
    pub interrupt_vectors: u16,
    /// 页面大小
    pub page_size: u32,
    /// 是否支持多路径
    pub multipath_enabled: bool,
    /// 超时设置（毫秒）
    pub timeout_ms: u32,
}

/// PCI配置空间
#[derive(Debug, Clone)]
pub struct PciConfigSpace {
    /// 厂商ID
    pub vendor_id: u16,
    /// 设备ID
    pub device_id: u16,
    /// 类代码
    pub class_code: u32,
    /// BAR0（寄存器基址）
    pub bar0: u64,
    /// BAR1（寄存器基址）
    pub bar1: u64,
    /// 中断线
    pub interrupt_line: u8,
    /// 中断引脚
    pub interrupt_pin: u8,
}

impl Default for NvmeControllerConfig {
    fn default() -> Self {
        Self {
            pci_config: PciConfigSpace {
                vendor_id: 0,
                device_id: 0,
                class_code: 0,
                bar0: 0,
                bar1: 0,
                interrupt_line: 0,
                interrupt_pin: 0,
            },
            max_queue_size: 65535,
            max_io_queues: 64,
            interrupt_vectors: 64,
            page_size: 4096,
            multipath_enabled: false,
            timeout_ms: 5000,
        }
    }
}

/// NVMe控制器
pub struct NvmeController {
    /// 控制器ID
    pub id: u32,
    /// 寄存器基址
    register_base: NvmeRegisterBase,
    /// 配置
    config: NvmeControllerConfig,
    /// 控制器状态
    state: Arc<Mutex<NvmeControllerState>>,
    /// 命名空间
    namespaces: Arc<Mutex<BTreeMap<u32, NvmeNamespace>>>,
    /// 管理队列
    admin_queue: Arc<Mutex<NvmeQueue>>,
    /// I/O队列
    io_queues: Arc<Mutex<Vec<NvmeQueue>>>,
    /// 待处理的命令
    pending_commands: Arc<Mutex<BTreeMap<u16, NvmeCommand>>>,
    /// 统计信息
    statistics: NvmeStatistics,
    /// 下一个命令ID
    next_command_id: AtomicU64,
}

/// NVMe命名空间
#[derive(Debug, Clone)]
pub struct NvmeNamespace {
    /// 命名空间ID
    pub id: u32,
    /// 命名空间大小（逻辑块数）
    pub size: u64,
    /// 命名空间容量（逻辑块数）
    pub capacity: u64,
    /// 逻辑块大小
    pub block_size: u32,
    /// 命名空间特征
    pub features: NvmeNamespaceFeatures,
    /// 数据保护信息
    pub data_protection: NvmeDataProtection,
    /// 命名空间状态
    pub state: NvmeNamespaceState,
}

/// NVMe命名空间特征
#[derive(Debug, Clone)]
pub struct NvmeNamespaceFeatures {
    /// 是否支持写保护
    pub write_protected: bool,
    /// 是否支持 Trim
    pub trim_supported: bool,
    /// 是否支持原子写入
    pub atomic_write_supported: bool,
    /// 是否支持写零
    pub write_zeroes_supported: bool,
    /// 是否支持扇区级保护
    pub sector_protection_enabled: bool,
    /// 是否支持多路径
    pub multipath_capable: bool,
    /// 命名空间类型
    pub namespace_type: NvmeNamespaceType,
}

/// NVMe命名空间类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvmeNamespaceType {
    /// 标准类型
    Standard,
    /// ZNS（区域命名空间）
    Zns,
    /// KV（键值命名空间）
    Kv,
    /// 未知类型
    Unknown,
}

/// NVMe数据保护
#[derive(Debug, Clone)]
pub struct NvmeDataProtection {
    /// 保护信息类型
    pub protection_type: u8,
    /// 保护信息大小
    pub protection_size: u8,
    /// 是否支持元数据
    pub metadata_supported: bool,
    /// 元数据大小
    pub metadata_size: u32,
}

/// NVMe命名空间状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvmeNamespaceState {
    /// 活跃状态
    Active,
    /// 配置中
    Configuring,
    /// 正在格式化
    Formatting,
    /// 不可用
    Unavailable,
    /// 只读
    ReadOnly,
    /// 已删除
    Deleted,
}

/// NVMe队列
#[derive(Debug, Clone)]
pub struct NvmeQueue {
    /// 队列ID
    pub id: u16,
    /// 队列类型
    pub queue_type: NvmeQueueType,
    /// 队列大小
    pub size: u16,
    /// 队列头指针
    pub head: Arc<Mutex<u16>>,
    /// 队列尾指针
    pub tail: Arc<Mutex<u16>>,
    /// 队列相位
    pub phase: Arc<Mutex<bool>>,
    /// 命令槽位
    pub command_slots: Arc<Mutex<Vec<NvmeCommandSlot>>>,
    /// 完成队列
    pub completion_queue: Arc<Mutex<Vec<NvmeCompletion>>>,
    /// 队列状态
    pub state: NvmeQueueState,
    /// 关联的中断向量
    pub interrupt_vector: Option<u16>,
}

/// NVMe队列类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvmeQueueType {
    /// 管理队列
    Admin,
    /// I/O队列
    Io,
}

/// NVMe队列状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvmeQueueState {
    /// 未初始化
    Uninitialized,
    /// 就绪
    Ready,
    /// 正在删除
    Deleting,
    /// 错误
    Error,
}

/// NVMe命令槽位
#[derive(Debug, Clone)]
pub struct NvmeCommandSlot {
    /// 命令ID
    pub command_id: u16,
    /// 是否正在使用
    pub in_use: bool,
    /// 命令
    pub command: Option<NvmeCommand>,
    /// 提交时间
    pub submit_time: u64,
}

/// NVMe命令
#[derive(Debug, Clone)]
pub struct NvmeCommand {
    /// 命令ID
    pub command_id: u16,
    /// 操作码
    pub opcode: NvmeOpcode,
    /// 命令标志
    pub flags: u8,
    /// 命名空间ID
    pub namespace_id: u32,
    /// 保留字段
    pub reserved: [u32; 2],
    /// 元数据指针
    pub metadata_ptr: u64,
    /// 数据指针
    pub data_ptr: u64,
    /// 数据长度
    pub data_length: u32,
    /// 命令特定字段
    pub cdw10: u32,
    pub cdw11: u32,
    pub cdw12: u32,
    pub cdw13: u32,
    pub cdw14: u32,
    pub cdw15: u32,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminOpcode {
    /// 删除I/O队列
    DeleteIoQueue = 0x00,
    /// 创建I/O队列
    CreateIoQueue = 0x01,
    /// 获取日志页
    GetLogPage = 0x02,
    /// 删除命名空间
    DeleteNamespace = 0x04,
    /// 创建命名空间
    CreateNamespace = 0x05,
    /// 获取命名空间
    GetNamespace = 0x06,
    /// 报告命名空间
    ReportNamespaces = 0x85,
    /// 附加命名空间
    AttachNamespace = 0x0B,
    /// 分离命名空间
    DetachNamespace = 0x86,
    /// 固件激活
    FirmwareActivate = 0x10,
    /// 固件下载
    FirmwareDownload = 0x11,
    /// 设备自检
    DeviceSelfTest = 0x14,
    /// 命名空间管理
    NamespaceManagement = 0x18,
    /// 安全发送
    SecuritySend = 0x81,
    /// 安全接收
    SecurityReceive = 0x82,
    /// 格式化命名空间
    FormatNamespace = 0x80,
    /// 管理器发送
    ManagementSend = 0x84,
    /// 管理器接收
    ManagementReceive = 0x83,
    /// 设置特性
    SetFeatures = 0x09,
    /// 获取特性
    GetFeatures = 0x0A,
    /// 异步事件请求
    AsyncEventRequest = 0x0C,
    /// 管理器NS变更
    ManagementNsChange = 0x0D,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvmOpcode {
    /// 读操作
    Read = 0x02,
    /// 写操作
    Write = 0x01,
    /// 比较操作
    Compare = 0x05,
    /// 写零
    WriteZeroes = 0x08,
    /// 数据集管理（Trim）
    DatasetManagement = 0x09,
    /// 残余注册
    ReservationRegister = 0x0D,
    /// 残余报告
    ReservationReport = 0x0E,
    /// 残余获取
    ReservationAcquire = 0x11,
    /// 残余释放
    ReservationRelease = 0x15,
}

/// 统一包装的操作码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvmeOpcode {
    Admin(AdminOpcode),
    Nvm(NvmOpcode),
    Unknown(u8),
}

impl NvmeOpcode {
    pub fn code(self) -> u8 {
        match self {
            NvmeOpcode::Admin(op) => op as u8,
            NvmeOpcode::Nvm(op) => op as u8,
            NvmeOpcode::Unknown(c) => c,
        }
    }
}

/// NVMe完成
#[derive(Debug, Clone)]
pub struct NvmeCompletion {
    /// 命令ID
    pub command_id: u16,
    /// 完成状态
    pub status: NvmeStatus,
    /// 完成队列头指针
    pub sq_head: u16,
    /// 完成队列ID
    pub sq_id: u16,
    /// 完成相位
    pub phase: bool,
    /// 错误信息
    pub error_info: Option<NvmeErrorInfo>,
}

/// NVMe状态
#[derive(Debug, Clone, Copy)]
pub struct NvmeStatus {
    /// 状态代码
    pub status_code: u16,
    /// 状态类型
    pub status_type: NvmeStatusType,
    /// 更多标志
    pub more: bool,
    /// 保留标志
    pub reserved: u8,
}

/// NVMe状态类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvmeStatusType {
    /// 通用命令状态
    Generic,
    /// 命令特定状态
    CommandSpecific,
    /// 媒体和完整性错误
    MediaError,
    /// 路径错误
    PathError,
    /// 供应商特定错误
    VendorSpecific,
}

/// NVMe错误信息
#[derive(Debug, Clone)]
pub struct NvmeErrorInfo {
    /// 错误代码
    pub error_code: u32,
    /// 错误位置
    pub error_location: u16,
    /// LBA
    pub lba: u64,
    /// 命名空间ID
    pub namespace_id: u32,
    /// 传输类型
    pub transport_type: u8,
    /// 命令类型
    pub command_type: u8,
    /// 状态码
    pub status_code: u16,
}

/// NVMe统计信息
#[derive(Debug, Default)]
pub struct NvmeStatistics {
    /// 总命令数
    pub total_commands: AtomicU64,
    /// 成功命令数
    pub successful_commands: AtomicU64,
    /// 失败命令数
    pub failed_commands: AtomicU64,
    /// 读取的字节数
    pub bytes_read: AtomicU64,
    /// 写入的字节数
    pub bytes_written: AtomicU64,
    /// 读取操作数
    pub read_operations: AtomicU64,
    /// 写入操作数
    pub write_operations: AtomicU64,
    /// 平均延迟（微秒）
    pub average_latency_us: AtomicU64,
    /// 错误重试次数
    pub error_retries: AtomicU64,
    /// 当前队列深度
    pub current_queue_depth: AtomicUsize,
    /// 最大队列深度
    pub max_queue_depth: AtomicUsize,
}

/// NVMe I/O请求
#[derive(Clone)]
pub struct NvmeIoRequest {
    /// 请求ID
    pub request_id: u64,
    /// 命名空间ID
    pub namespace_id: u32,
    /// 起始LBA
    pub start_lba: u64,
    /// 逻辑块数
    pub block_count: u32,
    /// 数据缓冲区
    pub data_buffer: Vec<u8>,
    /// 元数据缓冲区
    pub metadata_buffer: Option<Vec<u8>>,
    /// 请求类型
    pub request_type: NvmeIoRequestType,
    /// 完成回调
    pub completion_callback: Option<alloc::sync::Arc<dyn Fn(Result<(), NvmeError>) + Send + Sync>>, 
}

/// NVMe I/O请求类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvmeIoRequestType {
    /// 读操作
    Read,
    /// 写操作
    Write,
    /// 比较操作
    Compare,
    /// 写零操作
    WriteZeroes,
    /// 数据集管理（Trim）
    DatasetManagement,
}

/// NVMe错误类型
#[derive(Debug, Clone)]
pub enum NvmeError {
    /// 控制器错误
    ControllerError(String),
    /// 命令错误
    CommandError(NvmeStatus),
    /// 命名空间错误
    NamespaceError(String),
    /// 队列错误
    QueueError(String),
    /// 数据错误
    DataError(String),
    /// 超时错误
    TimeoutError,
    /// 设备未找到
    DeviceNotFound,
    /// 内存不足
    OutOfMemory,
    /// 无效参数
    InvalidParameter(String),
}

impl core::fmt::Display for NvmeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NvmeError::ControllerError(msg) => write!(f, "控制器错误: {}", msg),
            NvmeError::CommandError(status) => write!(f, "命令错误: {:?}", status),
            NvmeError::NamespaceError(msg) => write!(f, "命名空间错误: {}", msg),
            NvmeError::QueueError(msg) => write!(f, "队列错误: {}", msg),
            NvmeError::DataError(msg) => write!(f, "数据错误: {}", msg),
            NvmeError::TimeoutError => write!(f, "超时错误"),
            NvmeError::DeviceNotFound => write!(f, "设备未找到"),
            NvmeError::OutOfMemory => write!(f, "内存不足"),
            NvmeError::InvalidParameter(msg) => write!(f, "无效参数: {}", msg),
        }
    }
}

impl NvmeController {
    /// 创建新的NVMe控制器
    pub fn new(id: u32, register_base: NvmeRegisterBase, config: NvmeControllerConfig) -> Self {
        Self {
            id,
            register_base,
            config,
            state: Arc::new(Mutex::new(NvmeControllerState::Uninitialized)),
            namespaces: Arc::new(Mutex::new(BTreeMap::new())),
            admin_queue: Arc::new(Mutex::new(NvmeQueue::new(
                0,
                NvmeQueueType::Admin,
                64,
            ))),
            io_queues: Arc::new(Mutex::new(Vec::new())),
            pending_commands: Arc::new(Mutex::new(BTreeMap::new())),
            statistics: NvmeStatistics::default(),
            next_command_id: AtomicU64::new(1),
        }
    }

    /// 初始化控制器
    pub fn initialize(&self) -> Result<(), NvmeError> {
        {
            let mut state = self.state.lock();
            *state = NvmeControllerState::Initializing;
        }

        crate::println!("[nvme] 初始化控制器 {}", self.id);

        // 检查控制器状态
        self.check_controller_ready()?;

        // 重置控制器
        self.reset_controller()?;

        // 设置管理队列
        self.setup_admin_queue()?;

        // 识别控制器
        self.identify_controller()?;

        // 设置I/O队列
        self.setup_io_queues()?;

        // 扫描命名空间
        self.scan_namespaces()?;

        {
            let mut state = self.state.lock();
            *state = NvmeControllerState::Ready;
        }

        crate::println!("[nvme] 控制器 {} 初始化完成", self.id);
        Ok(())
    }

    /// 重置控制器
    pub fn reset_controller(&self) -> Result<(), NvmeError> {
        {
            let mut state = self.state.lock();
            *state = NvmeControllerState::Resetting;
        }

        crate::println!("[nvme] 重置控制器 {}", self.id);

        // 设置CC寄存器的ENABLE位为0
        self.write_register(NvmeRegister::Config, 0);

        // 等待CSTS寄存器的READY位为0
        let mut timeout = Duration::from_millis(self.config.timeout_ms as u64);
        let start_time = crate::time::timestamp_millis();

        while timeout.as_millis() > 0 {
            let status = self.read_register(NvmeRegister::Status);
            if (status & 0x1) == 0 {
                break;
            }

            let elapsed = crate::time::timestamp_millis() - start_time;
            if elapsed >= self.config.timeout_ms as u64 {
                return Err(NvmeError::TimeoutError);
            }

            timeout = Duration::from_millis(self.config.timeout_ms.saturating_sub(elapsed.min(u32::MAX as u64) as u32) as u64);
            crate::arch::wfi(); // 等待中断
        }

        // 设置CC寄存器
        let mut cc = 0u32;
        cc |= 4 << 20; // MQES - 最大队列大小
        cc |= 0 << 19; // CQR - 队列配置
        cc |= 1 << 18; // AMS - arb机制
        cc |= 0 << 17; // SHN - 关闭通知
        cc |= 1 << 16; // IOSQES - I/O提交队列大小
        cc |= 1 << 14; // IOCQES - I/O完成队列大小
        self.write_register(NvmeRegister::Config, cc);

        // 设置Admin Queue地址
        self.set_admin_queue_addresses()?;

        // 设置ENABLE位
        let cc = self.read_register(NvmeRegister::Config) | 0x1;
        self.write_register(NvmeRegister::Config, cc);

        // 等待READY位
        let start_time = crate::time::timestamp_millis();
        while timeout.as_millis() > 0 {
            let status = self.read_register(NvmeRegister::Status);
            if (status & 0x1) != 0 {
                break;
            }

            let elapsed = crate::time::timestamp_millis() - start_time;
            if elapsed >= self.config.timeout_ms as u64 {
                return Err(NvmeError::TimeoutError);
            }

            timeout = Duration::from_millis(self.config.timeout_ms.saturating_sub(elapsed.min(u32::MAX as u64) as u32) as u64);
            crate::arch::wfi();
        }

        crate::println!("[nvme] 控制器 {} 重置完成", self.id);
        Ok(())
    }

    /// 关闭控制器
    pub fn shutdown(&self) -> Result<(), NvmeError> {
        {
            let mut state = self.state.lock();
            *state = NvmeControllerState::Shutdown;
        }

        crate::println!("[nvme] 关闭控制器 {}", self.id);

        // 删除I/O队列
        self.delete_io_queues()?;

        // 设置SHN位
        let cc = self.read_register(NvmeRegister::Config) | (1 << 17);
        self.write_register(NvmeRegister::Config, cc);

        // 等待SHST位
        let start_time = crate::time::timestamp_millis();
        let timeout = Duration::from_millis(self.config.timeout_ms as u64);

        while timeout.as_millis() > 0 {
            let status = self.read_register(NvmeRegister::Status);
            let shst = (status >> 14) & 0x3;
            if shst == 2 {
                break; // Shutdown complete
            }

            let elapsed = crate::time::timestamp_millis() - start_time;
            if elapsed >= self.config.timeout_ms as u64 {
                return Err(NvmeError::TimeoutError);
            }

            crate::arch::wfi();
        }

        // 清除ENABLE位
        let cc = self.read_register(NvmeRegister::Config) & !0x1;
        self.write_register(NvmeRegister::Config, cc);

        crate::println!("[nvme] 控制器 {} 关闭完成", self.id);
        Ok(())
    }

    /// 提交I/O请求
    pub fn submit_io_request(&self, request: NvmeIoRequest) -> Result<u64, NvmeError> {
        let request_id = self.next_command_id.fetch_add(1, Ordering::SeqCst);

        // 获取I/O队列
        let mut io_queues = self.io_queues.lock();
        if io_queues.is_empty() {
            return Err(NvmeError::QueueError("没有可用的I/O队列".to_string()));
        }

        let queue = &mut io_queues[0]; // 使用第一个I/O队列

        // 创建NVMe命令
        let nvme_command = self.io_request_to_command(&request, request_id as u16)?;

        // 提交命令
        let command_id = self.submit_command_to_queue(queue, nvme_command)?;

        // 记录待处理的请求
        // Build command from request for tracking
        let nvme_command_for_tracking = NvmeCommand {
            command_id,
            opcode: match request.request_type {
                NvmeIoRequestType::Read => NvmeOpcode::Nvm(NvmOpcode::Read),
                NvmeIoRequestType::Write => NvmeOpcode::Nvm(NvmOpcode::Write),
                _ => NvmeOpcode::Unknown(0),
            },
            flags: 0,
            namespace_id: request.namespace_id,
            reserved: [0; 2],
            metadata_ptr: 0,
            data_ptr: 0,
            data_length: 0,
            cdw10: 0,
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        };
        let mut pending = self.pending_commands.lock();
        pending.insert(command_id, nvme_command_for_tracking);

        // 更新统计
        self.statistics.current_queue_depth.fetch_add(1, Ordering::SeqCst);
        self.statistics.total_commands.fetch_add(1, Ordering::SeqCst);

        Ok(request_id)
    }

    /// 获取命名空间
    pub fn get_namespace(&self, namespace_id: u32) -> Result<NvmeNamespace, NvmeError> {
        let namespaces = self.namespaces.lock();
        namespaces.get(&namespace_id)
            .cloned()
            .ok_or(NvmeError::NamespaceError(format!("命名空间 {} 不存在", namespace_id)))
    }

    /// 获取所有命名空间
    pub fn get_all_namespaces(&self) -> Result<Vec<NvmeNamespace>, NvmeError> {
        let namespaces = self.namespaces.lock();
        Ok(namespaces.values().cloned().collect())
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> NvmeStatistics {
        NvmeStatistics {
            total_commands: AtomicU64::new(self.statistics.total_commands.load(Ordering::SeqCst)),
            successful_commands: AtomicU64::new(self.statistics.successful_commands.load(Ordering::SeqCst)),
            failed_commands: AtomicU64::new(self.statistics.failed_commands.load(Ordering::SeqCst)),
            bytes_read: AtomicU64::new(self.statistics.bytes_read.load(Ordering::SeqCst)),
            bytes_written: AtomicU64::new(self.statistics.bytes_written.load(Ordering::SeqCst)),
            read_operations: AtomicU64::new(self.statistics.read_operations.load(Ordering::SeqCst)),
            write_operations: AtomicU64::new(self.statistics.write_operations.load(Ordering::SeqCst)),
            average_latency_us: AtomicU64::new(self.statistics.average_latency_us.load(Ordering::SeqCst)),
            error_retries: AtomicU64::new(self.statistics.error_retries.load(Ordering::SeqCst)),
            current_queue_depth: AtomicUsize::new(self.statistics.current_queue_depth.load(Ordering::SeqCst)),
            max_queue_depth: AtomicUsize::new(self.statistics.max_queue_depth.load(Ordering::SeqCst)),
        }
    }

    /// 私有辅助方法
    fn check_controller_ready(&self) -> Result<(), NvmeError> {
        let status = self.read_register(NvmeRegister::Status);
        if (status & 0x1) == 0 {
            return Err(NvmeError::ControllerError("控制器未就绪".to_string()));
        }
        Ok(())
    }

    fn setup_admin_queue(&self) -> Result<(), NvmeError> {
        let mut admin_queue = self.admin_queue.lock();
        admin_queue.initialize()?;
        Ok(())
    }

    fn setup_io_queues(&self) -> Result<(), NvmeError> {
        // 创建默认的I/O队列
        let io_queue = NvmeQueue::new(1, NvmeQueueType::Io, 1024);
        self.create_io_queue(&io_queue)?;

        let mut io_queues = self.io_queues.lock();
        io_queues.push(io_queue);

        Ok(())
    }

    fn identify_controller(&self) -> Result<(), NvmeError> {
        let command = NvmeCommand {
            command_id: 0,
            opcode: NvmeOpcode::Admin(AdminOpcode::GetLogPage),
            flags: 0,
            namespace_id: 0,
            reserved: [0; 2],
            metadata_ptr: 0,
            data_ptr: 0, // 需要分配缓冲区
            data_length: 4096,
            cdw10: 0x2, // Identify Controller
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        };

        self.submit_admin_command(command)?;
        self.wait_for_completion(0)?;

        Ok(())
    }

    fn scan_namespaces(&self) -> Result<(), NvmeError> {
        // 获取命名空间列表
        let command = NvmeCommand {
            command_id: 0,
            opcode: NvmeOpcode::Admin(AdminOpcode::ReportNamespaces),
            flags: 0,
            namespace_id: 0xFFFFFFFF, // 所有命名空间
            reserved: [0; 2],
            metadata_ptr: 0,
            data_ptr: 0, // 需要分配缓冲区
            data_length: 4096,
            cdw10: 0,
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        };

        self.submit_admin_command(command)?;
        let completion = self.wait_for_completion(0)?;

        // 解析命名空间列表并创建命名空间对象
        // 这里简化实现
        let namespace = NvmeNamespace {
            id: 1,
            size: 1024 * 1024, // 1GB
            capacity: 1024 * 1024,
            block_size: 512,
            features: NvmeNamespaceFeatures {
                write_protected: false,
                trim_supported: true,
                atomic_write_supported: false,
                write_zeroes_supported: true,
                sector_protection_enabled: false,
                multipath_capable: false,
                namespace_type: NvmeNamespaceType::Standard,
            },
            data_protection: NvmeDataProtection {
                protection_type: 0,
                protection_size: 0,
                metadata_supported: false,
                metadata_size: 0,
            },
            state: NvmeNamespaceState::Active,
        };

        let mut namespaces = self.namespaces.lock();
        namespaces.insert(namespace.id, namespace);

        crate::println!("[nvme] 发现 1 个命名空间");

        Ok(())
    }

    fn submit_admin_command(&self, command: NvmeCommand) -> Result<(), NvmeError> {
        let mut admin_queue = self.admin_queue.lock();
        admin_queue.submit_command(command)?;
        Ok(())
    }

    fn wait_for_completion(&self, command_id: u16) -> Result<NvmeCompletion, NvmeError> {
        let admin_queue = self.admin_queue.lock();
        admin_queue.wait_for_completion(command_id)
    }

    fn create_io_queue(&self, queue: &NvmeQueue) -> Result<(), NvmeError> {
        let command = NvmeCommand {
            command_id: 0,
            opcode: NvmeOpcode::Admin(AdminOpcode::CreateIoQueue),
            flags: 0,
            namespace_id: 0,
            reserved: [0; 2],
            metadata_ptr: 0,
            data_ptr: queue.as_address() as u64,
            data_length: 0,
            cdw10: queue.id as u32 | (queue.size as u32) << 16,
            cdw11: 1, // QID
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        };

        self.submit_admin_command(command)?;
        self.wait_for_completion(0)?;

        Ok(())
    }

    fn delete_io_queues(&self) -> Result<(), NvmeError> {
        let io_queues = self.io_queues.lock();

        for queue in io_queues.iter() {
            let command = NvmeCommand {
                command_id: 0,
                opcode: NvmeOpcode::Admin(AdminOpcode::DeleteIoQueue),
                flags: 0,
                namespace_id: 0,
                reserved: [0; 2],
                metadata_ptr: 0,
                data_ptr: 0,
                data_length: 0,
                cdw10: queue.id as u32,
                cdw11: 0,
                cdw12: 0,
                cdw13: 0,
                cdw14: 0,
                cdw15: 0,
            };

            self.submit_admin_command(command)?;
            self.wait_for_completion(0)?;
        }

        Ok(())
    }

    fn io_request_to_command(&self, request: &NvmeIoRequest, command_id: u16) -> Result<NvmeCommand, NvmeError> {
        let opcode = match request.request_type {
            NvmeIoRequestType::Read => NvmeOpcode::Nvm(NvmOpcode::Read),
            NvmeIoRequestType::Write => NvmeOpcode::Nvm(NvmOpcode::Write),
            NvmeIoRequestType::Compare => NvmeOpcode::Nvm(NvmOpcode::Compare),
            NvmeIoRequestType::WriteZeroes => NvmeOpcode::Nvm(NvmOpcode::WriteZeroes),
            NvmeIoRequestType::DatasetManagement => NvmeOpcode::Nvm(NvmOpcode::DatasetManagement),
        };

        Ok(NvmeCommand {
            command_id,
            opcode,
            flags: 0,
            namespace_id: request.namespace_id,
            reserved: [0; 2],
            metadata_ptr: request.metadata_buffer.as_ref()
                .map_or(0, |buf| buf.as_ptr() as u64),
            data_ptr: request.data_buffer.as_ptr() as u64,
            data_length: request.data_buffer.len() as u32,
            cdw10: request.start_lba as u32,
            cdw11: (request.start_lba >> 32) as u32,
            cdw12: request.block_count as u32,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        })
    }

    fn submit_command_to_queue(&self, queue: &mut NvmeQueue, command: NvmeCommand) -> Result<u16, NvmeError> {
        queue.submit_command(command)
    }

    fn set_admin_queue_addresses(&self) -> Result<(), NvmeError> {
        let admin_queue = self.admin_queue.lock();
        let admin_queue_addr = admin_queue.as_address();

        // 设置提交队列地址
        self.write_register(NvmeRegister::AdminSubmissionQueueAddr, (admin_queue_addr >> 2) as u32);

        // 设置完成队列地址（假设与提交队列相邻）
        let completion_queue_addr = admin_queue_addr + 4096; // 简化假设
        self.write_register(NvmeRegister::AdminCompletionQueueAddr, (completion_queue_addr >> 2) as u32);

        Ok(())
    }

    fn write_register(&self, register: NvmeRegister, value: u32) {
        // 简化实现，实际需要写入硬件寄存器
        let address = self.register_base + register as usize;
        unsafe {
            *(address as *mut u32) = value;
        }
    }

    fn read_register(&self, register: NvmeRegister) -> u32 {
        // 简化实现，实际需要从硬件寄存器读取
        let address = self.register_base + register as usize;
        unsafe {
            *(address as *const u32)
        }
    }
}

/// NVMe寄存器偏移
#[derive(Debug, Clone, Copy)]
pub enum NvmeRegister {
    /// 配置寄存器
    Config = 0x14,
    /// 状态寄存器
    Status = 0x1C,
    /// 管理提交队列地址
    AdminSubmissionQueueAddr = 0x28,
    /// 管理完成队列地址
    AdminCompletionQueueAddr = 0x30,
}

impl NvmeQueue {
    /// 创建新的NVMe队列
    pub fn new(id: u16, queue_type: NvmeQueueType, size: u16) -> Self {
        Self {
            id,
            queue_type,
            size,
            head: Arc::new(Mutex::new(0)),
            tail: Arc::new(Mutex::new(0)),
            phase: Arc::new(Mutex::new(false)),
            command_slots: Arc::new(Mutex::new(Vec::new())),
            completion_queue: Arc::new(Mutex::new(Vec::new())),
            state: NvmeQueueState::Uninitialized,
            interrupt_vector: None,
        }
    }

    /// 初始化队列
    pub fn initialize(&mut self) -> Result<(), NvmeError> {
        let mut slots = self.command_slots.lock();
        slots.clear();
        slots.reserve(self.size as usize);

        for i in 0..self.size {
            slots.push(NvmeCommandSlot {
                command_id: i,
                in_use: false,
                command: None,
                submit_time: 0,
            });
        }

        let mut completions = self.completion_queue.lock();
        completions.clear();
        completions.resize(self.size as usize, NvmeCompletion {
            command_id: 0,
            status: NvmeStatus {
                status_code: 0,
                status_type: NvmeStatusType::Generic,
                more: false,
                reserved: 0,
            },
            sq_head: 0,
            sq_id: 0,
            phase: false,
            error_info: None,
        });

        self.state = NvmeQueueState::Ready;
        Ok(())
    }

    /// 提交命令
    pub fn submit_command(&mut self, command: NvmeCommand) -> Result<u16, NvmeError> {
        if self.state != NvmeQueueState::Ready {
            return Err(NvmeError::QueueError("队列未就绪".to_string()));
        }

        let mut slots = self.command_slots.lock();
        let command_id = command.command_id as usize;

        if command_id >= slots.len() || slots[command_id].in_use {
            return Err(NvmeError::QueueError("命令槽位不可用".to_string()));
        }

        slots[command_id].in_use = true;
        slots[command_id].command = Some(command.clone());
        slots[command_id].submit_time = time::timestamp_nanos();

        // 更新尾指针
        let mut tail = self.tail.lock();
        *tail = (*tail + 1) % self.size;

        Ok(command.command_id)
    }

    /// 等待命令完成
    pub fn wait_for_completion(&self, command_id: u16) -> Result<NvmeCompletion, NvmeError> {
        let start_time = time::timestamp_millis();
        let timeout = Duration::from_millis(5000); // 5秒超时

        while timeout.as_millis() > 0 {
            let completions = self.completion_queue.lock();

            // 查找匹配的完成
            for completion in completions.iter() {
                if completion.command_id == command_id {
                    return Ok(completion.clone());
                }
            }

            // 检查超时
            let elapsed = time::timestamp_millis() - start_time;
            if elapsed >= timeout.as_millis() as u64 {
                return Err(NvmeError::TimeoutError);
            }

            crate::arch::wfi();
        }

        Err(NvmeError::TimeoutError)
    }

    /// 获取队列地址（用于物理地址映射）
    pub fn as_address(&self) -> usize {
        // 简化实现，实际应该返回物理地址
        self as *const NvmeQueue as usize
    }
}
