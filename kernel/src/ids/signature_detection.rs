/// Signature Detection Module for IDS

extern crate alloc;
///
/// This module implements signature-based intrusion detection using
/// pattern matching and rule-based detection.

use crate::subsystems::sync::{SpinLock, Mutex};
use crate::collections::HashMap;
use crate::compat::DefaultHasherBuilder;
use crate::subsystems::time::{SystemTime, UNIX_EPOCH};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};

/// Detection severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// Signature type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignatureType {
    /// Network packet patterns
    Network,
    /// System call sequences
    SystemCall,
    /// File patterns
    File,
    /// Process behavior
    Process,
    /// Registry patterns
    Registry,
    /// Memory patterns
    Memory,
    /// Web application attacks
    Web,
    /// Malware signatures
    Malware,
}

/// Match pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    /// Exact string match
    Exact,
    /// Regular expression
    Regex,
    /// Byte pattern
    Bytes,
    /// Wildcard pattern
    Wildcard,
    /// Numeric range
    Range,
    /// Set of values
    Set,
}

/// Detection signature
#[derive(Debug, Clone)]
pub struct Signature {
    /// Unique signature identifier
    pub id: u64,
    /// Signature name
    pub name: String,
    /// Signature description
    pub description: String,
    /// Signature type
    pub signature_type: SignatureType,
    /// Pattern to match
    pub pattern: String,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Detection severity
    pub severity: Severity,
    /// Associated threat category
    pub category: String,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
    /// Creation timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
    /// Is this signature active
    pub active: bool,
    /// Reference information
    pub references: Vec<String>,
    /// Context requirements
    pub context: HashMap<String, String>,
}

/// Detection event
#[derive(Debug, Clone)]
pub struct DetectionEvent {
    /// Event ID
    pub id: u64,
    /// Signature that triggered
    pub signature_id: u64,
    /// Event timestamp
    pub timestamp: u64,
    /// Source information
    pub source: String,
    /// Matched pattern
    pub matched_content: Vec<u8>,
    /// Event details
    pub details: HashMap<String, String>,
    /// Process information
    pub process_info: Option<ProcessInfo>,
    /// Network information
    pub network_info: Option<NetworkInfo>,
    /// File information
    pub file_info: Option<FileInfo>,
}

/// Process information for detection context
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    /// Process ID
    pub pid: u32,
    /// Parent Process ID
    pub ppid: u32,
    /// Process name
    pub name: String,
    /// Process path
    pub path: String,
    /// Command line arguments
    pub args: Vec<String>,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
}

/// Network information for detection context
#[derive(Debug, Clone)]
pub struct NetworkInfo {
    /// Source IP
    pub src_ip: String,
    /// Source port
    pub src_port: u16,
    /// Destination IP
    pub dst_ip: String,
    /// Destination port
    pub dst_port: u16,
    /// Protocol
    pub protocol: String,
    /// Packet size
    pub packet_size: usize,
}

/// File information for detection context
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// File path
    pub path: String,
    /// File name
    pub name: String,
    /// File size
    pub size: u64,
    /// File permissions
    pub permissions: u32,
    /// File hash (SHA-256)
    pub hash: String,
}

/// Signature detection engine
pub struct SignatureEngine {
    /// Active signatures
    signatures: HashMap<u64, Signature>,
    /// Signatures by type
    signatures_by_type: HashMap<SignatureType, Vec<u64>>,
    /// Detection events
    events: Vec<DetectionEvent>,
    /// Event counter
    event_counter: AtomicU64,
    /// Signature counter
    signature_counter: AtomicU64,
    /// Is the engine running
    running: AtomicBool,
    /// Statistics
    stats: SignatureStats,
    /// Engine lock
    engine_lock: SpinLock,
}

impl SignatureEngine {
    /// Create a new signature engine
    pub fn new() -> Self {
        Self {
            signatures: HashMap::with_hasher(DefaultHasherBuilder),
            signatures_by_type: HashMap::with_hasher(DefaultHasherBuilder),
            events: Vec::new(),
            event_counter: AtomicU64::new(0),
            signature_counter: AtomicU64::new(0),
            running: AtomicBool::new(false),
            stats: SignatureStats::new(),
            engine_lock: SpinLock::new(),
        }
    }

    /// Initialize signature engine with config
    pub fn init(&mut self, config: &crate::ids::SignatureDetectionConfig) -> Result<(), &'static str> {
        if !config.enabled {
            self.stop();
            return Ok(());
        }

        // Load signatures from configured database if needed (stub)
        self.start();
        Ok(())
    }

    /// Clean shutdown
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        self.stop();
        Ok(())
    }

    /// Start the signature engine
    pub fn start(&mut self) {
        let _lock = self.engine_lock.lock();
        self.running.store(true, Ordering::Relaxed);
        self.load_default_signatures();
    }

    /// Stop the signature engine
    pub fn stop(&mut self) {
        let _lock = self.engine_lock.lock();
        self.running.store(false, Ordering::Relaxed);
    }

    /// Add a new signature
    pub fn add_signature(&mut self, signature: Signature) {
        let _lock = self.engine_lock.lock();

        if !self.signatures.contains_key(&signature.id) {
            self.signature_counter.fetch_add(1, Ordering::Relaxed);
        }

        self.signatures.insert(signature.id, signature.clone());

        // Update type index
        let type_signatures = self.signatures_by_type
            .entry(signature.signature_type)
            .or_insert_with(Vec::new);

        if !type_signatures.contains(&signature.id) {
            type_signatures.push(signature.id);
        }

        self.stats.total_signatures = self.signatures.len();
    }

    /// Remove a signature
    pub fn remove_signature(&mut self, signature_id: u64) -> bool {
        let _lock = self.engine_lock.lock();

        if let Some(signature) = self.signatures.remove(&signature_id) {
            // Update type index
            if let Some(type_signatures) = self.signatures_by_type.get_mut(&signature.signature_type) {
                type_signatures.retain(|&id| id != signature_id);
            }

            self.stats.total_signatures = self.signatures.len();
            true
        } else {
            false
        }
    }

    /// Scan data for signature matches
    pub fn scan_data(&mut self, data: &[u8], signature_type: SignatureType, context: &HashMap<String, String>) -> Vec<DetectionEvent> {
        let _lock = self.engine_lock.lock();

        if !self.running.load(Ordering::Relaxed) {
            return Vec::new();
        }

        let mut events = Vec::new();

        let signature_ids_to_check: Vec<_> = self.signatures_by_type.get(&signature_type)
            .map(|ids| ids.iter().copied().collect::<Vec<_>>())
            .unwrap_or_default();

        for signature_id in signature_ids_to_check {
            if let Some(signature) = self.signatures.get(&signature_id) {
                if !signature.active {
                    continue;
                }

                if self.matches_signature(data, signature, context) {
                    let event = self.create_detection_event(signature_id, data, context);
                    events.push(event.clone());
                    self.add_event(event);
                    self.stats.detections += 1;
                }
            }
        }

        events
    }

    /// Scan network packet
    pub fn scan_packet(&mut self, packet: &[u8], packet_info: &NetworkInfo) -> Vec<DetectionEvent> {
        let mut context = HashMap::with_hasher(DefaultHasherBuilder);
        context.insert(String::from("src_ip"), packet_info.src_ip.clone());
        context.insert(String::from("dst_ip"), packet_info.dst_ip.clone());
        context.insert(String::from("src_port"), format!("{}", packet_info.src_port));
        context.insert(String::from("dst_port"), format!("{}", packet_info.dst_port));
        context.insert(String::from("protocol"), packet_info.protocol.clone());

        self.scan_data(packet, SignatureType::Network, &context)
    }

    /// Scan file
    pub fn scan_file(&mut self, file_data: &[u8], file_info: &FileInfo) -> Vec<DetectionEvent> {
        let mut context = HashMap::with_hasher(DefaultHasherBuilder);
        context.insert(String::from("file_path"), file_info.path.clone());
        context.insert(String::from("file_name"), file_info.name.clone());
        context.insert(String::from("file_size"), format!("{}", file_info.size));
        context.insert(String::from("file_hash"), file_info.hash.clone());

        self.scan_data(file_data, SignatureType::File, &context)
    }

    /// Get recent detection events
    pub fn get_recent_events(&self, count: usize) -> Vec<DetectionEvent> {
        self.events.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get signature by ID
    pub fn get_signature(&self, signature_id: u64) -> Option<&Signature> {
        self.signatures.get(&signature_id)
    }

    /// Get all signatures of a specific type
    pub fn get_signatures_by_type(&self, signature_type: SignatureType) -> Vec<&Signature> {
        if let Some(signature_ids) = self.signatures_by_type.get(&signature_type) {
            signature_ids.iter()
                .filter_map(|&id| self.signatures.get(&id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Update signature
    pub fn update_signature(&mut self, signature_id: u64, updated_signature: Signature) -> bool {
        let _lock = self.engine_lock.lock();

        if self.signatures.contains_key(&signature_id) {
            self.signatures.insert(signature_id, updated_signature);
            true
        } else {
            false
        }
    }

    /// Get statistics
    pub fn get_statistics(&self) -> &SignatureStats {
        &self.stats
    }

    /// Clear old events
    pub fn clear_old_events(&mut self, max_age_seconds: u64) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.events.retain(|event| {
            current_time - event.timestamp <= max_age_seconds
        });
    }

    /// Check if data matches a signature
    fn matches_signature(&self, data: &[u8], signature: &Signature, context: &HashMap<String, String>) -> bool {
        match signature.pattern_type {
            PatternType::Exact => {
                let pattern_bytes = signature.pattern.as_bytes();
                data.windows(pattern_bytes.len()).any(|window| window == pattern_bytes)
            }
            PatternType::Bytes => {
                self.match_bytes_pattern(data, &signature.pattern)
            }
            PatternType::Wildcard => {
                self.match_wildcard_pattern(data, &signature.pattern)
            }
            PatternType::Set => {
                self.match_set_pattern(data, &signature.pattern)
            }
            _ => {
                // For other pattern types, fallback to subsequence search using windows
                let pat = signature.pattern.as_bytes();
                if pat.is_empty() { return false; }
                data.windows(pat.len()).any(|w| w == pat)
            }
        }
    }

    /// Match byte pattern (hex format like "48 8B 05 ?? ?? ?? ??")
    fn match_bytes_pattern(&self, data: &[u8], pattern: &str) -> bool {
        let pattern_bytes: Vec<Option<u8>> = pattern
            .split_whitespace()
            .map(|part| {
                if part == "??" {
                    None
                } else {
                    u8::from_str_radix(part, 16).ok()
                }
            })
            .collect();

        if pattern_bytes.is_empty() {
            return false;
        }

        data.windows(pattern_bytes.len()).any(|window| {
            window.iter().zip(pattern_bytes.iter()).all(|(&data_byte, &pattern_byte)| {
                match pattern_byte {
                    Some(expected) => data_byte == expected,
                    None => true, // Wildcard matches any byte
                }
            })
        })
    }

    /// Match wildcard pattern
    fn match_wildcard_pattern(&self, data: &[u8], pattern: &str) -> bool {
        // Simple wildcard matching with * and ?
        let pattern_bytes = pattern.as_bytes();
        let mut data_idx = 0;
        let mut pattern_idx = 0;

        while data_idx < data.len() && pattern_idx < pattern_bytes.len() {
            match pattern_bytes[pattern_idx] {
                b'*' => {
                    // Skip to next pattern character
                    pattern_idx += 1;
                    if pattern_idx == pattern_bytes.len() {
                        return true; // * at end matches everything
                    }
                    // Find next occurrence of pattern character
                    while data_idx < data.len() && data[data_idx] != pattern_bytes[pattern_idx] {
                        data_idx += 1;
                    }
                }
                b'?' => {
                    // Match any single character
                    data_idx += 1;
                    pattern_idx += 1;
                }
                pattern_char => {
                    if data[data_idx] == pattern_char {
                        data_idx += 1;
                        pattern_idx += 1;
                    } else {
                        return false;
                    }
                }
            }
        }

        data_idx == data.len() && pattern_idx == pattern_bytes.len()
    }

    /// Match set pattern
    fn match_set_pattern(&self, data: &[u8], pattern: &str) -> bool {
        // Parse comma-separated values and check if data contains any
        let values: Vec<String> = pattern
            .split(',')
            .map(|s| String::from(s.trim()))
            .collect();

        for value in values {
            let pat = value.as_bytes();
            if pat.is_empty() { continue; }
            if data.windows(pat.len()).any(|w| w == pat) {
                return true;
            }
        }
        false
    }

    /// Create a detection event
    fn create_detection_event(&self, signature_id: u64, matched_content: &[u8], context: &HashMap<String, String>) -> DetectionEvent {
        let id = self.event_counter.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        DetectionEvent {
            id,
            signature_id,
            timestamp,
            source: String::from("SignatureEngine"),
            matched_content: matched_content.to_vec(),
            details: context.clone(),
            process_info: None,
            network_info: None,
            file_info: None,
        }
    }

    /// Add event to storage
    fn add_event(&mut self, event: DetectionEvent) {
        if self.events.len() >= 10000 {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    /// Load default signatures
    fn load_default_signatures(&mut self) {
        // Network attack signatures
        self.add_signature(Signature {
            id: self.signature_counter.fetch_add(1, Ordering::Relaxed) + 1,
            name: String::from("Port Scan Detection"),
            description: String::from("Detects potential port scanning activity"),
            signature_type: SignatureType::Network,
            pattern: String::from("SYN"),
            pattern_type: PatternType::Exact,
            severity: Severity::Medium,
            category: String::from("Reconnaissance"),
            confidence: 0.7,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            active: true,
            references: vec![String::from("CVE-2023-XXXX")],
            context: HashMap::with_hasher(DefaultHasherBuilder),
        });

        // Malware signatures
        self.add_signature(Signature {
            id: self.signature_counter.fetch_add(1, Ordering::Relaxed) + 1,
            name: String::from("EICAR Test File"),
            description: String::from("Standard antivirus test file signature"),
            signature_type: SignatureType::Malware,
            pattern: String::from("X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-ANTIVIRUS-TEST-FILE!$H+H*"),
            pattern_type: PatternType::Exact,
            severity: Severity::Info,
            category: String::from("Test"),
            confidence: 1.0,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            active: true,
            references: Vec::new(),
            context: HashMap::with_hasher(DefaultHasherBuilder),
        });

        // Web attack signatures
        self.add_signature(Signature {
            id: self.signature_counter.fetch_add(1, Ordering::Relaxed) + 1,
            name: String::from("SQL Injection Attempt"),
            description: String::from("Detects potential SQL injection attacks"),
            signature_type: SignatureType::Web,
            pattern: String::from("' OR '1'='1"),
            pattern_type: PatternType::Exact,
            severity: Severity::High,
            category: String::from("Injection"),
            confidence: 0.9,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            active: true,
            references: vec![String::from("OWASP Top 10")],
            context: HashMap::with_hasher(DefaultHasherBuilder),
        });
    }
}

/// Signature engine statistics
#[derive(Debug, Clone)]
pub struct SignatureStats {
    /// Total signatures loaded
    pub total_signatures: usize,
    /// Active signatures
    pub active_signatures: usize,
    /// Total detections made
    pub detections: u64,
    /// Events in storage
    pub events_stored: usize,
    /// Engine uptime in seconds
    pub uptime: u64,
}

impl SignatureStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            total_signatures: 0,
            active_signatures: 0,
            detections: 0,
            events_stored: 0,
            uptime: 0,
        }
    }
}

impl Default for SignatureStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a default signature engine
pub fn create_signature_engine() -> Arc<Mutex<SignatureEngine>> {
    Arc::new(Mutex::new(SignatureEngine::new()))
}

/// Export signature rules to file
pub fn export_signatures(signatures: &[Signature]) -> String {
    let mut output = String::from("Signature Rules Export\n");
    output.push_str("========================\n\n");

    for signature in signatures {
        output.push_str(&format!(
            "ID: {}\nName: {}\nType: {:?}\nSeverity: {:?}\nPattern: {}\nDescription: {}\nActive: {}\n\n",
            signature.id,
            signature.name,
            signature.signature_type,
            signature.severity,
            signature.pattern,
            signature.description,
            signature.active
        ));
    }

    output
}