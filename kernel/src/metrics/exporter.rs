//! Metrics exporters for various protocols and formats
//!
//! This module provides exporters for different monitoring systems:
//! - Prometheus exposition format
//! - OpenMetrics format
//! - StatsD protocol (client)
//! - HTTP scraping endpoint
//!
//! # Example
//!
//! ```rust
//! use kernel::metrics::exporter::PrometheusExporter;
//!//! use kernel::metrics::registry::Registry;
//!
//! let registry = Registry::new();
//! let exporter = PrometheusExporter::new(registry);
//!
//! let output = exporter.export();
//! ```

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::fmt;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::Display;

use super::registry::{Metric, MetricType, Registry};

/// Export error types
#[derive(Debug, Clone, PartialEq)]
pub enum ExportError {
    /// Registry error
    RegistryError(String),
    /// Format error
    FormatError(String),
    /// Network error (for push-based exporters)
    NetworkError(String),
    /// Invalid configuration
    InvalidConfig(String),
}

impl Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::RegistryError(msg) => write!(f, "Registry error: {}", msg),
            ExportError::FormatError(msg) => write!(f, "Format error: {}", msg),
            ExportError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ExportError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ExportError {}

/// Prometheus text format exporter
///
/// Exports metrics in the Prometheus exposition format.
/// See: https://prometheus.io/docs/instrumenting/exposition_formats/
#[derive(Clone)]
pub struct PrometheusExporter {
    registry: Arc<Registry>,
}

impl PrometheusExporter {
    /// Create a new Prometheus exporter
    ///
    /// # Arguments
    ///
    /// * `registry` - The metrics registry to export from
    pub fn new(registry: Arc<Registry>) -> Self {
        Self { registry }
    }

    /// Export all metrics in Prometheus text format
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::exporter::PrometheusExporter;
    /// use kernel::metrics::registry::Registry;
    /// use kernel::metrics::counter::Counter;
    ///
    /// let registry = Registry::new();
    /// let counter = Counter::new("test_counter").unwrap();
    /// counter.inc_by(42);
    /// registry.register_metric(Box::new(counter)).unwrap();
    ///
    /// let exporter = PrometheusExporter::new(Arc::new(registry));
    /// let output = exporter.export();
    /// assert!(output.contains("test_counter 42"));
    /// ```
    pub fn export(&self) -> String {
        self.registry.export_prometheus()
    }

    /// Export with custom timestamp
    ///
    /// Adds a timestamp to each metric sample.
    pub fn export_with_timestamp(&self, timestamp_ms: u64) -> String {
        let output = self.registry.export_prometheus();
        let mut result = String::new();

        for line in output.lines() {
            if line.starts_with('#') {
                // Comments stay as-is
                result.push_str(line);
                result.push('\n');
            } else if !line.is_empty() {
                // Add timestamp to data lines
                result.push_str(line);
                result.push(' ');
                result.push_str(&timestamp_ms.to_string());
                result.push('\n');
            }
        }

        result
    }

    /// Get content type for HTTP responses
    pub fn content_type() -> &'static str {
        "text/plain; version=0.0.4; charset=utf-8"
    }
}

/// OpenMetrics format exporter
///
/// Exports metrics in the OpenMetrics format.
/// See: https://github.com/OpenObservability/OpenMetrics
#[derive(Clone)]
pub struct OpenMetricsExporter {
    registry: Arc<Registry>,
}

impl OpenMetricsExporter {
    /// Create a new OpenMetrics exporter
    pub fn new(registry: Arc<Registry>) -> Self {
        Self { registry }
    }

    /// Export all metrics in OpenMetrics format
    ///
    /// OpenMetrics is a more efficient and modern format compared to
    /// Prometheus exposition format.
    pub fn export(&self) -> String {
        let output = self.registry.export_prometheus();
        let mut result = String::new();

        // Add OpenMetrics version header
        result.push_str("# EOF\n");

        for line in output.lines() {
            if line.starts_with("# TYPE") {
                // Convert TYPE format
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    result.push_str("# TYPE ");
                    result.push_str(parts[2]);
                    result.push(' ');
                    result.push_str(&parts[1].to_uppercase());
                    result.push('\n');
                }
            } else if !line.is_empty() {
                result.push_str(line);
                result.push('\n');
            }
        }

        // End-of-file marker
        result.push_str("# EOF\n");

        result
    }

    /// Export with EOF markers for streaming
    pub fn export_streaming(&self) -> String {
        let mut output = self.export();
        output.push_str("# EOF\n");
        output
    }

    /// Get content type for HTTP responses
    pub fn content_type() -> &'static str {
        "application/openmetrics-text; version=1.0.0; charset=utf-8"
    }
}

/// StatsD protocol client
///
/// Sends metrics to a StatsD server using the DogStatsD extension.
///
/// Note: This is a client-side implementation. The actual network operations
/// would need to be implemented in a networking layer.
#[derive(Debug, Clone)]
pub struct StatsDExporter {
    prefix: String,
    // In a real implementation, these would include network configuration
    _host: String,
    _port: u16,
}

impl StatsDExporter {
    /// Create a new StatsD exporter
    ///
    /// # Arguments
    ///
    /// * `host` - StatsD server hostname or IP
    /// * `port` - StatsD server port (typically 8125)
    /// * `prefix` - Optional prefix for all metric names
    pub fn new(host: impl Into<String>, port: u16, prefix: Option<impl Into<String>>) -> Self {
        Self {
            prefix: prefix.map(|p| p.into()).unwrap_or_default(),
            _host: host.into(),
            _port: port,
        }
    }

    /// Format a counter metric
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    /// * `value` - Counter increment value
    /// * `tags` - Optional tags (DogStatsD extension)
    pub fn format_counter(&self, name: &str, value: i64, tags: Option<&[(&str, &str)]>) -> String {
        let full_name = self.format_name(name);
        let tag_str = self.format_tags(tags);
        format!("{}:{}|c{}", full_name, value, tag_str)
    }

    /// Format a gauge metric
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    /// * `value` - Gauge value
    /// * `tags` - Optional tags
    pub fn format_gauge(&self, name: &str, value: f64, tags: Option<&[(&str, &str)]>) -> String {
        let full_name = self.format_name(name);
        let tag_str = self.format_tags(tags);
        format!("{}:{}|g{}", full_name, value, tag_str)
    }

    /// Format a timing/histogram metric
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    /// * `value` - Timing value in milliseconds
    /// * `tags` - Optional tags
    pub fn format_timing(&self, name: &str, value: f64, tags: Option<&[(&str, &str)]>) -> String {
        let full_name = self.format_name(name);
        let tag_str = self.format_tags(tags);
        format!("{}:{}|ms{}", full_name, value, tag_str)
    }

    /// Format a set metric (unique count)
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    /// * `value` - Value to add to set
    /// * `tags` - Optional tags
    pub fn format_set(&self, name: &str, value: &str, tags: Option<&[(&str, &str)]>) -> String {
        let full_name = self.format_name(name);
        let tag_str = self.format_tags(tags);
        format!("{}:{}|s{}", full_name, value, tag_str)
    }

    fn format_name(&self, name: &str) -> String {
        if self.prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}.{}", self.prefix, name)
        }
    }

    fn format_tags(&self, tags: Option<&[(&str, &str)]>) -> String {
        match tags {
            Some(tag_pairs) if !tag_pairs.is_empty() => {
                let tag_str = tag_pairs
                    .iter()
                    .map(|(k, v)| format!("{}:{}", k, v))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("|#{}", tag_str)
            }
            _ => String::new(),
        }
    }

    /// Send a counter metric
    ///
    /// Note: This is a placeholder. Real implementation would use UDP.
    #[cfg(feature = "std")]
    pub fn send_counter(&self, _name: &str, _value: i64, _tags: Option<&[(&str, &str)]>) -> Result<(), ExportError> {
        // In a real implementation, this would send UDP packet
        Ok(())
    }

    /// Send a gauge metric
    #[cfg(feature = "std")]
    pub fn send_gauge(&self, _name: &str, _value: f64, _tags: Option<&[(&str, &str)]>) -> Result<(), ExportError> {
        Ok(())
    }

    /// Send a timing metric
    #[cfg(feature = "std")]
    pub fn send_timing(&self, _name: &str, _value: f64, _tags: Option<&[(&str, &str)]>) -> Result<(), ExportError> {
        Ok(())
    }
}

/// HTTP scraping endpoint
///
/// Provides an HTTP endpoint for Prometheus to scrape metrics.
///
/// Note: This is a placeholder. Real implementation would integrate with
/// the kernel's HTTP server.
#[derive(Clone)]
pub struct HttpExporter {
    registry: Arc<Registry>,
    format: ExportFormat,
}

/// Export format for HTTP endpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Prometheus text format
    Prometheus,
    /// OpenMetrics format
    OpenMetrics,
    /// JSON format
    Json,
}

impl HttpExporter {
    /// Create a new HTTP exporter
    ///
    /// # Arguments
    ///
    /// * `registry` - Metrics registry
    /// * `format` - Export format
    pub fn new(registry: Arc<Registry>, format: ExportFormat) -> Self {
        Self { registry, format }
    }

    /// Get the scrape endpoint path
    pub fn path(&self) -> &'static str {
        "/metrics"
    }

    /// Get the content type based on format
    pub fn content_type(&self) -> &'static str {
        match self.format {
            ExportFormat::Prometheus => PrometheusExporter::content_type(),
            ExportFormat::OpenMetrics => OpenMetricsExporter::content_type(),
            ExportFormat::Json => "application/json",
        }
    }

    /// Handle a scrape request
    ///
    /// Returns the formatted metrics.
    pub fn handle_scrape(&self) -> String {
        match self.format {
            ExportFormat::Prometheus => {
                let exporter = PrometheusExporter::new(Arc::clone(&self.registry));
                exporter.export()
            }
            ExportFormat::OpenMetrics => {
                let exporter = OpenMetricsExporter::new(Arc::clone(&self.registry));
                exporter.export()
            }
            ExportFormat::Json => self.export_json(),
        }
    }

    /// Export metrics in JSON format
    pub fn export_json(&self) -> String {
        let output = self.registry.export_prometheus();
        let mut result = String::from("{\n");
        let mut is_first = true;

        // Simple parsing of prometheus output to JSON
        for line in output.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            // Split on first space
            if let Some(space_idx) = line.find(' ') {
                let name = &line[..space_idx];
                let value = &line[space_idx + 1..];

                if !is_first {
                    result.push_str(",\n");
                }
                is_first = false;

                result.push_str(&format!("  \"{}\": {}", name, value));
            }
        }

        result.push_str("\n}\n");
        result
    }
}

/// Push-based exporter configuration
///
/// Push exporters actively send metrics to a remote endpoint
/// instead of waiting to be scraped.
#[derive(Clone)]
pub struct PushExporter {
    registry: Arc<Registry>,
    endpoint: String,
    interval_ms: u64,
    format: ExportFormat,
}

impl PushExporter {
    /// Create a new push exporter
    ///
    /// # Arguments
    ///
    /// * `registry` - Metrics registry
    /// * `endpoint` - Push gateway URL
    /// * `interval_ms` - Push interval in milliseconds
    /// * `format` - Export format
    pub fn new(
        registry: Arc<Registry>,
        endpoint: impl Into<String>,
        interval_ms: u64,
        format: ExportFormat,
    ) -> Self {
        Self {
            registry,
            endpoint: endpoint.into(),
            interval_ms,
            format,
        }
    }

    /// Push metrics to the gateway
    ///
    /// Note: This is a placeholder. Real implementation would use HTTP.
    #[cfg(feature = "std")]
    pub fn push(&self) -> Result<(), ExportError> {
        let body = match self.format {
            ExportFormat::Prometheus => {
                let exporter = PrometheusExporter::new(Arc::clone(&self.registry));
                exporter.export()
            }
            ExportFormat::OpenMetrics => {
                let exporter = OpenMetricsExporter::new(Arc::clone(&self.registry));
                exporter.export()
            }
            ExportFormat::Json => {
                let http_exporter = HttpExporter::new(Arc::clone(&self.registry), ExportFormat::Json);
                http_exporter.export_json()
            }
        };

        // In a real implementation, this would send an HTTP POST
        log::debug!("Pushing metrics to {}: {} bytes", self.endpoint, body.len());

        Ok(())
    }

    /// Get the push interval
    pub fn interval(&self) -> u64 {
        self.interval_ms
    }

    /// Get the endpoint URL
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

/// Metrics export utilities
pub struct ExportUtils;

impl ExportUtils {
    /// Convert metric type to display string
    pub fn metric_type_to_string(metric_type: MetricType) -> &'static str {
        match metric_type {
            MetricType::Counter => "counter",
            MetricType::Gauge => "gauge",
            MetricType::Histogram => "histogram",
            MetricType::Summary => "summary",
        }
    }

    /// Sanitize metric name for export
    ///
    /// Replaces invalid characters with underscores.
    pub fn sanitize_name(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' || c == ':' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    /// Sanitize label value for export
    ///
    /// Escapes special characters according to Prometheus format.
    pub fn sanitize_label_value(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    }

    /// Format labels in Prometheus format
    pub fn format_labels(labels: &BTreeMap<String, String>) -> String {
        if labels.is_empty() {
            return String::new();
        }

        let pairs: Vec<String> = labels
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, Self::sanitize_label_value(v)))
            .collect();

        format!("{{{}}}", pairs.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::counter::Counter;
    use crate::metrics::gauge::Gauge;

    #[test]
    fn test_prometheus_exporter() {
        let registry = Arc::new(Registry::new());
        let counter = Counter::with_help("test_counter", "A test counter").unwrap();
        counter.inc_by(42);
        registry.register_metric(Box::new(counter)).unwrap();

        let exporter = PrometheusExporter::new(Arc::clone(&registry));
        let output = exporter.export();

        assert!(output.contains("# HELP test_counter A test counter"));
        assert!(output.contains("# TYPE test_counter counter"));
        assert!(output.contains("test_counter 42"));
    }

    #[test]
    fn test_prometheus_exporter_with_timestamp() {
        let registry = Arc::new(Registry::new());
        let counter = Counter::new("test_counter").unwrap();
        counter.inc_by(10);
        registry.register_metric(Box::new(counter)).unwrap();

        let exporter = PrometheusExporter::new(Arc::clone(&registry));
        let output = exporter.export_with_timestamp(1234567890);

        assert!(output.contains("1234567890"));
    }

    #[test]
    fn test_openmetrics_exporter() {
        let registry = Arc::new(Registry::new());
        let counter = Counter::new("test_counter").unwrap();
        counter.inc_by(100);
        registry.register_metric(Box::new(counter)).unwrap();

        let exporter = OpenMetricsExporter::new(Arc::clone(&registry));
        let output = exporter.export();

        assert!(output.contains("# TYPE"));
        assert!(output.contains("# EOF"));
    }

    #[test]
    fn test_statsd_exporter() {
        let exporter = StatsDExporter::new("localhost", 8125, Some("myprefix"));

        // Test counter format
        let counter = exporter.format_counter("mycounter", 1, None);
        assert!(counter.contains("myprefix.mycounter:1|c"));

        // Test with tags
        let counter_tags = exporter.format_counter(
            "mycounter",
            5,
            Some(&[("env", "prod"), ("service", "api")]),
        );
        assert!(counter_tags.contains("|#env:prod,service:api"));

        // Test gauge format
        let gauge = exporter.format_gauge("mygauge", 42.5, None);
        assert!(gauge.contains("mygauge:42.5|g"));

        // Test timing format
        let timing = exporter.format_timing("mytiming", 123.4, None);
        assert!(timing.contains("mytiming:123.4|ms"));

        // Test set format
        let set = exporter.format_set("myset", "user123", None);
        assert!(set.contains("myset:user123|s"));
    }

    #[test]
    fn test_http_exporter_prometheus() {
        let registry = Arc::new(Registry::new());
        let counter = Counter::new("test_counter").unwrap();
        counter.inc_by(42);
        registry.register_metric(Box::new(counter)).unwrap();

        let exporter = HttpExporter::new(Arc::clone(&registry), ExportFormat::Prometheus);

        assert_eq!(exporter.path(), "/metrics");
        assert_eq!(exporter.content_type(), PrometheusExporter::content_type());

        let output = exporter.handle_scrape();
        assert!(output.contains("test_counter 42"));
    }

    #[test]
    fn test_http_exporter_json() {
        let registry = Arc::new(Registry::new());
        let counter = Counter::new("test_counter").unwrap();
        counter.inc_by(42);
        registry.register_metric(Box::new(counter)).unwrap();

        let exporter = HttpExporter::new(Arc::clone(&registry), ExportFormat::Json);

        assert_eq!(exporter.content_type(), "application/json");

        let output = exporter.export_json();
        assert!(output.contains("\"test_counter\""));
        assert!(output.contains("\"type\""));
    }

    #[test]
    fn test_push_exporter() {
        let registry = Arc::new(Registry::new());
        let exporter = PushExporter::new(
            Arc::clone(&registry),
            "http://pushgateway:9091",
            10000,
            ExportFormat::Prometheus,
        );

        assert_eq!(exporter.endpoint(), "http://pushgateway:9091");
        assert_eq!(exporter.interval(), 10000);
    }

    #[test]
    fn test_export_utils_sanitize_name() {
        assert_eq!(ExportUtils::sanitize_name("valid_name"), "valid_name");
        assert_eq!(ExportUtils::sanitize_name("invalid-name"), "invalid_name");
        assert_eq!(ExportUtils::sanitize_name("invalid.name"), "invalid_name");
        assert_eq!(ExportUtils::sanitize_name("name with spaces"), "name_with_spaces");
    }

    #[test]
    fn test_export_utils_sanitize_label_value() {
        assert_eq!(ExportUtils::sanitize_label_value("simple"), "simple");
        assert_eq!(
            ExportUtils::sanitize_label_value("with\"quotes"),
            "with\\\"quotes"
        );
        assert_eq!(
            ExportUtils::sanitize_label_value("with\\backslash"),
            "with\\\\backslash"
        );
        assert_eq!(ExportUtils::sanitize_label_value("with\nnewline"), "with\\nnewline");
    }

    #[test]
    fn test_export_utils_format_labels() {
        let mut labels = BTreeMap::new();
        labels.insert("env".to_string(), "prod".to_string());
        labels.insert("service".to_string(), "api".to_string());

        let formatted = ExportUtils::format_labels(&labels);
        assert!(formatted.contains("env=\"prod\""));
        assert!(formatted.contains("service=\"api\""));
    }

    #[test]
    fn test_export_utils_metric_type_to_string() {
        assert_eq!(
            ExportUtils::metric_type_to_string(MetricType::Counter),
            "counter"
        );
        assert_eq!(ExportUtils::metric_type_to_string(MetricType::Gauge), "gauge");
        assert_eq!(
            ExportUtils::metric_type_to_string(MetricType::Histogram),
            "histogram"
        );
    }
}
