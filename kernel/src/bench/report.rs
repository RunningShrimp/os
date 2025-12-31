//! Performance Report Generation
//!
//! Comprehensive report generation for benchmark results, including:
//! - Statistical analysis
//! - Trend comparison
//! - Machine-readable output (JSON)
//! - Human-readable formatting
//! - Visual chart generation support

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use alloc::format;

use super::BenchmarkResult;

/// Complete performance report
#[derive(Debug, Clone)]
pub struct BenchmarkReport {
    /// Report timestamp
    pub timestamp: u64,
    /// System information
    pub system_info: SystemInfo,
    /// Scheduler benchmarks
    pub scheduler_results: Vec<BenchmarkResult>,
    /// Memory benchmarks
    pub memory_results: Vec<BenchmarkResult>,
    /// Network benchmarks
    pub network_results: Vec<BenchmarkResult>,
    /// Filesystem benchmarks
    pub filesystem_results: Vec<BenchmarkResult>,
    /// Syscall benchmarks
    pub syscall_results: Vec<BenchmarkResult>,
}

/// System information for benchmark context
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub cpu_name: String,
    pub cpu_cores: usize,
    pub cpu_frequency_mhz: u64,
    pub total_memory_mb: u64,
    pub kernel_version: String,
    pub compiler: String,
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            cpu_name: String::from("Unknown"),
            cpu_cores: 1,
            cpu_frequency_mhz: 3000,
            total_memory_mb: 1024,
            kernel_version: String::from("0.1.0"),
            compiler: String::from("rustc"),
        }
    }
}

/// Performance metrics summary
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Overall system performance score (0-100)
    pub overall_score: f64,
    /// Scheduler score
    pub scheduler_score: f64,
    /// Memory score
    pub memory_score: f64,
    /// Network score
    pub network_score: f64,
    /// Filesystem score
    pub filesystem_score: f64,
    /// Syscall score
    pub syscall_score: f64,
    /// Critical issues found
    pub issues: Vec<String>,
    /// Performance recommendations
    pub recommendations: Vec<String>,
}

impl BenchmarkReport {
    /// Create new benchmark report
    pub fn new(system_info: SystemInfo) -> Self {
        Self {
            timestamp: 0, // Will be set when report is finalized
            system_info,
            scheduler_results: Vec::new(),
            memory_results: Vec::new(),
            network_results: Vec::new(),
            filesystem_results: Vec::new(),
            syscall_results: Vec::new(),
        }
    }

    /// Add scheduler results
    pub fn add_scheduler_results(&mut self, results: Vec<BenchmarkResult>) {
        self.scheduler_results = results;
    }

    /// Add memory results
    pub fn add_memory_results(&mut self, results: Vec<BenchmarkResult>) {
        self.memory_results = results;
    }

    /// Add network results
    pub fn add_network_results(&mut self, results: Vec<BenchmarkResult>) {
        self.network_results = results;
    }

    /// Add filesystem results
    pub fn add_filesystem_results(&mut self, results: Vec<BenchmarkResult>) {
        self.filesystem_results = results;
    }

    /// Add syscall results
    pub fn add_syscall_results(&mut self, results: Vec<BenchmarkResult>) {
        self.syscall_results = results;
    }

    /// Generate performance metrics summary
    pub fn generate_metrics(&self) -> PerformanceMetrics {
        let scheduler_score = self.calculate_scheduler_score();
        let memory_score = self.calculate_memory_score();
        let network_score = self.calculate_network_score();
        let filesystem_score = self.calculate_filesystem_score();
        let syscall_score = self.calculate_syscall_score();

        let overall_score = (scheduler_score + memory_score + network_score
            + filesystem_score + syscall_score) / 5.0;

        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Analyze scheduler performance
        if scheduler_score < 50.0 {
            issues.push(String::from("Scheduler context switch latency is high"));
            recommendations.push(String::from("Consider optimizing context switch path"));
        }

        // Analyze memory performance
        if memory_score < 50.0 {
            issues.push(String::from("Memory allocation latency is high"));
            recommendations.push(String::from("Review allocator implementation and caching strategy"));
        }

        // Analyze network performance
        if network_score < 50.0 {
            issues.push(String::from("Network throughput is low"));
            recommendations.push(String::from("Optimize network stack and packet processing"));
        }

        // Analyze filesystem performance
        if filesystem_score < 50.0 {
            issues.push(String::from("Filesystem I/O is slow"));
            recommendations.push(String::from("Implement better caching and readahead"));
        }

        // Analyze syscall performance
        if syscall_score < 50.0 {
            issues.push(String::from("Syscall overhead is high"));
            recommendations.push(String::from("Optimize syscall entry/exit path"));
        }

        PerformanceMetrics {
            overall_score,
            scheduler_score,
            memory_score,
            network_score,
            filesystem_score,
            syscall_score,
            issues,
            recommendations,
        }
    }

    fn calculate_scheduler_score(&self) -> f64 {
        if self.scheduler_results.is_empty() {
            return 0.0;
        }

        let context_switch_ns = self
            .scheduler_results
            .iter()
            .find(|r| r.name == "context_switch_latency")
            .map(|r| r.avg_duration.as_nanos() as f64)
            .unwrap_or(1000.0);

        // Score: 100ns = 100 points, 1000ns = 50 points, 10000ns = 0 points
        f64::max(0.0, f64::min(100.0, 1000.0 - context_switch_ns.log(10.0) * 30.0))
    }

    fn calculate_memory_score(&self) -> f64 {
        if self.memory_results.is_empty() {
            return 0.0;
        }

        let slab_ns = self
            .memory_results
            .iter()
            .find(|r| r.name == "slab_allocator")
            .map(|r| r.avg_duration.as_nanos() as f64)
            .unwrap_or(100.0);

        // Score: 10ns = 100 points, 100ns = 50 points, 1000ns = 0 points
        f64::max(0.0, f64::min(100.0, 100.0 - slab_ns.log(10.0) * 30.0))
    }

    fn calculate_network_score(&self) -> f64 {
        if self.network_results.is_empty() {
            return 0.0;
        }

        // Score based on throughput (1 Gbps = 100 points)
        let throughput_mbps = self
            .network_results
            .iter()
            .find(|r| r.name == "tcp_throughput")
            .map(|r| {
                let bytes = 1024 * 1024;
                let duration_sec = r.total_duration.as_secs_f64();
                (bytes as f64 * 8.0) / (duration_sec * 1_000_000.0)
            })
            .unwrap_or(0.0);

        (throughput_mbps / 10.0).max(0.0).min(100.0)
    }

    fn calculate_filesystem_score(&self) -> f64 {
        if self.filesystem_results.is_empty() {
            return 0.0;
        }

        // Score based on sequential read (100 MB/s = 100 points)
        let read_mbps = self
            .filesystem_results
            .iter()
            .find(|r| r.name == "sequential_read")
            .map(|r| {
                let bytes = 10 * 1024 * 1024;
                let duration_sec = r.total_duration.as_secs_f64();
                (bytes as f64 / (1024.0 * 1024.0)) / duration_sec
            })
            .unwrap_or(0.0);

        (read_mbps).max(0.0).min(100.0)
    }

    fn calculate_syscall_score(&self) -> f64 {
        if self.syscall_results.is_empty() {
            return 0.0;
        }

        let null_syscall_ns = self
            .syscall_results
            .iter()
            .find(|r| r.name == "null_syscall")
            .map(|r| r.avg_duration.as_nanos() as f64)
            .unwrap_or(1000.0);

        // Score: 50ns = 100 points, 500ns = 50 points, 5000ns = 0 points
        f64::max(0.0, f64::min(100.0, 100.0 - null_syscall_ns.log(10.0) * 25.0))
    }

    /// Format report as human-readable text
    pub fn format_text(&self) -> String {
        let metrics = self.generate_metrics();

        let mut output = String::from("╔════════════════════════════════════════════════════════════╗\n");
        output.push_str("║           NOS Kernel Performance Benchmark Report            ║\n");
        output.push_str("╚════════════════════════════════════════════════════════════╝\n\n");

        output.push_str(&format!("System Information:\n"));
        output.push_str(&format!("  CPU: {} ({} cores @ {} MHz)\n",
            self.system_info.cpu_name,
            self.system_info.cpu_cores,
            self.system_info.cpu_frequency_mhz));
        output.push_str(&format!("  Memory: {} MB\n", self.system_info.total_memory_mb));
        output.push_str(&format!("  Kernel: {}\n", self.system_info.kernel_version));
        output.push_str(&format!("  Compiler: {}\n\n", self.system_info.compiler));

        output.push_str(&format!("Overall Performance Score: {:.1}/100\n\n", metrics.overall_score));

        output.push_str(&format!("Category Scores:\n"));
        output.push_str(&format!("  Scheduler: {:.1}/100\n", metrics.scheduler_score));
        output.push_str(&format!("  Memory:    {:.1}/100\n", metrics.memory_score));
        output.push_str(&format!("  Network:   {:.1}/100\n", metrics.network_score));
        output.push_str(&format!("  Filesystem: {:.1}/100\n", metrics.filesystem_score));
        output.push_str(&format!("  Syscall:   {:.1}/100\n\n", metrics.syscall_score));

        if !metrics.issues.is_empty() {
            output.push_str("Issues Found:\n");
            for issue in &metrics.issues {
                output.push_str(&format!("  - {}\n", issue));
            }
            output.push_str("\n");
        }

        if !metrics.recommendations.is_empty() {
            output.push_str("Recommendations:\n");
            for rec in &metrics.recommendations {
                output.push_str(&format!("  - {}\n", rec));
            }
            output.push_str("\n");
        }

        output.push_str("Detailed Results:\n");
        output.push_str(&format_results("Scheduler", &self.scheduler_results));
        output.push_str(&format_results("Memory", &self.memory_results));
        output.push_str(&format_results("Network", &self.network_results));
        output.push_str(&format_results("Filesystem", &self.filesystem_results));
        output.push_str(&format_results("Syscall", &self.syscall_results));

        output
    }

    /// Format report as machine-readable JSON
    pub fn format_json(&self) -> String {
        let metrics = self.generate_metrics();

        let mut json = String::from("{\n");
        json.push_str(&format!("  \"timestamp\": {},\n", self.timestamp));
        json.push_str("  \"system_info\": {\n");
        json.push_str(&format!("    \"cpu_name\": \"{}\",\n", self.system_info.cpu_name));
        json.push_str(&format!("    \"cpu_cores\": {},\n", self.system_info.cpu_cores));
        json.push_str(&format!("    \"cpu_frequency_mhz\": {},\n", self.system_info.cpu_frequency_mhz));
        json.push_str(&format!("    \"total_memory_mb\": {},\n", self.system_info.total_memory_mb));
        json.push_str(&format!("    \"kernel_version\": \"{}\",\n", self.system_info.kernel_version));
        json.push_str(&format!("    \"compiler\": \"{}\"\n", self.system_info.compiler));
        json.push_str("  },\n");

        json.push_str("  \"metrics\": {\n");
        json.push_str(&format!("    \"overall_score\": {:.2},\n", metrics.overall_score));
        json.push_str(&format!("    \"scheduler_score\": {:.2},\n", metrics.scheduler_score));
        json.push_str(&format!("    \"memory_score\": {:.2},\n", metrics.memory_score));
        json.push_str(&format!("    \"network_score\": {:.2},\n", metrics.network_score));
        json.push_str(&format!("    \"filesystem_score\": {:.2},\n", metrics.filesystem_score));
        json.push_str(&format!("    \"syscall_score\": {:.2}\n", metrics.syscall_score));
        json.push_str("  },\n");

        json.push_str("  \"results\": {\n");
        json.push_str(&format_json_results("scheduler", &self.scheduler_results));
        json.push_str(&format_json_results("memory", &self.memory_results));
        json.push_str(&format_json_results("network", &self.network_results));
        json.push_str(&format_json_results("filesystem", &self.filesystem_results));
        json.push_str(&format_json_results("syscall", &self.syscall_results));
        json.push_str("  }\n");
        json.push_str("}\n");

        json
    }

    /// Export data for chart generation
    pub fn export_chart_data(&self) -> BTreeMap<String, Vec<(String, f64)>> {
        let mut data = BTreeMap::new();

        // Export throughput data
        let mut throughput = Vec::new();
        for result in &self.network_results {
            throughput.push((result.name.clone(), result.throughput));
        }
        data.insert(String::from("throughput"), throughput);

        // Export latency data
        let mut latency = Vec::new();
        for result in &self.syscall_results {
            latency.push((result.name.clone(), result.avg_duration.as_nanos() as f64));
        }
        data.insert(String::from("latency"), latency);

        data
    }
}

fn format_results(category: &str, results: &[BenchmarkResult]) -> String {
    let mut output = format!("{}:\n", category);
    for result in results {
        output.push_str(&format!(
            "  {}: avg={:.2}ns, min={:.2}ns, max={:.2}ns, throughput={:.0}/s\n",
            result.name,
            result.avg_duration.as_nanos(),
            result.min_duration.as_nanos(),
            result.max_duration.as_nanos(),
            result.throughput
        ));
    }
    output.push_str("\n");
    output
}

fn format_json_results(category: &str, results: &[BenchmarkResult]) -> String {
    let mut output = format!("    \"{}\": [\n", category);
    for (i, result) in results.iter().enumerate() {
        output.push_str("      {\n");
        output.push_str(&format!("        \"name\": \"{}\",\n", result.name));
        output.push_str(&format!("        \"iterations\": {},\n", result.iterations));
        output.push_str(&format!("        \"avg_ns\": {},\n", result.avg_duration.as_nanos()));
        output.push_str(&format!("        \"min_ns\": {},\n", result.min_duration.as_nanos()));
        output.push_str(&format!("        \"max_ns\": {},\n", result.max_duration.as_nanos()));
        output.push_str(&format!("        \"std_dev\": {},\n", result.std_dev));
        output.push_str(&format!("        \"throughput\": {:.2}\n", result.throughput));
        output.push_str(&format!("      }}{}", if i < results.len() - 1 { "," } else { "" }));
        output.push('\n');
    }
    output.push_str("    ],\n");
    output
}

/// Compare two benchmark reports
pub fn compare_reports(
    baseline: &BenchmarkReport,
    current: &BenchmarkReport,
) -> ComparisonReport {
    let mut changes = Vec::new();

    // Compare scheduler performance
    if let (Some(b), Some(c)) = (
        baseline.scheduler_results.iter().find(|r| r.name == "context_switch_latency"),
        current.scheduler_results.iter().find(|r| r.name == "context_switch_latency"),
    ) {
        let pct_change = ((c.avg_duration.as_nanos() as f64 - b.avg_duration.as_nanos() as f64)
            / b.avg_duration.as_nanos() as f64) * 100.0;
        changes.push(format!(
            "Context switch latency: {:.1}% ({:+.1}ns)",
            pct_change,
            c.avg_duration.as_nanos() as f64 - b.avg_duration.as_nanos() as f64
        ));
    }

    ComparisonReport {
        baseline_score: baseline.generate_metrics().overall_score,
        current_score: current.generate_metrics().overall_score,
        score_change: current.generate_metrics().overall_score
            - baseline.generate_metrics().overall_score,
        changes,
    }
}

/// Comparison report between two benchmark runs
#[derive(Debug, Clone)]
pub struct ComparisonReport {
    pub baseline_score: f64,
    pub current_score: f64,
    pub score_change: f64,
    pub changes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_report_creation() {
        let report = BenchmarkReport::new(SystemInfo::default());
        assert_eq!(report.scheduler_results.len(), 0);
        assert_eq!(report.memory_results.len(), 0);
    }

    #[test_case]
    fn test_metrics_generation() {
        let mut report = BenchmarkReport::new(SystemInfo::default());
        let mut result = BenchmarkResult::new(String::from("test"));
        result.avg_duration = Duration::from_nanos(500);

        report.scheduler_results.push(result);
        let metrics = report.generate_metrics();

        assert!(metrics.scheduler_score >= 0.0);
        assert!(metrics.scheduler_score <= 100.0);
    }

    #[test_case]
    fn test_format_text() {
        let report = BenchmarkReport::new(SystemInfo::default());
        let text = report.format_text();
        assert!(text.contains("Performance Benchmark Report"));
        assert!(text.contains("System Information"));
    }
}
