/// Behavior Analysis Module for IDS

extern crate alloc;
///
/// This module implements behavioral analysis to detect unusual system
/// and user behavior that may indicate security threats.

use crate::subsystems::sync::{SpinLock, Mutex};
use crate::collections::HashMap;
use crate::compat::DefaultHasherBuilder;
use crate::subsystems::time::{SystemTime, UNIX_EPOCH};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

/// Behavior risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Normal,
    Suspicious,
    Anomalous,
    Malicious,
}

/// Behavior category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BehaviorCategory {
    /// User activity patterns
    UserActivity,
    /// Process execution patterns
    ProcessExecution,
    /// File access patterns
    FileAccess,
    /// Network usage patterns
    NetworkActivity,
    /// System resource usage
    ResourceUsage,
    /// Authentication patterns
    Authentication,
    /// Privilege escalation
    PrivilegeEscalation,
    /// Data exfiltration
    DataExfiltration,
}

/// Behavior event
#[derive(Debug, Clone)]
pub struct BehaviorEvent {
    /// Event identifier
    pub id: u64,
    /// Event timestamp
    pub timestamp: u64,
    /// Event category
    pub category: BehaviorCategory,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Subject (user, process, etc.)
    pub subject: String,
    /// Action performed
    pub action: String,
    /// Object involved
    pub object: String,
    /// Event details
    pub details: HashMap<String, String>,
    /// Event source
    pub source: String,
    /// Context information
    pub context: HashMap<String, String>,
}

/// Behavior profile baseline
#[derive(Debug, Clone)]
pub struct BehaviorBaseline {
    /// Subject identifier
    pub subject: String,
    /// Behavior category
    pub category: BehaviorCategory,
    /// Normal frequency range
    pub frequency_range: (f64, f64), // (min, max)
    /// Normal time patterns
    pub time_patterns: Vec<(u32, u32)>, // (hour_start, hour_end)
    /// Normal duration range
    pub duration_range: (f64, f64),
    /// Common patterns
    pub common_patterns: Vec<String>,
    /// Threshold values
    pub thresholds: HashMap<String, f64>,
    /// Creation timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
    /// Confidence level
    pub confidence: f32,
    /// Sample size used for baseline
    pub sample_size: usize,
}

/// Behavior anomaly
#[derive(Debug, Clone)]
pub struct BehaviorAnomaly {
    /// Anomaly identifier
    pub id: u64,
    /// Anomaly timestamp
    pub timestamp: u64,
    /// Subject involved
    pub subject: String,
    /// Anomaly category
    pub category: BehaviorCategory,
    /// Anomaly type
    pub anomaly_type: String,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Description
    pub description: String,
    /// Deviation score (0.0 - 1.0)
    pub deviation_score: f32,
    /// Baseline that was violated
    pub baseline_id: String,
    /// Related events
    pub related_events: Vec<u64>,
    /// Suggested actions
    pub suggested_actions: Vec<String>,
}

/// User behavior profile
#[derive(Debug, Clone)]
pub struct UserProfile {
    /// User ID
    pub user_id: u32,
    /// Username
    pub username: String,
    /// User role
    pub role: String,
    /// Department
    pub department: String,
    /// Login time patterns
    pub login_patterns: Vec<(u32, u32)>, // (hour_start, hour_end)
    /// Common applications
    pub common_apps: Vec<String>,
    /// File access patterns
    pub file_patterns: Vec<String>,
    /// Network usage patterns
    pub network_patterns: Vec<String>,
    /// Risk score
    pub risk_score: f32,
    /// Last activity timestamp
    pub last_activity: u64,
    /// Creation timestamp
    pub created_at: u64,
}

/// Process behavior profile
#[derive(Debug, Clone)]
pub struct ProcessProfile {
    /// Process name
    pub process_name: String,
    /// Process path
    pub path: String,
    /// Average CPU usage
    pub avg_cpu_usage: f64,
    /// Average memory usage
    pub avg_memory_usage: f64,
    /// Common parent processes
    pub common_parents: Vec<String>,
    /// Normal arguments
    pub common_args: Vec<String>,
    /// Normal network connections
    pub normal_connections: Vec<(String, u16)>, // (ip, port)
    /// File system access patterns
    pub file_access_patterns: Vec<String>,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Creation timestamp
    pub created_at: u64,
}

/// Behavior analysis engine
pub struct BehaviorAnalyzer {
    /// Behavior baselines
    baselines: HashMap<String, BehaviorBaseline>,
    /// User profiles
    user_profiles: HashMap<u32, UserProfile>,
    /// Process profiles
    event_patterns: HashMap<String, ProcessProfile>,
    /// Recent behavior events
    events: Vec<BehaviorEvent>,
    /// Detected anomalies
    anomalies: Vec<BehaviorAnomaly>,
    /// Event counter
    event_counter: AtomicU64,
    /// Anomaly counter
    anomaly_counter: AtomicU64,
    /// Analysis statistics
    stats: BehaviorStats,
    /// Analyzer lock
    analyzer_lock: SpinLock,
}

impl BehaviorAnalyzer {
    /// Create a new behavior analyzer
    pub fn new() -> Self {
        Self {
            baselines: HashMap::with_hasher(DefaultHasherBuilder),
            user_profiles: HashMap::with_hasher(DefaultHasherBuilder),
            event_patterns: HashMap::with_hasher(DefaultHasherBuilder),
            events: Vec::new(),
            anomalies: Vec::new(),
            event_counter: AtomicU64::new(0),
            anomaly_counter: AtomicU64::new(0),
            stats: BehaviorStats::new(),
            analyzer_lock: SpinLock::new(),
        }
    }

    /// Initialize the behavior analyzer with the config
    pub fn init(&mut self, _config: &crate::ids::BehaviorAnalysisConfig) -> Result<(), &'static str> {
        // In a real implementation we would configure models and load baselines.
        Ok(())
    }

    /// Shutdown/cleanup
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        self.events.clear();
        self.anomalies.clear();
        Ok(())
    }

    /// Record a behavior event
    pub fn record_event(&mut self, event: BehaviorEvent) {
        let _lock = self.analyzer_lock.lock();

        // Add event to storage
        let event_id = self.event_counter.fetch_add(1, Ordering::Relaxed);
        let mut event_with_id = event;
        event_with_id.id = event_id;

        self.events.push(event_with_id.clone());

        // Update user profile if applicable
        if let Some(user_id_str) = event_with_id.details.get("user_id") {
            if let Ok(user_id) = user_id_str.parse::<u32>() {
                self.update_user_profile(user_id, &event_with_id);
            }
        }

        // Update process profile if applicable
        if !event_with_id.subject.is_empty() {
            self.update_process_profile(&event_with_id.subject, &event_with_id);
        }

        // Check for anomalies
        self.check_for_anomalies(&event_with_id);

        // Update statistics
        self.stats.total_events += 1;
        self.stats.events_by_category
            .entry(event_with_id.category)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    /// Create or update behavior baseline
    pub fn update_baseline(&mut self, baseline: BehaviorBaseline) {
        let _lock = self.analyzer_lock.lock();

        let key = format!("{}:{}", baseline.subject, "category");
        self.baselines.insert(key, baseline);
        self.stats.total_baselines = self.baselines.len();
    }

    /// Analyze user behavior
    pub fn analyze_user_behavior(&mut self, user_id: u32, action: &str, resource: &str, details: &HashMap<String, String>) -> Option<BehaviorAnomaly> {
        let _lock = self.analyzer_lock.lock();

        if let Some(user_profile) = self.user_profiles.get(&user_id) {
            // Check time patterns
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let current_hour = ((current_time / 3600) % 24) as u32;

            let unusual_time = !user_profile.login_patterns.iter()
                .any(|(start, end)| current_hour >= *start && current_hour <= *end);

            // Check risk escalation
            let mut risk_factors = 0;
            if unusual_time { risk_factors += 1; }

            // Check for unusual file access
            if let Some(file_path) = details.get("file_path") {
                let unusual_file = !user_profile.file_patterns.iter()
                    .any(|pattern| file_path.contains(pattern));
                if unusual_file { risk_factors += 1; }
            }

            // Check for unusual application usage
            if let Some(app_name) = details.get("application") {
                let unusual_app = !user_profile.common_apps.iter()
                    .any(|app| app_name.contains(app));
                if unusual_app { risk_factors += 1; }
            }

            // Generate anomaly if multiple risk factors
            if risk_factors >= 2 {
                let anomaly = self.create_anomaly(
                    format!("User {}", user_id),
                    BehaviorCategory::UserActivity,
                    String::from("Unusual user behavior detected"),
                    RiskLevel::Suspicious,
                    risk_factors as f32 / 3.0,
                );
                return Some(anomaly);
            }
        }

        None
    }

    /// Analyze process behavior
    pub fn analyze_process_behavior(&mut self, process_name: &str, action: &str, details: &HashMap<String, String>) -> Option<BehaviorAnomaly> {
        let _lock = self.analyzer_lock.lock();

        if let Some(process_profile) = self.event_patterns.get(process_name) {
            // Check for suspicious process behavior
            let mut risk_factors = 0;

            // Check for unusual network connections
            if let Some(remote_ip) = details.get("remote_ip") {
                let unusual_connection = !process_profile.normal_connections.iter()
                    .any(|(ip, _port)| ip == remote_ip);
                if unusual_connection { risk_factors += 1; }
            }

            // Check for unusual file access
            if let Some(file_path) = details.get("file_path") {
                let unusual_access = !process_profile.file_access_patterns.iter()
                    .any(|pattern| file_path.contains(pattern));
                if unusual_access { risk_factors += 1; }
            }

            // Check for privilege escalation attempts
            if action == "setuid" || action == "sudo" {
                if !process_profile.common_parents.iter().any(|parent| parent.contains("sudo")) {
                    risk_factors += 2; // Higher weight for privilege escalation
                }
            }

            if risk_factors >= 2 {
                let anomaly = self.create_anomaly(
                    String::from(process_name),
                    BehaviorCategory::ProcessExecution,
                    String::from("Suspicious process behavior detected"),
                    RiskLevel::Anomalous,
                    risk_factors as f32 / 4.0,
                );
                return Some(anomaly);
            }
        }

        None
    }

    /// Get recent anomalies
    pub fn get_recent_anomalies(&self, count: usize) -> Vec<BehaviorAnomaly> {
        self.anomalies.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get user profile
    pub fn get_user_profile(&self, user_id: u32) -> Option<&UserProfile> {
        self.user_profiles.get(&user_id)
    }

    /// Get process profile
    pub fn get_process_profile(&self, process_name: &str) -> Option<&ProcessProfile> {
        self.event_patterns.get(process_name)
    }

    /// Get behavior statistics
    pub fn get_statistics(&self) -> &BehaviorStats {
        &self.stats
    }

    /// Clear old events and anomalies
    pub fn clear_old_data(&mut self, max_age_seconds: u64) {
        let _lock = self.analyzer_lock.lock();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Clear old events
        self.events.retain(|event| {
            current_time - event.timestamp <= max_age_seconds
        });

        // Clear old anomalies
        self.anomalies.retain(|anomaly| {
            current_time - anomaly.timestamp <= max_age_seconds
        });
    }

    /// Update user profile based on event
    fn update_user_profile(&mut self, user_id: u32, event: &BehaviorEvent) {
        let profile = self.user_profiles.entry(user_id).or_insert_with(|| {
            UserProfile {
                user_id,
                username: event.subject.clone(),
                role: String::from("Unknown"),
                department: String::from("Unknown"),
                login_patterns: Vec::new(),
                common_apps: Vec::new(),
                file_patterns: Vec::new(),
                network_patterns: Vec::new(),
                risk_score: 0.0,
                last_activity: event.timestamp,
                created_at: event.timestamp,
            }
        });

        // Update last activity
        profile.last_activity = event.timestamp;

        // Update patterns based on event
        if let Some(app_name) = event.details.get("application") {
            if !profile.common_apps.contains(app_name) {
                profile.common_apps.push(app_name.clone());
            }
        }

        if let Some(file_path) = event.details.get("file_path") {
            if !profile.file_patterns.iter().any(|pattern| file_path.contains(pattern)) {
                profile.file_patterns.push(file_path.clone());
            }
        }

        // Update risk score based on risk level
        match event.risk_level {
            RiskLevel::Malicious => profile.risk_score = (profile.risk_score + 0.5).min(1.0),
            RiskLevel::Anomalous => profile.risk_score = (profile.risk_score + 0.3).min(1.0),
            RiskLevel::Suspicious => profile.risk_score = (profile.risk_score + 0.1).min(1.0),
            RiskLevel::Normal => profile.risk_score = (profile.risk_score - 0.05).max(0.0),
        }
    }

    /// Update process profile based on event
    fn update_process_profile(&mut self, process_name: &str, event: &BehaviorEvent) {
        let profile = self.event_patterns.entry(String::from(process_name)).or_insert_with(|| {
            ProcessProfile {
                process_name: String::from(process_name),
                path: event.object.clone(),
                avg_cpu_usage: 0.0,
                avg_memory_usage: 0.0,
                common_parents: Vec::new(),
                common_args: Vec::new(),
                normal_connections: Vec::new(),
                file_access_patterns: Vec::new(),
                risk_level: RiskLevel::Normal,
                created_at: event.timestamp,
            }
        });

        // Update file access patterns
        if !event.object.is_empty() {
            if !profile.file_access_patterns.iter().any(|pattern| event.object.contains(pattern)) {
                profile.file_access_patterns.push(event.object.clone());
            }
        }

        // Update network connections
        if let Some(remote_ip) = event.details.get("remote_ip") {
            if let Some(port_str) = event.details.get("remote_port") {
                if let Ok(port) = port_str.parse::<u16>() {
                    let connection = (remote_ip.clone(), port);
                    if !profile.normal_connections.iter().any(|(ip, p)| ip == remote_ip && *p == port) {
                        profile.normal_connections.push(connection);
                    }
                }
            }
        }

        // Update risk level
        profile.risk_level = profile.risk_level.max(event.risk_level);
    }

    /// Check for anomalies based on event and baselines
    fn check_for_anomalies(&mut self, event: &BehaviorEvent) {
        let baseline_key = format!("{}:{:?}", event.subject, event.category);

        if let Some(baseline) = self.baselines.get(&baseline_key) {
            // Check for frequency anomalies
            let current_hour = ((event.timestamp / 3600) % 24) as u32;
            let unusual_time = !baseline.time_patterns.iter()
                .any(|(start, end)| current_hour >= *start && current_hour <= *end);

            if unusual_time && baseline.confidence > 0.7 {
                let anomaly = self.create_anomaly(
                    event.subject.clone(),
                    event.category,
                    format!("Unusual activity time for {}", event.subject),
                    RiskLevel::Suspicious,
                    0.6,
                );
                self.add_anomaly(anomaly);
            }
        }
    }

    /// Create a behavior anomaly
    fn create_anomaly(&self, subject: String, category: BehaviorCategory, description: String, risk_level: RiskLevel, score: f32) -> BehaviorAnomaly {
        let id = self.anomaly_counter.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        BehaviorAnomaly {
            id,
            timestamp,
            subject,
            category,
            anomaly_type: String::from("Behavioral Deviation"),
            risk_level,
            description,
            deviation_score: score,
            baseline_id: String::new(),
            related_events: Vec::new(),
            suggested_actions: vec![
                String::from("Investigate the unusual behavior"),
                String::from("Review recent activity logs"),
                String::from("Consider temporarily restricting access"),
            ],
        }
    }

    /// Add anomaly to storage
    fn add_anomaly(&mut self, anomaly: BehaviorAnomaly) {
        if self.anomalies.len() >= 5000 {
            self.anomalies.remove(0);
        }
        self.anomalies.push(anomaly);
        self.stats.total_anomalies += 1;
    }
}

/// Behavior analysis statistics
#[derive(Debug, Clone)]
pub struct BehaviorStats {
    /// Total behavior events
    pub total_events: u64,
    /// Events by category
    pub events_by_category: HashMap<BehaviorCategory, u64>,
    /// Total baselines
    pub total_baselines: usize,
    /// Total anomalies detected
    pub total_anomalies: u64,
    /// Active user profiles
    pub active_user_profiles: usize,
    /// Active process profiles
    pub active_process_profiles: usize,
}

impl BehaviorStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            total_events: 0,
            events_by_category: HashMap::with_hasher(DefaultHasherBuilder),
            total_baselines: 0,
            total_anomalies: 0,
            active_user_profiles: 0,
            active_process_profiles: 0,
        }
    }
}

impl Default for BehaviorStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a default behavior analyzer
pub fn create_behavior_analyzer() -> Arc<Mutex<BehaviorAnalyzer>> {
    Arc::new(Mutex::new(BehaviorAnalyzer::new()))
}

/// Export behavior analysis report
pub fn export_behavior_report(anomalies: &[BehaviorAnomaly], profiles: &[UserProfile]) -> String {
    let mut output = String::from("Behavior Analysis Report\n");
    output.push_str("========================\n\n");

    output.push_str(&format!("Total Anomalies: {}\n\n", anomalies.len()));

    output.push_str("Recent Anomalies:\n");
    output.push_str("----------------\n");
    for anomaly in anomalies.iter().take(10) {
        output.push_str(&format!(
            "ID: {} | Subject: {} | Risk: {:?} | {}\n",
            anomaly.id, anomaly.subject, anomaly.risk_level, anomaly.description
        ));
    }

    output.push_str("\n\nUser Profiles Summary:\n");
    output.push_str("---------------------\n");
    for profile in profiles {
        output.push_str(&format!(
            "User: {} | Role: {} | Risk Score: {:.2} | Last Activity: {}\n",
            profile.username, profile.role, profile.risk_score, profile.last_activity
        ));
    }

    output
}