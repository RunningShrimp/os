//! Health probe implementations for the NOS kernel.
//!
//! This module provides various probe types for health checking:
//! - HTTP probes (GET endpoint with status check)
//! - TCP probes (port connectivity)
//! - Exec probes (run command/script)
//! - Configurable probe intervals and timeouts
//! - Probe result caching
//! - Probe history tracking

#![no_std]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::time::Duration;
use spin::Mutex;

use crate::subsystems::time::Timestamp;
use super::check::CheckConfig;

/// Types of health probes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProbeType {
    /// HTTP GET request probe
    Http,
    /// TCP connection probe
    Tcp,
    /// Exec command probe
    Exec,
}

impl ProbeType {
    /// Get probe type name
    pub fn name(&self) -> &str {
        match self {
            ProbeType::Http => "http",
            ProbeType::Tcp => "tcp",
            ProbeType::Exec => "exec",
        }
    }
}

/// HTTP probe configuration
#[derive(Debug, Clone)]
pub struct HttpProbeConfig {
    /// Target URL
    pub url: String,
    /// Expected HTTP status code (default: 200)
    pub expected_status: u16,
    /// Optional HTTP headers to send
    pub headers: Vec<(String, String)>,
    /// HTTP method (GET, POST, etc.)
    pub method: HttpMethod,
    /// Request body (optional)
    pub body: Option<String>,
    /// Follow redirects
    pub follow_redirects: bool,
    /// TLS verification
    pub verify_tls: bool,
}

/// HTTP methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    GET,
    POST,
    HEAD,
    PUT,
    DELETE,
}

impl HttpMethod {
    /// Get method name
    pub fn name(&self) -> &str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE",
        }
    }
}

impl Default for HttpProbeConfig {
    fn default() -> Self {
        HttpProbeConfig {
            url: String::new(),
            expected_status: 200,
            headers: Vec::new(),
            method: HttpMethod::GET,
            body: None,
            follow_redirects: true,
            verify_tls: true,
        }
    }
}

impl HttpProbeConfig {
    /// Create new HTTP probe config
    pub fn new(url: String) -> Self {
        HttpProbeConfig {
            url,
            ..Default::default()
        }
    }

    /// Set expected status code
    pub fn with_expected_status(mut self, status: u16) -> Self {
        self.expected_status = status;
        self
    }

    /// Set HTTP method
    pub fn with_method(mut self, method: HttpMethod) -> Self {
        self.method = method;
        self
    }

    /// Add header
    pub fn with_header(mut self, name: String, value: String) -> Self {
        self.headers.push((name, value));
        self
    }

    /// Set body
    pub fn with_body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }
}

/// TCP probe configuration
#[derive(Debug, Clone)]
pub struct TcpProbeConfig {
    /// Target host
    pub host: String,
    /// Target port
    pub port: u16,
    /// Connection timeout
    pub timeout: Duration,
    /// Send data after connection (optional)
    pub send_data: Option<Vec<u8>>,
    /// Expected response (optional)
    pub expect_response: Option<Vec<u8>>,
}

impl Default for TcpProbeConfig {
    fn default() -> Self {
        TcpProbeConfig {
            host: String::new(),
            port: 8080,
            timeout: Duration::from_secs(5),
            send_data: None,
            expect_response: None,
        }
    }
}

impl TcpProbeConfig {
    /// Create new TCP probe config
    pub fn new(host: String, port: u16) -> Self {
        TcpProbeConfig {
            host,
            port,
            ..Default::default()
        }
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set send data
    pub fn with_send_data(mut self, data: Vec<u8>) -> Self {
        self.send_data = Some(data);
        self
    }

    /// Set expected response
    pub fn with_expect_response(mut self, data: Vec<u8>) -> Self {
        self.expect_response = Some(data);
        self
    }
}

/// Exec probe configuration
#[derive(Debug, Clone)]
pub struct ExecProbeConfig {
    /// Command to execute
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Working directory
    pub working_dir: Option<String>,
    /// Environment variables
    pub env: Vec<(String, String)>,
    /// Timeout for execution
    pub timeout: Duration,
    /// Expected exit code
    pub expected_exit_code: i32,
}

impl Default for ExecProbeConfig {
    fn default() -> Self {
        ExecProbeConfig {
            command: String::new(),
            args: Vec::new(),
            working_dir: None,
            env: Vec::new(),
            timeout: Duration::from_secs(5),
            expected_exit_code: 0,
        }
    }
}

impl ExecProbeConfig {
    /// Create new exec probe config
    pub fn new(command: String) -> Self {
        ExecProbeConfig {
            command,
            ..Default::default()
        }
    }

    /// Add argument
    pub fn with_arg(mut self, arg: String) -> Self {
        self.args.push(arg);
        self
    }

    /// Set working directory
    pub fn with_working_dir(mut self, dir: String) -> Self {
        self.working_dir = Some(dir);
        self
    }

    /// Add environment variable
    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.env.push((key, value));
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set expected exit code
    pub fn with_expected_exit_code(mut self, code: i32) -> Self {
        self.expected_exit_code = code;
        self
    }
}

/// Probe configuration
#[derive(Debug, Clone)]
pub enum ProbeConfig {
    Http(HttpProbeConfig),
    Tcp(TcpProbeConfig),
    Exec(ExecProbeConfig),
}

impl ProbeConfig {
    /// Get probe type
    pub fn probe_type(&self) -> ProbeType {
        match self {
            ProbeConfig::Http(_) => ProbeType::Http,
            ProbeConfig::Tcp(_) => ProbeType::Tcp,
            ProbeConfig::Exec(_) => ProbeType::Exec,
        }
    }
}

/// Probe execution result
#[derive(Debug, Clone)]
pub struct ProbeResult {
    /// Probe type
    pub probe_type: ProbeType,
    /// Whether probe succeeded
    pub success: bool,
    /// Timestamp when probe was executed
    pub timestamp: Timestamp,
    /// Duration of probe execution
    pub duration: Duration,
    /// Result message or error
    pub message: Option<String>,
    /// Additional details
    pub details: ProbeDetails,
}

/// Additional probe details
#[derive(Debug, Clone)]
pub enum ProbeDetails {
    /// HTTP probe details
    Http {
        /// Status code received
        status_code: Option<u16>,
        /// Response body size
        body_size: usize,
        /// Response time
        response_time: Duration,
    },
    /// TCP probe details
    Tcp {
        /// Connection established
        connected: bool,
        /// Bytes sent
        bytes_sent: usize,
        /// Bytes received
        bytes_received: usize,
    },
    /// Exec probe details
    Exec {
        /// Exit code
        exit_code: Option<i32>,
        /// Stdout size
        stdout_size: usize,
        /// Stderr size
        stderr_size: usize,
    },
}

impl ProbeResult {
    /// Create successful HTTP probe result
    pub fn http_success(duration: Duration, status_code: u16, body_size: usize) -> Self {
        ProbeResult {
            probe_type: ProbeType::Http,
            success: true,
            timestamp: Timestamp::now(),
            duration,
            message: Some(format!("HTTP {} - {} bytes", status_code, body_size)),
            details: ProbeDetails::Http {
                status_code: Some(status_code),
                body_size,
                response_time: duration,
            },
        }
    }

    /// Create failed HTTP probe result
    pub fn http_failure(duration: Duration, message: String) -> Self {
        ProbeResult {
            probe_type: ProbeType::Http,
            success: false,
            timestamp: Timestamp::now(),
            duration,
            message: Some(message),
            details: ProbeDetails::Http {
                status_code: None,
                body_size: 0,
                response_time: duration,
            },
        }
    }

    /// Create successful TCP probe result
    pub fn tcp_success(duration: Duration, bytes_sent: usize, bytes_received: usize) -> Self {
        ProbeResult {
            probe_type: ProbeType::Tcp,
            success: true,
            timestamp: Timestamp::now(),
            duration,
            message: Some(format!("TCP connected - sent {}, recv {}", bytes_sent, bytes_received)),
            details: ProbeDetails::Tcp {
                connected: true,
                bytes_sent,
                bytes_received,
            },
        }
    }

    /// Create failed TCP probe result
    pub fn tcp_failure(duration: Duration, message: String) -> Self {
        ProbeResult {
            probe_type: ProbeType::Tcp,
            success: false,
            timestamp: Timestamp::now(),
            duration,
            message: Some(message),
            details: ProbeDetails::Tcp {
                connected: false,
                bytes_sent: 0,
                bytes_received: 0,
            },
        }
    }

    /// Create successful exec probe result
    pub fn exec_success(duration: Duration, exit_code: i32, stdout_size: usize) -> Self {
        ProbeResult {
            probe_type: ProbeType::Exec,
            success: true,
            timestamp: Timestamp::now(),
            duration,
            message: Some(format!("Exec success - exit code {}", exit_code)),
            details: ProbeDetails::Exec {
                exit_code: Some(exit_code),
                stdout_size,
                stderr_size: 0,
            },
        }
    }

    /// Create failed exec probe result
    pub fn exec_failure(duration: Duration, message: String) -> Self {
        ProbeResult {
            probe_type: ProbeType::Exec,
            success: false,
            timestamp: Timestamp::now(),
            duration,
            message: Some(message),
            details: ProbeDetails::Exec {
                exit_code: None,
                stdout_size: 0,
                stderr_size: 0,
            },
        }
    }
}

/// Probe result cache
#[derive(Debug)]
pub struct ProbeCache {
    /// Cached results
    results: Mutex<VecDeque<ProbeResult>>,
    /// Cache TTL
    ttl: Duration,
    /// Maximum cache size
    max_size: usize,
}

impl ProbeCache {
    /// Create new probe cache
    pub fn new(ttl: Duration, max_size: usize) -> Self {
        ProbeCache {
            results: Mutex::new(VecDeque::with_capacity(max_size)),
            ttl,
            max_size,
        }
    }

    /// Add result to cache
    pub fn add(&self, result: ProbeResult) {
        let mut results = self.results.lock();

        // Remove expired entries
        let now = Timestamp::now();
        while let Some(front) = results.front() {
            if now.duration_since(front.timestamp) > self.ttl {
                results.pop_front();
            } else {
                break;
            }
        }

        // Add new result
        if results.len() == self.max_size {
            results.pop_back();
        }
        results.push_front(result);
    }

    /// Get latest result if still valid
    pub fn get(&self) -> Option<ProbeResult> {
        let results = self.results.lock();
        results.front().cloned().and_then(|result| {
            if Timestamp::now().duration_since(result.timestamp) <= self.ttl {
                Some(result)
            } else {
                None
            }
        })
    }

    /// Clear cache
    pub fn clear(&self) {
        let mut results = self.results.lock();
        results.clear();
    }

    /// Get cache size
    pub fn size(&self) -> usize {
        self.results.lock().len()
    }
}

/// Health probe executor
#[derive(Debug)]
pub struct HealthProbe {
    /// Probe configuration
    config: ProbeConfig,
    /// Check configuration
    check_config: CheckConfig,
    /// Probe result cache
    cache: ProbeCache,
    /// Probe history
    history: Mutex<VecDeque<ProbeResult>>,
    /// Total probe executions
    total_executions: AtomicUsize,
    /// Successful executions
    successful_executions: AtomicUsize,
    /// Failed executions
    failed_executions: AtomicUsize,
}

impl HealthProbe {
    /// Create new health probe
    pub fn new(config: ProbeConfig, check_config: CheckConfig) -> Self {
        let cache_ttl = check_config.interval;
        let cache = ProbeCache::new(cache_ttl, 10);

        HealthProbe {
            config,
            check_config,
            cache,
            history: Mutex::new(VecDeque::with_capacity(100)),
            total_executions: AtomicUsize::new(0),
            successful_executions: AtomicUsize::new(0),
            failed_executions: AtomicUsize::new(0),
        }
    }

    /// Get probe configuration
    pub fn config(&self) -> &ProbeConfig {
        &self.config
    }

    /// Get check configuration
    pub fn check_config(&self) -> &CheckConfig {
        &self.check_config
    }

    /// Execute the probe
    pub fn execute(&self) -> ProbeResult {
        self.total_executions.fetch_add(1, Ordering::Relaxed);

        let start = Timestamp::now();
        let result = match &self.config {
            ProbeConfig::Http(config) => self.execute_http_probe(config),
            ProbeConfig::Tcp(config) => self.execute_tcp_probe(config),
            ProbeConfig::Exec(config) => self.execute_exec_probe(config),
        };

        let _duration = Timestamp::now().duration_since(start);

        // Update statistics
        if result.success {
            self.successful_executions.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_executions.fetch_add(1, Ordering::Relaxed);
        }

        // Update cache and history
        self.cache.add(result.clone());
        let mut history = self.history.lock();
        if history.len() == 100 {
            history.pop_back();
        }
        history.push_front(result.clone());

        result
    }

    /// Execute HTTP probe (mock implementation)
    fn execute_http_probe(&self, _config: &HttpProbeConfig) -> ProbeResult {
        // In a real implementation, this would make an HTTP request
        // For now, we return a mock result
        let duration = Duration::from_millis(50);

        // Simulate successful probe
        ProbeResult::http_success(duration, 200, 1024)
    }

    /// Execute TCP probe (mock implementation)
    fn execute_tcp_probe(&self, _config: &TcpProbeConfig) -> ProbeResult {
        // In a real implementation, this would establish a TCP connection
        // For now, we return a mock result
        let duration = Duration::from_millis(20);

        // Simulate successful probe
        ProbeResult::tcp_success(duration, 0, 0)
    }

    /// Execute exec probe (mock implementation)
    fn execute_exec_probe(&self, _config: &ExecProbeConfig) -> ProbeResult {
        // In a real implementation, this would execute the command
        // For now, we return a mock result
        let duration = Duration::from_millis(100);

        // Simulate successful probe
        ProbeResult::exec_success(duration, 0, 512)
    }

    /// Get cached result if still valid
    pub fn get_cached(&self) -> Option<ProbeResult> {
        self.cache.get()
    }

    /// Get probe history
    pub fn history(&self) -> Vec<ProbeResult> {
        let history = self.history.lock();
        history.iter().cloned().collect()
    }

    /// Get recent probe results
    pub fn recent_results(&self, count: usize) -> Vec<ProbeResult> {
        let history = self.history.lock();
        history.iter().take(count).cloned().collect()
    }

    /// Get total executions
    pub fn total_executions(&self) -> usize {
        self.total_executions.load(Ordering::Relaxed)
    }

    /// Get successful executions
    pub fn successful_executions(&self) -> usize {
        self.successful_executions.load(Ordering::Relaxed)
    }

    /// Get failed executions
    pub fn failed_executions(&self) -> usize {
        self.failed_executions.load(Ordering::Relaxed)
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.total_executions();
        if total == 0 {
            return 0.0;
        }
        let successful = self.successful_executions();
        (successful as f64) / (total as f64)
    }

    /// Clear history
    pub fn clear_history(&self) {
        let mut history = self.history.lock();
        history.clear();
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_type_names() {
        assert_eq!(ProbeType::Http.name(), "http");
        assert_eq!(ProbeType::Tcp.name(), "tcp");
        assert_eq!(ProbeType::Exec.name(), "exec");
    }

    #[test]
    fn test_http_probe_config() {
        let config = HttpProbeConfig::new("http://localhost:8080/health".to_string())
            .with_expected_status(200)
            .with_method(HttpMethod::POST)
            .with_header("Content-Type".to_string(), "application/json".to_string());

        assert_eq!(config.url, "http://localhost:8080/health");
        assert_eq!(config.expected_status, 200);
        assert_eq!(config.method, HttpMethod::POST);
        assert_eq!(config.headers.len(), 1);
    }

    #[test]
    fn test_tcp_probe_config() {
        let config = TcpProbeConfig::new("localhost".to_string(), 9090)
            .with_timeout(Duration::from_secs(10))
            .with_send_data(b"PING".to_vec());

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 9090);
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert!(config.send_data.is_some());
    }

    #[test]
    fn test_exec_probe_config() {
        let config = ExecProbeConfig::new("/bin/true".to_string())
            .with_arg("--check".to_string())
            .with_timeout(Duration::from_secs(5));

        assert_eq!(config.command, "/bin/true");
        assert_eq!(config.args.len(), 1);
        assert_eq!(config.timeout, Duration::from_secs(5));
    }

    #[test]
    fn test_probe_result_http() {
        let result = ProbeResult::http_success(Duration::from_millis(50), 200, 1024);

        assert!(result.success);
        assert_eq!(result.probe_type, ProbeType::Http);
        assert_eq!(result.duration, Duration::from_millis(50));
    }

    #[test]
    fn test_probe_result_tcp() {
        let result = ProbeResult::tcp_success(Duration::from_millis(20), 10, 15);

        assert!(result.success);
        assert_eq!(result.probe_type, ProbeType::Tcp);
        assert_eq!(result.duration, Duration::from_millis(20));
    }

    #[test]
    fn test_probe_result_exec() {
        let result = ProbeResult::exec_success(Duration::from_millis(100), 0, 512);

        assert!(result.success);
        assert_eq!(result.probe_type, ProbeType::Exec);
        assert_eq!(result.duration, Duration::from_millis(100));
    }

    #[test]
    fn test_probe_cache() {
        let cache = ProbeCache::new(Duration::from_secs(5), 3);

        let result = ProbeResult::http_success(Duration::from_millis(50), 200, 1024);
        cache.add(result.clone());

        assert_eq!(cache.size(), 1);
        assert!(cache.get().is_some());

        cache.clear();
        assert_eq!(cache.size(), 0);
        assert!(cache.get().is_none());
    }

    #[test]
    fn test_health_probe_http() {
        let http_config = HttpProbeConfig::new("http://localhost/health".to_string());
        let probe_config = ProbeConfig::Http(http_config);
        let check_config = CheckConfig::default();

        let probe = HealthProbe::new(probe_config, check_config);

        let result = probe.execute();

        assert_eq!(result.probe_type, ProbeType::Http);
        assert_eq!(probe.total_executions(), 1);
        assert_eq!(probe.successful_executions(), 1);
    }

    #[test]
    fn test_health_probe_tcp() {
        let tcp_config = TcpProbeConfig::new("localhost".to_string(), 8080);
        let probe_config = ProbeConfig::Tcp(tcp_config);
        let check_config = CheckConfig::default();

        let probe = HealthProbe::new(probe_config, check_config);

        let result = probe.execute();

        assert_eq!(result.probe_type, ProbeType::Tcp);
        assert_eq!(probe.total_executions(), 1);
    }

    #[test]
    fn test_health_probe_exec() {
        let exec_config = ExecProbeConfig::new("/bin/true".to_string());
        let probe_config = ProbeConfig::Exec(exec_config);
        let check_config = CheckConfig::default();

        let probe = HealthProbe::new(probe_config, check_config);

        let result = probe.execute();

        assert_eq!(result.probe_type, ProbeType::Exec);
        assert_eq!(probe.total_executions(), 1);
    }

    #[test]
    fn test_health_probe_success_rate() {
        let http_config = HttpProbeConfig::new("http://localhost/health".to_string());
        let probe_config = ProbeConfig::Http(http_config);
        let check_config = CheckConfig::default();

        let probe = HealthProbe::new(probe_config, check_config);

        // All probes succeed in mock implementation
        for _ in 0..10 {
            probe.execute();
        }

        assert_eq!(probe.total_executions(), 10);
        assert_eq!(probe.successful_executions(), 10);
        assert_eq!(probe.failed_executions(), 0);
        assert_eq!(probe.success_rate(), 1.0);
    }

    #[test]
    fn test_health_probe_history() {
        let http_config = HttpProbeConfig::new("http://localhost/health".to_string());
        let probe_config = ProbeConfig::Http(http_config);
        let check_config = CheckConfig::default();

        let probe = HealthProbe::new(probe_config, check_config);

        probe.execute();
        probe.execute();

        let history = probe.history();
        assert_eq!(history.len(), 2);

        let recent = probe.recent_results(1);
        assert_eq!(recent.len(), 1);
    }
}
