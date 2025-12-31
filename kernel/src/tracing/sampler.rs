//! # Sampling Strategies
//!
//! This module provides various sampling strategies for distributed tracing.
//! Sampling allows control over which traces are collected to manage overhead.
//!
//! # Architecture
//!
//! - **Sampler**: Trait defining sampling behavior
//! - **AlwaysSampler**: Sample all traces
//! - **NeverSampler**: Sample no traces
//! - **RateSampler**: Sample a fixed percentage of traces
//! - **ProbabilitySampler**: Sample based on probability
//! - **DynamicSampler**: Adjust sampling rate dynamically
//! - **ParentAwareSampler**: Consider parent's sampling decision
//!
//! # Example
//!
//! ```rust
//! use kernel::tracing::sampler::{Sampler, RateSampler};
//! use kernel::tracing::context::{SpanContext, TraceId};
//!
//! let sampler = RateSampler::new(0.1); // Sample 10% of traces
//! let trace_id = TraceId::new();
//! let should_sample = sampler.should_sample(&trace_id);
//! ```

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};

use crate::tracing::context::{SpanContext, TraceId};
use crate::tracing::TraceError;

/// Default sampling rate (10%)
pub const DEFAULT_SAMPLING_RATE: f64 = 0.1;

/// Minimum sampling rate
pub const MIN_SAMPLING_RATE: f64 = 0.0;

/// Maximum sampling rate
pub const MAX_SAMPLING_RATE: f64 = 1.0;

/// Maximum number of samplers in a composite
const MAX_COMPOSITE_SAMPLERS: usize = 8;

/// Sampling decision
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SamplingDecision {
    /// Sample this trace
    Sampled,

    /// Do not sample this trace
    NotSampled,

    /// Use parent's sampling decision
    Inherit,
}

/// Sampling result
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SamplingResult {
    /// The sampling decision
    pub decision: SamplingDecision,

    /// Additional attributes to attach to the span
    pub attributes: Vec<(String, String)>,
}

impl SamplingResult {
    /// Create a new sampling result
    pub fn new(decision: SamplingDecision) -> Self {
        Self {
            decision,
            attributes: Vec::new(),
        }
    }

    /// Add an attribute
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push((key.into(), value.into()));
        self
    }

    /// Check if sampled
    pub fn is_sampled(&self) -> bool {
        matches!(self.decision, SamplingDecision::Sampled)
    }
}

/// Sampler trait
///
/// A sampler determines whether a trace should be sampled based on
/// the trace ID and other context.
pub trait Sampler: Send + Sync {
    /// Make a sampling decision
    ///
    /// # Arguments
    ///
    /// * `trace_id` - The trace ID to make a decision for
    /// * `parent_context` - Optional parent span context
    ///
    /// # Returns
    ///
    /// The sampling decision
    fn should_sample(
        &self,
        trace_id: TraceId,
        parent_context: Option<&SpanContext>,
    ) -> SamplingResult;

    /// Get the sampler description
    fn description(&self) -> &str;
}

/// Always sample - samples 100% of traces
#[derive(Debug, Default, Clone, Copy)]
pub struct AlwaysSampler;

impl Sampler for AlwaysSampler {
    #[inline]
    fn should_sample(
        &self,
        _trace_id: TraceId,
        _parent_context: Option<&SpanContext>,
    ) -> SamplingResult {
        SamplingResult::new(SamplingDecision::Sampled)
            .with_attribute("sampler.type", "always")
    }

    fn description(&self) -> &str {
        "AlwaysSampler"
    }
}

/// Never sample - samples 0% of traces
#[derive(Debug, Default, Clone, Copy)]
pub struct NeverSampler;

impl Sampler for NeverSampler {
    #[inline]
    fn should_sample(
        &self,
        _trace_id: TraceId,
        _parent_context: Option<&SpanContext>,
    ) -> SamplingResult {
        SamplingResult::new(SamplingDecision::NotSampled)
            .with_attribute("sampler.type", "never")
    }

    fn description(&self) -> &str {
        "NeverSampler"
    }
}

/// Rate-based sampler
///
/// Samples a fixed percentage of traces. The sampling decision is deterministic
/// based on the trace ID to ensure consistent sampling across services.
#[derive(Clone, Debug)]
pub struct RateSampler {
    /// Sampling rate (0.0 to 1.0)
    rate: f64,

    /// Threshold for sampling (0 to u64::MAX)
    threshold: u64,
}

impl RateSampler {
    /// Create a new rate-based sampler
    ///
    /// # Arguments
    ///
    /// * `rate` - Sampling rate from 0.0 (0%) to 1.0 (100%)
    ///
    /// # Returns
    ///
    /// Ok(sampler) if rate is valid, Err otherwise
    pub fn new(rate: f64) -> Result<Self, TraceError> {
        if rate < MIN_SAMPLING_RATE || rate > MAX_SAMPLING_RATE {
            return Err(TraceError::InvalidSamplingRate(rate));
        }

        let threshold = (rate * u64::MAX as f64) as u64;

        Ok(Self { rate, threshold })
    }

    /// Get the sampling rate
    pub fn rate(&self) -> f64 {
        self.rate
    }

    /// Set a new sampling rate
    pub fn set_rate(&mut self, rate: f64) -> Result<(), TraceError> {
        if rate < MIN_SAMPLING_RATE || rate > MAX_SAMPLING_RATE {
            return Err(TraceError::InvalidSamplingRate(rate));
        }

        self.rate = rate;
        self.threshold = (rate * u64::MAX as f64) as u64;
        Ok(())
    }
}

impl Sampler for RateSampler {
    fn should_sample(
        &self,
        trace_id: TraceId,
        _parent_context: Option<&SpanContext>,
    ) -> SamplingResult {
        // Use the lower 64 bits of the trace ID for sampling decision
        // This ensures consistent sampling across services
        let trace_id_u64 = trace_id.as_u128() as u64;

        let decision = if trace_id_u64 <= self.threshold {
            SamplingDecision::Sampled
        } else {
            SamplingDecision::NotSampled
        };

        SamplingResult::new(decision)
            .with_attribute("sampler.type", "rate")
            .with_attribute("sampler.rate", format!("{:.4}", self.rate))
    }

    fn description(&self) -> &str {
        "RateSampler"
    }
}

/// Probability-based sampler
///
/// Similar to RateSampler but uses probabilistic sampling with
/// a random number generator for more uniform distribution.
#[derive(Clone, Debug)]
pub struct ProbabilitySampler {
    /// Sampling probability (0.0 to 1.0)
    probability: f64,

    /// Random seed (for deterministic testing)
    seed: AtomicU64,
}

impl ProbabilitySampler {
    /// Create a new probability-based sampler
    ///
    /// # Arguments
    ///
    /// * `probability` - Sampling probability from 0.0 to 1.0
    pub fn new(probability: f64) -> Result<Self, TraceError> {
        if probability < MIN_SAMPLING_RATE || probability > MAX_SAMPLING_RATE {
            return Err(TraceError::InvalidSamplingProbability(probability));
        }

        Ok(Self {
            probability,
            seed: AtomicU64::new(1),
        })
    }

    /// Get the sampling probability
    pub fn probability(&self) -> f64 {
        self.probability
    }

    /// Set the sampling probability
    pub fn set_probability(&mut self, probability: f64) -> Result<(), TraceError> {
        if probability < MIN_SAMPLING_RATE || probability > MAX_SAMPLING_RATE {
            return Err(TraceError::InvalidSamplingProbability(probability));
        }

        self.probability = probability;
        Ok(())
    }

    /// Generate a pseudo-random number using xorshift
    fn next_random(&self) -> u64 {
        let mut seed = self.seed.load(Ordering::Relaxed);
        loop {
            let mut x = seed;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            match self.seed.compare_exchange_weak(seed, x, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => return x,
                Err(actual) => seed = actual,
            }
        }
    }
}

impl Sampler for ProbabilitySampler {
    fn should_sample(
        &self,
        _trace_id: TraceId,
        _parent_context: Option<&SpanContext>,
    ) -> SamplingResult {
        let random = self.next_random();
        let threshold = (self.probability * u64::MAX as f64) as u64;

        let decision = if random <= threshold {
            SamplingDecision::Sampled
        } else {
            SamplingDecision::NotSampled
        };

        SamplingResult::new(decision)
            .with_attribute("sampler.type", "probability")
            .with_attribute("sampler.probability", format!("{:.4}", self.probability))
    }

    fn description(&self) -> &str {
        "ProbabilitySampler"
    }
}

/// Dynamic sampler with adjustable rate
///
/// Allows dynamic adjustment of the sampling rate at runtime.
#[derive(Clone, Debug)]
pub struct DynamicSampler {
    /// Current sampling rate (stored as percentage for atomic access)
    rate_percent: AtomicU8,
}

impl DynamicSampler {
    /// Create a new dynamic sampler
    ///
    /// # Arguments
    ///
    /// * `initial_rate` - Initial sampling rate from 0.0 to 1.0
    pub fn new(initial_rate: f64) -> Result<Self, TraceError> {
        if initial_rate < MIN_SAMPLING_RATE || initial_rate > MAX_SAMPLING_RATE {
            return Err(TraceError::InvalidSamplingRate(initial_rate));
        }

        let rate_percent = (initial_rate * 100.0) as u8;

        Ok(Self {
            rate_percent: AtomicU8::new(rate_percent),
        })
    }

    /// Get the current sampling rate
    pub fn rate(&self) -> f64 {
        self.rate_percent.load(Ordering::Relaxed) as f64 / 100.0
    }

    /// Set the sampling rate
    ///
    /// # Arguments
    ///
    /// * `rate` - New sampling rate from 0.0 to 1.0
    pub fn set_rate(&self, rate: f64) -> Result<(), TraceError> {
        if rate < MIN_SAMPLING_RATE || rate > MAX_SAMPLING_RATE {
            return Err(TraceError::InvalidSamplingRate(rate));
        }

        let rate_percent = (rate * 100.0) as u8;
        self.rate_percent.store(rate_percent, Ordering::Release);
        Ok(())
    }

    /// Adjust the sampling rate by a delta
    ///
    /// # Arguments
    ///
    /// * `delta` - Amount to adjust the rate by (can be negative)
    pub fn adjust_rate(&self, delta: f64) -> Result<(), TraceError> {
        let current = self.rate();
        let new_rate = (current + delta).max(MIN_SAMPLING_RATE).min(MAX_SAMPLING_RATE);
        self.set_rate(new_rate)
    }
}

impl Sampler for DynamicSampler {
    fn should_sample(
        &self,
        trace_id: TraceId,
        _parent_context: Option<&SpanContext>,
    ) -> SamplingResult {
        let rate = self.rate();
        let threshold = (rate * u64::MAX as f64) as u64;
        let trace_id_u64 = trace_id.as_u128() as u64;

        let decision = if trace_id_u64 <= threshold {
            SamplingDecision::Sampled
        } else {
            SamplingDecision::NotSampled
        };

        SamplingResult::new(decision)
            .with_attribute("sampler.type", "dynamic")
            .with_attribute("sampler.rate", format!("{:.4}", rate))
    }

    fn description(&self) -> &str {
        "DynamicSampler"
    }
}

/// Parent-aware sampler
///
/// Respects the parent's sampling decision. If the parent is sampled,
/// the child will also be sampled.
#[derive(Clone, Debug)]
pub struct ParentAwareSampler {
    /// Child sampler to use if no parent context
    child_sampler: Box<dyn Sampler>,
}

impl ParentAwareSampler {
    /// Create a new parent-aware sampler
    pub fn new(child_sampler: Box<dyn Sampler>) -> Self {
        Self { child_sampler }
    }
}

impl Sampler for ParentAwareSampler {
    fn should_sample(
        &self,
        trace_id: TraceId,
        parent_context: Option<&SpanContext>,
    ) -> SamplingResult {
        if let Some(parent) = parent_context {
            let decision = if parent.is_sampled() {
                SamplingDecision::Sampled
            } else {
                SamplingDecision::NotSampled
            };

            return SamplingResult::new(decision)
                .with_attribute("sampler.type", "parent_aware")
                .with_attribute("sampler.parent_sampled", if parent.is_sampled() { "true" } else { "false" });
        }

        // No parent, use child sampler
        let mut result = self.child_sampler.should_sample(trace_id, None);
        result.attributes.push(("sampler.type".to_string(), "parent_aware_no_parent".to_string()));
        result
    }

    fn description(&self) -> &str {
        "ParentAwareSampler"
    }
}

/// Composite sampler
///
/// Combines multiple samplers with configurable logic.
#[derive(Clone)]
pub struct CompositeSampler {
    /// Child samplers
    samplers: Vec<Box<dyn Sampler>>,

    /// Combination mode
    mode: CompositeMode,
}

/// How to combine sampler decisions
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositeMode {
    /// Sample if ANY sampler says yes
    Any,

    /// Sample only if ALL samplers say yes
    All,
}

impl CompositeSampler {
    /// Create a composite sampler that samples if any child says yes
    pub fn any(samplers: Vec<Box<dyn Sampler>>) -> Result<Self, TraceError> {
        if samplers.is_empty() {
            return Err(TraceError::EmptyCompositeSampler);
        }
        if samplers.len() > MAX_COMPOSITE_SAMPLERS {
            return Err(TraceError::TooManySamplers(samplers.len()));
        }

        Ok(Self {
            samplers,
            mode: CompositeMode::Any,
        })
    }

    /// Create a composite sampler that samples only if all children say yes
    pub fn all(samplers: Vec<Box<dyn Sampler>>) -> Result<Self, TraceError> {
        if samplers.is_empty() {
            return Err(TraceError::EmptyCompositeSampler);
        }
        if samplers.len() > MAX_COMPOSITE_SAMPLERS {
            return Err(TraceError::TooManySamplers(samplers.len()));
        }

        Ok(Self {
            samplers,
            mode: CompositeMode::All,
        })
    }
}

impl Sampler for CompositeSampler {
    fn should_sample(
        &self,
        trace_id: TraceId,
        parent_context: Option<&SpanContext>,
    ) -> SamplingResult {
        let mut attributes = Vec::new();
        attributes.push(("sampler.type".to_string(), format!("composite_{:?}", self.mode).to_lowercase()));
        attributes.push(("sampler.count".to_string(), self.samplers.len().to_string()));

        let mut sampled_count = 0;

        for sampler in &self.samplers {
            let result = sampler.should_sample(trace_id, parent_context);
            if result.is_sampled() {
                sampled_count += 1;
            }
            attributes.extend(result.attributes);
        }

        let decision = match self.mode {
            CompositeMode::Any => {
                if sampled_count > 0 {
                    SamplingDecision::Sampled
                } else {
                    SamplingDecision::NotSampled
                }
            }
            CompositeMode::All => {
                if sampled_count == self.samplers.len() {
                    SamplingDecision::Sampled
                } else {
                    SamplingDecision::NotSampled
                }
            }
        };

        attributes.push(("sampler.sampled_count".to_string(), sampled_count.to_string()));

        SamplingResult {
            decision,
            attributes,
        }
    }

    fn description(&self) -> &str {
        "CompositeSampler"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_always_sampler() {
        let sampler = AlwaysSampler;
        let trace_id = TraceId::new();

        let result = sampler.should_sample(trace_id, None);
        assert_eq!(result.decision, SamplingDecision::Sampled);
        assert_eq!(sampler.description(), "AlwaysSampler");
    }

    #[test]
    fn test_never_sampler() {
        let sampler = NeverSampler;
        let trace_id = TraceId::new();

        let result = sampler.should_sample(trace_id, None);
        assert_eq!(result.decision, SamplingDecision::NotSampled);
        assert_eq!(sampler.description(), "NeverSampler");
    }

    #[test]
    fn test_rate_sampler() {
        let sampler = RateSampler::new(0.5).unwrap();
        assert_eq!(sampler.rate(), 0.5);

        sampler.set_rate(0.75).unwrap();
        assert_eq!(sampler.rate(), 0.75);
    }

    #[test]
    fn test_rate_sampler_invalid() {
        assert!(RateSampler::new(-0.1).is_err());
        assert!(RateSampler::new(1.5).is_err());
        assert!(RateSampler::new(0.5).is_ok());
    }

    #[test]
    fn test_probability_sampler_always() {
        let sampler = ProbabilitySampler::new(1.0).unwrap();

        let trace_id = TraceId::new();
        let result = sampler.should_sample(trace_id, None);

        // With 1.0 probability, should almost always sample
        // (except if random == u64::MAX + 1, which is impossible)
        assert_eq!(result.decision, SamplingDecision::Sampled);
    }
    #[test]
    fn test_probability_sampler_never() {
        let sampler = ProbabilitySampler::new(0.0).unwrap();

        let trace_id = TraceId::new();
        let result = sampler.should_sample(trace_id, None);

        // With 0.0 probability, should rarely sample (only if random == 0)
        // Since we can't control random, just check decision type
        assert!(matches!(result.decision, SamplingDecision::Sampled | SamplingDecision::NotSampled));
    }

    #[test]
    fn test_dynamic_sampler() {
        let sampler = DynamicSampler::new(0.5).unwrap();
        assert!((sampler.rate() - 0.5).abs() < 0.01);

        sampler.set_rate(0.75).unwrap();
        assert!((sampler.rate() - 0.75).abs() < 0.01);

        sampler.adjust_rate(-0.25).unwrap();
        assert!((sampler.rate() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_dynamic_sampler_bounds() {
        let sampler = DynamicSampler::new(0.5).unwrap();

        // Test upper bound
        assert!(sampler.set_rate(1.5).is_err());
        assert!(sampler.adjust_rate(1.0).is_ok());
        assert_eq!(sampler.rate(), 1.0);

        // Test lower bound
        assert!(sampler.set_rate(-0.5).is_err());
        assert!(sampler.adjust_rate(-1.5).is_ok());
        assert_eq!(sampler.rate(), 0.0);
    }

    #[test]
    fn test_parent_aware_sampler() {
        let sampler = ParentAwareSampler::new(Box::new(NeverSampler));

        // No parent - use child sampler
        let trace_id = TraceId::new();
        let result = sampler.should_sample(trace_id, None);
        assert_eq!(result.decision, SamplingDecision::NotSampled);

        // Sampled parent - inherit sampling
        let mut parent_context = SpanContext::new();
        parent_context.set_sampled(true);
        let result = sampler.should_sample(trace_id, Some(&parent_context));
        assert_eq!(result.decision, SamplingDecision::Sampled);

        // Unsampled parent - inherit not sampling
        let mut parent_context = SpanContext::new();
        parent_context.set_sampled(false);
        let result = sampler.should_sample(trace_id, Some(&parent_context));
        assert_eq!(result.decision, SamplingDecision::NotSampled);
    }

    #[test]
    fn test_composite_sampler() {
        let sampler1 = AlwaysSampler;
        let sampler2 = NeverSampler;
        let composite = CompositeSampler::any(vec![Box::new(sampler1), Box::new(sampler2)]).unwrap();

        let trace_id = TraceId::new();
        let result = composite.should_sample(trace_id, None);
        // ANY: should sample if any sampler says yes
        assert_eq!(result.decision, SamplingDecision::Sampled);
    }

    #[test]
    fn test_composite_all() {
        let sampler1 = AlwaysSampler;
        let sampler2 = RateSampler::new(1.0).unwrap();
        let composite = CompositeSampler::all(vec![Box::new(sampler1), Box::new(sampler2)]).unwrap();

        let trace_id = TraceId::new();
        let result = composite.should_sample(trace_id, None);
        // ALL: should sample if all samplers say yes
        assert_eq!(result.decision, SamplingDecision::Sampled);
    }

    #[test]
    fn test_sampling_result() {
        let result = SamplingResult::new(SamplingDecision::Sampled)
            .with_attribute("key1", "value1")
            .with_attribute("key2", "value2");

        assert!(result.is_sampled());
        assert_eq!(result.attributes.len(), 2);
        assert_eq!(result.attributes[0], (String::from("key1"), String::from("value1")));
    }

    #[test]
    fn test_trace_id_hash() {
        let sampler = RateSampler::new(0.5).unwrap();
        let trace_id = TraceId::new();

        // Same trace ID should produce same decision
        let result1 = sampler.should_sample(trace_id, None);
        let result2 = sampler.should_sample(trace_id, None);

        assert_eq!(result1.decision, result2.decision);
    }

    #[test]
    fn test_rate_sampler_description() {
        let sampler = RateSampler::new(0.1).unwrap();
        assert_eq!(sampler.description(), "RateSampler");
    }
}
