/// Correlation Engine Module for IDS

extern crate alloc;
///
/// This module implements advanced correlation analysis to connect
/// related security events and identify attack patterns.

use crate::sync::{SpinLock, Mutex};
use crate::collections::{HashMap, HashSet};
use crate::compat::DefaultHasherBuilder;
use crate::time::{SystemTime, UNIX_EPOCH};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

/// Correlation confidence level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorrelationConfidence {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Attack pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttackPattern {
    /// Port scanning
    PortScanning,
    /// Brute force attack
    BruteForce,
    /// Distributed denial of service
    DDoS,
    /// Command injection
    CommandInjection,
    /// SQL injection
    SQLInjection,
    /// Cross-site scripting
    XSS,
    /// Privilege escalation
    PrivilegeEscalation,
    /// Data exfiltration
    DataExfiltration,
    /// Lateral movement
    LateralMovement,
    /// Persistence mechanisms
    Persistence,
    /// Reconnaissance
    Reconnaissance,
    /// Unknown pattern
    Unknown,
}

/// Event correlation
#[derive(Debug, Clone)]
pub struct EventCorrelation {
    /// Correlation ID
    pub id: u64,
    /// Correlation timestamp
    pub timestamp: u64,
    /// Attack pattern type
    pub pattern: AttackPattern,
    /// Correlated event IDs
    pub event_ids: Vec<u64>,
    /// Correlation confidence
    pub confidence: CorrelationConfidence,
    /// Attack severity
    pub severity: u8, // 0-255
    /// Source actors or IP addresses
    pub sources: Vec<String>,
    /// Target systems
    pub targets: Vec<String>,
    /// Timeline of events
    pub timeline: Vec<(u64, String)>, // (timestamp, event_type)
    /// Attack stage
    pub stage: String,
    /// Attack description
    pub description: String,
    /// Related MITRE ATT&CK techniques
    pub mitre_techniques: Vec<String>,
    /// Indicators of compromise
    pub indicators: Vec<String>,
    /// Recommended actions
    pub recommended_actions: Vec<String>,
}

/// Correlation rule
#[derive(Debug, Clone)]
pub struct CorrelationRule {
    /// Rule ID
    pub id: u64,
    /// Rule name
    pub name: String,
    /// Attack pattern this rule detects
    pub pattern: AttackPattern,
    /// Event conditions (simplified)
    pub event_conditions: Vec<String>,
    /// Time window for correlation in seconds
    pub time_window: u64,
    /// Minimum event count
    pub min_events: usize,
    /// Maximum time span between first and last event
    pub max_time_span: u64,
    /// Required event types
    pub required_event_types: Vec<String>,
    /// Event source patterns
    pub source_patterns: Vec<String>,
    /// Target patterns
    pub target_patterns: Vec<String>,
    /// Correlation confidence
    pub confidence: CorrelationConfidence,
    /// Is this rule active
    pub active: bool,
    /// Rule description
    pub description: String,
}

/// Event cluster for correlation
#[derive(Debug, Clone)]
pub struct EventCluster {
    /// Cluster ID
    pub id: u64,
    /// Cluster center (simplified representation)
    pub center: String,
    /// Event IDs in cluster
    pub event_ids: Vec<u64>,
    /// Cluster timestamp
    pub timestamp: u64,
    /// Cluster type
    pub cluster_type: String,
    /// Cluster confidence
    pub confidence: f32,
    /// Cluster size
    pub size: usize,
}

/// Timeline event
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    /// Event ID
    pub id: u64,
    /// Event timestamp
    pub timestamp: u64,
    /// Event type
    pub event_type: String,
    /// Event source
    pub source: String,
    /// Event target
    pub target: String,
    /// Event severity
    pub severity: u8,
    /// Event details
    pub details: HashMap<String, String>,
}

/// Correlation engine
pub struct CorrelationEngine {
    /// Correlation rules
    rules: HashMap<u64, CorrelationRule>,
    /// Event buffer for correlation
    event_buffer: Vec<TimelineEvent>,
    /// Active correlations
    correlations: Vec<EventCorrelation>,
    /// Event clusters
    clusters: Vec<EventCluster>,
    /// Correlation statistics
    stats: CorrelationStats,
    /// Correlation counter
    correlation_counter: AtomicU64,
    /// Rule counter
    rule_counter: AtomicU64,
    /// Cluster counter
    cluster_counter: AtomicU64,
    /// Engine lock
    engine_lock: SpinLock,
}

impl CorrelationEngine {
    /// Create a new correlation engine
    pub fn new() -> Self {
        Self {
            rules: HashMap::with_hasher(DefaultHasherBuilder),
            event_buffer: Vec::new(),
            correlations: Vec::new(),
            clusters: Vec::new(),
            stats: CorrelationStats::new(),
            correlation_counter: AtomicU64::new(0),
            rule_counter: AtomicU64::new(0),
            cluster_counter: AtomicU64::new(0),
            engine_lock: SpinLock::new(),
        }
    }

    /// Initialize correlation engine with config
    pub fn init(&mut self, _config: &crate::ids::CorrelationConfig) -> Result<(), &'static str> {
        // No-op for now
        Ok(())
    }

    /// Analyze a set of detections and produce correlation results
    pub fn analyze_correlations(&mut self, _detections: &alloc::vec::Vec<crate::ids::IntrusionDetection>) -> Result<alloc::vec::Vec<crate::ids::CorrelationResult>, &'static str> {
        // Simple stub: no correlation logic yet
        Ok(Vec::new())
    }

    /// Shutdown the correlation engine
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        self.rules.clear();
        self.event_buffer.clear();
        self.correlations.clear();
        Ok(())
    }

    /// Add a timeline event for correlation
    pub fn add_event(&mut self, event: TimelineEvent) {
        let _lock = self.engine_lock.lock();

        // Add to buffer
        self.event_buffer.push(event.clone());

        // Limit buffer size
        if self.event_buffer.len() > 10000 {
            self.event_buffer.remove(0);
        }

        // Perform correlation
        self.perform_correlation();

        self.stats.total_events += 1;
    }

    /// Add a correlation rule
    pub fn add_rule(&mut self, rule: CorrelationRule) {
        let _lock = self.engine_lock.lock();

        if !self.rules.contains_key(&rule.id) {
            self.rule_counter.fetch_add(1, Ordering::Relaxed);
        }

        self.rules.insert(rule.id, rule);
        self.stats.total_rules = self.rules.len();
    }

    /// Get recent correlations
    pub fn get_recent_correlations(&self, count: usize) -> Vec<EventCorrelation> {
        self.correlations.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get attack pattern statistics
    pub fn get_pattern_statistics(&self) -> HashMap<AttackPattern, usize> {
        let mut pattern_stats = HashMap::with_hasher(DefaultHasherBuilder);

        for correlation in &self.correlations {
            let count = pattern_stats.entry(correlation.pattern).or_insert(0);
            *count += 1;
        }

        pattern_stats
    }

    /// Analyze attack timeline
    pub fn analyze_attack_timeline(&self, correlation_id: u64) -> Option<Vec<(u64, String)>> {
        if let Some(correlation) = self.correlations.iter().find(|c| c.id == correlation_id) {
            Some(correlation.timeline.clone())
        } else {
            None
        }
    }

    /// Get MITRE ATT&CK mapping
    pub fn get_mitre_mapping(&self) -> HashMap<AttackPattern, Vec<String>> {
        let mut mapping = HashMap::with_hasher(DefaultHasherBuilder);

        mapping.insert(AttackPattern::PortScanning, vec![
            String::from("T1595.002: Active Scanning: Scanning for Open Ports"),
            String::from("T1046: Network Service Scanning"),
        ]);

        mapping.insert(AttackPattern::BruteForce, vec![
            String::from("T1110.001: Brute Force: Password Guessing"),
            String::from("T1110.002: Brute Force: Password Cracking"),
        ]);

        mapping.insert(AttackPattern::PrivilegeEscalation, vec![
            String::from("T1068: Exploitation for Privilege Escalation"),
            String::from("T1548.003: Abuse Elevation Control Mechanism: Sudo and Sudo Caching"),
        ]);

        mapping.insert(AttackPattern::DataExfiltration, vec![
            String::from("T1041: Exfiltration Over C2 Channel"),
            String::from("T1567.001: Exfiltration Over Web Service: Exfiltration to Cloud Storage"),
        ]);

        mapping.insert(AttackPattern::LateralMovement, vec![
            String::from("T1021.002: Remote Services: SMB/Windows Admin Shares"),
            String::from("T1021.004: Remote Services: SSH"),
        ]);

        mapping
    }

    /// Get correlation statistics
    pub fn get_statistics(&self) -> &CorrelationStats {
        &self.stats
    }

    /// Clear old events and correlations
    pub fn clear_old_data(&mut self, max_age_seconds: u64) {
        let _lock = self.engine_lock.lock();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Clear old events
        self.event_buffer.retain(|event| {
            current_time - event.timestamp <= max_age_seconds
        });

        // Clear old correlations
        self.correlations.retain(|correlation| {
            current_time - correlation.timestamp <= max_age_seconds
        });

        // Clear old clusters
        self.clusters.retain(|cluster| {
            current_time - cluster.timestamp <= max_age_seconds
        });
    }

    /// Perform correlation analysis
    fn perform_correlation(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Apply correlation rules - collect to avoid borrowing conflict
        let active_rules: Vec<_> = self.rules.values().filter(|r| r.active).cloned().collect();
        for rule in active_rules {
            if let Some(correlation) = self.apply_correlation_rule(&rule) {
                self.correlations.push(correlation);
            }
        }

        // Perform event clustering
        self.perform_event_clustering();

        // Detect attack patterns using statistical analysis
        self.detect_attack_patterns();

        self.stats.correlations_found = self.correlations.len();
    }

    /// Apply a correlation rule to events
    fn apply_correlation_rule(&mut self, rule: &CorrelationRule) -> Option<EventCorrelation> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Find events within time window
        let recent_events: Vec<&TimelineEvent> = self.event_buffer.iter()
            .filter(|event| {
                current_time - event.timestamp <= rule.time_window
            })
            .collect();

        if recent_events.len() < rule.min_events {
            return None;
        }

        // Check for rule conditions
        let matching_events: Vec<&TimelineEvent> = recent_events.iter()
            .filter(|event| {
                self.matches_rule_conditions(event, rule)
            })
            .cloned()
            .collect();

        if matching_events.len() < rule.min_events {
            return None;
        }

        // Check time span
        if !matching_events.is_empty() {
            let min_time = matching_events.iter().map(|e| e.timestamp).min().unwrap();
            let max_time = matching_events.iter().map(|e| e.timestamp).max().unwrap();

            if max_time - min_time > rule.max_time_span {
                return None;
            }
        }

        // Create correlation
        let correlation_id = self.correlation_counter.fetch_add(1, Ordering::Relaxed);
        let event_ids: Vec<u64> = matching_events.iter().map(|e| e.id).collect();
        let sources: Vec<String> = matching_events.iter()
            .map(|e| e.source.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let targets: Vec<String> = matching_events.iter()
            .map(|e| e.target.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let timeline: Vec<(u64, String)> = matching_events.iter()
            .map(|e| (e.timestamp, e.event_type.clone()))
            .collect();

        Some(EventCorrelation {
            id: correlation_id,
            timestamp: current_time,
            pattern: rule.pattern,
            event_ids,
            confidence: rule.confidence,
            severity: self.calculate_correlation_severity(&matching_events),
            sources,
            targets,
            timeline,
            stage: String::from("Unknown"),
            description: rule.description.clone(),
            mitre_techniques: self.get_mitre_techniques_for_pattern(rule.pattern),
            indicators: self.extract_indicators(&matching_events),
            recommended_actions: self.get_recommended_actions(rule.pattern),
        })
    }

    /// Check if event matches rule conditions
    fn matches_rule_conditions(&self, event: &TimelineEvent, rule: &CorrelationRule) -> bool {
        // Check required event types
        if !rule.required_event_types.is_empty() &&
           !rule.required_event_types.contains(&event.event_type) {
            return false;
        }

        // Check source patterns
        if !rule.source_patterns.is_empty() {
            let source_matches = rule.source_patterns.iter()
                .any(|pattern| event.source.contains(pattern));
            if !source_matches {
                return false;
            }
        }

        // Check target patterns
        if !rule.target_patterns.is_empty() {
            let target_matches = rule.target_patterns.iter()
                .any(|pattern| event.target.contains(pattern));
            if !target_matches {
                return false;
            }
        }

        // Check event conditions (simplified)
        for condition in &rule.event_conditions {
            if !self.evaluate_event_condition(condition, event) {
                return false;
            }
        }

        true
    }

    /// Evaluate event condition
    fn evaluate_event_condition(&self, condition: &str, event: &TimelineEvent) -> bool {
        if condition.contains("severity:high") {
            return event.severity >= 150;
        }

        if condition.contains("severity:critical") {
            return event.severity >= 200;
        }

        if condition.starts_with("source:") {
            let pattern = condition.trim_start_matches("source:");
            return event.source.contains(pattern);
        }

        if condition.starts_with("target:") {
            let pattern = condition.trim_start_matches("target:");
            return event.target.contains(pattern);
        }

        true // Default to true for unknown conditions
    }

    /// Perform event clustering
    fn perform_event_clustering(&mut self) {
        // Simple clustering based on source and event type
        let mut clusters: HashMap<String, Vec<usize>> = HashMap::with_hasher(DefaultHasherBuilder);

        for (index, event) in self.event_buffer.iter().enumerate() {
            let cluster_key = format!("{}:{}", event.source, event.event_type);
            clusters.entry(cluster_key)
                .or_insert_with(Vec::new)
                .push(index);
        }

        // Create cluster objects
        for (cluster_key, event_indices) in clusters {
            if event_indices.len() >= 3 { // Minimum cluster size
                let cluster_id = self.cluster_counter.fetch_add(1, Ordering::Relaxed);
                let event_ids: Vec<u64> = event_indices.iter()
                    .map(|&index| self.event_buffer[index].id)
                    .collect();

                let cluster = EventCluster {
                    id: cluster_id,
                    center: cluster_key,
                    event_ids,
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    cluster_type: String::from("Source-Based"),
                    confidence: (event_indices.len() as f32 / 10.0).min(1.0),
                    size: event_indices.len(),
                };

                self.clusters.push(cluster);
            }
        }

        // Limit cluster storage
        if self.clusters.len() > 1000 {
            self.clusters.drain(0..self.clusters.len() - 1000);
        }

        self.stats.clusters_found = self.clusters.len();
    }

    /// Detect attack patterns using statistical analysis
    fn detect_attack_patterns(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Detect port scanning
        self.detect_port_scanning(current_time);

        // Detect brute force attacks
        self.detect_brute_force(current_time);

        // Detect DDoS attacks
        self.detect_ddos(current_time);
    }

    /// Detect port scanning patterns
    fn detect_port_scanning(&mut self, current_time: u64) {
        let time_window = 300; // 5 minutes
        let mut source_ports: HashMap<String, Vec<u16>> = HashMap::with_hasher(DefaultHasherBuilder);

        for event in &self.event_buffer {
            if current_time - event.timestamp > time_window {
                continue;
            }

            if event.event_type.contains("network") || event.event_type.contains("port") {
                if let Some(port_str) = event.details.get("dst_port") {
                    if let Ok(port) = port_str.parse::<u16>() {
                        source_ports.entry(event.source.clone())
                            .or_insert_with(Vec::new)
                            .push(port);
                    }
                }
            }
        }

        for (source, ports) in source_ports {
            let unique_ports: HashSet<_> = ports.iter().collect();
            if unique_ports.len() >= 10 { // Threshold for port scanning
                let correlation_id = self.correlation_counter.fetch_add(1, Ordering::Relaxed);

                let correlation = EventCorrelation {
                    id: correlation_id,
                    timestamp: current_time,
                    pattern: AttackPattern::PortScanning,
                    event_ids: Vec::new(), // Would need to track actual event IDs
                    confidence: CorrelationConfidence::High,
                    severity: 120,
                    sources: vec![source.clone()],
                    targets: Vec::new(),
                    timeline: Vec::new(),
                    stage: String::from("Reconnaissance"),
                    description: String::from("Port scanning activity detected"),
                    mitre_techniques: self.get_mitre_techniques_for_pattern(AttackPattern::PortScanning),
                    indicators: vec![format!("Source IP: {}", source)],
                    recommended_actions: vec![
                        String::from("Block source IP"),
                        String::from("Monitor for further activity"),
                    ],
                };

                self.correlations.push(correlation);
            }
        }
    }

    /// Detect brute force attack patterns
    fn detect_brute_force(&mut self, current_time: u64) {
        let time_window = 600; // 10 minutes
        let mut failed_attempts: HashMap<String, usize> = HashMap::with_hasher(DefaultHasherBuilder);

        for event in &self.event_buffer {
            if current_time - event.timestamp > time_window {
                continue;
            }

            if event.event_type.contains("login") || event.event_type.contains("auth") {
                if let Some(success) = event.details.get("success") {
                    if success == "false" {
                        let count = failed_attempts.entry(event.target.clone()).or_insert(0);
                        *count += 1;
                    }
                }
            }
        }

        for (target, count) in failed_attempts {
            if count >= 5 { // Threshold for brute force
                let correlation_id = self.correlation_counter.fetch_add(1, Ordering::Relaxed);

                let correlation = EventCorrelation {
                    id: correlation_id,
                    timestamp: current_time,
                    pattern: AttackPattern::BruteForce,
                    event_ids: Vec::new(),
                    confidence: CorrelationConfidence::High,
                    severity: 150,
                    sources: Vec::new(),
                    targets: vec![target.clone()],
                    timeline: Vec::new(),
                    stage: String::from("Credential Access"),
                    description: String::from("Brute force attack detected"),
                    mitre_techniques: self.get_mitre_techniques_for_pattern(AttackPattern::BruteForce),
                    indicators: vec![format!("Target account: {}", target)],
                    recommended_actions: vec![
                        String::from("Lock account temporarily"),
                        String::from("Increase monitoring"),
                    ],
                };

                self.correlations.push(correlation);
            }
        }
    }

    /// Detect DDoS attack patterns
    fn detect_ddos(&mut self, current_time: u64) {
        let time_window = 60; // 1 minute
        let mut request_count: HashMap<String, usize> = HashMap::with_hasher(DefaultHasherBuilder);

        for event in &self.event_buffer {
            if current_time - event.timestamp > time_window {
                continue;
            }

            if event.event_type.contains("request") || event.event_type.contains("network") {
                let count = request_count.entry(event.target.clone()).or_insert(0);
                *count += 1;
            }
        }

        for (target, count) in request_count {
            if count >= 1000 { // Threshold for DDoS
                let correlation_id = self.correlation_counter.fetch_add(1, Ordering::Relaxed);

                let correlation = EventCorrelation {
                    id: correlation_id,
                    timestamp: current_time,
                    pattern: AttackPattern::DDoS,
                    event_ids: Vec::new(),
                    confidence: CorrelationConfidence::VeryHigh,
                    severity: 200,
                    sources: Vec::new(),
                    targets: vec![target.clone()],
                    timeline: Vec::new(),
                    stage: String::from("Impact"),
                    description: String::from("DDoS attack detected"),
                    mitre_techniques: vec![
                        String::from("T1498: Network Denial of Service"),
                        String::from("T1499: Endpoint Denial of Service"),
                    ],
                    indicators: vec![format!("Target service: {}", target)],
                    recommended_actions: vec![
                        String::from("Activate DDoS mitigation"),
                        String::from("Block suspicious traffic"),
                    ],
                };

                self.correlations.push(correlation);
            }
        }
    }

    /// Calculate correlation severity
    fn calculate_correlation_severity(&self, events: &[&TimelineEvent]) -> u8 {
        if events.is_empty() {
            return 0;
        }

        let total_severity: u32 = events.iter().map(|e| e.severity as u32).sum();
        let avg_severity = total_severity / events.len() as u32;

        // Increase severity based on event count
        let event_multiplier = (events.len() as f32 / 10.0).max(1.0);
        let adjusted_severity = (avg_severity as f32 * event_multiplier) as u32;

        adjusted_severity.min(255) as u8
    }

    /// Get MITRE techniques for attack pattern
    fn get_mitre_techniques_for_pattern(&self, pattern: AttackPattern) -> Vec<String> {
        let mapping = self.get_mitre_mapping();
        mapping.get(&pattern).cloned().unwrap_or_default()
    }

    /// Extract indicators of compromise
    fn extract_indicators(&self, events: &[&TimelineEvent]) -> Vec<String> {
        let mut indicators = Vec::new();

        let sources: HashSet<_> = events.iter()
            .map(|e| e.source.clone())
            .collect();
        let targets: HashSet<_> = events.iter()
            .map(|e| e.target.clone())
            .collect();

        for source in sources {
            indicators.push(format!("Source: {}", source));
        }

        for target in targets {
            indicators.push(format!("Target: {}", target));
        }

        indicators
    }

    /// Get recommended actions for attack pattern
    fn get_recommended_actions(&self, pattern: AttackPattern) -> Vec<String> {
        match pattern {
            AttackPattern::PortScanning => vec![
                String::from("Block suspicious IP addresses"),
                String::from("Update firewall rules"),
                String::from("Monitor for follow-up attacks"),
            ],
            AttackPattern::BruteForce => vec![
                String::from("Implement account lockout policies"),
                String::from("Require multi-factor authentication"),
                String::from("Monitor for successful breaches"),
            ],
            AttackPattern::DDoS => vec![
                String::from("Activate DDoS protection"),
                String::from("Scale resources if possible"),
                String::from("Contact ISP for assistance"),
            ],
            _ => vec![
                String::from("Investigate the security incident"),
                String::from("Document findings"),
                String::from("Update security controls"),
            ],
        }
    }
}

/// Correlation engine statistics
#[derive(Debug, Clone)]
pub struct CorrelationStats {
    /// Total events processed
    pub total_events: u64,
    /// Total correlation rules
    pub total_rules: usize,
    /// Correlations found
    pub correlations_found: usize,
    /// Clusters found
    pub clusters_found: usize,
    /// Patterns by type
    pub patterns_by_type: HashMap<AttackPattern, usize>,
    /// Average correlation time in milliseconds
    pub avg_correlation_time: f64,
}

impl CorrelationStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            total_events: 0,
            total_rules: 0,
            correlations_found: 0,
            clusters_found: 0,
            patterns_by_type: HashMap::with_hasher(DefaultHasherBuilder),
            avg_correlation_time: 0.0,
        }
    }
}

impl Default for CorrelationStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a default correlation engine
pub fn create_correlation_engine() -> Arc<Mutex<CorrelationEngine>> {
    Arc::new(Mutex::new(CorrelationEngine::new()))
}

/// Export correlation analysis report
pub fn export_correlation_report(correlations: &[EventCorrelation], patterns: &HashMap<AttackPattern, usize>) -> String {
    let mut output = String::from("Correlation Analysis Report\n");
    output.push_str("==========================\n\n");

    output.push_str(&format!("Total Correlations: {}\n", correlations.len()));
    output.push_str(&format!("Attack Patterns Detected: {}\n\n", patterns.len()));

    output.push_str("Attack Pattern Summary:\n");
    output.push_str("----------------------\n");
    for (pattern, count) in patterns {
        output.push_str(&format!("{:?}: {} occurrences\n", pattern, count));
    }

    output.push_str("\n\nRecent Correlations:\n");
    output.push_str("---------------------\n");
    for correlation in correlations.iter().take(10) {
        output.push_str(&format!(
            "ID: {} | Pattern: {:?} | Confidence: {:?} | Severity: {} | Sources: {} | Targets: {}\n",
            correlation.id,
            correlation.pattern,
            correlation.confidence,
            correlation.severity,
            correlation.sources.len(),
            correlation.targets.len()
        ));
    }

    output
}