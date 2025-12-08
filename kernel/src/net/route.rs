//! IPv4 routing table implementation
//!
//! This module provides routing table functionality for determining the best
//! interface and next hop for packet delivery.

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use super::ipv4::Ipv4Addr;
use super::interface::Interface;

/// Routing table entry
#[derive(Debug, Clone)]
pub struct RouteEntry {
    /// Destination network address
    pub destination: Ipv4Addr,
    /// Network mask
    pub netmask: Ipv4Addr,
    /// Gateway address (None for directly connected networks)
    pub gateway: Option<Ipv4Addr>,
    /// Interface to use for this route
    pub interface_id: u32,
    /// Route metric (lower is preferred)
    pub metric: u32,
    /// Route is active
    pub active: bool,
}

impl RouteEntry {
    /// Create a new route entry
    pub fn new(
        destination: Ipv4Addr,
        netmask: Ipv4Addr,
        gateway: Option<Ipv4Addr>,
        interface_id: u32,
        metric: u32,
    ) -> Self {
        Self {
            destination,
            netmask,
            gateway,
            interface_id,
            metric,
            active: true,
        }
    }

    /// Check if this route matches the given destination
    pub fn matches(&self, destination: Ipv4Addr) -> bool {
        (destination.to_u32() & self.netmask.to_u32()) ==
        (self.destination.to_u32() & self.netmask.to_u32())
    }

    /// Get the network address
    pub fn network(&self) -> Ipv4Addr {
        Ipv4Addr::from_u32(
            self.destination.to_u32() & self.netmask.to_u32()
        )
    }

    /// Check if this is a default route (0.0.0.0/0)
    pub fn is_default(&self) -> bool {
        self.destination == Ipv4Addr::UNSPECIFIED &&
        self.netmask == Ipv4Addr::UNSPECIFIED
    }

    /// Get the prefix length of the netmask
    pub fn prefix_len(&self) -> u8 {
        Self::netmask_to_prefix_len(self.netmask)
    }

    /// Convert netmask to prefix length
    fn netmask_to_prefix_len(netmask: Ipv4Addr) -> u8 {
        let mask = netmask.to_u32();
        let mut count = 0;
        let mut bits = mask;

        // Count the number of consecutive 1s from the left
        while bits != 0 && (bits & 0x80000000) != 0 {
            count += 1;
            bits <<= 1;
        }

        // Verify that the remaining bits are all 0 (valid netmask)
        if bits != 0 {
            // Invalid netmask, return maximum
            32
        } else {
            count
        }
    }

    /// Convert prefix length to netmask
    pub fn prefix_len_to_netmask(prefix_len: u8) -> Ipv4Addr {
        if prefix_len > 32 {
            return Ipv4Addr::UNSPECIFIED;
        }

        let mask = if prefix_len == 0 {
            0
        } else {
            0xFFFFFFFF << (32 - prefix_len)
        };

        Ipv4Addr::from_u32(mask)
    }
}

/// Routing table
pub struct RoutingTable {
    /// Route entries
    entries: Vec<RouteEntry>,
    /// Cache for recently looked up routes
    cache: BTreeMap<Ipv4Addr, Option<RouteEntry>>,
    /// Maximum cache size
    max_cache_size: usize,
}

impl RoutingTable {
    /// Create a new routing table
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            cache: BTreeMap::new(),
            max_cache_size: 1000,
        }
    }

    /// Add a route to the table
    pub fn add_route(&mut self, route: RouteEntry) {
        // Remove existing route for the same network if it exists
        self.entries.retain(|r| {
            !(r.network() == route.network() && r.interface_id == route.interface_id)
        });

        self.entries.push(route);
        self.invalidate_cache();
    }

    /// Remove a route from the table
    pub fn remove_route(&mut self, destination: Ipv4Addr, netmask: Ipv4Addr, interface_id: u32) -> bool {
        let network = destination.to_u32() & netmask.to_u32();
        let original_len = self.entries.len();

        self.entries.retain(|r| {
            let route_network = r.destination.to_u32() & r.netmask.to_u32();
            !(route_network == network && r.interface_id == interface_id)
        });

        let removed = self.entries.len() < original_len;
        if removed {
            self.invalidate_cache();
        }
        removed
    }

    /// Find the best route for a destination address
    pub fn lookup_route(&mut self, destination: Ipv4Addr) -> Option<&RouteEntry> {
        // Check cache first
        if let Some(cached_result) = self.cache.get(&destination) {
            return cached_result.as_ref();
        }

        // Simple approach: return a direct reference without caching
        // This avoids borrow checker issues with self-borrowing
        self.entries.iter()
            .filter(|r| r.active && r.matches(destination))
            .min_by(|a, b| {
                a.metric.cmp(&b.metric)
                    .then_with(|| b.prefix_len().cmp(&a.prefix_len()))
            })
    }

    /// Get the next hop for a destination
    pub fn get_next_hop(&mut self, destination: Ipv4Addr) -> Option<(Ipv4Addr, u32)> {
        if let Some(route) = self.lookup_route(destination) {
            if let Some(gateway) = route.gateway {
                Some((gateway, route.interface_id))
            } else {
                // Directly connected network
                Some((destination, route.interface_id))
            }
        } else {
            None
        }
    }

    /// Check if a destination is reachable
    pub fn is_reachable(&mut self, destination: Ipv4Addr) -> bool {
        self.lookup_route(destination).is_some()
    }

    /// Add a directly connected network route
    pub fn add_direct_route(&mut self, network: Ipv4Addr, netmask: Ipv4Addr, interface_id: u32) {
        let route = RouteEntry::new(network, netmask, None, interface_id, 0);
        self.add_route(route);
    }

    /// Add a default route
    pub fn add_default_route(&mut self, gateway: Ipv4Addr, interface_id: u32) {
        let route = RouteEntry::new(
            Ipv4Addr::UNSPECIFIED,
            Ipv4Addr::UNSPECIFIED,
            Some(gateway),
            interface_id,
            1
        );
        self.add_route(route);
    }

    /// Get all routes
    pub fn routes(&self) -> &[RouteEntry] {
        &self.entries
    }

    /// Get all routes for a specific interface
    pub fn routes_for_interface(&self, interface_id: u32) -> Vec<&RouteEntry> {
        self.entries
            .iter()
            .filter(|r| r.interface_id == interface_id)
            .collect()
    }

    /// Flush the routing table
    pub fn flush(&mut self) {
        self.entries.clear();
        self.invalidate_cache();
    }

    /// Cache a route lookup result
    fn cache_entry(&mut self, destination: Ipv4Addr, route: Option<RouteEntry>) {
        // Remove oldest entry if cache is full
        if self.cache.len() >= self.max_cache_size {
            // Find the oldest key without holding a borrow
            let oldest_key = self.cache.iter().next().map(|(k, _)| *k);
            if let Some(key) = oldest_key {
                self.cache.remove(&key);
            }
        }

        self.cache.insert(destination, route);
    }

    /// Invalidate the route cache
    fn invalidate_cache(&mut self) {
        self.cache.clear();
    }

    /// Get routing table statistics
    pub fn stats(&self) -> RoutingTableStats {
        RoutingTableStats {
            total_routes: self.entries.len(),
            active_routes: self.entries.iter().filter(|r| r.active).count(),
            default_routes: self.entries.iter().filter(|r| r.is_default()).count(),
            cache_size: self.cache.len(),
            cache_hits: 0, // Would need to track this separately
            cache_misses: 0,
        }
    }
}

impl Default for RoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Routing table statistics
#[derive(Debug, Clone)]
pub struct RoutingTableStats {
    /// Total number of routes
    pub total_routes: usize,
    /// Number of active routes
    pub active_routes: usize,
    /// Number of default routes
    pub default_routes: usize,
    /// Cache size
    pub cache_size: usize,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
}

/// Route lookup result
#[derive(Debug, Clone)]
pub struct RouteLookupResult {
    /// Destination network
    pub destination: Ipv4Addr,
    /// Netmask
    pub netmask: Ipv4Addr,
    /// Gateway address (if any)
    pub gateway: Option<Ipv4Addr>,
    /// Interface ID
    pub interface_id: u32,
    /// Route metric
    pub metric: u32,
    /// Whether this is a default route
    pub is_default: bool,
}

impl RouteLookupResult {
    /// Create a new lookup result from a route entry
    pub fn from_route_entry(route: &RouteEntry) -> Self {
        Self {
            destination: route.destination,
            netmask: route.netmask,
            gateway: route.gateway,
            interface_id: route.interface_id,
            metric: route.metric,
            is_default: route.is_default(),
        }
    }
}

/// Route manager for handling multiple routing tables
pub struct RouteManager {
    /// Main routing table
    main_table: RoutingTable,
    /// Additional tables (e.g., per-process)
    tables: BTreeMap<String, RoutingTable>,
}

impl RouteManager {
    /// Create a new route manager
    pub fn new() -> Self {
        Self {
            main_table: RoutingTable::new(),
            tables: BTreeMap::new(),
        }
    }

    /// Get the main routing table
    pub fn main_table(&mut self) -> &mut RoutingTable {
        &mut self.main_table
    }

    /// Add a named routing table
    pub fn add_table(&mut self, name: String, table: RoutingTable) {
        self.tables.insert(name, table);
    }

    /// Get a named routing table
    pub fn get_table(&mut self, name: &str) -> Option<&mut RoutingTable> {
        self.tables.get_mut(name)
    }

    /// Remove a named routing table
    pub fn remove_table(&mut self, name: &str) -> Option<RoutingTable> {
        self.tables.remove(name)
    }

    /// Get all table names
    pub fn table_names(&self) -> Vec<&String> {
        self.tables.keys().collect()
    }
}

impl Default for RouteManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Common network configurations
pub mod common_routes {
    use super::*;

    /// Add common local routes
    pub fn add_local_routes(table: &mut RoutingTable, interface_id: u32, local_ip: Ipv4Addr, netmask: Ipv4Addr) {
        // Direct route to local network
        table.add_direct_route(local_ip, netmask, interface_id);

        // Loopback route
        table.add_direct_route(
            Ipv4Addr::new(127, 0, 0, 0),
            Ipv4Addr::new(255, 0, 0, 0),
            interface_id,
        );
    }

    /// Add multicast route
    pub fn add_multicast_route(table: &mut RoutingTable, interface_id: u32) {
        table.add_direct_route(
            Ipv4Addr::new(224, 0, 0, 0),
            Ipv4Addr::new(240, 0, 0, 0),
            interface_id,
        );
    }
}