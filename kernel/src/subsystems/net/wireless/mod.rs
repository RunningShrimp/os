//! Wireless networking subsystem
//!
//! This module provides comprehensive wireless networking support including:
//! - Wi-Fi (802.11 a/b/g/n/ac/ax)
//! - Bluetooth (Classic and BLE)
//! - Cellular (4G LTE/5G NR)
//! - LoRaWAN IoT
//! - Wireless device drivers
//! - Network virtualization (VXLAN, Geneve, WireGuard)

#![allow(dead_code)]

use alloc::{sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicU32, Ordering};

pub mod wifi;
pub mod bluetooth;
pub mod cellular;
pub mod lora;
pub mod driver;
pub mod virtual_net;

pub use wifi::{
    WifiDevice, WifiConfig, WifiSecurity, WifiState, WifiMode,
    WifiAuthType, WifiPhyType, WifiChannel, WifiScanResult,
    WifiStats, WpaConfig, WpaVersion,
};
pub use bluetooth::{
    BluetoothDevice, BluetoothConfig, BluetoothState,
    BluetoothMode, BluetoothVersion, BdAddr, BluetoothClass,
    BluetoothSdpRecord, A2dpConfig, HfpConfig, GattService,
    BluetoothStats,
};
pub use cellular::{
    CellularDevice, CellularConfig, CellularState,
    CellularTech, CellularRegState,
    SimState, CellularStats, CellularApn, CellularDataCall,
    LteConfig, NrConfig, NasState, RrcState,
};
pub use lora::{
    LoRaDevice, LoRaConfig, LoRaState, LoRaClass,
    LoRaChannelPlan, LoRaDataRate, LoRaJoinMode,
    LoRaMessage, LoRaStats,
};
pub use driver::{
    WirelessDriver, WirelessDriverConfig, WirelessDriverState,
    Mac80211Device, Cfg80211Config, RegulatoryDomain,
    WirelessCapabilities, WirelessStats, FirmwareInfo,
};
pub use virtual_net::{
    VirtualDevice, VirtualConfig, VirtualDeviceType,
    VxlanConfig, GeneveConfig, GreConfig, WireGuardConfig,
    VlanConfig, VirtualStats, TunnelEndpoint,
};

/// Wireless device identifier
pub type WirelessDeviceId = u32;

/// Wireless subsystem
pub struct WirelessSubsystem {
    /// Next device ID
    next_id: AtomicU32,
    /// Wi-Fi devices
    wifi_devices: Vec<Arc<wifi::WifiDevice>>,
    /// Bluetooth devices
    bluetooth_devices: Vec<Arc<bluetooth::BluetoothDevice>>,
    /// Cellular devices
    cellular_devices: Vec<Arc<cellular::CellularDevice>>,
    /// LoRa devices
    lora_devices: Vec<Arc<lora::LoRaDevice>>,
    /// Virtual devices
    virtual_devices: Vec<Arc<virtual_net::VirtualDevice>>,
}

impl WirelessSubsystem {
    /// Create a new wireless subsystem
    pub fn new() -> Self {
        Self {
            next_id: AtomicU32::new(1),
            wifi_devices: Vec::new(),
            bluetooth_devices: Vec::new(),
            cellular_devices: Vec::new(),
            lora_devices: Vec::new(),
            virtual_devices: Vec::new(),
        }
    }

    /// Add a Wi-Fi device
    pub fn add_wifi_device(&mut self, device: WifiDevice) -> WirelessDeviceId {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let device = Arc::new(device);
        self.wifi_devices.push(device);
        id
    }

    /// Add a Bluetooth device
    pub fn add_bluetooth_device(&mut self, device: BluetoothDevice) -> WirelessDeviceId {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let device = Arc::new(device);
        self.bluetooth_devices.push(device);
        id
    }

    /// Add a cellular device
    pub fn add_cellular_device(&mut self, device: CellularDevice) -> WirelessDeviceId {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let device = Arc::new(device);
        self.cellular_devices.push(device);
        id
    }

    /// Add a LoRa device
    pub fn add_lora_device(&mut self, device: LoRaDevice) -> WirelessDeviceId {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let device = Arc::new(device);
        self.lora_devices.push(device);
        id
    }

    /// Add a virtual device
    pub fn add_virtual_device(&mut self, device: VirtualDevice) -> WirelessDeviceId {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let device = Arc::new(device);
        self.virtual_devices.push(device);
        id
    }

    /// Get Wi-Fi device by ID
    pub fn get_wifi_device(&self, id: WirelessDeviceId) -> Option<&Arc<wifi::WifiDevice>> {
        self.wifi_devices.iter().find(|d| d.id() == id)
    }

    /// Get Bluetooth device by ID
    pub fn get_bluetooth_device(&self, id: WirelessDeviceId) -> Option<&Arc<bluetooth::BluetoothDevice>> {
        self.bluetooth_devices.iter().find(|d| d.id() == id)
    }

    /// Get cellular device by ID
    pub fn get_cellular_device(&self, id: WirelessDeviceId) -> Option<&Arc<cellular::CellularDevice>> {
        self.cellular_devices.iter().find(|d| d.id() == id)
    }

    /// Get LoRa device by ID
    pub fn get_lora_device(&self, id: WirelessDeviceId) -> Option<&Arc<lora::LoRaDevice>> {
        self.lora_devices.iter().find(|d| d.id() == id)
    }

    /// Get virtual device by ID
    pub fn get_virtual_device(&self, id: WirelessDeviceId) -> Option<&Arc<virtual_net::VirtualDevice>> {
        self.virtual_devices.iter().find(|d| d.id() == id)
    }

    /// Scan for wireless networks
    pub fn scan_networks(&self) -> Result<Vec<WifiScanResult>, WirelessError> {
        let mut results = Vec::new();
        for device in &self.wifi_devices {
            if device.is_enabled() {
                let device_results = device.scan()?;
                results.extend(device_results);
            }
        }
        Ok(results)
    }
}

/// Wireless error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WirelessError {
    /// Device not found
    DeviceNotFound,
    /// Operation not supported
    NotSupported,
    /// Invalid configuration
    InvalidConfig,
    /// Device busy
    DeviceBusy,
    /// Timeout
    Timeout,
    /// Authentication failed
    AuthenticationFailed,
    /// Connection failed
    ConnectionFailed,
    /// Scan failed
    ScanFailed,
    /// Firmware error
    FirmwareError,
    /// Hardware error
    HardwareError,
    /// No memory
    NoMemory,
    /// Invalid parameter
    InvalidParameter,
    /// Operation in progress
    InProgress,
    /// Not connected
    NotConnected,
    /// Already connected
    AlreadyConnected,
}

impl From<wifi::WifiError> for WirelessError {
    fn from(err: wifi::WifiError) -> Self {
        match err {
            // Map common errors
            _ => Self::ScanFailed,
        }
    }
}

impl core::fmt::Display for WirelessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DeviceNotFound => write!(f, "Device not found"),
            Self::NotSupported => write!(f, "Operation not supported"),
            Self::InvalidConfig => write!(f, "Invalid configuration"),
            Self::DeviceBusy => write!(f, "Device busy"),
            Self::Timeout => write!(f, "Operation timeout"),
            Self::AuthenticationFailed => write!(f, "Authentication failed"),
            Self::ConnectionFailed => write!(f, "Connection failed"),
            Self::ScanFailed => write!(f, "Scan failed"),
            Self::FirmwareError => write!(f, "Firmware error"),
            Self::HardwareError => write!(f, "Hardware error"),
            Self::NoMemory => write!(f, "No memory available"),
            Self::InvalidParameter => write!(f, "Invalid parameter"),
            Self::InProgress => write!(f, "Operation in progress"),
            Self::NotConnected => write!(f, "Not connected"),
            Self::AlreadyConnected => write!(f, "Already connected"),
        }
    }
}

/// Global wireless subsystem instance
static mut WIRELESS_SUBSYSTEM: Option<WirelessSubsystem> = None;
static WIRELESS_INIT: crate::subsystems::sync::Once = crate::subsystems::sync::Once::new();

/// Get the global wireless subsystem
pub fn wireless_subsystem() -> &'static mut WirelessSubsystem {
    unsafe {
        WIRELESS_INIT.call_once(|| {
            WIRELESS_SUBSYSTEM = Some(WirelessSubsystem::new());
        });
        WIRELESS_SUBSYSTEM.as_mut().unwrap()
    }
}

/// Initialize the wireless subsystem
pub fn init() {
    let _subsystem = wireless_subsystem();
    crate::log_info!("Wireless subsystem initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wireless_subsystem_creation() {
        let subsystem = WirelessSubsystem::new();
        assert_eq!(subsystem.wifi_devices.len(), 0);
        assert_eq!(subsystem.bluetooth_devices.len(), 0);
    }
}
