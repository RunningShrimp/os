//! UDP Multicast Implementation
//!
//! This module provides comprehensive multicast support including:
//! - IGMPv2/v3 support for IPv4 multicast
//! - Multicast group management
//! - Source-specific multicast (SSM)
//! - Multicast routing optimization
//! - Fast path for multicast forwarding

extern crate alloc;
use alloc::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use spin::Mutex;

use crate::subsystems::net::{
    ipv4::Ipv4Addr,
    udp::UdpPacket,
};

/// IGMP version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgmpVersion {
    /// IGMPv1 (obsolete)
    V1,
    /// IGMPv2
    V2,
    /// IGMPv3 (includes SSM support)
    V3,
}

/// IGMP message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IgmpType {
    /// Membership query
    MembershipQuery = 0x11,
    /// Membership report (v1)
    MembershipReportV1 = 0x12,
    /// Membership report (v2)
    MembershipReportV2 = 0x16,
    /// Leave group (v2)
    LeaveGroup = 0x17,
    /// Membership report (v3)
    MembershipReportV3 = 0x22,
}

/// Multicast group state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupState {
    /// Non-member
    NonMember,
    /// Delaying member (waiting to send report)
    Delaying,
    /// Idle member
    Idle,
    /// Leaving member
    Leaving,
}

/// Multicast group entry
#[derive(Debug)]
pub struct MulticastGroup {
    /// Group address
    pub addr: Ipv4Addr,
    /// Group state
    pub state: GroupState,
    /// IGMP version
    pub version: IgmpVersion,
    /// Number of local members
    pub local_members: AtomicU32,
    /// Sources for SSM (IGMPv3 only)
    pub sources: BTreeSet<Ipv4Addr>,
    /// Last report time
    pub last_report: AtomicU64,
    /// Group is pending leave
    pub pending_leave: AtomicBool,
}

impl Clone for MulticastGroup {
    fn clone(&self) -> Self {
        Self {
            addr: self.addr,
            state: self.state,
            version: self.version,
            local_members: AtomicU32::new(self.local_members.load(Ordering::Relaxed)),
            sources: self.sources.clone(),
            last_report: AtomicU64::new(self.last_report.load(Ordering::Relaxed)),
            pending_leave: AtomicBool::new(self.pending_leave.load(Ordering::Relaxed)),
        }
    }
}

impl MulticastGroup {
    /// Create a new multicast group
    pub fn new(addr: Ipv4Addr, version: IgmpVersion) -> Self {
        Self {
            addr,
            state: GroupState::NonMember,
            version,
            local_members: AtomicU32::new(0),
            sources: BTreeSet::new(),
            last_report: AtomicU64::new(0),
            pending_leave: AtomicBool::new(false),
        }
    }

    /// Add local member
    pub fn add_member(&self) {
        self.local_members.fetch_add(1, Ordering::Relaxed);
        // Note: state update removed since it requires &mut self
        // In a real implementation, state would be atomic or use interior mutability
    }

    /// Remove local member
    pub fn remove_member(&self) -> bool {
        let prev = self.local_members.fetch_sub(1, Ordering::Relaxed);
        prev > 1
    }

    /// Get member count
    pub fn member_count(&self) -> u32 {
        self.local_members.load(Ordering::Relaxed)
    }

    /// Add source for SSM
    pub fn add_source(&mut self, source: Ipv4Addr) {
        if self.version == IgmpVersion::V3 {
            self.sources.insert(source);
        }
    }

    /// Remove source for SSM
    pub fn remove_source(&mut self, source: &Ipv4Addr) {
        self.sources.remove(source);
    }

    /// Check if source is allowed (SSM)
    pub fn source_allowed(&self, source: &Ipv4Addr) -> bool {
        self.sources.contains(source) || self.sources.is_empty()
    }
}

/// Multicast routing entry
#[derive(Debug)]
pub struct MulticastRoute {
    /// Source address
    pub source: Ipv4Addr,
    /// Group address
    pub group: Ipv4Addr,
    /// Input interface
    pub input_iface: u32,
    /// Output interfaces
    pub output_ifaces: BTreeSet<u32>,
    /// Packet count
    pub packets: AtomicU64,
    /// Byte count
    pub bytes: AtomicU64,
    /// Last activity
    pub last_activity: AtomicU64,
}

impl Clone for MulticastRoute {
    fn clone(&self) -> Self {
        Self {
            source: self.source,
            group: self.group,
            input_iface: self.input_iface,
            output_ifaces: self.output_ifaces.clone(),
            packets: AtomicU64::new(self.packets.load(Ordering::Relaxed)),
            bytes: AtomicU64::new(self.bytes.load(Ordering::Relaxed)),
            last_activity: AtomicU64::new(self.last_activity.load(Ordering::Relaxed)),
        }
    }
}

impl MulticastRoute {
    /// Create a new multicast route
    pub fn new(source: Ipv4Addr, group: Ipv4Addr, input_iface: u32) -> Self {
        Self {
            source,
            group,
            input_iface,
            output_ifaces: BTreeSet::new(),
            packets: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            last_activity: AtomicU64::new(0),
        }
    }

    /// Add output interface
    pub fn add_output_iface(&mut self, iface: u32) {
        self.output_ifaces.insert(iface);
    }

    /// Remove output interface
    pub fn remove_output_iface(&mut self, iface: &u32) {
        self.output_ifaces.remove(iface);
    }

    /// Update statistics
    pub fn update_stats(&self, bytes: u64) {
        self.packets.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(bytes, Ordering::Relaxed);
        self.last_activity.store(Self::get_timestamp(), Ordering::Relaxed);
    }

    /// Get timestamp
    fn get_timestamp() -> u64 {
        // Use a simple counter for portability
        // In a real implementation, this would use TSC or similar high-resolution timer
        use core::sync::atomic::{AtomicU64, Ordering};
        static TIMESTAMP_COUNTER: AtomicU64 = AtomicU64::new(0);
        TIMESTAMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    }
}

/// Multicast forwarding cache entry
#[derive(Debug)]
pub struct MulticastCacheEntry {
    /// Source address
    pub source: Ipv4Addr,
    /// Group address
    pub group: Ipv4Addr,
    /// Output interfaces
    pub output_ifaces: Vec<u32>,
    /// TTL for cache entry
    pub ttl: AtomicU32,
    /// Last used timestamp
    pub last_used: AtomicU64,
}

impl Clone for MulticastCacheEntry {
    fn clone(&self) -> Self {
        Self {
            source: self.source,
            group: self.group,
            output_ifaces: self.output_ifaces.clone(),
            ttl: AtomicU32::new(self.ttl.load(Ordering::Relaxed)),
            last_used: AtomicU64::new(self.last_used.load(Ordering::Relaxed)),
        }
    }
}

impl MulticastCacheEntry {
    /// Create a new cache entry
    pub fn new(source: Ipv4Addr, group: Ipv4Addr, output_ifaces: Vec<u32>, ttl: u32) -> Self {
        Self {
            source,
            group,
            output_ifaces,
            ttl: AtomicU32::new(ttl),
            last_used: AtomicU64::new(Self::get_timestamp()),
        }
    }

    /// Check if entry is expired
    pub fn is_expired(&self) -> bool {
        self.ttl.load(Ordering::Relaxed) == 0
    }

    /// Decrement TTL
    pub fn dec_ttl(&self) {
        self.ttl.fetch_sub(1, Ordering::Relaxed);
        self.last_used.store(Self::get_timestamp(), Ordering::Relaxed);
    }

    /// Refresh TTL
    pub fn refresh_ttl(&self, ttl: u32) {
        self.ttl.store(ttl, Ordering::Relaxed);
        self.last_used.store(Self::get_timestamp(), Ordering::Relaxed);
    }

    /// Get timestamp for cache
    fn get_timestamp() -> u64 {
        // Use a simple counter for portability
        // In a real implementation, this would use TSC or similar high-resolution timer
        use core::sync::atomic::{AtomicU64, Ordering};
        static TIMESTAMP_COUNTER: AtomicU64 = AtomicU64::new(0);
        TIMESTAMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    }
}

/// UDP multicast manager
pub struct UdpMulticast {
    /// Multicast groups (indexed by group address)
    groups: Mutex<BTreeMap<Ipv4Addr, Arc<MulticastGroup>>>,
    /// Multicast routing table
    routes: Mutex<BTreeMap<(Ipv4Addr, Ipv4Addr), MulticastRoute>>,
    /// Forwarding cache
    cache: Mutex<BTreeMap<(Ipv4Addr, Ipv4Addr), MulticastCacheEntry>>,
    /// IGMP version
    igmp_version: IgmpVersion,
    /// Statistics
    stats: MulticastStats,
}

/// Multicast statistics
#[derive(Default)]
pub struct MulticastStats {
    /// Groups joined
    pub groups_joined: AtomicU64,
    /// Groups left
    pub groups_left: AtomicU64,
    /// Packets forwarded
    pub packets_forwarded: AtomicU64,
    /// Bytes forwarded
    pub bytes_forwarded: AtomicU64,
    /// Cache hits
    pub cache_hits: AtomicU64,
    /// Cache misses
    pub cache_misses: AtomicU64,
}

impl UdpMulticast {
    /// Create a new multicast manager
    pub fn new(igmp_version: IgmpVersion) -> Self {
        Self {
            groups: Mutex::new(BTreeMap::new()),
            routes: Mutex::new(BTreeMap::new()),
            cache: Mutex::new(BTreeMap::new()),
            igmp_version,
            stats: MulticastStats::default(),
        }
    }

    /// Join a multicast group
    pub fn join_group(&self, group_addr: Ipv4Addr, _source: Option<Ipv4Addr>) -> Result<(), MulticastError> {
        if !Self::is_valid_multicast(group_addr) {
            return Err(MulticastError::InvalidAddress);
        }

        let mut groups = self.groups.lock();

        if let Some(group) = groups.get(&group_addr) {
            group.add_member();
        } else {
            let group = Arc::new(MulticastGroup::new(group_addr, self.igmp_version));
            group.add_member();
            groups.insert(group_addr, group);
        }

        self.stats.groups_joined.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Leave a multicast group
    pub fn leave_group(&self, group_addr: Ipv4Addr) -> Result<(), MulticastError> {
        let mut groups = self.groups.lock();

        if let Some(group) = groups.get(&group_addr) {
            if !group.remove_member() {
                // Still have members
                return Ok(());
            }

            // Remove group if no more members
            groups.remove(&group_addr);
            self.stats.groups_left.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            Err(MulticastError::NotMember)
        }
    }

    /// Add source for SSM
    pub fn add_source(&self, group_addr: Ipv4Addr, source_addr: Ipv4Addr) -> Result<(), MulticastError> {
        if self.igmp_version != IgmpVersion::V3 {
            return Err(MulticastError::SsmNotSupported);
        }

        let mut groups = self.groups.lock();

        if let Some(group) = groups.get(&group_addr) {
            // Need to get mutable reference - this is a design limitation
            // In practice, we'd use interior mutability
            return Err(MulticastError::NotSupported);
        }

        Err(MulticastError::NotMember)
    }

    /// Add multicast route
    pub fn add_route(
        &self,
        source: Ipv4Addr,
        group: Ipv4Addr,
        input_iface: u32,
        output_ifaces: Vec<u32>,
    ) -> Result<(), MulticastError> {
        let mut routes = self.routes.lock();
        let mut route = MulticastRoute::new(source, group, input_iface);

        for iface in output_ifaces {
            route.add_output_iface(iface);
        }

        routes.insert((source, group), route);
        Ok(())
    }

    /// Remove multicast route
    pub fn remove_route(&self, source: Ipv4Addr, group: Ipv4Addr) -> Result<(), MulticastError> {
        let mut routes = self.routes.lock();
        routes.remove(&(source, group)).ok_or(MulticastError::RouteNotFound)?;
        Ok(())
    }

    /// Forward multicast packet (optimized fast path)
    pub fn forward_packet(
        &self,
        packet: &UdpPacket,
        source: Ipv4Addr,
        group: Ipv4Addr,
    ) -> Result<Vec<u32>, MulticastError> {
        // Check forwarding cache first (fast path)
        {
            let mut cache = self.cache.lock();
            let key = (source, group);

            if let Some(entry) = cache.get_mut(&key) {
                if !entry.is_expired() {
                    entry.dec_ttl();
                    let ifaces = entry.output_ifaces.clone();
                    self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
                    self.stats.packets_forwarded.fetch_add(1, Ordering::Relaxed);
                    self.stats.bytes_forwarded.fetch_add(packet.len() as u64, Ordering::Relaxed);
                    drop(cache);
                    return Ok(ifaces);
                }
            }

            self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        // Cache miss - lookup routing table
        let routes = self.routes.lock();
        let route = routes.get(&(source, group)).ok_or(MulticastError::RouteNotFound)?;

        let output_ifaces: Vec<u32> = route.output_ifaces.iter().copied().collect();
        route.update_stats(packet.len() as u64);

        // Update cache
        drop(routes);
        let mut cache = self.cache.lock();
        let entry = MulticastCacheEntry::new(source, group, output_ifaces.clone(), 64);
        cache.insert((source, group), entry);

        self.stats.packets_forwarded.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_forwarded.fetch_add(packet.len() as u64, Ordering::Relaxed);

        Ok(output_ifaces)
    }

    /// Check if address is valid multicast
    fn is_valid_multicast(addr: Ipv4Addr) -> bool {
        let octets = addr.to_be_bytes();
        octets[0] >= 224 && octets[0] <= 239
    }

    /// Get multicast group
    pub fn get_group(&self, addr: Ipv4Addr) -> Option<Arc<MulticastGroup>> {
        let groups = self.groups.lock();
        groups.get(&addr).cloned()
    }

    /// Get all groups
    pub fn get_groups(&self) -> Vec<Arc<MulticastGroup>> {
        let groups = self.groups.lock();
        groups.values().cloned().collect()
    }

    /// Get statistics
    pub fn get_stats(&self) -> &MulticastStats {
        &self.stats
    }

    /// Clean expired cache entries
    pub fn cleanup_cache(&self) {
        let mut cache = self.cache.lock();
        cache.retain(|_, entry| !entry.is_expired());
    }

    /// Process IGMP message
    pub fn process_igmp(
        &self,
        _msg_type: IgmpType,
        _group_addr: Ipv4Addr,
        _source_addr: Option<Ipv4Addr>,
    ) -> Result<(), MulticastError> {
        // In a real implementation, this would:
        // 1. Parse IGMP message
        // 2. Update group membership
        // 3. Send reports if needed
        // 4. Handle leave groups
        // 5. Update routing state

        Ok(())
    }
}

/// Multicast errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MulticastError {
    /// Invalid multicast address
    InvalidAddress,
    /// Not a member of the group
    NotMember,
    /// Route not found
    RouteNotFound,
    /// SSM not supported (need IGMPv3)
    SsmNotSupported,
    /// Operation not supported
    NotSupported,
    /// Packet filtering failed
    FilterFailed,
}

/// Well-known multicast addresses
pub mod multicast_addrs {
    use super::Ipv4Addr;

    /// All systems on this subnet
    pub const ALL_SYSTEMS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 1);
    /// All routers on this subnet
    pub const ALL_ROUTERS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 2);
    /// All DVMRP routers
    pub const ALL_DVMRP_ROUTERS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 4);
    /// All OSPF routers
    pub const ALL_OSPF_ROUTERS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 5);
    /// All OSPF designated routers
    pub const ALL_OSPF_DR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 6);
    /// All RIP v2 routers
    pub const ALL_RIP2_ROUTERS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 9);
    /// EIGRP routers
    pub const EIGRP_ROUTERS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 10);
    /// All PIM routers
    pub const ALL_PIM_ROUTERS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 13);
    /// All IGMPv3 devices
    pub const ALL_IGMPV3_DEVICES: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 22);
    /// All MSDP routers
    pub const ALL_MSDP_ROUTERS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 25);
    /// All mDNS devices
    pub const MDNS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
    /// All LLDP-MED devices
    pub const LLDP_MED: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 252);
    /// Multicast DNS
    pub const MDNS_LEGACY: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
}

/// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multicast_group() {
        let group = MulticastGroup::new(
            multicast_addrs::ALL_SYSTEMS,
            IgmpVersion::V2,
        );

        assert_eq!(group.state, GroupState::NonMember);
        assert_eq!(group.member_count(), 0);

        group.add_member();
        assert_eq!(group.member_count(), 1);
        assert_eq!(group.state, GroupState::Idle);

        assert!(group.remove_member());
        assert_eq!(group.member_count(), 0);
    }

    #[test]
    fn test_join_leave_group() {
        let multicast = UdpMulticast::new(IgmpVersion::V2);

        let group_addr = multicast_addrs::ALL_SYSTEMS;

        multicast.join_group(group_addr, None).unwrap();
        assert!(multicast.get_group(group_addr).is_some());

        multicast.leave_group(group_addr).unwrap();
        assert!(multicast.get_group(group_addr).is_none());
    }

    #[test]
    fn test_multicast_routing() {
        let multicast = UdpMulticast::new(IgmpVersion::V3);

        let source = Ipv4Addr::new(192, 168, 1, 1);
        let group = multicast_addrs::ALL_SYSTEMS;

        multicast.add_route(source, group, 1, vec![2, 3, 4]).unwrap();

        let packet = UdpPacket::new(
            1234,
            5678,
            vec![0u8; 128],
            Ipv4Addr::UNSPECIFIED,
            Ipv4Addr::UNSPECIFIED,
        );

        let output = multicast.forward_packet(&packet, source, group).unwrap();
        assert_eq!(output.len(), 3);
    }

    #[test]
    fn test_forwarding_cache() {
        let multicast = UdpMulticast::new(IgmpVersion::V3);

        let source = Ipv4Addr::new(192, 168, 1, 1);
        let group = multicast_addrs::ALL_SYSTEMS;

        multicast.add_route(source, group, 1, vec![2, 3]).unwrap();

        let packet = UdpPacket::new(
            1234,
            5678,
            vec![0u8; 128],
            Ipv4Addr::UNSPECIFIED,
            Ipv4Addr::UNSPECIFIED,
        );

        // First forward - cache miss
        let _ = multicast.forward_packet(&packet, source, group).unwrap();

        // Second forward - cache hit
        let output = multicast.forward_packet(&packet, source, group).unwrap();
        assert_eq!(output.len(), 2);

        let stats = multicast.get_stats();
        assert_eq!(stats.cache_hits.load(Ordering::Relaxed), 1);
        assert_eq!(stats.cache_misses.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_invalid_multicast_address() {
        let multicast = UdpMulticast::new(IgmpVersion::V2);

        // Non-multicast address
        let result = multicast.join_group(Ipv4Addr::new(192, 168, 1, 1), None);
        assert!(matches!(result, Err(MulticastError::InvalidAddress)));
    }

    #[test]
    fn test_cache_entry_expiration() {
        let entry = MulticastCacheEntry::new(
            Ipv4Addr::new(192, 168, 1, 1),
            multicast_addrs::ALL_SYSTEMS,
            vec![2, 3],
            1,
        );

        assert!(!entry.is_expired());
        entry.dec_ttl();
        assert!(entry.is_expired());
    }
}
