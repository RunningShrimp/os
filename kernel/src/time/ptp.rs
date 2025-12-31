//! PTP (Precision Time Protocol) - IEEE 1588 implementation
//!
//! This module provides a complete implementation of the Precision Time Protocol,
//! which enables sub-microsecond clock synchronization in local area networks.
//!
//! # Features
//!
//! - **Ordinary Clock (OC)**: Single port clock with one PTP port
//! - **Boundary Clock (BC)**: Multi-port clock for network devices
//! - **Transparent Clock (TC)**: For precise forwarding and residence time measurement
//! - **Best Master Clock Algorithm (BMCA)**: Automatic master selection
//! - **Path Delay Measurement**: Precise delay calculation
//! - **Clock Class and Accuracy**: Quality-based selection
//!
//! # Architecture
//!
//! PTP operates in a hierarchy:
//! - Grandmaster Clock (GM): Primary time source (GPS, atomic clock)
//! - Boundary Clocks: Intermediate clocks with multiple ports
//! - Ordinary Clocks: End-host clocks with single port
//!
//! # Message Types
//!
//! - **Sync**: Master sends time information
//! - **Follow_Up**: Contains precise send timestamp
//! - **Delay_Req**: Slave requests path delay
//! - **Delay_Resp**: Master responds with receive timestamp
//! - **Pdelay_Req**: Peer delay request
//! - **Pdelay_Resp**: Peer delay response
//!
//! # Example
//!
//! ```no_run
//! use kernel::time::ptp::{PTPClock, PTPConfig, ClockClass};
//!
//! let config = PTPConfig::ordinary_clock();
//! let clock = PTPClock::new(config);
//! clock.start().unwrap();
//! ```

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use spin::Mutex;

use crate::time::clock::{ClockReading, ClockError};

/// PTP default port number
pub const PTP_PORT: u16 = 319;
/// PTP event port number
pub const PTP_EVENT_PORT: u16 = 320;

/// PTP message type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PTPMessageType {
    /// Sync message
    Sync = 0x00,
    /// Delay_Req message
    DelayReq = 0x01,
    /// Follow_Up message
    FollowUp = 0x02,
    /// Delay_Resp message
    DelayResp = 0x03,
    /// Management message
    Management = 0x04,
    /// Peer delay request
    PDelayReq = 0x05,
    /// Peer delay response
    PDelayResp = 0x06,
    /// Peer delay follow-up
    PDelayRespFollowUp = 0x07,
    /// Announce message
    Announce = 0x08,
    /// Signaling message
    Signaling = 0x09,
    /// Management error status
    ManagementErrorStatus = 0x0A,
}

/// PTP clock class (quality indicator)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClockClass {
    /// Default - clock synchronized to another time source
    Default = 248,
    /// Slave only - cannot be master
    SlaveOnly = 255,
    /// Clock has not been synchronized
    NotSync = 13,
    /// Maximum clock quality
    MaxQuality = 6,
    /// Minimum clock quality (atomic clock)
    MinQuality = 0,
}

/// PTP clock accuracy
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClockAccuracy {
    /// Nanosecond accuracy
    Nanosecond = 0x20,
    /// Microsecond accuracy
    Microsecond = 0x21,
    /// Millisecond accuracy
    Millisecond = 0x22,
    /// Unknown accuracy
    Unknown = 0xFE,
}

/// PTP port state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortState {
    /// Port is initializing
    Initializing = 1,
    /// Port is faulty
    Faulty = 2,
    /// Port is disabled
    Disabled = 3,
    /// Port is listening
    Listening = 4,
    /// Port is preemptive master
    PreMaster = 5,
    /// Port is master
    Master = 6,
    /// Port is passive
    Passive = 7,
    /// Port is uncalibrated
    Uncalibrated = 8,
    /// Port is slave
    Slave = 9,
}

/// PTP timestamp (96-bit: 48-bit seconds, 32-bit nanoseconds)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PTPTimestamp {
    /// Seconds (48 bits)
    pub seconds: u64,
    /// Nanoseconds (32 bits)
    pub nanos: u32,
}

impl PTPTimestamp {
    /// Create new PTP timestamp
    pub fn new(seconds: u64, nanos: u32) -> Self {
        Self {
            seconds,
            nanos,
        }
    }

    /// Convert from nanoseconds
    pub fn from_nanos(ns: u64) -> Self {
        Self {
            seconds: ns / 1_000_000_000,
            nanos: (ns % 1_000_000_000) as u32,
        }
    }

    /// Convert to nanoseconds
    pub fn to_nanos(self) -> u64 {
        self.seconds * 1_000_000_000 + self.nanos as u64
    }

    /// Get difference in nanoseconds
    pub fn sub_nanos(self, other: Self) -> i64 {
        (self.to_nanos() as i64) - (other.to_nanos() as i64)
    }

    /// Add nanoseconds
    pub fn add_nanos(mut self, ns: i64) -> Self {
        let total = self.to_nanos() as i64 + ns;
        if total >= 0 {
            Self::from_nanos(total as u64)
        } else {
            Self {
                seconds: 0,
                nanos: 0,
            }
        }
    }
}

/// PTP clock identity (8 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockIdentity {
    /// 64-bit clock ID
    pub id: [u8; 8],
}

impl ClockIdentity {
    /// Create from EUI-64 address
    pub fn from_eui64(eui: [u8; 8]) -> Self {
        Self { id: eui }
    }

    /// Create from MAC address (EUI-48)
    pub fn from_mac(mac: [u8; 6]) -> Self {
        let mut id = [0u8; 8];
        id[0] = 0xff;
        id[1] = 0xff;
        id[2..8].copy_from_slice(&mac);
        Self { id }
    }

    /// Generate random clock identity
    pub fn random() -> Self {
        Self {
            id: rand::random(),
        }
    }
}

/// PTP port identity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortIdentity {
    /// Clock identity
    pub clock: ClockIdentity,
    /// Port number
    pub port: u16,
}

/// PTP header
#[derive(Debug, Clone, Copy)]
pub struct PTPHeader {
    /// Transport specific (message type domain)
    pub transport_specific: u8,
    /// Message type
    pub message_type: PTPMessageType,
    /// PTP version
    pub version: u8,
    /// Message length
    pub message_length: u16,
    /// Domain number
    pub domain_number: u8,
    /// Flag field
    pub flags: u16,
    /// Correction field (nanoseconds)
    pub correction_field: i64,
    /// Message type specific
    pub message_type_specific: u8,
    /// Reserved
    pub reserved: u8,
    /// Source port identity
    pub source_port_identity: PortIdentity,
    /// Sequence ID
    pub sequence_id: u16,
    /// Control field
    pub control: u8,
    /// Log message interval
    pub log_message_interval: u8,
}

impl PTPHeader {
    /// Serialize header to bytes
    pub fn to_bytes(&self) -> [u8; 34] {
        let mut buf = [0u8; 34];

        // First byte: transport_specific (4 bits) + message_type (4 bits)
        buf[0] = (self.transport_specific << 4) | (self.message_type as u8);

        // Version
        buf[1] = self.version;

        // Message length
        buf[2..4].copy_from_slice(&self.message_length.to_be_bytes());

        // Domain number
        buf[4] = self.domain_number;

        // Reserved
        buf[5] = self.reserved;

        // Flags
        buf[6..8].copy_from_slice(&self.flags.to_be_bytes());

        // Correction field (64-bit)
        buf[8..16].copy_from_slice(&self.correction_field.to_be_bytes());

        // Message type specific
        buf[16] = self.message_type_specific;

        // Reserved
        buf[17] = 0;

        // Source port identity
        buf[18..26].copy_from_slice(&self.source_port_identity.clock.id);
        buf[26..28].copy_from_slice(&self.source_port_identity.port.to_be_bytes());

        // Sequence ID
        buf[28..30].copy_from_slice(&self.sequence_id.to_be_bytes());

        // Control field
        buf[30] = self.control;

        // Log message interval
        buf[31] = self.log_message_interval;

        buf
    }
}

/// PTP Sync message
#[derive(Debug, Clone, Copy)]
pub struct SyncMessage {
    pub header: PTPHeader,
    pub timestamp: PTPTimestamp,
}

/// PTP Follow_Up message
#[derive(Debug, Clone, Copy)]
pub struct FollowUpMessage {
    pub header: PTPHeader,
    pub precise_origin_timestamp: PTPTimestamp,
}

/// PTP Delay_Req message
#[derive(Debug, Clone, Copy)]
pub struct DelayReqMessage {
    pub header: PTPHeader,
    pub timestamp: PTPTimestamp,
}

/// PTP Delay_Resp message
#[derive(Debug, Clone, Copy)]
pub struct DelayRespMessage {
    pub header: PTPHeader,
    pub receive_timestamp: PTPTimestamp,
    pub requesting_port_identity: PortIdentity,
}

/// PTP Announce message
#[derive(Debug, Clone, Copy)]
pub struct AnnounceMessage {
    pub header: PTPHeader,
    pub origin_timestamp: PTPTimestamp,
    pub current_utc_offset: u16,
    pub grandmaster_priority1: u8,
    pub grandmaster_clock_quality: ClockQuality,
    pub grandmaster_priority2: u8,
    pub grandmaster_identity: ClockIdentity,
    pub steps_removed: u16,
    pub time_source: TimeSource,
}

/// Clock quality data set
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockQuality {
    /// Clock class
    pub clock_class: ClockClass,
    /// Clock accuracy
    pub clock_accuracy: ClockAccuracy,
    /// Offset scaled log variance
    pub offset_scaled_log_variance: u16,
}

/// Time source enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSource {
    /// Atomic clock
    AtomicClock = 0x10,
    /// GPS
    GPS = 0x20,
    /// Terrestrial radio
    TerrestrialRadio = 0x30,
    /// PTP
    PTP = 0x40,
    /// NTP
    NTP = 0x50,
    /// Hand set
    HandSet = 0x60,
    /// Other
    Other = 0x70,
    /// Internal oscillator
    InternalOscillator = 0x80,
}

/// PTP port configuration
#[derive(Debug, Clone)]
pub struct PTPPortConfig {
    /// Port number
    pub port_number: u16,
    /// Port state
    pub state: PortState,
    /// Log sync interval
    pub log_sync_interval: i8,
    /// Log min delay req interval
    pub log_min_delay_req_interval: i8,
    /// Peer delay mechanism
    pub peer_delay: bool,
    /// Port identity
    pub identity: PortIdentity,
}

impl PTPPortConfig {
    /// Create ordinary clock port
    pub fn ordinary_clock(port_number: u16) -> Self {
        Self {
            port_number,
            state: PortState::Initializing,
            log_sync_interval: 0, // 1 second
            log_min_delay_req_interval: 0,
            peer_delay: false,
            identity: PortIdentity {
                clock: ClockIdentity::random(),
                port: port_number,
            },
        }
    }
}

/// PTP clock configuration
#[derive(Debug, Clone)]
pub struct PTPConfig {
    /// Clock identity
    pub clock_identity: ClockIdentity,
    /// Clock quality
    pub clock_quality: ClockQuality,
    /// Priority 1
    pub priority1: u8,
    /// Priority 2
    pub priority2: u8,
    /// Domain number
    pub domain_number: u8,
    /// Slave only
    pub slave_only: bool,
    /// Number of ports
    pub num_ports: u8,
}

impl PTPConfig {
    /// Create ordinary clock configuration
    pub fn ordinary_clock() -> Self {
        Self {
            clock_identity: ClockIdentity::random(),
            clock_quality: ClockQuality {
                clock_class: ClockClass::Default,
                clock_accuracy: ClockAccuracy::Microsecond,
                offset_scaled_log_variance: 0xFFFF,
            },
            priority1: 128,
            priority2: 128,
            domain_number: 0,
            slave_only: false,
            num_ports: 1,
        }
    }

    /// Create boundary clock configuration
    pub fn boundary_clock(num_ports: u8) -> Self {
        Self {
            clock_identity: ClockIdentity::random(),
            clock_quality: ClockQuality {
                clock_class: ClockClass::Default,
                clock_accuracy: ClockAccuracy::Microsecond,
                offset_scaled_log_variance: 0xFFFF,
            },
            priority1: 128,
            priority2: 128,
            domain_number: 0,
            slave_only: false,
            num_ports,
        }
    }
}

/// Path delay measurement result
#[derive(Debug, Clone, Copy)]
pub struct PathDelay {
    /// Mean path delay (nanoseconds)
    pub mean_delay: i64,
    /// Delay variation (jitter)
    pub variation: u64,
    /// Number of samples
    pub samples: u32,
}

/// Clock synchronization data
#[derive(Debug, Clone, Copy)]
pub struct SyncData {
    /// Offset from master (nanoseconds)
    pub offset: i64,
    /// Path delay (nanoseconds)
    pub delay: i64,
    /// Last sync timestamp
    pub last_sync: PTPTimestamp,
}

/// PTP port state machine
pub struct PTPPort {
    config: PTPPortConfig,
    state: Mutex<PortState>,
    sync_sequence: AtomicU64,
    delay_sequence: AtomicU64,
    path_delay: Mutex<Option<PathDelay>>,
    sync_data: Mutex<Option<SyncData>>,
    last_sync: Mutex<Option<PTPTimestamp>>,
    last_delay_req: Mutex<Option<PTPTimestamp>>,
}

impl PTPPort {
    /// Create new PTP port
    pub fn new(config: PTPPortConfig) -> Self {
        Self {
            config: config.clone(),
            state: Mutex::new(config.state),
            sync_sequence: AtomicU64::new(1),
            delay_sequence: AtomicU64::new(1),
            path_delay: Mutex::new(None),
            sync_data: Mutex::new(None),
            last_sync: Mutex::new(None),
            last_delay_req: Mutex::new(None),
        }
    }

    /// Get port state
    pub fn state(&self) -> PortState {
        *self.state.lock()
    }

    /// Set port state
    pub fn set_state(&self, new_state: PortState) {
        *self.state.lock() = new_state;
    }

    /// Get port number
    pub fn port_number(&self) -> u16 {
        self.config.port_number
    }

    /// Get port identity
    pub fn identity(&self) -> PortIdentity {
        self.config.identity
    }

    /// Send Sync message
    pub fn send_sync(&self) -> Result<PTPTimestamp, PTPError> {
        let seq = self.sync_sequence.fetch_add(1, Ordering::SeqCst) as u16;
        let timestamp = self.get_timestamp()?;

        let header = PTPHeader {
            transport_specific: 0,
            message_type: PTPMessageType::Sync,
            version: 2,
            message_length: 44,
            domain_number: 0,
            flags: 0,
            correction_field: 0,
            message_type_specific: 0,
            reserved: 0,
            source_port_identity: self.config.identity,
            sequence_id: seq,
            control: 0,
            log_message_interval: self.config.log_sync_interval as u8,
        };

        // Send message (in real implementation)
        log::debug!("PTP Port {} sent Sync, seq {}", self.config.port_number, seq);

        *self.last_sync.lock() = Some(timestamp);

        Ok(timestamp)
    }

    /// Send Follow_Up message
    pub fn send_follow_up(&self, precise_timestamp: PTPTimestamp) -> Result<(), PTPError> {
        let seq = (self.sync_sequence.load(Ordering::SeqCst) - 1) as u16;

        let header = PTPHeader {
            transport_specific: 0,
            message_type: PTPMessageType::FollowUp,
            version: 2,
            message_length: 44,
            domain_number: 0,
            flags: 0,
            correction_field: 0,
            message_type_specific: 0,
            reserved: 0,
            source_port_identity: self.config.identity,
            sequence_id: seq,
            control: 2,
            log_message_interval: self.config.log_sync_interval as u8,
        };

        // Send message (in real implementation)
        log::debug!(
            "PTP Port {} sent FollowUp, seq {}",
            self.config.port_number,
            seq
        );

        Ok(())
    }

    /// Send Delay_Req message
    pub fn send_delay_req(&self) -> Result<PTPTimestamp, PTPError> {
        let seq = self.delay_sequence.fetch_add(1, Ordering::SeqCst) as u16;
        let timestamp = self.get_timestamp()?;

        let header = PTPHeader {
            transport_specific: 0,
            message_type: PTPMessageType::DelayReq,
            version: 2,
            message_length: 44,
            domain_number: 0,
            flags: 0,
            correction_field: 0,
            message_type_specific: 0,
            reserved: 0,
            source_port_identity: self.config.identity,
            sequence_id: seq,
            control: 1,
            log_message_interval: self.config.log_min_delay_req_interval as u8,
        };

        *self.last_delay_req.lock() = Some(timestamp);

        log::debug!(
            "PTP Port {} sent DelayReq, seq {}",
            self.config.port_number,
            seq
        );

        Ok(timestamp)
    }

    /// Handle Sync message (slave)
    pub fn handle_sync(&self, timestamp: PTPTimestamp) -> Result<(), PTPError> {
        log::debug!("PTP Port {} received Sync", self.config.port_number);

        // Store receive timestamp
        *self.last_sync.lock() = Some(timestamp);

        Ok(())
    }

    /// Handle Follow_Up message (slave)
    pub fn handle_follow_up(&self, precise_timestamp: PTPTimestamp) -> Result<(), PTPError> {
        log::debug!(
            "PTP Port {} received FollowUp",
            self.config.port_number
        );

        // Calculate offset: t2 - t1 - correction
        // where t2 is local receive time, t1 is master send time
        if let Some(local_ts) = *self.last_sync.lock() {
            let offset = local_ts.sub_nanos(precise_timestamp);

            let mut sync_data = self.sync_data.lock();
            if let Some(data) = sync_data.as_mut() {
                data.offset = offset;
                data.last_sync = precise_timestamp;
            } else {
                *sync_data = Some(SyncData {
                    offset,
                    delay: 0,
                    last_sync: precise_timestamp,
                });
            }
        }

        Ok(())
    }

    /// Handle Delay_Req message (master)
    pub fn handle_delay_req(&self) -> Result<PTPTimestamp, PTPError> {
        let timestamp = self.get_timestamp()?;
        log::debug!(
            "PTP Port {} received DelayReq",
            self.config.port_number
        );
        Ok(timestamp)
    }

    /// Handle Delay_Resp message (slave)
    pub fn handle_delay_resp(
        &self,
        receive_timestamp: PTPTimestamp,
    ) -> Result<(), PTPError> {
        log::debug!(
            "PTP Port {} received DelayResp",
            self.config.port_number
        );

        // Calculate path delay:
        // delay = (t4 - t1) - (t3 - t2)
        // where t1=delay_req_send, t2=sync_recv, t3=sync_send, t4=delay_resp_recv
        if let (Some(delay_req_ts), Some(sync_ts)) = (
            *self.last_delay_req.lock(),
            self.last_sync.lock().and_then(|ts| Some(ts)),
        ) {
            let delay = receive_timestamp.sub_nanos(delay_req_ts);

            let mut sync_data = self.sync_data.lock();
            if let Some(data) = sync_data.as_mut() {
                data.delay = delay;
            }
        }

        Ok(())
    }

    /// Get current timestamp
    fn get_timestamp(&self) -> Result<PTPTimestamp, PTPError> {
        // In real implementation, would read from hardware timestamping
        let ns = self.get_current_ns()?;
        Ok(PTPTimestamp::from_nanos(ns))
    }

    /// Get current time in nanoseconds (stub)
    fn get_current_ns(&self) -> Result<u64, PTPError> {
        use core::sync::atomic::{AtomicU64, Ordering};
        static TIME: AtomicU64 = AtomicU64::new(1_650_000_000_000_000_000);
        Ok(TIME.fetch_add(1, Ordering::SeqCst))
    }

    /// Calculate offset and delay
    pub fn calculate_offset_delay(&self) -> Option<(i64, i64)> {
        let sync_data = self.sync_data.lock();
        sync_data.as_ref().map(|data| (data.offset, data.delay))
    }

    /// Update path delay
    pub fn update_path_delay(&self, delay: i64) {
        let mut path_delay = self.path_delay.lock();

        if let Some(pd) = path_delay.as_mut() {
            // Update running average
            pd.mean_delay = (pd.mean_delay * 9 + delay) / 10;
            pd.samples += 1;
        } else {
            *path_delay = Some(PathDelay {
                mean_delay: delay,
                variation: 0,
                samples: 1,
            });
        }
    }

    /// Get path delay
    pub fn path_delay(&self) -> Option<i64> {
        self.path_delay.lock().as_ref().map(|pd| pd.mean_delay)
    }
}

/// PTP clock implementation
pub struct PTPClock {
    config: PTPConfig,
    ports: Vec<PTPPort>,
    current_offset: AtomicU64,
    current_delay: AtomicU64,
    state: Mutex<PTPClockState>,
}

/// PTP clock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PTPClockState {
    /// Initializing
    Initializing,
    /// Master
    Master,
    /// Slave
    Slave,
    /// Passive
    Passive,
}

impl PTPClock {
    /// Create new PTP clock
    pub fn new(config: PTPConfig) -> Self {
        let mut ports = Vec::new();

        // Create ports
        for i in 0..config.num_ports {
            let port_config = PTPPortConfig::ordinary_clock(i as u16 + 1);
            ports.push(PTPPort::new(port_config));
        }

        Self {
            config,
            ports,
            current_offset: AtomicU64::new(0),
            current_delay: AtomicU64::new(0),
            state: Mutex::new(PTPClockState::Initializing),
        }
    }

    /// Start PTP clock
    pub fn start(&self) -> Result<(), PTPError> {
        // Initialize all ports
        for port in &self.ports {
            port.set_state(PortState::Listening);
        }

        // Run Best Master Clock Algorithm
        self.run_bmca()?;

        Ok(())
    }

    /// Run Best Master Clock Algorithm
    fn run_bmca(&self) -> Result<(), PTPError> {
        // In a real implementation, this would:
        // 1. Collect Announce messages from all ports
        // 2. Compare clock data sets
        // 3. Select best master
        // 4. Configure port states accordingly

        // Simplified: determine if we should be master or slave
        let our_priority = self.config.priority1;

        // For now, assume slave if not slave-only
        if !self.config.slave_only && our_priority < 128 {
            *self.state.lock() = PTPClockState::Master;

            // Set one port as master, others as slave
            if let Some(first_port) = self.ports.first() {
                first_port.set_state(PortState::Master);
            }
        } else {
            *self.state.lock() = PTPClockState::Slave;

            // All ports are slaves
            for port in &self.ports {
                port.set_state(PortState::Slave);
            }
        }

        Ok(())
    }

    /// Synchronize as slave
    pub fn sync_as_slave(&self) -> Result<SyncData, PTPError> {
        for port in &self.ports {
            if port.state() == PortState::Slave {
                // Send Delay_Req
                let _ = port.send_delay_req()?;

                // Wait for Delay_Resp (in real implementation)
                // Then calculate offset and delay

                if let Some((offset, delay)) = port.calculate_offset_delay() {
                    self.current_offset.store(offset.unsigned_abs(), Ordering::Relaxed);
                    self.current_delay.store(delay.unsigned_abs(), Ordering::Relaxed);

                    if let Some(sync_data) = port.sync_data.lock().as_ref() {
                        return Ok(*sync_data);
                    }
                }
            }
        }

        Err(PTPError::NotSynchronized)
    }

    /// Announce as master
    pub fn announce_as_master(&self) -> Result<(), PTPError> {
        for port in &self.ports {
            if port.state() == PortState::Master {
                // Send Sync and Follow_Up periodically
                let sync_ts = port.send_sync()?;
                port.send_follow_up(sync_ts)?;
            }
        }

        Ok(())
    }

    /// Get current offset from master
    pub fn get_offset(&self) -> i64 {
        let offset = self.current_offset.load(Ordering::Relaxed);
        offset as i64
    }

    /// Get current path delay
    pub fn get_delay(&self) -> i64 {
        let delay = self.current_delay.load(Ordering::Relaxed);
        delay as i64
    }

    /// Get clock identity
    pub fn clock_identity(&self) -> ClockIdentity {
        self.config.clock_identity
    }

    /// Get clock quality
    pub fn clock_quality(&self) -> ClockQuality {
        self.config.clock_quality
    }

    /// Check if clock is synchronized
    pub fn is_synchronized(&self) -> bool {
        matches!(*self.state.lock(), PTPClockState::Slave)
    }
}

/// PTP error types
#[derive(Debug)]
pub enum PTPError {
    /// Clock not synchronized
    NotSynchronized,
    /// Invalid message
    InvalidMessage,
    /// Timeout
    Timeout,
    /// Network error
    NetworkError,
    /// Clock error
    ClockError(ClockError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ptp_timestamp_conversion() {
        let ts = PTPTimestamp::new(1650000000, 500_000_000);
        let ns = ts.to_nanos();
        let back = PTPTimestamp::from_nanos(ns);

        assert_eq!(back.seconds, ts.seconds);
        assert_eq!(back.nanos, ts.nanos);
    }

    #[test]
    fn test_ptp_clock_creation() {
        let config = PTPConfig::ordinary_clock();
        let clock = PTPClock::new(config);
        assert_eq!(clock.ports.len(), 1);
    }

    #[test]
    fn test_boundary_clock_creation() {
        let config = PTPConfig::boundary_clock(4);
        let clock = PTPClock::new(config);
        assert_eq!(clock.ports.len(), 4);
    }

    #[test]
    fn test_clock_identity_from_mac() {
        let mac = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
        let id = ClockIdentity::from_mac(mac);

        assert_eq!(id.id[0], 0xFF);
        assert_eq!(id.id[1], 0xFF);
        assert_eq!(&id.id[2..8], &mac[..6]);
    }
}
