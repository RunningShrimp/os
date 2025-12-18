//! Enhanced routing functionality for NOS network stack
//!
//! This module provides advanced routing capabilities including:
//! - Policy-based routing
//! - Multipath routing
//! - Route aggregation and summarization
//! - Dynamic routing protocols support
//! - Route redistribution
//! - Traffic engineering
//! - Quality of Service (QoS) routing

extern crate alloc;
use alloc::collections::BTreeMap;


use alloc::vec::Vec;
use alloc::string::String;

use core::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use spin::Mutex;

use super::ipv4::Ipv4Addr;

use super::route::RouteEntry;
// RoutingTable在当前文件中未使用，暂时注释掉
// use super::route::RoutingTable;
use crate::time;

/// Enhanced routing entry with additional attributes
#[derive(Debug, Clone)]
pub struct EnhancedRouteEntry {
    /// Base route entry
    pub base: RouteEntry,
    /// Route source (static, connected, rip, ospf, bgp, etc.)
    pub source: RouteSource,
    /// Route tag for identification
    pub tag: u32,
    /// Administrative distance (lower is preferred)
    pub admin_distance: u8,
    /// Route age in seconds
    pub age: u64,
    /// Last update timestamp
    pub last_update: u64,
    /// Route preferences
    pub preferences: RoutePreferences,
    /// Traffic engineering attributes
    pub te_attributes: TrafficEngineeringAttributes,
    /// QoS class
    pub qos_class: Option<QoSClass>,
    /// Route statistics
    pub stats: RouteStats,
}

impl EnhancedRouteEntry {
    /// Create a new enhanced route entry
    pub fn new(base: RouteEntry, source: RouteSource) -> Self {
        let now = time::get_monotonic_time();
        Self {
            base,
            source,
            tag: 0,
            admin_distance: source.default_admin_distance(),
            age: 0,
            last_update: now,
            preferences: RoutePreferences::default(),
            te_attributes: TrafficEngineeringAttributes::default(),
            qos_class: None,
            stats: RouteStats::default(),
        }
    }

    /// Update the route age
    pub fn update_age(&mut self) {
        let now = time::get_monotonic_time();
        self.age = now - self.last_update;
    }

    /// Refresh the route
    pub fn refresh(&mut self) {
        self.last_update = time::get_monotonic_time();
        self.age = 0;
    }

    /// Check if this route is better than another
    pub fn is_better_than(&self, other: &EnhancedRouteEntry) -> bool {
        // Compare administrative distance
        if self.admin_distance != other.admin_distance {
            return self.admin_distance < other.admin_distance;
        }

        // Compare metrics
        if self.base.metric != other.base.metric {
            return self.base.metric < other.base.metric;
        }

        // Compare prefix length (longer prefix is more specific)
        let self_prefix = self.base.prefix_len();
        let other_prefix = other.base.prefix_len();
        if self_prefix != other_prefix {
            return self_prefix > other_prefix;
        }

        // Compare preferences
        self.preferences.is_better_than(&other.preferences)
    }

    /// Get the effective metric considering all factors
    pub fn effective_metric(&self) -> u32 {
        let mut metric = self.base.metric;
        
        // Apply administrative distance
        metric = metric.saturating_mul(self.admin_distance as u32);
        
        // Apply TE attributes
        metric = metric.saturating_add(self.te_attributes.additional_metric);
        
        metric
    }
}

/// Route source types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteSource {
    /// Directly connected network
    Connected,
    /// Static configuration
    Static,
    /// Routing Information Protocol
    Rip,
    /// Open Shortest Path First
    Ospf,
    /// Border Gateway Protocol
    Bgp,
    /// Enhanced Interior Gateway Routing Protocol
    Eigrp,
    /// Intermediate System to Intermediate System
    Isis,
    /// Multiprotocol BGP
    MpBgp,
    /// Policy-based routing
    Policy,
}

impl RouteSource {
    /// Get the default administrative distance for this route source
    pub fn default_admin_distance(self) -> u8 {
        match self {
            RouteSource::Connected => 0,
            RouteSource::Static => 1,
            RouteSource::Eigrp => 90,
            RouteSource::Ospf => 110,
            RouteSource::Isis => 115,
            RouteSource::Rip => 120,
            RouteSource::Eigrp => 170, // External EIGRP
            RouteSource::Bgp => 20,    // Internal BGP
            RouteSource::MpBgp => 200,  // External BGP
            RouteSource::Policy => 250,
        }
    }
}

/// Route preferences for advanced selection
#[derive(Debug, Clone, Default)]
pub struct RoutePreferences {
    /// Preference weight (0-100)
    pub weight: u8,
    /// Local preference (for BGP)
    pub local_pref: u32,
    /// AS path length (for BGP)
    pub as_path_len: u32,
    /// Origin type (for BGP)
    pub origin: BgpOrigin,
    /// Multi-exit discriminator (for BGP)
    pub med: u32,
    /// Community attributes (for BGP)
    pub communities: Vec<u32>,
}

impl RoutePreferences {
    /// Create new preferences
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if these preferences are better than others
    pub fn is_better_than(&self, other: &RoutePreferences) -> bool {
        // Compare weight
        if self.weight != other.weight {
            return self.weight > other.weight;
        }

        // Compare local preference
        if self.local_pref != other.local_pref {
            return self.local_pref > other.local_pref;
        }

        // Compare AS path length
        if self.as_path_len != other.as_path_len {
            return self.as_path_len < other.as_path_len;
        }

        // Compare origin
        if self.origin != other.origin {
            return self.origin.is_better_than(other.origin);
        }

        // Compare MED
        if self.med != other.med {
            return self.med < other.med;
        }

        false
    }
}

/// BGP origin types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgpOrigin {
    /// IGP - Interior Gateway Protocol
    Igp,
    /// EGP - Exterior Gateway Protocol
    Egp,
    /// Incomplete
    Incomplete,
}

impl BgpOrigin {
    /// Check if this origin is better than another
    pub fn is_better_than(self, other: BgpOrigin) -> bool {
        match (self, other) {
            (BgpOrigin::Igp, _) => true,
            (BgpOrigin::Egp, BgpOrigin::Igp) => false,
            (BgpOrigin::Egp, _) => true,
            (BgpOrigin::Incomplete, BgpOrigin::Incomplete) => false,
            (BgpOrigin::Incomplete, _) => false,
        }
    }
}

impl Default for BgpOrigin {
    fn default() -> Self {
        BgpOrigin::Incomplete
    }
}

/// Traffic engineering attributes
#[derive(Debug, Clone, Default)]
pub struct TrafficEngineeringAttributes {
    /// Additional metric for TE
    pub additional_metric: u32,
    /// Bandwidth constraint (Kbps)
    pub bandwidth: Option<u32>,
    /// Delay constraint (microseconds)
    pub delay: Option<u32>,
    /// Jitter constraint (microseconds)
    pub jitter: Option<u32>,
    /// Loss constraint (percentage * 100)
    pub loss: Option<u32>,
    /// Resource class affinity
    pub affinity: Option<u32>,
    /// Explicit route object
    pub ero: Vec<Ipv4Addr>,
    /// Record route object
    pub rro: Vec<Ipv4Addr>,
}

/// QoS classes for traffic differentiation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QoSClass {
    /// Best effort
    BestEffort,
    /// Background
    Background,
    /// Excellent effort
    ExcellentEffort,
    /// Critical applications
    CriticalApplications,
    /// Video
    Video,
    /// Voice
    Voice,
    /// Network control
    NetworkControl,
}

/// Route statistics
#[derive(Debug, Clone, Default)]
pub struct RouteStats {
    /// Number of packets forwarded using this route
    pub packets_forwarded: AtomicU64,
    /// Number of bytes forwarded using this route
    pub bytes_forwarded: AtomicU64,
    /// Number of lookup hits
    pub lookup_hits: AtomicU64,
    /// Number of lookup misses
    pub lookup_misses: AtomicU64,
    /// Number of times this route was selected
    pub selection_count: AtomicU64,
}

impl RouteStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment packet counter
    pub fn increment_packets(&self, count: u64) {
        self.packets_forwarded.fetch_add(count, Ordering::Relaxed);
    }

    /// Increment byte counter
    pub fn increment_bytes(&self, count: u64) {
        self.bytes_forwarded.fetch_add(count, Ordering::Relaxed);
    }

    /// Increment lookup hits
    pub fn increment_lookup_hits(&self) {
        self.lookup_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment lookup misses
    pub fn increment_lookup_misses(&self) {
        self.lookup_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment selection count
    pub fn increment_selection(&self) {
        self.selection_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get packet count
    pub fn get_packets(&self) -> u64 {
        self.packets_forwarded.load(Ordering::Relaxed)
    }

    /// Get byte count
    pub fn get_bytes(&self) -> u64 {
        self.bytes_forwarded.load(Ordering::Relaxed)
    }

    /// Get lookup hits
    pub fn get_lookup_hits(&self) -> u64 {
        self.lookup_hits.load(Ordering::Relaxed)
    }

    /// Get lookup misses
    pub fn get_lookup_misses(&self) -> u64 {
        self.lookup_misses.load(Ordering::Relaxed)
    }

    /// Get selection count
    pub fn get_selection_count(&self) -> u64 {
        self.selection_count.load(Ordering::Relaxed)
    }
}

/// Enhanced routing table with advanced features
pub struct EnhancedRoutingTable {
    /// Enhanced route entries
    entries: Vec<EnhancedRouteEntry>,
    /// Route cache for fast lookups
    cache: BTreeMap<Ipv4Addr, Option<EnhancedRouteEntry>>,
    /// Maximum cache size
    max_cache_size: usize,
    /// Cache statistics
    cache_stats: CacheStats,
    /// Multipath routes
    multipath_routes: BTreeMap<Ipv4Addr, Vec<EnhancedRouteEntry>>,
    /// Policy routes
    policy_routes: Vec<PolicyRoute>,
    /// Route aggregation rules
    aggregation_rules: Vec<AggregationRule>,
    /// Route redistribution configuration
    redistribution: RedistributionConfig,
    /// Global routing statistics
    global_stats: GlobalRoutingStats,
}

impl EnhancedRoutingTable {
    /// Create a new enhanced routing table
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            cache: BTreeMap::new(),
            max_cache_size: 1000,
            cache_stats: CacheStats::default(),
            multipath_routes: BTreeMap::new(),
            policy_routes: Vec::new(),
            aggregation_rules: Vec::new(),
            redistribution: RedistributionConfig::default(),
            global_stats: GlobalRoutingStats::default(),
        }
    }

    /// Add an enhanced route to the table
    pub fn add_route(&mut self, route: EnhancedRouteEntry) {
        // Remove existing route for the same network if it exists
        self.entries.retain(|r| {
            !(r.base.network() == route.base.network() && 
              r.base.interface_id == route.base.interface_id)
        });

        self.entries.push(route);
        self.invalidate_cache();
        self.update_multipath_routes();
        self.apply_aggregation();
    }

    /// Remove a route from the table
    pub fn remove_route(&mut self, destination: Ipv4Addr, netmask: Ipv4Addr, interface_id: u32) -> bool {
        let network = destination.to_u32() & netmask.to_u32();
        let original_len = self.entries.len();

        self.entries.retain(|r| {
            let route_network = r.base.destination.to_u32() & r.base.netmask.to_u32();
            !(route_network == network && r.base.interface_id == interface_id)
        });

        let removed = self.entries.len() < original_len;
        if removed {
            self.invalidate_cache();
            self.update_multipath_routes();
            self.apply_aggregation();
        }
        removed
    }

    /// Find the best route for a destination address
    pub fn lookup_route(&mut self, destination: Ipv4Addr) -> Option<&EnhancedRouteEntry> {
        // Check cache first
        if let Some(cached_result) = self.cache.get(&destination) {
            self.cache_stats.hits.fetch_add(1, Ordering::Relaxed);
            return cached_result.as_ref();
        }

        self.cache_stats.misses.fetch_add(1, Ordering::Relaxed);

        // Check policy routes first
        for policy_route in &self.policy_routes {
            if policy_route.matches(destination) {
                if let Some(route) = self.find_route_by_id(&policy_route.target_route_id) {
                    route.stats.increment_selection();
                    return Some(route);
                }
            }
        }

        // Find matching routes
        let mut matching_routes: Vec<&EnhancedRouteEntry> = self.entries
            .iter()
            .filter(|r| r.base.active && r.base.matches(destination))
            .collect();

        if matching_routes.is_empty() {
            self.cache_entry(destination, None);
            return None;
        }

        // Sort by preference
        matching_routes.sort_by(|a, b| {
            a.effective_metric().cmp(&b.effective_metric())
                .then_with(|| b.base.prefix_len().cmp(&a.base.prefix_len()))
        });

        // Get the best route
        let best_route = matching_routes[0];
        best_route.stats.increment_selection();

        // Cache the result
        self.cache_entry(destination, Some(best_route.clone()));

        Some(best_route)
    }

    /// Find all routes for a destination (for multipath)
    pub fn lookup_all_routes(&mut self, destination: Ipv4Addr) -> Vec<&EnhancedRouteEntry> {
        self.entries
            .iter()
            .filter(|r| r.base.active && r.base.matches(destination))
            .collect()
    }

    /// Get the next hop for a destination with multipath support
    pub fn get_next_hops(&mut self, destination: Ipv4Addr) -> Vec<(Ipv4Addr, u32)> {
        let routes = self.lookup_all_routes(destination);
        let mut next_hops = Vec::new();

        for route in routes {
            if let Some(gateway) = route.base.gateway {
                next_hops.push((gateway, route.base.interface_id));
            } else {
                // Directly connected network
                next_hops.push((destination, route.base.interface_id));
            }
        }

        next_hops
    }

    /// Get a single next hop using load balancing
    pub fn get_next_hop_lb(&mut self, destination: Ipv4Addr) -> Option<(Ipv4Addr, u32)> {
        let next_hops = self.get_next_hops(destination);
        
        if next_hops.is_empty() {
            return None;
        }

        // Simple round-robin load balancing
        let index = self.global_stats.lb_counter.fetch_add(1, Ordering::Relaxed) as usize % next_hops.len();
        Some(next_hops[index].clone())
    }

    /// Add a policy route
    pub fn add_policy_route(&mut self, policy_route: PolicyRoute) {
        self.policy_routes.push(policy_route);
        self.invalidate_cache();
    }

    /// Remove a policy route
    pub fn remove_policy_route(&mut self, route_id: &str) -> bool {
        let original_len = self.policy_routes.len();
        self.policy_routes.retain(|r| r.route_id != route_id);
        let removed = self.policy_routes.len() < original_len;
        if removed {
            self.invalidate_cache();
        }
        removed
    }

    /// Add an aggregation rule
    pub fn add_aggregation_rule(&mut self, rule: AggregationRule) {
        self.aggregation_rules.push(rule);
        self.apply_aggregation();
    }

    /// Remove an aggregation rule
    pub fn remove_aggregation_rule(&mut self, rule_id: &str) -> bool {
        let original_len = self.aggregation_rules.len();
        self.aggregation_rules.retain(|r| r.rule_id != rule_id);
        let removed = self.aggregation_rules.len() < original_len;
        if removed {
            self.apply_aggregation();
        }
        removed
    }

    /// Update route ages
    pub fn update_ages(&mut self) {
        for route in &mut self.entries {
            route.update_age();
        }
    }

    /// Flush old routes based on age
    pub fn flush_old_routes(&mut self, max_age: u64) {
        let original_len = self.entries.len();
        self.entries.retain(|r| r.age <= max_age);
        
        if self.entries.len() < original_len {
            self.invalidate_cache();
            self.update_multipath_routes();
            self.apply_aggregation();
        }
    }

    /// Get all routes
    pub fn routes(&self) -> &[EnhancedRouteEntry] {
        &self.entries
    }

    /// Get routes by source
    pub fn routes_by_source(&self, source: RouteSource) -> Vec<&EnhancedRouteEntry> {
        self.entries
            .iter()
            .filter(|r| r.source == source)
            .collect()
    }

    /// Get routes for a specific interface
    pub fn routes_for_interface(&self, interface_id: u32) -> Vec<&EnhancedRouteEntry> {
        self.entries
            .iter()
            .filter(|r| r.base.interface_id == interface_id)
            .collect()
    }

    /// Flush the routing table
    pub fn flush(&mut self) {
        self.entries.clear();
        self.cache.clear();
        self.multipath_routes.clear();
        self.global_stats = GlobalRoutingStats::default();
    }

    /// Get comprehensive statistics
    pub fn get_comprehensive_stats(&self) -> EnhancedRoutingTableStats {
        let total_routes = self.entries.len();
        let active_routes = self.entries.iter().filter(|r| r.base.active).count();
        let connected_routes = self.entries.iter().filter(|r| r.source == RouteSource::Connected).count();
        let static_routes = self.entries.iter().filter(|r| r.source == RouteSource::Static).count();
        let dynamic_routes = total_routes - connected_routes - static_routes;

        EnhancedRoutingTableStats {
            total_routes,
            active_routes,
            connected_routes,
            static_routes,
            dynamic_routes,
            cache_size: self.cache.len(),
            cache_hits: self.cache_stats.hits.load(Ordering::Relaxed),
            cache_misses: self.cache_stats.misses.load(Ordering::Relaxed),
            multipath_routes: self.multipath_routes.len(),
            policy_routes: self.policy_routes.len(),
            aggregation_rules: self.aggregation_rules.len(),
            total_packets_forwarded: self.global_stats.total_packets_forwarded.load(Ordering::Relaxed),
            total_bytes_forwarded: self.global_stats.total_bytes_forwarded.load(Ordering::Relaxed),
            total_lookups: self.global_stats.total_lookups.load(Ordering::Relaxed),
        }
    }

    /// Find a route by ID
    fn find_route_by_id(&self, route_id: &str) -> Option<&EnhancedRouteEntry> {
        self.entries.iter().find(|r| {
            // For now, use a simple approach - in a real implementation,
            // routes would have unique IDs
            false
        })
    }

    /// Cache a route lookup result
    fn cache_entry(&mut self, destination: Ipv4Addr, route: Option<EnhancedRouteEntry>) {
        // Remove oldest entry if cache is full
        if self.cache.len() >= self.max_cache_size {
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

    /// Update multipath routes
    fn update_multipath_routes(&mut self) {
        self.multipath_routes.clear();
        
        // Group routes by destination network
        let mut network_groups: BTreeMap<u32, Vec<&EnhancedRouteEntry>> = BTreeMap::new();
        
        for route in &self.entries {
            if route.base.active {
                let network = route.base.destination.to_u32() & route.base.netmask.to_u32();
                network_groups.entry(network).or_default().push(route);
            }
        }
        
        // Create multipath entries for networks with multiple routes
        for (network, routes) in network_groups {
            if routes.len() > 1 {
                let destination = Ipv4Addr::from_u32(network);
                self.multipath_routes.insert(destination, routes.into_iter().cloned().collect());
            }
        }
    }

    /// Apply aggregation rules
    fn apply_aggregation(&mut self) {
        for rule in &self.aggregation_rules {
            if rule.enabled {
                self.apply_aggregation_rule(rule);
            }
        }
    }

    /// Apply a single aggregation rule
    fn apply_aggregation_rule(&mut self, rule: &AggregationRule) {
        // Find routes that match aggregation criteria
        let matching_routes: Vec<EnhancedRouteEntry> = self.entries
            .iter()
            .filter(|r| rule.matches_route(r))
            .cloned()
            .collect();

        if matching_routes.len() >= rule.min_routes as usize {
            // Create aggregated route
            if let Some(aggregated_route) = rule.create_aggregated_route(&matching_routes) {
                // Remove original routes
                self.entries.retain(|r| !rule.matches_route(r));
                
                // Add aggregated route
                self.entries.push(aggregated_route);
            }
        }
    }
}

impl Default for EnhancedRoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Default)]
struct CacheStats {
    hits: AtomicU64,
    misses: AtomicU64,
}

/// Global routing statistics
#[derive(Debug, Default)]
struct GlobalRoutingStats {
    total_packets_forwarded: AtomicU64,
    total_bytes_forwarded: AtomicU64,
    total_lookups: AtomicU64,
    lb_counter: AtomicU32, // Load balancing counter
}

/// Enhanced routing table statistics
#[derive(Debug, Clone)]
pub struct EnhancedRoutingTableStats {
    /// Total number of routes
    pub total_routes: usize,
    /// Number of active routes
    pub active_routes: usize,
    /// Number of connected routes
    pub connected_routes: usize,
    /// Number of static routes
    pub static_routes: usize,
    /// Number of dynamic routes
    pub dynamic_routes: usize,
    /// Cache size
    pub cache_size: usize,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Number of multipath routes
    pub multipath_routes: usize,
    /// Number of policy routes
    pub policy_routes: usize,
    /// Number of aggregation rules
    pub aggregation_rules: usize,
    /// Total packets forwarded
    pub total_packets_forwarded: u64,
    /// Total bytes forwarded
    pub total_bytes_forwarded: u64,
    /// Total route lookups
    pub total_lookups: u64,
}

/// Policy route for conditional routing
#[derive(Debug, Clone)]
pub struct PolicyRoute {
    /// Route identifier
    pub route_id: String,
    /// Match conditions
    pub match_conditions: Vec<MatchCondition>,
    /// Route to use when conditions match
    pub target_route_id: String,
    /// Priority (lower is higher priority)
    pub priority: u32,
    /// Policy is active
    pub active: bool,
}

impl PolicyRoute {
    /// Create a new policy route
    pub fn new(route_id: String, match_conditions: Vec<MatchCondition>, target_route_id: String, priority: u32) -> Self {
        Self {
            route_id,
            match_conditions,
            target_route_id,
            priority,
            active: true,
        }
    }

    /// Check if this policy route matches a destination
    pub fn matches(&self, destination: Ipv4Addr) -> bool {
        if !self.active {
            return false;
        }

        self.match_conditions.iter().all(|condition| condition.matches(destination))
    }
}

/// Match condition for policy routing
#[derive(Debug, Clone)]
pub enum MatchCondition {
    /// Match destination address
    Destination(Ipv4Addr, Ipv4Addr), // address, mask
    /// Match source address
    Source(Ipv4Addr, Ipv4Addr), // address, mask
    /// Match interface
    Interface(u32),
    /// Match QoS class
    QoSClass(QoSClass),
    /// Match packet size range
    PacketSize(u16, u16), // min, max
    /// Match protocol
    Protocol(u8),
    /// Match port range
    PortRange(u16, u16), // min, max
}

impl MatchCondition {
    /// Check if this condition matches
    pub fn matches(&self, destination: Ipv4Addr) -> bool {
        match self {
            MatchCondition::Destination(addr, mask) => {
                (destination.to_u32() & mask.to_u32()) == (addr.to_u32() & mask.to_u32())
            }
            MatchCondition::Source(_, _) => {
                // Would need source address in real implementation
                false
            }
            MatchCondition::Interface(_) => {
                // Would need interface context in real implementation
                false
            }
            MatchCondition::QoSClass(_) => {
                // Would need QoS context in real implementation
                false
            }
            MatchCondition::PacketSize(_, _) => {
                // Would need packet context in real implementation
                false
            }
            MatchCondition::Protocol(_) => {
                // Would need protocol context in real implementation
                false
            }
            MatchCondition::PortRange(_, _) => {
                // Would need port context in real implementation
                false
            }
        }
    }
}

/// Route aggregation rule
#[derive(Debug, Clone)]
pub struct AggregationRule {
    /// Rule identifier
    pub rule_id: String,
    /// Minimum number of routes to aggregate
    pub min_routes: u32,
    /// Aggregated network prefix
    pub aggregated_prefix: (Ipv4Addr, Ipv4Addr),
    /// Route source filter
    pub source_filter: Option<RouteSource>,
    /// Interface filter
    pub interface_filter: Option<u32>,
    /// Metric for aggregated route
    pub aggregated_metric: u32,
    /// Rule is enabled
    pub enabled: bool,
}

impl AggregationRule {
    /// Create a new aggregation rule
    pub fn new(
        rule_id: String,
        min_routes: u32,
        aggregated_prefix: (Ipv4Addr, Ipv4Addr),
        aggregated_metric: u32,
    ) -> Self {
        Self {
            rule_id,
            min_routes,
            aggregated_prefix,
            source_filter: None,
            interface_filter: None,
            aggregated_metric,
            enabled: true,
        }
    }

    /// Check if this rule matches a route
    pub fn matches_route(&self, route: &EnhancedRouteEntry) -> bool {
        // Check source filter
        if let Some(source) = self.source_filter {
            if route.source != source {
                return false;
            }
        }

        // Check interface filter
        if let Some(interface_id) = self.interface_filter {
            if route.base.interface_id != interface_id {
                return false;
            }
        }

        true
    }

    /// Create an aggregated route from matching routes
    pub fn create_aggregated_route(&self, matching_routes: &[EnhancedRouteEntry]) -> Option<EnhancedRouteEntry> {
        if matching_routes.is_empty() {
            return None;
        }

        // Create base route entry
        let base = RouteEntry::new(
            self.aggregated_prefix.0,
            self.aggregated_prefix.1,
            None, // No gateway for aggregated routes
            matching_routes[0].base.interface_id,
            self.aggregated_metric,
        );

        // Create enhanced route entry
        let mut enhanced_route = EnhancedRouteEntry::new(base, RouteSource::Static);
        enhanced_route.tag = 0xFFFFFFFF; // Special tag for aggregated routes

        Some(enhanced_route)
    }
}

/// Route redistribution configuration
#[derive(Debug, Clone, Default)]
pub struct RedistributionConfig {
    /// Redistribution rules
    pub rules: Vec<RedistributionRule>,
}

/// Redistribution rule
#[derive(Debug, Clone)]
pub struct RedistributionRule {
    /// Source protocol
    pub from_protocol: RouteSource,
    /// Target protocol
    pub to_protocol: RouteSource,
    /// Route map to apply
    pub route_map: Option<String>,
    /// Metric to apply
    pub metric: Option<u32>,
    /// Tag to apply
    pub tag: Option<u32>,
    /// Rule is enabled
    pub enabled: bool,
}

/// Enhanced route manager for handling multiple routing tables
pub struct EnhancedRouteManager {
    /// Main enhanced routing table
    main_table: EnhancedRoutingTable,
    /// Additional tables (e.g., per-process)
    tables: BTreeMap<String, EnhancedRoutingTable>,
    /// Route redistribution manager
    redistribution_manager: RedistributionManager,
    /// Dynamic routing protocols
    routing_protocols: BTreeMap<String, Box<dyn RoutingProtocol>>,
}

impl EnhancedRouteManager {
    /// Create a new enhanced route manager
    pub fn new() -> Self {
        Self {
            main_table: EnhancedRoutingTable::new(),
            tables: BTreeMap::new(),
            redistribution_manager: RedistributionManager::new(),
            routing_protocols: BTreeMap::new(),
        }
    }

    /// Get the main enhanced routing table
    pub fn main_table(&mut self) -> &mut EnhancedRoutingTable {
        &mut self.main_table
    }

    /// Add a named enhanced routing table
    pub fn add_table(&mut self, name: String, table: EnhancedRoutingTable) {
        self.tables.insert(name, table);
    }

    /// Get a named enhanced routing table
    pub fn get_table(&mut self, name: &str) -> Option<&mut EnhancedRoutingTable> {
        self.tables.get_mut(name)
    }

    /// Remove a named enhanced routing table
    pub fn remove_table(&mut self, name: &str) -> Option<EnhancedRoutingTable> {
        self.tables.remove(name)
    }

    /// Get all table names
    pub fn table_names(&self) -> Vec<&String> {
        self.tables.keys().collect()
    }

    /// Add a routing protocol
    pub fn add_routing_protocol(&mut self, name: String, protocol: Box<dyn RoutingProtocol>) {
        self.routing_protocols.insert(name, protocol);
    }

    /// Remove a routing protocol
    pub fn remove_routing_protocol(&mut self, name: &str) -> Option<Box<dyn RoutingProtocol>> {
        self.routing_protocols.remove(name)
    }

    /// Start all routing protocols
    pub fn start_protocols(&mut self) {
        for (name, protocol) in &mut self.routing_protocols {
            if let Err(e) = protocol.start() {
                log::error!("Failed to start routing protocol {}: {:?}", name, e);
            }
        }
    }

    /// Stop all routing protocols
    pub fn stop_protocols(&mut self) {
        for (name, protocol) in &mut self.routing_protocols {
            if let Err(e) = protocol.stop() {
                log::error!("Failed to stop routing protocol {}: {:?}", name, e);
            }
        }
    }

    /// Update all routing protocols
    pub fn update_protocols(&mut self) {
        for (name, protocol) in &mut self.routing_protocols {
            if let Err(e) = protocol.update() {
                log::error!("Failed to update routing protocol {}: {:?}", name, e);
            }
        }
    }

    /// Perform route redistribution
    pub fn redistribute_routes(&mut self) {
        self.redistribution_manager.redistribute(&mut self.main_table, &mut self.tables);
    }
}

impl Default for EnhancedRouteManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for routing protocols
pub trait RoutingProtocol {
    /// Start the routing protocol
    fn start(&mut self) -> Result<(), RoutingProtocolError>;
    
    /// Stop the routing protocol
    fn stop(&mut self) -> Result<(), RoutingProtocolError>;
    
    /// Update the routing protocol
    fn update(&mut self) -> Result<(), RoutingProtocolError>;
    
    /// Get routes from this protocol
    fn get_routes(&self) -> Vec<EnhancedRouteEntry>;
    
    /// Get protocol statistics
    fn get_stats(&self) -> RoutingProtocolStats;
}

/// Routing protocol errors
#[derive(Debug, Clone)]
pub enum RoutingProtocolError {
    /// Protocol not initialized
    NotInitialized,
    /// Protocol already running
    AlreadyRunning,
    /// Configuration error
    ConfigurationError(String),
    /// Network error
    NetworkError(String),
    /// Protocol error
    ProtocolError(String),
}

/// Routing protocol statistics
#[derive(Debug, Clone)]
pub struct RoutingProtocolStats {
    /// Protocol name
    pub protocol_name: String,
    /// Protocol state
    pub state: ProtocolState,
    /// Number of routes learned
    pub routes_learned: u32,
    /// Number of routes advertised
    pub routes_advertised: u32,
    /// Number of neighbors
    pub neighbors: u32,
    /// Uptime in seconds
    pub uptime: u64,
}

/// Protocol state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolState {
    /// Not started
    NotStarted,
    /// Starting
    Starting,
    /// Running
    Running,
    /// Stopping
    Stopping,
    /// Stopped
    Stopped,
    /// Error
    Error,
}

/// Route redistribution manager
pub struct RedistributionManager {
    /// Redistribution rules
    rules: Vec<RedistributionRule>,
}

impl RedistributionManager {
    /// Create a new redistribution manager
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    /// Add a redistribution rule
    pub fn add_rule(&mut self, rule: RedistributionRule) {
        self.rules.push(rule);
    }

    /// Remove a redistribution rule
    pub fn remove_rule(&mut self, index: usize) -> Option<RedistributionRule> {
        if index < self.rules.len() {
            Some(self.rules.remove(index))
        } else {
            None
        }
    }

    /// Perform route redistribution
    pub fn redistribute(&mut self, main_table: &mut EnhancedRoutingTable, tables: &mut BTreeMap<String, EnhancedRoutingTable>) {
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            // Find source routes
            let source_routes = if rule.from_protocol == RouteSource::Connected {
                main_table.routes_by_source(RouteSource::Connected)
            } else {
                // Find routes from specific protocol tables
                let mut routes = Vec::new();
                for table in tables.values() {
                    routes.extend(table.routes_by_source(rule.from_protocol));
                }
                routes
            };

            // Apply redistribution
            for route in source_routes {
                let mut redistributed_route = route.clone();
                
                // Apply metric
                if let Some(metric) = rule.metric {
                    redistributed_route.base.metric = metric;
                }
                
                // Apply tag
                if let Some(tag) = rule.tag {
                    redistributed_route.tag = tag;
                }
                
                // Add to target table
                if rule.to_protocol == RouteSource::Static {
                    // Add to main table as static route
                    main_table.add_route(redistributed_route);
                }
            }
        }
    }
}

impl Default for RedistributionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global enhanced routing manager instance
static GLOBAL_ENHANCED_ROUTING_MANAGER: once_cell::sync::Lazy<Mutex<EnhancedRouteManager>> = 
    once_cell::sync::Lazy::new(|| Mutex::new(EnhancedRouteManager::new()));

/// Get global enhanced routing manager
pub fn get_global_enhanced_routing_manager() -> &'static Mutex<EnhancedRouteManager> {
    &GLOBAL_ENHANCED_ROUTING_MANAGER
}

/// Initialize enhanced routing subsystem
pub fn init_enhanced_routing() -> Result<(), RoutingProtocolError> {
    let manager = get_global_enhanced_routing_manager();
    let mut manager = manager.lock();
    
    // Start routing protocols
    manager.start_protocols();
    
    log::info!("Enhanced routing subsystem initialized");
    Ok(())
}

/// Enhanced routing utility functions
pub mod utils {
    use super::*;

    /// Calculate route administrative distance
    pub fn calculate_admin_distance(source: RouteSource, metric: u32) -> u8 {
        let base_distance = source.default_admin_distance();
        
        // Adjust based on metric (higher metric = higher distance)
        let adjustment = (metric / 1000) as u8;
        
        base_distance.saturating_add(adjustment)
    }

    /// Validate route entry
    pub fn validate_route(route: &EnhancedRouteEntry) -> bool {
        // Check if destination and netmask are valid
        if route.base.destination == Ipv4Addr::UNSPECIFIED && route.base.netmask != Ipv4Addr::UNSPECIFIED {
            return false;
        }
        
        // Check if netmask is valid
        let mask = route.base.netmask.to_u32();
        let mut has_zero = false;
        let mut has_one = false;
        
        for i in 0..32 {
            if (mask >> (31 - i)) & 1 == 1 {
                has_one = true;
                if has_zero {
                    return false; // Invalid netmask (1s after 0s)
                }
            } else {
                has_zero = true;
            }
        }
        
        // Check if gateway is valid for non-connected routes
        if route.source != RouteSource::Connected && route.base.gateway.is_none() && !route.base.is_default() {
            return false;
        }
        
        true
    }

    /// Calculate route priority
    pub fn calculate_route_priority(route: &EnhancedRouteEntry) -> u32 {
        let mut priority = 0;
        
        // Base priority from administrative distance (inverted, lower is better)
        priority += (255 - route.admin_distance as u32) << 24;
        
        // Add metric contribution
        priority += (0xFFFFFFFF - route.base.metric) & 0x00FFFFFF;
        
        // Add prefix length contribution
        priority += route.base.prefix_len() as u32;
        
        priority
    }

    /// Find common prefix between two networks
    pub fn find_common_prefix(
        addr1: Ipv4Addr,
        mask1: Ipv4Addr,
        addr2: Ipv4Addr,
        mask2: Ipv4Addr,
    ) -> (Ipv4Addr, Ipv4Addr) {
        let net1 = addr1.to_u32() & mask1.to_u32();
        let net2 = addr2.to_u32() & mask2.to_u32();
        
        // Find common bits
        let mut common_bits = 0;
        for i in 0..32 {
            if ((net1 >> (31 - i)) & 1) == ((net2 >> (31 - i)) & 1) {
                common_bits += 1;
            } else {
                break;
            }
        }
        
        // Create common mask
        let common_mask = if common_bits == 0 {
            0
        } else {
            0xFFFFFFFF << (32 - common_bits)
        };
        
        // Create common network
        let common_network = net1 & common_mask;
        
        (
            Ipv4Addr::from_u32(common_network),
            Ipv4Addr::from_u32(common_mask),
        )
    }

    /// Check if route is reachable via interface
    pub fn is_route_reachable_via_interface(
        route: &EnhancedRouteEntry,
        interface_ip: Ipv4Addr,
        interface_netmask: Ipv4Addr,
    ) -> bool {
        // Directly connected routes are always reachable
        if route.source == RouteSource::Connected {
            return true;
        }
        
        // Check if gateway is reachable via interface
        if let Some(gateway) = route.base.gateway {
            let interface_network = interface_ip.to_u32() & interface_netmask.to_u32();
            let gateway_network = gateway.to_u32() & interface_netmask.to_u32();
            
            interface_network == gateway_network
        } else {
            // No gateway, check if destination is directly reachable
            let interface_network = interface_ip.to_u32() & interface_netmask.to_u32();
            let dest_network = route.base.destination.to_u32() & route.base.netmask.to_u32();
            
            interface_network == dest_network
        }
    }

    /// Optimize routing table by removing redundant routes
    pub fn optimize_routing_table(routes: &mut Vec<EnhancedRouteEntry>) {
        // Sort routes by specificity (more specific first)
        routes.sort_by(|a, b| {
            b.base.prefix_len().cmp(&a.base.prefix_len())
                .then_with(|| a.admin_distance.cmp(&b.admin_distance))
        });
        
        // Remove redundant routes
        let mut i = 0;
        while i < routes.len() {
            let mut j = i + 1;
            while j < routes.len() {
                // Check if route j is covered by route i
                if is_route_covered(&routes[j], &routes[i]) {
                    routes.remove(j);
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }

    /// Check if route is covered by another route
    fn is_route_covered(route: &EnhancedRouteEntry, covering_route: &EnhancedRouteEntry) -> bool {
        // Check if covering route has larger prefix (less specific)
        if covering_route.base.prefix_len() >= route.base.prefix_len() {
            return false;
        }
        
        // Check if route destination is within covering route network
        let route_network = route.base.destination.to_u32() & route.base.netmask.to_u32();
        let covering_network = covering_route.base.destination.to_u32() & covering_route.base.netmask.to_u32();
        
        (route_network & covering_route.base.netmask.to_u32()) == covering_network
    }

    /// Calculate route convergence time
    pub fn calculate_convergence_time(
        old_routes: &[EnhancedRouteEntry],
        new_routes: &[EnhancedRouteEntry],
    ) -> u64 {
        // Simple implementation: count differences
        let mut differences = 0;
        
        // Count routes in old but not in new
        for old_route in old_routes {
            if !new_routes.iter().any(|r| {
                r.base.destination == old_route.base.destination &&
                r.base.netmask == old_route.base.netmask &&
                r.base.interface_id == old_route.base.interface_id
            }) {
                differences += 1;
            }
        }
        
        // Count routes in new but not in old
        for new_route in new_routes {
            if !old_routes.iter().any(|r| {
                r.base.destination == new_route.base.destination &&
                r.base.netmask == new_route.base.netmask &&
                r.base.interface_id == new_route.base.interface_id
            }) {
                differences += 1;
            }
        }
        
        // Estimate convergence time based on differences
        // This is a simplified calculation
        differences as u64 * 100 // 100ms per route change
    }

    /// Generate route summary
    pub fn generate_route_summary(routes: &[EnhancedRouteEntry]) -> RouteSummary {
        let mut summary = RouteSummary::default();
        
        for route in routes {
            summary.total_routes += 1;
            
            if route.base.active {
                summary.active_routes += 1;
            }
            
            match route.source {
                RouteSource::Connected => summary.connected_routes += 1,
                RouteSource::Static => summary.static_routes += 1,
                _ => summary.dynamic_routes += 1,
            }
            
            // Count by prefix length
            let prefix_len = route.base.prefix_len();
            if prefix_len <= 8 {
                summary.summary_routes_8 += 1;
            } else if prefix_len <= 16 {
                summary.summary_routes_16 += 1;
            } else if prefix_len <= 24 {
                summary.summary_routes_24 += 1;
            } else {
                summary.host_routes += 1;
            }
            
            // Count by interface
            let interface_id = route.base.interface_id;
            if !summary.routes_per_interface.contains_key(&interface_id) {
                summary.routes_per_interface.insert(interface_id, 0);
            }
            *summary.routes_per_interface.get_mut(&interface_id).unwrap() += 1;
        }
        
        summary
    }
}

/// Route summary information
#[derive(Debug, Clone, Default)]
pub struct RouteSummary {
    /// Total number of routes
    pub total_routes: u32,
    /// Number of active routes
    pub active_routes: u32,
    /// Number of connected routes
    pub connected_routes: u32,
    /// Number of static routes
    pub static_routes: u32,
    /// Number of dynamic routes
    pub dynamic_routes: u32,
    /// Number of /8 or less specific routes
    pub summary_routes_8: u32,
    /// Number of /16 or less specific routes
    pub summary_routes_16: u32,
    /// Number of /24 or less specific routes
    pub summary_routes_24: u32,
    /// Number of host routes (/32)
    pub host_routes: u32,
    /// Routes per interface
    pub routes_per_interface: alloc::collections::BTreeMap<u32, u32>,
}

/// Route change notification
#[derive(Debug, Clone)]
pub struct RouteChangeNotification {
    /// Change type
    pub change_type: RouteChangeType,
    /// Affected route
    pub route: EnhancedRouteEntry,
    /// Previous route (for updates)
    pub previous_route: Option<EnhancedRouteEntry>,
    /// Change timestamp
    pub timestamp: u64,
}

/// Route change types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteChangeType {
    /// Route added
    Added,
    /// Route removed
    Removed,
    /// Route updated
    Updated,
}

/// Route change listener trait
pub trait RouteChangeListener {
    /// Handle route change notification
    fn on_route_change(&self, notification: &RouteChangeNotification);
}

/// Route change notifier
pub struct RouteChangeNotifier {
    /// List of listeners
    listeners: Vec<Box<dyn RouteChangeListener>>,
}

impl RouteChangeNotifier {
    /// Create a new route change notifier
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }

    /// Add a route change listener
    pub fn add_listener(&mut self, listener: Box<dyn RouteChangeListener>) {
        self.listeners.push(listener);
    }

    /// Remove a route change listener
    pub fn remove_listener(&mut self, index: usize) -> Option<Box<dyn RouteChangeListener>> {
        if index < self.listeners.len() {
            Some(self.listeners.remove(index))
        } else {
            None
        }
    }

    /// Notify listeners of route change
    pub fn notify_change(&self, change_type: RouteChangeType, route: &EnhancedRouteEntry, previous_route: Option<&EnhancedRouteEntry>) {
        let notification = RouteChangeNotification {
            change_type,
            route: route.clone(),
            previous_route: previous_route.cloned(),
            timestamp: time::get_monotonic_time(),
        };

        for listener in &self.listeners {
            listener.on_route_change(&notification);
        }
    }
}

impl Default for RouteChangeNotifier {
    fn default() -> Self {
        Self::new()
    }
}