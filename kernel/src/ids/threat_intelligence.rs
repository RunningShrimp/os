/// Threat Intelligence Module for IDS

extern crate alloc;
///
/// This module implements threat intelligence integration to enhance
/// intrusion detection with external threat data.

use crate::sync::{SpinLock, Mutex};
use crate::collections::HashMap;
use crate::compat::DefaultHasherBuilder;
use crate::time::{SystemTime, UNIX_EPOCH};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

/// Threat intelligence source type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreatSource {
    /// Open Source Intelligence (OSINT)
    OSINT,
    /// Commercial threat feeds
    Commercial,
    /// Government intelligence
    Government,
    /// Industry sharing
    ISAC,
    /// Research organizations
    Research,
    /// Internal intelligence
    Internal,
    /// Community sharing
    Community,
}

/// Threat indicator type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndicatorType {
    /// IP addresses
    IPAddress,
    /// Domain names
    Domain,
    /// File hashes
    FileHash,
    /// URLs
    URL,
    /// Email addresses
    Email,
    /// Certificate fingerprints
    Certificate,
    /// Registry keys
    Registry,
    /// Process names
    ProcessName,
    /// Network signatures
    NetworkSignature,
    /// YARA rules
    YARARule,
}

/// Threat confidence level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatConfidence {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Threat severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// Threat indicator
#[derive(Debug, Clone)]
pub struct ThreatIndicator {
    /// Indicator value (IP, hash, domain, etc.)
    pub value: String,
    /// Indicator type
    pub indicator_type: IndicatorType,
    /// Threat description
    pub description: String,
    /// Confidence level
    pub confidence: ThreatConfidence,
    /// Severity level
    pub severity: ThreatSeverity,
    /// Source of intelligence
    pub source: ThreatSource,
    /// First seen timestamp
    pub first_seen: u64,
    /// Last seen timestamp
    pub last_seen: u64,
    /// Expiration timestamp
    pub expires_at: u64,
    /// Associated threat actors
    pub threat_actors: Vec<String>,
    /// Associated malware families
    pub malware_families: Vec<String>,
    /// Campaign information
    pub campaigns: Vec<String>,
    /// Tags and labels
    pub tags: Vec<String>,
    /// Reference links
    pub references: Vec<String>,
    /// Context information
    pub context: HashMap<String, String>,
    /// Is this indicator active
    pub active: bool,
}

/// Threat actor profile
#[derive(Debug, Clone)]
pub struct ThreatActor {
    /// Actor name
    pub name: String,
    /// Aliases
    pub aliases: Vec<String>,
    /// Description
    pub description: String,
    /// Country of origin
    pub country: String,
    /// Motivations
    pub motivations: Vec<String>,
    /// Capabilities
    pub capabilities: Vec<String>,
    /// Target industries
    pub targets: Vec<String>,
    /// Attack patterns
    pub attack_patterns: Vec<String>,
    /// Known TTPs (Tactics, Techniques, Procedures)
    pub ttps: Vec<String>,
    /// Associated indicators
    pub indicators: Vec<String>,
    /// First activity
    pub first_seen: u64,
    /// Last activity
    pub last_seen: u64,
    /// Activity level
    pub activity_level: f32, // 0.0 - 1.0
}

/// Campaign information
#[derive(Debug, Clone)]
pub struct Campaign {
    /// Campaign name/ID
    pub name: String,
    /// Description
    pub description: String,
    /// Threat actors involved
    pub threat_actors: Vec<String>,
    /// Start date
    pub start_date: u64,
    /// End date (if known)
    pub end_date: Option<u64>,
    /// Target geography
    pub targets: Vec<String>,
    /// Objectives
    pub objectives: Vec<String>,
    /// Associated malware
    pub malware: Vec<String>,
    /// Status
    pub status: CampaignStatus,
}

/// Campaign status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignStatus {
    Active,
    Dormant,
    Completed,
    Unknown,
}

/// Threat intelligence feed configuration
#[derive(Debug, Clone)]
pub struct FeedConfig {
    /// Feed name
    pub name: String,
    /// Feed URL or source
    pub source: String,
    /// Feed type
    pub feed_type: ThreatSource,
    /// Update frequency in seconds
    pub update_frequency: u64,
    /// Last update timestamp
    pub last_update: u64,
    /// Is this feed active
    pub active: bool,
    /// API key or credentials
    pub credentials: Option<String>,
    /// Supported indicator types
    pub indicator_types: Vec<IndicatorType>,
}

/// Threat intelligence integration engine
pub struct ThreatIntelligence {
    /// Threat indicators by value
    indicators: HashMap<String, ThreatIndicator>,
    /// Indicators by type
    indicators_by_type: HashMap<IndicatorType, Vec<String>>,
    /// Threat actors
    threat_actors: HashMap<String, ThreatActor>,
    /// Campaigns
    campaigns: HashMap<String, Campaign>,
    /// Feed configurations
    feeds: HashMap<String, FeedConfig>,
    /// Recent matches
    recent_matches: Vec<ThreatMatch>,
    /// Statistics
    stats: ThreatStats,
    /// Match counter
    match_counter: AtomicU64,
    /// Indicator counter
    indicator_counter: AtomicU64,
    /// Engine lock
    engine_lock: SpinLock,
}

/// Threat match result
#[derive(Debug, Clone)]
pub struct ThreatMatch {
    /// Match ID
    pub id: u64,
    /// Matched indicator
    pub indicator: ThreatIndicator,
    /// Match timestamp
    pub timestamp: u64,
    /// Context of match
    pub context: HashMap<String, String>,
    /// Source of detection
    pub source: String,
    /// Match confidence
    pub confidence: f32,
    /// Recommended actions
    pub actions: Vec<String>,
}

impl ThreatIntelligence {
    /// Create a new threat intelligence engine
    pub fn new() -> Self {
        Self {
            indicators: HashMap::with_hasher(DefaultHasherBuilder),
            indicators_by_type: HashMap::with_hasher(DefaultHasherBuilder),
            threat_actors: HashMap::with_hasher(DefaultHasherBuilder),
            campaigns: HashMap::with_hasher(DefaultHasherBuilder),
            feeds: HashMap::with_hasher(DefaultHasherBuilder),
            recent_matches: Vec::new(),
            stats: ThreatStats::new(),
            match_counter: AtomicU64::new(0),
            indicator_counter: AtomicU64::new(0),
            engine_lock: SpinLock::new(),
        }
    }

    /// Initialize threat intelligence engine with config
    pub fn init(&mut self, _config: &crate::ids::ThreatIntelligenceConfig) -> Result<(), &'static str> {
        // For now, no-op initialization
        Ok(())
    }

    /// Update intelligence data with new threat feeds
    pub fn update_intelligence(&mut self, data: Vec<crate::ids::ThreatData>) -> Result<(), &'static str> {
        for d in data {
            // Insert simple indicator placeholders
            let id = self.indicator_counter.fetch_add(1, Ordering::SeqCst);
            let indicator = ThreatIndicator {
                value: d.threat_id.clone(),
                indicator_type: IndicatorType::IPAddress,
                description: String::from("updated"),
                confidence: ThreatConfidence::Medium,
                severity: ThreatSeverity::Low,
                source: ThreatSource::Internal,
                first_seen: d.valid_until,
                last_seen: d.valid_until,
                expires_at: d.valid_until,
                threat_actors: Vec::new(),
                malware_families: Vec::new(),
                campaigns: Vec::new(),
                tags: Vec::new(),
                references: Vec::new(),
                context: HashMap::with_hasher(crate::compat::DefaultHasherBuilder),
                active: true,
            };
            self.indicators.insert(indicator.value.clone(), indicator);
        }
        Ok(())
    }

    /// Check whether a DetectionSource is considered a threat according to indicators
    pub fn is_threat_source(&mut self, source: &crate::ids::DetectionSource) -> Result<bool, &'static str> {
        let _lock = self.engine_lock.lock();

        // Only check network-related sources for now
        match source.source_type {
            crate::ids::SourceType::NetworkPacket | crate::ids::SourceType::NetworkConnection => {
                if source.source_address.is_empty() {
                    return Ok(false);
                }

                // direct indicator lookup (IP or domain)
                if let Some(ind) = self.indicators.get(&source.source_address) {
                    if ind.active && !self.is_expired(ind.expires_at) {
                        return Ok(true);
                    }
                }

                // no direct match - check partial matches against stored indicators
                for (value, ind) in &self.indicators {
                    if !ind.active || self.is_expired(ind.expires_at) {
                        continue;
                    }

                    if ind.indicator_type == IndicatorType::Domain && self.check_domain_match(&source.source_address, value) {
                        return Ok(true);
                    }
                    if ind.indicator_type == IndicatorType::IPAddress && &source.source_address == value {
                        return Ok(true);
                    }
                }

                Ok(false)
            }
            // For non-network sources we don't have intelligence yet
            _ => Ok(false),
        }
    }

    /// Shutdown the module
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        self.indicators.clear();
        self.indicators_by_type.clear();
        self.threat_actors.clear();
        self.campaigns.clear();
        Ok(())
    }

    /// Add a threat indicator
    pub fn add_indicator(&mut self, indicator: ThreatIndicator) {
        let _lock = self.engine_lock.lock();

        if !self.indicators.contains_key(&indicator.value) {
            self.indicator_counter.fetch_add(1, Ordering::Relaxed);
        }

        self.indicators.insert(indicator.value.clone(), indicator.clone());

        // Update type index
        let type_indicators = self.indicators_by_type
            .entry(indicator.indicator_type)
            .or_insert_with(Vec::new);

        if !type_indicators.contains(&indicator.value) {
            type_indicators.push(indicator.value.clone());
        }

        self.stats.total_indicators = self.indicators.len();
    }

    /// Check if a value matches any threat indicators
    pub fn check_indicator(&mut self, value: &str, context: &HashMap<String, String>) -> Vec<ThreatMatch> {
        let _lock = self.engine_lock.lock();

        let mut matches = Vec::new();

        // Check for exact matches first
        if let Some(indicator) = self.indicators.get(value) {
            if indicator.active && !self.is_expired(indicator.expires_at) {
                let threat_match = self.create_threat_match(indicator.clone(), context);
                matches.push(threat_match);
            }
        }

        // Check for partial matches (domains, URLs, etc.)
        for (indicator_value, indicator) in &self.indicators {
            if !indicator.active || self.is_expired(indicator.expires_at) {
                continue;
            }

            let partial_match = match indicator.indicator_type {
                IndicatorType::Domain => self.check_domain_match(value, indicator_value),
                IndicatorType::URL => self.check_url_match(value, indicator_value),
                IndicatorType::FileHash => self.check_hash_match(value, indicator_value),
                _ => false,
            };

            if partial_match && indicator_value != value {
                let threat_match = self.create_threat_match(indicator.clone(), context);
                matches.push(threat_match);
            }
        }

        // Store matches
        for threat_match in &matches {
            self.add_match(threat_match.clone());
        }

        matches
    }

    /// Check IP address reputation
    pub fn check_ip_reputation(&mut self, ip: &str) -> Option<&ThreatIndicator> {
        let _lock = self.engine_lock.lock();

        if let Some(indicator) = self.indicators.get(ip) {
            if indicator.active && !self.is_expired(indicator.expires_at) {
                return Some(indicator);
            }
        }

        None
    }

    /// Check domain reputation
    pub fn check_domain_reputation(&mut self, domain: &str) -> Option<&ThreatIndicator> {
        let _lock = self.engine_lock.lock();

        // Check exact domain match
        if let Some(indicator) = self.indicators.get(domain) {
            if indicator.active && !self.is_expired(indicator.expires_at) {
                return Some(indicator);
            }
        }

        // Check subdomain matches
        for (indicator_value, indicator) in &self.indicators {
            if indicator.active && !self.is_expired(indicator.expires_at) {
                if indicator.indicator_type == IndicatorType::Domain {
                    if self.is_subdomain(domain, indicator_value) || self.is_subdomain(indicator_value, domain) {
                        return Some(indicator);
                    }
                }
            }
        }

        None
    }

    /// Check file hash reputation
    pub fn check_hash_reputation(&mut self, hash: &str) -> Option<&ThreatIndicator> {
        let _lock = self.engine_lock.lock();

        if let Some(indicator) = self.indicators.get(hash) {
            if indicator.active && !self.is_expired(indicator.expires_at) {
                return Some(indicator);
            }
        }

        None
    }

    /// Add threat actor
    pub fn add_threat_actor(&mut self, actor: ThreatActor) {
        let _lock = self.engine_lock.lock();
        self.threat_actors.insert(actor.name.clone(), actor);
    }

    /// Add campaign
    pub fn add_campaign(&mut self, campaign: Campaign) {
        let _lock = self.engine_lock.lock();
        self.campaigns.insert(campaign.name.clone(), campaign);
    }

    /// Get threat actor by name
    pub fn get_threat_actor(&self, name: &str) -> Option<&ThreatActor> {
        self.threat_actors.get(name)
    }

    /// Get campaign by name
    pub fn get_campaign(&self, name: &str) -> Option<&Campaign> {
        self.campaigns.get(name)
    }

    /// Get recent threat matches
    pub fn get_recent_matches(&self, count: usize) -> Vec<ThreatMatch> {
        self.recent_matches.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Update threat feed
    pub fn update_feed(&mut self, feed_name: &str) -> Result<(), &'static str> {
        let _lock = self.engine_lock.lock();

        let feed_clone = self.feeds.get(feed_name).cloned();

        if let Some(feed) = feed_clone {
            if !feed.active {
                return Err("Feed is not active");
            }

            // In a real implementation, this would fetch data from the feed
            // For now, we'll simulate the update
            self.simulate_feed_update(&feed);
            self.stats.feeds_updated += 1;
            Ok(())
        } else {
            Err("Feed not found")
        }
    }

    /// Get threat intelligence statistics
    pub fn get_statistics(&self) -> &ThreatStats {
        &self.stats
    }

    /// Clear expired indicators
    pub fn clear_expired_indicators(&mut self) {
        let _lock = self.engine_lock.lock();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut expired_values = Vec::new();

        for (value, indicator) in &self.indicators {
            if self.is_expired(indicator.expires_at) {
                expired_values.push(value.clone());
            }
        }

        for value in expired_values {
            if let Some(indicator) = self.indicators.remove(&value) {
                // Update type index
                if let Some(type_indicators) = self.indicators_by_type.get_mut(&indicator.indicator_type) {
                    type_indicators.retain(|v| v != &value);
                }
            }
        }

        self.stats.total_indicators = self.indicators.len();
    }

    /// Check if indicator is expired
    fn is_expired(&self, expires_at: u64) -> bool {
        if expires_at == 0 {
            return false; // No expiration
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        current_time > expires_at
    }

    /// Check domain match
    fn check_domain_match(&self, value: &str, indicator: &str) -> bool {
        value == indicator ||
        value.ends_with(&format!(".{}", indicator)) ||
        indicator.ends_with(&format!(".{}", value))
    }

    /// Check URL match
    fn check_url_match(&self, value: &str, indicator: &str) -> bool {
        value.contains(indicator) || indicator.contains(value)
    }

    /// Check hash match (supports partial matching)
    fn check_hash_match(&self, value: &str, indicator: &str) -> bool {
        if value.len() >= 8 && indicator.len() >= 8 {
            value[..8] == indicator[..8]
        } else {
            value == indicator
        }
    }

    /// Check if one domain is a subdomain of another
    fn is_subdomain(&self, domain: &str, parent: &str) -> bool {
        if domain == parent {
            return false;
        }
        domain.ends_with(&format!(".{}", parent))
    }

    /// Create a threat match
    fn create_threat_match(&self, indicator: ThreatIndicator, context: &HashMap<String, String>) -> ThreatMatch {
        let id = self.match_counter.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let confidence = match indicator.confidence {
            ThreatConfidence::VeryHigh => 1.0,
            ThreatConfidence::High => 0.8,
            ThreatConfidence::Medium => 0.6,
            ThreatConfidence::Low => 0.4,
            ThreatConfidence::VeryLow => 0.2,
        };

        let mut actions = Vec::new();
        match indicator.severity {
            ThreatSeverity::Critical => {
                actions.push(String::from("Block immediately"));
                actions.push(String::from("Escalate to security team"));
            }
            ThreatSeverity::High => {
                actions.push(String::from("Monitor closely"));
                actions.push(String::from("Consider blocking"));
            }
            ThreatSeverity::Medium => {
                actions.push(String::from("Log for investigation"));
            }
            ThreatSeverity::Low => {
                actions.push(String::from("Monitor if possible"));
            }
            ThreatSeverity::Info => {
                actions.push(String::from("Record for intelligence"));
            }
        }

        ThreatMatch {
            id,
            indicator,
            timestamp,
            context: context.clone(),
            source: String::from("ThreatIntelligence"),
            confidence,
            actions,
        }
    }

    /// Add threat match to storage
    fn add_match(&mut self, threat_match: ThreatMatch) {
        if self.recent_matches.len() >= 10000 {
            self.recent_matches.remove(0);
        }
        self.recent_matches.push(threat_match);
        self.stats.total_matches += 1;
    }

    /// Simulate feed update (placeholder for real implementation)
    fn simulate_feed_update(&mut self, feed: &FeedConfig) {
        // In a real implementation, this would:
        // 1. Fetch data from the feed source
        // 2. Parse the threat indicators
        // 3. Validate and normalize the data
        // 4. Add indicators to the database

        // For now, we'll add some sample indicators
        let sample_indicators = vec![
            ThreatIndicator {
                value: String::from("192.168.100.1"),
                indicator_type: IndicatorType::IPAddress,
                description: String::from("Known malicious IP"),
                confidence: ThreatConfidence::High,
                severity: ThreatSeverity::High,
                source: feed.feed_type,
                first_seen: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                last_seen: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                expires_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() + 86400,
                threat_actors: vec![String::from("TestActor")],
                malware_families: Vec::new(),
                campaigns: Vec::new(),
                tags: vec![String::from("malware")],
                references: Vec::new(),
                context: HashMap::with_hasher(DefaultHasherBuilder),
                active: true,
            },
        ];

        for indicator in sample_indicators {
            self.add_indicator(indicator);
        }
    }
}

/// Threat intelligence statistics
#[derive(Debug, Clone)]
pub struct ThreatStats {
    /// Total indicators loaded
    pub total_indicators: usize,
    /// Indicators by type
    pub indicators_by_type: HashMap<IndicatorType, usize>,
    /// Total threat matches
    pub total_matches: u64,
    /// Threat actors tracked
    pub threat_actors: usize,
    /// Campaigns tracked
    pub campaigns: usize,
    /// Feeds updated
    pub feeds_updated: u64,
    /// Last update timestamp
    pub last_update: u64,
}

impl ThreatStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            total_indicators: 0,
            indicators_by_type: HashMap::with_hasher(DefaultHasherBuilder),
            total_matches: 0,
            threat_actors: 0,
            campaigns: 0,
            feeds_updated: 0,
            last_update: 0,
        }
    }
}

impl Default for ThreatStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a default threat intelligence engine
pub fn create_threat_intelligence() -> Arc<Mutex<ThreatIntelligence>> {
    Arc::new(Mutex::new(ThreatIntelligence::new()))
}

/// Export threat intelligence data
pub fn export_threat_intelligence(indicators: &[ThreatIndicator], actors: &[ThreatActor]) -> String {
    let mut output = String::from("Threat Intelligence Export\n");
    output.push_str("==========================\n\n");

    output.push_str(&format!("Total Indicators: {}\n", indicators.len()));
    output.push_str(&format!("Total Threat Actors: {}\n\n", actors.len()));

    output.push_str("High-Severity Indicators:\n");
    output.push_str("-------------------------\n");
    for indicator in indicators {
        if indicator.severity >= ThreatSeverity::High {
            output.push_str(&format!(
                "{} ({:?}) - {} - Severity: {:?}\n",
                indicator.value, indicator.indicator_type, indicator.description, indicator.severity
            ));
        }
    }

    output.push_str("\n\nThreat Actors:\n");
    output.push_str("-------------\n");
    for actor in actors {
        output.push_str(&format!(
            "{} - {} - Activity Level: {:.2}\n",
            actor.name, actor.description, actor.activity_level
        ));
    }

    output
}