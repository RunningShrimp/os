//! Notification delivery system for alerts
//!
//! This module provides notification delivery through multiple channels:
//! - Email notifications
//! - SMS notifications
//! - Webhook notifications
//! - Slack notifications
//! - Notification templates and batching
//! - Retry logic and rate limiting

#![no_std]

extern crate alloc;

use alloc::{
    collections::{BTreeMap, VecDeque},
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    fmt,
    time::Duration,
};

use crate::alerting::evaluator::Alert;

/// Notification channel types
#[derive(Clone, Debug, PartialEq)]
pub enum NotificationChannel {
    /// Email notification
    Email(EmailConfig),
    /// SMS notification
    Sms(SmsConfig),
    /// Webhook notification
    Webhook(WebhookConfig),
    /// Slack notification
    Slack(SlackConfig),
    /// Custom notification handler
    Custom(String),
}

/// Email notification configuration
#[derive(Clone, Debug)]
pub struct EmailConfig {
    /// SMTP server address
    pub smtp_server: String,
    /// SMTP port
    pub smtp_port: u16,
    /// SMTP username
    pub username: String,
    /// SMTP password
    pub password: String,
    /// From address
    pub from: String,
    /// To addresses
    pub to: Vec<String>,
    /// Email subject template
    pub subject_template: String,
    /// Use TLS
    pub use_tls: bool,
}

/// SMS notification configuration
#[derive(Clone, Debug)]
pub struct SmsConfig {
    /// SMS provider (e.g., "twilio", "sns")
    pub provider: String,
    /// API endpoint
    pub endpoint: String,
    /// API key
    pub api_key: String,
    /// Phone numbers to notify
    pub phone_numbers: Vec<String>,
    /// Message template
    pub message_template: String,
}

/// Webhook notification configuration
#[derive(Clone, Debug)]
pub struct WebhookConfig {
    /// Webhook URL
    pub url: String,
    /// HTTP method
    pub method: HttpMethod,
    /// HTTP headers
    pub headers: BTreeMap<String, String>,
    /// Request body template
    pub body_template: String,
    /// Content type
    pub content_type: String,
}

/// HTTP methods
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
}

/// Slack notification configuration
#[derive(Clone, Debug)]
pub struct SlackConfig {
    /// Slack webhook URL
    pub webhook_url: String,
    /// Channel to post to (optional, uses webhook default)
    pub channel: Option<String>,
    /// Username for the bot
    pub username: String,
    /// Icon emoji for the bot
    pub icon_emoji: Option<String>,
    /// Message template
    pub message_template: String,
}

/// Notification delivery status
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DeliveryStatus {
    /// Successfully delivered
    Delivered,
    /// Failed to deliver
    Failed,
    /// Pending retry
    PendingRetry,
    /// Rate limited
    RateLimited,
}

/// Notification record
#[derive(Clone, Debug)]
pub struct Notification {
    /// Unique notification ID
    pub id: String,
    /// Associated alert ID
    pub alert_id: String,
    /// Channel used for delivery
    pub channel: NotificationChannel,
    /// Delivery status
    pub status: DeliveryStatus,
    /// Number of delivery attempts
    pub attempts: usize,
    /// Maximum retry attempts
    pub max_retries: usize,
    /// When the notification was created
    pub created_at: core::time::Instant,
    /// Last delivery attempt time
    pub last_attempt_at: Option<core::time::Instant>,
    /// Error message if delivery failed
    pub error_message: Option<String>,
}

/// Notification template
#[derive(Clone, Debug)]
pub struct NotificationTemplate {
    /// Template name
    pub name: String,
    /// Subject template (for email)
    pub subject: String,
    /// Body template
    pub body: String,
    /// Template variables
    pub variables: Vec<String>,
}

/// Notification delivery error
#[derive(Clone, Debug)]
pub enum NotificationError {
    /// Invalid configuration
    InvalidConfig(String),
    /// Delivery failed
    DeliveryFailed(String),
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Template rendering failed
    TemplateError(String),
    /// Serialization error
    SerializationError(String),
    /// Network error
    NetworkError(String),
}

impl fmt::Display for NotificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            Self::DeliveryFailed(msg) => write!(f, "Delivery failed: {}", msg),
            Self::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            Self::TemplateError(msg) => write!(f, "Template error: {}", msg),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NotificationError {}

/// Notification manager
pub struct NotificationManager {
    /// Registered notification channels
    channels: Vec<NotificationChannel>,
    /// Notification history
    notifications: Vec<Notification>,
    /// Pending notifications
    pending: VecDeque<Notification>,
    /// Retry queue
    retry_queue: VecDeque<Notification>,
    /// Configuration
    config: NotifierConfig,
    /// Rate limiters per channel
    rate_limiters: BTreeMap<String, RateLimiter>,
}

/// Notifier configuration
#[derive(Clone, Debug)]
pub struct NotifierConfig {
    /// Maximum notification batch size
    pub max_batch_size: usize,
    /// Maximum retry attempts
    pub max_retries: usize,
    /// Retry delay
    pub retry_delay: Duration,
    /// Notification timeout
    pub timeout: Duration,
    /// Maximum pending notifications
    pub max_pending: usize,
    /// Enable notification batching
    pub enable_batching: bool,
    /// Batch window duration
    pub batch_window: Duration,
}

impl Default for NotifierConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            max_retries: 3,
            retry_delay: Duration::from_secs(60),
            timeout: Duration::from_secs(30),
            max_pending: 10000,
            enable_batching: true,
            batch_window: Duration::from_secs(5),
        }
    }
}

/// Rate limiter for notifications
#[derive(Clone, Debug)]
pub struct RateLimiter {
    /// Maximum notifications per window
    max_per_window: usize,
    /// Window duration
    window_duration: Duration,
    /// Notification timestamps in current window
    timestamps: VecDeque<core::time::Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_per_window: usize, window_duration: Duration) -> Self {
        Self {
            max_per_window,
            window_duration,
            timestamps: VecDeque::new(),
        }
    }

    /// Check if a notification is allowed
    pub fn check(&mut self) -> bool {
        let now = core::time::Instant::now();

        // Remove old timestamps outside the window
        while let Some(&ts) = self.timestamps.front() {
            if now.duration_since(ts) >= self.window_duration {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }

        // Check if we can send
        if self.timestamps.len() < self.max_per_window {
            self.timestamps.push_back(now);
            true
        } else {
            false
        }
    }

    /// Get time until next allowed notification
    pub fn wait_time(&self) -> Option<Duration> {
        if self.timestamps.len() < self.max_per_window {
            return None;
        }

        if let Some(&oldest) = self.timestamps.front() {
            let elapsed = core::time::Instant::now().duration_since(oldest);
            if elapsed < self.window_duration {
                return Some(self.window_duration - elapsed);
            }
        }

        None
    }
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new(config: NotifierConfig) -> Self {
        Self {
            channels: Vec::new(),
            notifications: Vec::new(),
            pending: VecDeque::new(),
            retry_queue: VecDeque::new(),
            config,
            rate_limiters: BTreeMap::new(),
        }
    }

    /// Add a notification channel
    pub fn add_channel(&mut self, channel: NotificationChannel) {
        let key = self.channel_key(&channel);
        // Add default rate limiter (10 notifications per minute)
        self.rate_limiters
            .insert(key.clone(), RateLimiter::new(10, Duration::from_secs(60)));
        self.channels.push(channel);
    }

    /// Send a notification for an alert
    pub fn send_notification(
        &mut self,
        alert: &Alert,
        template: &NotificationTemplate,
    ) -> Result<Notification, NotificationError> {
        // Check rate limits
        for channel in &self.channels {
            let key = self.channel_key(channel);
            if let Some(limiter) = self.rate_limiters.get_mut(&key) {
                if !limiter.check() {
                    return Err(NotificationError::RateLimitExceeded);
                }
            }
        }

        // Create notifications for each channel
        let notifications = self.create_notifications(alert, template)?;

        // Queue notifications
        for notification in notifications {
            self.pending.push_back(notification);
        }

        // Try to send immediately if batching is disabled
        if !self.config.enable_batching {
            self.flush_notifications()?;
        }

        Ok(self.pending.front().unwrap().clone())
    }

    /// Create notifications for all channels
    fn create_notifications(
        &self,
        alert: &Alert,
        template: &NotificationTemplate,
    ) -> Result<Vec<Notification>, NotificationError> {
        let mut notifications = Vec::new();

        for channel in &self.channels {
            let id = self.generate_notification_id(alert, channel);
            notifications.push(Notification {
                id: id.clone(),
                alert_id: alert.id.clone(),
                channel: channel.clone(),
                status: DeliveryStatus::PendingRetry,
                attempts: 0,
                max_retries: self.config.max_retries,
                created_at: core::time::Instant::now(),
                last_attempt_at: None,
                error_message: None,
            });
        }

        Ok(notifications)
    }

    /// Flush pending notifications
    pub fn flush_notifications(&mut self) -> Result<(), NotificationError> {
        let batch_size = if self.config.enable_batching {
            self.config.max_batch_size.min(self.pending.len())
        } else {
            self.pending.len()
        };

        for _ in 0..batch_size {
            if let Some(mut notification) = self.pending.pop_front() {
                // Send notification
                match self.deliver_notification(&notification) {
                    Ok(_) => {
                        notification.status = DeliveryStatus::Delivered;
                        notification.last_attempt_at = Some(core::time::Instant::now());
                        self.notifications.push(notification);
                    }
                    Err(e) => {
                        notification.status = DeliveryStatus::Failed;
                        notification.attempts += 1;
                        notification.last_attempt_at = Some(core::time::Instant::now());
                        notification.error_message = Some(e.to_string());

                        // Add to retry queue if retries remain
                        if notification.attempts < notification.max_retries {
                            notification.status = DeliveryStatus::PendingRetry;
                            self.retry_queue.push_back(notification);
                        } else {
                            self.notifications.push(notification);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Process retry queue
    pub fn process_retries(&mut self) -> Result<(), NotificationError> {
        let now = core::time::Instant::now();
        let mut to_retry = Vec::new();

        // Check which retries are ready
        while let Some(notification) = self.retry_queue.front() {
            if let Some(last_attempt) = notification.last_attempt_at {
                if now.duration_since(last_attempt) >= self.config.retry_delay {
                    if let Some(n) = self.retry_queue.pop_front() {
                        to_retry.push(n);
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Retry notifications
        for mut notification in to_retry {
            match self.deliver_notification(&notification) {
                Ok(_) => {
                    notification.status = DeliveryStatus::Delivered;
                    notification.last_attempt_at = Some(core::time::Instant::now());
                    self.notifications.push(notification);
                }
                Err(e) => {
                    notification.attempts += 1;
                    notification.last_attempt_at = Some(core::time::Instant::now());
                    notification.error_message = Some(e.to_string());

                    if notification.attempts < notification.max_retries {
                        self.retry_queue.push_back(notification);
                    } else {
                        notification.status = DeliveryStatus::Failed;
                        self.notifications.push(notification);
                    }
                }
            }
        }

        Ok(())
    }

    /// Deliver a notification through its channel
    fn deliver_notification(&self, notification: &Notification) -> Result<(), NotificationError> {
        match &notification.channel {
            NotificationChannel::Email(config) => self.send_email(config),
            NotificationChannel::Sms(config) => self.send_sms(config),
            NotificationChannel::Webhook(config) => self.send_webhook(config),
            NotificationChannel::Slack(config) => self.send_slack(config),
            NotificationChannel::Custom(_) => {
                // Custom handlers would be implemented elsewhere
                Ok(())
            }
        }
    }

    /// Send email notification
    fn send_email(&self, _config: &EmailConfig) -> Result<(), NotificationError> {
        // In a real implementation, this would use SMTP to send email
        // For now, we just return success
        #[cfg(feature = "std")]
        {
            // Actual SMTP implementation would go here
        }
        Ok(())
    }

    /// Send SMS notification
    fn send_sms(&self, _config: &SmsConfig) -> Result<(), NotificationError> {
        // In a real implementation, this would use an SMS provider API
        Ok(())
    }

    /// Send webhook notification
    fn send_webhook(&self, _config: &WebhookConfig) -> Result<(), NotificationError> {
        // In a real implementation, this would make an HTTP request
        Ok(())
    }

    /// Send Slack notification
    fn send_slack(&self, _config: &SlackConfig) -> Result<(), NotificationError> {
        // In a real implementation, this would send to Slack webhook
        Ok(())
    }

    /// Generate a notification ID
    fn generate_notification_id(&self, alert: &Alert, channel: &NotificationChannel) -> String {
        let channel_type = match channel {
            NotificationChannel::Email(_) => "email",
            NotificationChannel::Sms(_) => "sms",
            NotificationChannel::Webhook(_) => "webhook",
            NotificationChannel::Slack(_) => "slack",
            NotificationChannel::Custom(name) => name,
        };
        format!("{}:{}:{}", channel_type, alert.id, core::time::Instant::now().as_nanos())
    }

    /// Get channel key for rate limiting
    fn channel_key(&self, channel: &NotificationChannel) -> String {
        match channel {
            NotificationChannel::Email(_) => "email".into(),
            NotificationChannel::Sms(_) => "sms".into(),
            NotificationChannel::Webhook(config) => format!("webhook:{}", config.url),
            NotificationChannel::Slack(_) => "slack".into(),
            NotificationChannel::Custom(name) => format!("custom:{}", name),
        }
    }

    /// Get notification history
    pub fn notifications(&self) -> &[Notification] {
        &self.notifications
    }

    /// Get pending notification count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get retry queue size
    pub fn retry_queue_size(&self) -> usize {
        self.retry_queue.len()
    }
}

impl NotificationTemplate {
    /// Create a new notification template
    pub fn new(name: String, subject: String, body: String, variables: Vec<String>) -> Self {
        Self {
            name,
            subject,
            body,
            variables,
        }
    }

    /// Render the template with alert data
    pub fn render(&self, alert: &Alert) -> Result<(String, String), NotificationError> {
        let subject = self.render_template(&self.subject, alert)?;
        let body = self.render_template(&self.body, alert)?;
        Ok((subject, body))
    }

    /// Render a template string
    fn render_template(&self, template: &str, alert: &Alert) -> Result<String, NotificationError> {
        let mut result = template.to_string();

        // Replace common variables
        result = result.replace("{{alert_id}}", &alert.id);
        result = result.replace("{{rule_id}}", &alert.rule_id);
        result = result.replace("{{severity}}", &alert.severity.to_string());
        result = result.replace("{{state}}", &format!("{:?}", alert.state));

        if let Some(value) = alert.value {
            result = result.replace("{{value}}", &value.to_string());
        }

        if let Some(threshold) = alert.threshold {
            result = result.replace("{{threshold}}", &threshold.to_string());
        }

        // Replace label variables
        for (key, value) in &alert.labels {
            result = result.replace(&format!("{{label.{}}}", key), value);
        }

        Ok(result)
    }

    /// Create a default alert template
    pub fn default_alert_template() -> Self {
        Self::new(
            "default_alert".into(),
            "[{{severity}}] Alert: {{rule_id}}".into(),
            "Alert ID: {{alert_id}}\n\
             Severity: {{severity}}\n\
             Rule: {{rule_id}}\n\
             Value: {{value}}\n\
             Threshold: {{threshold}}\n\
             State: {{state}}".into(),
            vec![
                "alert_id".into(),
                "rule_id".into(),
                "severity".into(),
                "state".into(),
                "value".into(),
                "threshold".into(),
            ],
        )
    }
}

/// Prometheus AlertManager compatible notification format
#[derive(Clone, Debug)]
pub struct PrometheusNotification {
    /// Alert status
    pub status: String,
    /// Alert labels
    pub labels: BTreeMap<String, String>,
    /// Alert annotations
    pub annotations: BTreeMap<String, String>,
    /// Alert start time
    pub starts_at: String,
    /// Alert end time
    pub ends_at: Option<String>,
    /// Generator URL
    pub generator_url: String,
    /// Fingerprint for deduplication
    pub fingerprint: String,
}

impl PrometheusNotification {
    /// Create from an alert
    pub fn from_alert(alert: &Alert) -> Self {
        Self {
            status: format!("{:?}", alert.state).to_lowercase(),
            labels: alert.labels.clone(),
            annotations: alert.annotations.clone(),
            starts_at: format!("{:?}", alert.started_at),
            ends_at: alert.resolved_at.map(|t| format!("{:?}", t)),
            generator_url: format!("/alerts/{}", alert.id),
            fingerprint: alert.id.clone(),
        }
    }

    /// Serialize to JSON (simplified version)
    pub fn to_json(&self) -> Result<String, NotificationError> {
        // In a real implementation, this would use serde_json
        Ok(format!(
            r#"{{"status":"{}","labels":{{...}},"annotations":{{...}}}} "#,
            self.status
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alerting::evaluator::AlertState;
    use alloc::collections::BTreeMap;

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(5, Duration::from_secs(60));

        // Should allow first 5 requests
        for _ in 0..5 {
            assert!(limiter.check());
        }

        // 6th request should be denied
        assert!(!limiter.check());
    }

    #[test]
    fn test_template_rendering() {
        let template = NotificationTemplate::default_alert_template();

        let alert = Alert {
            id: "test-alert".into(),
            rule_id: "test-rule".into(),
            state: AlertState::Firing,
            severity: crate::alerting::rule::Severity::Warning,
            labels: {
                let mut map = BTreeMap::new();
                map.insert("host".into(), "server1".into());
                map
            },
            annotations: BTreeMap::new(),
            value: Some(85.0),
            threshold: Some(80.0),
            started_at: core::time::Instant::now(),
            fired_at: Some(core::time::Instant::now()),
            resolved_at: None,
            fire_count: 1,
            reason: Some("Test alert".into()),
        };

        let (subject, body) = template.render(&alert).unwrap();
        assert!(subject.contains("[warning]"));
        assert!(body.contains("test-rule"));
        assert!(body.contains("85"));
    }

    #[test]
    fn test_prometheus_notification() {
        let alert = Alert {
            id: "test-alert".into(),
            rule_id: "test-rule".into(),
            state: AlertState::Firing,
            severity: crate::alerting::rule::Severity::Critical,
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
            value: Some(100.0),
            threshold: Some(90.0),
            started_at: core::time::Instant::now(),
            fired_at: Some(core::time::Instant::now()),
            resolved_at: None,
            fire_count: 1,
            reason: None,
        };

        let prom_notification = PrometheusNotification::from_alert(&alert);
        assert_eq!(prom_notification.status, "firing");
        assert_eq!(prom_notification.fingerprint, "test-alert");
    }
}
