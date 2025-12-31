//! Alert silencing and suppression mechanisms
//!
//! This module provides:
//! - Alert silencing rules (time-based, label-based)
//! - Alert inhibition (dependency-based suppression)
//! - Alert suppression (maintenance windows)
//! - Silence expiration and management
//! - Silence query and API

#![no_std]

extern crate alloc;

use alloc::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    fmt,
    time::Duration,
};

use crate::alerting::evaluator::Alert;

/// Silence rule for suppressing alerts
#[derive(Clone, Debug)]
pub struct SilenceRule {
    /// Unique silence ID
    pub id: String,
    /// Silence name/description
    pub name: String,
    /// Label matchers for alerts to silence
    pub matchers: Vec<Matcher>,
    /// Silence type
    pub silence_type: SilenceType,
    /// When the silence starts
    pub starts_at: core::time::Instant,
    /// When the silence ends (None for indefinite)
    pub ends_at: Option<core::time::Instant>,
    /// Who created the silence
    pub created_by: String,
    /// Comments explaining the silence
    pub comment: String,
    /// Whether the silence is active
    pub active: bool,
}

/// Silence type
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SilenceType {
    /// Time-based silence (scheduled)
    Scheduled,
    /// Maintenance window silence
    Maintenance,
    /// Manual silence (user-initiated)
    Manual,
    /// Automatic silence (system-initiated)
    Automatic,
}

/// Label matcher for selecting alerts
#[derive(Clone, Debug, PartialEq)]
pub struct Matcher {
    /// Label name to match
    pub name: String,
    /// Match operation
    pub op: MatchOp,
    /// Value to match (not used for MatchOp::Present)
    pub value: Option<String>,
    /// Whether the matcher is negated
    pub is_regex: bool,
}

/// Match operations
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MatchOp {
    /// Equality match
    Equal,
    /// Not equal match
    NotEqual,
    /// Regex match
    RegexMatch,
    /// Regex not match
    RegexNotMatch,
    /// Label is present
    Present,
}

/// Inhibition rule for dependency-based suppression
#[derive(Clone, Debug)]
pub struct InhibitionRule {
    /// Unique inhibition rule ID
    pub id: String,
    /// Source alert matchers (the alert that inhibits others)
    pub source_matchers: Vec<Matcher>,
    /// Target alert matchers (alerts to be inhibited)
    pub target_matchers: Vec<Matcher>,
    /// Labels that must be equal between source and target
    pub equal_labels: BTreeSet<String>,
    /// Whether the rule is active
    pub active: bool,
}

/// Maintenance window for suppressing alerts
#[derive(Clone, Debug)]
pub struct MaintenanceWindow {
    /// Unique maintenance ID
    pub id: String,
    /// Maintenance window name
    pub name: String,
    /// Label matchers for affected alerts
    pub matchers: Vec<Matcher>,
    /// Start time
    pub starts_at: core::time::Instant,
    /// End time
    pub ends_at: Option<core::time::Instant>,
    /// Maintenance type
    pub maintenance_type: MaintenanceType,
    /// Description of maintenance
    pub description: String,
}

/// Maintenance window types
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaintenanceType {
    /// Planned maintenance
    Planned,
    /// Emergency maintenance
    Emergency,
    /// Upgrade/patch
    Upgrade,
    /// Hardware maintenance
    Hardware,
    /// Network maintenance
    Network,
}

/// Silencing error types
#[derive(Clone, Debug)]
pub enum SilencingError {
    /// Invalid matcher configuration
    InvalidMatcher(String),
    /// Invalid time range
    InvalidTimeRange(String),
    /// Silence not found
    SilenceNotFound(String),
    /// Conflicting silence rules
    ConflictingSilence(String),
    /// Invalid regex pattern
    InvalidRegex(String),
}

impl fmt::Display for SilencingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMatcher(msg) => write!(f, "Invalid matcher: {}", msg),
            Self::InvalidTimeRange(msg) => write!(f, "Invalid time range: {}", msg),
            Self::SilenceNotFound(id) => write!(f, "Silence not found: {}", id),
            Self::ConflictingSilence(msg) => write!(f, "Conflicting silence: {}", msg),
            Self::InvalidRegex(msg) => write!(f, "Invalid regex: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SilencingError {}

/// Silence manager
pub struct SilenceManager {
    /// Active silence rules
    silences: BTreeMap<String, SilenceRule>,
    /// Inhibition rules
    inhibitions: Vec<InhibitionRule>,
    /// Maintenance windows
    maintenance_windows: BTreeMap<String, MaintenanceWindow>,
    /// Expired silences (for history)
    expired_silences: Vec<SilenceRule>,
}

impl SilenceManager {
    /// Create a new silence manager
    pub fn new() -> Self {
        Self {
            silences: BTreeMap::new(),
            inhibitions: Vec::new(),
            maintenance_windows: BTreeMap::new(),
            expired_silences: Vec::new(),
        }
    }

    /// Add a silence rule
    pub fn add_silence(&mut self, silence: SilenceRule) -> Result<(), SilencingError> {
        self.validate_silence(&silence)?;
        self.silences.insert(silence.id.clone(), silence);
        Ok(())
    }

    /// Remove a silence rule
    pub fn remove_silence(&mut self, id: &str) -> Result<(), SilencingError> {
        self.silences
            .remove(id)
            .ok_or_else(|| SilencingError::SilenceNotFound(id.into()))?;
        Ok(())
    }

    /// Get a silence rule by ID
    pub fn get_silence(&self, id: &str) -> Option<&SilenceRule> {
        self.silences.get(id)
    }

    /// Get all active silences
    pub fn active_silences(&self) -> Vec<&SilenceRule> {
        self.silences
            .values()
            .filter(|s| s.active && self.is_silence_active(s))
            .collect()
    }

    /// Check if an alert should be silenced
    pub fn should_silence(&self, alert: &Alert) -> Option<&SilenceRule> {
        // Check silence rules
        for silence in self.active_silences() {
            if self.alert_matches_silence(alert, silence) {
                return Some(silence);
            }
        }
        None
    }

    /// Check if an alert should be inhibited
    pub fn should_inhibit(&self, alert: &Alert, all_alerts: &[Alert]) -> bool {
        for inhibition in &self.inhibitions {
            if !inhibition.active {
                continue;
            }

            // Check if target matches
            if !self.alert_matches_matchers(alert, &inhibition.target_matchers) {
                continue;
            }

            // Check if there's a matching source alert
            for source_alert in all_alerts {
                if source_alert.id == alert.id {
                    continue;
                }

                if self.alert_matches_matchers(source_alert, &inhibition.source_matchers) {
                    // Check equal labels constraint
                    if self.labels_equal(alert, source_alert, &inhibition.equal_labels) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if an alert is in a maintenance window
    pub fn in_maintenance(&self, alert: &Alert) -> Option<&MaintenanceWindow> {
        let now = core::time::Instant::now();

        for window in self.maintenance_windows.values() {
            // Check if window is active
            if now < window.starts_at {
                continue;
            }
            if let Some(end) = window.ends_at {
                if now > end {
                    continue;
                }
            }

            // Check if alert matches window
            if self.alert_matches_matchers(alert, &window.matchers) {
                return Some(window);
            }
        }
        None
    }

    /// Add an inhibition rule
    pub fn add_inhibition(&mut self, inhibition: InhibitionRule) -> Result<(), SilencingError> {
        self.validate_inhibition(&inhibition)?;
        self.inhibitions.push(inhibition);
        Ok(())
    }

    /// Remove an inhibition rule
    pub fn remove_inhibition(&mut self, id: &str) -> Result<(), SilencingError> {
        let original_len = self.inhibitions.len();
        self.inhibitions.retain(|i| i.id != id);
        if self.inhibitions.len() == original_len {
            return Err(SilencingError::SilenceNotFound(id.into()));
        }
        Ok(())
    }

    /// Create a maintenance window
    pub fn create_maintenance(
        &mut self,
        window: MaintenanceWindow,
    ) -> Result<(), SilencingError> {
        self.validate_maintenance_window(&window)?;
        self.maintenance_windows.insert(window.id.clone(), window);
        Ok(())
    }

    /// End a maintenance window
    pub fn end_maintenance(&mut self, id: &str) -> Result<(), SilencingError> {
        if let Some(window) = self.maintenance_windows.get_mut(id) {
            window.ends_at = Some(core::time::Instant::now());
            Ok(())
        } else {
            Err(SilencingError::SilenceNotFound(id.into()))
        }
    }

    /// Update silence states (expire old silences)
    pub fn update(&mut self) {
        let now = core::time::Instant::now();
        let mut to_expire = Vec::new();

        for (id, silence) in &self.silences {
            if !self.is_silence_active(silence) {
                to_expire.push(id.clone());
            }
        }

        for id in to_expire {
            if let Some(mut silence) = self.silences.remove(&id) {
                silence.active = false;
                self.expired_silences.push(silence);
            }
        }
    }

    /// Validate a silence rule
    fn validate_silence(&self, silence: &SilenceRule) -> Result<(), SilencingError> {
        if silence.id.is_empty() {
            return Err(SilencingError::InvalidMatcher(
                "Silence ID cannot be empty".into(),
            ));
        }

        if silence.matchers.is_empty() {
            return Err(SilencingError::InvalidMatcher(
                "Silence must have at least one matcher".into(),
            ));
        }

        // Validate time range
        if let Some(end) = silence.ends_at {
            if end < silence.starts_at {
                return Err(SilencingError::InvalidTimeRange(
                    "End time is before start time".into(),
                ));
            }
        }

        Ok(())
    }

    /// Validate an inhibition rule
    fn validate_inhibition(&self, inhibition: &InhibitionRule) -> Result<(), SilencingError> {
        if inhibition.source_matchers.is_empty() {
            return Err(SilencingError::InvalidMatcher(
                "Inhibition rule must have source matchers".into(),
            ));
        }

        if inhibition.target_matchers.is_empty() {
            return Err(SilencingError::InvalidMatcher(
                "Inhibition rule must have target matchers".into(),
            ));
        }

        Ok(())
    }

    /// Validate a maintenance window
    fn validate_maintenance_window(&self, window: &MaintenanceWindow) -> Result<(), SilencingError> {
        if window.id.is_empty() {
            return Err(SilencingError::InvalidMatcher(
                "Maintenance window ID cannot be empty".into(),
            ));
        }

        if window.matchers.is_empty() {
            return Err(SilencingError::InvalidMatcher(
                "Maintenance window must have at least one matcher".into(),
            ));
        }

        if let Some(end) = window.ends_at {
            if end < window.starts_at {
                return Err(SilencingError::InvalidTimeRange(
                    "End time is before start time".into(),
                ));
            }
        }

        Ok(())
    }

    /// Check if a silence is currently active
    fn is_silence_active(&self, silence: &SilenceRule) -> bool {
        let now = core::time::Instant::now();

        if now < silence.starts_at {
            return false;
        }

        if let Some(end) = silence.ends_at {
            if now > end {
                return false;
            }
        }

        true
    }

    /// Check if an alert matches a silence rule
    fn alert_matches_silence(&self, alert: &Alert, silence: &SilenceRule) -> bool {
        self.alert_matches_matchers(alert, &silence.matchers)
    }

    /// Check if an alert matches a set of matchers
    fn alert_matches_matchers(&self, alert: &Alert, matchers: &[Matcher]) -> bool {
        // All matchers must match
        for matcher in matchers {
            if !self.matcher_matches(alert, matcher) {
                return false;
            }
        }
        true
    }

    /// Check if a single matcher matches an alert
    fn matcher_matches(&self, alert: &Alert, matcher: &Matcher) -> bool {
        let label_value = alert.labels.get(&matcher.name);

        match matcher.op {
            MatchOp::Equal => {
                if let Some(expected) = &matcher.value {
                    label_value.map_or(false, |v| v == expected)
                } else {
                    false
                }
            }
            MatchOp::NotEqual => {
                if let Some(expected) = &matcher.value {
                    label_value.map_or(true, |v| v != expected)
                } else {
                    true
                }
            }
            MatchOp::Present => label_value.is_some(),
            MatchOp::RegexMatch => {
                // Simplified regex matching (real implementation would use regex crate)
                if let Some(expected) = &matcher.value {
                    label_value.map_or(false, |v| v.contains(expected))
                } else {
                    false
                }
            }
            MatchOp::RegexNotMatch => {
                if let Some(expected) = &matcher.value {
                    label_value.map_or(true, |v| !v.contains(expected))
                } else {
                    true
                }
            }
        }
    }

    /// Check if two alerts have equal labels for the specified keys
    fn labels_equal(&self, alert1: &Alert, alert2: &Alert, keys: &BTreeSet<String>) -> bool {
        for key in keys {
            let val1 = alert1.labels.get(key);
            let val2 = alert2.labels.get(key);
            if val1 != val2 {
                return false;
            }
        }
        true
    }

    /// Get all silence rules
    pub fn silences(&self) -> &BTreeMap<String, SilenceRule> {
        &self.silences
    }

    /// Get all inhibition rules
    pub fn inhibitions(&self) -> &[InhibitionRule] {
        &self.inhibitions
    }

    /// Get all maintenance windows
    pub fn maintenance_windows(&self) -> &BTreeMap<String, MaintenanceWindow> {
        &self.maintenance_windows
    }

    /// Get expired silence history
    pub fn expired_silences(&self) -> &[SilenceRule] {
        &self.expired_silences
    }
}

impl Default for SilenceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Matcher {
    /// Create a new equality matcher
    pub fn equal(name: String, value: String) -> Self {
        Self {
            name,
            op: MatchOp::Equal,
            value: Some(value),
            is_regex: false,
        }
    }

    /// Create a new regex matcher
    pub fn regex_match(name: String, value: String) -> Self {
        Self {
            name,
            op: MatchOp::RegexMatch,
            value: Some(value),
            is_regex: true,
        }
    }

    /// Create a new present matcher
    pub fn present(name: String) -> Self {
        Self {
            name,
            op: MatchOp::Present,
            value: None,
            is_regex: false,
        }
    }
}

impl SilenceRule {
    /// Create a new silence rule
    pub fn new(
        id: String,
        name: String,
        matchers: Vec<Matcher>,
        silence_type: SilenceType,
        starts_at: core::time::Instant,
        ends_at: Option<core::time::Instant>,
        created_by: String,
        comment: String,
    ) -> Self {
        Self {
            id,
            name,
            matchers,
            silence_type,
            starts_at,
            ends_at,
            created_by,
            comment,
            active: true,
        }
    }

    /// Create a silence for a specific duration
    pub fn with_duration(
        id: String,
        name: String,
        matchers: Vec<Matcher>,
        silence_type: SilenceType,
        duration: Duration,
        created_by: String,
        comment: String,
    ) -> Self {
        let starts_at = core::time::Instant::now();
        let ends_at = Some(starts_at + duration);

        Self {
            id,
            name,
            matchers,
            silence_type,
            starts_at,
            ends_at,
            created_by,
            comment,
            active: true,
        }
    }
}

impl InhibitionRule {
    /// Create a new inhibition rule
    pub fn new(
        id: String,
        source_matchers: Vec<Matcher>,
        target_matchers: Vec<Matcher>,
        equal_labels: BTreeSet<String>,
    ) -> Self {
        Self {
            id,
            source_matchers,
            target_matchers,
            equal_labels,
            active: true,
        }
    }
}

impl MaintenanceWindow {
    /// Create a new maintenance window
    pub fn new(
        id: String,
        name: String,
        matchers: Vec<Matcher>,
        starts_at: core::time::Instant,
        ends_at: Option<core::time::Instant>,
        maintenance_type: MaintenanceType,
        description: String,
    ) -> Self {
        Self {
            id,
            name,
            matchers,
            starts_at,
            ends_at,
            maintenance_type,
            description,
        }
    }

    /// Create a maintenance window with a duration
    pub fn with_duration(
        id: String,
        name: String,
        matchers: Vec<Matcher>,
        duration: Duration,
        maintenance_type: MaintenanceType,
        description: String,
    ) -> Self {
        let starts_at = core::time::Instant::now();
        let ends_at = Some(starts_at + duration);

        Self {
            id,
            name,
            matchers,
            starts_at,
            ends_at,
            maintenance_type,
            description,
        }
    }
}

/// Query builder for silence rules
pub struct SilenceQueryBuilder {
    matchers: Vec<Matcher>,
    active_only: bool,
}

impl SilenceQueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            matchers: Vec::new(),
            active_only: true,
        }
    }

    /// Add a matcher to the query
    pub fn with_matcher(mut self, matcher: Matcher) -> Self {
        self.matchers.push(matcher);
        self
    }

    /// Set whether to query only active silences
    pub fn active_only(mut self, active_only: bool) -> Self {
        self.active_only = active_only;
        self
    }

    /// Execute the query
    pub fn query<'a>(&self, manager: &'a SilenceManager) -> Vec<&'a SilenceRule> {
        manager
            .silences()
            .values()
            .filter(|silence| {
                if self.active_only && !silence.active {
                    return false;
                }

                // Match against query matchers
                for matcher in &self.matchers {
                    // This would need the alert context to properly match
                    // For now, we'll just check if the silence has matching matchers
                }

                true
            })
            .collect()
    }
}

impl Default for SilenceQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use crate::alerting::evaluator::AlertState;
    use crate::alerting::rule::Severity;

    #[test]
    fn test_matcher_equal() {
        let matcher = Matcher::equal("host".into(), "server1".into());

        let alert = Alert {
            id: "test".into(),
            rule_id: "rule1".into(),
            state: AlertState::Firing,
            severity: Severity::Warning,
            labels: {
                let mut map = BTreeMap::new();
                map.insert("host".into(), "server1".into());
                map
            },
            annotations: BTreeMap::new(),
            value: None,
            threshold: None,
            started_at: core::time::Instant::now(),
            fired_at: None,
            resolved_at: None,
            fire_count: 0,
            reason: None,
        };

        let manager = SilenceManager::new();
        assert!(manager.matcher_matches(&alert, &matcher));
    }

    #[test]
    fn test_matcher_present() {
        let matcher = Matcher::present("host".into());

        let alert = Alert {
            id: "test".into(),
            rule_id: "rule1".into(),
            state: AlertState::Firing,
            severity: Severity::Warning,
            labels: {
                let mut map = BTreeMap::new();
                map.insert("host".into(), "server1".into());
                map
            },
            annotations: BTreeMap::new(),
            value: None,
            threshold: None,
            started_at: core::time::Instant::now(),
            fired_at: None,
            resolved_at: None,
            fire_count: 0,
            reason: None,
        };

        let manager = SilenceManager::new();
        assert!(manager.matcher_matches(&alert, &matcher));
    }

    #[test]
    fn test_silence_creation() {
        let silence = SilenceRule::with_duration(
            "silence-1".into(),
            "Test silence".into(),
            vec![Matcher::equal("alertname".into(), "TestAlert".into())],
            SilenceType::Manual,
            Duration::from_secs(3600),
            "admin".into(),
            "Testing silence".into(),
        );

        assert_eq!(silence.id, "silence-1");
        assert_eq!(silence.silence_type, SilenceType::Manual);
        assert!(silence.ends_at.is_some());
    }

    #[test]
    fn test_silence_manager() {
        let mut manager = SilenceManager::new();

        let silence = SilenceRule::with_duration(
            "silence-2".into(),
            "Test silence".into(),
            vec![Matcher::equal("host".into(), "server1".into())],
            SilenceType::Maintenance,
            Duration::from_secs(3600),
            "admin".into(),
            "Maintenance".into(),
        );

        manager.add_silence(silence).unwrap();
        assert_eq!(manager.silences().len(), 1);
    }

    #[test]
    fn test_inhibition_rule() {
        let rule = InhibitionRule::new(
            "inhibit-1".into(),
            vec![Matcher::equal("severity".into(), "critical".into())],
            vec![Matcher::equal("severity".into(), "warning".into())],
            {
                let mut set = BTreeSet::new();
                set.insert("host".into());
                set
            },
        );

        let mut manager = SilenceManager::new();
        manager.add_inhibition(rule).unwrap();
        assert_eq!(manager.inhibitions().len(), 1);
    }

    #[test]
    fn test_maintenance_window() {
        let window = MaintenanceWindow::with_duration(
            "maint-1".into(),
            "Server maintenance".into(),
            vec![Matcher::equal("host".into(), "server1".into())],
            Duration::from_secs(7200),
            MaintenanceType::Planned,
            "Scheduled maintenance".into(),
        );

        let mut manager = SilenceManager::new();
        manager.create_maintenance(window).unwrap();
        assert_eq!(manager.maintenance_windows().len(), 1);
    }

    #[test]
    fn test_labels_equal() {
        let alert1 = Alert {
            id: "1".into(),
            rule_id: "rule1".into(),
            state: AlertState::Firing,
            severity: Severity::Warning,
            labels: {
                let mut map = BTreeMap::new();
                map.insert("host".into(), "server1".into());
                map.insert("region".into(), "us-west".into());
                map
            },
            annotations: BTreeMap::new(),
            value: None,
            threshold: None,
            started_at: core::time::Instant::now(),
            fired_at: None,
            resolved_at: None,
            fire_count: 0,
            reason: None,
        };

        let alert2 = Alert {
            id: "2".into(),
            rule_id: "rule2".into(),
            state: AlertState::Firing,
            severity: Severity::Warning,
            labels: {
                let mut map = BTreeMap::new();
                map.insert("host".into(), "server1".into());
                map.insert("region".into(), "us-west".into());
                map
            },
            annotations: BTreeMap::new(),
            value: None,
            threshold: None,
            started_at: core::time::Instant::now(),
            fired_at: None,
            resolved_at: None,
            fire_count: 0,
            reason: None,
        };

        let manager = SilenceManager::new();
        let mut keys = BTreeSet::new();
        keys.insert("host".into());
        keys.insert("region".into());

        assert!(manager.labels_equal(&alert1, &alert2, &keys));
    }
}
