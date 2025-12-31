//! Metrics export functionality
//!
//! Provides export of metrics in various formats (Prometheus, JSON, etc.).

extern crate alloc;

use alloc::{string::String, vec::Vec, collections::BTreeMap, format};
use crate::subsystems::sync::Mutex;

use crate::monitoring::metrics::{MetricType, MetricsCollector};
use crate::monitoring::sampling::get_sampler;

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Prometheus text format
    Prometheus,
    /// JSON format
    Json,
    /// Plain text format
    Text,
}

/// Export metrics in specified format
pub fn export_metrics(collector: &MetricsCollector, format: ExportFormat) -> String {
    match format {
        ExportFormat::Prometheus => export_prometheus(collector),
        ExportFormat::Json => export_json(collector),
        ExportFormat::Text => export_text(collector),
    }
}

/// Export metrics in Prometheus text format
pub fn export_prometheus(collector: &MetricsCollector) -> String {
    let metrics = collector.collect_metrics();
    let mut output = String::new();

    // Group metrics by name (remove type suffix)
    let mut grouped: BTreeMap<String, Vec<(String, u64, MetricType)>> = BTreeMap::new();

    for (key, value) in metrics.iter() {
        // Parse key to extract name and type
        if let Some(pos) = key.find('_') {
            let name = &key[..pos];
            let type_suffix = &key[pos + 1..];

            let metric_type = match type_suffix {
                "counter" => MetricType::Counter,
                "gauge" => MetricType::Gauge,
                "histogram" => MetricType::Histogram,
                _ => MetricType::Counter,
            };

            grouped
                .entry(String::from(name))
                .or_insert_with(Vec::new)
                .push((key.clone(), *value, metric_type));
        }
    }

    // Export in Prometheus format
    for (name, entries) in grouped.iter() {
        if let Some((_, _, metric_type)) = entries.first() {
            // Output TYPE and HELP
            output.push_str("# TYPE ");
            output.push_str(name);
            output.push(' ');
            match metric_type {
                MetricType::Counter => output.push_str("counter\n"),
                MetricType::Gauge => output.push_str("gauge\n"),
                MetricType::Histogram => output.push_str("histogram\n"),
            }
        }

        // Output values
        for (key, value, _) in entries.iter() {
            output.push_str(key);
            output.push(' ');
            output.push_str(&format!("{}", value));
            output.push('\n');
        }

        output.push('\n');
    }

    output
}

/// Export metrics in JSON format
pub fn export_json(collector: &MetricsCollector) -> String {
    let metrics = collector.collect_metrics();
    let mut output = String::from("{\n");

    output.push_str("  \"metrics\": {\n");

    let mut first = true;
    for (key, value) in metrics.iter() {
        if !first {
            output.push_str(",\n");
        }
        first = false;

        output.push_str("    \"");
        output.push_str(key);
        output.push_str("\": ");
        output.push_str(&format!("{}", value));
    }

    output.push_str("\n  }\n}\n");
    output
}

/// Export metrics in plain text format
pub fn export_text(collector: &MetricsCollector) -> String {
    let metrics = collector.collect_metrics();
    let mut output = String::from("System Metrics\n");
    output.push_str("===============\n\n");

    // Group by category
    let mut categories: BTreeMap<&str, Vec<(String, u64)>> = BTreeMap::new();

    for (key, value) in metrics.iter() {
        let category = if key.starts_with("syscall") {
            "System Calls"
        } else if key.starts_with("process") {
            "Processes"
        } else if key.starts_with("memory") {
            "Memory"
        } else if key.starts_with("scheduler") {
            "Scheduler"
        } else if key.starts_with("lock") {
            "Locks"
        } else {
            "Other"
        };

        categories
            .entry(category)
            .or_insert_with(Vec::new)
            .push((key.clone(), *value));
    }

    // Output by category
    for (category, entries) in categories.iter() {
        output.push_str(category);
        output.push_str(":\n");

        for (key, value) in entries.iter() {
            output.push_str("  ");
            output.push_str(key);
            output.push_str(": ");
            output.push_str(&format!("{}", value));
            output.push('\n');
        }

        output.push('\n');
    }

    output
}

/// Export sampling statistics
pub fn export_sampling_stats(sampler_name: &str) -> String {
    if let Some(sampler) = get_sampler(sampler_name) {
        let summary = sampler.summary();
        summary.format()
    } else {
        format!("Sampler '{}' not found", sampler_name)
    }
}

/// Export all sampling statistics in JSON
pub fn export_all_sampling_stats() -> String {
    let summaries = crate::monitoring::sampling::get_all_summaries();
    let mut output = String::from("{\n");
    output.push_str("  \"samplers\": [\n");

    for (i, summary) in summaries.iter().enumerate() {
        if i > 0 {
            output.push_str(",\n");
        }

        output.push_str("    {\n");
        output.push_str(&format!("      \"name\": \"{}\",\n", summary.sampler_name));
        output.push_str(&format!("      \"total_calls\": {},\n", summary.total_samples));
        output.push_str(&format!("      \"recorded_samples\": {},\n", summary.recorded_samples));
        output.push_str(&format!("      \"avg_duration_ns\": {},\n", summary.avg_duration));
        output.push_str(&format!("      \"min_duration_ns\": {},\n", summary.min_duration));
        output.push_str(&format!("      \"max_duration_ns\": {},\n", summary.max_duration));
        output.push_str(&format!("      \"p50_ns\": {},\n", summary.p50));
        output.push_str(&format!("      \"p95_ns\": {},\n", summary.p95));
        output.push_str(&format!("      \"p99_ns\": {}\n", summary.p99));
        output.push_str("    }");
    }

    output.push_str("\n  ]\n}\n");
    output
}

/// Export profiler data
pub fn export_profiler_data(format: ExportFormat) -> String {
    use crate::monitoring::profiler::get_profiler;

    if let Some(profiler) = get_profiler() {
        match format {
            ExportFormat::Json => profiler.export_json(),
            ExportFormat::Text => profiler.format_flame_graph(),
            _ => String::from("Profiler data not available in this format"),
        }
    } else {
        String::from("Profiler not initialized")
    }
}

/// Export all monitoring data
pub fn export_all_data(format: ExportFormat) -> String {
    let mut output = String::new();

    // Add metrics
    if let Ok(collector) = crate::monitoring::metrics::get_metrics_collector()
        .try_get()
    {
        output.push_str("# Metrics\n");
        output.push_str(&export_metrics(&collector, format));
        output.push('\n');
    }

    // Add sampling stats
    output.push_str("# Sampling Statistics\n");
    if format == ExportFormat::Json {
        output.push_str(&export_all_sampling_stats());
    } else {
        let summaries = crate::monitoring::sampling::get_all_summaries();
        for summary in summaries.iter() {
            output.push_str(&summary.format());
            output.push_str("\n\n");
        }
    }
    output.push('\n');

    // Add profiler data if available
    output.push_str("# Profiler Data\n");
    output.push_str(&export_profiler_data(format));

    output
}

/// Export to file (via VFS)
pub fn export_to_file(path: &str, data: &str) -> Result<(), &'static str> {
    // TODO: Implement actual file writing via VFS
    // This would use crate::vfs::filesystem
    crate::println!("[export] Would write {} bytes to {}", data.len(), path);
    Ok(())
}

/// Streaming exporter for real-time metrics
pub struct StreamingExporter {
    /// Buffer for accumulated data
    buffer: Mutex<String>,
    /// Maximum buffer size before flush
    max_buffer_size: usize,
    /// Export format
    format: ExportFormat,
    /// Enable/disable flag
    enabled: core::sync::atomic::AtomicBool,
}

impl StreamingExporter {
    /// Create a new streaming exporter
    pub fn new(format: ExportFormat, max_buffer_size: usize) -> Self {
        Self {
            buffer: Mutex::new(String::new()),
            max_buffer_size,
            format,
            enabled: core::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Start streaming
    pub fn start(&self) {
        self.enabled.store(true, core::sync::atomic::Ordering::Release);
    }

    /// Stop streaming
    pub fn stop(&self) {
        self.enabled.store(false, core::sync::atomic::Ordering::Release);
    }

    /// Add data to buffer
    pub fn add_data(&self, data: &str) {
        if !self.enabled.load(core::sync::atomic::Ordering::Relaxed) {
            return;
        }

        let mut buffer = self.buffer.lock();
        buffer.push_str(data);

        // Auto-flush if buffer is too large
        if buffer.len() >= self.max_buffer_size {
            self.flush_internal(&mut buffer);
        }
    }

    /// Flush buffer to destination
    pub fn flush(&self) {
        let mut buffer = self.buffer.lock();
        self.flush_internal(&mut buffer);
    }

    /// Internal flush implementation
    fn flush_internal(&self, buffer: &mut String) {
        if buffer.is_empty() {
            return;
        }

        // TODO: Actually write to destination (file, network, etc.)
        crate::println!("[export] Flushing {} bytes", buffer.len());
        buffer.clear();
    }

    /// Get current buffer content
    pub fn get_buffer_content(&self) -> String {
        self.buffer.lock().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_prometheus() {
        // Note: This test would need a proper MetricsCollector instance
        // For now, we just test that the function exists
        let result = export_prometheus(&MetricsCollector::new());
        assert!(result.contains("# TYPE"));
    }

    #[test]
    fn test_export_json() {
        let result = export_json(&MetricsCollector::new());
        assert!(result.contains("{"));
        assert!(result.contains("}"));
        assert!(result.contains("metrics"));
    }

    #[test]
    fn test_export_text() {
        let result = export_text(&MetricsCollector::new());
        assert!(result.contains("System Metrics"));
    }

    #[test]
    fn test_streaming_exporter() {
        let exporter = StreamingExporter::new(ExportFormat::Text, 1024);

        assert!(!exporter.enabled.load(core::sync::atomic::Ordering::Relaxed));

        exporter.start();
        assert!(exporter.enabled.load(core::sync::atomic::Ordering::Relaxed));

        exporter.add_data("test data\n");
        let content = exporter.get_buffer_content();
        assert!(content.contains("test data"));

        exporter.flush();
        let content = exporter.get_buffer_content();
        assert!(content.is_empty());

        exporter.stop();
        assert!(!exporter.enabled.load(core::sync::atomic::Ordering::Relaxed));
    }
}
