//! IPv4 packet fragmentation and reassembly
//!
//! This module handles IP packet fragmentation for transmission and reassembly
//! of received fragments.

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::vec;
use core::time::Duration;
use core::sync::atomic::{AtomicU64, Ordering};

use super::ipv4::{Ipv4Addr, Ipv4Header, Ipv4Packet};

/// Maximum fragment reassembly timeout in seconds
const FRAGMENT_TIMEOUT: Duration = Duration::from_secs(60);

/// Maximum number of concurrent reassembly entries
const MAX_REASSEMBLY_ENTRIES: usize = 1024;

/// Fragment identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FragmentId {
    /// Source IP address
    pub src_ip: Ipv4Addr,
    /// Destination IP address
    pub dst_ip: Ipv4Addr,
    /// Protocol
    pub protocol: u8,
    /// Identification field
    pub identification: u16,
}

impl FragmentId {
    /// Create a new fragment ID
    pub fn new(src_ip: Ipv4Addr, dst_ip: Ipv4Addr, protocol: u8, identification: u16) -> Self {
        Self {
            src_ip,
            dst_ip,
            protocol,
            identification,
        }
    }
}

/// Reassembly fragment
#[derive(Debug, Clone)]
pub struct Fragment {
    /// Fragment data
    pub data: Vec<u8>,
    /// Fragment offset in bytes
    pub offset: usize,
    /// Fragment length
    pub length: usize,
    /// More fragments flag
    pub more_fragments: bool,
    /// Reception timestamp
    pub timestamp: u64,
}

impl Fragment {
    /// Create a new fragment
    pub fn new(data: Vec<u8>, offset: usize, more_fragments: bool) -> Self {
        let length = data.len();
        Self {
            data,
            offset,
            length,
            more_fragments,
            timestamp: Self::current_time(),
        }
    }

    /// Get current time (placeholder)
    fn current_time() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Check if fragment has expired
    pub fn is_expired(&self) -> bool {
        let now = Self::current_time();
        let elapsed = now.saturating_sub(self.timestamp);
        elapsed >= FRAGMENT_TIMEOUT.as_secs()
    }
}

/// Reassembly entry
#[derive(Debug, Clone)]
pub struct ReassemblyEntry {
    /// Fragment ID
    pub id: FragmentId,
    /// Total datagram length (unknown until all fragments received)
    pub total_length: Option<usize>,
    /// Received fragments
    pub fragments: BTreeMap<usize, Fragment>,
    /// Creation timestamp
    pub created_at: u64,
    /// Last update timestamp
    pub last_update: u64,
}

impl ReassemblyEntry {
    /// Create a new reassembly entry
    pub fn new(id: FragmentId) -> Self {
        let now = Fragment::current_time();
        Self {
            id,
            total_length: None,
            fragments: BTreeMap::new(),
            created_at: now,
            last_update: now,
        }
    }

    /// Add a fragment to this entry
    pub fn add_fragment(&mut self, fragment: Fragment) -> Result<(), FragmentError> {
        // Check if fragment overlaps with existing fragments
        let fragment_end = fragment.offset + fragment.length;

        for (&existing_offset, existing_fragment) in &self.fragments {
            let existing_end = existing_offset + existing_fragment.length;

            // Check for overlap
            if fragment.offset < existing_end && fragment_end > existing_offset {
                return Err(FragmentError::Overlap);
            }
        }

        // Add the fragment
        self.fragments.insert(fragment.offset, fragment);
        self.last_update = Fragment::current_time();

        Ok(())
    }

    /// Check if all fragments have been received
    pub fn is_complete(&self) -> bool {
        if self.fragments.is_empty() {
            return false;
        }

        // Check if the last fragment has the "more fragments" flag cleared
        let has_last_fragment = self.fragments.values().any(|f| !f.more_fragments);
        if !has_last_fragment {
            return false;
        }

        // Calculate expected total length and check for gaps
        if let Some(total_length) = self.calculate_total_length() {
            self.check_no_gaps(total_length)
        } else {
            false
        }
    }

    /// Calculate the total datagram length
    pub fn calculate_total_length(&self) -> Option<usize> {
        if self.fragments.is_empty() {
            return None;
        }

        // Find the last fragment (highest offset + length)
        let mut max_end = 0;
        for fragment in self.fragments.values() {
            let end = fragment.offset + fragment.length;
            if end > max_end {
                max_end = end;
            }
        }

        // Check if we have a complete datagram from 0 to max_end with no gaps
        if self.check_no_gaps(max_end) {
            Some(max_end)
        } else {
            None
        }
    }

    /// Check if there are no gaps in the fragment coverage up to total_length
    fn check_no_gaps(&self, total_length: usize) -> bool {
        if self.fragments.is_empty() {
            return false;
        }

        // Sort fragments by offset
        let mut sorted_fragments: Vec<_> = self.fragments.values().collect();
        sorted_fragments.sort_by_key(|f| f.offset);

        // Check if first fragment starts at offset 0
        if sorted_fragments[0].offset != 0 {
            return false;
        }

        // Check for gaps between consecutive fragments
        for window in sorted_fragments.windows(2) {
            let current = &window[0];
            let next = &window[1];

            let current_end = current.offset + current.length;
            if current_end != next.offset {
                return false;
            }
        }

        // Check if the last fragment covers up to total_length
        let last = sorted_fragments.last().unwrap();
        (last.offset + last.length) == total_length
    }

    /// Reassemble the complete datagram
    pub fn reassemble(&self) -> Result<Vec<u8>, FragmentError> {
        let total_length = self.calculate_total_length()
            .ok_or(FragmentError::IncompleteDatagram)?;

        let mut datagram = vec![0u8; total_length];

        for fragment in self.fragments.values() {
            let end = fragment.offset + fragment.length;
            if end > total_length {
                return Err(FragmentError::InvalidFragment);
            }

            datagram[fragment.offset..end].copy_from_slice(&fragment.data);
        }

        Ok(datagram)
    }

    /// Check if this entry has expired
    pub fn is_expired(&self) -> bool {
        let now = Fragment::current_time();
        let elapsed = now.saturating_sub(self.created_at);
        elapsed >= FRAGMENT_TIMEOUT.as_secs()
    }

    /// Get entry statistics
    pub fn stats(&self) -> ReassemblyStats {
        ReassemblyStats {
            fragment_count: self.fragments.len(),
            total_bytes: self.fragments.values().map(|f| f.length).sum(),
            complete: self.is_complete(),
            age_seconds: Fragment::current_time().saturating_sub(self.created_at),
        }
    }
}

/// Reassembly statistics
#[derive(Debug, Clone)]
pub struct ReassemblyStats {
    /// Number of fragments
    pub fragment_count: usize,
    /// Total bytes received
    pub total_bytes: usize,
    /// Whether reassembly is complete
    pub complete: bool,
    /// Age in seconds
    pub age_seconds: u64,
}

/// Fragment reassembly manager
pub struct FragmentReassembler {
    /// Active reassembly entries
    entries: BTreeMap<FragmentId, ReassemblyEntry>,
    /// Maximum number of entries
    max_entries: usize,
    /// Cleanup interval
    cleanup_interval: u64,
    /// Last cleanup time
    last_cleanup: AtomicU64,
    /// Statistics
    stats: FragmentReassemblerStats,
}

impl FragmentReassembler {
    /// Create a new fragment reassembler
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            max_entries: MAX_REASSEMBLY_ENTRIES,
            cleanup_interval: 10, // Clean up every 10 time units
            last_cleanup: AtomicU64::new(0),
            stats: FragmentReassemblerStats::new(),
        }
    }

    /// Process an incoming fragment
    pub fn process_fragment(
        &mut self,
        header: &Ipv4Header,
        payload: &[u8],
    ) -> Result<Option<Vec<u8>>, FragmentError> {
        // Check if packet is fragmented
        if !header.more_fragments() && header.fragment_offset() == 0 {
            // Not a fragment, return payload as-is
            return Ok(Some(payload.to_vec()));
        }

        // Create fragment ID
        let fragment_id = FragmentId::new(
            header.source_addr,
            header.dest_addr,
            header.protocol,
            header.identification,
        );

        // Create fragment object
        let offset = (header.fragment_offset() as usize) * 8; // Convert to bytes
        let fragment = Fragment::new(
            payload.to_vec(),
            offset,
            header.more_fragments(),
        );

        // Get or create reassembly entry
        let entry = self.entries.entry(fragment_id).or_insert_with(|| {
            self.stats.total_datagrams += 1;
            ReassemblyEntry::new(fragment_id)
        });

        // Add fragment to entry
        entry.add_fragment(fragment.clone())?;
        self.stats.total_fragments += 1;

        // Check if reassembly is complete
        if entry.is_complete() {
            match entry.reassemble() {
                Ok(datum) => {
                    self.stats.successful_reassemblies += 1;
                    // Remove completed entry
                    self.entries.remove(&fragment_id);
                    Ok(Some(datum))
                }
                Err(e) => {
                    self.stats.failed_reassemblies += 1;
                    Err(e)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Clean up expired entries
    pub fn cleanup(&mut self) {
        let now = Fragment::current_time();
        let last_cleanup = self.last_cleanup.load(Ordering::Relaxed);

        if now.saturating_sub(last_cleanup) < self.cleanup_interval {
            return;
        }

        let mut expired_count = 0;
        self.entries.retain(|_, entry| {
            let keep = !entry.is_expired();
            if !keep {
                expired_count += 1;
                self.stats.timeouts += 1;
            }
            keep
        });

        // Remove oldest entries if we have too many
        while self.entries.len() > self.max_entries {
            // Find the oldest key without holding a borrow
            let oldest_key = self.entries.iter().next().map(|(k, _)| *k);
            if let Some(key) = oldest_key {
                self.entries.remove(&key);
                self.stats.evicted += 1;
            } else {
                break;
            }
        }

        self.last_cleanup.store(now, Ordering::Relaxed);

        if expired_count > 0 {
            crate::log_info!("Cleaned up {} expired reassembly entries", expired_count);
        }
    }

    /// Get reassembler statistics
    pub fn stats(&self) -> FragmentReassemblerStats {
        let mut stats = self.stats.clone();
        stats.active_entries = self.entries.len();
        stats
    }

    /// Get active reassembly entries
    pub fn active_entries(&self) -> impl Iterator<Item = &ReassemblyEntry> {
        self.entries.values()
    }

    /// Flush all reassembly entries
    pub fn flush(&mut self) {
        self.entries.clear();
    }
}

impl Default for FragmentReassembler {
    fn default() -> Self {
        Self::new()
    }
}

/// Fragment reassembler statistics
#[derive(Debug, Clone)]
pub struct FragmentReassemblerStats {
    /// Total datagrams being reassembled
    pub total_datagrams: u64,
    /// Total fragments received
    pub total_fragments: u64,
    /// Successful reassemblies
    pub successful_reassemblies: u64,
    /// Failed reassemblies
    pub failed_reassemblies: u64,
    /// Reassembly timeouts
    pub timeouts: u64,
    /// Evicted entries
    pub evicted: u64,
    /// Currently active entries
    pub active_entries: usize,
}

impl FragmentReassemblerStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            total_datagrams: 0,
            total_fragments: 0,
            successful_reassemblies: 0,
            failed_reassemblies: 0,
            timeouts: 0,
            evicted: 0,
            active_entries: 0,
        }
    }
}

/// Fragmentation for outgoing packets
pub struct Fragmenter {
    /// Maximum transmission unit (MTU)
    mtu: usize,
    /// Identification counter
    next_id: u16,
}

impl Fragmenter {
    /// Create a new fragmenter
    pub fn new(mtu: usize) -> Self {
        Self {
            mtu: mtu.saturating_sub(Ipv4Header::MIN_SIZE), // Leave room for IP header
            next_id: 1,
        }
    }

    /// Fragment an IP packet if necessary
    pub fn fragment_packet(
        &mut self,
        packet: &Ipv4Packet,
    ) -> Result<Vec<Ipv4Packet>, FragmentError> {
        let total_size = Ipv4Header::MIN_SIZE + packet.payload.len();

        // Check if fragmentation is needed
        if total_size <= self.mtu {
            // No fragmentation needed
            return Ok(vec![packet.clone()]);
        }

        // Check if fragmentation is allowed
        if packet.header.dont_fragment() {
            return Err(FragmentError::FragmentationNeeded);
        }

        self.do_fragmentation(packet)
    }

    /// Perform actual fragmentation
    fn do_fragmentation(&self, packet: &Ipv4Packet) -> Result<Vec<Ipv4Packet>, FragmentError> {
        let payload = &packet.payload;
        let payload_size = payload.len();

        // Calculate maximum payload per fragment (must be multiple of 8 bytes)
        let max_payload = (self.mtu - Ipv4Header::MIN_SIZE) & !7;
        if max_payload == 0 {
            return Err(FragmentError::PacketTooSmall);
        }

        let mut fragments = Vec::new();
        let mut offset = 0;
        let fragment_id = packet.header.identification;

        while offset < payload_size {
            let remaining = payload_size - offset;
            let fragment_payload_len = remaining.min(max_payload);

            // Extract fragment payload
            let fragment_payload = payload[offset..offset + fragment_payload_len].to_vec();

            // Determine if this is the last fragment
            let is_last = offset + fragment_payload_len == payload_size;

            // Create fragment header
            let mut fragment_header = packet.header.clone();
            fragment_header.set_identification(fragment_id);
            fragment_header.total_length = (Ipv4Header::MIN_SIZE + fragment_payload_len) as u16;
            fragment_header.set_fragmentation(
                packet.header.dont_fragment(),
                !is_last,
                (offset / 8) as u16,
            );

            // Calculate new checksum
            fragment_header.set_checksum();

            // Create fragment packet
            let fragment = Ipv4Packet {
                header: fragment_header,
                payload: fragment_payload,
            };

            fragments.push(fragment);
            offset += fragment_payload_len;
        }

        Ok(fragments)
    }

    /// Get next identification value
    pub fn next_identification(&mut self) -> u16 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }
}

/// Fragmentation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FragmentError {
    /// Packet too small to fragment
    PacketTooSmall,
    /// Fragmentation needed but DF flag set
    FragmentationNeeded,
    /// Overlapping fragments
    Overlap,
    /// Incomplete datagram
    IncompleteDatagram,
    /// Invalid fragment
    InvalidFragment,
    /// Too many fragments
    TooManyFragments,
    /// Reassembly timeout
    ReassemblyTimeout,
    /// Buffer exhausted
    BufferExhausted,
}

// Use logging system
// log_info is a macro, use crate::log_info!() instead
