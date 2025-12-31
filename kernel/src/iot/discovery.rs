//! # Device Discovery and Provisioning
//!
//! This module provides comprehensive device discovery and provisioning capabilities
//! for IoT devices using various protocols and standards.
//!
//! ## Supported Discovery Methods
//!
//! - **mDNS (Multicast DNS)**: DNS-based service discovery
//! - **DNS-SD (DNS-based Service Discovery)**: Service registration and discovery
//! - **SSDP (Simple Service Discovery Protocol)**: UPnP device discovery
//! - **UPnP**: Universal Plug and Play
//! - **ZeroConf**: Zero-configuration networking
//!
//! ## Features
//!
//! - Automatic device enumeration
//! - Service registration and discovery
//! - Device identification and pairing
//! - Dynamic device addition/removal
//! - Service metadata management
//! - Multi-homed network support
//!
//! ## Usage Examples
//!
//! ### Discover devices
//!
//! ```no_run
//! use kernel::iot::discovery::DeviceDiscovery;
//!
//! let discovery = DeviceDiscovery::new();
//! let devices = discovery.discover_mdns_devices("_mqtt._tcp").await?;
//! for device in devices {
//!     println!("Found: {}", device.name);
//! }
//! # Ok::<(), kernel::iot::IotError>(())
//! ```

#![allow(dead_code)]
#![warn(missing_docs)]

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use core::sync::atomic::AtomicU32;

use crate::iot::{DeviceId, DeviceInfo, IotError, IotResult};

// =============================================================================
// mDNS (Multicast DNS)
// =============================================================================

/// mDNS record type
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u16)]
pub enum MdnsRecordType {
    A = 1,
    AAAA = 28,
    PTR = 12,
    TXT = 16,
    SRV = 33,
    ANY = 255,
}

/// mDNS service record
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsServiceRecord {
    /// Service instance name
    pub instance: String,
    /// Service type (e.g., "_mqtt._tcp")
    pub service_type: String,
    /// Domain (usually "local")
    pub domain: String,
    /// Hostname
    pub hostname: String,
    /// Port
    pub port: u16,
    /// TXT records
    pub txt: BTreeMap<String, String>,
    /// TTL in seconds
    pub ttl: u32,
    /// Last update timestamp
    pub last_update: u64,
}

impl MdnsServiceRecord {
    /// Create a new mDNS service record
    pub fn new(
        instance: impl Into<String>,
        service_type: impl Into<String>,
        hostname: impl Into<String>,
        port: u16,
    ) -> Self {
        Self {
            instance: instance.into(),
            service_type: service_type.into(),
            domain: "local".to_string(),
            hostname: hostname.into(),
            port,
            txt: BTreeMap::new(),
            ttl: 4500,
            last_update: 0,
        }
    }

    /// Full service name (instance.service_type.domain)
    pub fn full_name(&self) -> String {
        format!("{}.{}.{}", self.instance, self.service_type, self.domain)
    }

    /// Add TXT record
    pub fn add_txt(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.txt.insert(key.into(), value.into());
    }

    /// Get TXT value
    pub fn get_txt(&self, key: &str) -> Option<&String> {
        self.txt.get(key)
    }
}

/// mDNS responder
pub struct MdnsResponder {
    /// Hostname
    hostname: String,
    /// Registered services
    services: BTreeMap<String, MdnsServiceRecord>,
    /// Record sequence number
    seq_num: AtomicU32,
}

impl MdnsResponder {
    /// Create a new mDNS responder
    pub fn new(hostname: impl Into<String>) -> Self {
        Self {
            hostname: hostname.into(),
            services: BTreeMap::new(),
            seq_num: AtomicU32::new(0),
        }
    }

    /// Register a service
    pub fn register_service(&mut self, service: MdnsServiceRecord) -> IotResult<()> {
        let key = service.full_name();
        self.services.insert(key.clone(), service);
        crate::log_info!("mDNS registered service: {}", key);
        Ok(())
    }

    /// Unregister a service
    pub fn unregister_service(&mut self, service_name: &str) -> IotResult<()> {
        if self.services.remove(service_name).is_some() {
            crate::log_info!("mDNS unregistered service: {}", service_name);
            Ok(())
        } else {
            Err(IotError::ServiceNotFound(service_name.to_string()))
        }
    }

    /// Get registered services
    pub fn services(&self) -> &BTreeMap<String, MdnsServiceRecord> {
        &self.services
    }

    /// Respond to mDNS query
    pub fn handle_query(&self, _service_type: &str) -> Vec<MdnsServiceRecord> {
        // In a real implementation, this would:
        // 1. Parse mDNS query packet
        // 2. Match against registered services
        // 3. Construct and send response

        self.services.values().cloned().collect()
    }
}

/// mDNS browser
pub struct MdnsBrowser {
    /// Discovered services
    discovered: BTreeMap<String, MdnsServiceRecord>,
    /// Last update time
    last_update: u64,
}

impl MdnsBrowser {
    /// Create a new mDNS browser
    pub fn new() -> Self {
        Self {
            discovered: BTreeMap::new(),
            last_update: 0,
        }
    }

    /// Browse for services
    pub fn browse(&mut self, service_type: impl Into<String>) -> IotResult<Vec<MdnsServiceRecord>> {
        let service_type = service_type.into();

        // In a real implementation, this would:
        // 1. Send mDNS query packet
        // 2. Listen for responses
        // 3. Parse and cache service records

        crate::log_info!("mDNS browsing for: {}", &service_type);

        Ok(self
            .discovered
            .values()
            .filter(|s| s.service_type == service_type)
            .cloned()
            .collect())
    }

    /// Resolve service (get full details)
    pub fn resolve(&mut self, service_name: &str) -> IotResult<&MdnsServiceRecord> {
        self.discovered
            .get(service_name)
            .ok_or_else(|| IotError::ServiceNotFound(service_name.to_string()))
    }

    /// Add discovered service
    pub fn add_service(&mut self, service: MdnsServiceRecord) {
        let key = service.full_name();
        self.discovered.insert(key, service);
    }

    /// Remove expired services
    pub fn cleanup(&mut self, current_time: u64) {
        let ttl_threshold = current_time.saturating_sub(4500); // 75 minutes

        self.discovered
            .retain(|_, s| s.last_update > ttl_threshold);
    }

    /// Get all discovered services
    pub fn discovered(&self) -> &BTreeMap<String, MdnsServiceRecord> {
        &self.discovered
    }
}

impl Default for MdnsBrowser {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// DNS-SD (DNS-based Service Discovery)
// =============================================================================

/// DNS-SD service type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsSdServiceType {
    /// Service name (e.g., "mqtt")
    pub name: String,
    /// Protocol (tcp or udp)
    pub protocol: String,
}

impl DnsSdServiceType {
    /// Create a new DNS-SD service type
    pub fn new(name: impl Into<String>, protocol: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            protocol: protocol.into(),
        }
    }

    /// Full service type string (e.g., "_mqtt._tcp")
    pub fn to_string(&self) -> String {
        format!("_{}.{}", self.name, self.protocol)
    }
}

/// DNS-SD service instance
#[derive(Debug, Clone)]
pub struct DnsSdInstance {
    /// Instance name
    pub name: String,
    /// Service type
    pub service_type: DnsSdServiceType,
    /// Domain
    pub domain: String,
    /// Hostname
    pub hostname: String,
    /// Port
    pub port: u16,
    /// TXT records
    pub txt: BTreeMap<String, String>,
    /// IPv4 addresses
    pub ipv4: Vec<[u8; 4]>,
    /// IPv6 addresses
    pub ipv6: Vec<[u8; 16]>,
}

impl DnsSdInstance {
    /// Create a new DNS-SD instance
    pub fn new(
        name: impl Into<String>,
        service_type: DnsSdServiceType,
        hostname: impl Into<String>,
        port: u16,
    ) -> Self {
        Self {
            name: name.into(),
            service_type,
            domain: "local".to_string(),
            hostname: hostname.into(),
            port,
            txt: BTreeMap::new(),
            ipv4: Vec::new(),
            ipv6: Vec::new(),
        }
    }

    /// Full instance name
    pub fn full_name(&self) -> String {
        format!(
            "{}.{}.{}",
            self.name,
            self.service_type.to_string(),
            self.domain
        )
    }

    /// Add TXT record
    pub fn add_txt(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.txt.insert(key.into(), value.into());
    }

    /// Add IPv4 address
    pub fn add_ipv4(&mut self, addr: [u8; 4]) {
        self.ipv4.push(addr);
    }

    /// Add IPv6 address
    pub fn add_ipv6(&mut self, addr: [u8; 16]) {
        self.ipv6.push(addr);
    }
}

// =============================================================================
// SSDP (Simple Service Discovery Protocol)
// =============================================================================

/// SSDP search target
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsdpSearchTarget {
    /// Search all devices
    All,
    /// Root devices
    RootDevice,
    /// Specific device type
    DeviceType(String),
    /// Specific UUID
    Uuid(String),
}

/// SSDP notification type
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum SsdpNotifyType {
    Alive = 0,
    Byebye = 1,
    Update = 2,
}

/// SSDP message
#[derive(Debug, Clone)]
pub struct SsdpMessage {
    /// Message type (M-SEARCH, NOTIFY, etc.)
    pub message_type: String,
    /// Search target
    pub search_target: SsdpSearchTarget,
    /// Unique service name
    pub usn: String,
    /// Location (URL)
    pub location: String,
    /// Server
    pub server: String,
    /// Cache control (max-age)
    pub cache_control: u32,
    /// Host
    pub host: String,
    /// Notification type (for NOTIFY messages)
    pub notify_type: Option<SsdpNotifyType>,
    /// Additional headers
    pub headers: BTreeMap<String, String>,
}

impl SsdpMessage {
    /// Create a new SSDP M-SEARCH message
    pub fn new_search(search_target: SsdpSearchTarget, mx: u32) -> Self {
        Self {
            message_type: "M-SEARCH".to_string(),
            search_target,
            usn: String::new(),
            location: String::new(),
            server: String::new(),
            cache_control: mx,
            host: "239.255.255.250:1900".to_string(),
            notify_type: None,
            headers: BTreeMap::new(),
        }
    }

    /// Create a new SSDP NOTIFY message
    pub fn new_notify(
        notify_type: SsdpNotifyType,
        search_target: SsdpSearchTarget,
        location: impl Into<String>,
        usn: impl Into<String>,
    ) -> Self {
        Self {
            message_type: "NOTIFY".to_string(),
            search_target,
            usn: usn.into(),
            location: location.into(),
            server: String::new(),
            cache_control: 1800,
            host: "239.255.255.250:1900".to_string(),
            notify_type: Some(notify_type),
            headers: BTreeMap::new(),
        }
    }
}

/// SSDP server
pub struct SsdpServer {
    /// Server UUID
    uuid: String,
    /// Registered devices
    devices: BTreeMap<String, SsdpMessage>,
    /// Listening flag
    listening: bool,
}

impl SsdpServer {
    /// Create a new SSDP server
    pub fn new(uuid: impl Into<String>) -> Self {
        Self {
            uuid: uuid.into(),
            devices: BTreeMap::new(),
            listening: false,
        }
    }

    /// Register a device
    pub fn register_device(&mut self, device: SsdpMessage) -> IotResult<()> {
        let usn = device.usn.clone();
        self.devices.insert(usn.clone(), device);
        crate::log_info!("SSDP registered device: {}", usn);
        Ok(())
    }

    /// Unregister a device
    pub fn unregister_device(&mut self, usn: &str) -> IotResult<()> {
        if self.devices.remove(usn).is_some() {
            crate::log_info!("SSDP unregistered device: {}", usn);
            Ok(())
        } else {
            Err(IotError::DeviceNotFound(usn.to_string()))
        }
    }

    /// Start listening
    pub fn start(&mut self) -> IotResult<()> {
        self.listening = true;
        crate::log_info!("SSDP server started");
        Ok(())
    }

    /// Stop listening
    pub fn stop(&mut self) -> IotResult<()> {
        self.listening = false;
        crate::log_info!("SSDP server stopped");
        Ok(())
    }

    /// Handle M-SEARCH request
    pub fn handle_search(&self, _search_target: &SsdpSearchTarget) -> Vec<SsdpMessage> {
        self.devices.values().cloned().collect()
    }
}

/// SSDP client
pub struct SsdpClient {
    /// Discovered devices
    devices: BTreeMap<String, SsdpMessage>,
}

impl SsdpClient {
    /// Create a new SSDP client
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
        }
    }

    /// Search for devices
    pub fn search(&mut self, search_target: SsdpSearchTarget) -> IotResult<Vec<SsdpMessage>> {
        crate::log_info!("SSDP search: {:?}", search_target);

        // In a real implementation, this would:
        // 1. Send M-SEARCH message
        // 2. Listen for responses
        // 3. Parse and cache responses

        Ok(self.devices.values().cloned().collect())
    }

    /// Add discovered device
    pub fn add_device(&mut self, device: SsdpMessage) {
        let usn = device.usn.clone();
        self.devices.insert(usn, device);
    }

    /// Remove device
    pub fn remove_device(&mut self, usn: &str) {
        self.devices.remove(usn);
    }

    /// Get discovered devices
    pub fn devices(&self) -> &BTreeMap<String, SsdpMessage> {
        &self.devices
    }
}

impl Default for SsdpClient {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// UPnP Device Discovery
// =============================================================================

/// UPnP device type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpnpDeviceType {
    /// Domain
    pub domain: String,
    /// Device type
    pub device_type: String,
    /// Version
    pub version: u8,
}

impl UpnpDeviceType {
    /// Create a new UPnP device type
    pub fn new(device_type: impl Into<String>, version: u8) -> Self {
        Self {
            domain: "schemas-upnp-org".to_string(),
            device_type: device_type.into(),
            version,
        }
    }

    /// Full device type string
    pub fn to_string(&self) -> String {
        format!("urn:{}:device:{}:{}", self.domain, self.device_type, self.version)
    }
}

/// UPnP service type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpnpServiceType {
    /// Domain
    pub domain: String,
    /// Service type
    pub service_type: String,
    /// Version
    pub version: u8,
}

impl UpnpServiceType {
    /// Create a new UPnP service type
    pub fn new(service_type: impl Into<String>, version: u8) -> Self {
        Self {
            domain: "schemas-upnp-org".to_string(),
            service_type: service_type.into(),
            version,
        }
    }

    /// Full service type string
    pub fn to_string(&self) -> String {
        format!("urn:{}:service:{}:{}", self.domain, self.service_type, self.version)
    }
}

/// UPnP device
#[derive(Debug, Clone)]
pub struct UpnpDevice {
    /// UDN (Unique Device Name)
    pub udn: String,
    /// Friendly name
    pub friendly_name: String,
    /// Device type
    pub device_type: Option<UpnpDeviceType>,
    /// Manufacturer
    pub manufacturer: String,
    /// Model name
    pub model_name: String,
    /// Model number
    pub model_number: String,
    /// Serial number
    pub serial_number: String,
    /// Location (description URL)
    pub location: String,
    /// Services
    pub services: Vec<UpnpServiceType>,
    /// Embedded devices
    pub devices: Vec<UpnpDevice>,
}

impl UpnpDevice {
    /// Create a new UPnP device
    pub fn new(udn: impl Into<String>, friendly_name: impl Into<String>) -> Self {
        Self {
            udn: udn.into(),
            friendly_name: friendly_name.into(),
            device_type: None,
            manufacturer: String::new(),
            model_name: String::new(),
            model_number: String::new(),
            serial_number: String::new(),
            location: String::new(),
            services: Vec::new(),
            devices: Vec::new(),
        }
    }

    /// Add service
    pub fn add_service(&mut self, service: UpnpServiceType) {
        self.services.push(service);
    }

    /// Add embedded device
    pub fn add_device(&mut self, device: UpnpDevice) {
        self.devices.push(device);
    }
}

/// UPnP control point
pub struct UpnpControlPoint {
    /// Discovered devices
    devices: BTreeMap<String, UpnpDevice>,
}

impl UpnpControlPoint {
    /// Create a new UPnP control point
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
        }
    }

    /// Discover devices
    pub fn discover(&mut self) -> IotResult<Vec<UpnpDevice>> {
        crate::log_info!("UPnP discovering devices");

        // In a real implementation, this would:
        // 1. Send SSDP M-SEARCH
        // 2. Parse device descriptions
        // 3. Cache device information

        Ok(self.devices.values().cloned().collect())
    }

    /// Get device by UDN
    pub fn get_device(&self, udn: &str) -> Option<&UpnpDevice> {
        self.devices.get(udn)
    }

    /// Add discovered device
    pub fn add_device(&mut self, device: UpnpDevice) {
        let udn = device.udn.clone();
        self.devices.insert(udn, device);
    }

    /// Remove device
    pub fn remove_device(&mut self, udn: &str) {
        self.devices.remove(udn);
    }

    /// Invoke action on service
    pub fn invoke_action(
        &self,
        _device: &UpnpDevice,
        _service: &UpnpServiceType,
        _action: &str,
        _arguments: &BTreeMap<String, String>,
    ) -> IotResult<BTreeMap<String, String>> {
        // In a real implementation, this would:
        // 1. Send SOAP action request
        // 2. Parse response
        // 3. Return result

        Ok(BTreeMap::new())
    }
}

impl Default for UpnpControlPoint {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ZeroConf Networking
// =============================================================================

/// ZeroConf service
#[derive(Debug, Clone)]
pub struct ZeroConfService {
    /// Service name
    pub name: String,
    /// Service type
    pub service_type: String,
    /// Domain
    pub domain: String,
    /// Hostname
    pub hostname: String,
    /// Port
    pub port: u16,
    /// TXT records
    pub txt: BTreeMap<String, String>,
}

impl ZeroConfService {
    /// Create a new ZeroConf service
    pub fn new(
        name: impl Into<String>,
        service_type: impl Into<String>,
        port: u16,
    ) -> Self {
        Self {
            name: name.into(),
            service_type: service_type.into(),
            domain: "local".to_string(),
            hostname: String::new(),
            port,
            txt: BTreeMap::new(),
        }
    }

    /// Full service name
    pub fn full_name(&self) -> String {
        format!("{}.{}.{}", self.name, self.service_type, self.domain)
    }
}

/// ZeroConf client
pub struct ZeroConfClient {
    /// mDNS responder
    mdns_responder: Option<MdnsResponder>,
    /// mDNS browser
    mdns_browser: Option<MdnsBrowser>,
    /// SSDP server
    ssdp_server: Option<SsdpServer>,
    /// SSDP client
    ssdp_client: Option<SsdpClient>,
    /// UPnP control point
    upnp_control_point: Option<UpnpControlPoint>,
}

impl ZeroConfClient {
    /// Create a new ZeroConf client
    pub fn new() -> Self {
        Self {
            mdns_responder: None,
            mdns_browser: None,
            ssdp_server: None,
            ssdp_client: None,
            upnp_control_point: None,
        }
    }

    /// Enable mDNS
    pub fn enable_mdns(&mut self, hostname: impl Into<String>) {
        self.mdns_responder = Some(MdnsResponder::new(hostname));
        self.mdns_browser = Some(MdnsBrowser::new());
    }

    /// Enable SSDP
    pub fn enable_ssdp(&mut self, uuid: impl Into<String>) {
        self.ssdp_server = Some(SsdpServer::new(uuid));
        self.ssdp_client = Some(SsdpClient::new());
    }

    /// Enable UPnP
    pub fn enable_upnp(&mut self) {
        self.upnp_control_point = Some(UpnpControlPoint::new());
    }

    /// Register service
    pub fn register_service(&mut self, service: ZeroConfService) -> IotResult<()> {
        if let Some(ref mut responder) = self.mdns_responder {
            let mdns_service = MdnsServiceRecord::new(
                service.name,
                service.service_type,
                service.hostname,
                service.port,
            );
            responder.register_service(mdns_service)?;
            Ok(())
        } else {
            Err(IotError::NotSupported("mDNS not enabled".to_string()))
        }
    }

    /// Browse for services
    pub fn browse(&mut self, service_type: impl Into<String>) -> IotResult<Vec<ZeroConfService>> {
        if let Some(ref mut browser) = self.mdns_browser {
            let mdns_services = browser.browse(service_type)?;
            let services = mdns_services
                .into_iter()
                .map(|s| ZeroConfService {
                    name: s.instance,
                    service_type: s.service_type,
                    domain: s.domain,
                    hostname: s.hostname,
                    port: s.port,
                    txt: s.txt,
                })
                .collect();
            Ok(services)
        } else {
            Err(IotError::NotSupported("mDNS not enabled".to_string()))
        }
    }
}

impl Default for ZeroConfClient {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Unified Device Discovery
// =============================================================================

/// Device discovery manager (unified interface)
pub struct DeviceDiscovery {
    /// mDNS browser
    mdns_browser: MdnsBrowser,
    /// SSDP client
    ssdp_client: SsdpClient,
    /// UPnP control point
    upnp_control_point: UpnpControlPoint,
    /// ZeroConf client
    zeroconf: ZeroConfClient,
    /// Discovered devices
    discovered_devices: BTreeMap<String, DeviceInfo>,
    /// Discovery callbacks
    callbacks: BTreeMap<String, Box<dyn Fn(&DeviceInfo) -> IotResult<()>>>,
}

impl DeviceDiscovery {
    /// Create a new device discovery manager
    pub fn new() -> Self {
        Self {
            mdns_browser: MdnsBrowser::new(),
            ssdp_client: SsdpClient::new(),
            upnp_control_point: UpnpControlPoint::new(),
            zeroconf: ZeroConfClient::new(),
            discovered_devices: BTreeMap::new(),
            callbacks: BTreeMap::new(),
        }
    }

    /// Start discovery
    pub fn start(&mut self) -> IotResult<()> {
        crate::log_info!("Device discovery started");
        Ok(())
    }

    /// Stop discovery
    pub fn stop(&mut self) -> IotResult<()> {
        crate::log_info!("Device discovery stopped");
        Ok(())
    }

    /// Discover mDNS devices
    pub fn discover_mdns_devices(&mut self, service_type: &str) -> IotResult<Vec<DeviceInfo>> {
        let services = self.mdns_browser.browse(service_type)?;

        let devices: Vec<DeviceInfo> = services
            .into_iter()
            .map(|s| {
                let device_id = DeviceId::new("mdns", &s.hostname);
                DeviceInfo {
                    id: device_id,
                    name: s.instance,
                    firmware_version: String::new(),
                    capabilities: Vec::new(),
                    protocol: s.service_type,
                    address: format!("{}:{}", s.hostname, s.port),
                    last_seen: 0,
                    battery_level: None,
                    signal_strength: None,
                }
            })
            .collect();

        for device in &devices {
            self.discovered_devices.insert(device.id.id.clone(), device.clone());
        }

        Ok(devices)
    }

    /// Discover SSDP devices
    pub fn discover_ssdp_devices(&mut self) -> IotResult<Vec<DeviceInfo>> {
        let devices = self.ssdp_client.search(SsdpSearchTarget::All)?;

        let device_infos: Vec<DeviceInfo> = devices
            .into_iter()
            .map(|d| {
                let device_id = DeviceId::new("ssdp", &d.usn);
                DeviceInfo {
                    id: device_id,
                    name: d.server.clone(),
                    firmware_version: String::new(),
                    capabilities: Vec::new(),
                    protocol: "ssdp".to_string(),
                    address: d.location,
                    last_seen: 0,
                    battery_level: None,
                    signal_strength: None,
                }
            })
            .collect();

        for device in &device_infos {
            self.discovered_devices.insert(device.id.id.clone(), device.clone());
        }

        Ok(device_infos)
    }

    /// Discover UPnP devices
    pub fn discover_upnp_devices(&mut self) -> IotResult<Vec<DeviceInfo>> {
        let devices = self.upnp_control_point.discover()?;

        let device_infos: Vec<DeviceInfo> = devices
            .into_iter()
            .map(|d| {
                let device_id = DeviceId::new("upnp", &d.udn);
                DeviceInfo {
                    id: device_id,
                    name: d.friendly_name.clone(),
                    firmware_version: d.model_number.clone(),
                    capabilities: Vec::new(),
                    protocol: "upnp".to_string(),
                    address: d.location,
                    last_seen: 0,
                    battery_level: None,
                    signal_strength: None,
                }
            })
            .collect();

        for device in &device_infos {
            self.discovered_devices.insert(device.id.id.clone(), device.clone());
        }

        Ok(device_infos)
    }

    /// Get all discovered devices
    pub fn get_discovered_devices(&self) -> Vec<DeviceInfo> {
        self.discovered_devices.values().cloned().collect()
    }

    /// Get device by ID
    pub fn get_device(&self, device_id: &str) -> Option<DeviceInfo> {
        self.discovered_devices.get(device_id).cloned()
    }

    /// Register discovery callback
    pub fn register_callback<F>(&mut self, name: impl Into<String>, callback: F)
    where
        F: Fn(&DeviceInfo) -> IotResult<()> + 'static,
    {
        self.callbacks.insert(name.into(), Box::new(callback));
    }

    /// Trigger callbacks for device
    fn trigger_callbacks(&self, device: &DeviceInfo) -> IotResult<()> {
        for callback in self.callbacks.values() {
            callback(device)?;
        }
        Ok(())
    }
}

impl Default for DeviceDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdns_service_record() {
        let service = MdnsServiceRecord::new("MyMQTT", "_mqtt._tcp", "mymqtt.local", 1883);
        assert_eq!(service.instance, "MyMQTT");
        assert_eq!(service.port, 1883);
        assert_eq!(service.domain, "local");

        service.add_txt("version", "1.0");
        assert_eq!(service.get_txt("version"), Some(&String::from("1.0")));
    }

    #[test]
    fn test_mdns_responder() {
        let mut responder = MdnsResponder::new("mydevice.local");

        let service = MdnsServiceRecord::new("TestService", "_http._tcp", "test.local", 8080);
        responder.register_service(service).unwrap();

        assert_eq!(responder.services().len(), 1);
    }

    #[test]
    fn test_mdns_browser() {
        let mut browser = MdnsBrowser::new();

        let service = MdnsServiceRecord::new("TestService", "_http._tcp", "test.local", 8080);
        browser.add_service(service);

        let services = browser.browse("_http._tcp").unwrap();
        assert_eq!(services.len(), 1);
    }

    #[test]
    fn test_dns_sd_service_type() {
        let service_type = DnsSdServiceType::new("mqtt", "tcp");
        assert_eq!(service_type.to_string(), "_mqtt._tcp");
    }

    #[test]
    fn test_dns_sd_instance() {
        let service_type = DnsSdServiceType::new("http", "tcp");
        let instance = DnsSdInstance::new("Web Server", service_type, "web.local", 80);

        instance.add_txt("path", "/");
        instance.add_ipv4([192, 168, 1, 100]);

        assert_eq!(instance.port, 80);
        assert_eq!(instance.txt.len(), 1);
        assert_eq!(instance.ipv4.len(), 1);
    }

    #[test]
    fn test_ssdp_message() {
        let search = SsdpMessage::new_search(SsdpSearchTarget::All, 3);
        assert_eq!(search.message_type, "M-SEARCH");
        assert_eq!(search.cache_control, 3);

        let notify = SsdpMessage::new_notify(
            SsdpNotifyType::Alive,
            SsdpSearchTarget::RootDevice,
            "http://192.168.1.1:5000/desc.xml",
            "uuid:12345678-1234-1234-1234-123456789abc::upnp:rootdevice",
        );
        assert_eq!(notify.message_type, "NOTIFY");
        assert_eq!(notify.notify_type, Some(SsdpNotifyType::Alive));
    }

    #[test]
    fn test_ssdp_server() {
        let mut server = SsdpServer::new("uuid:12345678");

        let device = SsdpMessage::new_notify(
            SsdpNotifyType::Alive,
            SsdpSearchTarget::RootDevice,
            "http://192.168.1.1:5000/desc.xml",
            "uuid:12345678::upnp:rootdevice",
        );

        server.register_device(device).unwrap();
        server.start().unwrap();

        assert!(server.listening);
        assert_eq!(server.devices.len(), 1);
    }

    #[test]
    fn test_upnp_device_type() {
        let device_type = UpnpDeviceType::new("MediaServer", 1);
        assert!(device_type.to_string().contains("MediaServer"));
        assert!(device_type.to_string().contains("urn:"));
    }

    #[test]
    fn test_upnp_service_type() {
        let service_type = UpnpServiceType::new("ContentDirectory", 1);
        assert!(service_type.to_string().contains("ContentDirectory"));
    }

    #[test]
    fn test_upnp_device() {
        let mut device = UpnpDevice::new("uuid:12345678", "My Media Server");

        device.manufacturer = "Acme Inc".to_string();
        device.model_name = "MediaServer 3000".to_string();

        let service_type = UpnpServiceType::new("ContentDirectory", 1);
        device.add_service(service_type);

        assert_eq!(device.services.len(), 1);
    }

    #[test]
    fn test_zeroconf_service() {
        let service = ZeroConfService::new("My Service", "_http._tcp", 8080);

        service.add_txt("path", "/api");
        assert_eq!(service.port, 8080);
        assert_eq!(service.txt.len(), 1);
    }

    #[test]
    fn test_device_discovery() {
        let mut discovery = DeviceDiscovery::new();
        discovery.start().unwrap();

        // Test that discovery is running
        let devices = discovery.get_discovered_devices();
        assert_eq!(devices.len(), 0); // No devices discovered yet
    }

    #[test]
    fn test_device_id() {
        let id = DeviceId::new("mdns", "mydevice.local");
        assert_eq!(id.device_type, "mdns");
        assert_eq!(id.id, "mydevice.local");
    }
}
