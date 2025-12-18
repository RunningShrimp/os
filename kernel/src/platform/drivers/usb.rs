// USB（通用串行总线）驱动程序
//
// 提供全面的USB支持，包括主机控制器、设备枚举、
// 传输管理和USB类驱动程序支持。
//
// 主要功能：
// - USB主机控制器支持（EHCI/XHCI）
// - 设备枚举和配置
// - 控制传输、中断传输、批量传输、等时传输
// - USB类驱动程序支持
// - 电源管理
// - 热插拔支持

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;

use crate::time;

/// USB主机控制器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbHostControllerType {
    /// EHCI (Enhanced Host Controller Interface)
    Ehci,
    /// XHCI (eXtensible Host Controller Interface)
    Xhci,
    /// OHCI (Open Host Controller Interface)
    Ohci,
    /// UHCI (Universal Host Controller Interface)
    Uhci,
    /// 未知类型
    Unknown,
}

/// USB速度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbSpeed {
    /// 低速 (1.5 Mbps)
    Low,
    /// 全速 (12 Mbps)
    Full,
    /// 高速 (480 Mbps)
    High,
    /// 超高速 (5 Gbps)
    SuperSpeed,
    /// 超高速+ (10 Gbps)
    SuperSpeedPlus,
    /// 未知速度
    Unknown,
}

/// USB传输类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbTransferType {
    /// 控制传输
    Control,
    /// 等时传输
    Isochronous,
    /// 批量传输
    Bulk,
    /// 中断传输
    Interrupt,
}

/// USB方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbDirection {
    /// 输出（主机到设备）
    Out,
    /// 输入（设备到主机）
    In,
}

/// USB设备状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbDeviceState {
    /// 未连接
    NotConnected,
    /// 已连接
    Connected,
    /// 正在枚举
    Enumerating,
    /// 已配置
    Configured,
    /// 已挂起
    Suspended,
    /// 错误状态
    Error,
}

/// USB主机控制器
pub struct UsbHostController {
    /// 控制器ID
    pub id: u32,
    /// 控制器类型
    pub controller_type: UsbHostControllerType,
    /// 寄存器基址
    pub register_base: usize,
    /// 中断线
    pub interrupt_line: u8,
    /// 支持的USB版本
    pub usb_version: UsbVersion,
    /// 控制器能力
    pub capabilities: UsbControllerCapabilities,
    /// 连接的设备
    pub devices: Arc<Mutex<BTreeMap<u8, UsbDevice>>>,
    /// 根端口
    pub root_ports: Arc<Mutex<Vec<UsbPort>>>,
    /// 传输管理器
    pub transfer_manager: Arc<Mutex<UsbTransferManager>>,
    /// 统计信息
    pub statistics: UsbStatistics,
    /// 是否启用
    pub enabled: core::sync::atomic::AtomicBool,
}

/// USB版本
#[derive(Debug, Clone, Copy)]
pub struct UsbVersion {
    /// 主版本号
    pub major: u8,
    /// 次版本号
    pub minor: u8,
    /// 修订版本号
    pub revision: u8,
}

impl UsbVersion {
    /// USB 1.1
    pub const USB_1_1: Self = Self { major: 1, minor: 1, revision: 0 };
    /// USB 2.0
    pub const USB_2_0: Self = Self { major: 2, minor: 0, revision: 0 };
    /// USB 3.0
    pub const USB_3_0: Self = Self { major: 3, minor: 0, revision: 0 };
    /// USB 3.1
    pub const USB_3_1: Self = Self { major: 3, minor: 1, revision: 0 };
    /// USB 3.2
    pub const USB_3_2: Self = Self { major: 3, minor: 2, revision: 0 };
}

/// USB主机控制器能力
#[derive(Debug, Clone)]
pub struct UsbControllerCapabilities {
    /// 支持的端口数
    pub port_count: u8,
    /// 支持的最大设备数
    pub max_devices: u16,
    /// 支持的传输类型
    pub supported_transfers: Vec<UsbTransferType>,
    /// 支持的USB速度
    pub supported_speeds: Vec<UsbSpeed>,
    /// 支持的电源管理
    pub power_management: bool,
    /// 支持的设备唤醒
    pub remote_wakeup: bool,
    /// 支持的USB OTG
    pub otg_support: bool,
    /// 支持的64位地址
    pub address_64_bit: bool,
    /// 支持的设备认证
    pub device_authentication: bool,
}

/// USB端口
#[derive(Debug, Clone)]
pub struct UsbPort {
    /// 端口号
    pub port_number: u8,
    /// 端口状态
    pub status: UsbPortStatus,
    /// 连接的设备
    pub connected_device: Option<u8>, // 设备地址
    /// 端口能力
    pub capabilities: UsbPortCapabilities,
    /// 上次状态变化时间
    pub last_status_change: u64,
}

/// USB端口状态
#[derive(Debug, Clone, Copy)]
pub struct UsbPortStatus {
    /// 连接状态
    pub connected: bool,
    /// 连接状态变化
    pub connect_change: bool,
    /// 启用状态
    pub enabled: bool,
    /// 启用状态变化
    pub enable_change: bool,
    /// 过流状态
    pub over_current: bool,
    /// 过流状态变化
    pub over_current_change: bool,
    /// 重置状态
    pub reset: bool,
    /// 电源状态
    pub power: bool,
    /// 低速设备连接
    pub low_speed: bool,
    /// 高速设备连接
    pub high_speed: bool,
    /// 超高速设备连接
    pub super_speed: bool,
}

/// USB端口能力
#[derive(Debug, Clone)]
pub struct UsbPortCapabilities {
    /// 支持的速度
    pub supported_speeds: Vec<UsbSpeed>,
    /// 最大电流（毫安）
    pub max_current: u16,
    /// 是否支持远程唤醒
    pub remote_wakeup: bool,
    /// 是否支持电源切换
    pub power_switching: bool,
    /// 是否支持过流保护
    pub over_current_protection: bool,
}

/// USB设备
#[derive(Debug, Clone)]
pub struct UsbDevice {
    /// 设备地址
    pub address: u8,
    /// 设备状态
    pub state: UsbDeviceState,
    /// 设备描述符
    pub device_descriptor: UsbDeviceDescriptor,
    /// 配置描述符
    pub configurations: Vec<UsbConfigurationDescriptor>,
    /// 当前配置索引
    pub current_configuration: Option<u8>,
    /// 接口描述符
    pub interfaces: Vec<UsbInterfaceDescriptor>,
    /// 端点描述符
    pub endpoints: Vec<UsbEndpointDescriptor>,
    /// 字符串描述符
    pub string_descriptors: BTreeMap<u8, String>,
    /// 设备类
    pub device_class: UsbDeviceClass,
    /// 设备速度
    pub speed: UsbSpeed,
    /// 端口号
    pub port_number: u8,
    /// 父设备地址（集线器）
    pub parent_address: Option<u8>,
    /// 连接时间
    pub connection_time: u64,
}

/// USB设备描述符
#[derive(Debug, Clone)]
pub struct UsbDeviceDescriptor {
    /// 描述符长度
    pub length: u8,
    /// 描述符类型
    pub descriptor_type: u8,
    /// USB规范版本（BCD）
    pub usb_version: u16,
    /// 设备类
    pub device_class: u8,
    /// 设备子类
    pub device_subclass: u8,
    /// 设备协议
    pub device_protocol: u8,
    /// 最大包大小（EP0）
    pub max_packet_size: u8,
    /// 厂商ID
    pub vendor_id: u16,
    /// 产品ID
    pub product_id: u16,
    /// 设备版本（BCD）
    pub device_version: u16,
    /// 厂商字符串索引
    pub manufacturer_index: u8,
    /// 产品字符串索引
    pub product_index: u8,
    /// 序列号字符串索引
    pub serial_number_index: u8,
    /// 配置数量
    pub num_configurations: u8,
}

/// USB配置描述符
#[derive(Debug, Clone)]
pub struct UsbConfigurationDescriptor {
    /// 配置值
    pub configuration_value: u8,
    /// 配置字符串索引
    pub configuration_index: u8,
    /// 配置属性
    pub attributes: u8,
    /// 最大功耗（2mA单位）
    pub max_power: u8,
    /// 接口数量
    pub num_interfaces: u8,
}

/// USB接口描述符
#[derive(Debug, Clone)]
pub struct UsbInterfaceDescriptor {
    /// 接口号
    pub interface_number: u8,
    /// 交替设置
    pub alternate_setting: u8,
    /// 接口类
    pub interface_class: u8,
    /// 接口子类
    pub interface_subclass: u8,
    /// 接口协议
    pub interface_protocol: u8,
    /// 接口字符串索引
    pub interface_index: u8,
    /// 端点数量
    pub num_endpoints: u8,
}

/// USB端点描述符
#[derive(Debug, Clone)]
pub struct UsbEndpointDescriptor {
    /// 端点地址
    pub endpoint_address: u8,
    /// 端点属性
    pub attributes: u8,
    /// 最大包大小
    pub max_packet_size: u16,
    /// 传输间隔（ms）
    pub interval: u8,
    /// 传输类型
    pub transfer_type: UsbTransferType,
    /// 方向
    pub direction: UsbDirection,
}

/// USB设备类
#[derive(Debug, Clone)]
pub enum UsbDeviceClass {
    /// 音频类
    Audio,
    /// 通信类
    Communications,
    /// HID类
    Hid,
    /// 物理类
    Physical,
    /// 图像类
    Image,
    /// 打印机类
    Printer,
    /// 大容量存储类
    MassStorage,
    /// 集线器类
    Hub,
    /// CDC-Data类
    CdcData,
    /// 智能卡类
    SmartCard,
    /// 内容安全类
    ContentSecurity,
    /// 视频类
    Video,
    /// 个人医疗保健类
    PersonalHealthcare,
    /// 音视频类
    AudioVideo,
    /// 无线类
    Wireless,
    /// 杂项类
    Miscellaneous,
    /// 应用程序类
    ApplicationSpecific,
    /// 厂商特定类
    VendorSpecific(u8),
    /// 未知类
    Unknown,
}

/// USB传输管理器
pub struct UsbTransferManager {
    /// 待处理的传输
    pub pending_transfers: Arc<Mutex<BTreeMap<u32, UsbTransfer>>>,
    /// 完成的传输
    pub completed_transfers: Arc<Mutex<Vec<UsbTransfer>>>,
    /// 传输ID计数器
    pub next_transfer_id: AtomicU64,
    /// 最大并发传输数
    pub max_concurrent_transfers: AtomicUsize,
    /// 当前传输数
    pub current_transfers: AtomicUsize,
}

/// USB传输请求
#[derive(Clone)]
pub struct UsbTransfer {
    /// 传输ID
    pub transfer_id: u32,
    /// 设备地址
    pub device_address: u8,
    /// 端点地址
    pub endpoint_address: u8,
    /// 传输类型
    pub transfer_type: UsbTransferType,
    /// 方向
    pub direction: UsbDirection,
    /// 数据缓冲区
    pub data_buffer: Vec<u8>,
    /// 传输长度
    pub transfer_length: u32,
    /// 最大包大小
    pub max_packet_size: u16,
    /// 传输超时（毫秒）
    pub timeout: u32,
    /// 传输状态
    pub status: UsbTransferStatus,
    /// 完成回调
    pub completion_callback: Option<alloc::sync::Arc<dyn Fn(Result<(), UsbError>) + Send + Sync>>,
    /// 提交时间
    pub submit_time: u64,
    /// 完成时间
    pub complete_time: Option<u64>,
}

/// USB传输状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbTransferStatus {
    /// 等待中
    Pending,
    /// 正在进行
    InProgress,
    /// 已完成
    Completed,
    /// 已取消
    Cancelled,
    /// 错误
    Error,
    /// 超时
    Timeout,
}

/// USB类驱动程序接口
pub trait UsbClassDriver {
    /// 获取类驱动程序名称
    fn name(&self) -> &str;
    /// 支持的设备类
    fn supported_classes(&self) -> Vec<u8>;
    /// 初始化设备
    fn init_device(&self, device: &UsbDevice) -> Result<(), UsbError>;
    /// 关闭设备
    fn shutdown_device(&self, device: &UsbDevice) -> Result<(), UsbError>;
    /// 处理控制传输
    fn handle_control_transfer(&self, device: &UsbDevice, setup: &UsbSetupPacket) -> Result<Vec<u8>, UsbError>;
    /// 处理设备断开
    fn handle_device_disconnect(&self, device: &UsbDevice);
}

/// USB设置包
#[derive(Debug, Clone)]
pub struct UsbSetupPacket {
    /// 请求类型
    pub request_type: u8,
    /// 请求代码
    pub request: u8,
    /// 值
    pub value: u16,
    /// 索引
    pub index: u16,
    /// 长度
    pub length: u16,
}

/// USB统计信息
#[derive(Debug, Default)]
pub struct UsbStatistics {
    /// 总传输数
    pub total_transfers: AtomicU64,
    /// 成功传输数
    pub successful_transfers: AtomicU64,
    /// 失败传输数
    pub failed_transfers: AtomicU64,
    /// 控制传输数
    pub control_transfers: AtomicU64,
    /// 批量传输数
    pub bulk_transfers: AtomicU64,
    /// 中断传输数
    pub interrupt_transfers: AtomicU64,
    /// 等时传输数
    pub isochronous_transfers: AtomicU64,
    /// 传输字节数
    pub bytes_transferred: AtomicU64,
    /// 平均延迟（微秒）
    pub average_latency_us: AtomicU64,
    /// 枚举的设备数
    pub enumerated_devices: AtomicUsize,
    /// 当前连接的设备数
    pub connected_devices: AtomicUsize,
    /// 热插拔事件数
    pub hotplug_events: AtomicU64,
}

/// USB错误类型
#[derive(Debug, Clone)]
pub enum UsbError {
    /// 设备未响应
    DeviceNotResponding,
    /// 传输失败
    TransferFailed(String),
    /// 设备枚举失败
    EnumerationFailed(String),
    /// 描述符错误
    DescriptorError(String),
    /// 不支持的传输类型
    UnsupportedTransferType,
    /// 设备忙碌
    DeviceBusy,
    /// 传输超时
    TransferTimeout,
    /// 设备断开
    DeviceDisconnected,
    /// 内存不足
    OutOfMemory,
    /// 无效参数
    InvalidParameter(String),
    /// 权限错误
    PermissionDenied,
    /// 配置错误
    ConfigurationError(String),
}

impl core::fmt::Display for UsbError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UsbError::DeviceNotResponding => write!(f, "设备未响应"),
            UsbError::TransferFailed(msg) => write!(f, "传输失败: {}", msg),
            UsbError::EnumerationFailed(msg) => write!(f, "设备枚举失败: {}", msg),
            UsbError::DescriptorError(msg) => write!(f, "描述符错误: {}", msg),
            UsbError::UnsupportedTransferType => write!(f, "不支持的传输类型"),
            UsbError::DeviceBusy => write!(f, "设备忙碌"),
            UsbError::TransferTimeout => write!(f, "传输超时"),
            UsbError::DeviceDisconnected => write!(f, "设备断开"),
            UsbError::OutOfMemory => write!(f, "内存不足"),
            UsbError::InvalidParameter(msg) => write!(f, "无效参数: {}", msg),
            UsbError::PermissionDenied => write!(f, "权限错误"),
            UsbError::ConfigurationError(msg) => write!(f, "配置错误: {}", msg),
        }
    }
}

impl UsbHostController {
    /// 创建新的USB主机控制器
    pub fn new(id: u32, controller_type: UsbHostControllerType, register_base: usize) -> Self {
        Self {
            id,
            controller_type,
            register_base,
            interrupt_line: 0,
            usb_version: UsbVersion::USB_2_0,
            capabilities: UsbControllerCapabilities::default(),
            devices: Arc::new(Mutex::new(BTreeMap::new())),
            root_ports: Arc::new(Mutex::new(Vec::new())),
            transfer_manager: Arc::new(Mutex::new(UsbTransferManager::new())),
            statistics: UsbStatistics::default(),
            enabled: core::sync::atomic::AtomicBool::new(false),
        }
    }

    /// 初始化控制器
    pub fn initialize(&mut self) -> Result<(), UsbError> {
        crate::println!("[usb] 初始化控制器 {} ({:?})", self.id, self.controller_type);

        // 检测控制器能力
        self.detect_capabilities()?;

        // 重置控制器
        self.reset_controller()?;

        // 初始化根端口
        self.initialize_root_ports()?;

        // 启用控制器
        self.enable_controller()?;

        // 扫描设备
        self.scan_devices()?;

        self.enabled.store(true, Ordering::SeqCst);

        crate::println!("[usb] 控制器 {} 初始化完成", self.id);
        Ok(())
    }

    /// 关闭控制器
    pub fn shutdown(&mut self) -> Result<(), UsbError> {
        crate::println!("[usb] 关闭控制器 {}", self.id);

        self.enabled.store(false, Ordering::SeqCst);

        // 断开所有设备
        self.disconnect_all_devices()?;

        // 禁用控制器
        self.disable_controller()?;

        crate::println!("[usb] 控制器 {} 关闭完成", self.id);
        Ok(())
    }

    /// 提交USB传输
    pub fn submit_transfer(&mut self, transfer: UsbTransfer) -> Result<u32, UsbError> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Err(UsbError::ConfigurationError("控制器未启用".to_string()));
        }

        let mut manager = self.transfer_manager.lock();
        let transfer_id = manager.submit_transfer(transfer)?;

        // 执行传输（简化实现）
        self.execute_transfer(transfer_id)?;

        Ok(transfer_id)
    }

    /// 取消传输
    pub fn cancel_transfer(&self, transfer_id: u32) -> Result<(), UsbError> {
        let mut manager = self.transfer_manager.lock();
        manager.cancel_transfer(transfer_id)?;
        Ok(())
    }

    /// 获取设备
    pub fn get_device(&self, device_address: u8) -> Result<UsbDevice, UsbError> {
        let devices = self.devices.lock();
        devices.get(&device_address)
            .cloned()
            .ok_or(UsbError::DeviceNotResponding)
    }

    /// 获取所有设备
    pub fn get_all_devices(&self) -> Result<Vec<UsbDevice>, UsbError> {
        let devices = self.devices.lock();
        Ok(devices.values().cloned().collect())
    }

    /// 处理端口状态变化
    pub fn handle_port_status_change(&self, port_number: u8) -> Result<(), UsbError> {
        let mut ports = self.root_ports.lock();
        if let Some(port) = ports.get_mut(port_number as usize) {
            let old_status = port.status;
            let new_status = self.read_port_status(port_number)?;

            // 检测连接变化
            if old_status.connected && !new_status.connected {
                // 设备断开
                self.handle_device_disconnect(port_number)?;
            } else if !old_status.connected && new_status.connected {
                // 设备连接
                self.handle_device_connect(port_number)?;
            }

            port.status = new_status;
            port.last_status_change = time::timestamp_millis();
        }

        Ok(())
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> UsbStatistics {
        UsbStatistics {
            total_transfers: AtomicU64::new(self.statistics.total_transfers.load(Ordering::SeqCst)),
            successful_transfers: AtomicU64::new(self.statistics.successful_transfers.load(Ordering::SeqCst)),
            failed_transfers: AtomicU64::new(self.statistics.failed_transfers.load(Ordering::SeqCst)),
            control_transfers: AtomicU64::new(self.statistics.control_transfers.load(Ordering::SeqCst)),
            bulk_transfers: AtomicU64::new(self.statistics.bulk_transfers.load(Ordering::SeqCst)),
            interrupt_transfers: AtomicU64::new(self.statistics.interrupt_transfers.load(Ordering::SeqCst)),
            isochronous_transfers: AtomicU64::new(self.statistics.isochronous_transfers.load(Ordering::SeqCst)),
            bytes_transferred: AtomicU64::new(self.statistics.bytes_transferred.load(Ordering::SeqCst)),
            average_latency_us: AtomicU64::new(self.statistics.average_latency_us.load(Ordering::SeqCst)),
            enumerated_devices: AtomicUsize::new(self.statistics.enumerated_devices.load(Ordering::SeqCst)),
            connected_devices: AtomicUsize::new(self.statistics.connected_devices.load(Ordering::SeqCst)),
            hotplug_events: AtomicU64::new(self.statistics.hotplug_events.load(Ordering::SeqCst)),
        }
    }

    /// 私有辅助方法
    fn detect_capabilities(&mut self) -> Result<(), UsbError> {
        // 简化实现，实际需要读取控制器寄存器
        self.capabilities = UsbControllerCapabilities {
            port_count: 4,
            max_devices: 127,
            supported_transfers: vec![
                UsbTransferType::Control,
                UsbTransferType::Bulk,
                UsbTransferType::Interrupt,
                UsbTransferType::Isochronous,
            ],
            supported_speeds: vec![
                UsbSpeed::Low,
                UsbSpeed::Full,
                UsbSpeed::High,
            ],
            power_management: true,
            remote_wakeup: true,
            otg_support: false,
            address_64_bit: false,
            device_authentication: false,
        };

        Ok(())
    }

    fn reset_controller(&self) -> Result<(), UsbError> {
        crate::println!("[usb] 重置控制器 {}", self.id);

        // 简化实现，实际需要写入控制器寄存器
        match self.controller_type {
            UsbHostControllerType::Ehci => {
                // EHCI重置序列
                unsafe {
                    let usbcmd = (self.register_base + 0x20) as *mut u32;
                    let usbsts = (self.register_base + 0x24) as *mut u32;

                    // 停止控制器
                    *usbcmd = *usbcmd & !0x1;
                    // 等待停止
                    while (*usbsts & 0x1000) == 0 {
                        crate::arch::wfi();
                    }
                    // 重置控制器
                    *usbcmd = *usbcmd | 0x2;
                    // 等待重置完成
                    while (*usbcmd & 0x2) != 0 {
                        crate::arch::wfi();
                    }
                }
            }
            UsbHostControllerType::Xhci => {
                // XHCI重置序列
                unsafe {
                    let usbcmd = (self.register_base + 0x20) as *mut u32;
                    *usbcmd = *usbcmd | 0x1; // HCReset
                    while (*usbcmd & 0x1) != 0 {
                        crate::arch::wfi();
                    }
                }
            }
            _ => {
                return Err(UsbError::ConfigurationError("不支持的控制器类型".to_string()));
            }
        }

        Ok(())
    }

    fn initialize_root_ports(&self) -> Result<(), UsbError> {
        let mut ports = self.root_ports.lock();
        ports.clear();

        for i in 0..self.capabilities.port_count {
            let port = UsbPort {
                port_number: i,
                status: UsbPortStatus {
                    connected: false,
                    connect_change: false,
                    enabled: false,
                    enable_change: false,
                    over_current: false,
                    over_current_change: false,
                    reset: false,
                    power: true,
                    low_speed: false,
                    high_speed: false,
                    super_speed: false,
                },
                connected_device: None,
                capabilities: UsbPortCapabilities {
                    supported_speeds: vec![
                        UsbSpeed::Low,
                        UsbSpeed::Full,
                        UsbSpeed::High,
                    ],
                    max_current: 500,
                    remote_wakeup: true,
                    power_switching: true,
                    over_current_protection: true,
                },
                last_status_change: 0,
            };
            ports.push(port);
        }

        // 为所有端口供电
        for i in 0..self.capabilities.port_count {
            self.power_port(i)?;
        }

        Ok(())
    }

    fn enable_controller(&self) -> Result<(), UsbError> {
        crate::println!("[usb] 启用控制器 {}", self.id);

        match self.controller_type {
            UsbHostControllerType::Ehci => {
                unsafe {
                    let usbcmd = (self.register_base + 0x20) as *mut u32;
                    *usbcmd = *usbcmd | 0x1; // Run
                }
            }
            UsbHostControllerType::Xhci => {
                unsafe {
                    let usbsts = (self.register_base + 0x24) as *mut u32;
                    let usbcmd = (self.register_base + 0x20) as *mut u32;

                    // 清除所有状态位
                    *usbsts = 0xFFFFFFFF;
                    // 启用控制器
                    *usbcmd = *usbcmd | 0x1; // Run/Stop (R/S)
                }
            }
            _ => {
                return Err(UsbError::ConfigurationError("不支持的控制器类型".to_string()));
            }
        }

        Ok(())
    }

    fn disable_controller(&self) -> Result<(), UsbError> {
        match self.controller_type {
            UsbHostControllerType::Ehci => {
                unsafe {
                    let usbcmd = (self.register_base + 0x20) as *mut u32;
                    *usbcmd = *usbcmd & !0x1; // Stop
                }
            }
            UsbHostControllerType::Xhci => {
                unsafe {
                    let usbcmd = (self.register_base + 0x20) as *mut u32;
                    *usbcmd = *usbcmd & !0x1; // Run/Stop (R/S)
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn power_port(&self, port_number: u8) -> Result<(), UsbError> {
        match self.controller_type {
            UsbHostControllerType::Ehci => {
                let portsc_offset = 0x64 + (port_number as usize) * 4;
                unsafe {
                    let portsc = (self.register_base + portsc_offset) as *mut u32;
                    *portsc = *portsc | 0x1000; // PP - Port Power
                }
            }
            UsbHostControllerType::Xhci => {
                // XHCI端口电源管理更复杂，这里简化
            }
            _ => {}
        }

        Ok(())
    }

    fn read_port_status(&self, port_number: u8) -> Result<UsbPortStatus, UsbError> {
        let mut status = UsbPortStatus {
            connected: false,
            connect_change: false,
            enabled: false,
            enable_change: false,
            over_current: false,
            over_current_change: false,
            reset: false,
            power: true,
            low_speed: false,
            high_speed: false,
            super_speed: false,
        };

        match self.controller_type {
            UsbHostControllerType::Ehci => {
                let portsc_offset = 0x64 + (port_number as usize) * 4;
                unsafe {
                    let portsc = (self.register_base + portsc_offset) as *const u32;
                    let portsc_val = *portsc;

                    status.connected = (portsc_val & 0x1) != 0;
                    status.connect_change = (portsc_val & 0x2) != 0;
                    status.enabled = (portsc_val & 0x4) != 0;
                    status.enable_change = (portsc_val & 0x8) != 0;
                    status.over_current = (portsc_val & 0x10) != 0;
                    status.over_current_change = (portsc_val & 0x20) != 0;
                    status.reset = (portsc_val & 0x100) != 0;
                    status.power = (portsc_val & 0x1000) != 0;
                    status.low_speed = (portsc_val & 0x200) != 0;
                    status.high_speed = (portsc_val & 0x400) != 0;
                }
            }
            UsbHostControllerType::Xhci => {
                // XHCI端口状态读取，简化实现
            }
            _ => {
                return Err(UsbError::ConfigurationError("不支持的控制器类型".to_string()));
            }
        }

        Ok(status)
    }

    fn scan_devices(&self) -> Result<(), UsbError> {
        crate::println!("[usb] 扫描设备");

        let ports = self.root_ports.lock();
        for (i, port) in ports.iter().enumerate() {
            if port.status.connected {
                self.enumerate_device(i as u8)?;
            }
        }

        Ok(())
    }

    fn handle_device_connect(&self, port_number: u8) -> Result<(), UsbError> {
        crate::println!("[usb] 端口 {} 设备连接", port_number);

        self.statistics.hotplug_events.fetch_add(1, Ordering::SeqCst);

        // 重置设备
        self.reset_device(port_number)?;

        // 枚举设备
        self.enumerate_device(port_number)?;

        Ok(())
    }

    fn handle_device_disconnect(&self, port_number: u8) -> Result<(), UsbError> {
        crate::println!("[usb] 端口 {} 设备断开", port_number);

        self.statistics.hotplug_events.fetch_add(1, Ordering::SeqCst);

        let mut ports = self.root_ports.lock();
        if let Some(port) = ports.get_mut(port_number as usize) {
            if let Some(device_address) = port.connected_device {
                // 移除设备
                let mut devices = self.devices.lock();
                if let Some(device) = devices.remove(&device_address) {
                    // 通知类驱动程序
                    self.notify_device_disconnected(&device);
                }

                port.connected_device = None;
                self.statistics.connected_devices.fetch_sub(1, Ordering::SeqCst);
            }
        }

        Ok(())
    }

    fn reset_device(&self, port_number: u8) -> Result<(), UsbError> {
        crate::println!("[usb] 重置端口 {} 的设备", port_number);

        match self.controller_type {
            UsbHostControllerType::Ehci => {
                let portsc_offset = 0x64 + (port_number as usize) * 4;
                unsafe {
                    let portsc = (self.register_base + portsc_offset) as *mut u32;

                    // 设置复位位
                    *portsc = *portsc | 0x100;
                    // 等待复位完成
                    while (*portsc & 0x100) != 0 {
                        crate::arch::wfi();
                    }
                }
            }
            UsbHostControllerType::Xhci => {
                // XHCI设备复位
            }
            _ => {
                return Err(UsbError::ConfigurationError("不支持的控制器类型".to_string()));
            }
        }

        Ok(())
    }

    fn enumerate_device(&self, port_number: u8) -> Result<(), UsbError> {
        crate::println!("[usb] 枚举端口 {} 的设备", port_number);

        // 分配设备地址
        let device_address = self.allocate_device_address()?;

        // 获取设备描述符
        let device_descriptor = self.get_device_descriptor(device_address)?;

        // 检测设备类
        let device_class = self.detect_device_class(&device_descriptor);

        // 创建设备对象
        let device = UsbDevice {
            address: device_address,
            state: UsbDeviceState::Enumerating,
            device_descriptor,
            configurations: Vec::new(),
            current_configuration: None,
            interfaces: Vec::new(),
            endpoints: Vec::new(),
            string_descriptors: BTreeMap::new(),
            device_class: device_class.clone(),
            speed: UsbSpeed::Full, // 需要检测
            port_number,
            parent_address: None,
            connection_time: time::timestamp_millis(),
        };

        // 设置设备地址
        self.set_device_address(device_address, device_address)?;

        // 获取配置描述符
        self.get_configurations(&device)?;

        // 选择配置
        self.select_configuration(&device, 1)?;

        // 添加到设备列表
        {
            let mut devices = self.devices.lock();
            devices.insert(device_address, device.clone());

            // 更新端口信息
            let mut ports = self.root_ports.lock();
            if let Some(port) = ports.get_mut(port_number as usize) {
                port.connected_device = Some(device_address);
            }
        }

        // 更新统计
        self.statistics.enumerated_devices.fetch_add(1, Ordering::SeqCst);
        self.statistics.connected_devices.fetch_add(1, Ordering::SeqCst);

        crate::println!("[usb] 设备枚举完成: 地址={}, 类={:?}", device_address, device_class);

        // 通知类驱动程序
        self.notify_device_connected(&device);

        Ok(())
    }

    fn allocate_device_address(&self) -> Result<u8, UsbError> {
        let devices = self.devices.lock();
        for addr in 1..128 {
            if !devices.contains_key(&addr) {
                return Ok(addr);
            }
        }
        Err(UsbError::ConfigurationError("无可用设备地址".to_string()))
    }

    fn get_device_descriptor(&self, _device_address: u8) -> Result<UsbDeviceDescriptor, UsbError> {
        // 简化实现，返回示例描述符
        Ok(UsbDeviceDescriptor {
            length: 18,
            descriptor_type: 1,
            usb_version: 0x0200,
            device_class: 0,
            device_subclass: 0,
            device_protocol: 0,
            max_packet_size: 64,
            vendor_id: 0x1234,
            product_id: 0x5678,
            device_version: 0x0100,
            manufacturer_index: 1,
            product_index: 2,
            serial_number_index: 3,
            num_configurations: 1,
        })
    }

    fn set_device_address(&self, _device_address: u8, _new_address: u8) -> Result<(), UsbError> {
        // 简化实现
        Ok(())
    }

    fn get_configurations(&self, _device: &UsbDevice) -> Result<(), UsbError> {
        // 简化实现
        Ok(())
    }

    fn select_configuration(&self, _device: &UsbDevice, _config_value: u8) -> Result<(), UsbError> {
        // 简化实现
        Ok(())
    }

    fn detect_device_class(&self, descriptor: &UsbDeviceDescriptor) -> UsbDeviceClass {
        match descriptor.device_class {
            0x01 => UsbDeviceClass::Audio,
            0x02 => UsbDeviceClass::Communications,
            0x03 => UsbDeviceClass::Hid,
            0x05 => UsbDeviceClass::Physical,
            0x06 => UsbDeviceClass::Image,
            0x07 => UsbDeviceClass::Printer,
            0x08 => UsbDeviceClass::MassStorage,
            0x09 => UsbDeviceClass::Hub,
            0x0A => UsbDeviceClass::CdcData,
            0x0B => UsbDeviceClass::SmartCard,
            0x0D => UsbDeviceClass::ContentSecurity,
            0x0E => UsbDeviceClass::Video,
            0x0F => UsbDeviceClass::PersonalHealthcare,
            0x10 => UsbDeviceClass::AudioVideo,
            0xE0 => UsbDeviceClass::Wireless,
            0xEF => UsbDeviceClass::Miscellaneous,
            0xFE => UsbDeviceClass::ApplicationSpecific,
            0xFF => UsbDeviceClass::VendorSpecific(descriptor.device_subclass),
            _ => UsbDeviceClass::Unknown,
        }
    }

    fn notify_device_connected(&self, _device: &UsbDevice) {
        // 简化实现，实际需要通知相应的类驱动程序
        crate::println!("[usb] 设备连接事件通知");
    }

    fn notify_device_disconnected(&self, _device: &UsbDevice) {
        // 简化实现，实际需要通知相应的类驱动程序
        crate::println!("[usb] 设备断开事件通知");
    }

    fn disconnect_all_devices(&self) -> Result<(), UsbError> {
        let devices = self.devices.lock();
        for (_, device) in devices.iter() {
            self.notify_device_disconnected(device);
        }

        let mut devices = self.devices.lock();
        devices.clear();

        let mut ports = self.root_ports.lock();
        for port in ports.iter_mut() {
            port.connected_device = None;
        }

        self.statistics.connected_devices.store(0, Ordering::SeqCst);

        Ok(())
    }

    fn execute_transfer(&self, transfer_id: u32) -> Result<(), UsbError> {
        let mut manager = self.transfer_manager.lock();
        manager.execute_transfer(transfer_id)
    }
}

impl UsbTransferManager {
    /// 创建新的传输管理器
    pub fn new() -> Self {
        Self {
            pending_transfers: Arc::new(Mutex::new(BTreeMap::new())),
            completed_transfers: Arc::new(Mutex::new(Vec::new())),
            next_transfer_id: AtomicU64::new(1),
            max_concurrent_transfers: AtomicUsize::new(64),
            current_transfers: AtomicUsize::new(0),
        }
    }

    /// 提交传输
    pub fn submit_transfer(&mut self, mut transfer: UsbTransfer) -> Result<u32, UsbError> {
        let transfer_id = self.next_transfer_id.fetch_add(1, Ordering::SeqCst) as u32;

        // 检查并发传输限制
        let current = self.current_transfers.load(Ordering::SeqCst);
        let max = self.max_concurrent_transfers.load(Ordering::SeqCst);
        if current >= max {
            return Err(UsbError::DeviceBusy);
        }

        transfer.transfer_id = transfer_id;
        transfer.status = UsbTransferStatus::Pending;
        transfer.submit_time = time::timestamp_nanos();

        let mut pending = self.pending_transfers.lock();
        pending.insert(transfer_id, transfer);

        self.current_transfers.fetch_add(1, Ordering::SeqCst);

        Ok(transfer_id)
    }

    /// 取消传输
    pub fn cancel_transfer(&mut self, transfer_id: u32) -> Result<(), UsbError> {
        let mut pending = self.pending_transfers.lock();
        if let Some(mut transfer) = pending.remove(&transfer_id) {
            transfer.status = UsbTransferStatus::Cancelled;
            transfer.complete_time = Some(time::timestamp_nanos());

            let mut completed = self.completed_transfers.lock();
            completed.push(transfer);

            self.current_transfers.fetch_sub(1, Ordering::SeqCst);
        }

        Ok(())
    }

    /// 执行传输
    pub fn execute_transfer(&mut self, transfer_id: u32) -> Result<(), UsbError> {
        let transfer = {
            let pending = self.pending_transfers.lock();
            pending.get(&transfer_id).cloned()
        };

        if let Some(mut transfer) = transfer {
            transfer.status = UsbTransferStatus::InProgress;

            // 简化实现：模拟传输成功
            transfer.status = UsbTransferStatus::Completed;
            transfer.complete_time = Some(time::timestamp_nanos());

            // 从待处理列表移除
            let mut pending = self.pending_transfers.lock();
            pending.remove(&transfer_id);

            // 添加到完成列表
            let mut completed = self.completed_transfers.lock();
            completed.push(transfer);

            self.current_transfers.fetch_sub(1, Ordering::SeqCst);

            // 调用完成回调
            if let Some(ref transfer) = completed.last() {
                if let Some(ref callback) = transfer.completion_callback {
                    callback(Ok(()));
                }
            }
        }

        Ok(())
    }
}

impl Default for UsbControllerCapabilities {
    fn default() -> Self {
        Self {
            port_count: 4,
            max_devices: 127,
            supported_transfers: vec![
                UsbTransferType::Control,
                UsbTransferType::Bulk,
                UsbTransferType::Interrupt,
                UsbTransferType::Isochronous,
            ],
            supported_speeds: vec![
                UsbSpeed::Low,
                UsbSpeed::Full,
                UsbSpeed::High,
            ],
            power_management: true,
            remote_wakeup: true,
            otg_support: false,
            address_64_bit: false,
            device_authentication: false,
        }
    }
}
