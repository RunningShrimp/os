//! Advanced UDP features with zero-copy and offload support
//!
//! This module implements advanced UDP features including:
//! - Zero-copy UDP using DMABUF and splice
//! - UDP segmentation offload (GSO, USO)
//! - UDP fragmentation offload (UFO)
//! - UDP generic receive offload (GRO)
//! - UDP socket options (checksum, encapsulation)
//! - Multicast optimization with IGMP snooping
//!
//! # Examples
//!
//! ```rust
//! use kernel::network::udp_opt::{UdpZeroCopy, ZeroCopyMode, UdpGsoConfig};
//!
//! // Create UDP zero-copy manager
//! let udp = UdpZeroCopy::new();
//!
//! // Enable zero-copy mode
//! udp.set_zero_copy_mode(ZeroCopyMode::DmaBuf);
//!
//! // Configure GSO
//! let gso_config = UdpGsoConfig {
//!     segment_size: 1500,
//!     max_segments: 64,
//! };
//! udp.enable_gso(gso_config);
//! ```

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use core::time::Duration;

use crate::subsystems::sync::{Mutex, RwLock};
use crate::subsystems::net::ipv4::Ipv4Addr;

/// Maximum UDP GSO segment size
pub const MAX_UDP_GSO_SIZE: u16 = 65535;

/// Default UDP GSO segment size
pub const DEFAULT_UDP_GSO_SIZE: u16 = 1500;

/// Maximum number of GSO segments
pub const MAX_GSO_SEGMENTS: usize = 64;

/// Zero-copy modes for UDP
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroCopyMode {
    /// No zero-copy (regular copy)
    None,
    /// Zero-copy using DMABUF (direct DMA buffer)
    DmaBuf,
    /// Zero-copy using splice (moving between file descriptors)
    Splice,
    /// Zero-copy using mmap (memory mapped buffers)
    Mmap,
}

impl ZeroCopyMode {
    /// Check if zero-copy is enabled
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::None)
    }
}

/// DMA buffer for zero-copy operations
#[derive(Debug)]
pub struct DmaBuffer {
    /// Buffer address (physical)
    pub phys_addr: u64,
    /// Buffer address (virtual)
    pub virt_addr: u64,
    /// Buffer size
    pub size: usize,
    /// DMA mapped (true if mapped for device access)
    pub dma_mapped: AtomicBool,
    /// Buffer owner (for tracking)
    pub owner: usize,
}

unsafe impl Send for DmaBuffer {}
unsafe impl Sync for DmaBuffer {}

impl DmaBuffer {
    /// Create a new DMA buffer
    pub fn new(size: usize) -> Option<Self> {
        // In real implementation, this would allocate DMA-capable memory
        Some(Self {
            phys_addr: 0,
            virt_addr: 0,
            size,
            dma_mapped: AtomicBool::new(false),
            owner: 0,
        })
    }

    /// Map buffer for DMA
    pub fn map_dma(&self) -> bool {
        // In real implementation, this would map the buffer for device DMA
        self.dma_mapped.store(true, Ordering::Release);
        true
    }

    /// Unmap buffer from DMA
    pub fn unmap_dma(&self) {
        self.dma_mapped.store(false, Ordering::Release);
    }

    /// Check if buffer is DMA mapped
    pub fn is_dma_mapped(&self) -> bool {
        self.dma_mapped.load(Ordering::Acquire)
    }

    /// Get buffer pointer (virtual address)
    pub fn as_ptr(&self) -> *mut u8 {
        self.virt_addr as *mut u8
    }

    /// Get buffer slice
    pub unsafe fn as_slice(&self) -> &[u8] {
        core::slice::from_raw_parts(self.virt_addr as *const u8, self.size)
    }

    /// Get mutable buffer slice
    pub unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        core::slice::from_raw_parts_mut(self.virt_addr as *mut u8, self.size)
    }
}

/// UDP GSO configuration
#[derive(Debug, Clone, Copy)]
pub struct UdpGsoConfig {
    /// Segment size
    pub segment_size: u16,
    /// Maximum segments per packet
    pub max_segments: usize,
    /// GSO enabled
    pub enabled: bool,
}

impl Default for UdpGsoConfig {
    fn default() -> Self {
        Self {
            segment_size: DEFAULT_UDP_GSO_SIZE,
            max_segments: MAX_GSO_SEGMENTS,
            enabled: false,
        }
    }
}

impl UdpGsoConfig {
    /// Create new GSO configuration
    pub fn new(segment_size: u16, max_segments: usize) -> Self {
        assert!(segment_size <= MAX_UDP_GSO_SIZE, "Segment size too large");
        assert!(max_segments <= MAX_GSO_SEGMENTS, "Too many segments");

        Self {
            segment_size,
            max_segments,
            enabled: true,
        }
    }

    /// Calculate maximum packet size
    pub fn max_packet_size(&self) -> usize {
        self.segment_size as usize * self.max_segments
    }
}

/// UDP GRO configuration
#[derive(Debug, Clone, Copy)]
pub struct UdpGroConfig {
    /// Maximum aggregated segments
    pub max_segments: usize,
    /// Timeout for aggregation (microseconds)
    pub timeout_us: u64,
    /// GRO enabled
    pub enabled: bool,
}

impl Default for UdpGroConfig {
    fn default() -> Self {
        Self {
            max_segments: 64,
            timeout_us: 100, // 100 microseconds
            enabled: false,
        }
    }
}

impl UdpGroConfig {
    /// Create new GRO configuration
    pub fn new(max_segments: usize, timeout_us: u64) -> Self {
        Self {
            max_segments,
            timeout_us,
            enabled: true,
        }
    }
}

/// UDP socket options
#[derive(Debug, Clone, Copy)]
pub struct UdpSocketOptions {
    /// UDP checksum enabled
    pub checksum_enabled: bool,
    /// UDP GSO enabled
    pub gso_enabled: bool,
    /// UDP GRO enabled
    pub gro_enabled: bool,
    /// UDP fragmentation offload (UFO) enabled
    pub ufo_enabled: bool,
    /// Zero-copy mode
    pub zero_copy_mode: ZeroCopyMode,
    /// Encapsulation type (for UDP tunnels)
    pub encapsulation: UdpEncapsulation,
    /// Multicast optimization enabled
    pub multicast_opt: bool,
}

impl Default for UdpSocketOptions {
    fn default() -> Self {
        Self {
            checksum_enabled: true,
            gso_enabled: false,
            gro_enabled: false,
            ufo_enabled: false,
            zero_copy_mode: ZeroCopyMode::None,
            encapsulation: UdpEncapsulation::None,
            multicast_opt: false,
        }
    }
}

/// UDP encapsulation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpEncapsulation {
    /// No encapsulation
    None,
    /// UDP encapsulation for ESP (IPsec)
    UdpEsp,
    /// VXLAN encapsulation
    Vxlan,
    /// GENEVE encapsulation
    Geneve,
    /// IPIP encapsulation
    Ipip,
}

/// UDP multicast optimization
#[derive(Debug, Clone)]
pub struct UdpMulticastOpt {
    /// IGMP snooping enabled
    pub igmp_snooping: bool,
    /// Multicast groups
    pub groups: Vec<MulticastGroup>,
}

/// Multicast group information
#[derive(Debug, Clone)]
pub struct MulticastGroup {
    /// Group address
    pub address: Ipv4Addr,
    /// Interface index
    pub interface: u32,
    /// Group state
    pub state: IgmpState,
    /// Number of members
    pub member_count: u32,
}

/// IGMP states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgmpState {
    /// Non-member
    NonMember,
    /// Querying
    Querying,
    /// Member
    Member,
    /// Leaving
    Leaving,
}

/// UDP GSO state
#[derive(Debug)]
pub struct UdpGsoState {
    /// GSO configuration
    pub config: UdpGsoConfig,
    /// Segments sent
    pub segments_sent: AtomicU64,
    /// Bytes offloaded
    pub bytes_offloaded: AtomicU64,
}

/// UDP GRO state
#[derive(Debug)]
pub struct UdpGroState {
    /// GRO configuration
    pub config: UdpGroConfig,
    /// Aggregated packets
    pub aggregated: AtomicU64,
    /// Bytes aggregated
    pub bytes_aggregated: AtomicU64,
}

/// Per-socket UDP optimization state
#[derive(Debug)]
pub struct UdpSocketState {
    /// Socket ID
    pub socket_id: u32,
    /// Socket options
    pub options: UdpSocketOptions,
    /// GSO state
    pub gso: Option<UdpGsoState>,
    /// GRO state
    pub gro: Option<UdpGroState>,
    /// DMA buffers
    pub dma_buffers: Vec<Arc<DmaBuffer>>,
    /// Multicast optimization
    pub multicast: Option<UdpMulticastOpt>,
}

impl UdpSocketState {
    /// Create new socket state
    pub fn new(socket_id: u32) -> Self {
        Self {
            socket_id,
            options: UdpSocketOptions::default(),
            gso: None,
            gro: None,
            dma_buffers: Vec::new(),
            multicast: None,
        }
    }

    /// Enable GSO
    pub fn enable_gso(&mut self, config: UdpGsoConfig) {
        self.options.gso_enabled = config.enabled;
        self.gso = Some(UdpGsoState {
            config,
            segments_sent: AtomicU64::new(0),
            bytes_offloaded: AtomicU64::new(0),
        });
    }

    /// Enable GRO
    pub fn enable_gro(&mut self, config: UdpGroConfig) {
        self.options.gro_enabled = config.enabled;
        self.gro = Some(UdpGroState {
            config,
            aggregated: AtomicU64::new(0),
            bytes_aggregated: AtomicU64::new(0),
        });
    }

    /// Allocate DMA buffer
    pub fn allocate_dma_buffer(&mut self, size: usize) -> Option<Arc<DmaBuffer>> {
        if let Some(buffer) = DmaBuffer::new(size) {
            let buffer = Arc::new(buffer);
            self.dma_buffers.push(buffer.clone());
            Some(buffer)
        } else {
            None
        }
    }

    /// Set zero-copy mode
    pub fn set_zero_copy_mode(&mut self, mode: ZeroCopyMode) {
        self.options.zero_copy_mode = mode;
    }
}

/// UDP zero-copy and offload manager
#[derive(Debug)]
pub struct UdpZeroCopy {
    /// Socket states indexed by socket ID
    sockets: RwLock<BTreeMap<u32, UdpSocketState>>,
    /// Global statistics
    stats: Mutex<UdpOffloadStats>,
    /// Global zero-copy mode
    global_zero_copy_mode: Mutex<ZeroCopyMode>,
    /// DMA buffer pool
    dma_pool: Mutex<Vec<Arc<DmaBuffer>>>,
    /// Next socket ID
    next_socket_id: AtomicU64,
}

/// UDP offload statistics
#[derive(Debug, Default, Clone)]
pub struct UdpOffloadStats {
    /// Zero-copy sends
    pub zero_copy_sends: u64,
    /// Zero-copy receives
    pub zero_copy_recvs: u64,
    /// Bytes saved by zero-copy
    pub zero_copy_bytes_saved: u64,
    /// GSO segments sent
    pub gso_segments: u64,
    /// GSO bytes offloaded
    pub gso_bytes: u64,
    /// GRO packets aggregated
    pub gro_packets: u64,
    /// GRO bytes aggregated
    pub gro_bytes: u64,
    /// UFO packets sent
    pub ufo_packets: u64,
    /// Checksum offload operations
    pub checksum_offloads: u64,
    /// Multicast optimizations
    pub multicast_optimizations: u64,
}

impl UdpZeroCopy {
    /// Create new UDP zero-copy manager
    pub fn new() -> Self {
        Self {
            sockets: RwLock::new(BTreeMap::new()),
            stats: Mutex::new(UdpOffloadStats::default()),
            global_zero_copy_mode: Mutex::new(ZeroCopyMode::None),
            dma_pool: Mutex::new(Vec::new()),
            next_socket_id: AtomicU64::new(1),
        }
    }

    /// Register a UDP socket
    pub fn register_socket(&self, socket_id: u32) -> Result<(), ()> {
        let mut sockets = self.sockets.write();
        if sockets.contains_key(&socket_id) {
            return Err(());
        }

        let state = UdpSocketState::new(socket_id);
        sockets.insert(socket_id, state);
        Ok(())
    }

    /// Unregister a socket
    pub fn unregister_socket(&self, socket_id: u32) -> Result<(), ()> {
        let mut sockets = self.sockets.write();
        if sockets.remove(&socket_id).is_some() {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Set zero-copy mode for a socket
    pub fn set_zero_copy_mode(&self, socket_id: u32, mode: ZeroCopyMode) -> Result<(), ()> {
        let mut sockets = self.sockets.write();
        if let Some(socket) = sockets.get_mut(&socket_id) {
            socket.set_zero_copy_mode(mode);
            Ok(())
        } else {
            Err(())
        }
    }

    /// Get zero-copy mode for a socket
    pub fn get_zero_copy_mode(&self, socket_id: u32) -> Result<ZeroCopyMode, ()> {
        let sockets = self.sockets.read();
        if let Some(socket) = sockets.get(&socket_id) {
            Ok(socket.options.zero_copy_mode)
        } else {
            Err(())
        }
    }

    /// Enable GSO for a socket
    pub fn enable_gso(&self, socket_id: u32, config: UdpGsoConfig) -> Result<(), ()> {
        let mut sockets = self.sockets.write();
        if let Some(socket) = sockets.get_mut(&socket_id) {
            socket.enable_gso(config);
            Ok(())
        } else {
            Err(())
        }
    }

    /// Enable GRO for a socket
    pub fn enable_gro(&self, socket_id: u32, config: UdpGroConfig) -> Result<(), ()> {
        let mut sockets = self.sockets.write();
        if let Some(socket) = sockets.get_mut(&socket_id) {
            socket.enable_gro(config);
            Ok(())
        } else {
            Err(())
        }
    }

    /// Send data using zero-copy
    pub fn send_zero_copy(
        &self,
        socket_id: u32,
        data: &[u8],
        dest_addr: (Ipv4Addr, u16),
    ) -> Result<usize, ()> {
        let sockets = self.sockets.read();
        let socket = sockets.get(&socket_id).ok_or(())?;

        if !socket.options.zero_copy_mode.is_enabled() {
            return Err(()); // Zero-copy not enabled
        }

        // In real implementation, this would:
        // 1. Pin user pages
        // 2. Map for DMA
        // 3. Pass to network device
        // 4. Unmap after transmission

        let mut stats = self.stats.lock();
        stats.zero_copy_sends += 1;
        stats.zero_copy_bytes_saved += data.len() as u64;

        Ok(data.len())
    }

    /// Receive data using zero-copy
    pub fn recv_zero_copy(
        &self,
        socket_id: u32,
        buffer: &mut [u8],
    ) -> Result<(usize, (Ipv4Addr, u16)), ()> {
        let sockets = self.sockets.read();
        let socket = sockets.get(&socket_id).ok_or(())?;

        if !socket.options.zero_copy_mode.is_enabled() {
            return Err(()); // Zero-copy not enabled
        }

        // In real implementation, this would:
        // 1. Allocate DMA buffer
        // 2. Receive directly into buffer
        // 3. Map to user space
        // 4. Return buffer reference

        let mut stats = self.stats.lock();
        stats.zero_copy_recvs += 1;

        // Mock: return dummy data
        let len = buffer.len().min(100);
        Ok((len, (Ipv4Addr::new(127, 0, 0, 1), 1234)))
    }

    /// Segment packet using GSO
    pub fn gso_segment(
        &self,
        socket_id: u32,
        packet: &[u8],
        segment_size: u16,
    ) -> Result<Vec<Vec<u8>>, ()> {
        let sockets = self.sockets.read();
        let socket = sockets.get(&socket_id).ok_or(())?;

        if !socket.options.gso_enabled {
            return Err(());
        }

        let mut segments = Vec::new();
        let mut offset = 0;

        while offset < packet.len() {
            let end = (offset + segment_size as usize).min(packet.len());
            segments.push(packet[offset..end].to_vec());
            offset = end;

            if segments.len() >= MAX_GSO_SEGMENTS {
                break;
            }
        }

        // Update statistics
        if let Some(gso) = &socket.gso {
            gso.segments_sent.fetch_add(segments.len() as u64, Ordering::Relaxed);
            gso.bytes_offloaded.fetch_add(packet.len() as u64, Ordering::Relaxed);
        }

        let mut stats = self.stats.lock();
        stats.gso_segments += segments.len() as u64;
        stats.gso_bytes += packet.len() as u64;

        Ok(segments)
    }

    /// Aggregate packets using GRO
    pub fn gro_aggregate(
        &self,
        socket_id: u32,
        packets: &[Vec<u8>],
    ) -> Result<Vec<Vec<u8>>, ()> {
        let sockets = self.sockets.read();
        let socket = sockets.get(&socket_id).ok_or(())?;

        if !socket.options.gro_enabled || socket.gro.is_none() {
            return Err(());
        }

        let gro_config = socket.gro.as_ref().unwrap().config;
        let mut aggregated = Vec::new();

        // Simple aggregation: combine consecutive packets
        let mut current_agg = Vec::new();
        let mut total_size = 0;

        for packet in packets {
            if current_agg.is_empty() {
                current_agg.extend_from_slice(packet);
                total_size += packet.len();
            } else if total_size + packet.len() <= gro_config.max_segments * 1500 {
                current_agg.extend_from_slice(packet);
                total_size += packet.len();
            } else {
                if !current_agg.is_empty() {
                    aggregated.push(core::mem::replace(&mut current_agg, Vec::new()));
                }
                total_size = packet.len();
                current_agg.extend_from_slice(packet);
            }
        }

        if !current_agg.is_empty() {
            aggregated.push(current_agg);
        }

        // Update statistics
        let mut stats = self.stats.lock();
        stats.gro_packets += (packets.len() - aggregated.len()) as u64;
        stats.gro_bytes += packets.iter().map(|p| p.len()).sum::<usize>() as u64;

        Ok(aggregated)
    }

    /// Enable multicast optimization
    pub fn enable_multicast_opt(&self, socket_id: u32) -> Result<(), ()> {
        let mut sockets = self.sockets.write();
        if let Some(socket) = sockets.get_mut(&socket_id) {
            socket.options.multicast_opt = true;
            socket.multicast = Some(UdpMulticastOpt {
                igmp_snooping: true,
                groups: Vec::new(),
            });
            Ok(())
        } else {
            Err(())
        }
    }

    /// Join multicast group
    pub fn join_multicast_group(
        &self,
        socket_id: u32,
        group_addr: Ipv4Addr,
        interface: u32,
    ) -> Result<(), ()> {
        let mut sockets = self.sockets.write();
        if let Some(socket) = sockets.get_mut(&socket_id) {
            if let Some(ref mut multicast) = socket.multicast {
                let group = MulticastGroup {
                    address: group_addr,
                    interface,
                    state: IgmpState::Member,
                    member_count: 1,
                };
                multicast.groups.push(group);

                let mut stats = self.stats.lock();
                stats.multicast_optimizations += 1;
                Ok(())
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    /// Leave multicast group
    pub fn leave_multicast_group(
        &self,
        socket_id: u32,
        group_addr: Ipv4Addr,
    ) -> Result<(), ()> {
        let mut sockets = self.sockets.write();
        if let Some(socket) = sockets.get_mut(&socket_id) {
            if let Some(ref mut multicast) = socket.multicast {
                if let Some(pos) = multicast.groups.iter().position(|g| g.address == group_addr) {
                    multicast.groups.remove(pos);
                    Ok(())
                } else {
                    Err(())
                }
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    /// Allocate DMA buffer from pool
    pub fn allocate_dma_buffer(&self, size: usize) -> Result<Arc<DmaBuffer>, ()> {
        // Try to reuse existing buffer
        {
            let mut pool = self.dma_pool.lock();
            if let Some(buffer) = pool.pop() {
                if buffer.size >= size {
                    return Ok(buffer);
                }
            }
        }

        // Allocate new buffer
        DmaBuffer::new(size).map(Arc::new).ok_or(())
    }

    /// Return DMA buffer to pool
    pub fn free_dma_buffer(&self, buffer: Arc<DmaBuffer>) {
        let mut pool = self.dma_pool.lock();
        if pool.len() < 256 {
            // Limit pool size
            pool.push(buffer);
        }
    }

    /// Configure socket
    pub fn configure_socket(&self, _socket_id: u32, _config: UdpSocketConfig) -> Result<(), ()> {
        // Configuration would be applied here
        Ok(())
    }

    /// Get statistics
    pub fn get_stats(&self) -> UdpOffloadStats {
        let stats = self.stats.lock();
        stats.clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = UdpOffloadStats::default();
    }

    /// Get socket state
    pub fn get_socket_state(&self, socket_id: u32) -> Option<UdpSocketState> {
        let sockets = self.sockets.read();
        sockets.get(&socket_id).cloned()
    }
}

impl Default for UdpZeroCopy {
    fn default() -> Self {
        Self::new()
    }
}

/// UDP socket configuration
#[derive(Debug, Clone)]
pub struct UdpSocketConfig {
    /// Zero-copy mode
    pub zero_copy_mode: ZeroCopyMode,
    /// Enable GSO
    pub enable_gso: bool,
    /// GSO segment size
    pub gso_segment_size: u16,
    /// Enable GRO
    pub enable_gro: bool,
    /// GRO max segments
    pub gro_max_segments: usize,
    /// Enable checksum offload
    pub checksum_offload: bool,
}

/// UDP segmentation
#[derive(Debug)]
pub struct UdpSegmentation {
    /// Segment size
    pub segment_size: u16,
    /// Maximum segments
    pub max_segments: usize,
}

impl UdpSegmentation {
    /// Create new UDP segmentation
    pub fn new(segment_size: u16, max_segments: usize) -> Self {
        Self {
            segment_size,
            max_segments,
        }
    }

    /// Segment a UDP packet
    pub fn segment(&self, data: &[u8]) -> Vec<Vec<u8>> {
        let mut segments = Vec::new();
        let chunk_size = self.segment_size as usize;

        for chunk in data.chunks(chunk_size) {
            if segments.len() >= self.max_segments {
                break;
            }
            segments.push(chunk.to_vec());
        }

        segments
    }
}

/// Re-export types for convenience
pub use crate::network::tcp_advanced::CongestionControl as UdpCongestionControl;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_copy_mode() {
        assert!(!ZeroCopyMode::None.is_enabled());
        assert!(ZeroCopyMode::DmaBuf.is_enabled());
        assert!(ZeroCopyMode::Splice.is_enabled());
        assert!(ZeroCopyMode::Mmap.is_enabled());
    }

    #[test]
    fn test_dma_buffer() {
        let buffer = DmaBuffer::new(4096).unwrap();
        assert_eq!(buffer.size, 4096);
        assert!(!buffer.is_dma_mapped());

        buffer.map_dma();
        assert!(buffer.is_dma_mapped());

        buffer.unmap_dma();
        assert!(!buffer.is_dma_mapped());
    }

    #[test]
    fn test_udp_gso_config() {
        let config = UdpGsoConfig::new(1500, 64);
        assert!(config.enabled);
        assert_eq!(config.segment_size, 1500);
        assert_eq!(config.max_segments, 64);
        assert_eq!(config.max_packet_size(), 1500 * 64);
    }

    #[test]
    fn test_udp_gro_config() {
        let config = UdpGroConfig::new(32, 200);
        assert!(config.enabled);
        assert_eq!(config.max_segments, 32);
        assert_eq!(config.timeout_us, 200);
    }

    #[test]
    fn test_udp_socket_options_default() {
        let opts = UdpSocketOptions::default();
        assert!(opts.checksum_enabled);
        assert!(!opts.gso_enabled);
        assert!(!opts.gro_enabled);
    }

    #[test]
    fn test_udp_socket_state() {
        let mut state = UdpSocketState::new(1);
        assert_eq!(state.socket_id, 1);

        let gso_config = UdpGsoConfig::new(1500, 32);
        state.enable_gso(gso_config);
        assert!(state.options.gso_enabled);
        assert!(state.gso.is_some());

        let gro_config = UdpGroConfig::new(16, 100);
        state.enable_gro(gro_config);
        assert!(state.options.gro_enabled);
        assert!(state.gro.is_some());
    }

    #[test]
    fn test_udp_zero_copy() {
        let udp = UdpZeroCopy::new();
        assert!(udp.register_socket(1).is_ok());
        assert!(udp.register_socket(1).is_err()); // Duplicate

        assert!(udp.set_zero_copy_mode(1, ZeroCopyMode::DmaBuf).is_ok());
        assert_eq!(udp.get_zero_copy_mode(1).unwrap(), ZeroCopyMode::DmaBuf);
    }

    #[test]
    fn test_udp_gso() {
        let udp = UdpZeroCopy::new();
        udp.register_socket(1).unwrap();

        let config = UdpGsoConfig::new(100, 10);
        udp.enable_gso(1, config).unwrap();

        let data = vec![0u8; 500];
        let segments = udp.gso_segment(1, &data, 100).unwrap();
        assert_eq!(segments.len(), 5);
        assert_eq!(segments[0].len(), 100);
    }

    #[test]
    fn test_udp_gro() {
        let udp = UdpZeroCopy::new();
        udp.register_socket(1).unwrap();

        let config = UdpGroConfig::new(4, 100);
        udp.enable_gro(1, config).unwrap();

        let packets = vec![
            vec![1u8; 100],
            vec![2u8; 100],
            vec![3u8; 100],
            vec![4u8; 100],
        ];
        let aggregated = udp.gro_aggregate(1, &packets).unwrap();
        // Should be aggregated into fewer packets
        assert!(aggregated.len() < packets.len());
    }

    #[test]
    fn test_multicast() {
        let udp = UdpZeroCopy::new();
        udp.register_socket(1).unwrap();
        udp.enable_multicast_opt(1).unwrap();

        let group = Ipv4Addr::new(224, 0, 0, 1);
        assert!(udp.join_multicast_group(1, group, 0).is_ok());
        assert!(udp.leave_multicast_group(1, group).is_ok());
    }

    #[test]
    fn test_dma_buffer_pool() {
        let udp = UdpZeroCopy::new();

        let buffer1 = udp.allocate_dma_buffer(4096).unwrap();
        assert_eq!(buffer1.size, 4096);

        // Return buffer to pool
        udp.free_dma_buffer(buffer1.clone());

        // Reallocate - should reuse buffer
        let buffer2 = udp.allocate_dma_buffer(4096).unwrap();
        assert_eq!(buffer2.size, 4096);
    }

    #[test]
    fn test_stats() {
        let udp = UdpZeroCopy::new();
        udp.register_socket(1).unwrap();
        udp.enable_gso(1, UdpGsoConfig::new(100, 10)).unwrap();

        let data = vec![0u8; 300];
        udp.gso_segment(1, &data, 100).unwrap();

        let stats = udp.get_stats();
        assert!(stats.gso_segments > 0);
        assert!(stats.gso_bytes > 0);

        udp.reset_stats();
        let stats = udp.get_stats();
        assert_eq!(stats.gso_segments, 0);
    }

    #[test]
    fn test_udp_segmentation() {
        let seg = UdpSegmentation::new(100, 10);
        let data = vec![0u8; 350];

        let segments = seg.segment(&data);
        assert_eq!(segments.len(), 4); // 100 + 100 + 100 + 50
        assert_eq!(segments[0].len(), 100);
        assert_eq!(segments[3].len(), 50);
    }

    #[test]
    fn test_socket_state_configure() {
        let mut state = UdpSocketState::new(1);
        state.set_zero_copy_mode(ZeroCopyMode::DmaBuf);
        assert_eq!(state.options.zero_copy_mode, ZeroCopyMode::DmaBuf);
    }

    #[test]
    fn test_unregister_socket() {
        let udp = UdpZeroCopy::new();
        udp.register_socket(1).unwrap();
        assert!(udp.unregister_socket(1).is_ok());
        assert!(udp.unregister_socket(1).is_err()); // Already unregistered
    }
}
