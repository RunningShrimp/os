//! # Context Propagation
//!
//! This module handles context propagation across process and service boundaries.
//! It implements multiple propagation formats including W3C Trace Context and B3.
//!
//! # Architecture
//!
//! - **TextMapPropagator**: Propagates context through text-based headers
//! - **BinaryPropagator**: Propagates context through binary format
//! - **W3CPropagator**: W3C Trace Context standard propagation
//! - **B3Propagator**: B3 propagation format (single-header and multi-header)
//! - **Injector**: Injects context into carriers
//! - **Extractor**: Extracts context from carriers
//!
//! # Example
//!
//! ```rust
//! use kernel::tracing::propagation::{W3CPropagator, TextMapPropagator};
//! use kernel::tracing::context::SpanContext;
//!
//! let propagator = W3CPropagator::new();
//! let context = SpanContext::new();
//!
//! // Inject into HTTP headers
//! let mut headers = Vec::new();
//! propagator.inject(&context, &mut headers);
//!
//! // Extract from HTTP headers
//! let extracted = propagator.extract(&headers).unwrap();
//! ```

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::tracing::context::{SpanContext, SpanId, TraceId};
use crate::tracing::TraceError;

/// Maximum number of headers to process during extraction
const MAX_HEADERS: usize = 64;

/// Maximum header value length
const MAX_HEADER_VALUE_LENGTH: usize = 4096;

/// Propagator trait for context propagation
///
/// A propagator injects and extracts trace context from carriers (e.g., HTTP headers).
pub trait Propagator<C>: Send + Sync {
    /// Inject context into a carrier
    fn inject(&self, context: &SpanContext, carrier: &mut C) -> Result<(), TraceError>;

    /// Extract context from a carrier
    fn extract(&self, carrier: &C) -> Result<Option<SpanContext>, TraceError>;
}

/// Text-based carrier (e.g., HTTP headers)
///
/// TextMap carriers are key-value collections where both keys and values are strings.
pub trait TextMapCarrier {
    /// Get a value by key
    fn get(&self, key: &str) -> Option<String>;

    /// Set a key-value pair
    fn set(&mut self, key: impl Into<String>, value: impl Into<String>);

    /// Get all keys
    fn keys(&self) -> Vec<String>;

    /// Remove a key
    fn remove(&mut self, key: &str);
}

/// Vec-based text map carrier for HTTP-like headers
#[derive(Clone, Debug, Default)]
pub struct HeaderCarrier {
    headers: Vec<(String, String)>,
}

impl HeaderCarrier {
    pub fn new() -> Self {
        Self {
            headers: Vec::with_capacity(8),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            headers: Vec::with_capacity(capacity),
        }
    }

    pub fn from_headers(headers: Vec<(String, String)>) -> Self {
        Self { headers }
    }

    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }
}

impl TextMapCarrier for HeaderCarrier {
    fn get(&self, key: &str) -> Option<String> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
    }

    fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key_str = key.into();
        // Remove existing key if present
        self.headers.retain(|(k, _)| k.to_lowercase() != key_str.to_lowercase());
        self.headers.push((key_str, value.into()));
    }

    fn keys(&self) -> Vec<String> {
        self.headers.iter().map(|(k, _)| k.clone()).collect()
    }

    fn remove(&mut self, key: &str) {
        self.headers.retain(|(k, _)| k.to_lowercase() != key.to_lowercase());
    }
}

/// W3C Trace Context propagator
///
/// Implements the W3C Trace Context standard for context propagation.
/// Uses `traceparent` and `tracestate` headers.
#[derive(Clone, Debug, Default)]
pub struct W3CPropagator {
    _private: (),
}

impl W3CPropagator {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl<C> Propagator<C> for W3CPropagator
where
    C: TextMapCarrier,
{
    fn inject(&self, context: &SpanContext, carrier: &mut C) -> Result<(), TraceError> {
        let traceparent = context.to_w3c_header();
        carrier.set("traceparent", traceparent);

        if !context.trace_state().is_empty() {
            let tracestate = context.trace_state().to_header();
            carrier.set("tracestate", tracestate);
        }

        Ok(())
    }

    fn extract(&self, carrier: &C) -> Result<Option<SpanContext>, TraceError> {
        let traceparent = match carrier.get("traceparent") {
            Some(value) => value,
            None => return Ok(None),
        };

        // Validate header value length
        if traceparent.len() > MAX_HEADER_VALUE_LENGTH {
            return Err(TraceError::HeaderTooLong);
        }

        let mut context = SpanContext::from_w3c_header(&traceparent)?;

        // Extract tracestate if present
        if let Some(tracestate_value) = carrier.get("tracestate") {
            if tracestate_value.len() > MAX_HEADER_VALUE_LENGTH {
                return Err(TraceError::HeaderTooLong);
            }

            let trace_state = crate::tracing::context::TraceState::from_headers(&[tracestate_value])?;
            *context.trace_state_mut() = trace_state;
        }

        Ok(Some(context))
    }
}

/// B3 propagator (multiple header format)
///
/// Implements the B3 propagation format with multiple headers:
/// - X-B3-TraceId: 128-bit or 64-bit trace ID
/// - X-B3-SpanId: 64-bit span ID
/// - X-B3-ParentSpanId: 64-bit parent span ID (optional)
/// - X-B3-Sampled: 1 or 0 for sampled/not sampled
/// - X-B3-Flags: debug flag
#[derive(Clone, Debug)]
pub struct B3Propagator {
    /// Use 128-bit trace IDs
    use_128bit_trace_id: bool,

    /// Include parent span ID
    include_parent_span_id: bool,
}

impl B3Propagator {
    pub fn new() -> Self {
        Self {
            use_128bit_trace_id: true,
            include_parent_span_id: true,
        }
    }

    /// Use 64-bit trace IDs
    pub fn with_64bit_trace_id(mut self) -> Self {
        self.use_128bit_trace_id = false;
        self
    }

    /// Exclude parent span ID from propagation
    pub fn without_parent_span_id(mut self) -> Self {
        self.include_parent_span_id = false;
        self
    }
}

impl Default for B3Propagator {
    fn default() -> Self {
        Self::new()
    }
}

impl<C> Propagator<C> for B3Propagator
where
    C: TextMapCarrier,
{
    fn inject(&self, context: &SpanContext, carrier: &mut C) -> Result<(), TraceError> {
        // Inject trace ID
        let trace_id_hex = if self.use_128bit_trace_id {
            context.trace_id().to_hex()
        } else {
            // Use lower 64 bits
            format!("{:016x}", context.trace_id().as_u128() as u64)
        };
        carrier.set("x-b3-traceid", trace_id_hex);

        // Inject span ID
        carrier.set("x-b3-spanid", context.span_id().to_hex());

        // Inject parent span ID if present and enabled
        if self.include_parent_span_id {
            if let Some(parent_id) = context.parent_span_id() {
                carrier.set("x-b3-parentspanid", parent_id.to_hex());
            }
        }

        // Inject sampled flag
        let sampled = if context.is_sampled() { "1" } else { "0" };
        carrier.set("x-b3-sampled", sampled);

        Ok(())
    }

    fn extract(&self, carrier: &C) -> Result<Option<SpanContext>, TraceError> {
        let trace_id_str = match carrier.get("x-b3-traceid") {
            Some(id) => id,
            None => return Ok(None),
        };

        if trace_id_str.len() > MAX_HEADER_VALUE_LENGTH {
            return Err(TraceError::HeaderTooLong);
        }

        // Parse trace ID
        let trace_id = if self.use_128bit_trace_id {
            TraceId::from_hex(&trace_id_str)?
        } else {
            // 64-bit trace ID, pad to 128-bit
            let id_64 = u64::from_str_radix(&trace_id_str, 16)
                .map_err(|_| TraceError::InvalidTraceIdFormat)?;
            TraceId::from_u128(id_64 as u128)
        };

        // Parse span ID
        let span_id_str = carrier
            .get("x-b3-spanid")
            .ok_or(TraceError::MissingSpanId)?;
        if span_id_str.len() > MAX_HEADER_VALUE_LENGTH {
            return Err(TraceError::HeaderTooLong);
        }
        let span_id = SpanId::from_hex(&span_id_str)?;

        // Parse parent span ID (optional)
        let parent_span_id = if self.include_parent_span_id {
            if let Some(parent_str) = carrier.get("x-b3-parentspanid") {
                if parent_str.len() > MAX_HEADER_VALUE_LENGTH {
                    return Err(TraceError::HeaderTooLong);
                }
                Some(SpanId::from_hex(&parent_str)?)
            } else {
                None
            }
        } else {
            None
        };

        // Parse sampled flag
        let flags = if let Some(sampled_str) = carrier.get("x-b3-sampled") {
            match sampled_str.as_str() {
                "1" => crate::tracing::context::TraceFlags::sampled(),
                _ => crate::tracing::context::TraceFlags::unsampled(),
            }
        } else {
            // Default to sampled if not specified
            crate::tracing::context::TraceFlags::sampled()
        };

        let context = SpanContext::from_parts(trace_id, span_id, parent_span_id, flags);
        Ok(Some(context))
    }
}

/// B3 single-header propagator
///
/// Implements B3 propagation with a single header: `b3`
///
/// Format: `{trace_id}-{span_id}-{parent_span_id}-{sampled}`
#[derive(Clone, Debug, Default)]
pub struct B3SinglePropagator {
    _private: (),
}

impl B3SinglePropagator {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl<C> Propagator<C> for B3SinglePropagator
where
    C: TextMapCarrier,
{
    fn inject(&self, context: &SpanContext, carrier: &mut C) -> Result<(), TraceError> {
        let mut parts = Vec::with_capacity(4);

        // Trace ID and span ID are required
        parts.push(context.trace_id().to_hex());
        parts.push(context.span_id().to_hex());

        // Parent span ID (optional)
        if let Some(parent_id) = context.parent_span_id() {
            parts.push(parent_id.to_hex());
        }

        // Sampled flag
        parts.push(if context.is_sampled() { "1" } else { "0" });

        carrier.set("b3", parts.join("-"));
        Ok(())
    }

    fn extract(&self, carrier: &C) -> Result<Option<SpanContext>, TraceError> {
        let b3_value = match carrier.get("b3") {
            Some(value) => value,
            None => return Ok(None),
        };

        if b3_value.len() > MAX_HEADER_VALUE_LENGTH {
            return Err(TraceError::HeaderTooLong);
        }

        let parts: Vec<&str> = b3_value.split('-').collect();
        if parts.len() < 2 {
            return Err(TraceError::InvalidB3Header);
        }

        // Parse trace ID
        let trace_id_hex = parts[0];
        let trace_id = if trace_id_hex.len() == 32 {
            TraceId::from_hex(trace_id_hex)?
        } else if trace_id_hex.len() == 16 {
            // 64-bit trace ID
            let id_64 = u64::from_str_radix(trace_id_hex, 16)
                .map_err(|_| TraceError::InvalidTraceIdFormat)?;
            TraceId::from_u128(id_64 as u128)
        } else {
            return Err(TraceError::InvalidB3Header);
        };

        // Parse span ID
        let span_id = SpanId::from_hex(parts[1])?;

        // Parse parent span ID (optional)
        let parent_span_id = if parts.len() >= 3 && !parts[2].is_empty() {
            Some(SpanId::from_hex(parts[2])?)
        } else {
            None
        };

        // Parse sampled flag
        let flags = if parts.len() >= 4 {
            match parts[3] {
                "1" | "d" => crate::tracing::context::TraceFlags::sampled(),
                _ => crate::tracing::context::TraceFlags::unsampled(),
            }
        } else {
            crate::tracing::context::TraceFlags::sampled()
        };

        let context = SpanContext::from_parts(trace_id, span_id, parent_span_id, flags);
        Ok(Some(context))
    }
}

/// Composite propagator
///
/// Tries multiple propagators in order until one succeeds.
#[derive(Clone, Debug)]
pub struct CompositePropagator {
    propagators: Vec<PropagatorType>,
}

/// Enum of propagator types for composite propagator
#[derive(Clone, Debug)]
pub enum PropagatorType {
    W3C(W3CPropagator),
    B3(B3Propagator),
    B3Single(B3SinglePropagator),
}

impl CompositePropagator {
    pub fn new() -> Self {
        Self {
            propagators: Vec::with_capacity(3),
        }
    }

    /// Add a propagator
    pub fn add_propagator(mut self, propagator: PropagatorType) -> Self {
        self.propagators.push(propagator);
        self
    }

    /// Create with common propagators (W3C and B3)
    pub fn with_common() -> Self {
        Self::new()
            .add_propagator(PropagatorType::W3C(W3CPropagator::new()))
            .add_propagator(PropagatorType::B3(B3Propagator::new()))
            .add_propagator(PropagatorType::B3Single(B3SinglePropagator::new()))
    }
}

impl Default for CompositePropagator {
    fn default() -> Self {
        Self::with_common()
    }
}

impl<C> Propagator<C> for CompositePropagator
where
    C: TextMapCarrier,
{
    fn inject(&self, context: &SpanContext, carrier: &mut C) -> Result<(), TraceError> {
        // Inject using all propagators
        for propagator_type in &self.propagators {
            let result = match propagator_type {
                PropagatorType::W3C(p) => <W3CPropagator as Propagator<C>>::inject(p, context, carrier),
                PropagatorType::B3(p) => <B3Propagator as Propagator<C>>::inject(p, context, carrier),
                PropagatorType::B3Single(p) => {
                    <B3SinglePropagator as Propagator<C>>::inject(p, context, carrier)
                }
            };
            // Continue even if one fails, as different formats may succeed
            let _ = result;
        }
        Ok(())
    }

    fn extract(&self, carrier: &C) -> Result<Option<SpanContext>, TraceError> {
        // Try each propagator in order
        for propagator_type in &self.propagators {
            let result = match propagator_type {
                PropagatorType::W3C(p) => <W3CPropagator as Propagator<C>>::extract(p, carrier),
                PropagatorType::B3(p) => <B3Propagator as Propagator<C>>::extract(p, carrier),
                PropagatorType::B3Single(p) => <B3SinglePropagator as Propagator<C>>::extract(p, carrier),
            };

            if let Ok(Some(context)) = result {
                return Ok(Some(context));
            }
        }

        Ok(None)
    }
}

/// Binary propagator for efficient binary format propagation
///
/// Binary format is more efficient than text-based formats and is suitable
/// for high-performance scenarios or binary protocols.
#[derive(Clone, Debug, Default)]
pub struct BinaryPropagator {
    _private: (),
}

impl BinaryPropagator {
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Maximum binary context size
    pub const MAX_BINARY_SIZE: usize = 64;

    /// Inject context into binary format
    ///
    /// Binary format:
    /// - Version: 1 byte
    /// - Trace ID: 16 bytes
    /// - Span ID: 8 bytes
    /// - Parent Span ID: 8 bytes (optional, present if version & 0x01)
    /// - Flags: 1 byte
    pub fn inject_binary(&self, context: &SpanContext) -> Result<Vec<u8>, TraceError> {
        let mut buffer = Vec::with_capacity(34);

        // Version byte
        let version = 0x01;
        buffer.push(version);

        // Trace ID (16 bytes)
        let trace_id_bytes = context.trace_id().as_u128().to_be_bytes();
        buffer.extend_from_slice(&trace_id_bytes);

        // Span ID (8 bytes)
        let span_id_bytes = context.span_id().as_u64().to_be_bytes();
        buffer.extend_from_slice(&span_id_bytes);

        // Parent span ID if present (8 bytes)
        if let Some(parent_id) = context.parent_span_id() {
            buffer.extend_from_slice(&parent_id.as_u64().to_be_bytes());
        }

        // Flags
        buffer.push(context.flags().as_u8());

        Ok(buffer)
    }

    /// Extract context from binary format
    pub fn extract_binary(&self, data: &[u8]) -> Result<Option<SpanContext>, TraceError> {
        if data.is_empty() {
            return Ok(None);
        }

        if data.len() > Self::MAX_BINARY_SIZE {
            return Err(TraceError::BinaryContextTooLarge);
        }

        if data.len() < 26 {
            // Minimum: version (1) + trace_id (16) + span_id (8) + flags (1)
            return Err(TraceError::InvalidBinaryContext);
        }

        let version = data[0];
        if version != 0x01 {
            return Err(TraceError::UnsupportedBinaryVersion(version));
        }

        // Parse trace ID
        let mut trace_id_bytes = [0u8; 16];
        trace_id_bytes.copy_from_slice(&data[1..17]);
        let trace_id = u128::from_be_bytes(trace_id_bytes);

        // Parse span ID
        let mut span_id_bytes = [0u8; 8];
        span_id_bytes.copy_from_slice(&data[17..25]);
        let span_id = u64::from_be_bytes(span_id_bytes);

        // Check for parent span ID
        let (parent_span_id, flags_offset) = if data.len() >= 34 {
            let mut parent_bytes = [0u8; 8];
            parent_bytes.copy_from_slice(&data[25..33]);
            (Some(SpanId::from_u64(u64::from_be_bytes(parent_bytes))), 33)
        } else {
            (None, 25)
        };

        // Parse flags
        let flags = crate::tracing::context::TraceFlags::from_u8(data[flags_offset]);

        let context = SpanContext::from_parts(
            TraceId::from_u128(trace_id),
            SpanId::from_u64(span_id),
            parent_span_id,
            flags,
        );

        Ok(Some(context))
    }
}

/// In-process context propagation
///
/// Handles context propagation within the same process using
/// thread-local-like storage.
pub struct InProcessPropagator {
    // In a real implementation, this would use TLS or task-local storage
    // For now, we provide a placeholder
    _private: (),
}

impl InProcessPropagator {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

/// In-process context carrier
pub struct InProcessCarrier {
    context: Option<SpanContext>,
}

impl InProcessCarrier {
    pub fn new() -> Self {
        Self { context: None }
    }

    pub fn with_context(context: SpanContext) -> Self {
        Self {
            context: Some(context),
        }
    }

    pub fn get_context(&self) -> Option<&SpanContext> {
        self.context.as_ref()
    }

    pub fn set_context(&mut self, context: SpanContext) {
        self.context = Some(context);
    }
}

impl Default for InProcessCarrier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracing::context::{SpanContext, TraceFlags};

    #[test]
    fn test_header_carrier() {
        let mut carrier = HeaderCarrier::new();

        carrier.set("Content-Type", "application/json");
        carrier.set("Accept", "application/json");

        assert_eq!(carrier.get("content-type"), Some(String::from("application/json")));
        assert_eq!(carrier.get("Content-Type"), Some(String::from("application/json")));

        carrier.remove("content-type");
        assert_eq!(carrier.get("Content-Type"), None);
    }

    #[test]
    fn test_w3c_propagator() {
        let propagator = W3CPropagator::new();
        let context = SpanContext::new();
        let mut carrier = HeaderCarrier::new();

        propagator.inject(&context, &mut carrier).unwrap();

        assert!(carrier.get("traceparent").is_some());
        let traceparent = carrier.get("traceparent").unwrap();
        assert!(traceparent.starts_with("00-"));

        let extracted = propagator.extract(&carrier).unwrap().unwrap();
        assert_eq!(extracted.trace_id(), context.trace_id());
        assert_eq!(extracted.span_id(), context.span_id());
    }

    #[test]
    fn test_b3_propagator() {
        let propagator = B3Propagator::new();
        let context = SpanContext::new();
        let mut carrier = HeaderCarrier::new();

        propagator.inject(&context, &mut carrier).unwrap();

        assert!(carrier.get("x-b3-traceid").is_some());
        assert!(carrier.get("x-b3-spanid").is_some());
        assert!(carrier.get("x-b3-sampled").is_some());

        let extracted = propagator.extract(&carrier).unwrap().unwrap();
        assert_eq!(extracted.trace_id(), context.trace_id());
        assert_eq!(extracted.span_id(), context.span_id());
    }

    #[test]
    fn test_b3_single_propagator() {
        let propagator = B3SinglePropagator::new();
        let context = SpanContext::new();
        let mut carrier = HeaderCarrier::new();

        propagator.inject(&context, &mut carrier).unwrap();

        let b3_header = carrier.get("b3").unwrap();
        assert!(b3_header.contains('-'));

        let extracted = propagator.extract(&carrier).unwrap().unwrap();
        assert_eq!(extracted.trace_id(), context.trace_id());
        assert_eq!(extracted.span_id(), context.span_id());
    }

    #[test]
    fn test_composite_propagator() {
        let propagator = CompositePropagator::with_common();
        let context = SpanContext::new();
        let mut carrier = HeaderCarrier::new();

        propagator.inject(&context, &mut carrier).unwrap();

        // Should have headers from multiple formats
        assert!(carrier.get("traceparent").is_some());
        assert!(carrier.get("x-b3-traceid").is_some());
        assert!(carrier.get("b3").is_some());

        let extracted = propagator.extract(&carrier).unwrap().unwrap();
        assert_eq!(extracted.trace_id(), context.trace_id());
    }

    #[test]
    fn test_binary_propagator() {
        let propagator = BinaryPropagator::new();
        let context = SpanContext::new();

        let binary = propagator.inject_binary(&context).unwrap();
        assert!(binary.len() >= 26);

        let extracted = propagator.extract_binary(&binary).unwrap().unwrap();
        assert_eq!(extracted.trace_id(), context.trace_id());
        assert_eq!(extracted.span_id(), context.span_id());
    }

    #[test]
    fn test_binary_propagator_with_parent() {
        let propagator = BinaryPropagator::new();
        let parent_context = SpanContext::new();
        let context = SpanContext::child(&parent_context);

        let binary = propagator.inject_binary(&context).unwrap();
        assert!(binary.len() >= 34);

        let extracted = propagator.extract_binary(&binary).unwrap().unwrap();
        assert_eq!(extracted.parent_span_id(), context.parent_span_id());
    }

    #[test]
    fn test_extract_empty_carrier() {
        let propagator = W3CPropagator::new();
        let carrier = HeaderCarrier::new();

        let extracted = propagator.extract(&carrier).unwrap();
        assert!(extracted.is_none());
    }

    #[test]
    fn test_invalid_traceparent() {
        let propagator = W3CPropagator::new();
        let mut carrier = HeaderCarrier::new();
        carrier.set("traceparent", "invalid");

        assert!(propagator.extract(&carrier).is_err());
    }
}
