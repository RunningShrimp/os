//! IPv6 routing implementation
//!
//! This module provides IPv6 routing table management and longest prefix match lookup.
//! Implements RFC 4291 (IPv6 Address Architecture) for prefix handling.

extern crate alloc;
use alloc::vec::Vec;

use crate::subsystems::net::ipv6::Ipv6Addr;

/// IPv6 prefix (network address and prefix length)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv6Prefix {
    /// Network address
    pub network: Ipv6Addr,
    /// Prefix length (0-128)
    pub prefix_len: u8,
}

impl Ipv6Prefix {
    /// Create a new IPv6 prefix
    pub fn new(network: Ipv6Addr, prefix_len: u8) -> Self {
        Self {
            network: Self::apply_mask(network, prefix_len),
            prefix_len,
        }
    }

    /// Apply prefix mask to address
    fn apply_mask(addr: Ipv6Addr, prefix_len: u8) -> Ipv6Addr {
        let mut octets = addr.0;

        let full_bytes = (prefix_len / 8) as usize;
        let partial_bits = prefix_len % 8;

        // Zero out bytes beyond the prefix
        for i in full_bytes..16 {
            octets[i] = 0;
        }

        // Zero out partial bits in the last byte
        if partial_bits > 0 && full_bytes < 16 {
            let mask = !((1u8 << (8 - partial_bits)) - 1);
            octets[full_bytes] &= mask;
        }

        Ipv6Addr(octets)
    }

    /// Check if an address is within this prefix
    pub fn contains(&self, addr: Ipv6Addr) -> bool {
        let masked = Self::apply_mask(addr, self.prefix_len);
        masked == self.network
    }

    /// Get the default route (::/0)
    pub const fn default_route() -> Self {
        Self {
            network: Ipv6Addr::UNSPECIFIED,
            prefix_len: 0,
        }
    }

    /// Get the loopback prefix (::1/128)
    pub const fn loopback() -> Self {
        Self {
            network: Ipv6Addr::LOCALHOST,
            prefix_len: 128,
        }
    }

    /// Get the link-local prefix (fe80::/10)
    pub const fn link_local() -> Self {
        Self {
            network: Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            prefix_len: 10,
        }
    }

    /// Get the unique local prefix (fc00::/7)
    pub const fn unique_local() -> Self {
        Self {
            network: Ipv6Addr([0xfc, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            prefix_len: 7,
        }
    }

    /// Get the multicast prefix (ff00::/8)
    pub const fn multicast() -> Self {
        Self {
            network: Ipv6Addr([0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            prefix_len: 8,
        }
    }

    /// Convert to string representation
    pub fn to_string(&self) -> alloc::string::String {
        use alloc::fmt::Write;
        let mut s = alloc::string::String::new();
        write!(s, "{}/{}", self.network, self.prefix_len).ok();
        s
    }
}

/// IPv6 route entry
#[derive(Debug, Clone)]
pub struct Ipv6RouteEntry {
    /// Destination prefix
    pub dest: Ipv6Prefix,
    /// Next hop address (optional for directly connected networks)
    pub next_hop: Option<Ipv6Addr>,
    /// Output interface index
    pub interface: u32,
    /// Route metric (lower is preferred)
    pub metric: u32,
    /// Route is active
    pub active: bool,
}

impl Ipv6RouteEntry {
    /// Create a new route entry
    pub fn new(dest: Ipv6Prefix, next_hop: Option<Ipv6Addr>, interface: u32, metric: u32) -> Self {
        Self {
            dest,
            next_hop,
            interface,
            metric,
            active: true,
        }
    }

    /// Create a directly connected route
    pub fn connected(dest: Ipv6Prefix, interface: u32) -> Self {
        Self::new(dest, None, interface, 0)
    }

    /// Create a gateway route
    pub fn gateway(dest: Ipv6Prefix, gateway: Ipv6Addr, interface: u32, metric: u32) -> Self {
        Self::new(dest, Some(gateway), interface, metric)
    }

    /// Create a default route
    pub fn default(gateway: Ipv6Addr, interface: u32, metric: u32) -> Self {
        Self::gateway(Ipv6Prefix::default_route(), gateway, interface, metric)
    }

    /// Check if route is a default route
    pub fn is_default(&self) -> bool {
        self.dest.prefix_len == 0
    }

    /// Check if route is directly connected
    pub fn is_connected(&self) -> bool {
        self.next_hop.is_none()
    }
}

/// IPv6 routing table statistics
#[derive(Debug, Clone, Default)]
pub struct Ipv6RoutingTableStats {
    /// Total number of routes
    pub total_routes: usize,
    /// Number of active routes
    pub active_routes: usize,
    /// Number of route lookups
    pub lookups: u64,
    /// Number of cache hits
    pub cache_hits: u64,
    /// Number of cache misses
    pub cache_misses: u64,
}

/// IPv6 routing table
#[derive(Debug, Clone)]
pub struct Ipv6RoutingTable {
    /// Route entries
    routes: Vec<Ipv6RouteEntry>,
    /// Statistics
    stats: Ipv6RoutingTableStats,
}

impl Ipv6RoutingTable {
    /// Create a new routing table
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            stats: Ipv6RoutingTableStats::default(),
        }
    }

    /// Add a route to the table
    pub fn add_route(&mut self, route: Ipv6RouteEntry) {
        self.routes.push(route);
        self.stats.total_routes = self.routes.len();
        self.stats.active_routes += 1;
    }

    /// Remove a route from the table
    pub fn remove_route(&mut self, index: usize) -> Result<(), ()> {
        if index < self.routes.len() {
            if self.routes[index].active {
                self.stats.active_routes -= 1;
            }
            self.routes.remove(index);
            self.stats.total_routes = self.routes.len();
            Ok(())
        } else {
            Err(())
        }
    }

    /// Find the best route for a destination address
    pub fn lookup(&mut self, dest: Ipv6Addr) -> Option<&Ipv6RouteEntry> {
        self.stats.lookups += 1;

        // Find the longest prefix match
        let mut best_match: Option<&Ipv6RouteEntry> = None;
        let mut best_prefix_len = -1i32;

        for route in &self.routes {
            if !route.active {
                continue;
            }

            if route.dest.contains(dest) {
                let prefix_len = route.dest.prefix_len as i32;
                if prefix_len > best_prefix_len {
                    best_match = Some(route);
                    best_prefix_len = prefix_len;
                }
            }
        }

        if best_match.is_some() {
            self.stats.cache_hits += 1;
        } else {
            self.stats.cache_misses += 1;
        }

        best_match
    }

    /// Find route by destination prefix
    pub fn find_route(&self, dest: Ipv6Prefix) -> Option<usize> {
        self.routes
            .iter()
            .position(|r| r.dest == dest && r.active)
    }

    /// Get a route by index
    pub fn get_route(&self, index: usize) -> Option<&Ipv6RouteEntry> {
        self.routes.get(index)
    }

    /// Get a mutable route by index
    pub fn get_route_mut(&mut self, index: usize) -> Option<&mut Ipv6RouteEntry> {
        self.routes.get_mut(index)
    }

    /// Get all routes
    pub fn routes(&self) -> &[Ipv6RouteEntry] {
        &self.routes
    }

    /// Clear all routes
    pub fn clear(&mut self) {
        self.routes.clear();
        self.stats.total_routes = 0;
        self.stats.active_routes = 0;
    }

    /// Get statistics
    pub fn stats(&self) -> Ipv6RoutingTableStats {
        self.stats.clone()
    }

    /// Update route metrics
    pub fn update_metric(&mut self, index: usize, metric: u32) -> Result<(), ()> {
        if index < self.routes.len() {
            self.routes[index].metric = metric;
            Ok(())
        } else {
            Err(())
        }
    }

    /// Activate a route
    pub fn activate_route(&mut self, index: usize) -> Result<(), ()> {
        if index < self.routes.len() && !self.routes[index].active {
            self.routes[index].active = true;
            self.stats.active_routes += 1;
            Ok(())
        } else {
            Err(())
        }
    }

    /// Deactivate a route
    pub fn deactivate_route(&mut self, index: usize) -> Result<(), ()> {
        if index < self.routes.len() && self.routes[index].active {
            self.routes[index].active = false;
            self.stats.active_routes -= 1;
            Ok(())
        } else {
            Err(())
        }
    }

    /// Sort routes by metric (useful for optimization)
    pub fn sort_by_metric(&mut self) {
        self.routes.sort_by_key(|r| r.metric);
    }

    /// Remove duplicate routes
    pub fn dedup(&mut self) {
        let mut seen = alloc::collections::BTreeSet::new();
        self.routes.retain(|route| {
            let key = (route.dest, route.next_hop, route.interface);
            seen.insert(key)
        });
        self.stats.total_routes = self.routes.len();
        self.stats.active_routes = self.routes.iter().filter(|r| r.active).count();
    }

    /// Get default route
    pub fn default_route(&self) -> Option<&Ipv6RouteEntry> {
        self.routes
            .iter()
            .find(|r| r.active && r.is_default())
    }

    /// Get directly connected routes
    pub fn connected_routes(&self) -> impl Iterator<Item = &Ipv6RouteEntry> {
        self.routes.iter().filter(|r| r.active && r.is_connected())
    }

    /// Get gateway routes
    pub fn gateway_routes(&self) -> impl Iterator<Item = &Ipv6RouteEntry> {
        self.routes
            .iter()
            .filter(|r| r.active && !r.is_connected())
    }
}

impl Default for Ipv6RoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

/// IPv6 route manager
pub struct Ipv6RouteManager {
    /// Routing table
    table: Ipv6RoutingTable,
    /// Route cache (for recently used routes)
    cache: Option<(Ipv6Addr, usize)>,
}

impl Ipv6RouteManager {
    /// Create a new route manager
    pub fn new() -> Self {
        Self {
            table: Ipv6RoutingTable::new(),
            cache: None,
        }
    }

    /// Add a route
    pub fn add_route(&mut self, route: Ipv6RouteEntry) {
        self.table.add_route(route);
        self.invalidate_cache();
    }

    /// Remove a route
    pub fn remove_route(&mut self, index: usize) -> Result<(), ()> {
        let result = self.table.remove_route(index);
        if result.is_ok() {
            self.invalidate_cache();
        }
        result
    }

    /// Look up the best route for a destination
    pub fn lookup(&mut self, dest: Ipv6Addr) -> Option<&Ipv6RouteEntry> {
        // Check cache first
        if let Some((cached_dest, cached_index)) = self.cache {
            if cached_dest == dest {
                if let Some(route) = self.table.get_route(cached_index) {
                    if route.active && route.dest.contains(dest) {
                        return Some(route);
                    }
                }
            }
        }

        // Perform full lookup
        let result = self.table.lookup(dest);

        // Update cache
        if let Some(route) = result {
            if let Some(index) = self.table.routes().iter().position(|r| r as *const _ == route as *const _) {
                self.cache = Some((dest, index));
            }
        }

        result
    }

    /// Invalidate the route cache
    fn invalidate_cache(&mut self) {
        self.cache = None;
    }

    /// Get the routing table
    pub fn table(&self) -> &Ipv6RoutingTable {
        &self.table
    }

    /// Get a mutable reference to the routing table
    pub fn table_mut(&mut self) -> &mut Ipv6RoutingTable {
        self.invalidate_cache();
        &mut self.table
    }

    /// Add a default route
    pub fn add_default_route(&mut self, gateway: Ipv6Addr, interface: u32, metric: u32) {
        self.add_route(Ipv6RouteEntry::default(gateway, interface, metric));
    }

    /// Add a directly connected route
    pub fn add_connected_route(&mut self, prefix: Ipv6Prefix, interface: u32) {
        self.add_route(Ipv6RouteEntry::connected(prefix, interface));
    }

    /// Get statistics
    pub fn stats(&self) -> Ipv6RoutingTableStats {
        self.table.stats()
    }

    /// Flush all routes
    pub fn flush(&mut self) {
        self.table.clear();
        self.invalidate_cache();
    }

    /// Optimize routing table (sort and dedup)
    pub fn optimize(&mut self) {
        self.table.sort_by_metric();
        self.table.dedup();
        self.invalidate_cache();
    }
}

impl Default for Ipv6RouteManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipv6_prefix() {
        let prefix = Ipv6Prefix::new(Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]), 64);
        assert!(prefix.contains(Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2])));
        assert!(!prefix.contains(Ipv6Addr([0xfe, 0x81, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])));
    }

    #[test]
    fn test_routing_table_lookup() {
        let mut table = Ipv6RoutingTable::new();

        // Add routes
        table.add_route(Ipv6RouteEntry::connected(
            Ipv6Prefix::link_local(),
            1,
        ));

        let gateway = Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
        table.add_route(Ipv6RouteEntry::default(gateway, 1, 1));

        // Lookup
        let dest = Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let route = table.lookup(dest);
        assert!(route.is_some());
        assert!(route.unwrap().is_connected());

        // Lookup default route
        let internet = Ipv6Addr([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let route = table.lookup(internet);
        assert!(route.is_some());
        assert!(route.unwrap().is_default());
    }

    #[test]
    fn test_longest_prefix_match() {
        let mut table = Ipv6RoutingTable::new();

        // Add overlapping routes
        table.add_route(Ipv6RouteEntry::connected(
            Ipv6Prefix::new(Ipv6Addr([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]), 32),
            1,
        ));

        table.add_route(Ipv6RouteEntry::connected(
            Ipv6Prefix::new(Ipv6Addr([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]), 48),
            1,
        ));

        // Lookup should match the /48 (longer prefix)
        let dest = Ipv6Addr([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let route = table.lookup(dest);
        assert!(route.is_some());
        assert_eq!(route.unwrap().dest.prefix_len, 48);
    }
}
