//! Structured metadata for log records
//!
//! This module provides facilities for attaching structured metadata
//! to log records:
//! - Key-value metadata pairs
//! - Request ID tracking for distributed tracing
//! - Trace ID propagation
//! - User context tracking
//! - Component context

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use core::hash::Hash;
use core::sync::atomic::{AtomicU64, Ordering};

/// Metadata container for structured logging
#[derive(Clone, Debug, Default)]
pub struct Metadata {
    /// Key-value pairs
    data: BTreeMap<String, String>,
}

impl Metadata {
    /// Create a new empty metadata container
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    /// Insert a key-value pair
    pub fn insert(&mut self, key: String, value: String) {
        self.data.insert(key, value);
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }

    /// Remove a key-value pair
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.data.remove(key)
    }

    /// Check if a key exists
    pub fn contains(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Iterate over all entries
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.data.iter()
    }

    /// Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.data.keys()
    }

    /// Get all values
    pub fn values(&self) -> impl Iterator<Item = &String> {
        self.data.values()
    }

    /// Merge another metadata into this one
    pub fn merge(&mut self, other: Metadata) {
        for (key, value) in other.data {
            self.data.insert(key, value);
        }
    }

    /// Convert to map
    pub fn to_map(&self) -> BTreeMap<String, String> {
        self.data.clone()
    }

    /// Create from map
    pub fn from_map(map: BTreeMap<String, String>) -> Self {
        Self { data: map }
    }
}

/// Metadata builder for fluent construction
pub struct MetadataBuilder {
    metadata: Metadata,
}

impl MetadataBuilder {
    /// Create a new metadata builder
    pub fn new() -> Self {
        Self {
            metadata: Metadata::new(),
        }
    }

    /// Add a key-value pair
    pub fn add(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add a string key-value pair
    pub fn add_str(self, key: &str, value: &str) -> Self {
        self.add(key.to_string(), value.to_string())
    }

    /// Add an integer value
    pub fn add_i64(self, key: String, value: i64) -> Self {
        self.add(key, value.to_string())
    }

    /// Add an unsigned integer value
    pub fn add_u64(self, key: String, value: u64) -> Self {
        self.add(key, value.to_string())
    }

    /// Add a boolean value
    pub fn add_bool(self, key: String, value: bool) -> Self {
        self.add(key, value.to_string())
    }

    /// Add a float value
    pub fn add_f64(self, key: String, value: f64) -> Self {
        self.add(key, value.to_string())
    }

    /// Build the metadata
    pub fn build(self) -> Metadata {
        self.metadata
    }
}

impl Default for MetadataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Request ID generator for tracking requests
#[derive(Debug)]
pub struct RequestIdGenerator {
    /// Counter for generating unique IDs
    counter: AtomicU64,
    /// Node ID (for distributed systems)
    node_id: u64,
}

impl RequestIdGenerator {
    /// Create a new request ID generator
    pub fn new(node_id: u64) -> Self {
        Self {
            counter: AtomicU64::new(0),
            node_id,
        }
    }

    /// Generate a new request ID
    pub fn generate(&self) -> RequestId {
        let counter = self.counter.fetch_add(1, Ordering::Relaxed);
        RequestId::new(self.node_id, counter)
    }

    /// Parse a request ID from string
    pub fn parse(s: &str) -> Option<RequestId> {
        RequestId::from_str(s)
    }
}

/// Request ID for tracking individual requests
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RequestId {
    /// Node ID
    node_id: u64,
    /// Counter value
    counter: u64,
}

impl RequestId {
    /// Create a new request ID
    pub fn new(node_id: u64, counter: u64) -> Self {
        Self { node_id, counter }
    }

    /// Create from string
    pub fn from_str(s: &str) -> Option<Self> {
        // Format: "req-{node_id}-{counter}"
        if !s.starts_with("req-") {
            return None;
        }

        let parts: Vec<&str> = s[4..].split('-').collect();
        if parts.len() != 2 {
            return None;
        }

        let node_id = u64::from_str_radix(parts.get(0)?, 16).ok()?;
        let counter = u64::from_str_radix(parts.get(1)?, 16).ok()?;

        Some(Self { node_id, counter })
    }

    /// Get node ID
    pub fn node_id(&self) -> u64 {
        self.node_id
    }

    /// Get counter value
    pub fn counter(&self) -> u64 {
        self.counter
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        format!("req-{:x}-{:x}", self.node_id, self.counter)
    }

    /// Create a child request ID (for nested calls)
    pub fn child(&self) -> RequestId {
        // Increment the counter for child requests
        RequestId {
            node_id: self.node_id,
            counter: self.counter + 1,
        }
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Trace ID for distributed tracing
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TraceId {
    /// High 64 bits
    high: u64,
    /// Low 64 bits
    low: u64,
}

impl TraceId {
    /// Generate a new random trace ID
    pub fn new() -> Self {
        // In a real implementation, this would use a proper RNG
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let val = COUNTER.fetch_add(1, Ordering::Relaxed);

        Self {
            high: val,
            low: val.wrapping_mul(0x517cc1b727220a95),
        }
    }

    /// Create from bytes (16 bytes)
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        let high = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let low = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
        Self { high, low }
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..8].copy_from_slice(&self.high.to_be_bytes());
        bytes[8..16].copy_from_slice(&self.low.to_be_bytes());
        bytes
    }

    /// Parse from string (hex format)
    pub fn from_str(s: &str) -> Option<Self> {
        // Format: 32 hex characters
        if s.len() != 32 {
            return None;
        }

        let high = u64::from_str_radix(&s[0..16], 16).ok()?;
        let low = u64::from_str_radix(&s[16..32], 16).ok()?;

        Some(Self { high, low })
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        format!("{:032x}", self.high as u128 * (u128::MAX / 2 + 1) + self.low as u128)
    }

    /// Check if trace ID is valid (non-zero)
    pub fn is_valid(&self) -> bool {
        self.high != 0 || self.low != 0
    }

    /// Create an invalid (zero) trace ID
    pub fn invalid() -> Self {
        Self { high: 0, low: 0 }
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

/// Span ID for individual spans within a trace
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpanId(u64);

impl SpanId {
    /// Generate a new span ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Create from raw value
    pub fn from_u64(val: u64) -> Self {
        Self(val)
    }

    /// Get the raw value
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        format!("{:016x}", self.0)
    }

    /// Parse from hex string
    pub fn from_str(s: &str) -> Option<Self> {
        if s.len() != 16 {
            return None;
        }
        u64::from_str_radix(s, 16).ok().map(Self)
    }

    /// Check if valid (non-zero)
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl fmt::Display for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl Default for SpanId {
    fn default() -> Self {
        Self::new()
    }
}

/// Trace context for distributed tracing
#[derive(Clone, Debug)]
pub struct TraceContext {
    /// Trace ID
    trace_id: TraceId,
    /// Parent span ID
    parent_span_id: Option<SpanId>,
    /// Current span ID
    span_id: SpanId,
    /// Trace flags
    flags: u8,
}

impl TraceContext {
    /// Create a new trace context (new trace)
    pub fn new() -> Self {
        Self {
            trace_id: TraceId::new(),
            parent_span_id: None,
            span_id: SpanId::new(),
            flags: 0,
        }
    }

    /// Create a child context (new span in existing trace)
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            parent_span_id: Some(self.span_id),
            span_id: SpanId::new(),
            flags: self.flags,
        }
    }

    /// Create from trace ID and span ID
    pub fn from_ids(trace_id: TraceId, span_id: SpanId) -> Self {
        Self {
            trace_id,
            parent_span_id: None,
            span_id,
            flags: 0,
        }
    }

    /// Get trace ID
    pub fn trace_id(&self) -> &TraceId {
        &self.trace_id
    }

    /// Get parent span ID
    pub fn parent_span_id(&self) -> Option<SpanId> {
        self.parent_span_id
    }

    /// Get current span ID
    pub fn span_id(&self) -> SpanId {
        self.span_id
    }

    /// Check if sampled
    pub fn is_sampled(&self) -> bool {
        self.flags & 0x01 != 0
    }

    /// Set sampled flag
    pub fn set_sampled(&mut self, sampled: bool) {
        if sampled {
            self.flags |= 0x01;
        } else {
            self.flags &= !0x01;
        }
    }

    /// Get flags
    pub fn flags(&self) -> u8 {
        self.flags
    }

    /// Convert to W3C traceparent format
    pub fn to_traceparent(&self) -> String {
        format!(
            "00-{}-{}-{:02x}",
            self.trace_id.to_hex(),
            self.span_id.to_hex(),
            self.flags
        )
    }

    /// Parse from W3C traceparent format
    pub fn from_traceparent(s: &str) -> Option<Self> {
        // Format: 00-{trace_id}-{span_id}-{flags}
        if !s.starts_with("00-") {
            return None;
        }

        let parts: Vec<&str> = s[3..].split('-').collect();
        if parts.len() != 3 {
            return None;
        }

        let trace_id = TraceId::from_str(parts.get(0)?)?;
        let span_id = SpanId::from_str(parts.get(1)?)?;
        let flags = u8::from_str_radix(parts.get(2)?, 16).ok()?;

        Some(Self {
            trace_id,
            parent_span_id: None,
            span_id,
            flags,
        })
    }

    /// Inject into metadata
    pub fn inject_into_metadata(&self, metadata: &mut Metadata) {
        metadata.insert("trace_id".to_string(), self.trace_id.to_hex());
        metadata.insert("span_id".to_string(), self.span_id.to_hex());
        if let Some(parent) = self.parent_span_id {
            metadata.insert("parent_span_id".to_string(), parent.to_hex());
        }
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}

/// User context for tracking user information
#[derive(Clone, Debug, Default)]
pub struct UserContext {
    /// User ID
    user_id: Option<String>,
    /// User name
    user_name: Option<String>,
    /// User email
    user_email: Option<String>,
    /// User roles
    roles: Vec<String>,
    /// Session ID
    session_id: Option<String>,
    /// Additional metadata
    metadata: Metadata,
}

impl UserContext {
    /// Create a new user context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set user ID
    pub fn with_user_id(mut self, id: String) -> Self {
        self.user_id = Some(id);
        self
    }

    /// Set user name
    pub fn with_user_name(mut self, name: String) -> Self {
        self.user_name = Some(name);
        self
    }

    /// Set user email
    pub fn with_user_email(mut self, email: String) -> Self {
        self.user_email = Some(email);
        self
    }

    /// Add a role
    pub fn with_role(mut self, role: String) -> Self {
        self.roles.push(role);
        self
    }

    /// Set session ID
    pub fn with_session_id(mut self, id: String) -> Self {
        self.session_id = Some(id);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata.merge(metadata);
        self
    }

    /// Get user ID
    pub fn user_id(&self) -> Option<&String> {
        self.user_id.as_ref()
    }

    /// Get user name
    pub fn user_name(&self) -> Option<&String> {
        self.user_name.as_ref()
    }

    /// Get user email
    pub fn user_email(&self) -> Option<&String> {
        self.user_email.as_ref()
    }

    /// Get roles
    pub fn roles(&self) -> &[String] {
        &self.roles
    }

    /// Get session ID
    pub fn session_id(&self) -> Option<&String> {
        self.session_id.as_ref()
    }

    /// Check if user has a role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Inject into metadata
    pub fn inject_into_metadata(&self, metadata: &mut Metadata) {
        if let Some(user_id) = &self.user_id {
            metadata.insert("user_id".to_string(), user_id.clone());
        }
        if let Some(user_name) = &self.user_name {
            metadata.insert("user_name".to_string(), user_name.clone());
        }
        if let Some(user_email) = &self.user_email {
            metadata.insert("user_email".to_string(), user_email.clone());
        }
        if let Some(session_id) = &self.session_id {
            metadata.insert("session_id".to_string(), session_id.clone());
        }
        if !self.roles.is_empty() {
            metadata.insert("user_roles".to_string(), self.roles.join(","));
        }
        // Merge additional metadata
        metadata.merge(self.metadata.clone());
    }
}

/// Component context for tracking component information
#[derive(Clone, Debug, Default)]
pub struct ComponentContext {
    /// Component name
    component_name: Option<String>,
    /// Component version
    component_version: Option<String>,
    /// Instance ID
    instance_id: Option<String>,
    /// Hostname
    hostname: Option<String>,
    /// Additional metadata
    metadata: Metadata,
}

impl ComponentContext {
    /// Create a new component context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set component name
    pub fn with_component_name(mut self, name: String) -> Self {
        self.component_name = Some(name);
        self
    }

    /// Set component version
    pub fn with_component_version(mut self, version: String) -> Self {
        self.component_version = Some(version);
        self
    }

    /// Set instance ID
    pub fn with_instance_id(mut self, id: String) -> Self {
        self.instance_id = Some(id);
        self
    }

    /// Set hostname
    pub fn with_hostname(mut self, hostname: String) -> Self {
        self.hostname = Some(hostname);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata.merge(metadata);
        self
    }

    /// Inject into metadata
    pub fn inject_into_metadata(&self, metadata: &mut Metadata) {
        if let Some(name) = &self.component_name {
            metadata.insert("component_name".to_string(), name.clone());
        }
        if let Some(version) = &self.component_version {
            metadata.insert("component_version".to_string(), version.clone());
        }
        if let Some(instance_id) = &self.instance_id {
            metadata.insert("instance_id".to_string(), instance_id.clone());
        }
        if let Some(hostname) = &self.hostname {
            metadata.insert("hostname".to_string(), hostname.clone());
        }
        metadata.merge(self.metadata.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_basic() {
        let mut metadata = Metadata::new();
        assert!(metadata.is_empty());

        metadata.insert("key1".to_string(), "value1".to_string());
        metadata.insert("key2".to_string(), "value2".to_string());

        assert_eq!(metadata.len(), 2);
        assert!(metadata.contains("key1"));
        assert_eq!(metadata.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_metadata_builder() {
        let metadata = MetadataBuilder::new()
            .add_str("string", "value")
            .add_i64("int".to_string(), 42)
            .add_bool("bool".to_string(), true)
            .add_f64("float".to_string(), 3.14)
            .build();

        assert_eq!(metadata.len(), 4);
        assert_eq!(metadata.get("string"), Some(&"value".to_string()));
        assert_eq!(metadata.get("int"), Some(&"42".to_string()));
        assert_eq!(metadata.get("bool"), Some(&"true".to_string()));
    }

    #[test]
    fn test_metadata_merge() {
        let mut metadata1 = Metadata::new();
        metadata1.insert("key1".to_string(), "value1".to_string());

        let mut metadata2 = Metadata::new();
        metadata2.insert("key2".to_string(), "value2".to_string());

        metadata1.merge(metadata2);
        assert_eq!(metadata1.len(), 2);
        assert!(metadata1.contains("key2"));
    }

    #[test]
    fn test_request_id() {
        let generator = RequestIdGenerator::new(1);
        let id1 = generator.generate();
        let id2 = generator.generate();

        assert_eq!(id1.node_id(), 1);
        assert_eq!(id2.node_id(), 1);
        assert_eq!(id2.counter(), id1.counter() + 1);

        let child = id1.child();
        assert_eq!(child.node_id(), id1.node_id());
        assert_eq!(child.counter(), id1.counter() + 1);
    }

    #[test]
    fn test_request_id_string() {
        let id = RequestId::new(0x1234, 0x5678);
        let s = id.to_string();
        assert_eq!(s, "req-1234-5678");

        let parsed = RequestId::from_str(&s).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn test_trace_id() {
        let id1 = TraceId::new();
        let id2 = TraceId::new();

        assert!(id1.is_valid());
        assert!(id2.is_valid());
        assert_ne!(id1, id2);

        let invalid = TraceId::invalid();
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_trace_id_bytes() {
        let id = TraceId::new();
        let bytes = id.to_bytes();
        let id2 = TraceId::from_bytes(bytes);

        assert_eq!(id, id2);
    }

    #[test]
    fn test_trace_id_hex() {
        let id = TraceId::new();
        let hex = id.to_hex();
        assert_eq!(hex.len(), 32);

        let parsed = TraceId::from_str(&hex).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_span_id() {
        let id1 = SpanId::new();
        let id2 = SpanId::new();

        assert!(id1.is_valid());
        assert!(id2.is_valid());
        assert_ne!(id1, id2);

        let id3 = SpanId::from_u64(42);
        assert_eq!(id3.as_u64(), 42);
    }

    #[test]
    fn test_trace_context() {
        let context = TraceContext::new();
        assert!(context.trace_id().is_valid());
        assert!(context.span_id().is_valid());
        assert!(!context.is_sampled());

        let child = context.child();
        assert_eq!(child.trace_id(), context.trace_id());
        assert_eq!(child.parent_span_id(), Some(context.span_id()));
        assert_ne!(child.span_id(), context.span_id());
    }

    #[test]
    fn test_trace_context_traceparent() {
        let context = TraceContext::new();
        let traceparent = context.to_traceparent();
        assert!(traceparent.starts_with("00-"));

        let parsed = TraceContext::from_traceparent(&traceparent).unwrap();
        assert_eq!(parsed.trace_id(), context.trace_id());
        assert_eq!(parsed.span_id(), context.span_id());
    }

    #[test]
    fn test_trace_context_inject() {
        let context = TraceContext::new();
        let mut metadata = Metadata::new();
        context.inject_into_metadata(&mut metadata);

        assert!(metadata.contains("trace_id"));
        assert!(metadata.contains("span_id"));
    }

    #[test]
    fn test_user_context() {
        let context = UserContext::new()
            .with_user_id("user123".to_string())
            .with_user_name("John Doe".to_string())
            .with_role("admin".to_string())
            .with_role("user".to_string());

        assert_eq!(context.user_id(), Some(&"user123".to_string()));
        assert!(context.has_role("admin"));
        assert!(context.has_role("user"));
        assert!(!context.has_role("guest"));
    }

    #[test]
    fn test_user_context_inject() {
        let context = UserContext::new()
            .with_user_id("user123".to_string())
            .with_user_email("user@example.com".to_string());

        let mut metadata = Metadata::new();
        context.inject_into_metadata(&mut metadata);

        assert_eq!(
            metadata.get("user_id"),
            Some(&"user123".to_string())
        );
        assert_eq!(
            metadata.get("user_email"),
            Some(&"user@example.com".to_string())
        );
    }

    #[test]
    fn test_component_context() {
        let context = ComponentContext::new()
            .with_component_name("my-service".to_string())
            .with_component_version("1.0.0".to_string())
            .with_hostname("host1".to_string());

        let mut metadata = Metadata::new();
        context.inject_into_metadata(&mut metadata);

        assert_eq!(
            metadata.get("component_name"),
            Some(&"my-service".to_string())
        );
        assert_eq!(
            metadata.get("component_version"),
            Some(&"1.0.0".to_string())
        );
        assert_eq!(metadata.get("hostname"), Some(&"host1".to_string()));
    }

    #[test]
    fn test_metadata_iteration() {
        let mut metadata = Metadata::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        metadata.insert("key2".to_string(), "value2".to_string());
        metadata.insert("key3".to_string(), "value3".to_string());

        let keys: Vec<&String> = metadata.keys().collect();
        assert_eq!(keys.len(), 3);

        let values: Vec<&String> = metadata.values().collect();
        assert_eq!(values.len(), 3);

        let count = metadata.iter().count();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_metadata_remove() {
        let mut metadata = Metadata::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        metadata.insert("key2".to_string(), "value2".to_string());

        let removed = metadata.remove("key1");
        assert_eq!(removed, Some("value1".to_string()));
        assert!(!metadata.contains("key1"));
        assert_eq!(metadata.len(), 1);
    }

    #[test]
    fn test_metadata_clear() {
        let mut metadata = Metadata::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        metadata.insert("key2".to_string(), "value2".to_string());

        metadata.clear();
        assert!(metadata.is_empty());
        assert_eq!(metadata.len(), 0);
    }

    #[test]
    fn test_trace_context_sampled() {
        let mut context = TraceContext::new();
        assert!(!context.is_sampled());

        context.set_sampled(true);
        assert!(context.is_sampled());

        context.set_sampled(false);
        assert!(!context.is_sampled());
    }
}
