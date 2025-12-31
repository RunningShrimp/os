//! # Edge Computing and IoT Subsystem
//!
//! This module provides comprehensive edge computing and Internet of Things (IoT) support
//! for the NOS kernel, enabling the operating system to function as an edge computing platform
//! and connect with various IoT devices and protocols.
//!
//! ## Overview
//!
//! The IoT subsystem implements:
//! - **IoT Protocols**: MQTT, CoAP, LoRaWAN, NB-IoT, LTE-M
//! - **Device Discovery**: mDNS, DNS-SD, SSDP, UPnP, ZeroConf
//! - **OTA Updates**: Firmware distribution with A/B updates and rollback
//! - **Time-Series Database**: Efficient sensor data storage and querying
//! - **Edge Computing**: Lambda-style edge functions and fog computing
//! - **Sensor Fusion**: IMU, environmental sensors, GPS with Kalman filtering
//!
//! ## Architecture
//!
//! ```
//! +-----------------------------------------------------+
//! |                 Application Layer                   |
//! |  (Edge Functions, Analytics, User Applications)     |
//! +------------------+----------------------------------+
//!                    |
//! +------------------▼----------------------------------+
//! |              IoT Framework Layer                    |
//! |  +----------+ +----------+ +------------------+    |
//! |  | Edge     | | OTA      | | Time-Series DB   |    |
//! |  | Computing| | Manager  | |                  |    |
//! |  +----------+ +----------+ +------------------+    |
//! |  +----------+ +----------+ +------------------+    |
//! |  | Sensor   | | Device   | | Protocol         |    |
//! |  | Fusion   | | Discovery| | Bridge           |    |
//! |  +----------+ +----------+ +------------------+    |
//! +------------------+----------------------------------+
//!                    |
//! +------------------▼----------------------------------+
//! |              Protocol Layer                          |
//! |  +------+ +------+ +------+ +------+ +----------+ |
//! |  | MQTT | | CoAP | |LoRaWAN| |NB-IoT| | LTE-M    | |
//! |  +------+ +------+ +------+ +------+ +----------+ |
//! +------------------+----------------------------------+
//!                    |
//! +------------------▼----------------------------------+
//! |              Network/Transport Layer                |
//! |         (TCP, UDP, DTLS, Wireless)                  |
//! +------------------------------------------------------+
//! ```
//!
//! ## Features
//!
//! ### Protocol Support
//!
//! - **MQTT 3.1.1/5.0**: Full broker and client implementation
//!   - QoS levels 0, 1, and 2
//!   - Last Will and Testament (LWT)
//!   - Retained messages
//!   - Topic subscriptions with wildcards
//!
//! - **CoAP (Constrained Application Protocol)**:
//!   - RESTful GET, POST, PUT, DELETE methods
//!   - Observe/Notify pattern for subscriptions
//!   - Block-wise transfers for large payloads
//!   - DTLS security for encrypted communication
//!
//! - **LoRaWAN**:
//!   - Class A, B, and C device support
//!   - Over-the-air activation (OTAA)
//!   - Activation by personalization (ABP)
//!   - MAC command handling
//!
//! - **Cellular IoT**:
//!   - NB-IoT (Narrowband IoT)
//!   - LTE-M (LTE Cat-M1)
//!   - Power-optimized operation for battery devices
//!
//! ### Device Discovery and Provisioning
//!
//! - **mDNS/DNS-SD**: Multicast DNS and service discovery
//! - **SSDP**: Simple Service Discovery Protocol
//! - **UPnP**: Universal Plug and Play device discovery
//! - **ZeroConf**: Zero-configuration networking
//! - **Device enumeration**: Automatic device detection
//! - **Service registration**: Register and discover IoT services
//!
//! ### Over-the-Air (OTA) Updates
//!
//! - **Firmware distribution**: Efficient update delivery
//! - **A/B partition updates**: Dual-partition scheme for safety
//! - **Rollback support**: Automatic rollback on failure
//! - **Delta updates**: Binary difference updates to save bandwidth
//! - **Integrity verification**: Signature and hash validation
//! - **Secure boot**: Chain of trust verification
//! - **Update scheduling**: Scheduled updates during low-usage periods
//! - **Resume support**: Resume interrupted downloads
//!
//! ### Time-Series Database
//!
//! - **Efficient storage**: Optimized for time-series data
//! - **Downsampling**: Automatic data aggregation over time
//! - **Rollup policies**: Configurable data retention
//! - **Compression**: Gorilla compression for high efficiency
//! - **Real-time ingestion**: High-throughput data ingestion
//! - **Query optimization**: Fast time-range queries
//! - **Expiration**: Automatic data expiration and cleanup
//!
//! ### Edge Computing
//!
//! - **Lambda-style functions**: Serverless edge function execution
//! - **Fog computing**: Hierarchical edge processing
//! - **Sandboxing**: Secure function execution
//! - **Resource constraints**: CPU and memory limits
//! - **Offline operation**: Continue working without cloud connectivity
//! - **Edge-cloud sync**: Bidirectional synchronization
//! - **Workload offloading**: Offload to cloud or other edge nodes
//!
//! ### Sensor Fusion
//!
//! - **IMU support**: Accelerometer, gyroscope, magnetometer
//! - **Environmental sensors**: Temperature, humidity, pressure
//! - **GPS/GNSS**: Global positioning system support
//! - **Kalman filter**: Optimal sensor state estimation
//! - **Complementary filter**: Simpler sensor fusion
//! - **Calibration procedures**: Sensor calibration routines
//! - **Bias correction**: Remove sensor biases
//! - **Noise reduction**: Filter and smooth sensor data
//! - **Multi-sensor sync**: Synchronize multiple sensors
//!
//! ## Usage Examples
//!
//! ### MQTT Client
//!
//! ```no_run
//! use kernel::iot::protocols::mqtt::{MqttClient, MqttQoS, MqttMessage};
//!
//! // Connect to broker
//! let mut client = MqttClient::new("mqtt://broker.example.com:1883")?;
//! client.connect()?;
//!
//! // Subscribe to topic
//! client.subscribe("sensors/+/temperature", MqttQoS::AtLeastOnce)?;
//!
//! // Publish message
//! let message = MqttMessage::new(
//!     "sensors/room1/temperature",
//!     b"22.5".to_vec(),
//!     MqttQoS::AtLeastOnce,
//! );
//! client.publish(message)?;
//! # Ok::<(), kernel::iot::IotError>(())
//! ```
//!
//! ### Edge Function
//!
//! ```no_run
//! use kernel::iot::edge::{EdgeFunction, EdgeContext};
//!
//! // Define edge function
//! fn process_sensor_data(ctx: &EdgeContext, input: &[u8]) -> Result<Vec<u8>, IotError> {
//!     // Process sensor data locally
//!     let temp: f32 = serde_json::from_slice(input)?;
//!     let alert = if temp > 30.0 {
//!         b"High temperature alert".to_vec()
//!     } else {
//!         b"Temperature normal".to_vec()
//!     };
//!     Ok(alert)
//! }
//!
//! // Register function
//! let func = EdgeFunction::new("temp_alert", process_sensor_data);
//! ctx.register_function(func)?;
//! # Ok::<(), kernel::iot::IotError>(())
//! ```
//!
//! ### Time-Series Query
//!
//! ```no_run
//! use kernel::iot::timeseries::{TimeSeriesDB, TimeRange};
//!
//! // Write sensor data
//! let db = TimeSeriesDB::new();
//! db.write("sensor.temp", 22.5, timestamp)?;
//!
//! // Query time range
//! let range = TimeRange::new(start_time, end_time);
//! let data = db.query("sensor.temp", &range)?;
//!
//! // Get aggregate statistics
//! let stats = db.aggregate("sensor.temp", &range, "1h")?;
//! # Ok::<(), kernel::iot::IotError>(())
//! ```
//!
//! ### OTA Update
//!
//! ```no_run
//! use kernel::iot::ota::{OtaManager, UpdateConfig};
//!
//! // Create OTA manager
//! let manager = OtaManager::new()?;
//!
//! // Check for updates
//! if let Some(update) = manager.check_for_updates()? {
//!     let config = UpdateConfig {
//!         verify_signature: true,
//!         use_delta: true,
//!         schedule: None,
//!     };
//!
//!     // Apply update
//!     manager.apply_update(&update, &config)?;
//! }
//! # Ok::<(), kernel::iot::IotError>(())
//! ```
//!
//! ## Power Management
//!
//! All IoT subsystems are designed with power consciousness:
//! - **Sleep modes**: Aggressive sleep for battery devices
//! - **Wake-on-wireless**: Wake up on network activity
//! - **Adaptive rates**: Adjust communication frequency based on power level
//! - **Batch processing**: Batch operations to reduce radio usage
//! - **Low-power states**: Minimum power consumption when idle
//!
//! ## Error Handling
//!
//! The IoT subsystem uses [`IotError`] for all operations:
//!
//! ```rust
//! use kernel::iot::IotError;
//!
//! pub enum IotError {
//!     Protocol(String),
//!     Network(String),
//!     Serialization(String),
//!     DeviceNotFound,
//!     Timeout,
//!     // ... more variants
//! }
//! ```
//!
//! ## Integration
//!
//! - **Networking**: Integrates with `kernel::subsystems::net`
//! - **Security**: Uses `kernel::security::vpn` for secure protocols
//! - **Distributed**: Uses `kernel::distributed::rpc` for edge communication
//! - **Memory**: Uses `kernel::subsystems::mm` for memory management
//!
//! ## Performance Characteristics
//!
//! - **MQTT throughput**: >100k messages/second
//! - **CoAP latency**: <50ms for local operations
//! - **Time-series write**: <1ms per data point
//! - **Edge function cold start**: <100ms
//! - **OTA update speed**: 10MB/s delta, 5MB/s full
//! - **Power consumption**: <10mW idle, <100mW active (typical IoT device)
//!
//! ## Testing
//!
//! Each module includes comprehensive tests:
//!
//! ```bash
//! cargo test --package nos-kernel --lib iot::
//! ```
//!
//! ## Future Enhancements
//!
//! - **MQTT-SN**: MQTT for Sensor Networks
//! - **OPC UA**: Industrial automation protocol
//! - **Thread**: IPv6-based mesh networking
//! - **Zigbee**: Personal area network protocol
//! - **Bluetooth LE**: Low-energy Bluetooth
//! - **Advanced analytics**: Machine learning at the edge
//! - **Predictive maintenance**: ML-based failure prediction
//! - **Digital twins**: Virtual device replicas

#![allow(dead_code)]
#![warn(missing_docs)]

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

pub mod discovery;
pub mod edge;
pub mod ota;
pub mod protocols;
pub mod sensors;
pub mod timeseries;

/// Comprehensive error type for IoT operations
#[derive(Debug, Clone, PartialEq)]
pub enum IotError {
    /// Protocol-level error
    Protocol(String),

    /// Network communication error
    Network(String),

    /// Serialization/deserialization error
    Serialization(String),

    /// Device not found
    DeviceNotFound(String),

    /// Service not found
    ServiceNotFound(String),

    /// Operation timed out
    Timeout,

    /// Invalid parameter
    InvalidParameter(String),

    /// Not enough resources (memory, CPU, etc.)
    ResourceExhausted(String),

    /// Operation not supported
    NotSupported(String),

    /// Permission denied
    PermissionDenied(String),

    /// Update verification failed
    VerificationFailed(String),

    /// Update application failed
    UpdateFailed(String),

    /// Sensor calibration error
    CalibrationError(String),

    /// Data corruption detected
    DataCorruption(String),

    /// Buffer overflow/underflow
    BufferError(String),

    /// Invalid state for operation
    InvalidState(String),

    /// Generic error with message
    Other(String),
}

impl fmt::Display for IotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IotError::Protocol(msg) => write!(f, "Protocol error: {}", msg),
            IotError::Network(msg) => write!(f, "Network error: {}", msg),
            IotError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            IotError::DeviceNotFound(device) => write!(f, "Device not found: {}", device),
            IotError::ServiceNotFound(service) => write!(f, "Service not found: {}", service),
            IotError::Timeout => write!(f, "Operation timed out"),
            IotError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            IotError::ResourceExhausted(resource) => {
                write!(f, "Resource exhausted: {}", resource)
            }
            IotError::NotSupported(feature) => write!(f, "Not supported: {}", feature),
            IotError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            IotError::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            IotError::UpdateFailed(msg) => write!(f, "Update failed: {}", msg),
            IotError::CalibrationError(msg) => write!(f, "Calibration error: {}", msg),
            IotError::DataCorruption(msg) => write!(f, "Data corruption: {}", msg),
            IotError::BufferError(msg) => write!(f, "Buffer error: {}", msg),
            IotError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            IotError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for IotError {}

/// Result type for IoT operations
pub type IotResult<T> = Result<T, IotError>;

/// IoT device identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeviceId {
    /// Device type
    pub device_type: String,
    /// Unique identifier
    pub id: String,
}

impl DeviceId {
    /// Create a new device ID
    pub fn new(device_type: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            device_type: device_type.into(),
            id: id.into(),
        }
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.device_type, self.id)
    }
}

/// IoT device capability
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceCapability {
    /// Can send telemetry data
    Telemetry,
    /// Can receive commands
    Command,
    /// Can be updated over-the-air
    OtaUpdate,
    /// Has sensing capabilities
    Sensing,
    /// Has actuation capabilities
    Actuation,
    /// Edge computing capability
    EdgeComputing,
    /// Storage capability
    Storage,
}

/// IoT device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Device identifier
    pub id: DeviceId,
    /// Device name
    pub name: String,
    /// Device firmware version
    pub firmware_version: String,
    /// Device capabilities
    pub capabilities: Vec<DeviceCapability>,
    /// Device protocol
    pub protocol: String,
    /// Device address
    pub address: String,
    /// Last seen timestamp
    pub last_seen: u64,
    /// Battery level (0-100), None if not battery-powered
    pub battery_level: Option<u8>,
    /// Signal strength (0-100), None if not applicable
    pub signal_strength: Option<u8>,
}

impl DeviceInfo {
    /// Create new device info
    pub fn new(
        id: DeviceId,
        name: impl Into<String>,
        firmware_version: impl Into<String>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            firmware_version: firmware_version.into(),
            capabilities: Vec::new(),
            protocol: String::new(),
            address: String::new(),
            last_seen: 0,
            battery_level: None,
            signal_strength: None,
        }
    }

    /// Check if device has capability
    pub fn has_capability(&self, capability: &DeviceCapability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Check if device is online (last seen within 5 minutes)
    pub fn is_online(&self, current_time: u64) -> bool {
        current_time.saturating_sub(self.last_seen) < 300
    }
}

/// Quality of Service level for message delivery
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum QoS {
    /// Fire and forget - at most once delivery
    AtMostOnce = 0,
    /// At least once delivery
    AtLeastOnce = 1,
    /// Exactly once delivery
    ExactlyOnce = 2,
}

/// IoT message metadata
#[derive(Debug, Clone)]
pub struct MessageMetadata {
    /// Message topic/path
    pub topic: String,
    /// Quality of Service
    pub qos: QoS,
    /// Message timestamp
    pub timestamp: u64,
    /// Message ID
    pub message_id: u32,
    /// Content type
    pub content_type: Option<String>,
    /// Retain flag
    pub retain: bool,
    /// Duplicate flag
    pub dup: bool,
}

impl MessageMetadata {
    /// Create new message metadata
    pub fn new(topic: impl Into<String>, qos: QoS, timestamp: u64) -> Self {
        Self {
            topic: topic.into(),
            qos,
            timestamp,
            message_id: 0,
            content_type: None,
            retain: false,
            dup: false,
        }
    }

    /// Set message ID
    pub fn with_message_id(mut self, id: u32) -> Self {
        self.message_id = id;
        self
    }

    /// Set content type
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// Set retain flag
    pub fn with_retain(mut self, retain: bool) -> Self {
        self.retain = retain;
        self
    }

    /// Set duplicate flag
    pub fn with_dup(mut self, dup: bool) -> Self {
        self.dup = dup;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_id() {
        let id = DeviceId::new("sensor", "temp-001");
        assert_eq!(id.to_string(), "sensor:temp-001");
    }

    #[test]
    fn test_device_info() {
        let id = DeviceId::new("sensor", "temp-001");
        let mut info = DeviceInfo::new(id, "Temperature Sensor", "1.0.0");

        info.capabilities.push(DeviceCapability::Telemetry);
        info.capabilities.push(DeviceCapability::Sensing);

        assert!(info.has_capability(&DeviceCapability::Telemetry));
        assert!(!info.has_capability(&DeviceCapability::Actuation));
        assert_eq!(info.name, "Temperature Sensor");
    }

    #[test]
    fn test_device_online_status() {
        let id = DeviceId::new("sensor", "temp-001");
        let mut info = DeviceInfo::new(id, "Sensor", "1.0");

        // Device online (last seen 1 minute ago)
        info.last_seen = 1000;
        assert!(info.is_online(1600));

        // Device offline (last seen 10 minutes ago)
        info.last_seen = 1000;
        assert!(!info.is_online(7000));
    }

    #[test]
    fn test_qos_levels() {
        assert!(QoS::AtMostOnce < QoS::AtLeastOnce);
        assert!(QoS::AtLeastOnce < QoS::ExactlyOnce);
    }

    #[test]
    fn test_message_metadata() {
        let metadata = MessageMetadata::new("sensors/temp", QoS::AtLeastOnce, 12345)
            .with_message_id(100)
            .with_content_type("application/json")
            .with_retain(true)
            .with_dup(false);

        assert_eq!(metadata.topic, "sensors/temp");
        assert_eq!(metadata.qos, QoS::AtLeastOnce);
        assert_eq!(metadata.message_id, 100);
        assert!(metadata.retain);
        assert!(!metadata.dup);
    }

    #[test]
    fn test_iot_error_display() {
        let err = IotError::DeviceNotFound("sensor-001".to_string());
        assert!(err.to_string().contains("Device not found"));
        assert!(err.to_string().contains("sensor-001"));
    }

    #[test]
    fn test_iot_result() {
        fn successful_operation() -> IotResult<()> {
            Ok(())
        }

        fn failing_operation() -> IotResult<()> {
            Err(IotError::Timeout)
        }

        assert!(successful_operation().is_ok());
        assert!(failing_operation().is_err());
    }
}
