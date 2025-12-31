//! # Trace Context Management
//!
//! This module provides trace context management following the W3C Trace Context
//! standard. It handles trace and span ID generation, context propagation, and
//! storage.
//!
//! # Architecture
//!
//! - **TraceId**: 128-bit trace identifier
//! - **SpanId**: 64-bit span identifier
//! - **SpanContext**: Immutable container for trace identifiers
//! - **ContextStorage**: Thread-local-like storage for context
//! - **TraceState**: Key-value store for trace-specific data
//!
//! # Example
//!
//! ```rust
//! use kernel::tracing::context::SpanContext;
//!
//! // Create a new root context
//! let context = SpanContext::new();
//!
//! // Create a child context
//! let child = SpanContext::child(&context);
//!
//! // Extract from W3C headers
//! let headers = vec![
//!     ("traceparent", "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"),
//! ];
//! let context = SpanContext::from_w3c_headers(&headers).unwrap();
//! ```

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::tracing::TraceError;

/// Trace ID version (W3C Trace Context)
const TRACE_ID_VERSION: u8 = 0;

/// Length of trace ID in hex characters
const TRACE_ID_HEX_LENGTH: usize = 32;

/// Length of span ID in hex characters
const SPAN_ID_HEX_LENGTH: usize = 16;

/// 128-bit trace identifier
///
/// Trace IDs uniquely identify a distributed trace. They are 128-bit values
/// represented as 32 hex characters.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceId(u128);

impl TraceId {
    /// Generate a new random trace ID
    pub fn new() -> Self {
        Self::from_u128(Self::generate_u128())
    }

    /// Create from u128 value
    #[inline]
    pub const fn from_u128(value: u128) -> Self {
        Self(value)
    }

    /// Get the inner u128 value
    #[inline]
    pub const fn as_u128(self) -> u128 {
        self.0
    }

    /// Create from hex string
    pub fn from_hex(hex: &str) -> Result<Self, TraceError> {
        if hex.len() != TRACE_ID_HEX_LENGTH {
            return Err(TraceError::InvalidTraceIdLength {
                expected: TRACE_ID_HEX_LENGTH,
                actual: hex.len(),
            });
        }

        let value = u128::from_str_radix(hex, 16).map_err(|_| TraceError::InvalidTraceIdFormat)?;

        Ok(Self(value))
    }

    /// Convert to hex string
    pub fn to_hex(self) -> String {
        format!("{:032x}", self.0)
    }

    /// Check if trace ID is valid (non-zero)
    #[inline]
    pub fn is_valid(self) -> bool {
        self.0 != 0
    }

    /// Generate random u128
    fn generate_u128() -> u128 {
        // In a real implementation, use a proper CSPRNG
        // For now, use a combination of time and atomic counter
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let counter = COUNTER.fetch_add(1, Ordering::SeqCst) as u128;

        extern "C" {
            fn rust_currrent_time() -> u64;
        }
        let timestamp = unsafe { rust_currrent_time() } as u128;

        (timestamp << 64) | counter
    }

    /// Invalid trace ID (all zeros)
    pub const INVALID: Self = Self(0);
}

impl Default for TraceId {
    fn default() -> Self {
        Self::INVALID
    }
}

impl fmt::Debug for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TraceId({})", self.to_hex())
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// 64-bit span identifier
///
/// Span IDs uniquely identify a span within a trace. They are 64-bit values
/// represented as 16 hex characters.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanId(u64);

impl SpanId {
    /// Generate a new random span ID
    pub fn new() -> Self {
        Self::from_u64(Self::generate_u64())
    }

    /// Create from u64 value
    #[inline]
    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }

    /// Get the inner u64 value
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Create from hex string
    pub fn from_hex(hex: &str) -> Result<Self, TraceError> {
        if hex.len() != SPAN_ID_HEX_LENGTH {
            return Err(TraceError::InvalidSpanIdLength {
                expected: SPAN_ID_HEX_LENGTH,
                actual: hex.len(),
            });
        }

        let value = u64::from_str_radix(hex, 16).map_err(|_| TraceError::InvalidSpanIdFormat)?;

        Ok(Self(value))
    }

    /// Convert to hex string
    pub fn to_hex(self) -> String {
        format!("{:016x}", self.0)
    }

    /// Check if span ID is valid (non-zero)
    #[inline]
    pub fn is_valid(self) -> bool {
        self.0 != 0
    }

    /// Generate random u64
    fn generate_u64() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        extern "C" {
            fn rust_currrent_time() -> u64;
        }
        let timestamp = unsafe { rust_currrent_time() };

        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);

        timestamp.wrapping_mul(31).wrapping_add(counter)
    }

    /// Invalid span ID (all zeros)
    pub const INVALID: Self = Self(0);
}

impl Default for SpanId {
    fn default() -> Self {
        Self::INVALID
    }
}

impl fmt::Debug for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpanId({})", self.to_hex())
    }
}

impl fmt::Display for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Span context contains trace identification information
///
/// SpanContext is an immutable container that includes:
/// - Trace ID: Identifies the distributed trace
/// - Span ID: Identifies this span
/// - Parent Span ID: Identifies the parent span (if any)
/// - Trace flags: Contains sampling and other flags
/// - Trace state: Vendor-specific trace data
///
/// This context is propagated across process boundaries to maintain
/// trace continuity in distributed systems.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpanContext {
    /// Trace identifier
    trace_id: TraceId,

    /// Span identifier
    span_id: SpanId,

    /// Parent span identifier (None for root spans)
    parent_span_id: Option<SpanId>,

    /// Trace flags
    flags: TraceFlags,

    /// Trace state (vendor-specific data)
    trace_state: TraceState,
}

impl SpanContext {
    /// Create a new root span context
    ///
    /// Generates a new trace ID and span ID with no parent.
    pub fn new() -> Self {
        Self {
            trace_id: TraceId::new(),
            span_id: SpanId::new(),
            parent_span_id: None,
            flags: TraceFlags::SAMPLED,
            trace_state: TraceState::new(),
        }
    }

    /// Create a new root span context with custom trace ID
    pub fn with_trace_id(trace_id: TraceId) -> Self {
        Self {
            trace_id,
            span_id: SpanId::new(),
            parent_span_id: None,
            flags: TraceFlags::SAMPLED,
            trace_state: TraceState::new(),
        }
    }

    /// Create a child span context
    ///
    /// Creates a new span context that is a child of the given parent.
    pub fn child(parent: &SpanContext) -> Self {
        Self {
            trace_id: parent.trace_id,
            span_id: SpanId::new(),
            parent_span_id: Some(parent.span_id),
            flags: parent.flags,
            trace_state: parent.trace_state.clone(),
        }
    }

    /// Create span context from components
    pub fn from_parts(
        trace_id: TraceId,
        span_id: SpanId,
        parent_span_id: Option<SpanId>,
        flags: TraceFlags,
    ) -> Self {
        Self {
            trace_id,
            span_id,
            parent_span_id,
            flags,
            trace_state: TraceState::new(),
        }
    }

    /// Get the trace ID
    #[inline]
    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    /// Get the span ID
    #[inline]
    pub fn span_id(&self) -> SpanId {
        self.span_id
    }

    /// Get the parent span ID
    #[inline]
    pub fn parent_span_id(&self) -> Option<SpanId> {
        self.parent_span_id
    }

    /// Get the trace flags
    #[inline]
    pub fn flags(&self) -> TraceFlags {
        self.flags
    }

    /// Check if the span is sampled
    #[inline]
    pub fn is_sampled(&self) -> bool {
        self.flags.is_sampled()
    }

    /// Set the sampled flag
    #[inline]
    pub fn set_sampled(&mut self, sampled: bool) {
        self.flags.set_sampled(sampled);
    }

    /// Get the trace state
    #[inline]
    pub fn trace_state(&self) -> &TraceState {
        &self.trace_state
    }

    /// Get mutable reference to trace state
    #[inline]
    pub fn trace_state_mut(&mut self) -> &mut TraceState {
        &mut self.trace_state
    }

    /// Check if context is valid (has valid trace and span IDs)
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.trace_id.is_valid() && self.span_id.is_valid()
    }

    /// Create from W3C traceparent header value
    ///
    /// Format: `version-trace_id-span_id-parent_id-flags`
    /// Example: `00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01`
    pub fn from_w3c_header(value: &str) -> Result<Self, TraceError> {
        let parts: Vec<&str> = value.split('-').collect();
        if parts.len() != 4 {
            return Err(TraceError::InvalidTraceParentHeader);
        }

        // Parse version
        let version = u8::from_str_radix(parts[0], 16)
            .map_err(|_| TraceError::InvalidTraceParentVersion)?;
        if version != TRACE_ID_VERSION {
            return Err(TraceError::UnsupportedTraceVersion(version));
        }

        // Parse trace ID
        let trace_id = TraceId::from_hex(parts[1])?;

        // Parse span ID
        let span_id = SpanId::from_hex(parts[2])?;

        // Parse flags
        let flags = TraceFlags::from_u8(u8::from_str_radix(parts[3], 16)
            .map_err(|_| TraceError::InvalidTraceFlags)?);

        Ok(Self {
            trace_id,
            span_id,
            parent_span_id: None,
            flags,
            trace_state: TraceState::new(),
        })
    }

    /// Convert to W3C traceparent header value
    ///
    /// Returns the header value without the "traceparent:" prefix
    pub fn to_w3c_header(&self) -> String {
        format!(
            "{:02x}-{}-{}-{:02x}",
            TRACE_ID_VERSION,
            self.trace_id.to_hex(),
            self.span_id.to_hex(),
            self.flags.as_u8()
        )
    }

    /// Extract context from HTTP headers
    ///
    /// Looks for W3C Trace Context headers (traceparent and tracestate)
    pub fn from_w3c_headers(headers: &[(impl AsRef<str>, impl AsRef<str>)]) -> Result<Self, TraceError> {
        let mut traceparent = None;
        let mut tracestate_values = Vec::new();

        for (key, value) in headers {
            let key_str = key.as_ref().to_lowercase();
            let value_str = value.as_ref();

            if key_str == "traceparent" {
                traceparent = Some(value_str);
            } else if key_str == "tracestate" {
                tracestate_values.push(value_str.to_string());
            }
        }

        let context = traceparent
            .ok_or(TraceError::MissingTraceParent)?
            .and_then(|tp| Self::from_w3c_header(tp))?;

        // Parse tracestate if present
        if !tracestate_values.is_empty() {
            let state = TraceState::from_headers(&tracestate_values)?;
            // Note: We'd need to return context with state, but context is immutable
            // In real implementation, would reconstruct with state
        }

        Ok(context)
    }

    /// Inject context into HTTP headers
    ///
    /// Returns W3C Trace Context headers
    pub fn to_w3c_headers(&self) -> Vec<(String, String)> {
        let mut headers = Vec::with_capacity(2);
        headers.push(("traceparent".to_string(), self.to_w3c_header()));

        if !self.trace_state.is_empty() {
            headers.push(("tracestate".to_string(), self.trace_state.to_header()));
        }

        headers
    }

    /// Create a remote span context
    ///
    /// Used when receiving context from a remote service
    pub fn remote(
        trace_id: TraceId,
        span_id: SpanId,
        flags: TraceFlags,
    ) -> Self {
        Self {
            trace_id,
            span_id,
            parent_span_id: None,
            flags,
            trace_state: TraceState::new(),
        }
    }
}

impl Default for SpanContext {
    fn default() -> Self {
        Self {
            trace_id: TraceId::default(),
            span_id: SpanId::default(),
            parent_span_id: None,
            flags: TraceFlags::default(),
            trace_state: TraceState::default(),
        }
    }
}

/// Trace flags (W3C Trace Context)
///
/// Trace flags contain metadata about the trace, such as whether it is sampled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TraceFlags(u8);

impl TraceFlags {
    /// Create new trace flags
    #[inline]
    pub const fn new(flags: u8) -> Self {
        Self(flags)
    }

    /// Get the raw u8 value
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }

    /// Create from u8 value
    #[inline]
    pub const fn from_u8(value: u8) -> Self {
        Self(value)
    }

    /// Check if sampled flag is set
    #[inline]
    pub const fn is_sampled(self) -> bool {
        self.0 & 0x01 != 0
    }

    /// Set the sampled flag
    #[inline]
    pub fn set_sampled(&mut self, sampled: bool) {
        if sampled {
            self.0 |= 0x01;
        } else {
            self.0 &= !0x01;
        }
    }

    /// Create with sampled flag
    #[inline]
    pub const fn sampled() -> Self {
        Self(0x01)
    }

    /// Create with unsampled flag
    #[inline]
    pub const fn unsampled() -> Self {
        Self(0x00)
    }
}

impl Default for TraceFlags {
    fn default() -> Self {
        Self::SAMPLED
    }
}

impl TraceFlags {
    /// Default sampled flags
    pub const SAMPLED: Self = Self(0x01);
}

/// Trace state (W3C Trace Context)
///
/// TraceState is a vendor-specific key-value store for trace data.
/// It's used to propagate vendor-specific data across process boundaries.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TraceState {
    /// Key-value pairs
    entries: Vec<(String, String)>,
}

impl TraceState {
    /// Create new empty trace state
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(4),
        }
    }

    /// Add a key-value pair
    ///
    /// # Returns
    ///
    /// Ok(()) if added, Err if limit exceeded or invalid key
    pub fn put(&mut self, key: impl Into<String>, value: impl Into<String>) -> Result<(), TraceError> {
        let key_str = key.into();
        self.validate_key(&key_str)?;

        // Remove existing key if present
        self.entries.retain(|(k, _)| k != &key_str);

        self.entries.push((key_str, value.into()));
        Ok(())
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// Delete a key
    pub fn delete(&mut self, key: &str) -> bool {
        let original_len = self.entries.len();
        self.entries.retain(|(k, _)| k != key);
        self.entries.len() < original_len
    }

    /// Get all entries
    pub fn entries(&self) -> &[(String, String)] {
        &self.entries
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Validate a key
    fn validate_key(&self, key: &str) -> Result<(), TraceError> {
        // W3C Trace Context specification:
        // - Key must match: [a-z0-9]{1,256} or [a-z0-9]{1,14}@[a-z0-9]{1,14}
        // - Maximum 32 entries
        // - Maximum 512 characters per value

        if self.entries.len() >= 32 {
            return Err(TraceError::TraceStateLimitExceeded);
        }

        if key.len() > 256 {
            return Err(TraceError::InvalidTraceStateKey(key.to_string()));
        }

        if !key.is_ascii() || !key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '@' || c == '-' || c == '_') {
            return Err(TraceError::InvalidTraceStateKey(key.to_string()));
        }

        Ok(())
    }

    /// Parse from tracestate headers
    ///
    /// Tracestate header format: `vendor1=value1,vendor2=value2`
    pub fn from_headers(values: &[String]) -> Result<Self, TraceError> {
        let mut state = Self::new();

        for value in values {
            for entry in value.split(',') {
                let entry = entry.trim();
                let parts: Vec<&str> = entry.splitn(2, '=').collect();
                if parts.len() != 2 {
                    continue;
                }

                let key = parts[0].trim();
                let value = parts[1].trim();

                if value.len() > 512 {
                    return Err(TraceError::TraceStateValueTooLong);
                }

                // Don't fail on duplicate keys, just skip
                let _ = state.put(key, value);
            }
        }

        Ok(state)
    }

    /// Convert to tracestate header value
    pub fn to_header(&self) -> String {
        self.entries
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// Context storage for thread-local-like behavior
///
/// In a no_std kernel environment, we use a task-local storage mechanism.
/// This is a simplified version that would be replaced with proper TLS
/// in a full implementation.
pub struct ContextStorage {
    /// Current span context
    current_context: Option<SpanContext>,
}

impl ContextStorage {
    /// Create new context storage
    pub fn new() -> Self {
        Self {
            current_context: None,
        }
    }

    /// Get the current span context
    pub fn get(&self) -> Option<&SpanContext> {
        self.current_context.as_ref()
    }

    /// Set the current span context
    pub fn set(&mut self, context: SpanContext) {
        self.current_context = Some(context);
    }

    /// Clear the current span context
    pub fn clear(&mut self) {
        self.current_context = None;
    }

    /// Execute a function with a specific context
    pub fn with_context<F, R>(&mut self, context: &SpanContext, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let previous = self.current_context.clone();
        self.current_context = Some(context.clone());
        let result = f();
        self.current_context = previous;
        result
    }
}

impl Default for ContextStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_id_generation() {
        let id1 = TraceId::new();
        let id2 = TraceId::new();

        assert!(id1.is_valid());
        assert!(id2.is_valid());
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_trace_id_hex_conversion() {
        let id = TraceId::from_u128(0x4bf92f3577b34da6a3ce929d0e0e4736);
        assert_eq!(id.to_hex(), "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(TraceId::from_hex("4bf92f3577b34da6a3ce929d0e0e4736"), Ok(id));
    }

    #[test]
    fn test_trace_id_invalid() {
        assert!(!TraceId::INVALID.is_valid());
        assert!(!TraceId::from_u128(0).is_valid());
    }

    #[test]
    fn test_span_id_generation() {
        let id1 = SpanId::new();
        let id2 = SpanId::new();

        assert!(id1.is_valid());
        assert!(id2.is_valid());
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_span_id_hex_conversion() {
        let id = SpanId::from_u64(0x00f067aa0ba902b7);
        assert_eq!(id.to_hex(), "00f067aa0ba902b7");
        assert_eq!(SpanId::from_hex("00f067aa0ba902b7"), Ok(id));
    }

    #[test]
    fn test_span_context_creation() {
        let context = SpanContext::new();

        assert!(context.is_valid());
        assert!(context.is_sampled());
        assert_eq!(context.parent_span_id(), None);
    }

    #[test]
    fn test_child_span_context() {
        let parent = SpanContext::new();
        let child = SpanContext::child(&parent);

        assert_eq!(child.trace_id(), parent.trace_id());
        assert_ne!(child.span_id(), parent.span_id());
        assert_eq!(child.parent_span_id(), Some(parent.span_id()));
        assert_eq!(child.is_sampled(), parent.is_sampled());
    }

    #[test]
    fn test_w3c_traceparent() {
        let header = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let context = SpanContext::from_w3c_header(header).unwrap();

        assert_eq!(context.trace_id(), TraceId::from_hex("4bf92f3577b34da6a3ce929d0e0e4736").unwrap());
        assert_eq!(context.span_id(), SpanId::from_hex("00f067aa0ba902b7").unwrap());
        assert!(context.is_sampled());

        let output = context.to_w3c_header();
        assert_eq!(output, header);
    }

    #[test]
    fn test_w3c_headers_roundtrip() {
        let original = SpanContext::new();
        let headers = original.to_w3c_headers();

        let restored = SpanContext::from_w3c_headers(&headers).unwrap();
        assert_eq!(restored.trace_id(), original.trace_id());
        assert_eq!(restored.span_id(), original.span_id());
        assert_eq!(restored.is_sampled(), original.is_sampled());
    }

    #[test]
    fn test_trace_flags() {
        let flags = TraceFlags::sampled();
        assert!(flags.is_sampled());

        let mut flags = TraceFlags::unsampled();
        assert!(!flags.is_sampled());
        flags.set_sampled(true);
        assert!(flags.is_sampled());
    }

    #[test]
    fn test_trace_state() {
        let mut state = TraceState::new();
        assert!(state.is_empty());

        state.put("vendor1", "value1").unwrap();
        state.put("vendor2", "value2").unwrap();

        assert_eq!(state.len(), 2);
        assert_eq!(state.get("vendor1"), Some("value1"));
        assert_eq!(state.get("vendor2"), Some("value2"));

        state.delete("vendor1");
        assert_eq!(state.len(), 1);
        assert_eq!(state.get("vendor1"), None);
    }

    #[test]
    fn test_trace_state_limits() {
        let mut state = TraceState::new();

        // Should succeed up to 32 entries
        for i in 0..32 {
            assert!(state.put(format!("vendor{}", i), format!("value{}", i)).is_ok());
        }

        // Should fail on 33rd entry
        assert!(matches!(
            state.put("vendor33", "value33"),
            Err(TraceError::TraceStateLimitExceeded)
        ));
    }

    #[test]
    fn test_trace_state_header() {
        let mut state = TraceState::new();
        state.put("vendor1", "value1").unwrap();
        state.put("vendor2", "value2").unwrap();

        let header = state.to_header();
        assert!(header.contains("vendor1=value1"));
        assert!(header.contains("vendor2=value2"));

        let parsed = TraceState::from_headers(&[header]).unwrap();
        assert_eq!(parsed.get("vendor1"), Some("value1"));
        assert_eq!(parsed.get("vendor2"), Some("value2"));
    }

    #[test]
    fn test_context_storage() {
        let mut storage = ContextStorage::new();
        assert!(storage.get().is_none());

        let context = SpanContext::new();
        storage.set(context.clone());
        assert!(storage.get().is_some());

        storage.clear();
        assert!(storage.get().is_none());
    }

    #[test]
    fn test_context_with_scope() {
        let mut storage = ContextStorage::new();
        let context1 = SpanContext::new();
        let context2 = SpanContext::new();

        let result = storage.with_context(&context1, || {
            assert_eq!(storage.get().unwrap().trace_id(), context1.trace_id());
            storage.with_context(&context2, || {
                assert_eq!(storage.get().unwrap().trace_id(), context2.trace_id());
                42
            })
        });

        assert_eq!(result, 42);
        assert!(storage.get().is_none());
    }
}
