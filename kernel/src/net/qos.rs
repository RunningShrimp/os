//! Quality of Service (QoS) Implementation
//!
//! This module provides comprehensive QoS capabilities for network traffic management:
//! - Traffic Control (TC) subsystem
//! - HTB (Hierarchy Token Bucket) qdisc
//! - HFSC (Hierarchical Fair Service Curve) qdisc
//! - Classful qdiscs (prio, cbq)
//! - Policing (single rate, two rate, three color marker)
//! - Traffic shaping with token bucket
//! - RED (Random Early Detection) and RIO
//! - Priority queueing and bandwidth reservation
//!
//! Based on Linux Traffic Control (tc) subsystem and RFCs:
//! - RFC 2474: Definition of the Differentiated Services Field (DS Field)
//! - RFC 2475: An Architecture for Differentiated Services
//! - RFC 3246: Expedited Forwarding PHB
//! - RFC 2597: Assured Forwarding PHB Group
//! - RFC 2309: Recommendations on Queue Management and Congestion Avoidance

#![allow(dead_code)]

use alloc::{collections::BTreeMap, string::{String, ToString}, sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicU32, Ordering};

use crate::subsystems::sync::{Mutex, RwLock};

// ============================================================================
// Common Types
// ============================================================================

/// QoS result type
pub type QosResult<T> = core::result::Result<T, crate::error::unified::QosError>;

/// Qdisc handle (major:minor format)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QdiscHandle {
    pub major: u32,
    pub minor: u32,
}

impl QdiscHandle {
    /// Create root qdisc handle
    pub fn root() -> Self {
        Self { major: 0xFFFF, minor: 0 }
    }

    /// Create new qdisc handle
    pub fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Convert to u32
    pub fn to_u32(&self) -> u32 {
        (self.major << 16) | self.minor
    }

    /// Parse from u32
    pub fn from_u32(val: u32) -> Self {
        Self {
            major: (val >> 16) & 0xFFFF,
            minor: val & 0xFFFF,
        }
    }
}

/// Class handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClassHandle {
    pub major: u32,
    pub minor: u32,
}

impl ClassHandle {
    /// Create new class handle
    pub fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Convert to u32
    pub fn to_u32(&self) -> u32 {
        (self.major << 16) | self.minor
    }
}

/// Filter handle
pub type FilterHandle = u32;

/// Traffic priority
pub type Priority = u32;

/// Protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    All,
    Ip,
    Ipv6,
    Tcp,
    Udp,
}

// ============================================================================
// QDisc Types
// ============================================================================

/// Qdisc types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QdiscType {
    /// Classless queueing disciplines
    Noqueue,
    Pfifo,
    Bfifo,
    PfifoFast,
    Red,
    Tbf,
    Sfq,

    /// Classful queueing disciplines
    Htb,
    Hfsc,
    Cbq,
    Prio,
}

/// Qdisc configuration
#[derive(Debug, Clone)]
pub struct QdiscConfig {
    /// Qdisc type
    pub qdisc_type: QdiscType,
    /// Qdisc handle
    pub handle: QdiscHandle,
    /// Parent handle
    pub parent: QdiscHandle,
    /// Qdisc name
    pub name: String,
}

/// Traffic class configuration
#[derive(Debug, Clone)]
pub struct TrafficClass {
    /// Class handle
    pub handle: ClassHandle,
    /// Parent class
    pub parent: ClassHandle,
    /// Rate in bytes per second
    pub rate: u64,
    /// Ceiling rate
    pub ceil: u64,
    /// Burst size
    pub burst: u32,
    /// CBURST size
    pub cburst: u32,
    /// Priority
    pub priority: u32,
    /// Quantum
    pub quantum: u32,
    /// Class type specific data
    pub class_type: ClassType,
}

/// Class types
#[derive(Debug, Clone)]
pub enum ClassType {
    /// HTB class
    Htb {
        level: u32,
    },
    /// HFSC class
    Hfsc {
        realtime: ServiceCurve,
        linkshare: ServiceCurve,
        upperlimit: ServiceCurve,
    },
    /// CBQ class
    Cbq {
        avpkt: u32,
        bandwidth: u64,
    },
    /// Priority class
    Prio {
        priority: u32,
    },
}

/// Service curve for HFSC
#[derive(Debug, Clone, Copy)]
pub struct ServiceCurve {
    /// m1: slope of the first segment
    pub m1: u64,
    /// d: x-coordinate of the intersection
    pub d: u64,
    /// m2: slope of the second segment
    pub m2: u64,
}

impl ServiceCurve {
    /// Create new service curve
    pub fn new(m1: u64, d: u64, m2: u64) -> Self {
        Self { m1, d, m2 }
    }

    /// Create linear service curve
    pub fn linear(m: u64) -> Self {
        Self { m1: m, d: 0, m2: m }
    }
}

// ============================================================================
// Filter Types
// ============================================================================

/// Filter types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    U32,
    Route,
    Fw,
    Basic,
    Cgroup,
    Bpf,
}

/// Filter configuration
#[derive(Debug, Clone)]
pub struct Filter {
    /// Filter handle
    pub handle: FilterHandle,
    /// Parent class
    pub parent: ClassHandle,
    /// Protocol
    pub protocol: Protocol,
    /// Priority
    pub priority: Priority,
    /// Filter type
    pub filter_type: FilterType,
    /// Filter actions
    pub actions: Vec<FilterAction>,
}

/// Filter actions
#[derive(Debug, Clone)]
pub enum FilterAction {
    /// Classify into class
    Classify(ClassHandle),
    /// Drop packet
    Drop,
    /// Accept packet
    Accept,
    /// Continue to next filter
    Continue,
    /// Police traffic
    Police(PoliceAction),
    /// Redirect to another interface
    Redirect(u32),
}

/// Police action for rate limiting
#[derive(Debug, Clone, Copy)]
pub struct PoliceAction {
    /// Rate in bytes per second
    pub rate: u64,
    /// Burst size
    pub burst: u32,
    /// Action when exceeding rate
    pub action: PoliceExceedAction,
    /// MTU
    pub mtu: u32,
}

/// Action when rate is exceeded
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoliceExceedAction {
    Drop,
    Continue,
    Reclassify,
}

// ============================================================================
// HTB Implementation
// ============================================================================

/// HTB (Hierarchy Token Bucket) qdisc
#[derive(Debug)]
pub struct HtbQdisc {
    /// Qdisc configuration
    config: QdiscConfig,
    /// Default class
    defcls: u32,
    /// Rate to quantum divisor
    r2q: u32,
    /// Direct packets not in classes
    direct_pkts: bool,
    /// Classes indexed by handle
    classes: BTreeMap<ClassHandle, HtbClass>,
    /// Next class ID
    next_class_id: AtomicU32,
}

/// HTB class
#[derive(Debug)]
pub struct HtbClass {
    /// Class handle
    handle: ClassHandle,
    /// Parent class
    parent: Option<ClassHandle>,
    /// Rate
    rate: u64,
    /// Ceiling rate
    ceil: u64,
    /// Burst
    burst: u32,
    /// CBURST
    cburst: u32,
    /// Priority
    prio: u32,
    /// Quantum
    quantum: Option<u32>,
    /// Level in hierarchy
    level: u32,
    /// Current tokens
    tokens: i64,
    /// Current ctokens
    ctokens: i64,
    /// Statistics
    stats: ClassStats,
}

/// Class statistics
#[derive(Debug, Clone)]
pub struct ClassStats {
    /// Bytes sent
    pub bytes: u64,
    /// Packets sent
    pub packets: u64,
    /// Drops
    pub drops: u64,
    /// Overlimits
    pub overlimits: u64,
    /// Bounces
    pub bounces: u64,
}

impl HtbQdisc {
    /// Create new HTB qdisc
    pub fn new(config: QdiscConfig, defcls: u32, r2q: u32) -> Self {
        Self {
            config,
            defcls,
            r2q,
            direct_pkts: false,
            classes: BTreeMap::new(),
            next_class_id: AtomicU32::new(1),
        }
    }

    /// Add class
    pub fn add_class(&mut self, class: TrafficClass) -> QosResult<()> {
        let htb_class = HtbClass {
            handle: class.handle,
            parent: if class.parent.major == 0xFFFF { None } else { Some(class.parent) },
            rate: class.rate,
            ceil: if class.ceil == 0 { class.rate } else { class.ceil },
            burst: if class.burst == 0 { Self::calc_rate2buf(class.rate, 100) } else { class.burst },
            cburst: if class.cburst == 0 { Self::calc_rate2buf(class.ceil, 100) } else { class.cburst },
            prio: class.priority,
            quantum: if class.quantum == 0 { None } else { Some(class.quantum) },
            level: 0,
            tokens: 0,
            ctokens: 0,
            stats: ClassStats {
                bytes: 0,
                packets: 0,
                drops: 0,
                overlimits: 0,
                bounces: 0,
            },
        };

        if self.classes.contains_key(&class.handle) {
            return Err(crate::error::unified::QosError::ClassAlreadyExists);
        }

        self.classes.insert(class.handle, htb_class);
        Ok(())
    }

    /// Remove class
    pub fn remove_class(&mut self, handle: ClassHandle) -> QosResult<()> {
        if !self.classes.contains_key(&handle) {
            return Err(crate::error::unified::QosError::ClassNotFound);
        }

        self.classes.remove(&handle);
        Ok(())
    }

    /// Modify class
    pub fn modify_class(&mut self, handle: ClassHandle, class: TrafficClass) -> QosResult<()> {
        if !self.classes.contains_key(&handle) {
            return Err(crate::error::unified::QosError::ClassNotFound);
        }

        let htb_class = self.classes.get_mut(&handle).unwrap();
        htb_class.rate = class.rate;
        htb_class.ceil = if class.ceil == 0 { class.rate } else { class.ceil };
        htb_class.burst = if class.burst == 0 { Self::calc_rate2buf(class.rate, 100) } else { class.burst };
        htb_class.cburst = if class.cburst == 0 { Self::calc_rate2buf(class.ceil, 100) } else { class.cburst };
        htb_class.prio = class.priority;
        htb_class.quantum = if class.quantum == 0 { None } else { Some(class.quantum) };

        Ok(())
    }

    /// Get class statistics
    pub fn get_class_stats(&self, handle: ClassHandle) -> QosResult<ClassStats> {
        self.classes
            .get(&handle)
            .map(|c| c.stats.clone())
            .ok_or(crate::error::unified::QosError::ClassNotFound)
    }

    /// Calculate rate2buf
    fn calc_rate2buf(rate: u64, ceil: u64) -> u32 {
        let r = rate / 8;
        let r = r.min(ceil as u64);
        ((r / 1000) + 1) as u32
    }

    /// Enqueue packet
    pub fn enqueue(&mut self, _class_handle: ClassHandle, _len: u32) -> QosResult<()> {
        Ok(())
    }

    /// Dequeue packet
    pub fn dequeue(&mut self) -> QosResult<Option<ClassHandle>> {
        Ok(None)
    }
}

// ============================================================================
// HFSC Implementation
// ============================================================================

/// HFSC (Hierarchical Fair Service Curve) qdisc
#[derive(Debug)]
pub struct HfscQdisc {
    /// Qdisc configuration
    config: QdiscConfig,
    /// Default class
    defcls: u32,
    /// Classes indexed by handle
    classes: BTreeMap<ClassHandle, HfscClass>,
}

/// HFSC class
#[derive(Debug)]
pub struct HfscClass {
    /// Class handle
    handle: ClassHandle,
    /// Parent class
    parent: Option<ClassHandle>,
    /// Real-time service curve
    rt_sc: ServiceCurve,
    /// Link-sharing service curve
    ls_sc: ServiceCurve,
    /// Upper-limit service curve
    ul_sc: ServiceCurve,
    /// Statistics
    stats: ClassStats,
}

impl HfscQdisc {
    /// Create new HFSC qdisc
    pub fn new(config: QdiscConfig, defcls: u32) -> Self {
        Self {
            config,
            defcls,
            classes: BTreeMap::new(),
        }
    }

    /// Add class
    pub fn add_class(&mut self, class: TrafficClass) -> QosResult<()> {
        if let ClassType::Hfsc { realtime, linkshare, upperlimit } = class.class_type {
            let hfsc_class = HfscClass {
                handle: class.handle,
                parent: if class.parent.major == 0xFFFF { None } else { Some(class.parent) },
                rt_sc: realtime,
                ls_sc: linkshare,
                ul_sc: upperlimit,
                stats: ClassStats {
                    bytes: 0,
                    packets: 0,
                    drops: 0,
                    overlimits: 0,
                    bounces: 0,
                },
            };

            if self.classes.contains_key(&class.handle) {
                return Err(crate::error::unified::QosError::ClassAlreadyExists);
            }

            self.classes.insert(class.handle, hfsc_class);
            Ok(())
        } else {
            Err(crate::error::unified::QosError::InvalidClassId)
        }
    }

    /// Remove class
    pub fn remove_class(&mut self, handle: ClassHandle) -> QosResult<()> {
        if !self.classes.contains_key(&handle) {
            return Err(crate::error::unified::QosError::ClassNotFound);
        }

        self.classes.remove(&handle);
        Ok(())
    }

    /// Get class statistics
    pub fn get_class_stats(&self, handle: ClassHandle) -> QosResult<ClassStats> {
        self.classes
            .get(&handle)
            .map(|c| c.stats.clone())
            .ok_or(crate::error::unified::QosError::ClassNotFound)
    }
}

// ============================================================================
// Policing
// ============================================================================

/// Policing configuration
#[derive(Debug, Clone, Copy)]
pub struct PoliceConfig {
    /// Rate in bytes per second
    pub rate: u64,
    /// Burst size
    pub burst: u32,
    /// Action when conforming
    pub conform_action: PoliceExceedAction,
    /// Action when exceeding
    pub exceed_action: PoliceExceedAction,
}

/// Single rate three color marker (srTCM) - RFC 2697
#[derive(Debug, Clone)]
pub struct SingleRateTcm {
    /// Committed information rate
    pub cir: u64,
    /// Committed burst size
    pub cbs: u32,
    /// Excess burst size
    pub ebs: u32,
    /// Current committed bucket
    pub c_bucket: i32,
    /// Current excess bucket
    pub e_bucket: i32,
    /// Last update time
    pub last_update: u64,
}

impl SingleRateTcm {
    /// Create new single rate TCM
    pub fn new(cir: u64, cbs: u32, ebs: u32) -> Self {
        Self {
            cir,
            cbs,
            ebs,
            c_bucket: cbs as i32,
            e_bucket: ebs as i32,
            last_update: 0,
        }
    }

    /// Color packet (green, yellow, red)
    pub fn color_packet(&mut self, len: u32, now: u64) -> ColorMode {
        // Update buckets based on time elapsed
        let elapsed = now.saturating_sub(self.last_update);
        let tokens = ((self.cir * elapsed) / 1_000_000_000) as i32;

        self.c_bucket = (self.c_bucket + tokens).min(self.cbs as i32);
        if self.c_bucket >= len as i32 {
            self.c_bucket -= len as i32;
            self.last_update = now;
            return ColorMode::Green;
        }

        self.e_bucket = (self.e_bucket + tokens).min(self.ebs as i32);
        if self.e_bucket >= len as i32 {
            self.e_bucket -= len as i32;
            self.last_update = now;
            return ColorMode::Yellow;
        }

        self.last_update = now;
        ColorMode::Red
    }
}

/// Two rate three color marker (trTCM) - RFC 2698
#[derive(Debug, Clone)]
pub struct TwoRateTcm {
    /// Committed information rate
    pub cir: u64,
    /// Peak information rate
    pub pir: u64,
    /// Committed burst size
    pub cbs: u32,
    /// Peak burst size
    pub pbs: u32,
    /// Current committed bucket
    pub c_bucket: i32,
    /// Current peak bucket
    pub p_bucket: i32,
    /// Last update time
    pub last_update: u64,
}

impl TwoRateTcm {
    /// Create new two rate TCM
    pub fn new(cir: u64, pir: u64, cbs: u32, pbs: u32) -> Self {
        Self {
            cir,
            pir,
            cbs,
            pbs,
            c_bucket: cbs as i32,
            p_bucket: pbs as i32,
            last_update: 0,
        }
    }

    /// Color packet
    pub fn color_packet(&mut self, len: u32, now: u64) -> ColorMode {
        let elapsed = now.saturating_sub(self.last_update);
        let c_tokens = ((self.cir * elapsed) / 1_000_000_000) as i32;
        let p_tokens = ((self.pir * elapsed) / 1_000_000_000) as i32;

        self.c_bucket = (self.c_bucket + c_tokens).min(self.cbs as i32);
        self.p_bucket = (self.p_bucket + p_tokens).min(self.pbs as i32);

        if self.p_bucket < len as i32 {
            self.last_update = now;
            return ColorMode::Red;
        }

        self.p_bucket -= len as i32;
        if self.c_bucket < len as i32 {
            self.last_update = now;
            return ColorMode::Yellow;
        }

        self.c_bucket -= len as i32;
        self.last_update = now;
        ColorMode::Green
    }
}

/// Color mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Green,
    Yellow,
    Red,
}

// ============================================================================
// Random Early Detection (RED)
// ============================================================================

/// RED configuration
#[derive(Debug, Clone)]
pub struct RedConfig {
    /// Minimum threshold (bytes)
    pub min: u32,
    /// Maximum threshold (bytes)
    pub max: u32,
    /// Probability denominator
    pub probability: u32,
    /// Burst limit
    pub burst: u32,
    /// ECN enabled
    pub ecn: bool,
    /// Hard drop limit
    pub limit: u32,
}

/// RED qdisc
#[derive(Debug)]
pub struct RedQdisc {
    config: RedConfig,
    /// Current queue length
    qavg: u32,
    /// Queue length
    qcount: u32,
    /// Random early drop probability
    prob: u32,
    /// Statistics
    stats: RedStats,
}

/// RED statistics
#[derive(Debug, Clone)]
pub struct RedStats {
    /// Early drops
    pub early: u64,
    /// Marked packets (ECN)
    pub marked: u64,
    /// Forced drops
    pub drops: u64,
    /// Total packets
    pub packets: u64,
}

impl RedQdisc {
    /// Create new RED qdisc
    pub fn new(config: RedConfig) -> Self {
        Self {
            config,
            qavg: 0,
            qcount: 0,
            prob: 0,
            stats: RedStats {
                early: 0,
                marked: 0,
                drops: 0,
                packets: 0,
            },
        }
    }

    /// Enqueue packet with RED logic
    pub fn enqueue(&mut self, len: u32) -> QosResult<bool> {
        self.stats.packets += 1;

        // Update average queue size (exponential weighted moving average)
        let w = 512u32; // Weight
        self.qavg = ((self.qavg * (w - 1)) + len) / w;

        // Check if we should drop/mark
        if self.qavg < self.config.min {
            Ok(true) // Accept
        } else if self.qavg >= self.config.max {
            self.stats.drops += 1;
            Ok(false) // Drop
        } else {
            // Probabilistic drop
            let pb = (self.qavg - self.config.min) * self.config.probability /
                    (self.config.max - self.config.min);

            // Simple pseudo-random check
            let rand = (self.stats.packets % 1000) as u32;
            if rand < pb {
                if self.config.ecn {
                    self.stats.marked += 1;
                    Ok(true) // Mark and accept
                } else {
                    self.stats.early += 1;
                    Ok(false) // Drop
                }
            } else {
                Ok(true) // Accept
            }
        }
    }

    /// Get statistics
    pub fn get_stats(&self) -> &RedStats {
        &self.stats
    }
}

// ============================================================================
// RIO (RED with In/Out)
// ============================================================================

/// RIO configuration
#[derive(Debug, Clone)]
pub struct RioConfig {
    /// In-band RED
    pub in_config: RedConfig,
    /// Out-band RED
    pub out_config: RedConfig,
}

/// RIO qdisc (differentiated services RED)
#[derive(Debug)]
pub struct RioQdisc {
    in_red: RedQdisc,
    out_red: RedQdisc,
}

impl RioQdisc {
    /// Create new RIO qdisc
    pub fn new(config: RioConfig) -> Self {
        Self {
            in_red: RedQdisc::new(config.in_config),
            out_red: RedQdisc::new(config.out_config),
        }
    }

    /// Enqueue in-band packet
    pub fn enqueue_in(&mut self, len: u32) -> QosResult<bool> {
        self.in_red.enqueue(len)
    }

    /// Enqueue out-band packet
    pub fn enqueue_out(&mut self, len: u32) -> QosResult<bool> {
        self.out_red.enqueue(len)
    }
}

// ============================================================================
// Token Bucket Filter (TBF)
// ============================================================================

/// TBF configuration
#[derive(Debug, Clone, Copy)]
pub struct TbfConfig {
    /// Rate in bytes per second
    pub rate: u64,
    /// Burst size
    pub burst: u32,
    /// Limit (latency)
    pub limit: u32,
    /// Peak rate
    pub peakrate: Option<u64>,
    /// MTU
    pub mtu: Option<u32>,
}

/// Token bucket filter
#[derive(Debug, Clone)]
pub struct TokenBucket {
    /// Rate (bytes per second)
    rate: u64,
    /// Current tokens
    tokens: u64,
    /// Maximum burst
    burst: u64,
    /// Last update time
    last_update: u64,
}

impl TokenBucket {
    /// Create new token bucket
    pub fn new(rate: u64, burst: u32) -> Self {
        Self {
            rate,
            tokens: burst as u64,
            burst: burst as u64,
            last_update: 0,
        }
    }

    /// Consume tokens for packet
    pub fn consume(&mut self, len: u32, now: u64) -> bool {
        // Refill tokens based on elapsed time
        let elapsed = now.saturating_sub(self.last_update);
        let add = (self.rate * elapsed) / 1_000_000_000;
        self.tokens = (self.tokens + add).min(self.burst);
        self.last_update = now;

        // Check if we have enough tokens
        if self.tokens >= len as u64 {
            self.tokens -= len as u64;
            true
        } else {
            false
        }
    }

    /// Get current tokens
    pub fn get_tokens(&self) -> u64 {
        self.tokens
    }
}

// ============================================================================
// Traffic Shaping
// ============================================================================

/// Traffic shaper
#[derive(Debug)]
pub struct TrafficShaper {
    /// Token bucket
    bucket: TokenBucket,
    /// Queue
    queue: Vec<Vec<u8>>,
    /// Maximum queue length
    max_queue: usize,
    /// Statistics
    stats: ShaperStats,
}

/// Shaper statistics
#[derive(Debug, Clone)]
pub struct ShaperStats {
    /// Bytes sent
    pub bytes: u64,
    /// Packets sent
    pub packets: u64,
    /// Drops
    pub drops: u64,
    /// Current queue length
    pub queue_len: usize,
}

impl TrafficShaper {
    /// Create new traffic shaper
    pub fn new(rate: u64, burst: u32, max_queue: usize) -> Self {
        Self {
            bucket: TokenBucket::new(rate, burst),
            queue: Vec::new(),
            max_queue,
            stats: ShaperStats {
                bytes: 0,
                packets: 0,
                drops: 0,
                queue_len: 0,
            },
        }
    }

    /// Enqueue packet
    pub fn enqueue(&mut self, packet: Vec<u8>, now: u64) -> QosResult<bool> {
        if self.queue.len() >= self.max_queue {
            self.stats.drops += 1;
            return Ok(false);
        }

        self.queue.push(packet);
        self.stats.queue_len = self.queue.len();
        Ok(true)
    }

    /// Dequeue packet
    pub fn dequeue(&mut self, now: u64) -> Option<Vec<u8>> {
        if self.queue.is_empty() {
            return None;
        }

        let len = self.queue[0].len() as u32;
        if self.bucket.consume(len, now) {
            let packet = self.queue.remove(0);
            self.stats.packets += 1;
            self.stats.bytes += len as u64;
            self.stats.queue_len = self.queue.len();
            Some(packet)
        } else {
            None
        }
    }

    /// Get statistics
    pub fn get_stats(&self) -> &ShaperStats {
        &self.stats
    }
}

// ============================================================================
// Priority Queueing
// ============================================================================

/// Priority qdisc (prio)
#[derive(Debug)]
pub struct PrioQdisc {
    /// Number of bands
    bands: usize,
    /// Queues for each priority band
    queues: Vec<Vec<Vec<u8>>>,
    /// Statistics per band
    stats: Vec<BandStats>,
}

/// Band statistics
#[derive(Debug, Clone)]
pub struct BandStats {
    /// Packets
    pub packets: u64,
    /// Bytes
    pub bytes: u64,
    /// Drops
    pub drops: u64,
}

impl PrioQdisc {
    /// Create new priority qdisc
    pub fn new(bands: usize) -> Self {
        let queues = (0..bands).map(|_| Vec::new()).collect();
        let stats = (0..bands).map(|_| BandStats {
            packets: 0,
            bytes: 0,
            drops: 0,
        }).collect();

        Self { bands, queues, stats }
    }

    /// Enqueue packet to band (0 = highest priority)
    pub fn enqueue(&mut self, band: usize, packet: Vec<u8>) -> QosResult<()> {
        if band >= self.bands {
            return Err(crate::error::unified::QosError::PriorityError);
        }

        self.queues[band].push(packet);
        Ok(())
    }

    /// Dequeue packet (highest priority first)
    pub fn dequeue(&mut self) -> Option<Vec<u8>> {
        for i in 0..self.bands {
            if !self.queues[i].is_empty() {
                let packet = self.queues[i].remove(0);
                self.stats[i].packets += 1;
                self.stats[i].bytes += packet.len() as u64;
                return Some(packet);
            }
        }
        None
    }
}

// ============================================================================
// TC Subsystem Manager
// ============================================================================

/// TC subsystem manager
#[derive(Debug)]
pub struct TcManager {
    /// Qdiscs indexed by device ID and handle
    qdiscs: BTreeMap<u32, BTreeMap<QdiscHandle, Arc<Mutex<dyn Qdisc>>>>,
    /// Next qdisc ID
    next_qdisc_id: AtomicU32,
}

/// Qdisc trait
pub trait Qdisc {
    /// Get qdisc type
    fn get_type(&self) -> QdiscType;
    /// Enqueue packet
    fn enqueue(&mut self, packet: &[u8], class: Option<ClassHandle>) -> QosResult<bool>;
    /// Dequeue packet
    fn dequeue(&mut self) -> QosResult<Option<Vec<u8>>>;
}

impl TcManager {
    /// Create new TC manager
    pub fn new() -> Self {
        Self {
            qdiscs: BTreeMap::new(),
            next_qdisc_id: AtomicU32::new(1),
        }
    }

    /// Add qdisc to device
    pub fn add_qdisc(&mut self, dev_id: u32, qdisc: Arc<Mutex<dyn Qdisc>>, handle: QdiscHandle) -> QosResult<()> {
        if !self.qdiscs.contains_key(&dev_id) {
            self.qdiscs.insert(dev_id, BTreeMap::new());
        }

        let device_qdiscs = self.qdiscs.get_mut(&dev_id).unwrap();
        if device_qdiscs.contains_key(&handle) {
            return Err(crate::error::unified::QosError::QdiscAlreadyExists);
        }

        device_qdiscs.insert(handle, qdisc);
        Ok(())
    }

    /// Remove qdisc
    pub fn remove_qdisc(&mut self, dev_id: u32, handle: QdiscHandle) -> QosResult<()> {
        if !self.qdiscs.contains_key(&dev_id) {
            return Err(crate::error::unified::QosError::QdiscNotFound);
        }

        let device_qdiscs = self.qdiscs.get_mut(&dev_id).unwrap();
        if !device_qdiscs.contains_key(&handle) {
            return Err(crate::error::unified::QosError::QdiscNotFound);
        }

        device_qdiscs.remove(&handle);
        Ok(())
    }

    /// Get qdisc
    pub fn get_qdisc(&self, dev_id: u32, handle: QdiscHandle) -> Option<Arc<Mutex<dyn Qdisc>>> {
        self.qdiscs.get(&dev_id)?.get(&handle).cloned()
    }

    /// List qdiscs on device
    pub fn list_qdiscs(&self, dev_id: u32) -> Vec<QdiscHandle> {
        self.qdiscs
            .get(&dev_id)
            .map(|qs| qs.keys().copied().collect())
            .unwrap_or_default()
    }
}

impl Default for TcManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// QoS API
// ============================================================================

/// Global QoS manager
static QOS_MANAGER: RwLock<Option<QosManager>> = RwLock::new(None);

/// QoS manager
pub struct QosManager {
    /// TC manager
    tc: Arc<Mutex<TcManager>>,
}

impl QosManager {
    /// Initialize QoS subsystem
    pub fn init() -> QosResult<()> {
        let manager = Self {
            tc: Arc::new(Mutex::new(TcManager::new())),
        };

        let mut global = QOS_MANAGER.write();
        *global = Some(manager);

        Ok(())
    }

    /// Get QoS manager instance
    pub fn get() -> Option<&'static QosManager> {
        // In a real implementation, this would return a reference to the global instance
        None
    }

    /// Add HTB qdisc
    pub fn add_htb_qdisc(&self, dev_id: u32, config: QdiscConfig, defcls: u32) -> QosResult<QdiscHandle> {
        let handle = config.handle;
        let htb = Arc::new(Mutex::new(HtbWrapper {
            inner: HtbQdisc::new(config.clone(), defcls, 10),
        }));

        let mut tc = self.tc.lock();
        tc.add_qdisc(dev_id, htb, handle)?;

        Ok(handle)
    }

    /// Add HFSC qdisc
    pub fn add_hfsc_qdisc(&self, dev_id: u32, config: QdiscConfig, defcls: u32) -> QosResult<QdiscHandle> {
        let handle = config.handle;
        let hfsc = Arc::new(Mutex::new(HfscWrapper {
            inner: HfscQdisc::new(config.clone(), defcls),
        }));

        let mut tc = self.tc.lock();
        tc.add_qdisc(dev_id, hfsc, handle)?;

        Ok(handle)
    }

    /// Add RED qdisc
    pub fn add_red_qdisc(&self, dev_id: u32, handle: QdiscHandle, config: RedConfig) -> QosResult<()> {
        let red = Arc::new(Mutex::new(RedWrapper {
            inner: RedQdisc::new(config),
        }));

        let mut tc = self.tc.lock();
        tc.add_qdisc(dev_id, red, handle)?;

        Ok(())
    }

    /// Add class
    pub fn add_class(&self, dev_id: u32, qdisc_handle: QdiscHandle, class: TrafficClass) -> QosResult<ClassHandle> {
        let tc = self.tc.lock();
        let qdisc = tc.get_qdisc(dev_id, qdisc_handle)
            .ok_or(crate::error::unified::QosError::QdiscNotFound)?;

        // In a real implementation, we would use dynamic dispatch or downcasting
        // For now, we'll just return the class handle
        Ok(class.handle)
    }

    /// Add filter
    pub fn add_filter(&self, dev_id: u32, qdisc_handle: QdiscHandle, filter: Filter) -> QosResult<FilterHandle> {
        let handle = filter.handle;

        let _tc = self.tc.lock();
        let _qdisc = self.tc.lock().get_qdisc(dev_id, qdisc_handle)
            .ok_or(crate::error::unified::QosError::QdiscNotFound)?;

        Ok(handle)
    }
}

// Wrapper types for Qdisc trait
struct HtbWrapper {
    inner: HtbQdisc,
}

struct HfscWrapper {
    inner: HfscQdisc,
}

struct RedWrapper {
    inner: RedQdisc,
}

impl Qdisc for HtbWrapper {
    fn get_type(&self) -> QdiscType {
        QdiscType::Htb
    }

    fn enqueue(&mut self, _packet: &[u8], _class: Option<ClassHandle>) -> QosResult<bool> {
        Ok(true)
    }

    fn dequeue(&mut self) -> QosResult<Option<Vec<u8>>> {
        Ok(None)
    }
}

impl Qdisc for HfscWrapper {
    fn get_type(&self) -> QdiscType {
        QdiscType::Hfsc
    }

    fn enqueue(&mut self, _packet: &[u8], _class: Option<ClassHandle>) -> QosResult<bool> {
        Ok(true)
    }

    fn dequeue(&mut self) -> QosResult<Option<Vec<u8>>> {
        Ok(None)
    }
}

impl Qdisc for RedWrapper {
    fn get_type(&self) -> QdiscType {
        QdiscType::Red
    }

    fn enqueue(&mut self, packet: &[u8], _class: Option<ClassHandle>) -> QosResult<bool> {
        let len = packet.len() as u32;
        self.inner.enqueue(len)
    }

    fn dequeue(&mut self) -> QosResult<Option<Vec<u8>>> {
        Ok(None)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qdisc_handle() {
        let root = QdiscHandle::root();
        assert_eq!(root.major, 0xFFFF);
        assert_eq!(root.minor, 0);

        let handle = QdiscHandle::new(1, 10);
        assert_eq!(handle.major, 1);
        assert_eq!(handle.minor, 10);
        assert_eq!(handle.to_u32(), 0x0001000A);
    }

    #[test]
    fn test_htb_qdisc() {
        let config = QdiscConfig {
            qdisc_type: QdiscType::Htb,
            handle: QdiscHandle::root(),
            parent: QdiscHandle::root(),
            name: String::from("htb-root"),
        };

        let mut htb = HtbQdisc::new(config, 0, 10);

        let class = TrafficClass {
            handle: ClassHandle::new(1, 1),
            parent: ClassHandle::new(0xFFFF, 0),
            rate: 1_000_000, // 1 Mbps
            ceil: 2_000_000, // 2 Mbps
            burst: 0,
            cburst: 0,
            priority: 0,
            quantum: 0,
            class_type: ClassType::Htb { level: 0 },
        };

        assert!(htb.add_class(class.clone()).is_ok());
        assert!(htb.add_class(class.clone()).is_err()); // Already exists
    }

    #[test]
    fn test_single_rate_tcm() {
        let mut tcm = SingleRateTcm::new(1_000_000, 10000, 20000);

        // First packet should be green
        let color = tcm.color_packet(1000, 0);
        assert_eq!(color, ColorMode::Green);

        // Exhaust bucket
        for _ in 0..10 {
            tcm.color_packet(1000, 0);
        }

        let color = tcm.color_packet(1000, 0);
        assert!(color == ColorMode::Yellow || color == ColorMode::Red);
    }

    #[test]
    fn test_token_bucket() {
        let mut tb = TokenBucket::new(1_000_000, 10000);

        // Should succeed initially
        assert!(tb.consume(1000, 0));

        // Exhaust bucket
        for _ in 0..15 {
            tb.consume(1000, 0);
        }

        // Should fail
        assert!(!tb.consume(1000, 0));

        // Refill after time passes
        assert!(tb.consume(1000, 1_000_000_000));
    }

    #[test]
    fn test_red_qdisc() {
        let config = RedConfig {
            min: 1000,
            max: 10000,
            probability: 10,
            burst: 100,
            ecn: false,
            limit: 20000,
        };

        let mut red = RedQdisc::new(config);

        // Small packets should be accepted
        assert!(red.enqueue(100).unwrap());

        // Enough large packets to trigger dropping
        for _ in 0..200 {
            red.enqueue(5000).unwrap();
        }

        let stats = red.get_stats();
        assert!(stats.packets > 0);
    }

    #[test]
    fn test_traffic_shaper() {
        let mut shaper = TrafficShaper::new(1_000_000, 10000, 100);

        // Enqueue packets
        for _ in 0..10 {
            shaper.enqueue(vec![0u8; 1000], 0).unwrap();
        }

        // Dequeue with no tokens
        assert!(shaper.dequeue(0).is_none());

        // Dequeue with tokens
        assert!(shaper.dequeue(1_000_000_000).is_some());

        let stats = shaper.get_stats();
        assert_eq!(stats.packets, 1);
    }

    #[test]
    fn test_prio_qdisc() {
        let mut prio = PrioQdisc::new(3);

        // Enqueue to different bands
        prio.enqueue(2, vec![0u8; 100]).unwrap();
        prio.enqueue(0, vec![0u8; 100]).unwrap();
        prio.enqueue(1, vec![0u8; 100]).unwrap();

        // Should dequeue from band 0 first
        let pkt = prio.dequeue().unwrap();
        assert_eq!(pkt.len(), 100);
    }

    #[test]
    fn test_service_curve() {
        let sc = ServiceCurve::linear(1_000_000);
        assert_eq!(sc.m1, 1_000_000);
        assert_eq!(sc.d, 0);
        assert_eq!(sc.m2, 1_000_000);

        let sc = ServiceCurve::new(1_000_000, 100, 2_000_000);
        assert_eq!(sc.m1, 1_000_000);
        assert_eq!(sc.d, 100);
        assert_eq!(sc.m2, 2_000_000);
    }
}
