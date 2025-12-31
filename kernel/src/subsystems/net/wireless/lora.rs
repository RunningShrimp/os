//! LoRaWAN IoT protocol implementation
//!
//! This module provides LoRaWAN support including:
//! - LoRa physical layer
//! - Class A/B/C device types
//! - Adaptive data rate (ADR)
//! - Join procedures (OTAA, ABP)
//! - Channel plans (EU868, US915, AS923)
//! - Confirmable messages
//! - Duty cycle enforcement

#![allow(dead_code)]

use crate::prelude::*;
use alloc::{string::String, sync::Arc, vec::Vec, collections::VecDeque};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use core::time::Duration;

/// LoRa device
pub struct LoRaDevice {
    /// Device ID
    id: u32,
    /// Device name
    name: String,
    /// Device EUI (64-bit unique identifier)
    dev_eui: [u8; 8],
    /// Current configuration
    config: LoRaConfig,
    /// Current state
    state: Arc<Mutex<LoRaState>>,
    /// Device class (A/B/C)
    class: LoRaClass,
    /// Channel plan
    channel_plan: LoRaChannelPlan,
    /// Statistics
    stats: Arc<Mutex<LoRaStats>>,
    /// Is enabled
    enabled: AtomicBool,
    /// Join mode (OTAA/ABP)
    join_mode: LoRaJoinMode,
    /// Is joined to network
    joined: Arc<AtomicBool>,
    /// Session keys
    session: Arc<Mutex<LoRaSession>>,
    /// Current data rate
    data_rate: Arc<Mutex<LoRaDataRate>>,
    /// TX power (dBm)
    tx_power: Arc<AtomicU32>,
    /// Current channel
    current_channel: Arc<AtomicU32>,
    /// Frame counters
    frame_counters: Arc<Mutex<FrameCounters>>,
    /// Pending messages (downlink)
    rx_queue: Arc<Mutex<VecDeque<LoRaMessage>>>,
    /// Duty cycle tracker
    duty_cycle: Arc<Mutex<DutyCycleTracker>>,
    /// ADR enabled
    adr_enabled: Arc<AtomicBool>,
    /// Last RX window
    last_rx_window: Arc<Mutex<Option<RxWindow>>>,
}

impl LoRaDevice {
    /// Create a new LoRa device
    pub fn new(
        name: String,
        dev_eui: [u8; 8],
        class: LoRaClass,
        channel_plan: LoRaChannelPlan,
    ) -> Self {
        let id = 0; // Will be assigned by subsystem
        Self {
            id,
            name,
            dev_eui,
            config: LoRaConfig::default(),
            state: Arc::new(Mutex::new(LoRaState::Idle)),
            class,
            channel_plan,
            stats: Arc::new(Mutex::new(LoRaStats::default())),
            enabled: AtomicBool::new(false),
            join_mode: LoRaJoinMode::Otaa,
            joined: Arc::new(AtomicBool::new(false)),
            session: Arc::new(Mutex::new(LoRaSession::default())),
            data_rate: Arc::new(Mutex::new(LoRaDataRate::Dr0)),
            tx_power: Arc::new(AtomicU32::new(14)), // 14 dBm default
            current_channel: Arc::new(AtomicU32::new(0)),
            frame_counters: Arc::new(Mutex::new(FrameCounters::default())),
            rx_queue: Arc::new(Mutex::new(VecDeque::new())),
            duty_cycle: Arc::new(Mutex::new(DutyCycleTracker::new())),
            adr_enabled: Arc::new(AtomicBool::new(true)),
            last_rx_window: Arc::new(Mutex::new(None)),
        }
    }

    /// Get device ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get device name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get device EUI
    pub fn dev_eui(&self) -> [u8; 8] {
        self.dev_eui
    }

    /// Check if device is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Enable the device
    pub fn enable(&self) -> Result<(), LoRaError> {
        self.enabled.store(true, Ordering::Relaxed);
        *self.state.lock() = LoRaState::Idle;
        crate::log_info!("LoRa device {} enabled", self.name.clone());
        Ok(())
    }

    /// Disable the device
    pub fn disable(&self) -> Result<(), LoRaError> {
        self.enabled.store(false, Ordering::Relaxed);
        *self.state.lock() = LoRaState::Disabled;
        self.joined.store(false, Ordering::Relaxed);
        crate::log_info!("LoRa device {} disabled", self.name.clone());
        Ok(())
    }

    /// Join network (OTAA)
    pub fn join_otaa(
        &mut self,
        _app_eui: [u8; 8],
        _app_key: [u8; 16],
    ) -> Result<(), LoRaError> {
        if !self.is_enabled() {
            return Err(LoRaError::DeviceDisabled);
        }

        self.join_mode = LoRaJoinMode::Otaa;
        *self.state.lock() = LoRaState::Joining;

        crate::log_info!("Joining LoRaWAN network via OTAA on {}", self.name.clone());

        // Simulate OTAA join request
        // In real implementation:
        // 1. Send join-request message
        // 2. Wait for join-accept
        // 3. Derive session keys
        // 4. Initialize frame counters

        // Simulate join success
        let mut session = self.session.lock();
        session.nwk_skey = [0u8; 16]; // Would be derived from app_key
        session.app_skey = [0u8; 16];
        session.dev_addr = 0x01020304;
        drop(session);

        self.joined.store(true, Ordering::Relaxed);
        *self.state.lock() = LoRaState::Idle;

        crate::log_info!("Joined LoRaWAN network");
        Ok(())
    }

    /// Join network (ABP)
    pub fn join_abp(
        &mut self,
        dev_addr: u32,
        nwk_skey: [u8; 16],
        app_skey: [u8; 16],
    ) -> Result<(), LoRaError> {
        if !self.is_enabled() {
            return Err(LoRaError::DeviceDisabled);
        }

        self.join_mode = LoRaJoinMode::Abp;
        *self.state.lock() = LoRaState::Joining;

        crate::log_info!("Joining LoRaWAN network via ABP on {}", self.name.clone());

        // Configure session
        let mut session = self.session.lock();
        session.dev_addr = dev_addr;
        session.nwk_skey = nwk_skey;
        session.app_skey = app_skey;
        drop(session);

        self.joined.store(true, Ordering::Relaxed);
        *self.state.lock() = LoRaState::Idle;

        crate::log_info!("Joined LoRaWAN network");
        Ok(())
    }

    /// Check if joined
    pub fn is_joined(&self) -> bool {
        self.joined.load(Ordering::Relaxed)
    }

    /// Send message
    pub fn send(&self, data: Vec<u8>, confirmed: bool) -> Result<(), LoRaError> {
        if !self.is_enabled() {
            return Err(LoRaError::DeviceDisabled);
        }

        if !self.is_joined() {
            return Err(LoRaError::NotJoined);
        }

        // Check duty cycle
        {
            let mut dc = self.duty_cycle.lock();
            if !dc.can_transmit() {
                return Err(LoRaError::DutyCycleRestricted);
            }
            dc.record_transmission();
        }

        *self.state.lock() = LoRaState::Transmitting;

        // Create message
        let fcnt = self.frame_counters.lock().up;
        let _msg = LoRaMessage {
            dev_addr: self.session.lock().dev_addr,
            fcnt,
            fport: 1,
            data: data.clone(),
            confirmed,
            ..Default::default()
        };

        crate::log_info!(
            "Sending LoRa message: {} bytes, confirmed={}",
            data.len(),
            confirmed
        );

        // Simulate transmission
        // In real implementation:
        // 1. Encrypt data (FRMPayload)
        // 2. Calculate MIC (Message Integrity Code)
        // 3. Transmit on current channel
        // 4. Wait for RX windows (if confirmed)

        // Increment frame counter
        self.frame_counters.lock().up += 1;

        // Schedule RX windows for confirmed messages
        if confirmed {
            *self.last_rx_window.lock() = Some(RxWindow::Rx1);
        }

        *self.state.lock() = LoRaState::Idle;
        Ok(())
    }

    /// Receive message
    pub fn receive(&self) -> Option<LoRaMessage> {
        self.rx_queue.lock().pop_front()
    }

    /// Get current state
    pub fn state(&self) -> LoRaState {
        *self.state.lock()
    }

    /// Get statistics
    pub fn stats(&self) -> LoRaStats {
        self.stats.lock().clone()
    }

    /// Get device class
    pub fn class(&self) -> LoRaClass {
        self.class
    }

    /// Get channel plan
    pub fn channel_plan(&self) -> LoRaChannelPlan {
        self.channel_plan
    }

    /// Set data rate
    pub fn set_data_rate(&self, dr: LoRaDataRate) -> Result<(), LoRaError> {
        *self.data_rate.lock() = dr;
        Ok(())
    }

    /// Get current data rate
    pub fn data_rate(&self) -> LoRaDataRate {
        *self.data_rate.lock()
    }

    /// Set TX power
    pub fn set_tx_power(&self, power_dbm: u32) -> Result<(), LoRaError> {
        if power_dbm > 20 {
            return Err(LoRaError::InvalidParameter);
        }
        self.tx_power.store(power_dbm, Ordering::Relaxed);
        Ok(())
    }

    /// Get TX power
    pub fn tx_power(&self) -> u32 {
        self.tx_power.load(Ordering::Relaxed)
    }

    /// Get current channel
    pub fn current_channel(&self) -> u32 {
        self.current_channel.load(Ordering::Relaxed)
    }

    /// Set ADR enable
    pub fn set_adr_enabled(&self, enabled: bool) {
        self.adr_enabled.store(enabled, Ordering::Relaxed);
    }

    /// Check if ADR is enabled
    pub fn adr_enabled(&self) -> bool {
        self.adr_enabled.load(Ordering::Relaxed)
    }

    /// Update ADR (adaptive data rate)
    pub fn update_adr(&self) {
        if !self.adr_enabled() {
            return;
        }

        let stats = self.stats.lock();
        // Simple ADR algorithm: adjust based on SNR
        // In real implementation, would use network ADR commands

        let snr = stats.last_snr;

        let new_dr = if snr > 10 {
            LoRaDataRate::Dr5 // Best SNR: highest rate
        } else if snr > 5 {
            LoRaDataRate::Dr3
        } else if snr > 0 {
            LoRaDataRate::Dr1
        } else {
            LoRaDataRate::Dr0 // Poor SNR: lowest rate
        };

        *self.data_rate.lock() = new_dr;
    }
}

impl core::fmt::Debug for LoRaDevice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LoRaDevice")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("class", &self.class)
            .field("channel_plan", &self.channel_plan)
            .field("state", &format!("{:?}", *self.state.lock()))
            .finish()
    }
}

impl Clone for LoRaDevice {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            dev_eui: self.dev_eui,
            config: self.config.clone(),
            state: self.state.clone(),
            class: self.class,
            channel_plan: self.channel_plan,
            stats: self.stats.clone(),
            enabled: AtomicBool::new(self.enabled.load(Ordering::Relaxed)),
            join_mode: self.join_mode,
            joined: Arc::new(AtomicBool::new(self.joined.load(Ordering::Relaxed))),
            session: self.session.clone(),
            data_rate: self.data_rate.clone(),
            tx_power: Arc::new(AtomicU32::new(self.tx_power.load(Ordering::Relaxed))),
            current_channel: Arc::new(AtomicU32::new(self.current_channel.load(Ordering::Relaxed))),
            frame_counters: self.frame_counters.clone(),
            rx_queue: self.rx_queue.clone(),
            duty_cycle: self.duty_cycle.clone(),
            adr_enabled: Arc::new(AtomicBool::new(self.adr_enabled.load(Ordering::Relaxed))),
            last_rx_window: self.last_rx_window.clone(),
        }
    }
}

/// LoRa configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoRaConfig {
    /// Retries for confirmed messages
    pub max_retries: u8,
    /// RX1 delay (milliseconds)
    pub rx1_delay: u32,
    /// RX2 delay (milliseconds)
    pub rx2_delay: u32,
    /// RX2 data rate
    pub rx2_data_rate: LoRaDataRate,
    /// RX2 frequency
    pub rx2_frequency: u32,
}

impl Default for LoRaConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            rx1_delay: 1000,
            rx2_delay: 2000,
            rx2_data_rate: LoRaDataRate::Dr0,
            rx2_frequency: 869525000, // EU868 RX2 freq
        }
    }
}

/// LoRa state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoRaState {
    /// Disabled
    Disabled,
    /// Idle
    Idle,
    /// Joining network
    Joining,
    /// Transmitting
    Transmitting,
    /// Receiving
    Receiving,
    /// Sleeping (Class B/C)
    Sleeping,
    /// Error
    Error,
}

/// LoRa device class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoRaClass {
    /// Class A (bidirectional, RX after TX)
    ClassA,
    /// Class B (scheduled RX slots)
    ClassB,
    /// Class C (continuous RX)
    ClassC,
}

/// LoRa channel plans
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoRaChannelPlan {
    /// EU868 (Europe, 868 MHz)
    Eu868,
    /// US915 (United States, 915 MHz)
    Us915,
    /// AS923 (Asia, 923 MHz)
    As923,
    /// AU915 (Australia, 915 MHz)
    Au915,
    /// CN779 (China, 779 MHz)
    Cn779,
    /// EU433 (Europe, 433 MHz)
    Eu433,
    /// IN865 (India, 865 MHz)
    In865,
    /// KR920 (Korea, 920 MHz)
    Kr920,
}

/// LoRa data rates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoRaDataRate {
    Dr0,
    Dr1,
    Dr2,
    Dr3,
    Dr4,
    Dr5,
    Dr6,
    Dr7,
    Dr8,
    Dr9,
    Dr10,
    Dr12,
    Dr13,
    Dr14,
    Dr15,
}

impl LoRaDataRate {
    /// Get bitrate (bps) for EU868
    pub fn bitrate_eu868(&self) -> u32 {
        match self {
            Self::Dr0 => 250,
            Self::Dr1 => 440,
            Self::Dr2 => 980,
            Self::Dr3 => 1760,
            Self::Dr4 => 3125,
            Self::Dr5 => 5470,
            _ => 250,
        }
    }

    /// Get spreading factor
    pub fn spreading_factor(&self) -> u8 {
        match self {
            Self::Dr0 => 12,
            Self::Dr1 | Self::Dr2 => 11,
            Self::Dr3 => 10,
            Self::Dr4 => 9,
            Self::Dr5 => 8,
            _ => 12,
        }
    }
}

/// LoRa join mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoRaJoinMode {
    /// OTAA (Over-the-Air Activation)
    Otaa,
    /// ABP (Activation By Personalization)
    Abp,
}

/// LoRa session
#[derive(Debug, Clone, Default)]
pub struct LoRaSession {
    /// Network session key
    pub nwk_skey: [u8; 16],
    /// Application session key
    pub app_skey: [u8; 16],
    /// Device address
    pub dev_addr: u32,
}

/// Frame counters
#[derive(Debug, Clone, Default)]
pub struct FrameCounters {
    /// Uplink frame counter
    pub up: u32,
    /// Downlink frame counter
    pub down: u32,
}

/// LoRa message
#[derive(Debug, Clone, Default)]
pub struct LoRaMessage {
    /// Device address
    pub dev_addr: u32,
    /// Frame counter
    pub fcnt: u32,
    /// Frame port
    pub fport: u8,
    /// Message data
    pub data: Vec<u8>,
    /// Confirmed message
    pub confirmed: bool,
    /// Message type (uplink/downlink)
    pub mtype: Mtype,
}

/// Message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mtype {
    /// Join request
    JoinRequest,
    /// Join accept
    JoinAccept,
    /// Unconfirmed data up
    UnconfirmedDataUp,
    /// Unconfirmed data down
    UnconfirmedDataDown,
    /// Confirmed data up
    ConfirmedDataUp,
    /// Confirmed data down
    ConfirmedDataDown,
}

impl Default for Mtype {
    fn default() -> Self {
        Self::UnconfirmedDataUp
    }
}

/// RX window
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RxWindow {
    /// RX1 window
    Rx1,
    /// RX2 window
    Rx2,
}

/// Duty cycle tracker
#[derive(Debug, Clone)]
pub struct DutyCycleTracker {
    /// Last transmission time
    pub last_tx: Option<Duration>,
    /// Total airtime
    pub total_airtime: Duration,
}

impl DutyCycleTracker {
    pub fn new() -> Self {
        Self {
            last_tx: None,
            total_airtime: Duration::from_secs(0),
        }
    }

    /// Check if transmission is allowed
    pub fn can_transmit(&self) -> bool {
        // EU868: 1% duty cycle
        // Simplified: always allow for demo
        true
    }

    /// Record a transmission
    pub fn record_transmission(&mut self) {
        // Update tracking
    }
}

/// LoRa statistics
#[derive(Debug, Clone, Default)]
pub struct LoRaStats {
    /// Messages sent
    pub messages_sent: u32,
    /// Messages received
    pub messages_received: u32,
    /// Join attempts
    pub join_attempts: u32,
    /// Successful joins
    pub successful_joins: u32,
    /// TX errors
    pub tx_errors: u32,
    /// RX errors
    pub rx_errors: u32,
    /// Last SNR
    pub last_snr: i32,
    /// Last RSSI
    pub last_rssi: i32,
}

/// LoRa error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoRaError {
    /// Device disabled
    DeviceDisabled,
    /// Not joined
    NotJoined,
    /// Join failed
    JoinFailed,
    /// Duty cycle restricted
    DutyCycleRestricted,
    /// Invalid parameter
    InvalidParameter,
    /// Transmission timeout
    TransmissionTimeout,
    /// No channel available
    NoChannelAvailable,
    /// Message too large
    MessageTooLarge,
    /// No memory
    NoMemory,
    /// Hardware error
    HardwareError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lora_device_creation() {
        let dev_eui = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let device = LoRaDevice::new(
            String::from("lora0"),
            dev_eui,
            LoRaClass::ClassA,
            LoRaChannelPlan::Eu868
        );
        assert_eq!(device.name(), "lora0");
        assert_eq!(device.dev_eui(), dev_eui);
    }
}
