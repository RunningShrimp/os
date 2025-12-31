//! Network Slicing Implementation for 5G/6G
//!
//! This module implements network slicing as specified in 3GPP specifications:
//! - Slice instantiation and lifecycle management
//! - Quality of Service (QoS) guarantees per slice
//! - Resource isolation (compute, storage, network)
//! - Slice selection function
//! - Service Level Agreement (SLA) monitoring and enforcement
//! - Dynamic slice scaling
//!
//! Based on 3GPP TS 23.501, 23.503, 28.531, 28.532

#![allow(dead_code)]

use alloc::{collections::BTreeMap, string::String, sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};

use crate::subsystems::sync::Mutex;

// ============================================================================
// Network Slice Types
// ============================================================================

/// Slice identifier (S-NSSAI: Single Network Slice Selection Assistance Information)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SliceId {
    /// Slice/Service Type (SST)
    pub sst: u8,
    /// Slice Differentiator (SD)
    pub sd: Option<u32>,
}

impl SliceId {
    /// Create new slice ID
    pub fn new(sst: u8, sd: Option<u32>) -> Self {
        Self { sst, sd }
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![self.sst];
        if let Some(sd) = self.sd {
            bytes.extend_from_slice(&sd.to_be_bytes());
        }
        bytes
    }
}

/// Standard SST values defined by 3GPP
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardSst {
    /// eMBB: Enhanced Mobile Broadband
    EnhancedMobileBroadband = 1,
    /// URLLC: Ultra-Reliable Low Latency Communications
    Urllc = 2,
    /// mMTC: Massive Machine Type Communications
    MassiveMtc = 3,
    /// V2X: Vehicle-to-Everything
    V2X = 4,
    /// MIoT: Mission-Critical IoT
    MissionCriticalIoT = 5,
}

impl StandardSst {
    /// Get SST value
    pub fn sst(&self) -> u8 {
        *self as u8
    }
}

// ============================================================================
// Slice Configuration
// ============================================================================

/// Slice configuration
#[derive(Debug, Clone)]
pub struct SliceConfig {
    /// Slice identifier
    pub slice_id: SliceId,
    /// Slice name
    pub name: String,
    /// Slice type
    pub slice_type: SliceType,
    /// QoS requirements
    pub qos_requirements: QosRequirements,
    /// Resource requirements
    pub resource_requirements: ResourceRequirements,
    /// SLA parameters
    pub sla_params: SlaParams,
    /// Priority level (1-10, higher is more important)
    pub priority: u8,
}

/// Slice type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceType {
    /// Enhanced Mobile Broadband
    EmBB,
    /// Ultra-Reliable Low Latency Communications
    UrLLC,
    /// Massive Machine Type Communications
    mMTC,
    /// Custom slice type
    Custom { sst: u8 },
}

/// QoS requirements
#[derive(Debug, Clone)]
pub struct QosRequirements {
    /// Guaranteed bitrate downlink (kbps)
    pub gbr_dl_kbps: u32,
    /// Guaranteed bitrate uplink (kbps)
    pub gbr_ul_kbps: u32,
    /// Maximum bitrate downlink (kbps)
    pub mbr_dl_kbps: u32,
    /// Maximum bitrate uplink (kbps)
    pub mbr_ul_kbps: u32,
    /// Maximum packet loss rate (1e-5)
    pub max_loss_rate: u32,
    /// Maximum latency (ms)
    pub max_latency_ms: u32,
    /// Jitter budget (ms)
    pub jitter_budget_ms: u32,
}

/// Resource requirements
#[derive(Debug, Clone)]
pub struct ResourceRequirements {
    /// Compute capacity (MIPS)
    pub compute_mips: u32,
    /// Storage capacity (MB)
    pub storage_mb: u32,
    /// Network bandwidth (Mbps)
    pub bandwidth_mbps: u32,
    /// CPU cores
    pub cpu_cores: u8,
    /// Memory (GB)
    pub memory_gb: u32,
    /// Maximum number of UEs
    pub max_ues: u32,
}

/// SLA parameters
#[derive(Debug, Clone)]
pub struct SlaParams {
    /// Availability target (percentage)
    pub availability_target: u8, // 99.999 = 99.999%
    /// Monitoring interval (seconds)
    pub monitoring_interval_sec: u32,
    /// Alert threshold (percentage)
    pub alert_threshold: u8,
    /// Auto-scaling enabled
    pub auto_scaling: bool,
    /// Minimum guaranteed resources
    pub min_resources: ResourceRequirements,
    /// Maximum allowed resources
    pub max_resources: ResourceRequirements,
}

// ============================================================================
// Slice State
// ============================================================================

/// Slice lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceState {
    /// Slice is being created
    Creating,
    /// Slice is active
    Active,
    /// Slice is suspended
    Suspended,
    /// Slice is being deleted
    Deleting,
    /// Slice is terminated
    Terminated,
}

/// Slice instance
#[derive(Debug)]
pub struct SliceInstance {
    /// Slice configuration
    config: SliceConfig,
    /// Current state
    state: SliceState,
    /// Allocated resources
    allocated_resources: ResourceRequirements,
    /// Current performance metrics
    metrics: SliceMetrics,
    /// Connected UEs
    connected_ues: Vec<u64>,
    /// Creation timestamp
    created_at: u64,
}

/// Slice performance metrics
#[derive(Debug, Default)]
pub struct SliceMetrics {
    /// Total data transmitted (bytes)
    pub bytes_tx: AtomicU64,
    /// Total data received (bytes)
    pub bytes_rx: AtomicU64,
    /// Active sessions
    pub active_sessions: AtomicU64,
    /// Total sessions
    pub total_sessions: AtomicU64,
    /// Average latency (microseconds, scaled)
    pub avg_latency_us: AtomicU64,
    /// Packet loss count
    pub packet_losses: AtomicU64,
    /// SLA violations
    pub sla_violations: AtomicU64,
}

impl SliceInstance {
    /// Create new slice instance
    pub fn new(config: SliceConfig) -> Self {
        Self {
            config,
            state: SliceState::Creating,
            allocated_resources: ResourceRequirements {
                compute_mips: 0,
                storage_mb: 0,
                bandwidth_mbps: 0,
                cpu_cores: 0,
                memory_gb: 0,
                max_ues: 0,
            },
            metrics: SliceMetrics::default(),
            connected_ues: Vec::new(),
            created_at: 0,
        }
    }

    /// Get slice ID
    pub fn slice_id(&self) -> &SliceId {
        &self.config.slice_id
    }

    /// Get slice state
    pub fn state(&self) -> SliceState {
        self.state
    }

    /// Activate slice
    pub fn activate(&mut self) -> Result<(), SliceError> {
        if self.state != SliceState::Creating && self.state != SliceState::Suspended {
            return Err(SliceError::InvalidState);
        }

        self.state = SliceState::Active;
        crate::log_info!("Slice {} activated", self.config.slice_id.sst);

        Ok(())
    }

    /// Suspend slice
    pub fn suspend(&mut self) -> Result<(), SliceError> {
        if self.state != SliceState::Active {
            return Err(SliceError::InvalidState);
        }

        self.state = SliceState::Suspended;
        crate::log_info!("Slice {} suspended", self.config.slice_id.sst);

        Ok(())
    }

    /// Terminate slice
    pub fn terminate(&mut self) -> Result<(), SliceError> {
        if self.state == SliceState::Terminated {
            return Err(SliceError::InvalidState);
        }

        self.state = SliceState::Terminated;
        crate::log_info!("Slice {} terminated", self.config.slice_id.sst);

        Ok(())
    }

    /// Add UE to slice
    pub fn add_ue(&mut self, ue_id: u64) -> Result<(), SliceError> {
        if self.state != SliceState::Active {
            return Err(SliceError::InvalidState);
        }

        if self.connected_ues.len() >= self.config.resource_requirements.max_ues as usize {
            return Err(SliceError::MaxUesExceeded);
        }

        self.connected_ues.push(ue_id);
        self.metrics.active_sessions.fetch_add(1, Ordering::Relaxed);
        self.metrics.total_sessions.fetch_add(1, Ordering::Relaxed);

        crate::log_info!("UE {} added to slice {}", ue_id, self.config.slice_id.sst);

        Ok(())
    }

    /// Remove UE from slice
    pub fn remove_ue(&mut self, ue_id: u64) -> Result<(), SliceError> {
        if let Some(pos) = self.connected_ues.iter().position(|&id| id == ue_id) {
            self.connected_ues.remove(pos);
            self.metrics.active_sessions.fetch_sub(1, Ordering::Relaxed);
            crate::log_info!("UE {} removed from slice {}", ue_id, self.config.slice_id.sst);
            Ok(())
        } else {
            Err(SliceError::UeNotFound)
        }
    }

    /// Get metrics
    pub fn get_metrics(&self) -> &SliceMetrics {
        &self.metrics
    }

    /// Record data transfer
    pub fn record_data_transfer(&self, tx_bytes: u64, rx_bytes: u64) {
        self.metrics.bytes_tx.fetch_add(tx_bytes, Ordering::Relaxed);
        self.metrics.bytes_rx.fetch_add(rx_bytes, Ordering::Relaxed);
    }

    /// Record latency
    pub fn record_latency(&self, latency_us: u64) {
        // Exponential moving average
        let current = self.metrics.avg_latency_us.load(Ordering::Relaxed);
        let alpha = 0.1;
        let new_avg = ((current as f64) * (1.0 - alpha) + (latency_us as f64) * alpha) as u64;
        self.metrics.avg_latency_us.store(new_avg, Ordering::Relaxed);
    }

    /// Check SLA compliance
    pub fn check_sla_compliance(&self) -> bool {
        let avg_latency_ms = self.metrics.avg_latency_us.load(Ordering::Relaxed) / 1000;
        let max_latency = self.config.qos_requirements.max_latency_ms;

        let avg_latency_compliant = avg_latency_ms <= max_latency as u64;

        let packet_loss_rate = if self.metrics.bytes_rx.load(Ordering::Relaxed) > 0 {
            (self.metrics.packet_losses.load(Ordering::Relaxed) * 100000) /
            self.metrics.bytes_rx.load(Ordering::Relaxed)
        } else {
            0
        };

        let loss_compliant = packet_loss_rate <= self.config.qos_requirements.max_loss_rate as u64;

        avg_latency_compliant && loss_compliant
    }
}

// ============================================================================
// Network Slicing Manager
// ============================================================================

/// Network slicing manager
#[derive(Debug)]
pub struct NetworkSlicingManager {
    slices: Mutex<BTreeMap<SliceId, Arc<Mutex<SliceInstance>>>>,
    next_ue_id: AtomicU64,
    total_resources: ResourceRequirements,
    available_resources: Mutex<ResourceRequirements>,
    stats: SlicingManagerStats,
}

/// Slicing manager statistics
#[derive(Debug, Default)]
pub struct SlicingManagerStats {
    /// Total slices created
    pub slices_created: AtomicU64,
    /// Total slices active
    pub slices_active: AtomicU64,
    /// Total UEs served
    pub total_ues: AtomicU64,
    /// SLA violations
    pub sla_violations: AtomicU64,
}

impl NetworkSlicingManager {
    /// Create new slicing manager
    pub fn new(total_resources: ResourceRequirements) -> Self {
        Self {
            slices: Mutex::new(BTreeMap::new()),
            next_ue_id: AtomicU64::new(1),
            total_resources: total_resources.clone(),
            available_resources: Mutex::new(total_resources),
            stats: SlicingManagerStats::default(),
        }
    }

    /// Create network slice
    pub fn create_slice(&self, config: SliceConfig) -> Result<SliceId, SliceError> {
        // Check resource availability
        let available = self.available_resources.lock();
        let req = &config.resource_requirements;

        let compute_available = available.compute_mips >= req.compute_mips;
        let storage_available = available.storage_mb >= req.storage_mb;
        let bandwidth_available = available.bandwidth_mbps >= req.bandwidth_mbps;

        if !compute_available || !storage_available || !bandwidth_available {
            return Err(SliceError::InsufficientResources);
        }

        drop(available);

        // Create slice instance
        let slice = SliceInstance::new(config.clone());
        let slice_id = config.slice_id.clone();

        // Allocate resources
        let mut available = self.available_resources.lock();
        available.compute_mips -= req.compute_mips;
        available.storage_mb -= req.storage_mb;
        available.bandwidth_mbps -= req.bandwidth_mbps;

        // Add to slices map
        let mut slices = self.slices.lock();
        slices.insert(slice_id.clone(), Arc::new(Mutex::new(slice)));

        self.stats.slices_created.fetch_add(1, Ordering::Relaxed);

        crate::log_info!("Network slice created: SST={}", slice_id.sst);

        Ok(slice_id)
    }

    /// Activate slice
    pub fn activate_slice(&self, slice_id: &SliceId) -> Result<(), SliceError> {
        let slices = self.slices.lock();

        if let Some(slice) = slices.get(slice_id) {
            let mut slice = slice.lock();
            slice.activate()?;
            self.stats.slices_active.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            Err(SliceError::SliceNotFound)
        }
    }

    /// Suspend slice
    pub fn suspend_slice(&self, slice_id: &SliceId) -> Result<(), SliceError> {
        let slices = self.slices.lock();

        if let Some(slice) = slices.get(slice_id) {
            let mut slice = slice.lock();
            slice.suspend()?;
            self.stats.slices_active.fetch_sub(1, Ordering::Relaxed);
            Ok(())
        } else {
            Err(SliceError::SliceNotFound)
        }
    }

    /// Delete slice
    pub fn delete_slice(&self, slice_id: &SliceId) -> Result<(), SliceError> {
        let slices = self.slices.lock();

        if let Some(slice) = slices.get(slice_id) {
            let slice = slice.lock();
            let state = slice.state();

            if state == SliceState::Active {
                return Err(SliceError::InvalidState);
            }

            // Release resources
            let req = &slice.config.resource_requirements;
            let mut available = self.available_resources.lock();
            available.compute_mips += req.compute_mips;
            available.storage_mb += req.storage_mb;
            available.bandwidth_mbps += req.bandwidth_mbps;
        }

        drop(slices);

        let mut slices = self.slices.lock();
        slices.remove(slice_id);

        crate::log_info!("Network slice deleted: SST={}", slice_id.sst);

        Ok(())
    }

    /// Select slice for UE
    pub fn select_slice_for_ue(
        &self,
        requested_sst: u8,
        ue_capabilities: &UeCapabilities,
    ) -> Result<SliceId, SliceError> {
        let slices = self.slices.lock();

        // Find matching slice
        for (slice_id, slice) in slices.iter() {
            let slice = slice.lock();

            if slice.state() != SliceState::Active {
                continue;
            }

            // Check SST match
            if slice_id.sst != requested_sst {
                continue;
            }

            // Check UE count limit
            if slice.connected_ues.len() >= slice.config.resource_requirements.max_ues as usize {
                continue;
            }

            // Check UE capabilities
            if !self.check_ue_compatibility(&slice.config, ue_capabilities) {
                continue;
            }

            let selected_id = slice_id.clone();
            drop(slice);
            drop(slices);

            return Ok(selected_id);
        }

        Err(SliceError::NoSuitableSlice)
    }

    /// Check UE compatibility with slice
    fn check_ue_compatibility(
        &self,
        config: &SliceConfig,
        capabilities: &UeCapabilities,
    ) -> bool {
        // Check if UE supports required features
        match config.slice_type {
            SliceType::UrLLC => capabilities.supports_low_latency,
            SliceType::EmBB => capabilities.supports_high_bandwidth,
            SliceType::mMTC => capabilities.supports_low_power,
            SliceType::Custom { .. } => true,
        }
    }

    /// Attach UE to slice
    pub fn attach_ue_to_slice(&self, slice_id: &SliceId) -> Result<u64, SliceError> {
        let slices = self.slices.lock();

        if let Some(_slice) = slices.get(slice_id) {
            let ue_id = self.next_ue_id.fetch_add(1, Ordering::Relaxed);
            drop(slices);

            let slices = self.slices.lock();
            let slice = slices.get(slice_id).unwrap();
            let mut slice = slice.lock();
            slice.add_ue(ue_id)?;

            self.stats.total_ues.fetch_add(1, Ordering::Relaxed);

            crate::log_info!("UE {} attached to slice {}", ue_id, slice_id.sst);

            Ok(ue_id)
        } else {
            Err(SliceError::SliceNotFound)
        }
    }

    /// Detach UE from slice
    pub fn detach_ue_from_slice(&self, slice_id: &SliceId, ue_id: u64) -> Result<(), SliceError> {
        let slices = self.slices.lock();

        if let Some(slice) = slices.get(slice_id) {
            let mut slice = slice.lock();
            slice.remove_ue(ue_id)?;

            crate::log_info!("UE {} detached from slice {}", ue_id, slice_id.sst);

            Ok(())
        } else {
            Err(SliceError::SliceNotFound)
        }
    }

    /// Monitor all slices for SLA compliance
    pub fn monitor_slices(&self) -> Vec<(SliceId, bool)> {
        let slices = self.slices.lock();
        let mut results = Vec::new();

        for (slice_id, slice) in slices.iter() {
            let slice = slice.lock();
            let compliant = slice.check_sla_compliance();

            if !compliant {
                self.stats.sla_violations.fetch_add(1, Ordering::Relaxed);
            }

            results.push((slice_id.clone(), compliant));
        }

        results
    }

    /// Scale slice resources
    pub fn scale_slice_resources(
        &self,
        slice_id: &SliceId,
        scale_factor: f32,
    ) -> Result<(), SliceError> {
        let slices = self.slices.lock();

        if let Some(slice) = slices.get(slice_id) {
            let mut slice = slice.lock();
            let req = &slice.config.resource_requirements;

            // Calculate new resource requirements
            let new_compute = (req.compute_mips as f32 * scale_factor) as u32;
            let new_storage = (req.storage_mb as f32 * scale_factor) as u32;
            let new_bandwidth = (req.bandwidth_mbps as f32 * scale_factor) as u32;

            // Check if within SLA limits
            if new_compute > slice.config.sla_params.max_resources.compute_mips {
                return Err(SliceError::SlaViolation);
            }

            // Update allocated resources
            slice.allocated_resources.compute_mips = new_compute;
            slice.allocated_resources.storage_mb = new_storage;
            slice.allocated_resources.bandwidth_mbps = new_bandwidth;

            crate::log_info!(
                "Slice {} scaled by {:.1}x",
                slice_id.sst,
                scale_factor
            );

            Ok(())
        } else {
            Err(SliceError::SliceNotFound)
        }
    }

    /// Get slice metrics
    pub fn get_slice_metrics(&self, slice_id: &SliceId) -> Result<SliceMetrics, SliceError> {
        let slices = self.slices.lock();

        if let Some(slice) = slices.get(slice_id) {
            let slice = slice.lock();

            // Clone the metrics
            Ok(SliceMetrics {
                bytes_tx: AtomicU64::new(slice.metrics.bytes_tx.load(Ordering::Relaxed)),
                bytes_rx: AtomicU64::new(slice.metrics.bytes_rx.load(Ordering::Relaxed)),
                active_sessions: AtomicU64::new(slice.metrics.active_sessions.load(Ordering::Relaxed)),
                total_sessions: AtomicU64::new(slice.metrics.total_sessions.load(Ordering::Relaxed)),
                avg_latency_us: AtomicU64::new(slice.metrics.avg_latency_us.load(Ordering::Relaxed)),
                packet_losses: AtomicU64::new(slice.metrics.packet_losses.load(Ordering::Relaxed)),
                sla_violations: AtomicU64::new(slice.metrics.sla_violations.load(Ordering::Relaxed)),
            })
        } else {
            Err(SliceError::SliceNotFound)
        }
    }

    /// Get manager statistics
    pub fn get_stats(&self) -> (u64, u64, u64, u64) {
        (
            self.stats.slices_created.load(Ordering::Relaxed),
            self.stats.slices_active.load(Ordering::Relaxed),
            self.stats.total_ues.load(Ordering::Relaxed),
            self.stats.sla_violations.load(Ordering::Relaxed),
        )
    }

    /// Get all slice IDs
    pub fn get_all_slices(&self) -> Vec<SliceId> {
        let slices = self.slices.lock();
        slices.keys().cloned().collect()
    }
}

/// UE capabilities
#[derive(Debug, Clone)]
pub struct UeCapabilities {
    /// Supports low latency
    pub supports_low_latency: bool,
    /// Supports high bandwidth
    pub supports_high_bandwidth: bool,
    /// Supports low power operation
    pub supports_low_power: bool,
    /// Maximum supported bandwidth (Mbps)
    pub max_bandwidth_mbps: u32,
    /// Supported features
    pub features: Vec<String>,
}

// ============================================================================
// Slice Selection Function (SSF)
// ============================================================================

/// Slice selection function
#[derive(Debug)]
pub struct SliceSelectionFunction {
    manager: Arc<NetworkSlicingManager>,
    selection_policy: SelectionPolicy,
}

/// Slice selection policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionPolicy {
    /// First matching slice
    FirstMatch,
    /// Load balancing across slices
    LoadBalancing,
    /// Priority-based selection
    PriorityBased,
    /// Random selection
    Random,
}

impl SliceSelectionFunction {
    /// Create new SSF
    pub fn new(manager: Arc<NetworkSlicingManager>, policy: SelectionPolicy) -> Self {
        Self {
            manager,
            selection_policy: policy,
        }
    }

    /// Select best slice for UE
    pub fn select_slice(
        &self,
        requested_sst: u8,
        ue_capabilities: &UeCapabilities,
    ) -> Result<SliceId, SliceError> {
        match self.selection_policy {
            SelectionPolicy::FirstMatch => {
                self.manager.select_slice_for_ue(requested_sst, ue_capabilities)
            }
            SelectionPolicy::LoadBalancing => {
                self.select_with_load_balancing(requested_sst, ue_capabilities)
            }
            SelectionPolicy::PriorityBased => {
                self.select_with_priority(requested_sst, ue_capabilities)
            }
            SelectionPolicy::Random => {
                self.select_random(requested_sst, ue_capabilities)
            }
        }
    }

    /// Select slice with load balancing
    fn select_with_load_balancing(
        &self,
        requested_sst: u8,
        _ue_capabilities: &UeCapabilities,
    ) -> Result<SliceId, SliceError> {
        let slices = self.manager.slices.lock();
        let mut candidates: Vec<_> = slices
            .iter()
            .filter(|(id, _)| id.sst == requested_sst)
            .filter(|(_, slice)| {
                let slice = slice.lock();
                slice.state() == SliceState::Active &&
                slice.connected_ues.len() < slice.config.resource_requirements.max_ues as usize
            })
            .collect();

        // Sort by number of connected UEs (ascending)
        candidates.sort_by(|a, b| {
            let a_ue = a.1.lock().connected_ues.len();
            let b_ue = b.1.lock().connected_ues.len();
            a_ue.cmp(&b_ue)
        });

        if let Some((slice_id, _)) = candidates.first() {
            Ok((*slice_id).clone())
        } else {
            Err(SliceError::NoSuitableSlice)
        }
    }

    /// Select slice with priority
    fn select_with_priority(
        &self,
        requested_sst: u8,
        _ue_capabilities: &UeCapabilities,
    ) -> Result<SliceId, SliceError> {
        let slices = self.manager.slices.lock();
        let mut candidates: Vec<_> = slices
            .iter()
            .filter(|(id, _)| id.sst == requested_sst)
            .collect();

        // Sort by priority (descending)
        candidates.sort_by(|a, b| {
            let a_slice = a.1.lock();
            let b_slice = b.1.lock();
            b_slice.config.priority.cmp(&a_slice.config.priority)
        });

        for (slice_id, slice) in candidates {
            let slice = slice.lock();
            if slice.state() == SliceState::Active &&
               slice.connected_ues.len() < slice.config.resource_requirements.max_ues as usize {
                return Ok(slice_id.clone());
            }
        }

        Err(SliceError::NoSuitableSlice)
    }

    /// Select random slice
    fn select_random(
        &self,
        requested_sst: u8,
        _ue_capabilities: &UeCapabilities,
    ) -> Result<SliceId, SliceError> {
        let slices = self.manager.slices.lock();
        let candidates: Vec<_> = slices
            .iter()
            .filter(|(id, _)| id.sst == requested_sst)
            .filter(|(_, slice)| {
                let slice = slice.lock();
                slice.state() == SliceState::Active &&
                slice.connected_ues.len() < slice.config.resource_requirements.max_ues as usize
            })
            .collect();

        if candidates.is_empty() {
            return Err(SliceError::NoSuitableSlice);
        }

        // Simple random selection (using count)
        let index = self.manager.stats.slices_created.load(Ordering::Relaxed) as usize % candidates.len();
        Ok(candidates[index].0.clone())
    }
}

// ============================================================================
// Errors
// ============================================================================

/// Slice errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SliceError {
    SliceNotFound,
    InvalidState,
    InsufficientResources,
    MaxUesExceeded,
    UeNotFound,
    NoSuitableSlice,
    SlaViolation,
    ConfigurationError,
}

// ============================================================================
// Default configurations
// ============================================================================

impl Default for QosRequirements {
    fn default() -> Self {
        Self {
            gbr_dl_kbps: 1000,
            gbr_ul_kbps: 1000,
            mbr_dl_kbps: 10000,
            mbr_ul_kbps: 5000,
            max_loss_rate: 100, // 0.001%
            max_latency_ms: 100,
            jitter_budget_ms: 10,
        }
    }
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            compute_mips: 1000,
            storage_mb: 1024,
            bandwidth_mbps: 100,
            cpu_cores: 2,
            memory_gb: 4,
            max_ues: 100,
        }
    }
}

impl Default for SlaParams {
    fn default() -> Self {
        Self {
            availability_target: 99, // 99%
            monitoring_interval_sec: 60,
            alert_threshold: 95,
            auto_scaling: true,
            min_resources: ResourceRequirements::default(),
            max_resources: ResourceRequirements {
                compute_mips: 10000,
                storage_mb: 10240,
                bandwidth_mbps: 1000,
                cpu_cores: 16,
                memory_gb: 64,
                max_ues: 1000,
            },
        }
    }
}

impl Default for UeCapabilities {
    fn default() -> Self {
        Self {
            supports_low_latency: true,
            supports_high_bandwidth: true,
            supports_low_power: false,
            max_bandwidth_mbps: 1000,
            features: vec![],
        }
    }
}
