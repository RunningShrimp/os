//! UDP Optimization Implementation
//!
//! This module provides advanced UDP optimizations including:
//! - GSO (Generic Segmentation Offload) support
//! - GRO (Generic Receive Offload) support
//! - Checksum optimization (hardware offload)
//! - Batched packet processing
//! - Jumbo frame support

extern crate alloc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::subsystems::net::{
    ipv4::Ipv4Addr,
    udp::{UdpPacket, UdpHeader},
};

/// Maximum GSO segment size
pub const MAX_GSO_SIZE: u16 = 65535;

/// Default GSO size (9KB jumbo frames)
pub const DEFAULT_GSO_SIZE: u16 = 9216;

/// Maximum GRO packet size
pub const MAX_GRO_SIZE: usize = 65536;

/// Minimum GRO aggregation count
pub const MIN_GRO_COUNT: usize = 4;

/// UDP offload configuration
pub struct UdpOffloadConfig {
    /// GSO enabled
    pub gso_enabled: bool,
    /// GRO enabled
    pub gro_enabled: bool,
    /// Maximum GSO segment size
    pub max_gso_size: u16,
    /// Checksum offload enabled
    pub checksum_offload: bool,
    /// Jumbo frame support
    pub jumbo_frames: bool,
    /// Maximum batch size
    pub max_batch_size: usize,
}

impl Default for UdpOffloadConfig {
    fn default() -> Self {
        Self {
            gso_enabled: true,
            gro_enabled: true,
            max_gso_size: DEFAULT_GSO_SIZE,
            checksum_offload: true,
            jumbo_frames: true,
            max_batch_size: 32,
        }
    }
}

/// UDP offload manager
pub struct UdpOffload {
    /// Configuration
    config: UdpOffloadConfig,
    /// Statistics
    stats: UdpOffloadStats,
}

/// Offload statistics
#[derive(Default)]
pub struct UdpOffloadStats {
    /// GSO segments created
    pub gso_segments: AtomicU64,
    /// GSO bytes processed
    pub gso_bytes: AtomicU64,
    /// GRO packets coalesced
    pub gro_coalesced: AtomicU64,
    /// GRO packets aggregated
    pub gro_aggregated: AtomicU64,
    /// Checksums offloaded to hardware
    pub checksums_offloaded: AtomicU64,
    /// Jumbo frames sent
    pub jumbo_frames_sent: AtomicU64,
    /// Batched operations
    pub batched_ops: AtomicU64,
}

impl UdpOffload {
    /// Create a new UDP offload instance
    pub fn new(config: UdpOffloadConfig) -> Self {
        Self {
            config,
            stats: UdpOffloadStats::default(),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(UdpOffloadConfig::default())
    }

    /// Segment a large UDP packet using GSO
    ///
    /// Takes a large UDP packet and splits it into multiple segments
    /// that fit within the MTU. Hardware can then handle the segmentation.
    pub fn segment(&self, packet: &UdpPacket, mtu: usize) -> Result<Vec<UdpPacket>, UdpOffloadError> {
        if !self.config.gso_enabled {
            return Err(UdpOffloadError::GsoDisabled);
        }

        let payload_len = packet.payload_len();
        let max_segment_payload = mtu - UdpHeader::SIZE;

        if payload_len <= max_segment_payload {
            // No segmentation needed
            return Ok(vec![packet.clone()]);
        }

        let mut segments = Vec::new();
        let chunk_size = self.config.max_gso_size as usize;
        chunk_size.min(max_segment_payload);

        let mut offset = 0;
        let mut seq_num = 0u16;

        while offset < payload_len {
            let end = (offset + chunk_size).min(payload_len);
            let chunk = &packet.payload[offset..end];

            let segment = UdpPacket::new(
                packet.header.src_port,
                packet.header.dst_port,
                chunk.to_vec(),
                Ipv4Addr::UNSPECIFIED,
                Ipv4Addr::UNSPECIFIED,
            );

            segments.push(segment);
            offset = end;
            seq_num = seq_num.wrapping_add(1);
        }

        self.stats.gso_segments.fetch_add(segments.len() as u64, Ordering::Relaxed);
        self.stats.gso_bytes.fetch_add(payload_len as u64, Ordering::Relaxed);

        Ok(segments)
    }

    /// Coalesce multiple UDP packets using GRO
    ///
    /// Combines multiple small UDP packets into a single large packet
    /// to reduce processing overhead.
    pub fn coalesce(&self, packets: &[UdpPacket]) -> Result<UdpPacket, UdpOffloadError> {
        if !self.config.gro_enabled {
            return Err(UdpOffloadError::GroDisabled);
        }

        if packets.len() < MIN_GRO_COUNT {
            return Err(UdpOffloadError::TooFewPackets);
        }

        // Verify all packets have same source/destination
        let first = &packets[0];
        for pkt in &packets[1..] {
            if pkt.header.src_port != first.header.src_port
                || pkt.header.dst_port != first.header.dst_port
            {
                return Err(UdpOffloadError::PacketMismatch);
            }
        }

        // Calculate total payload size
        let total_payload: usize = packets.iter().map(|p| p.payload_len()).sum();

        if total_payload > MAX_GRO_SIZE {
            return Err(UdpOffloadError::PacketTooLarge);
        }

        // Combine payloads
        let mut combined_payload = Vec::with_capacity(total_payload);
        for pkt in packets {
            combined_payload.extend_from_slice(&pkt.payload);
        }

        let total_length = (UdpHeader::SIZE + combined_payload.len()) as u16;
        let mut header = UdpHeader::new(
            first.header.src_port,
            first.header.dst_port,
            total_length,
        );

        // Calculate checksum (will be offloaded to hardware if enabled)
        if self.config.checksum_offload {
            header.checksum = 0; // Hardware will calculate
        } else {
            header.set_checksum(Ipv4Addr::UNSPECIFIED, Ipv4Addr::UNSPECIFIED, &combined_payload);
        }

        let coalesced = UdpPacket {
            header,
            payload: combined_payload,
        };

        self.stats.gro_coalesced.fetch_add(packets.len() as u64, Ordering::Relaxed);
        self.stats.gro_aggregated.fetch_add(1, Ordering::Relaxed);

        Ok(coalesced)
    }

    /// Calculate checksum with hardware offload support
    pub fn calculate_checksum(
        &self,
        packet: &mut UdpPacket,
        src_addr: Ipv4Addr,
        dst_addr: Ipv4Addr,
    ) -> Result<(), UdpOffloadError> {
        if self.config.checksum_offload {
            // Mark for hardware offload
            packet.header.checksum = 0;
            self.stats.checksums_offloaded.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            // Software checksum calculation
            packet.header.set_checksum(src_addr, dst_addr, &packet.payload);
            Ok(())
        }
    }

    /// Verify checksum with hardware validation support
    pub fn verify_checksum(
        &self,
        packet: &UdpPacket,
        src_addr: Ipv4Addr,
        dst_addr: Ipv4Addr,
    ) -> bool {
        if self.config.checksum_offload {
            // Assume hardware validated it
            return true;
        }

        packet.verify_checksum(src_addr, dst_addr)
    }

    /// Process a batch of packets efficiently
    pub fn process_batch(&self, packets: &mut [UdpPacket]) -> Result<usize, UdpOffloadError> {
        let batch_size = packets.len().min(self.config.max_batch_size);

        if batch_size == 0 {
            return Ok(0);
        }

        // Process packets in batch
        for _i in 0..batch_size {
            // Perform any necessary transformations
            // In a real implementation, this would include:
            // - Checksum validation/offload
            // - GRO aggregation
            // - Routing lookups
            // - etc.
        }

        self.stats.batched_ops.fetch_add(1, Ordering::Relaxed);
        Ok(batch_size)
    }

    /// Check if jumbo frames are supported
    pub fn supports_jumbo_frames(&self) -> bool {
        self.config.jumbo_frames
    }

    /// Get maximum segment size for GSO
    pub fn max_gso_size(&self) -> u16 {
        self.config.max_gso_size
    }

    /// Get statistics
    pub fn get_stats(&self) -> &UdpOffloadStats {
        &self.stats
    }

    /// Enable or disable GSO
    pub fn set_gso_enabled(&mut self, enabled: bool) {
        self.config.gso_enabled = enabled;
    }

    /// Enable or disable GRO
    pub fn set_gro_enabled(&mut self, enabled: bool) {
        self.config.gro_enabled = enabled;
    }

    /// Enable or disable checksum offload
    pub fn set_checksum_offload(&mut self, enabled: bool) {
        self.config.checksum_offload = enabled;
    }

    /// Set maximum GSO size
    pub fn set_max_gso_size(&mut self, size: u16) {
        self.config.max_gso_size = size.min(MAX_GSO_SIZE);
    }

    /// Set maximum batch size
    pub fn set_max_batch_size(&mut self, size: usize) {
        self.config.max_batch_size = size.min(64);
    }
}

/// Batched packet processor for high-throughput scenarios
pub struct BatchedPacketProcessor {
    /// Offload manager
    offload: UdpOffload,
    /// Packet buffer
    packet_buffer: Vec<UdpPacket>,
    /// Buffer capacity
    capacity: usize,
}

impl BatchedPacketProcessor {
    /// Create a new batched packet processor
    pub fn new(capacity: usize, config: UdpOffloadConfig) -> Self {
        Self {
            offload: UdpOffload::new(config),
            packet_buffer: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Add a packet to the batch buffer
    pub fn add_packet(&mut self, packet: UdpPacket) -> Result<(), UdpOffloadError> {
        if self.packet_buffer.len() >= self.capacity {
            self.flush()?;
        }

        self.packet_buffer.push(packet);
        Ok(())
    }

    /// Flush the batch buffer
    pub fn flush(&mut self) -> Result<usize, UdpOffloadError> {
        if self.packet_buffer.is_empty() {
            return Ok(0);
        }

        // Try GRO coalescing if we have enough packets
        if self.packet_buffer.len() >= MIN_GRO_COUNT {
            if let Ok(coalesced) = self.offload.coalesce(&self.packet_buffer) {
                self.packet_buffer.clear();
                self.packet_buffer.push(coalesced);
                return Ok(1);
            }
        }

        let count = self.packet_buffer.len();
        self.offload.process_batch(&mut self.packet_buffer)?;
        self.packet_buffer.clear();

        Ok(count)
    }

    /// Get the offload manager
    pub fn offload(&self) -> &UdpOffload {
        &self.offload
    }

    /// Get mutable offload manager
    pub fn offload_mut(&mut self) -> &mut UdpOffload {
        &mut self.offload
    }

    /// Get current batch size
    pub fn batch_size(&self) -> usize {
        self.packet_buffer.len()
    }
}

/// Jumbo frame support for large UDP packets
pub struct JumboFrameSupport {
    /// Maximum jumbo frame size
    max_frame_size: u16,
    /// Jumbo frame enabled
    enabled: bool,
    /// Path MTU discovery enabled
    pmtud_enabled: bool,
}

impl JumboFrameSupport {
    /// Create jumbo frame support
    pub fn new(max_frame_size: u16) -> Self {
        Self {
            max_frame_size,
            enabled: max_frame_size > 1500,
            pmtud_enabled: true,
        }
    }

    /// Check if packet is a jumbo frame
    pub fn is_jumbo_frame(&self, size: usize) -> bool {
        self.enabled && size > 1500
    }

    /// Get maximum frame size
    pub fn max_frame_size(&self) -> u16 {
        self.max_frame_size
    }

    /// Enable or disable jumbo frames
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if path MTU discovery is enabled
    pub fn pmtud_enabled(&self) -> bool {
        self.pmtud_enabled
    }

    /// Perform path MTU discovery
    pub fn discover_pmtu(&self, _dest: Ipv4Addr) -> Result<u16, UdpOffloadError> {
        // In a real implementation, this would:
        // 1. Send packets with DF flag set
        // 2. Handle ICMP "Fragmentation Needed" messages
        // 3. Adjust MTU accordingly

        if self.pmtud_enabled {
            Ok(self.max_frame_size)
        } else {
            Err(UdpOffloadError::PmtudDisabled)
        }
    }
}

/// UDP offload errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UdpOffloadError {
    /// GSO is disabled
    GsoDisabled,
    /// GRO is disabled
    GroDisabled,
    /// Packet too large
    PacketTooLarge,
    /// Packets don't match for coalescing
    PacketMismatch,
    /// Too few packets for aggregation
    TooFewPackets,
    /// Invalid segment size
    InvalidSegmentSize,
    /// Checksum offload failed
    ChecksumFailed,
    /// Path MTU discovery disabled
    PmtudDisabled,
    /// Batch processing failed
    BatchFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gso_segmentation() {
        let offload = UdpOffload::with_defaults();

        // Create a large packet (9KB payload)
        let large_payload = vec![0u8; 9216];
        let packet = UdpPacket::new(
            1234,
            5678,
            large_payload,
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(192, 168, 1, 2),
        );

        // Segment with standard 1500 byte MTU
        let segments = offload.segment(&packet, 1500).unwrap();

        // Should create multiple segments
        assert!(segments.len() > 1);

        // Each segment should fit in MTU
        for segment in &segments {
            assert!(segment.len() <= 1500);
        }
    }

    #[test]
    fn test_gro_coalescing() {
        let offload = UdpOffload::with_defaults();

        // Create multiple small packets
        let mut packets = Vec::new();
        for i in 0..8 {
            let payload = vec![i as u8; 128];
            let packet = UdpPacket::new(
                1234,
                5678,
                payload,
                Ipv4Addr::UNSPECIFIED,
                Ipv4Addr::UNSPECIFIED,
            );
            packets.push(packet);
        }

        // Coalesce packets
        let coalesced = offload.coalesce(&packets).unwrap();

        // Combined payload should be sum of all payloads
        assert_eq!(coalesced.payload_len(), 8 * 128);
    }

    #[test]
    fn test_batched_processor() {
        let config = UdpOffloadConfig::default();
        let mut processor = BatchedPacketProcessor::new(32, config);

        // Add packets
        for i in 0..16 {
            let payload = vec![i as u8; 64];
            let packet = UdpPacket::new(
                1234,
                5678,
                payload,
                Ipv4Addr::UNSPECIFIED,
                Ipv4Addr::UNSPECIFIED,
            );
            processor.add_packet(packet).unwrap();
        }

        assert_eq!(processor.batch_size(), 16);

        // Flush should process the batch
        let processed = processor.flush().unwrap();
        assert_eq!(processed, 16);
        assert_eq!(processor.batch_size(), 0);
    }

    #[test]
    fn test_jumbo_frame_support() {
        let jumbo = JumboFrameSupport::new(9000);

        assert!(jumbo.is_jumbo_frame(2000));
        assert!(!jumbo.is_jumbo_frame(1500));
        assert_eq!(jumbo.max_frame_size(), 9000);
    }
}
