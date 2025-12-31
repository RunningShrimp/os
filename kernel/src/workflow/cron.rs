//! Cron expression parser and scheduler
//!
//! This module provides comprehensive cron expression parsing and scheduling
//! with support for the standard 6-field format (second to year).

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

/// Represents a parsed cron expression
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronExpression {
    /// Second field (0-59)
    pub second: CronField,
    /// Minute field (0-59)
    pub minute: CronField,
    /// Hour field (0-23)
    pub hour: CronField,
    /// Day of month (1-31)
    pub day_of_month: CronField,
    /// Month (1-12)
    pub month: CronField,
    /// Day of week (0-6, 0=Sunday)
    pub day_of_week: CronField,
}

/// A single field in a cron expression
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CronField {
    /// Match all values
    All,
    /// Match a specific value
    Value(u32),
    /// Match a range of values
    Range { min: u32, max: u32 },
    /// Match specific values in a list
    List(Vec<u32>),
    /// Match values with a step
    Step { base: StepBase, step: u32 },
}

/// Base for step expressions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepBase {
    All,
    Value(u32),
    Range { min: u32, max: u32 },
}

/// Errors that can occur during cron parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CronError {
    /// Invalid field count (expected 6)
    InvalidFieldCount { expected: usize, got: usize },
    /// Invalid value for field
    InvalidValue { field: String, value: String },
    /// Value out of range
    OutOfRange { field: String, value: i64, min: u32, max: u32 },
    /// Invalid syntax
    InvalidSyntax { error: String },
    /// Invalid step value
    InvalidStep { step: String },
    /// Day of week and day of month both restricted
    BothDaySpecified,
    /// Empty expression
    EmptyExpression,
}

impl fmt::Display for CronError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFieldCount { expected, got } => {
                write!(f, "Invalid field count: expected {}, got {}", expected, got)
            }
            Self::InvalidValue { field, value } => {
                write!(f, "Invalid value for field {}: '{}'", field, value)
            }
            Self::OutOfRange { field, value, min, max: _ } => {
                write!(f, "Value {} for field {} out of range ({})", value, field, min)
            }
            Self::InvalidSyntax { error } => {
                write!(f, "Invalid syntax: {}", error)
            }
            Self::InvalidStep { step } => {
                write!(f, "Invalid step value: {}", step)
            }
            Self::BothDaySpecified => {
                write!(f, "Cannot specify both day of month and day of week")
            }
            Self::EmptyExpression => {
                write!(f, "Empty cron expression")
            }
        }
    }
}

/// Result type for cron operations
pub type CronResult<T> = Result<T, CronError>;

/// Constraints for each cron field
#[derive(Debug, Clone, Copy)]
struct FieldConstraints {
    min: u32,
    max: u32,
}

/// Field definitions
const SECOND_CONSTRAINTS: FieldConstraints = FieldConstraints { min: 0, max: 59 };
const MINUTE_CONSTRAINTS: FieldConstraints = FieldConstraints { min: 0, max: 59 };
const HOUR_CONSTRAINTS: FieldConstraints = FieldConstraints { min: 0, max: 23 };
const DAY_CONSTRAINTS: FieldConstraints = FieldConstraints { min: 1, max: 31 };
const MONTH_CONSTRAINTS: FieldConstraints = FieldConstraints { min: 1, max: 12 };
const DOW_CONSTRAINTS: FieldConstraints = FieldConstraints { min: 0, max: 6 };

/// Named values for months
const MONTH_NAMES: &[(&str, u32)] = &[
    ("JAN", 1), ("FEB", 2), ("MAR", 3), ("APR", 4),
    ("MAY", 5), ("JUN", 6), ("JUL", 7), ("AUG", 8),
    ("SEP", 9), ("OCT", 10), ("NOV", 11), ("DEC", 12),
];

/// Named values for days of week
const DOW_NAMES: &[(&str, u32)] = &[
    ("SUN", 0), ("MON", 1), ("TUE", 2), ("WED", 3),
    ("THU", 4), ("FRI", 5), ("SAT", 6),
];

impl CronExpression {
    /// Parse a cron expression string
    ///
    /// Format: "second minute hour day month dow"
    /// Example: "0 * * * * *" - Every minute
    /// Example: "0 0 * * * *" - Every hour
    /// Example: "0 0 0 * * *" - Every day at midnight
    pub fn parse(expression: &str) -> CronResult<Self> {
        if expression.trim().is_empty() {
            return Err(CronError::EmptyExpression);
        }

        let fields: Vec<&str> = expression.split_whitespace().collect();
        if fields.len() != 6 {
            return Err(CronError::InvalidFieldCount {
                expected: 6,
                got: fields.len(),
            });
        }

        let second = Self::parse_field(fields[0], SECOND_CONSTRAINTS, None)?;
        let minute = Self::parse_field(fields[1], MINUTE_CONSTRAINTS, None)?;
        let hour = Self::parse_field(fields[2], HOUR_CONSTRAINTS, None)?;
        let day_of_month = Self::parse_field(fields[3], DAY_CONSTRAINTS, None)?;
        let month = Self::parse_field(fields[4], MONTH_CONSTRAINTS, Some(MONTH_NAMES))?;
        let day_of_week = Self::parse_field(fields[5], DOW_CONSTRAINTS, Some(DOW_NAMES))?;

        // Check if both day fields are restricted (not * or ?)
        let day_restricted = !matches!(day_of_month, CronField::All);
        let dow_restricted = !matches!(day_of_week, CronField::All);

        if day_restricted && dow_restricted {
            return Err(CronError::BothDaySpecified);
        }

        Ok(Self {
            second,
            minute,
            hour,
            day_of_month,
            month,
            day_of_week,
        })
    }

    /// Parse a single field
    fn parse_field(
        field: &str,
        constraints: FieldConstraints,
        names: Option<&'static [(&'static str, u32)]>,
    ) -> CronResult<CronField> {
        let field = field.trim();

        // Handle wildcard
        if field == "*" || field == "?" {
            return Ok(CronField::All);
        }

        // Handle named values
        if let Some(names) = names {
            let upper_field = field.to_uppercase();
            for (name, value) in names {
                if upper_field == *name {
                    return Ok(CronField::Value(*value));
                }
            }
        }

        // Handle step expressions
        if field.contains('/') {
            return Self::parse_step(field, constraints, names);
        }

        // Handle ranges
        if field.contains('-') {
            let parts: Vec<&str> = field.split('-').collect();
            if parts.len() != 2 {
                return Err(CronError::InvalidSyntax {
                    error: format!("Invalid range: {}", field),
                });
            }

            let min = Self::parse_value(parts[0], constraints, names)?;
            let max = Self::parse_value(parts[1], constraints, names)?;

            if min > max {
                return Err(CronError::InvalidSyntax {
                    error: format!("Range min > max: {} > {}", min, max),
                });
            }

            return Ok(CronField::Range { min, max });
        }

        // Handle lists
        if field.contains(',') {
            let values: Vec<&str> = field.split(',').collect();
            let mut parsed_values = Vec::new();
            for val in values {
                parsed_values.push(Self::parse_value(val, constraints, names)?);
            }
            parsed_values.sort();
            parsed_values.dedup();
            return Ok(CronField::List(parsed_values));
        }

        // Handle single value
        let value = Self::parse_value(field, constraints, names)?;
        Ok(CronField::Value(value))
    }

    /// Parse a single value
    fn parse_value(
        value: &str,
        constraints: FieldConstraints,
        names: Option<&'static [(&'static str, u32)]>,
    ) -> CronResult<u32> {
        let value = value.trim();

        // Try named value first
        if let Some(names) = names {
            let upper_value = value.to_uppercase();
            for (name, num) in names.iter() {
                if upper_value == *name {
                    return Ok(*num);
                }
            }
        }

        // Parse as number
        value.parse::<i64>()
            .map_err(|_| CronError::InvalidValue {
                field: "unknown".to_string(),
                value: value.to_string(),
            })
            .and_then(|v| {
                if v < constraints.min as i64 || v > constraints.max as i64 {
                    Err(CronError::OutOfRange {
                        field: "field".to_string(),
                        value: v,
                        min: constraints.min,
                        max: constraints.max,
                    })
                } else {
                    Ok(v as u32)
                }
            })
    }

    /// Parse step expression
    fn parse_step(
        field: &str,
        constraints: FieldConstraints,
        names: Option<&'static [(&'static str, u32)]>,
    ) -> CronResult<CronField> {
        let parts: Vec<&str> = field.split('/').collect();
        if parts.len() != 2 {
            return Err(CronError::InvalidSyntax {
                error: format!("Invalid step expression: {}", field),
            });
        }

        let step = Self::parse_value(parts[1], constraints, names)?;
        if step == 0 {
            return Err(CronError::InvalidStep {
                step: "Step cannot be zero".to_string(),
            });
        }

        let base = if parts[0] == "*" || parts[0] == "" {
            StepBase::All
        } else if parts[0].contains('-') {
            let range_parts: Vec<&str> = parts[0].split('-').collect();
            let min = Self::parse_value(range_parts[0], constraints, names)?;
            let max = Self::parse_value(range_parts[1], constraints, names)?;
            StepBase::Range { min, max }
        } else {
            let value = Self::parse_value(parts[0], constraints, names)?;
            StepBase::Value(value)
        };

        Ok(CronField::Step { base, step })
    }

    /// Check if a value matches the field
    pub fn matches_field(field: &CronField, value: u32) -> bool {
        match field {
            CronField::All => true,
            CronField::Value(v) => *v == value,
            CronField::Range { min, max } => value >= *min && value <= *max,
            CronField::List(values) => values.contains(&value),
            CronField::Step { base, step } => {
                match base {
                    StepBase::All => value % step == 0,
                    StepBase::Range { min, max } => {
                        value >= *min && value <= *max && (value - min) % step == 0
                    }
                    StepBase::Value(v) => value >= *v && (value - v) % step == 0,
                }
            }
        }
    }

    /// Check if a timestamp matches this cron expression
    pub fn matches(&self, second: u32, minute: u32, hour: u32,
                   day: u32, month: u32, dow: u32) -> bool {
        Self::matches_field(&self.second, second)
            && Self::matches_field(&self.minute, minute)
            && Self::matches_field(&self.hour, hour)
            && Self::matches_field(&self.month, month)
            && (Self::matches_field(&self.day_of_month, day)
                || Self::matches_field(&self.day_of_week, dow))
    }
}

impl fmt::Display for CronExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {} {} {} {}",
            Self::field_to_string(&self.second),
            Self::field_to_string(&self.minute),
            Self::field_to_string(&self.hour),
            Self::field_to_string(&self.day_of_month),
            Self::field_to_string(&self.month),
            Self::field_to_string(&self.day_of_week),
        )
    }
}

impl CronExpression {
    fn field_to_string(field: &CronField) -> String {
        match field {
            CronField::All => "*".to_string(),
            CronField::Value(v) => v.to_string(),
            CronField::Range { min, max } => format!("{}-{}", min, max),
            CronField::List(values) => {
                values.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            }
            CronField::Step { base, step } => {
                let base_str = match base {
                    StepBase::All => "*".to_string(),
                    StepBase::Value(v) => v.to_string(),
                    StepBase::Range { min, max } => format!("{}-{}", min, max),
                };
                format!("{}/{}", base_str, step)
            }
        }
    }
}

/// Cron scheduler for calculating next execution times
#[derive(Clone)]
pub struct CronScheduler {
    /// The parsed cron expression
    expression: CronExpression,
}

impl CronScheduler {
    /// Create a new scheduler from a cron expression
    pub fn new(expression: &str) -> CronResult<Self> {
        Ok(Self {
            expression: CronExpression::parse(expression)?,
        })
    }

    /// Create a scheduler from a parsed expression
    pub fn from_expression(expression: CronExpression) -> Self {
        Self { expression }
    }

    /// Get the cron expression
    pub fn expression(&self) -> &CronExpression {
        &self.expression
    }

    /// Calculate the next execution time after a given timestamp
    ///
    /// Parameters:
    /// - current_sec, current_min, current_hour: Current time
    /// - current_day, current_month, current_year: Current date
    ///
    /// Returns the next execution timestamp as (sec, min, hour, day, month, year)
    pub fn next_execution(
        &self,
        current_sec: u32,
        current_min: u32,
        current_hour: u32,
        current_day: u32,
        current_month: u32,
        current_year: u32,
    ) -> Option<(u32, u32, u32, u32, u32, u32)> {
        // Start from the next second
        let mut sec = current_sec + 1;
        let mut min = current_min;
        let mut hour = current_hour;
        let mut day = current_day;
        let mut month = current_month;
        let mut year = current_year;

        // Try to find the next matching time
        // Limit iterations to prevent infinite loops
        for _ in 0..4 * 365 * 24 * 60 {  // 4 years max
            // Normalize seconds
            if sec > 59 {
                sec = 0;
                min += 1;
            }

            // Normalize minutes
            if min > 59 {
                min = 0;
                hour += 1;
            }

            // Normalize hours
            if hour > 23 {
                hour = 0;
                day += 1;
            }

            // Normalize days (simplified - doesn't handle all month lengths)
            if day > 31 {
                day = 1;
                month += 1;
            }

            // Normalize months
            if month > 12 {
                month = 1;
                year += 1;
            }

            // Check if current time matches
            // Calculate day of week (simplified Zeller's congruence)
            let dow = Self::day_of_week(day, month, year);

            if self.expression.matches(sec, min, hour, day, month, dow) {
                return Some((sec, min, hour, day, month, year));
            }

            // Increment second
            sec += 1;
        }

        None
    }

    /// Calculate day of week using Zeller's congruence
    /// Returns 0=Sunday, 1=Monday, ..., 6=Saturday
    fn day_of_week(day: u32, month: u32, year: u32) -> u32 {
        let mut m = month as i64;
        let mut y = year as i64;

        if m < 3 {
            m += 12;
            y -= 1;
        }

        let k = day as i64;
        let d = y % 100;
        let c = y / 100;

        let h = (k + (13 * (m + 1)) / 5 + d + d / 4 + c / 4 + 5 * c) % 7;

        // Convert from Saturday=0 to Sunday=0
        ((h + 5) % 7) as u32
    }

    /// Check if a specific time matches the schedule
    pub fn is_scheduled(&self, sec: u32, min: u32, hour: u32,
                        day: u32, month: u32, dow: u32) -> bool {
        self.expression.matches(sec, min, hour, day, month, dow)
    }

    /// Get all matching seconds in a minute
    pub fn matching_seconds(&self) -> Vec<u32> {
        self.matching_values(&self.expression.second, 0, 59)
    }

    /// Get all matching minutes in an hour
    pub fn matching_minutes(&self) -> Vec<u32> {
        self.matching_values(&self.expression.minute, 0, 59)
    }

    /// Get all matching hours in a day
    pub fn matching_hours(&self) -> Vec<u32> {
        self.matching_values(&self.expression.hour, 0, 23)
    }

    /// Get all matching days in a month
    pub fn matching_days(&self) -> Vec<u32> {
        self.matching_values(&self.expression.day_of_month, 1, 31)
    }

    /// Get all matching months
    pub fn matching_months(&self) -> Vec<u32> {
        self.matching_values(&self.expression.month, 1, 12)
    }

    /// Get all matching days of week
    pub fn matching_dow(&self) -> Vec<u32> {
        self.matching_values(&self.expression.day_of_week, 0, 6)
    }

    /// Get matching values for a field
    fn matching_values(&self, field: &CronField, min: u32, max: u32) -> Vec<u32> {
        let mut values = Vec::new();
        for i in min..=max {
            if CronExpression::matches_field(field, i) {
                values.push(i);
            }
        }
        values
    }

    /// Calculate the interval between executions in seconds (approximate)
    pub fn interval_seconds(&self) -> Option<u64> {
        let seconds = self.matching_seconds();
        let minutes = self.matching_minutes();
        let hours = self.matching_hours();

        if !seconds.is_empty() && seconds.len() < 60 {
            // Check if it's a regular interval
            if seconds.len() == 1 {
                return Some(60);  // Once per minute
            }
            // Calculate smallest interval
            let mut min_interval = u64::MAX;
            for i in 1..seconds.len() {
                let interval = (seconds[i] - seconds[i - 1]) as u64;
                min_interval = min_interval.min(interval);
            }
            return Some(min_interval);
        }

        if !minutes.is_empty() && minutes.len() < 60 {
            if minutes.len() == 1 {
                return Some(3600);  // Once per hour
            }
        }

        if !hours.is_empty() && hours.len() < 24 {
            if hours.len() == 1 {
                return Some(86400);  // Once per day
            }
        }

        None
    }
}

/// Validate a cron expression without parsing
pub fn validate_cron(expression: &str) -> CronResult<()> {
    CronExpression::parse(expression)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let cron = CronExpression::parse("0 * * * * *").unwrap();
        assert!(matches!(cron.second, CronField::Value(0)));
        assert!(matches!(cron.minute, CronField::All));
    }

    #[test]
    fn test_parse_range() {
        let cron = CronExpression::parse("0 0 9-17 * * *").unwrap();
        assert!(matches!(cron.hour, CronField::Range { min: 9, max: 17 }));
    }

    #[test]
    fn test_parse_list() {
        let cron = CronExpression::parse("0 0 9,12,17 * * *").unwrap();
        assert!(matches!(cron.hour, CronField::List(_)));
    }

    #[test]
    fn test_parse_step() {
        let cron = CronExpression::parse("0 */5 * * * *").unwrap();
        assert!(matches!(cron.minute, CronField::Step { .. }));
    }

    #[test]
    fn test_parse_names() {
        let cron = CronExpression::parse("0 0 12 * * MON").unwrap();
        assert!(matches!(cron.day_of_week, CronField::Value(1)));
    }

    #[test]
    fn test_invalid_field_count() {
        let result = CronExpression::parse("* * * * *");
        assert!(matches!(result, Err(CronError::InvalidFieldCount { .. })));
    }

    #[test]
    fn test_both_days_specified() {
        let result = CronExpression::parse("0 0 12 1 * *");
        assert!(matches!(result, Err(CronError::BothDaySpecified)));
    }

    #[test]
    fn test_matches() {
        let cron = CronExpression::parse("0 30 9 * * *").unwrap();
        assert!(cron.matches(0, 30, 9, 1, 1, 0));
        assert!(!cron.matches(1, 30, 9, 1, 1, 0));
    }

    #[test]
    fn test_scheduler() {
        let scheduler = CronScheduler::new("0 30 9 * * *").unwrap();
        let next = scheduler.next_execution(0, 0, 0, 1, 1, 2024);
        assert!(next.is_some());
    }

    #[test]
    fn test_matching_values() {
        let scheduler = CronScheduler::new("0,15,30,45 * * * * *").unwrap();
        let seconds = scheduler.matching_seconds();
        assert_eq!(seconds, vec![0, 15, 30, 45]);
    }
}
