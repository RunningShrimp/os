//! Multiple Access Schemes for Satellite Communication
//!
//! This module implements various multiple access schemes for sharing satellite
//! communication resources among multiple users.
//!
//! # Features
//! - TDMA (Time Division Multiple Access) with frame synchronization
//! - CDMA (Code Division Multiple Access) with spreading codes
//! - FDMA (Frequency Division Multiple Access) channelization
//! - DAMA (Demand Assigned Multiple Access) reservation system
//! - ALOHA random access
//! - Power control algorithms

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::time::Duration;
use libm::log10;

use super::sat_types::{SatComError, SatResult, Timestamp};

// ============================================================================
// TDMA (Time Division Multiple Access)
// ============================================================================

/// TDMA frame configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TdmaConfig {
    /// Frame duration (ms)
    pub frame_duration_ms: u64,

    /// Number of time slots per frame
    pub num_slots: usize,

    /// Guard time between slots (us)
    pub guard_time_us: u64,

    /// Slot duration (ms)
    pub slot_duration_ms: u64,
}

impl TdmaConfig {
    /// Create new TDMA configuration
    pub fn new(frame_duration_ms: u64, num_slots: usize, guard_time_us: u64) -> Self {
        let slot_duration_ms = (frame_duration_ms * 1000 - (guard_time_us * num_slots as u64)) / num_slots as u64 / 1000;

        Self {
            frame_duration_ms,
            num_slots,
            guard_time_us,
            slot_duration_ms,
        }
    }

    /// Get slot index for current time
    pub fn current_slot(&self, frame_start: Timestamp, current: Timestamp) -> usize {
        let elapsed = current.duration_since(&frame_start).as_millis() as u64;
        let frame_elapsed = elapsed % self.frame_duration_ms;
        (frame_elapsed / self.slot_duration_ms) as usize % self.num_slots
    }

    /// Get time until next slot
    pub fn time_to_next_slot(&self, frame_start: Timestamp, current: Timestamp) -> Duration {
        let elapsed = current.duration_since(&frame_start).as_millis() as u64;
        let frame_elapsed = elapsed % self.frame_duration_ms;
        let current_slot = (frame_elapsed / self.slot_duration_ms) as usize;
        let next_slot_start = (current_slot + 1) % self.num_slots;

        let next_slot_ms = next_slot_start as u64 * self.slot_duration_ms;
        Duration::from_millis(next_slot_ms.saturating_sub(frame_elapsed))
    }
}

/// TDMA slot assignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TdmaSlot {
    /// Slot index
    pub index: usize,

    /// Assigned user ID
    pub user_id: u32,

    /// Slot status
    pub status: SlotStatus,
}

/// Slot status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotStatus {
    Free,
    Assigned,
    Reserved,
    Contention,
}

/// TDMA controller
pub struct TdmaController {
    config: TdmaConfig,

    /// Slot assignments
    slots: Vec<TdmaSlot>,

    /// Current user's assigned slot
    assigned_slot: Option<usize>,

    /// Frame start time
    frame_start: Timestamp,

    /// Statistics
    stats: TdmaStatistics,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TdmaStatistics {
    pub frames_transmitted: u64,
    pub slots_used: usize,
    pub slot_utilization: f64,
    pub collisions: u64,
}

impl TdmaController {
    /// Create new TDMA controller
    pub fn new(config: TdmaConfig) -> Self {
        let slots = (0..config.num_slots)
            .map(|i| TdmaSlot {
                index: i,
                user_id: 0,
                status: SlotStatus::Free,
            })
            .collect();

        Self {
            config,
            slots,
            assigned_slot: None,
            frame_start: Timestamp::now(),
            stats: TdmaStatistics {
                frames_transmitted: 0,
                slots_used: 0,
                slot_utilization: 0.0,
                collisions: 0,
            },
        }
    }

    /// Request slot assignment
    pub fn request_slot(&mut self, user_id: u32) -> SatResult<usize> {
        // Find free slot
        if let Some(slot_idx) = self
            .slots
            .iter()
            .position(|s| s.status == SlotStatus::Free)
        {
            self.slots[slot_idx].status = SlotStatus::Assigned;
            self.slots[slot_idx].user_id = user_id;
            self.assigned_slot = Some(slot_idx);
            Ok(slot_idx)
        } else {
            Err(SatComError::ResourceNotAvailable)
        }
    }

    /// Release slot assignment
    pub fn release_slot(&mut self, slot_idx: usize) -> SatResult<()> {
        if slot_idx < self.slots.len() {
            self.slots[slot_idx].status = SlotStatus::Free;
            self.slots[slot_idx].user_id = 0;

            if self.assigned_slot == Some(slot_idx) {
                self.assigned_slot = None;
            }

            Ok(())
        } else {
            Err(SatComError::InvalidParameter("Invalid slot index".to_string()))
        }
    }

    /// Check if it's our turn to transmit
    pub fn is_tx_slot(&self, current: Timestamp) -> bool {
        if let Some(slot) = self.assigned_slot {
            let current_slot = self.config.current_slot(self.frame_start, current);
            current_slot == slot
        } else {
            false
        }
    }

    /// Get time until next transmit slot
    pub fn time_to_tx(&self, current: Timestamp) -> Duration {
        if let Some(slot) = self.assigned_slot {
            let current_slot = self.config.current_slot(self.frame_start, current);

            if current_slot == slot {
                Duration::from_secs(0)
            } else if current_slot < slot {
                let slots_wait = slot - current_slot;
                Duration::from_millis(slots_wait as u64 * self.config.slot_duration_ms)
            } else {
                let slots_wait = self.config.num_slots - current_slot + slot;
                Duration::from_millis(slots_wait as u64 * self.config.slot_duration_ms)
            }
        } else {
            Duration::MAX
        }
    }

    /// Update frame reference
    pub fn update_frame(&mut self, frame_start: Timestamp) {
        self.frame_start = frame_start;
        self.stats.frames_transmitted += 1;
        self.update_statistics();
    }

    /// Update statistics
    fn update_statistics(&mut self) {
        self.stats.slots_used = self
            .slots
            .iter()
            .filter(|s| s.status != SlotStatus::Free)
            .count();
        self.stats.slot_utilization =
            self.stats.slots_used as f64 / self.slots.len() as f64;
    }

    /// Get statistics
    pub fn statistics(&self) -> &TdmaStatistics {
        &self.stats
    }
}

// ============================================================================
// CDMA (Code Division Multiple Access)
// ============================================================================

/// Spreading code type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpreadingCode {
    Walsh(u8),          // Walsh-Hadamard code (length = 2^order)
    Gold(u32),          // Gold code (from LFSR)
    Kasami(u32),        // Kasami code
    PcPn(u32),          // Pseudo-random noise code
}

impl SpreadingCode {
    /// Get code length
    pub fn length(&self) -> usize {
        match self {
            SpreadingCode::Walsh(order) => 1 << order,
            SpreadingCode::Gold(seed) => (((seed >> 16) & 0xFF) + 1) as usize,
            SpreadingCode::Kasami(seed) => (((seed >> 16) & 0xFF) + 1) as usize,
            SpreadingCode::PcPn(seed) => (((seed >> 16) & 0xFF) + 1) as usize,
        }
    }

    /// Generate code sequence
    pub fn generate(&self) -> Vec<i8> {
        match self {
            SpreadingCode::Walsh(order) => self.generate_walsh(*order),
            SpreadingCode::Gold(seed) => self.generate_gold(*seed),
            SpreadingCode::Kasami(seed) => self.generate_kasami(*seed),
            SpreadingCode::PcPn(seed) => self.generate_pn(*seed),
        }
    }

    fn generate_walsh(&self, order: u8) -> Vec<i8> {
        let len = 1 << order;
        let mut code = vec![1i8; len * len];

        // Generate Walsh-Hadamard matrix
        let mut size = 1;
        while size < len {
            for i in 0..size {
                for j in 0..size {
                    code[i * len + j + size] = code[i * len + j];
                    code[(i + size) * len + j] = code[i * len + j];
                    code[(i + size) * len + j + size] = -code[i * len + j];
                }
            }
            size *= 2;
        }

        // Return first row (or use index to select different codes)
        code[..len].to_vec()
    }

    fn generate_gold(&self, seed: u32) -> Vec<i8> {
        let len = (((seed >> 16) & 0xFF) + 1) as usize;

        // Preferred pair of m-sequences
        let mut lfsr1 = (seed & 0xFFFF) | 0x01;
        let mut lfsr2 = ((seed >> 8) & 0xFFFF) | 0x01;

        let mut code = Vec::with_capacity(len);
        for _ in 0..len {
            // XOR of two LFSR outputs
            let bit = ((lfsr1 & 1) ^ (lfsr2 & 1)) as i8;

            // Map 0 -> 1, 1 -> -1
            code.push(if bit == 0 { 1 } else { -1 });

            // Update LFSRs (primitive polynomials)
            lfsr1 = (lfsr1 >> 1) ^ (((lfsr1 & 1) * 0x8016) & 0xFFFF);
            lfsr2 = (lfsr2 >> 1) ^ (((lfsr2 & 1) * 0x8010) & 0xFFFF);
        }

        code
    }

    fn generate_kasami(&self, _seed: u32) -> Vec<i8> {
        // Simplified Kasami sequence generation
        vec![1, -1, 1, 1, -1, 1, -1, -1]
    }

    fn generate_pn(&self, seed: u32) -> Vec<i8> {
        let len = (((seed >> 16) & 0xFF) + 1) as usize;
        let mut lfsr = seed | 0x01;

        let mut code = Vec::with_capacity(len);
        for _ in 0..len {
            let bit = (lfsr & 1) as i8;
            code.push(if bit == 0 { 1 } else { -1 });

            // Update LFSR
            lfsr = (lfsr >> 1) ^ (((lfsr & 1) * 0x8016) & 0xFFFF);
        }

        code
    }
}

/// CDMA channel
#[derive(Debug, Clone)]
pub struct CdmaChannel {
    /// Channel code
    code: SpreadingCode,

    /// User ID
    user_id: u32,

    /// Spreading factor
    spreading_factor: usize,

    /// Channel status
    active: bool,
}

impl CdmaChannel {
    /// Create new CDMA channel
    pub fn new(user_id: u32, code: SpreadingCode) -> Self {
        Self {
            code,
            user_id,
            spreading_factor: code.length(),
            active: true,
        }
    }

    /// Spread data
    pub fn spread(&self, data: &[i8]) -> Vec<i8> {
        let code = self.code.generate();
        let mut spread = Vec::with_capacity(data.len() * self.spreading_factor);

        for &symbol in data {
            for &chip in &code {
                spread.push(symbol * chip);
            }
        }

        spread
    }

    /// Despread data
    pub fn despread(&self, spread_data: &[i8]) -> Vec<i8> {
        let code = self.code.generate();
        let mut data = Vec::with_capacity(spread_data.len() / self.spreading_factor);

        for chunk in spread_data.chunks(self.spreading_factor) {
            let mut correlation = 0i32;

            for (i, &chip) in chunk.iter().enumerate() {
                if i < code.len() {
                    correlation += (chip * code[i]) as i32;
                }
            }

            // Make hard decision
            let symbol = if correlation > 0 { 1 } else { -1 };
            data.push(symbol);
        }

        data
    }
}

/// CDMA controller
pub struct CdmaController {
    /// Active channels
    channels: BTreeMap<u32, CdmaChannel>,

    /// Processing gain (dB)
    processing_gain: f64,

    /// Statistics
    stats: CdmaStatistics,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CdmaStatistics {
    pub active_channels: usize,
    pub total_channels: usize,
    pub average_load: f64,
}

impl CdmaController {
    /// Create new CDMA controller
    pub fn new(max_channels: usize) -> Self {
        Self {
            channels: BTreeMap::new(),
            processing_gain: 10.0 * log10(max_channels as f64),
            stats: CdmaStatistics {
                active_channels: 0,
                total_channels: max_channels,
                average_load: 0.0,
            },
        }
    }

    /// Assign channel to user
    pub fn assign_channel(&mut self, user_id: u32, code: SpreadingCode) -> SatResult<()> {
        if self.channels.contains_key(&user_id) {
            return Err(SatComError::InvalidParameter(
                "User already has channel".to_string(),
            ));
        }

        let channel = CdmaChannel::new(user_id, code);
        self.channels.insert(user_id, channel);

        self.update_statistics();
        Ok(())
    }

    /// Release channel
    pub fn release_channel(&mut self, user_id: u32) -> SatResult<()> {
        if self.channels.remove(&user_id).is_some() {
            self.update_statistics();
            Ok(())
        } else {
            Err(SatComError::InvalidParameter("User not found".to_string()))
        }
    }

    /// Get channel for user
    pub fn get_channel(&self, user_id: u32) -> Option<&CdmaChannel> {
        self.channels.get(&user_id)
    }

    /// Update statistics
    fn update_statistics(&mut self) {
        self.stats.active_channels = self.channels.len();
        self.stats.average_load =
            self.stats.active_channels as f64 / self.stats.total_channels as f64;
    }

    /// Get statistics
    pub fn statistics(&self) -> &CdmaStatistics {
        &self.stats
    }
}

// ============================================================================
// FDMA (Frequency Division Multiple Access)
// ============================================================================

/// FDMA channel
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FdmaChannel {
    /// Channel ID
    pub id: u32,

    /// Center frequency (Hz)
    pub center_freq: f64,

    /// Bandwidth (Hz)
    pub bandwidth: f64,

    /// Assigned user
    pub user_id: Option<u32>,
}

/// FDMA controller
pub struct FdmaController {
    /// Available channels
    channels: Vec<FdmaChannel>,

    /// Channel assignments
    assignments: BTreeMap<u32, usize>, // user_id -> channel_idx

    /// Total system bandwidth
    total_bandwidth: f64,
}

impl FdmaController {
    /// Create new FDMA controller
    pub fn new(total_bandwidth: f64, num_channels: usize) -> Self {
        let channel_bw = total_bandwidth / num_channels as f64;

        let channels = (0..num_channels)
            .map(|i| FdmaChannel {
                id: i as u32,
                center_freq: (i as f64 + 0.5) * channel_bw,
                bandwidth: channel_bw,
                user_id: None,
            })
            .collect();

        Self {
            channels,
            assignments: BTreeMap::new(),
            total_bandwidth,
        }
    }

    /// Assign channel to user
    pub fn assign_channel(&mut self, user_id: u32) -> SatResult<FdmaChannel> {
        // Find free channel
        if let Some(idx) = self.channels.iter().position(|c| c.user_id.is_none()) {
            self.channels[idx].user_id = Some(user_id);
            self.assignments.insert(user_id, idx);

            Ok(self.channels[idx])
        } else {
            Err(SatComError::ResourceNotAvailable)
        }
    }

    /// Release channel
    pub fn release_channel(&mut self, user_id: u32) -> SatResult<()> {
        if let Some(&idx) = self.assignments.get(&user_id) {
            self.channels[idx].user_id = None;
            self.assignments.remove(&user_id);
            Ok(())
        } else {
            Err(SatComError::InvalidParameter("User not found".to_string()))
        }
    }

    /// Get channel for user
    pub fn get_channel(&self, user_id: u32) -> Option<FdmaChannel> {
        self.assignments
            .get(&user_id)
            .map(|&idx| self.channels[idx])
    }
}

// ============================================================================
// DAMA (Demand Assigned Multiple Access)
// ============================================================================

/// DAMA request
#[derive(Debug, Clone, PartialEq)]
pub struct DamaRequest {
    /// User ID
    pub user_id: u32,

    /// Requested bandwidth (Hz)
    pub bandwidth: f64,

    /// Requested duration (ms)
    pub duration_ms: u64,

    /// Priority (0 = highest)
    pub priority: u8,

    /// Request timestamp
    pub timestamp: Timestamp,
}

/// DAMA controller
pub struct DamaController {
    /// Pending requests
    pending: Vec<DamaRequest>,

    /// Active allocations
    allocations: BTreeMap<u32, DamaAllocation>,

    /// Total available bandwidth
    total_bandwidth: f64,

    /// Allocated bandwidth
    allocated_bandwidth: f64,
}

/// Active DAMA allocation
#[derive(Debug, Clone, PartialEq)]
pub struct DamaAllocation {
    pub user_id: u32,
    pub bandwidth: f64,
    pub expires_at: Timestamp,
}

impl DamaController {
    /// Create new DAMA controller
    pub fn new(total_bandwidth: f64) -> Self {
        Self {
            pending: Vec::new(),
            allocations: BTreeMap::new(),
            total_bandwidth,
            allocated_bandwidth: 0.0,
        }
    }

    /// Submit bandwidth request
    pub fn request_bandwidth(&mut self, request: DamaRequest) -> SatResult<()> {
        // Check if request can be satisfied
        let available = self.total_bandwidth - self.allocated_bandwidth;

        if request.bandwidth <= available {
            // Grant immediately
            self.grant_allocation(&request)?;
            Ok(())
        } else {
            // Queue request
            self.pending.push(request);
            self.pending.sort_by_key(|r| r.priority);
            Err(SatComError::ResourceNotAvailable)
        }
    }

    /// Grant allocation
    fn grant_allocation(&mut self, request: &DamaRequest) -> SatResult<()> {
        let expires_at =
            Timestamp::from_nanos(request.timestamp.as_nanos() + request.duration_ms * 1_000_000);

        let allocation = DamaAllocation {
            user_id: request.user_id,
            bandwidth: request.bandwidth,
            expires_at,
        };

        self.allocated_bandwidth += request.bandwidth;
        self.allocations.insert(request.user_id, allocation);

        Ok(())
    }

    /// Process pending requests
    pub fn process_pending(&mut self) {
        // Check for expired allocations
        let now = Timestamp::now();
        let expired: Vec<u32> = self
            .allocations
            .iter()
            .filter(|(_, alloc)| alloc.expires_at.as_nanos() < now.as_nanos())
            .map(|(user_id, _)| *user_id)
            .collect();

        for user_id in expired {
            self.release_allocation(user_id);
        }

        // Try to grant pending requests
        let mut i = 0;
        while i < self.pending.len() {
            let available = self.total_bandwidth - self.allocated_bandwidth;

            if self.pending[i].bandwidth <= available {
                let request = self.pending.remove(i);
                let _ = self.grant_allocation(&request);
            } else {
                i += 1;
            }
        }
    }

    /// Release allocation
    fn release_allocation(&mut self, user_id: u32) {
        if let Some(alloc) = self.allocations.remove(&user_id) {
            self.allocated_bandwidth -= alloc.bandwidth;
        }
    }

    /// Get allocation for user
    pub fn get_allocation(&self, user_id: u32) -> Option<&DamaAllocation> {
        self.allocations.get(&user_id)
    }
}

// ============================================================================
// ALOHA Random Access
// ============================================================================

/// ALOHA controller
pub struct AlohaController {
    /// Backoff parameters
    min_backoff_ms: u64,
    max_backoff_ms: u64,

    /// Statistics
    stats: AlohaStatistics,

    /// Last collision time
    last_collision: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlohaStatistics {
    pub tx_attempts: u64,
    pub collisions: u64,
    pub successes: u64,
    pub collision_rate: f64,
}

impl AlohaController {
    /// Create new ALOHA controller
    pub fn new(min_backoff_ms: u64, max_backoff_ms: u64) -> Self {
        Self {
            min_backoff_ms,
            max_backoff_ms,
            stats: AlohaStatistics {
                tx_attempts: 0,
                collisions: 0,
                successes: 0,
                collision_rate: 0.0,
            },
            last_collision: None,
        }
    }

    /// Calculate backoff time
    pub fn calculate_backoff(&self, attempt: u8) -> Duration {
        let range_ms = self.max_backoff_ms - self.min_backoff_ms;
        let backoff_ms =
            self.min_backoff_ms + (attempt as u64 % (range_ms + 1));

        Duration::from_millis(backoff_ms)
    }

    /// Record transmission attempt
    pub fn record_attempt(&mut self) {
        self.stats.tx_attempts += 1;
    }

    /// Record collision
    pub fn record_collision(&mut self) {
        self.stats.collisions += 1;
        self.last_collision = Some(Timestamp::now());
        self.update_collision_rate();
    }

    /// Record success
    pub fn record_success(&mut self) {
        self.stats.successes += 1;
        self.update_collision_rate();
    }

    /// Update collision rate
    fn update_collision_rate(&mut self) {
        if self.stats.tx_attempts > 0 {
            self.stats.collision_rate =
                self.stats.collisions as f64 / self.stats.tx_attempts as f64;
        }
    }

    /// Get statistics
    pub fn statistics(&self) -> &AlohaStatistics {
        &self.stats
    }
}

// ============================================================================
// Power Control
// ============================================================================

/// Power control algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerControlAlgorithm {
    /// Open-loop (based on path loss)
    OpenLoop,

    /// Closed-loop (based on received power)
    ClosedLoop,

    /// Adaptive (combination)
    Adaptive,
}

/// Power control controller
pub struct PowerController {
    /// Algorithm type
    algorithm: PowerControlAlgorithm,

    /// Target received power (dBm)
    target_power: f64,

    /// Current transmit power (dBm)
    current_power: f64,

    /// Maximum power (dBm)
    max_power: f64,

    /// Minimum power (dBm)
    min_power: f64,

    /// Step size (dB)
    step_size: f64,
}

impl PowerController {
    /// Create new power controller
    pub fn new(
        algorithm: PowerControlAlgorithm,
        target_power: f64,
        max_power: f64,
        min_power: f64,
    ) -> Self {
        Self {
            algorithm,
            target_power,
            current_power: min_power,
            max_power,
            min_power,
            step_size: 1.0,
        }
    }

    /// Update power based on received signal strength
    pub fn update_power(&mut self, received_power: f64) -> f64 {
        let error = self.target_power - received_power;

        // Adjust power
        if error > 0.5 {
            self.current_power = (self.current_power + self.step_size).min(self.max_power);
        } else if error < -0.5 {
            self.current_power = (self.current_power - self.step_size).max(self.min_power);
        }

        self.current_power
    }

    /// Get current transmit power
    pub fn current_power(&self) -> f64 {
        self.current_power
    }

    /// Set power based on path loss (open-loop)
    pub fn set_power_from_pathloss(&mut self, path_loss_db: f64) {
        let required_power = self.target_power + path_loss_db;
        self.current_power = required_power.clamp(self.min_power, self.max_power);
    }
}
