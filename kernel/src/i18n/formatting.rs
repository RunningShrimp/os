//! Locale-Aware Formatting
//!
//! This module implements locale-aware formatting for dates, times, numbers,
//! currencies, and other values:
//! - Date and time formatting
//! - Number formatting with grouping
//! - Currency formatting
//! - Percentage formatting
//! - Custom format patterns
//!
//! ## Features
//!
//! - **Date/Time**: Multiple formats (short, medium, long, full)
//! - **Numbers**: Locale-specific grouping and decimal separators
//! - **Currency**: Symbol placement and precision
//! - **Percentages**: Localized percentage formatting
//! - **Patterns**: Custom format pattern support

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// Formatting Constants
// ============================================================================

/// Default date format
pub const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d";

/// Default time format
pub const DEFAULT_TIME_FORMAT: &str = "%H:%M:%S";

/// Default datetime format
pub const DEFAULT_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// Default number of decimal places for currency
pub const DEFAULT_CURRENCY_DECIMALS: u8 = 2;

// ============================================================================
// Date and Time Formatting
// ============================================================================

/// Date format style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateStyle {
    /// Short format (e.g., "12/31/2024")
    Short,

    /// Medium format (e.g., "Dec 31, 2024")
    Medium,

    /// Long format (e.g., "December 31, 2024")
    Long,

    /// Full format (e.g., "Tuesday, December 31, 2024")
    Full,

    /// Custom format with pattern
    Custom(&'static str),
}

/// Time format style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeStyle {
    /// Short format (e.g., "2:30 PM")
    Short,

    /// Medium format (e.g., "2:30:45 PM")
    Medium,

    /// Long format (e.g., "2:30:45 PM EST")
    Long,

    /// Full format (e.g., "2:30:45 PM Eastern Standard Time")
    Full,

    /// Custom format with pattern
    Custom(&'static str),
}

/// Date and time components
#[derive(Debug, Clone, Copy)]
pub struct DateTime {
    /// Year
    pub year: u32,

    /// Month (1-12)
    pub month: u8,

    /// Day (1-31)
    pub day: u8,

    /// Hour (0-23)
    pub hour: u8,

    /// Minute (0-59)
    pub minute: u8,

    /// Second (0-59)
    pub second: u8,

    /// Nanoseconds (0-999999999)
    pub nanosecond: u32,

    /// Day of week (0=Sunday, 6=Saturday)
    pub day_of_week: u8,

    /// Day of year (1-366)
    pub day_of_year: u16,
}

impl DateTime {
    /// Create a new DateTime
    pub fn new(
        year: u32,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Self {
        Self {
            year,
            month: month.min(12),
            day: day.min(31),
            hour: hour.min(23),
            minute: minute.min(59),
            second: second.min(59),
            nanosecond: 0,
            day_of_week: 0,
            day_of_year: 0,
        }
    }

    /// Get current system time as DateTime
    pub fn now() -> Self {
        // Use kernel time
        let ts = crate::subsystems::time::timestamp_nanos();
        let seconds = (ts / 1_000_000_000) as i64;

        // Simplified conversion from Unix timestamp
        // This is a basic implementation
        let days_since_epoch = (seconds / 86400) as u32;
        let year = 1970 + days_since_epoch / 365;
        let day_of_year = (days_since_epoch % 365) as u16;

        Self {
            year,
            month: 1,
            day: 1,
            hour: ((seconds % 86400) / 3600) as u8,
            minute: ((seconds % 3600) / 60) as u8,
            second: (seconds % 60) as u8,
            nanosecond: (ts % 1_000_000_000) as u32,
            day_of_week: ((days_since_epoch + 4) % 7) as u8, // Jan 1, 1970 was Thursday
            day_of_year,
        }
    }
}

/// Date and time formatter
pub struct DateTimeFormatter {
    /// Locale code
    locale: String,

    /// Date format patterns for each style
    date_formats: [String; 4],

    /// Time format patterns for each style
    time_formats: [String; 4],

    /// Month names
    month_names: Vec<String>,

    /// Short month names
    short_month_names: Vec<String>,

    /// Weekday names
    weekday_names: Vec<String>,

    /// Short weekday names
    short_weekday_names: Vec<String>,

    /// AM/PM markers
    am_marker: String,
    pm_marker: String,

    /// Statistics
    stats: FormatterStats,
}

#[derive(Debug, Clone, Default)]
pub struct FormatterStats {
    pub date_formats: u64,
    pub time_formats: u64,
    pub number_formats: u64,
    pub currency_formats: u64,
}

impl DateTimeFormatter {
    /// Create a new formatter for the given locale
    pub fn new(locale: String) -> Self {
        Self::with_locale(&locale)
    }

    /// Create formatter with locale-specific settings
    fn with_locale(locale: &str) -> Self {
        match locale {
            "en_US" | "en" => Self::english_us(),
            "zh_CN" | "zh" => Self::chinese_cn(),
            "ja_JP" | "ja" => Self::japanese_jp(),
            "de_DE" | "de" => Self::german_de(),
            "fr_FR" | "fr" => Self::french_fr(),
            "es_ES" | "es" => Self::spanish_es(),
            _ => Self::english_us(), // Default
        }
    }

    /// English (United States) formatter
    fn english_us() -> Self {
        Self {
            locale: String::from("en_US"),
            date_formats: [
                String::from("%m/%d/%Y"),    // Short
                String::from("%b %d, %Y"),   // Medium
                String::from("%B %d, %Y"),   // Long
                String::from("%A, %B %d, %Y"), // Full
            ],
            time_formats: [
                String::from("%I:%M %p"),           // Short
                String::from("%I:%M:%S %p"),       // Medium
                String::from("%I:%M:%S %p %Z"),    // Long
                String::from("%I:%M:%S %p %Z"),    // Full
            ],
            month_names: vec![
                String::from("January"), String::from("February"), String::from("March"),
                String::from("April"), String::from("May"), String::from("June"),
                String::from("July"), String::from("August"), String::from("September"),
                String::from("October"), String::from("November"), String::from("December"),
            ],
            short_month_names: vec![
                String::from("Jan"), String::from("Feb"), String::from("Mar"),
                String::from("Apr"), String::from("May"), String::from("Jun"),
                String::from("Jul"), String::from("Aug"), String::from("Sep"),
                String::from("Oct"), String::from("Nov"), String::from("Dec"),
            ],
            weekday_names: vec![
                String::from("Sunday"), String::from("Monday"), String::from("Tuesday"),
                String::from("Wednesday"), String::from("Thursday"), String::from("Friday"),
                String::from("Saturday"),
            ],
            short_weekday_names: vec![
                String::from("Sun"), String::from("Mon"), String::from("Tue"),
                String::from("Wed"), String::from("Thu"), String::from("Fri"),
                String::from("Sat"),
            ],
            am_marker: String::from("AM"),
            pm_marker: String::from("PM"),
            stats: FormatterStats::default(),
        }
    }

    /// Chinese (China) formatter
    fn chinese_cn() -> Self {
        Self {
            locale: String::from("zh_CN"),
            date_formats: [
                String::from("%Y/%m/%d"),          // Short
                String::from("%Y年%m月%d日"),     // Medium
                String::from("%Y年%m月%d日"),     // Long
                String::from("%Y年%m月%d日 %A"),  // Full
            ],
            time_formats: [
                String::from("%H:%M"),          // Short
                String::from("%H:%M:%S"),       // Medium
                String::from("%H:%M:%S %Z"),    // Long
                String::from("%H:%M:%S %Z"),    // Full
            ],
            month_names: vec![
                String::from("一月"), String::from("二月"), String::from("三月"),
                String::from("四月"), String::from("五月"), String::from("六月"),
                String::from("七月"), String::from("八月"), String::from("九月"),
                String::from("十月"), String::from("十一月"), String::from("十二月"),
            ],
            short_month_names: vec![
                String::from("1月"), String::from("2月"), String::from("3月"),
                String::from("4月"), String::from("5月"), String::from("6月"),
                String::from("7月"), String::from("8月"), String::from("9月"),
                String::from("10月"), String::from("11月"), String::from("12月"),
            ],
            weekday_names: vec![
                String::from("星期日"), String::from("星期一"), String::from("星期二"),
                String::from("星期三"), String::from("星期四"), String::from("星期五"),
                String::from("星期六"),
            ],
            short_weekday_names: vec![
                String::from("周日"), String::from("周一"), String::from("周二"),
                String::from("周三"), String::from("周四"), String::from("周五"),
                String::from("周六"),
            ],
            am_marker: String::from("上午"),
            pm_marker: String::from("下午"),
            stats: FormatterStats::default(),
        }
    }

    /// Japanese (Japan) formatter
    fn japanese_jp() -> Self {
        Self {
            locale: String::from("ja_JP"),
            date_formats: [
                String::from("%Y/%m/%d"),           // Short
                String::from("%Y年%m月%d日"),      // Medium
                String::from("%Y年%m月%d日"),      // Long
                String::from("%Y年%m月%d日(%a)"), // Full
            ],
            time_formats: [
                String::from("%H:%M"),          // Short
                String::from("%H:%M:%S"),       // Medium
                String::from("%H:%M:%S %Z"),    // Long
                String::from("%H:%M:%S %Z"),    // Full
            ],
            month_names: vec![
                String::from("1月"), String::from("2月"), String::from("3月"),
                String::from("4月"), String::from("5月"), String::from("6月"),
                String::from("7月"), String::from("8月"), String::from("9月"),
                String::from("10月"), String::from("11月"), String::from("12月"),
            ],
            short_month_names: vec![
                String::from("1月"), String::from("2月"), String::from("3月"),
                String::from("4月"), String::from("5月"), String::from("6月"),
                String::from("7月"), String::from("8月"), String::from("9月"),
                String::from("10月"), String::from("11月"), String::from("12月"),
            ],
            weekday_names: vec![
                String::from("日曜日"), String::from("月曜日"), String::from("火曜日"),
                String::from("水曜日"), String::from("木曜日"), String::from("金曜日"),
                String::from("土曜日"),
            ],
            short_weekday_names: vec![
                String::from("日"), String::from("月"), String::from("火"),
                String::from("水"), String::from("木"), String::from("金"),
                String::from("土"),
            ],
            am_marker: String::from("午前"),
            pm_marker: String::from("午後"),
            stats: FormatterStats::default(),
        }
    }

    /// German (Germany) formatter
    fn german_de() -> Self {
        Self {
            locale: String::from("de_DE"),
            date_formats: [
                String::from("%d.%m.%Y"),           // Short
                String::from("%d. %b %Y"),          // Medium
                String::from("%d. %B %Y"),          // Long
                String::from("%A, %d. %B %Y"),      // Full
            ],
            time_formats: [
                String::from("%H:%M"),          // Short
                String::from("%H:%M:%S"),       // Medium
                String::from("%H:%M:%S %Z"),    // Long
                String::from("%H:%M:%S %Z"),    // Full
            ],
            month_names: vec![
                String::from("Januar"), String::from("Februar"), String::from("März"),
                String::from("April"), String::from("Mai"), String::from("Juni"),
                String::from("Juli"), String::from("August"), String::from("September"),
                String::from("Oktober"), String::from("November"), String::from("Dezember"),
            ],
            short_month_names: vec![
                String::from("Jan"), String::from("Feb"), String::from("Mär"),
                String::from("Apr"), String::from("Mai"), String::from("Jun"),
                String::from("Jul"), String::from("Aug"), String::from("Sep"),
                String::from("Okt"), String::from("Nov"), String::from("Dez"),
            ],
            weekday_names: vec![
                String::from("Sonntag"), String::from("Montag"), String::from("Dienstag"),
                String::from("Mittwoch"), String::from("Donnerstag"), String::from("Freitag"),
                String::from("Samstag"),
            ],
            short_weekday_names: vec![
                String::from("So"), String::from("Mo"), String::from("Di"),
                String::from("Mi"), String::from("Do"), String::from("Fr"),
                String::from("Sa"),
            ],
            am_marker: String::from("vorm."),
            pm_marker: String::from("nachm."),
            stats: FormatterStats::default(),
        }
    }

    /// French (France) formatter
    fn french_fr() -> Self {
        Self {
            locale: String::from("fr_FR"),
            date_formats: [
                String::from("%d/%m/%Y"),           // Short
                String::from("%d %b %Y"),           // Medium
                String::from("%d %B %Y"),           // Long
                String::from("%A %d %B %Y"),        // Full
            ],
            time_formats: [
                String::from("%H:%M"),          // Short
                String::from("%H:%M:%S"),       // Medium
                String::from("%H:%M:%S %Z"),    // Long
                String::from("%H:%M:%S %Z"),    // Full
            ],
            month_names: vec![
                String::from("janvier"), String::from("février"), String::from("mars"),
                String::from("avril"), String::from("mai"), String::from("juin"),
                String::from("juillet"), String::from("août"), String::from("septembre"),
                String::from("octobre"), String::from("novembre"), String::from("décembre"),
            ],
            short_month_names: vec![
                String::from("janv."), String::from("févr."), String::from("mars"),
                String::from("avr."), String::from("mai"), String::from("juin"),
                String::from("juil."), String::from("août"), String::from("sept."),
                String::from("oct."), String::from("nov."), String::from("déc."),
            ],
            weekday_names: vec![
                String::from("dimanche"), String::from("lundi"), String::from("mardi"),
                String::from("mercredi"), String::from("jeudi"), String::from("vendredi"),
                String::from("samedi"),
            ],
            short_weekday_names: vec![
                String::from("dim."), String::from("lun."), String::from("mar."),
                String::from("mer."), String::from("jeu."), String::from("ven."),
                String::from("sam."),
            ],
            am_marker: String::from("AM"),
            pm_marker: String::from("PM"),
            stats: FormatterStats::default(),
        }
    }

    /// Spanish (Spain) formatter
    fn spanish_es() -> Self {
        Self {
            locale: String::from("es_ES"),
            date_formats: [
                String::from("%d/%m/%Y"),           // Short
                String::from("%d %b %Y"),           // Medium
                String::from("%d de %B de %Y"),     // Long
                String::from("%A, %d de %B de %Y"), // Full
            ],
            time_formats: [
                String::from("%H:%M"),          // Short
                String::from("%H:%M:%S"),       // Medium
                String::from("%H:%M:%S %Z"),    // Long
                String::from("%H:%M:%S %Z"),    // Full
            ],
            month_names: vec![
                String::from("enero"), String::from("febrero"), String::from("marzo"),
                String::from("abril"), String::from("mayo"), String::from("junio"),
                String::from("julio"), String::from("agosto"), String::from("septiembre"),
                String::from("octubre"), String::from("noviembre"), String::from("diciembre"),
            ],
            short_month_names: vec![
                String::from("ene"), String::from("feb"), String::from("mar"),
                String::from("abr"), String::from("may"), String::from("jun"),
                String::from("jul"), String::from("ago"), String::from("sep"),
                String::from("oct"), String::from("nov"), String::from("dic"),
            ],
            weekday_names: vec![
                String::from("domingo"), String::from("lunes"), String::from("martes"),
                String::from("miércoles"), String::from("jueves"), String::from("viernes"),
                String::from("sábado"),
            ],
            short_weekday_names: vec![
                String::from("dom"), String::from("lun"), String::from("mar"),
                String::from("mié"), String::from("jue"), String::from("vie"),
                String::from("sáb"),
            ],
            am_marker: String::from("AM"),
            pm_marker: String::from("PM"),
            stats: FormatterStats::default(),
        }
    }

    /// Format a date with the specified style
    pub fn format_date(&mut self, date: &DateTime, style: DateStyle) -> String {
        self.stats.date_formats += 1;

        let pattern = match style {
            DateStyle::Short => &self.date_formats[0],
            DateStyle::Medium => &self.date_formats[1],
            DateStyle::Long => &self.date_formats[2],
            DateStyle::Full => &self.date_formats[3],
            DateStyle::Custom(pattern) => &String::from(pattern),
        };

        self.apply_date_pattern(date, pattern)
    }

    /// Format a time with the specified style
    pub fn format_time(&mut self, time: &DateTime, style: TimeStyle) -> String {
        self.stats.time_formats += 1;

        let pattern = match style {
            TimeStyle::Short => &self.time_formats[0],
            TimeStyle::Medium => &self.time_formats[1],
            TimeStyle::Long => &self.time_formats[2],
            TimeStyle::Full => &self.time_formats[3],
            TimeStyle::Custom(pattern) => &String::from(pattern),
        };

        self.apply_time_pattern(time, pattern)
    }

    /// Format both date and time
    pub fn format_datetime(&mut self, datetime: &DateTime, date_style: DateStyle, time_style: TimeStyle) -> String {
        let date_str = self.format_date(datetime, date_style);
        let time_str = self.format_time(datetime, time_style);

        alloc::format!("{} {}", date_str, time_str)
    }

    /// Apply date format pattern
    fn apply_date_pattern(&self, date: &DateTime, pattern: &str) -> String {
        let mut result = pattern.to_string();

        // Year
        result = result.replace("%Y", &alloc::format!("{}", date.year));
        result = result.replace("%y", &alloc::format!("{:02}", date.year % 100));

        // Month
        result = result.replace("%m", &alloc::format!("{:02}", date.month));
        result = result.replace("%b", &self.short_month_names.get((date.month - 1) as usize)
            .map(|s| s.as_str()).unwrap_or(""));
        result = result.replace("%B", &self.month_names.get((date.month - 1) as usize)
            .map(|s| s.as_str()).unwrap_or(""));

        // Day
        result = result.replace("%d", &alloc::format!("{:02}", date.day));
        result = result.replace("%e", &alloc::format!("{}", date.day));

        // Weekday
        let dow = (date.day_of_week % 7) as usize;
        result = result.replace("%a", &self.short_weekday_names.get(dow)
            .map(|s| s.as_str()).unwrap_or(""));
        result = result.replace("%A", &self.weekday_names.get(dow)
            .map(|s| s.as_str()).unwrap_or(""));

        // Day of year
        result = result.replace("%j", &alloc::format!("{:03}", date.day_of_year));

        result
    }

    /// Apply time format pattern
    fn apply_time_pattern(&self, time: &DateTime, pattern: &str) -> String {
        let mut result = pattern.to_string();

        // Hour (24-hour)
        result = result.replace("%H", &alloc::format!("{:02}", time.hour));
        result = result.replace("%k", &alloc::format!("{}", time.hour));

        // Hour (12-hour)
        let hour_12 = if time.hour % 12 == 0 { 12 } else { time.hour % 12 };
        result = result.replace("%I", &alloc::format!("{:02}", hour_12));
        result = result.replace("%l", &alloc::format!("{}", hour_12));

        // Minute
        result = result.replace("%M", &alloc::format!("{:02}", time.minute));

        // Second
        result = result.replace("%S", &alloc::format!("{:02}", time.second));

        // AM/PM
        let ampm = if time.hour < 12 { &self.am_marker } else { &self.pm_marker };
        result = result.replace("%p", ampm);

        // Timezone (placeholder)
        result = result.replace("%Z", "UTC");

        result
    }

    /// Get formatter statistics
    pub fn stats(&self) -> &FormatterStats {
        &self.stats
    }
}

// ============================================================================
// Number Formatting
// ============================================================================

/// Number formatter
pub struct NumberFormatter {
    /// Locale code
    locale: String,

    /// Decimal separator
    decimal_separator: String,

    /// Thousands separator
    thousands_separator: String,

    /// Grouping size
    grouping_size: u8,

    /// Statistics
    stats: FormatterStats,
}

impl NumberFormatter {
    /// Create a new number formatter for the given locale
    pub fn new(locale: String) -> Self {
        Self::with_locale(&locale)
    }

    /// Create formatter with locale-specific settings
    fn with_locale(locale: &str) -> Self {
        match locale {
            "en_US" | "en" => Self {
                locale: String::from(locale),
                decimal_separator: String::from("."),
                thousands_separator: String::from(","),
                grouping_size: 3,
                stats: FormatterStats::default(),
            },
            "zh_CN" | "zh" => Self {
                locale: String::from(locale),
                decimal_separator: String::from("."),
                thousands_separator: String::from(","),
                grouping_size: 4,
                stats: FormatterStats::default(),
            },
            "de_DE" | "de" => Self {
                locale: String::from(locale),
                decimal_separator: String::from(","),
                thousands_separator: String::from("."),
                grouping_size: 3,
                stats: FormatterStats::default(),
            },
            "fr_FR" | "fr" => Self {
                locale: String::from(locale),
                decimal_separator: String::from(","),
                thousands_separator: String::from(" "),
                grouping_size: 3,
                stats: FormatterStats::default(),
            },
            _ => Self::english_us(),
        }
    }

    /// English (US) formatter
    fn english_us() -> Self {
        Self {
            locale: String::from("en_US"),
            decimal_separator: String::from("."),
            thousands_separator: String::from(","),
            grouping_size: 3,
            stats: FormatterStats::default(),
        }
    }

    /// Format a number with the specified number of decimal places
    pub fn format(&mut self, number: f64, decimals: u8) -> String {
        self.stats.number_formats += 1;

        // Separate integer and fractional parts
        let int_part = number.trunc() as i64;
        let frac_part = (number.fract().abs() * 10_f64.powi(decimals as i32)).round() as u64;

        // Format integer part with grouping
        let int_str = self.format_integer(int_part);

        // Combine with fractional part
        if decimals > 0 {
            alloc::format!("{}{}{:0width$}", int_str, self.decimal_separator, frac_part, width = decimals as usize)
        } else {
            int_str
        }
    }

    /// Format integer part with grouping
    fn format_integer(&self, mut number: i64) -> String {
        if number == 0 {
            return String::from("0");
        }

        let negative = number < 0;
        if negative {
            number = -number;
        }

        let mut groups = Vec::new();
        while number > 0 {
            let group = (number % 10_u64.pow(self.grouping_size as u32)) as u64;
            groups.push(alloc::format!("{:0width$}", group, width = self.grouping_size as usize));
            number /= 10_u64.pow(self.grouping_size as u32);
        }

        groups.reverse();

        let result = groups.join(&self.thousands_separator);

        if negative {
            alloc::format!("-{}", result)
        } else {
            result
        }
    }

    /// Format a percentage
    pub fn format_percent(&mut self, number: f64, decimals: u8) -> String {
        let percentage = number * 100.0;
        let formatted = self.format(percentage, decimals);
        alloc::format!("{}%", formatted)
    }

    /// Get formatter statistics
    pub fn stats(&self) -> &FormatterStats {
        &self.stats
    }
}

// ============================================================================
// Currency Formatting
// ============================================================================

/// Currency formatter
pub struct CurrencyFormatter {
    /// Number formatter for values
    number_formatter: NumberFormatter,

    /// Currency symbol
    symbol: String,

    /// Symbol position (true = before, false = after)
    symbol_before: bool,

    /// Space between symbol and value
    space_after_symbol: bool,

    /// Decimal places
    decimals: u8,

    /// Statistics
    stats: FormatterStats,
}

impl CurrencyFormatter {
    /// Create a new currency formatter for the given locale
    pub fn new(locale: String) -> Self {
        Self::with_locale(&locale)
    }

    /// Create formatter with locale-specific settings
    fn with_locale(locale: &str) -> Self {
        match locale {
            "en_US" | "en" => Self {
                number_formatter: NumberFormatter::new(String::from(locale)),
                symbol: String::from("$"),
                symbol_before: true,
                space_after_symbol: false,
                decimals: 2,
                stats: FormatterStats::default(),
            },
            "zh_CN" | "zh" => Self {
                number_formatter: NumberFormatter::new(String::from(locale)),
                symbol: String::from("¥"),
                symbol_before: true,
                space_after_symbol: false,
                decimals: 2,
                stats: FormatterStats::default(),
            },
            "ja_JP" | "ja" => Self {
                number_formatter: NumberFormatter::new(String::from(locale)),
                symbol: String::from("¥"),
                symbol_before: true,
                space_after_symbol: false,
                decimals: 0,
                stats: FormatterStats::default(),
            },
            "de_DE" | "de" => Self {
                number_formatter: NumberFormatter::new(String::from(locale)),
                symbol: String::from("€"),
                symbol_before: false,
                space_after_symbol: true,
                decimals: 2,
                stats: FormatterStats::default(),
            },
            "fr_FR" | "fr" => Self {
                number_formatter: NumberFormatter::new(String::from(locale)),
                symbol: String::from("€"),
                symbol_before: false,
                space_after_symbol: true,
                decimals: 2,
                stats: FormatterStats::default(),
            },
            "es_ES" | "es" => Self {
                number_formatter: NumberFormatter::new(String::from(locale)),
                symbol: String::from("€"),
                symbol_before: true,
                space_after_symbol: true,
                decimals: 2,
                stats: FormatterStats::default(),
            },
            _ => Self::english_us(),
        }
    }

    /// English (US) formatter
    fn english_us() -> Self {
        Self {
            number_formatter: NumberFormatter::new(String::from("en_US")),
            symbol: String::from("$"),
            symbol_before: true,
            space_after_symbol: false,
            decimals: 2,
            stats: FormatterStats::default(),
        }
    }

    /// Format a currency amount
    pub fn format(&mut self, amount: f64) -> String {
        self.stats.currency_formats += 1;

        let formatted = self.number_formatter.format(amount, self.decimals);

        if self.symbol_before {
            let space = if self.space_after_symbol { " " } else { "" };
            alloc::format!("{}{}{}", self.symbol, space, formatted)
        } else {
            let space = if self.space_after_symbol { " " } else {""};
            alloc::format!("{}{}{}", formatted, space, self.symbol)
        }
    }

    /// Set custom currency symbol
    pub fn with_symbol(mut self, symbol: String) -> Self {
        self.symbol = symbol;
        self
    }

    /// Set decimal places
    pub fn with_decimals(mut self, decimals: u8) -> Self {
        self.decimals = decimals;
        self
    }

    /// Get formatter statistics
    pub fn stats(&self) -> &FormatterStats {
        &self.stats
    }
}

#[cfg(feature = "kernel_tests")]
mod tests {
    use super::*;

    #[test]
    fn test_date_formatting() {
        let mut formatter = DateTimeFormatter::new(String::from("en_US"));
        let date = DateTime::new(2024, 12, 31, 14, 30, 45);

        let formatted = formatter.format_date(&date, DateStyle::Short);
        assert!(formatted.contains("2024"));
        assert!(formatted.contains("12"));
        assert!(formatted.contains("31"));
    }

    #[test]
    fn test_time_formatting() {
        let mut formatter = DateTimeFormatter::new(String::from("en_US"));
        let time = DateTime::new(2024, 12, 31, 14, 30, 45);

        let formatted = formatter.format_time(&time, TimeStyle::Short);
        assert!(formatted.contains("30"));
    }

    #[test]
    fn test_number_formatting() {
        let mut formatter = NumberFormatter::new(String::from("en_US"));

        assert_eq!(formatter.format(1234.56, 2), "1,234.56");
        assert_eq!(formatter.format(1000000.0, 0), "1,000,000");
    }

    #[test]
    fn test_currency_formatting() {
        let mut formatter = CurrencyFormatter::new(String::from("en_US"));

        assert_eq!(formatter.format(1234.56), "$1,234.56");
        assert_eq!(formatter.format(0.99), "$0.99");
    }

    #[test]
    fn test_percentage_formatting() {
        let mut formatter = NumberFormatter::new(String::from("en_US"));

        assert_eq!(formatter.format_percent(0.1234, 2), "12.34%");
        assert_eq!(formatter.format_percent(1.0, 0), "100%");
    }
}
