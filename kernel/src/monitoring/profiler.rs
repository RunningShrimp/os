//! CPU profiler with stack sampling
//!
//! Provides CPU profiling capabilities with minimal overhead.

extern crate alloc;

use alloc::{vec::Vec, string::String, collections::BTreeMap};
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::subsystems::sync::Mutex;
use crate::subsystems::time;

/// CPU profiler
pub struct Profiler {
    /// Enable/disable flag
    enabled: AtomicBool,
    /// Sample buffer
    samples: Mutex<Vec<ProfilerSample>>,
    /// Sample interval in nanoseconds
    sample_interval_ns: u64,
    /// Total samples taken
    sample_count: AtomicU64,
    /// Profiler name
    name: String,
    /// Maximum samples to collect
    max_samples: usize,
}

/// Profiler sample containing IP and stack trace
#[derive(Debug, Clone)]
pub struct ProfilerSample {
    /// Instruction pointer
    pub instruction_pointer: u64,
    /// Stack trace (addresses)
    pub stack_trace: Vec<u64>,
    /// Timestamp (nanoseconds)
    pub timestamp: u64,
    /// CPU ID
    pub cpu_id: u32,
}

/// Flame graph node
#[derive(Debug, Clone)]
pub struct FlameGraphNode {
    /// Function name or address
    pub name: String,
    /// Total samples in this node
    pub samples: u64,
    /// Total samples in children
    pub total_samples: u64,
    /// Child nodes
    pub children: Vec<FlameGraphNode>,
}

impl Profiler {
    /// Create a new profiler
    ///
    /// # Arguments
    /// * `name` - Profiler name
    /// * `sample_interval_ns` - Sampling interval in nanoseconds
    /// * `max_samples` - Maximum number of samples to collect (0 = unlimited)
    pub fn new(name: &str, sample_interval_ns: u64, max_samples: usize) -> Self {
        Self {
            enabled: AtomicBool::new(false),
            samples: Mutex::new(Vec::new()),
            sample_interval_ns,
            sample_count: AtomicU64::new(0),
            name: String::from(name),
            max_samples,
        }
    }

    /// Start profiling
    pub fn start(&self) {
        self.enabled.store(true, Ordering::Release);
        crate::println!("[profiler] {} started", self.name);
    }

    /// Stop profiling
    pub fn stop(&self) {
        self.enabled.store(false, Ordering::Release);
        crate::println!("[profiler] {} stopped", self.name);
    }

    /// Check if profiling is active
    pub fn is_running(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Take a sample (called by timer interrupt or profiler thread)
    pub fn sample(&self) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        // Check if we've reached max samples
        let count = self.sample_count.fetch_add(1, Ordering::Relaxed);
        if self.max_samples > 0 && count >= self.max_samples as u64 {
            self.enabled.store(false, Ordering::Release);
            return;
        }

        // Capture current state
        let sample = ProfilerSample {
            instruction_pointer: Self::current_ip(),
            stack_trace: Self::capture_stack_trace(),
            timestamp: time::hrtime_nanos(),
            cpu_id: Self::current_cpu_id(),
        };

        self.samples.lock().push(sample);
    }

    /// Get all samples
    pub fn get_samples(&self) -> Vec<ProfilerSample> {
        self.samples.lock().clone()
    }

    /// Get sample count
    pub fn sample_count(&self) -> u64 {
        self.sample_count.load(Ordering::Relaxed)
    }

    /// Clear all samples
    pub fn clear(&self) {
        self.samples.lock().clear();
        self.sample_count.store(0, Ordering::Relaxed);
    }

    /// Generate flame graph data
    pub fn flame_graph(&self) -> FlameGraphNode {
        let samples = self.samples.lock();

        if samples.is_empty() {
            return FlameGraphNode {
                name: String::from("root"),
                samples: 0,
                total_samples: 0,
                children: Vec::new(),
            };
        }

        // Build frequency map of stack traces
        let mut frequency_map: BTreeMap<Vec<u64>, u64> = BTreeMap::new();
        for sample in samples.iter() {
            *frequency_map.entry(sample.stack_trace.clone()).or_insert(0) += 1;
        }

        // Build flame graph tree
        let root = Self::build_flame_tree(&frequency_map);
        root
    }

    /// Build flame graph tree from frequency map
    fn build_flame_tree(frequency_map: &BTreeMap<Vec<u64>, u64>) -> FlameGraphNode {
        let mut root = FlameGraphNode {
            name: String::from("root"),
            samples: 0,
            total_samples: 0,
            children: Vec::new(),
        };

        for (stack_trace, count) in frequency_map.iter() {
            root.total_samples += count;
            Self::insert_into_tree(&mut root, stack_trace, *count);
        }

        root.samples = root.total_samples;
        root
    }

    /// Insert a stack trace into the flame graph tree
    fn insert_into_tree(node: &mut FlameGraphNode, stack_trace: &[u64], count: u64) {
        if stack_trace.is_empty() {
            node.samples += count;
            return;
        }

        let current_addr = stack_trace[0];
        let name = Self::addr_to_name(current_addr);

        // Find or create child node
        let child_exists = node.children.iter().any(|c| c.name == name);
        if child_exists {
            let child = node.children.iter_mut().find(|c| c.name == name).unwrap();
            child.total_samples += count;
            Self::insert_into_tree(child, &stack_trace[1..], count);
        } else {
            let mut new_child = FlameGraphNode {
                name,
                samples: 0,
                total_samples: count,
                children: Vec::new(),
            };
            Self::insert_into_tree(&mut new_child, &stack_trace[1..], count);
            node.children.push(new_child);
        }
    }

    /// Convert address to function name (stub implementation)
    fn addr_to_name(addr: u64) -> String {
        // TODO: Implement symbol resolution
        alloc::format!("fn_{:#x}", addr)
    }

    /// Get current instruction pointer (stub implementation)
    fn current_ip() -> u64 {
        // TODO: Implement actual IP capture
        // This would typically use:
        // - x86_64: __builtin_return_address or inline asm
        // - ARM: special register read
        0
    }

    /// Capture stack trace (stub implementation)
    fn capture_stack_trace() -> Vec<u64> {
        // TODO: Implement actual stack unwinding
        // This would typically use:
        // - libunwind
        // - frame pointer walking
        // - ORC unwind tables
        Vec::new()
    }

    /// Get current CPU ID (stub implementation)
    fn current_cpu_id() -> u32 {
        // TODO: Implement actual CPU ID retrieval
        0
    }

    /// Format flame graph as string (SVG-like format)
    pub fn format_flame_graph(&self) -> String {
        let root = self.flame_graph();
        Self::format_node(&root, 0)
    }

    /// Format a flame graph node recursively
    fn format_node(node: &FlameGraphNode, depth: usize) -> String {
        let indent = "  ".repeat(depth);
        let mut output = format!("{}{} ({} samples)\n", indent, node.name, node.samples);

        for child in &node.children {
            output.push_str(&Self::format_node(child, depth + 1));
        }

        output
    }

    /// Export samples as JSON
    pub fn export_json(&self) -> String {
        let samples = self.samples.lock();
        let mut output = String::from("{\n");
        output.push_str(&format!("  \"name\": \"{}\",\n", self.name));
        output.push_str(&format!("  \"sample_count\": {},\n", samples.len()));
        output.push_str("  \"samples\": [\n");

        for (i, sample) in samples.iter().enumerate() {
            if i > 0 {
                output.push_str(",\n");
            }
            output.push_str("    {\n");
            output.push_str(&format!("      \"ip\": \"{}\",\n", sample.instruction_pointer));
            output.push_str(&format!("      \"timestamp\": {},\n", sample.timestamp));
            output.push_str(&format!("      \"cpu_id\": {},\n", sample.cpu_id));
            output.push_str("      \"stack\": [");
            for (j, addr) in sample.stack_trace.iter().enumerate() {
                if j > 0 {
                    output.push_str(", ");
                }
                output.push_str(&format!("\"{}\"", addr));
            }
            output.push_str("]\n");
            output.push_str("    }");
        }

        output.push_str("\n  ]\n}\n");
        output
    }
}

/// Global profiler instance
static GLOBAL_PROFILER: Mutex<Option<Profiler>> = Mutex::new(None);

/// Initialize global profiler
pub fn init_profiler(sample_interval_ns: u64, max_samples: usize) {
    let mut profiler = GLOBAL_PROFILER.lock();
    if profiler.is_none() {
        *profiler = Some(Profiler::new("global", sample_interval_ns, max_samples));
        crate::println!("[profiler] Global profiler initialized");
    }
}

/// Get global profiler
pub fn get_profiler() -> Option<&'static Profiler> {
    // Note: This is a simplified implementation
    // A production version would use proper static initialization
    None
}

/// Start global profiling
pub fn start_profiling() {
    if let Some(profiler) = get_profiler() {
        profiler.start();
    }
}

/// Stop global profiling
pub fn stop_profiling() {
    if let Some(profiler) = get_profiler() {
        profiler.stop();
    }
}

/// Profile a function call (manual instrumentation)
pub fn profile_function<F, R>(name: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    let start = time::hrtime_nanos();
    let result = f();
    let duration = time::hrtime_nanos().saturating_sub(start);

    // Log profile data
    crate::println!("[profile] {} took {} ns", name, duration);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_creation() {
        let profiler = Profiler::new("test", 1_000_000, 1000);
        assert_eq!(profiler.name, "test");
        assert!(!profiler.is_running());
        assert_eq!(profiler.sample_count(), 0);
    }

    #[test]
    fn test_profiler_start_stop() {
        let profiler = Profiler::new("test", 1_000_000, 1000);
        assert!(!profiler.is_running());

        profiler.start();
        assert!(profiler.is_running());

        profiler.stop();
        assert!(!profiler.is_running());
    }

    #[test]
    fn test_profiler_clear() {
        let profiler = Profiler::new("test", 1_000_000, 1000);
        profiler.start();

        // Simulate some samples
        profiler.sample();
        profiler.sample();

        assert_eq!(profiler.sample_count(), 2);

        profiler.clear();
        assert_eq!(profiler.sample_count(), 0);
    }

    #[test]
    fn test_flame_graph_empty() {
        let profiler = Profiler::new("test", 1_000_000, 1000);
        let flame = profiler.flame_graph();

        assert_eq!(flame.name, "root");
        assert_eq!(flame.samples, 0);
        assert!(flame.children.is_empty());
    }

    #[test]
    fn test_flame_graph_node_format() {
        let node = FlameGraphNode {
            name: String::from("test_func"),
            samples: 100,
            total_samples: 150,
            children: vec![
                FlameGraphNode {
                    name: String::from("child1"),
                    samples: 30,
                    total_samples: 50,
                    children: Vec::new(),
                },
            ],
        };

        let formatted = Profiler::format_node(&node, 0);
        assert!(formatted.contains("test_func"));
        assert!(formatted.contains("100 samples"));
        assert!(formatted.contains("child1"));
    }

    #[test]
    fn test_profile_function() {
        let result = profile_function("test_func", || {
            let mut sum = 0;
            for i in 0..100 {
                sum += i;
            }
            sum
        });

        assert_eq!(result, 4950);
    }
}
