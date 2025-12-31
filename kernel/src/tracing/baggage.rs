//! # Baggage Propagation
//!
//! This module implements baggage propagation for distributed tracing.
//! Baggage allows key-value pairs to be propagated across process boundaries.
//!
//! # Architecture
//!
//! - **Baggage**: Container for baggage key-value pairs
//! - **BaggagePropagator**: Injects and extracts baggage from carriers
//! - **BaggageLimits**: Configuration for baggage limits
//! - **BaggageEntry**: Single key-value pair with metadata
//!
//! # Example
//!
//! ```rust
//! use kernel::tracing::baggage::{Baggage, BaggagePropagator};
//! use kernel::tracing::propagation::HeaderCarrier;
//!
//! let mut baggage = Baggage::new();
//! baggage.set("user.id", "12345");
//! baggage.set("tenant.id", "abcde");
//!
//! let propagator = BaggagePropagator::new();
//! let mut carrier = HeaderCarrier::new();
//! propagator.inject_baggage(&baggage, &mut carrier);
//! ```

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::tracing::TraceError;

/// Maximum number of baggage entries
const MAX_BAGGAGE_ENTRIES: usize = 180;

/// Maximum baggage key length
const MAX_BAGGAGE_KEY_LENGTH: usize = 128;

/// Maximum baggage value length
const MAX_BAGGAGE_VALUE_LENGTH: usize = 4096;

/// Maximum total baggage size (all keys and values)
const MAX_BAGGAGE_TOTAL_SIZE: usize = 8192;

/// Default baggage limits
pub const DEFAULT_BAGGAGE_LIMITS: BaggageLimits = BaggageLimits {
    max_entries: 64,
    max_key_length: 128,
    max_value_length: 512,
    max_total_size: 4096,
};

/// Baggage limits configuration
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BaggageLimits {
    /// Maximum number of baggage entries
    pub max_entries: usize,

    /// Maximum key length
    pub max_key_length: usize,

    /// Maximum value length
    pub max_value_length: usize,

    /// Maximum total size (all keys and values combined)
    pub max_total_size: usize,
}

impl Default for BaggageLimits {
    fn default() -> Self {
        DEFAULT_BAGGAGE_LIMITS
    }
}

/// Baggage entry with metadata
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaggageEntry {
    /// Entry key
    pub key: String,

    /// Entry value
    pub value: String,

    /// Optional metadata (e.g., properties from W3C spec)
    pub metadata: Option<String>,
}

impl BaggageEntry {
    /// Create a new baggage entry
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            metadata: None,
        }
    }

    /// Create with metadata
    pub fn with_metadata(
        key: impl Into<String>,
        value: impl Into<String>,
        metadata: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            metadata: Some(metadata.into()),
        }
    }
}

/// Baggage container
///
/// Stores key-value pairs that propagate across process boundaries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Baggage {
    /// Baggage entries
    entries: BTreeMap<String, BaggageEntry>,

    /// Limits
    limits: BaggageLimits,

    /// Current total size
    total_size: usize,
}

impl Baggage {
    /// Create a new empty baggage container
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            limits: BaggageLimits::default(),
            total_size: 0,
        }
    }

    /// Create with custom limits
    pub fn with_limits(limits: BaggageLimits) -> Self {
        Self {
            entries: BTreeMap::new(),
            limits,
            total_size: 0,
        }
    }

    /// Set a baggage entry
    ///
    /// # Arguments
    ///
    /// * `key` - Entry key
    /// * `value` - Entry value
    ///
    /// # Returns
    ///
    /// Ok(()) if successful, Err if limits exceeded
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) -> Result<(), TraceError> {
        self.set_with_metadata(key, value, None)
    }

    /// Set a baggage entry with metadata
    ///
    /// # Arguments
    ///
    /// * `key` - Entry key
    /// * `value` - Entry value
    /// * `metadata` - Optional metadata
    pub fn set_with_metadata(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
        metadata: Option<impl Into<String>>,
    ) -> Result<(), TraceError> {
        let key_str = key.into();
        let value_str = value.into();

        // Validate key length
        if key_str.len() > self.limits.max_key_length {
            return Err(TraceError::TraceStateKey(key_str));
        }

        // Validate value length
        if value_str.len() > self.limits.max_value_length {
            return Err(TraceError::TraceStateValueTooLong);
        }

        // Calculate new size
        let entry_size = key_str.len() + value_str.len();
        let new_total = if let Some(existing) = self.entries.get(&key_str) {
            self.total_size - existing.key.len() - existing.value.len() + entry_size
        } else {
            self.total_size + entry_size
        };

        // Check total size
        if new_total > self.limits.max_total_size {
            return Err(TraceError::TraceStateValueTooLong);
        }

        // Check entry count
        if !self.entries.contains_key(&key_str) && self.entries.len() >= self.limits.max_entries {
            return Err(TraceError::TraceStateLimitExceeded);
        }

        // Insert or update
        let metadata_str = metadata.map(|m| m.into());
        self.entries.insert(
            key_str.clone(),
            BaggageEntry {
                key: key_str.clone(),
                value: value_str,
                metadata: metadata_str,
            },
        );

        self.total_size = new_total;
        Ok(())
    }

    /// Get a baggage value
    ///
    /// # Arguments
    ///
    /// * `key` - Entry key
    ///
    /// # Returns
    ///
    /// The value if present
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(|e| e.value.as_str())
    }

    /// Get a baggage entry with metadata
    pub fn get_entry(&self, key: &str) -> Option<&BaggageEntry> {
        self.entries.get(key)
    }

    /// Remove a baggage entry
    ///
    /// # Arguments
    ///
    /// * `key` - Entry key
    ///
    /// # Returns
    ///
    /// The removed entry if present
    pub fn remove(&mut self, key: &str) -> Option<BaggageEntry> {
        let entry = self.entries.remove(key)?;
        self.total_size -= entry.key.len() + entry.value.len();
        Some(entry)
    }

    /// Get all entries
    pub fn entries(&self) -> &BTreeMap<String, BaggageEntry> {
        &self.entries
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_size = 0;
    }

    /// Merge another baggage into this one
    ///
    /// Entries in `other` take precedence over existing entries.
    pub fn merge(&mut self, other: &Baggage) -> Result<(), TraceError> {
        for (key, entry) in other.entries.iter() {
            self.set_with_metadata(
                &entry.key,
                &entry.value,
                entry.metadata.as_ref().map(|m| m.as_str()),
            )?;
        }
        Ok(())
    }

    /// Validate a key according to W3C specification
    ///
    /// Keys must match: `[a-zA-Z0-9!#$%&'*+-.^_`|~]+`
    pub fn validate_key(key: &str) -> bool {
        if key.is_empty() || key.len() > MAX_BAGGAGE_KEY_LENGTH {
            return false;
        }

        key.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '!' | '#' | '$' | '%' | '&' | '\'' | '*' | '+' | '-' | '.' | '^' | '_' | '`' | '|' | '~'))
    }

    /// Validate a value according to W3C specification
    ///
    /// Values can contain any printable ASCII character except `,` and `;`
    pub fn validate_value(value: &str) -> bool {
        if value.len() > MAX_BAGGAGE_VALUE_LENGTH {
            return false;
        }

        !value.chars().any(|c| c == ',' || c == ';')
    }

    /// Convert to W3C baggage header format
    ///
    /// Format: `key1=value1,key2=value2;metadata`
    pub fn to_header(&self) -> String {
        self.entries
            .iter()
            .map(|(_, entry)| {
                let mut s = format!("{}={}", entry.key, entry.value);
                if let Some(ref metadata) = entry.metadata {
                    s.push(';');
                    s.push_str(metadata);
                }
                s
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Parse from W3C baggage header format
    pub fn from_header(header: &str) -> Result<Self, TraceError> {
        let mut baggage = Self::new();

        // Parse comma-separated entries
        let mut entry_start = 0;
        while entry_start < header.len() {
            // Find next comma (not inside quoted strings)
            let mut entry_end = entry_start;
            let mut in_quotes = false;

            while entry_end < header.len() {
                let ch = header.as_bytes()[entry_end];
                if ch == b'"' {
                    in_quotes = !in_quotes;
                } else if ch == b',' && !in_quotes {
                    break;
                }
                entry_end += 1;
            }

            let entry_str = &header[entry_start..entry_end];
            if !entry_str.is_empty() {
                baggage.parse_entry(entry_str)?;
            }

            entry_start = entry_end + 1;
        }

        Ok(baggage)
    }

    /// Parse a single baggage entry
    fn parse_entry(&mut self, entry: &str) -> Result<(), TraceError> {
        // Find first '=' (key-value separator)
        let eq_pos = entry
            .find('=')
            .ok_or(TraceError::InvalidTraceStateKey("Missing '='".to_string()))?;

        let key = &entry[..eq_pos];
        let rest = &entry[eq_pos + 1..];

        // Check for metadata separator ';'
        let (value, metadata) = if let Some(semicolon_pos) = rest.find(';') {
            (&rest[..semicolon_pos], Some(&rest[semicolon_pos + 1..]))
        } else {
            (rest, None)
        };

        // Validate
        if !Self::validate_key(key) || !Self::validate_value(value) {
            return Err(TraceError::InvalidTraceStateKey(key.to_string()));
        }

        self.set_with_metadata(key, value, metadata)?;
        Ok(())
    }
}

/// Baggage propagator
///
/// Injects and extracts baggage from carriers (e.g., HTTP headers).
#[derive(Clone, Debug, Default)]
pub struct BaggagePropagator {
    /// Baggage limits
    limits: BaggageLimits,
}

impl BaggagePropagator {
    /// Create a new baggage propagator
    pub fn new() -> Self {
        Self {
            limits: BaggageLimits::default(),
        }
    }

    /// Create with custom limits
    pub fn with_limits(limits: BaggageLimits) -> Self {
        Self { limits }
    }

    /// Inject baggage into a carrier
    ///
    /// # Arguments
    ///
    /// * `baggage` - Baggage to inject
    /// * `carrier` - Carrier to inject into
    pub fn inject_baggage<C>(
        &self,
        baggage: &Baggage,
        carrier: &mut C,
    ) -> Result<(), TraceError>
    where
        C: crate::tracing::propagation::TextMapCarrier,
    {
        if !baggage.is_empty() {
            let header = baggage.to_header();
            carrier.set("baggage", header);
        }
        Ok(())
    }

    /// Extract baggage from a carrier
    ///
    /// # Arguments
    ///
    /// * `carrier` - Carrier to extract from
    ///
    /// # Returns
    ///
    /// The extracted baggage
    pub fn extract_baggage<C>(&self, carrier: &C) -> Result<Baggage, TraceError>
    where
        C: crate::tracing::propagation::TextMapCarrier,
    {
        if let Some(header_value) = carrier.get("baggage") {
            Baggage::from_header(&header_value)
        } else {
            Ok(Baggage::new())
        }
    }
}

/// In-memory baggage carrier for testing
#[derive(Clone, Debug, Default)]
pub struct BaggageCarrier {
    baggage: Baggage,
}

impl BaggageCarrier {
    pub fn new() -> Self {
        Self {
            baggage: Baggage::new(),
        }
    }

    pub fn with_baggage(baggage: Baggage) -> Self {
        Self { baggage }
    }

    pub fn get_baggage(&self) -> &Baggage {
        &self.baggage
    }

    pub fn get_baggage_mut(&mut self) -> &mut Baggage {
        &mut self.baggage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracing::propagation::HeaderCarrier;

    #[test]
    fn test_baggage_creation() {
        let baggage = Baggage::new();
        assert!(baggage.is_empty());
        assert_eq!(baggage.len(), 0);
    }

    #[test]
    fn test_baggage_set_get() {
        let mut baggage = Baggage::new();
        assert!(baggage.set("user.id", "12345").is_ok());
        assert_eq!(baggage.get("user.id"), Some("12345"));
        assert_eq!(baggage.len(), 1);
    }

    #[test]
    fn test_baggage_update() {
        let mut baggage = Baggage::new();
        assert!(baggage.set("key", "value1").is_ok());
        assert!(baggage.set("key", "value2").is_ok());
        assert_eq!(baggage.get("key"), Some("value2"));
        assert_eq!(baggage.len(), 1);
    }

    #[test]
    fn test_baggage_remove() {
        let mut baggage = Baggage::new();
        assert!(baggage.set("key", "value").is_ok());
        baggage.remove("key");
        assert_eq!(baggage.get("key"), None);
        assert_eq!(baggage.len(), 0);
    }

    #[test]
    fn test_baggage_with_metadata() {
        let mut baggage = Baggage::new();
        assert!(baggage
            .set_with_metadata("key", "value", Some("metadata"))
            .is_ok());

        let entry = baggage.get_entry("key").unwrap();
        assert_eq!(entry.value, "value");
        assert_eq!(entry.metadata, Some("metadata".to_string()));
    }

    #[test]
    fn test_baggage_limits() {
        let limits = BaggageLimits {
            max_entries: 2,
            max_key_length: 10,
            max_value_length: 10,
            max_total_size: 50,
        };

        let mut baggage = Baggage::with_limits(limits);
        assert!(baggage.set("key1", "value1").is_ok());
        assert!(baggage.set("key2", "value2").is_ok());

        // Third entry should fail
        assert!(matches!(
            baggage.set("key3", "value3"),
            Err(TraceError::TraceStateLimitExceeded)
        ));
    }

    #[test]
    fn test_baggage_key_too_long() {
        let limits = BaggageLimits {
            max_entries: 10,
            max_key_length: 5,
            max_value_length: 10,
            max_total_size: 100,
        };

        let mut baggage = Baggage::with_limits(limits);
        assert!(matches!(
            baggage.set("very_long_key", "value"),
            Err(TraceError::TraceStateKey(_))
        ));
    }

    #[test]
    fn test_baggage_value_too_long() {
        let limits = BaggageLimits {
            max_entries: 10,
            max_key_length: 10,
            max_value_length: 5,
            max_total_size: 100,
        };

        let mut baggage = Baggage::with_limits(limits);
        assert!(matches!(
            baggage.set("key", "very_long_value"),
            Err(TraceError::TraceStateValueTooLong)
        ));
    }

    #[test]
    fn test_baggage_merge() {
        let mut baggage1 = Baggage::new();
        baggage1.set("key1", "value1").unwrap();

        let mut baggage2 = Baggage::new();
        baggage2.set("key2", "value2").unwrap();
        baggage2.set("key1", "updated").unwrap();

        baggage1.merge(&baggage2).unwrap();
        assert_eq!(baggage1.get("key1"), Some("updated"));
        assert_eq!(baggage1.get("key2"), Some("value2"));
    }

    #[test]
    fn test_baggage_clear() {
        let mut baggage = Baggage::new();
        baggage.set("key1", "value1").unwrap();
        baggage.set("key2", "value2").unwrap();

        baggage.clear();
        assert!(baggage.is_empty());
    }

    #[test]
    fn test_validate_key() {
        assert!(Baggage::validate_key("valid-key-123"));
        assert!(Baggage::validate_key("Valid_Key!#$%"));
        assert!(!Baggage::validate_key("invalid,key"));
        assert!(!Baggage::validate_key(""));
        assert!(!Baggage::validate_key("a".repeat(200).as_str()));
    }

    #[test]
    fn test_validate_value() {
        assert!(Baggage::validate_value("valid-value"));
        assert!(!Baggage::validate_value("invalid,value"));
        assert!(!Baggage::validate_value("invalid;value"));
        assert!(!Baggage::validate_value(&"a".repeat(5000)));
    }

    #[test]
    fn test_baggage_to_header() {
        let mut baggage = Baggage::new();
        baggage.set("key1", "value1").unwrap();
        baggage.set("key2", "value2").unwrap();

        let header = baggage.to_header();
        assert!(header.contains("key1=value1"));
        assert!(header.contains("key2=value2"));
    }

    #[test]
    fn test_baggage_from_header() {
        let header = "key1=value1,key2=value2,key3=value3;metadata";
        let baggage = Baggage::from_header(header).unwrap();

        assert_eq!(baggage.get("key1"), Some("value1"));
        assert_eq!(baggage.get("key2"), Some("value2"));
        assert_eq!(baggage.get("key3"), Some("value3"));

        let entry = baggage.get_entry("key3").unwrap();
        assert_eq!(entry.metadata, Some("metadata".to_string()));
    }

    #[test]
    fn test_baggage_roundtrip() {
        let mut original = Baggage::new();
        original.set("user.id", "12345").unwrap();
        original.set("tenant.id", "abcde").unwrap();
        original
            .set_with_metadata("session.id", "xyz", Some("prop=value"))
            .unwrap();

        let header = original.to_header();
        let restored = Baggage::from_header(&header).unwrap();

        assert_eq!(restored.get("user.id"), original.get("user.id"));
        assert_eq!(restored.get("tenant.id"), original.get("tenant.id"));
        assert_eq!(restored.get("session.id"), original.get("session.id"));
    }

    #[test]
    fn test_baggage_propagator_inject() {
        let mut baggage = Baggage::new();
        baggage.set("key1", "value1").unwrap();

        let propagator = BaggagePropagator::new();
        let mut carrier = HeaderCarrier::new();

        assert!(propagator.inject_baggage(&baggage, &mut carrier).is_ok());
        assert!(carrier.get("baggage").is_some());
    }

    #[test]
    fn test_baggage_propagator_extract() {
        let mut carrier = HeaderCarrier::new();
        carrier.set("baggage", "key1=value1,key2=value2");

        let propagator = BaggagePropagator::new();
        let baggage = propagator.extract_baggage(&carrier).unwrap();

        assert_eq!(baggage.get("key1"), Some("value1"));
        assert_eq!(baggage.get("key2"), Some("value2"));
    }

    #[test]
    fn test_baggage_empty_carrier() {
        let carrier = HeaderCarrier::new();
        let propagator = BaggagePropagator::new();

        let baggage = propagator.extract_baggage(&carrier).unwrap();
        assert!(baggage.is_empty());
    }

    #[test]
    fn test_baggage_entry() {
        let entry = BaggageEntry::new("key", "value");
        assert_eq!(entry.key, "key");
        assert_eq!(entry.value, "value");
        assert!(entry.metadata.is_none());
    }

    #[test]
    fn test_baggage_entry_with_metadata() {
        let entry = BaggageEntry::with_metadata("key", "value", "metadata");
        assert_eq!(entry.key, "key");
        assert_eq!(entry.value, "value");
        assert_eq!(entry.metadata, Some("metadata".to_string()));
    }
}
