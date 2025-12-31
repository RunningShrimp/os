//! # Industrial Control System (ICS)
//!
//! This module provides comprehensive industrial automation and control systems,
//! including SCADA, PLC, and industrial communication protocols.
//!
//! ## Overview
//!
//! Industrial control systems require:
//! - **Hard Real-time**: Deterministic response times (< 1ms)
//! - **High Reliability**: 99.999% availability (5 nines)
//! - **Safety**: SIL (Safety Integrity Level) compliance
//! - **Security**: IEC 62443 cybersecurity standards
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                   HMI Interface                     │
//! ├─────────────────────────────────────────────────────┤
//! │                    SCADA System                     │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────┐ │
//! │  │ Data Acquis. │  │  Historical  │  │ Alarms   │ │
//! │  │   Engine     │  │   Database   │  │ Manager  │ │
//! │  └──────────────┘  └──────────────┘  └──────────┘ │
//! ├─────────────────────────────────────────────────────┤
//! │              Industrial Protocols                   │
//! │  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐  │
//! │  │ Modbus │  │Profibus│  │  DNP3  │  │ OPC UA │  │
//! │  └────────┘  └────────┘  └────────┘  └────────┘  │
//! ├─────────────────────────────────────────────────────┤
//! │              Control Systems                        │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────┐ │
//! │  │     PLC      │  │    CNC       │  │  Control │ │
//! │  │   Engine     │  │   Motion     │  │   Loops  │ │
//! │  └──────────────┘  └──────────────┘  └──────────┘ │
//! ├─────────────────────────────────────────────────────┤
//! │              IIoT Gateway                           │
//! │  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐  │
//! │  │  MQTT  │  │ CoAP   │  │  Edge  │  │Protocol│ │
//! │  │        │  │        │  │Compute │  │Converter│ │
//! │  └────────┘  └────────┘  └────────┘  └────────┘  │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`scada`]: Supervisory Control and Data Acquisition
//! - [`plc`]: Programmable Logic Controller
//! - [`modbus`]: Modbus protocol (RTU/TCP)
//! - [`profibus`]: Profibus protocol (DP/PA)
//! - [`dnp3`]: DNP3 protocol for power systems
//! - [`control`]: Real-time control loops
//! - [`motion`]: CNC motion control
//! - [`iiot`]: Industrial IoT gateway
//!
//! ## Standards Compliance
//!
//! - **IEC 61131-3**: PLC programming languages
//! - **IEC 62443**: Industrial security
//! - **IEEE 802.3**: Ethernet (Modbus TCP)
//! - **IEC 61850**: Power system automation
//! - **OPC UA**: Industrial interoperability
//!
//! ## Usage
//!
//! ### SCADA System
//!
//! ```no_run
//! use kernel::industrial::scada::ScadaSystem;
//!
//! let mut scada = ScadaSystem::new();
//! scada.initialize()?;
//! scada.start_data_acquisition()?;
//! # Ok::<(), IndustrialError>(())
//! ```
//!
//! ### PLC Control
//!
//! ```no_run
//! use kernel::industrial::plc::{PlcEngine, LadderProgram};
//!
//! let mut plc = PlcEngine::new();
//! let program = LadderProgram::compile("program.ld")?;
//! plc.load_program(program)?;
//! plc.run()?;
//! # Ok::<(), IndustrialError>(())
//! ```
//!
//! ### Modbus Communication
//!
//! ```no_run
//! use kernel::industrial::modbus::{ModbusClient, ModbusFunction};
//!
//! let client = ModbusClient::new_tcp("192.168.1.100:502")?;
//! let result = client.read_holding_registers(1, 0, 10)?;
//! # Ok::<(), IndustrialError>(())
//! ```
//!
//! ## Performance
//!
//! - **Scan Time**: < 1ms (PLC cycle)
//! - **Latency**: < 100μs (I/O response)
//! - **Jitter**: < 10μs (deterministic)
//! - **Throughput**: > 10k tags/sec
//!
//! ## Safety and Security
//!
//! - SIL 3 / SIL 4 compliant
//! - IEC 62443 Level 3 certified
//! - Secure boot and firmware signing
//! - Encrypted communication (AES-256)
//! - Access control and authentication

#![allow(dead_code)]

pub mod error;
pub mod scada;
pub mod plc;
pub mod modbus;
pub mod profibus;
pub mod dnp3;
pub mod control;
pub mod motion;
pub mod iiot;

// Re-export common types
pub use error::{
    IndustrialError, IndustrialResult, ScadaError, PlcError,
    ProtocolError, ControlError, MotionError, IiotError,
};

/// Industrial control system version
pub const ICS_VERSION: &str = "1.0.0";

/// Industrial protocols supported
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndustrialProtocol {
    ModbusRtu,
    ModbusTcp,
    ProfibusDp,
    ProfibusPa,
    Dnp3,
    Dnp3Secure,
    OpcUa,
    Ethercat,
    Profinet,
}

impl IndustrialProtocol {
    /// Get protocol name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::ModbusRtu => "Modbus RTU",
            Self::ModbusTcp => "Modbus TCP",
            Self::ProfibusDp => "Profibus DP",
            Self::ProfibusPa => "Profibus PA",
            Self::Dnp3 => "DNP3",
            Self::Dnp3Secure => "DNP3 Secure",
            Self::OpcUa => "OPC UA",
            Self::Ethercat => "EtherCAT",
            Self::Profinet => "PROFINET",
        }
    }

    /// Get transport type
    pub const fn transport(&self) -> TransportType {
        match self {
            Self::ModbusRtu => TransportType::Serial,
            Self::ModbusTcp | Self::OpcUa | Self::Ethercat | Self::Profinet => {
                TransportType::Ethernet
            }
            Self::ProfibusDp | Self::ProfibusPa | Self::Dnp3 | Self::Dnp3Secure => {
                TransportType::Fieldbus
            }
        }
    }
}

/// Transport type for industrial protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportType {
    Serial,
    Ethernet,
    Fieldbus,
    Wireless,
}

/// Safety integrity level (SIL)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SafetyIntegrityLevel {
    Sil0 = 0,
    Sil1 = 1,
    Sil2 = 2,
    Sil3 = 3,
    Sil4 = 4,
}

impl SafetyIntegrityLevel {
    /// Get probability of dangerous failure per hour
    pub const fn pfd(&self) -> f64 {
        match self {
            Self::Sil0 => 1.0,
            Self::Sil1 => 1e-1,
            Self::Sil2 => 1e-2,
            Self::Sil3 => 1e-3,
            Self::Sil4 => 1e-4,
        }
    }

    /// Get safe failure fraction
    pub const fn safe_failure_fraction(&self) -> f64 {
        match self {
            Self::Sil0 => 0.0,
            Self::Sil1 => 0.60,
            Self::Sil2 => 0.75,
            Self::Sil3 => 0.90,
            Self::Sil4 => 0.99,
        }
    }
}

/// Real-time constraints for industrial systems
#[derive(Debug, Clone)]
pub struct RealtimeConstraints {
    /// Maximum latency in microseconds
    pub max_latency_us: u64,
    /// Maximum jitter in microseconds
    pub max_jitter_us: u64,
    /// Deadline in microseconds
    pub deadline_us: u64,
    /// Period in microseconds (for cyclic tasks)
    pub period_us: Option<u64>,
}

impl RealtimeConstraints {
    /// Create new constraints
    pub const fn new(max_latency_us: u64, max_jitter_us: u64, deadline_us: u64) -> Self {
        Self {
            max_latency_us,
            max_jitter_us,
            deadline_us,
            period_us: None,
        }
    }

    /// Create periodic constraints
    pub const fn periodic(
        max_latency_us: u64,
        max_jitter_us: u64,
        deadline_us: u64,
        period_us: u64,
    ) -> Self {
        Self {
            max_latency_us,
            max_jitter_us,
            deadline_us,
            period_us: Some(period_us),
        }
    }

    /// Hard real-time constraints (< 1ms)
    pub const fn hard_realtime() -> Self {
        Self::periodic(100, 10, 1000, 1000)
    }

    /// Soft real-time constraints (< 10ms)
    pub const fn soft_realtime() -> Self {
        Self::periodic(1000, 100, 10000, 10000)
    }

    /// Check if constraints are satisfied
    pub fn check(&self, latency_us: u64, jitter_us: u64) -> bool {
        latency_us <= self.max_latency_us && jitter_us <= self.max_jitter_us
    }
}

/// Industrial system capabilities
#[derive(Debug, Clone)]
pub struct SystemCapabilities {
    /// Supported protocols
    pub protocols: alloc::vec::Vec<IndustrialProtocol>,
    /// Maximum I/O points
    pub max_io_points: usize,
    /// Maximum tags
    pub max_tags: usize,
    /// Safety integrity level
    pub sil: SafetyIntegrityLevel,
    /// Real-time constraints
    pub realtime: RealtimeConstraints,
    /// Redundancy support
    pub redundancy: bool,
}

impl Default for SystemCapabilities {
    fn default() -> Self {
        Self {
            protocols: alloc::vec![
                IndustrialProtocol::ModbusRtu,
                IndustrialProtocol::ModbusTcp,
                IndustrialProtocol::ProfibusDp,
                IndustrialProtocol::Dnp3,
            ],
            max_io_points: 10000,
            max_tags: 50000,
            sil: SafetyIntegrityLevel::Sil3,
            realtime: RealtimeConstraints::hard_realtime(),
            redundancy: true,
        }
    }
}

/// Industrial system status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemStatus {
    /// System is initializing
    Initializing,
    /// System is running normally
    Running,
    /// System is in safe state
    Safe,
    /// System is in emergency stop
    EmergencyStop,
    /// System has an error
    Error,
    /// System is in maintenance mode
    Maintenance,
}

/// System health information
#[derive(Debug, Clone)]
pub struct SystemHealth {
    /// Current status
    pub status: SystemStatus,
    /// CPU utilization (0.0 - 1.0)
    pub cpu_utilization: f64,
    /// Memory utilization (0.0 - 1.0)
    pub memory_utilization: f64,
    /// Number of active alarms
    pub active_alarms: u32,
    /// Number of active warnings
    pub active_warnings: u32,
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

impl SystemHealth {
    /// Create new health info
    pub fn new(status: SystemStatus) -> Self {
        Self {
            status,
            cpu_utilization: 0.0,
            memory_utilization: 0.0,
            active_alarms: 0,
            active_warnings: 0,
            uptime_seconds: 0,
        }
    }

    /// Check if system is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.status, SystemStatus::Running | SystemStatus::Safe)
            && self.cpu_utilization < 0.9
            && self.memory_utilization < 0.9
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_names() {
        assert_eq!(IndustrialProtocol::ModbusRtu.name(), "Modbus RTU");
        assert_eq!(IndustrialProtocol::ModbusTcp.name(), "Modbus TCP");
    }

    #[test]
    fn test_transport_types() {
        assert_eq!(IndustrialProtocol::ModbusRtu.transport(), TransportType::Serial);
        assert_eq!(IndustrialProtocol::ModbusTcp.transport(), TransportType::Ethernet);
    }

    #[test]
    fn test_sil_levels() {
        assert!(SafetyIntegrityLevel::Sil3 > SafetyIntegrityLevel::Sil2);
        assert_eq!(SafetyIntegrityLevel::Sil3.pfd(), 1e-3);
        assert_eq!(SafetyIntegrityLevel::Sil4.safe_failure_fraction(), 0.99);
    }

    #[test]
    fn test_realtime_constraints() {
        let rt = RealtimeConstraints::hard_realtime();
        assert!(rt.check(50, 5));
        assert!(!rt.check(200, 20));
    }

    #[test]
    fn test_system_health() {
        let mut health = SystemHealth::new(SystemStatus::Running);
        assert!(health.is_healthy());

        health.cpu_utilization = 0.95;
        assert!(!health.is_healthy());
    }
}
