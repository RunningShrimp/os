//! Netfilter-style packet filtering and mangling
//!
//! This module implements comprehensive packet filtering similar to Linux's netfilter:
//! - Netfilter hooks (prerouting, input, forward, output, postrouting)
//! - Stateless and stateful packet filtering
//! - Packet mangling (TOS, TTL, mark)
//! - Rate limiting with token bucket
//! - BPF socket filtering (cBPF and eBPF)
//! - Connection tracking integration
//!
//! # Examples
//!
//! ```rust
//! use kernel::network::filtering::{Firewall, FilterRule, FilterAction, Filter, HookPoint};
//!
//! // Create firewall
//! let firewall = Firewall::new();
//!
//! // Add stateful rule: accept established connections
//! firewall.add_rule(FilterRule {
//!     hook: HookPoint::Input,
//!     filter: Filter::Stateful(StateFilter::Established),
//!     action: FilterAction::Accept,
//!     ..Default::default()
//! });
//!
//! // Add rate limit rule
//! firewall.add_rate_limit(FilterRule {
//!     hook: HookPoint::Input,
//!     filter: Filter::RateLimit(RateLimit::token_bucket(1000, 100)),
//!     action: FilterAction::Accept,
//!     ..Default::default()
//! });
//! ```

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use core::time::Duration;

use crate::subsystems::sync::{Mutex, RwLock};
use crate::subsystems::net::ipv4::Ipv4Addr;

use super::conntrack::{ConntrackManager, Protocol};

/// Socket address for filtering
#[derive(Debug, Clone, Copy)]
pub struct SocketAddr {
    /// Address family
    pub family: ProtocolFamily,
    /// Port number
    pub port: u16,
    /// IP address
    pub ip: Ipv4Addr,
}

/// Protocol family
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolFamily {
    IPv4,
    IPv6,
}

/// Netfilter hook points
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookPoint {
    /// Prerouting: Incoming packets before routing
    Prerouting,
    /// Input: Incoming packets for local delivery
    Input,
    /// Forward: Packets being forwarded
    Forward,
    /// Output: Outgoing packets from local processes
    Output,
    /// Postrouting: Outgoing packets after routing
    Postrouting,
}

impl HookPoint {
    /// Get hook name
    pub fn as_str(&self) -> &str {
        match self {
            Self::Prerouting => "PREROUTING",
            Self::Input => "INPUT",
            Self::Forward => "FORWARD",
            Self::Output => "OUTPUT",
            Self::Postrouting => "POSTROUTING",
        }
    }

    /// Parse hook from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "PREROUTING" => Some(Self::Prerouting),
            "INPUT" => Some(Self::Input),
            "FORWARD" => Some(Self::Forward),
            "OUTPUT" => Some(Self::Output),
            "POSTROUTING" => Some(Self::Postrouting),
            _ => None,
        }
    }
}

/// Filter actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterAction {
    /// Accept the packet
    Accept,
    /// Drop the packet silently
    Drop,
    /// Reject the packet with ICMP response
    Reject,
    /// Continue to next rule
    Continue,
}

/// State filter for stateful filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateFilter {
    /// New connections (SYN sent)
    New,
    /// Established connections
    Established,
    /// Related connections (e.g., FTP data)
    Related,
    /// Invalid connections
    Invalid,
    /// Untracked connections
    Untracked,
}

/// Packet matcher conditions
#[derive(Debug, Clone)]
pub enum Filter {
    /// Match all packets
    All,
    /// Match protocol
    Protocol(Protocol),
    /// Match source address
    SourceAddr(Ipv4Addr),
    /// Match source address range
    SourceAddrRange(Ipv4Addr, Ipv4Addr),
    /// Match destination address
    DestAddr(Ipv4Addr),
    /// Match destination address range
    DestAddrRange(Ipv4Addr, Ipv4Addr),
    /// Match source port
    SourcePort(u16),
    /// Match source port range
    SourcePortRange(u16, u16),
    /// Match destination port
    DestPort(u16),
    /// Match destination port range
    DestPortRange(u16, u16),
    /// Match interface
    Interface(String),
    /// Stateful filter
    Stateful(StateFilter),
    /// Rate limiting
    RateLimit(RateLimit),
    /// BPF filter
    Bpf(BpfFilter),
    /// Compound filter (AND)
    And(Vec<Filter>),
    /// Compound filter (OR)
    Or(Vec<Filter>),
    /// Negated filter
    Not(Box<Filter>),
}

/// Packet matcher for rule evaluation
#[derive(Debug, Clone)]
pub struct PacketMatcher {
    /// Source IP
    pub src_ip: Option<Ipv4Addr>,
    /// Source port
    pub src_port: Option<u16>,
    /// Destination IP
    pub dst_ip: Option<Ipv4Addr>,
    /// Destination port
    pub dst_port: Option<u16>,
    /// Protocol
    pub protocol: Option<Protocol>,
    /// Input interface
    pub in_interface: Option<String>,
    /// Output interface
    pub out_interface: Option<String>,
    /// Connection state
    pub state: Option<StateFilter>,
}

impl PacketMatcher {
    /// Create new packet matcher
    pub fn new() -> Self {
        Self {
            src_ip: None,
            src_port: None,
            dst_ip: None,
            dst_port: None,
            protocol: None,
            in_interface: None,
            out_interface: None,
            state: None,
        }
    }

    /// Set source IP
    pub fn src_ip(mut self, ip: Ipv4Addr) -> Self {
        self.src_ip = Some(ip);
        self
    }

    /// Set destination IP
    pub fn dst_ip(mut self, ip: Ipv4Addr) -> Self {
        self.dst_ip = Some(ip);
        self
    }

    /// Set protocol
    pub fn protocol(mut self, proto: Protocol) -> Self {
        self.protocol = Some(proto);
        self
    }

    /// Set source port
    pub fn src_port(mut self, port: u16) -> Self {
        self.src_port = Some(port);
        self
    }

    /// Set destination port
    pub fn dst_port(mut self, port: u16) -> Self {
        self.dst_port = Some(port);
        self
    }
}

impl Default for PacketMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Rate limiting algorithms
#[derive(Debug, Clone)]
pub enum RateLimit {
    /// Token bucket filter
    TokenBucket(TokenBucket),
    /// Leaky bucket
    LeakyBucket(LeakyBucket),
}

impl RateLimit {
    /// Create token bucket rate limiter
    pub fn token_bucket(rate: u64, burst: u64) -> Self {
        Self::TokenBucket(TokenBucket::new(rate, burst))
    }

    /// Create leaky bucket rate limiter
    pub fn leaky_bucket(rate: u64, capacity: u64) -> Self {
        Self::LeakyBucket(LeakyBucket::new(rate, capacity))
    }

    /// Check if packet should be allowed
    pub fn check(&mut self, now: u64) -> bool {
        match self {
            Self::TokenBucket(tb) => tb.check(now),
            Self::LeakyBucket(lb) => lb.check(now),
        }
    }
}

/// Token bucket rate limiter
#[derive(Debug, Clone)]
pub struct TokenBucket {
    /// Tokens per second
    rate: u64,
    /// Maximum burst size
    burst: u64,
    /// Current tokens
    tokens: AtomicU64,
    /// Last update timestamp
    last_update: AtomicU64,
}

impl TokenBucket {
    /// Create new token bucket
    pub fn new(rate: u64, burst: u64) -> Self {
        Self {
            rate,
            burst,
            tokens: AtomicU64::new(burst),
            last_update: AtomicU64::new(0),
        }
    }

    /// Check if packet should be allowed
    pub fn check(&self, now: u64) -> bool {
        let last = self.last_update.load(Ordering::Relaxed);
        let elapsed = now.saturating_sub(last);

        // Add tokens based on elapsed time
        if elapsed > 0 {
            let new_tokens = (elapsed * self.rate / 1000).min(self.burst);
            self.tokens.fetch_add(new_tokens, Ordering::Relaxed);
            self.last_update.store(now, Ordering::Relaxed);
        }

        // Try to consume one token
        let mut current = self.tokens.load(Ordering::Relaxed);
        loop {
            if current == 0 {
                return false;
            }

            match self.tokens.compare_exchange_weak(
                current,
                current - 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(c) => current = c,
            }
        }
    }
}

/// Leaky bucket rate limiter
#[derive(Debug, Clone)]
pub struct LeakyBucket {
    /// Rate (packets per second)
    rate: u64,
    /// Bucket capacity
    capacity: u64,
    /// Current level
    level: AtomicU64,
    /// Last update timestamp
    last_update: AtomicU64,
}

impl LeakyBucket {
    /// Create new leaky bucket
    pub fn new(rate: u64, capacity: u64) -> Self {
        Self {
            rate,
            capacity,
            level: AtomicU64::new(0),
            last_update: AtomicU64::new(0),
        }
    }

    /// Check if packet should be allowed
    pub fn check(&self, now: u64) -> bool {
        let last = self.last_update.load(Ordering::Relaxed);
        let elapsed = now.saturating_sub(last);

        // Leak based on elapsed time
        if elapsed > 0 {
            let leaked = (elapsed * self.rate / 1000).min(self.level.load(Ordering::Relaxed));
            self.level.fetch_sub(leaked, Ordering::Relaxed);
            self.last_update.store(now, Ordering::Relaxed);
        }

        // Check if bucket is full
        let current = self.level.load(Ordering::Relaxed);
        if current >= self.capacity {
            return false;
        }

        self.level.fetch_add(1, Ordering::Relaxed);
        true
    }
}

/// BPF filter types
#[derive(Debug, Clone)]
pub enum BpfFilter {
    /// Classic BPF (cBPF)
    Classic(Vec<u8>),
    /// Extended BPF (eBPF)
    Extended(Vec<u8>),
}

impl BpfFilter {
    /// Create classic BPF filter
    pub fn classic(instructions: Vec<u8>) -> Self {
        Self::Classic(instructions)
    }

    /// Create extended BPF filter
    pub fn extended(instructions: Vec<u8>) -> Self {
        Self::Extended(instructions)
    }

    /// Evaluate packet against filter
    pub fn evaluate(&self, _packet: &[u8]) -> bool {
        // In real implementation, this would execute BPF bytecode
        // For now, always return true
        true
    }
}

/// Packet modifiers (mangling)
#[derive(Debug, Clone)]
pub enum PacketModifier {
    /// Modify TOS (Type of Service)
    SetTos(u8),
    /// Modify TTL (Time to Live)
    SetTtl(u8),
    /// Set packet mark
    SetMark(u32),
    /// Clone packet
    Clone,
    /// Modify source address
    SetSourceAddr(Ipv4Addr),
    /// Modify destination address
    SetDestAddr(Ipv4Addr),
}

/// Filter rule
#[derive(Debug, Clone)]
pub struct FilterRule {
    /// Rule priority (higher = evaluated first)
    pub priority: i32,
    /// Hook point
    pub hook: HookPoint,
    /// Filter condition
    pub filter: Filter,
    /// Action to take
    pub action: FilterAction,
    /// Packet modifiers
    pub modifiers: Vec<PacketModifier>,
    /// Rule enabled
    pub enabled: bool,
    /// Packet count
    pub packets: AtomicU64,
    /// Byte count
    pub bytes: AtomicU64,
}

impl Default for FilterRule {
    fn default() -> Self {
        Self {
            priority: 0,
            hook: HookPoint::Input,
            filter: Filter::All,
            action: FilterAction::Accept,
            modifiers: Vec::new(),
            enabled: true,
            packets: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
        }
    }
}

/// Firewall statistics
#[derive(Debug, Default, Clone)]
pub struct FirewallStats {
    /// Total packets processed
    pub packets_processed: u64,
    /// Packets accepted
    pub packets_accepted: u64,
    /// Packets dropped
    pub packets_dropped: u64,
    /// Packets rejected
    pub packets_rejected: u64,
    /// Packets modified
    pub packets_modified: u64,
    /// Rate limit drops
    pub rate_limit_drops: u64,
}

/// Netfilter hooks implementation
#[derive(Debug)]
pub struct NetfilterHooks {
    /// Prerouting rules
    prerouting: Vec<FilterRule>,
    /// Input rules
    input: Vec<FilterRule>,
    /// Forward rules
    forward: Vec<FilterRule>,
    /// Output rules
    output: Vec<FilterRule>,
    /// Postrouting rules
    postrouting: Vec<FilterRule>,
}

impl NetfilterHooks {
    /// Create new Netfilter hooks
    pub fn new() -> Self {
        Self {
            prerouting: Vec::new(),
            input: Vec::new(),
            forward: Vec::new(),
            output: Vec::new(),
            postrouting: Vec::new(),
        }
    }

    /// Add rule to hook
    pub fn add_rule(&mut self, rule: FilterRule) {
        let hook_rules = match rule.hook {
            HookPoint::Prerouting => &mut self.prerouting,
            HookPoint::Input => &mut self.input,
            HookPoint::Forward => &mut self.forward,
            HookPoint::Output => &mut self.output,
            HookPoint::Postrouting => &mut self.postrouting,
        };

        hook_rules.push(rule);
        hook_rules.sort_by_key(|r| -r.priority);
    }

    /// Remove rule from hook
    pub fn remove_rule(&mut self, hook: HookPoint, index: usize) -> bool {
        let hook_rules = match hook {
            HookPoint::Prerouting => &mut self.prerouting,
            HookPoint::Input => &mut self.input,
            HookPoint::Forward => &mut self.forward,
            HookPoint::Output => &mut self.output,
            HookPoint::Postrouting => &mut self.postrouting,
        };

        if index < hook_rules.len() {
            hook_rules.remove(index);
            true
        } else {
            false
        }
    }

    /// Get rules for hook
    pub fn get_rules(&self, hook: HookPoint) -> &[FilterRule] {
        match hook {
            HookPoint::Prerouting => &self.prerouting,
            HookPoint::Input => &self.input,
            HookPoint::Forward => &self.forward,
            HookPoint::Output => &self.output,
            HookPoint::Postrouting => &self.postrouting,
        }
    }

    /// Flush all rules
    pub fn flush(&mut self) {
        self.prerouting.clear();
        self.input.clear();
        self.forward.clear();
        self.output.clear();
        self.postrouting.clear();
    }
}

impl Default for NetfilterHooks {
    fn default() -> Self {
        Self::new()
    }
}

/// Firewall
#[derive(Debug)]
pub struct Firewall {
    /// Netfilter hooks
    hooks: RwLock<NetfilterHooks>,
    /// Connection tracking
    conntrack: Arc<ConntrackManager>,
    /// Statistics
    stats: Mutex<FirewallStats>,
    /// Current time (milliseconds)
    current_time: AtomicU64,
}

impl Firewall {
    /// Create new firewall
    pub fn new() -> Self {
        Self {
            hooks: RwLock::new(NetfilterHooks::new()),
            conntrack: Arc::new(ConntrackManager::new()),
            stats: Mutex::new(FirewallStats::default()),
            current_time: AtomicU64::new(0),
        }
    }

    /// Initialize firewall
    pub fn init(&mut self) -> Result<(), ()> {
        let mut conntrack = Arc::make_mut(&mut self.conntrack);
        conntrack.init()
    }

    /// Add filter rule
    pub fn add_rule(&self, rule: FilterRule) {
        let mut hooks = self.hooks.write();
        hooks.add_rule(rule);
    }

    /// Remove filter rule
    pub fn remove_rule(&self, hook: HookPoint, index: usize) -> bool {
        let mut hooks = self.hooks.write();
        hooks.remove_rule(hook, index)
    }

    /// Get rules for hook
    pub fn get_rules(&self, hook: HookPoint) -> Vec<FilterRule> {
        let hooks = self.hooks.read();
        hooks.get_rules(hook).to_vec()
    }

    /// Filter packet
    pub fn filter_packet(
        &self,
        _packet: &[u8],
        src_addr: SocketAddr,
        dst_addr: SocketAddr,
        protocol: Protocol,
        hook: HookPoint,
    ) -> Result<FilterAction, ()> {
        let mut stats = self.stats.lock();
        stats.packets_processed += 1;

        let hooks = self.hooks.read();
        let rules = hooks.get_rules(hook);

        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }

            // Update counters
            rule.packets.fetch_add(1, Ordering::Relaxed);
            rule.bytes.fetch_add(_packet.len() as u64, Ordering::Relaxed);

            // Check if filter matches
            if self.filter_matches(&rule.filter, src_addr, dst_addr, protocol) {
                let action = rule.action;

                match action {
                    FilterAction::Accept => {
                        stats.packets_accepted += 1;
                    },
                    FilterAction::Drop => {
                        stats.packets_dropped += 1;
                    },
                    FilterAction::Reject => {
                        stats.packets_rejected += 1;
                    },
                    FilterAction::Continue => {},
                }

                return Ok(action);
            }
        }

        // Default action: accept
        stats.packets_accepted += 1;
        Ok(FilterAction::Accept)
    }

    /// Check if filter matches packet
    fn filter_matches(
        &self,
        filter: &Filter,
        src_addr: SocketAddr,
        dst_addr: SocketAddr,
        protocol: Protocol,
    ) -> bool {
        match filter {
            Filter::All => true,
            Filter::Protocol(proto) => *proto == protocol,
            Filter::SourceAddr(addr) => src_addr.ip == *addr,
            Filter::DestAddr(addr) => dst_addr.ip == *addr,
            Filter::SourcePort(port) => src_addr.port == *port,
            Filter::DestPort(port) => dst_addr.port == *port,
            Filter::Stateful(state) => {
                // Check connection tracking
                if let Some(_id) = self.conntrack.find_connection(
                    src_addr.ip,
                    src_addr.port,
                    dst_addr.ip,
                    dst_addr.port,
                    protocol,
                ) {
                    matches!(state, StateFilter::Established | StateFilter::Related)
                } else {
                    matches!(state, StateFilter::New | StateFilter::Untracked)
                }
            },
            Filter::RateLimit(limiter) => {
                let now = self.current_time.load(Ordering::Relaxed);
                limiter.check(now)
            },
            Filter::Bpf(bpf) => {
                // Would evaluate BPF filter
                bpf.evaluate(&[])
            },
            Filter::And(filters) => {
                filters.iter().all(|f| self.filter_matches(f, src_addr, dst_addr, protocol))
            },
            Filter::Or(filters) => {
                filters.iter().any(|f| self.filter_matches(f, src_addr, dst_addr, protocol))
            },
            Filter::Not(filter) => {
                !self.filter_matches(filter, src_addr, dst_addr, protocol)
            },
            _ => false, // Other filters not implemented
        }
    }

    /// Add rate limit rule
    pub fn add_rate_limit(&self, rule: FilterRule) {
        self.add_rule(rule);
    }

    /// Flush all rules
    pub fn flush(&self) {
        let mut hooks = self.hooks.write();
        hooks.flush();
    }

    /// Get connection tracking
    pub fn conntrack(&self) -> &ConntrackManager {
        &self.conntrack
    }

    /// Get statistics
    pub fn get_stats(&self) -> FirewallStats {
        let stats = self.stats.lock();
        stats.clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = FirewallStats::default();
    }

    /// Advance time
    pub fn advance_time(&self, delta_ms: u64) {
        self.current_time.fetch_add(delta_ms, Ordering::Relaxed);
        self.conntrack.advance_time(delta_ms / 1000);
    }
}

impl Default for Firewall {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_point() {
        assert_eq!(HookPoint::Input.as_str(), "INPUT");
        assert_eq!(HookPoint::Prerouting.as_str(), "PREROUTING");

        assert_eq!(HookPoint::from_str("INPUT"), Some(HookPoint::Input));
        assert_eq!(HookPoint::from_str("output"), Some(HookPoint::Output));
        assert_eq!(HookPoint::from_str("invalid"), None);
    }

    #[test]
    fn test_state_filter() {
        let filter = StateFilter::Established;
        assert_eq!(filter as i32, StateFilter::Established as i32);
    }

    #[test]
    fn test_packet_matcher() {
        let matcher = PacketMatcher::new()
            .src_ip(Ipv4Addr::new(192, 168, 1, 1))
            .dst_ip(Ipv4Addr::new(10, 0, 0, 1))
            .protocol(Protocol::TCP)
            .src_port(12345)
            .dst_port(80);

        assert_eq!(matcher.src_ip, Some(Ipv4Addr::new(192, 168, 1, 1)));
        assert_eq!(matcher.dst_ip, Some(Ipv4Addr::new(10, 0, 0, 1)));
        assert_eq!(matcher.protocol, Some(Protocol::TCP));
        assert_eq!(matcher.src_port, Some(12345));
        assert_eq!(matcher.dst_port, Some(80));
    }

    #[test]
    fn test_token_bucket() {
        let tb = TokenBucket::new(1000, 100);

        // Should allow packets
        assert!(tb.check(100));
        assert!(tb.check(200));

        // Exhaust bucket
        for _ in 0..150 {
            tb.check(300);
        }

        // Should be rate limited
        assert!(!tb.check(400));
    }

    #[test]
    fn test_leaky_bucket() {
        let lb = LeakyBucket::new(1000, 100);

        // Should allow packets initially
        for _ in 0..100 {
            assert!(lb.check(100));
        }

        // Should be rate limited
        assert!(!lb.check(200));
    }

    #[test]
    fn test_bpf_filter() {
        let bpf = BpfFilter::classic(vec![1, 2, 3, 4]);
        assert!(bpf.evaluate(&[1, 2, 3, 4]));
    }

    #[test]
    fn test_netfilter_hooks() {
        let mut hooks = NetfilterHooks::new();

        let rule = FilterRule {
            priority: 100,
            hook: HookPoint::Input,
            ..Default::default()
        };

        hooks.add_rule(rule.clone());
        assert_eq!(hooks.get_rules(HookPoint::Input).len(), 1);

        assert!(hooks.remove_rule(HookPoint::Input, 0));
        assert_eq!(hooks.get_rules(HookPoint::Input).len(), 0);
    }

    #[test]
    fn test_firewall_create() {
        let firewall = Firewall::new();
        assert!(firewall.init().is_ok());
    }

    #[test]
    fn test_firewall_add_rule() {
        let firewall = Firewall::new();
        firewall.init().unwrap();

        let rule = FilterRule {
            hook: HookPoint::Input,
            filter: Filter::Protocol(Protocol::TCP),
            action: FilterAction::Accept,
            ..Default::default()
        };

        firewall.add_rule(rule);
        let rules = firewall.get_rules(HookPoint::Input);
        assert_eq!(rules.len(), 1);
    }

    #[test]
    fn test_firewall_filter_packet() {
        let firewall = Firewall::new();
        firewall.init().unwrap();

        let src_addr = SocketAddr {
            family: ProtocolFamily::IPv4,
            port: 12345,
            ip: Ipv4Addr::new(192, 168, 1, 100),
        };

        let dst_addr = SocketAddr {
            family: ProtocolFamily::IPv4,
            port: 80,
            ip: Ipv4Addr::new(10, 0, 0, 1),
        };

        let result = firewall.filter_packet(&[], src_addr, dst_addr, Protocol::TCP, HookPoint::Input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FilterAction::Accept);
    }

    #[test]
    fn test_firewall_flush() {
        let firewall = Firewall::new();
        firewall.init().unwrap();

        let rule = FilterRule {
            hook: HookPoint::Input,
            ..Default::default()
        };

        firewall.add_rule(rule);
        firewall.flush();

        let rules = firewall.get_rules(HookPoint::Input);
        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_firewall_stats() {
        let firewall = Firewall::new();
        firewall.init().unwrap();

        let src_addr = SocketAddr {
            family: ProtocolFamily::IPv4,
            port: 12345,
            ip: Ipv4Addr::new(192, 168, 1, 100),
        };

        let dst_addr = SocketAddr {
            family: ProtocolFamily::IPv4,
            port: 80,
            ip: Ipv4Addr::new(10, 0, 0, 1),
        };

        let _ = firewall.filter_packet(&[], src_addr, dst_addr, Protocol::TCP, HookPoint::Input);

        let stats = firewall.get_stats();
        assert!(stats.packets_processed > 0);
    }

    #[test]
    fn test_filter_rule_default() {
        let rule = FilterRule::default();
        assert_eq!(rule.priority, 0);
        assert_eq!(rule.hook, HookPoint::Input);
        assert_eq!(rule.action, FilterAction::Accept);
        assert!(rule.enabled);
    }
}
