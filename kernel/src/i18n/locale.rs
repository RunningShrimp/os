//! Locale Management
//!
//! This module implements locale management for internationalization:
//! - Locale detection and setting
//! - Number/currency/date formatting
//! - Locale-aware string comparison
//!
//! Features:
//! - Standard locale codes (en_US, zh_CN, etc.)
//! - ICU-like formatting
//! - Unicode collation
//! - Timezone support

use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::sync::Arc;

// ============================================================================
// Locale Constants
// ============================================================================

/// Maximum supported locales
pub const MAX_LOCALES: usize = 1 << 8;

/// Default locale
pub const DEFAULT_LOCALE: &str = "en_US";

// ============================================================================
// Locale Code
// ============================================================================

/// Locale code (language_territory)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LocaleCode {
    pub language: String,
    pub territory: String,
    pub encoding: String,
}

impl LocaleCode {
    pub fn new(language: String, territory: String, encoding: String) -> Self {
        Self {
            language,
            territory,
            encoding,
        }
    }

    pub fn from_str(code: &str) -> Self {
        let parts: Vec<&str> = code.split(|c| c == '_' || c == '.').collect();
        
        Self {
            language: parts.get(0).unwrap_or(&"en").to_string(),
            territory: parts.get(1).unwrap_or(&"US").to_string(),
            encoding: parts.get(2).unwrap_or(&"UTF-8").to_string(),
        }
    }

    pub fn to_string(&self) -> String {
        alloc::format!("{}_{}.{}", self.language, self.territory, self.encoding)
    }
}

// ============================================================================
// Locale Data
// ============================================================================

/// Locale data (formatting rules)
#[derive(Debug, Clone)]
pub struct LocaleData {
    pub locale_code: LocaleCode,
    pub name: String,
    pub native_name: String,
    
    /// Decimal separator
    pub decimal_separator: String,
    
    /// Thousands separator
    pub thousands_separator: String,
    
    /// Currency symbol
    pub currency_symbol: String,
    
    /// Currency format (symbol position: 0=before, 1=after)
    pub currency_symbol_position: u8,
    
    /// Date format
    pub date_format: String,
    
    /// Time format
    pub time_format: String,
    
    /// Short date format
    pub short_date_format: String,
    
    /// Long date format
    pub long_date_format: String,
    
    /// Timezone name
    pub timezone: String,
    
    /// UTC offset (seconds)
    pub utc_offset: i32,
    
    /// Week starts on (0=Sunday, 1=Monday)
    pub week_start_day: u8,
    
    /// Weekday names
    pub weekday_names: Vec<String>,
    
    /// Short weekday names
    pub short_weekday_names: Vec<String>,
    
    /// Month names
    pub month_names: Vec<String>,
    
    /// Short month names
    pub short_month_names: Vec<String>,
}

impl LocaleData {
    pub fn en_us() -> Self {
        Self {
            locale_code: LocaleCode::from_str("en_US.UTF-8"),
            name: String::from("English (United States)"),
            native_name: String::from("English (United States)"),
            decimal_separator: String::from("."),
            thousands_separator: String::from(","),
            currency_symbol: String::from("$"),
            currency_symbol_position: 0,
            date_format: String::from("%m/%d/%Y"),
            time_format: String::from("%I:%M:%S %p"),
            short_date_format: String::from("%m/%d/%y"),
            long_date_format: String::from("%A, %B %d, %Y"),
            timezone: String::from("America/New_York"),
            utc_offset: -18000, // EST
            week_start_day: 0,
            weekday_names: {
    let mut v = alloc::vec::Vec::new();
    v.push(String::from("Sunday"));
    v.push(String::from("Monday"));
    v.push(String::from("Tuesday"));
    v.push(String::from("Wednesday"));
    v.push(String::from("Thursday"));
    v.push(String::from("Friday"));
    v.push(String::from("Saturday"));
    v
},
            short_weekday_names: {
    let mut v = alloc::vec::Vec::new();
    v.push(String::from("Sun"));
    v.push(String::from("Mon"));
    v.push(String::from("Tue"));
    v.push(String::from("Wed"));
    v.push(String::from("Thu"));
    v.push(String::from("Fri"));
    v.push(String::from("Sat"));
    v
},
            month_names: {
    let mut v = alloc::vec::Vec::new();
    v.push(String::from("January"));
    v.push(String::from("February"));
    v.push(String::from("March"));
    v.push(String::from("April"));
    v.push(String::from("May"));
    v.push(String::from("June"));
    v.push(String::from("July"));
    v.push(String::from("August"));
    v.push(String::from("September"));
    v.push(String::from("October"));
    v.push(String::from("November"));
    v.push(String::from("December"));
    v
},
            short_month_names: {
    let mut v = alloc::vec::Vec::new();
    v.push(String::from("Jan"));
    v.push(String::from("Feb"));
    v.push(String::from("Mar"));
    v.push(String::from("Apr"));
    v.push(String::from("May"));
    v.push(String::from("Jun"));
    v.push(String::from("Jul"));
    v.push(String::from("Aug"));
    v.push(String::from("Sep"));
    v.push(String::from("Oct"));
    v.push(String::from("Nov"));
    v.push(String::from("Dec"));
    v
},
        }
    }

    pub fn zh_cn() -> Self {
        Self {
            locale_code: LocaleCode::from_str("zh_CN.UTF-8"),
            name: String::from("Chinese (China)"),
            native_name: String::from("中文（中国）"),
            decimal_separator: String::from("."),
            thousands_separator: String::from(","),
            currency_symbol: String::from("¥"),
            currency_symbol_position: 1,
            date_format: String::from("%Y/%m/%d"),
            time_format: String::from("%H:%M:%S"),
            short_date_format: String::from("%Y/%m/%d"),
            long_date_format: String::from("%Y年%m月%d日 %A"),
            timezone: String::from("Asia/Shanghai"),
            utc_offset: 28800, // CST
            week_start_day: 1,
            weekday_names: {
    let mut v = alloc::vec::Vec::new();
    v.push(String::from("星期日"));
    v.push(String::from("星期一"));
    v.push(String::from("星期二"));
    v.push(String::from("星期三"));
    v.push(String::from("星期四"));
    v.push(String::from("星期五"));
    v.push(String::from("星期六"));
    v
},
            short_weekday_names: {
    let mut v = alloc::vec::Vec::new();
    v.push(String::from("日"));
    v.push(String::from("一"));
    v.push(String::from("二"));
    v.push(String::from("三"));
    v.push(String::from("四"));
    v.push(String::from("五"));
    v.push(String::from("六"));
    v
},
            month_names: {
    let mut v = alloc::vec::Vec::new();
    v.push(String::from("一月"));
    v.push(String::from("二月"));
    v.push(String::from("三月"));
    v.push(String::from("四月"));
    v.push(String::from("五月"));
    v.push(String::from("六月"));
    v.push(String::from("七月"));
    v.push(String::from("八月"));
    v.push(String::from("九月"));
    v.push(String::from("十月"));
    v.push(String::from("十一月"));
    v.push(String::from("十二月"));
    v
},
            short_month_names: {
    let mut v = alloc::vec::Vec::new();
    v.push(String::from("1月"));
    v.push(String::from("2月"));
    v.push(String::from("3月"));
    v.push(String::from("4月"));
    v.push(String::from("5月"));
    v.push(String::from("6月"));
    v.push(String::from("7月"));
    v.push(String::from("8月"));
    v.push(String::from("9月"));
    v.push(String::from("10月"));
    v.push(String::from("11月"));
    v.push(String::from("12月"));
    v
},
        }
    }

    pub fn format_number(&self, number: f64, decimals: u32) -> String {
        // Round to specified decimals
        let multiplier = 10_f64.powi(decimals as i32);
        let rounded = (number * multiplier).round() / multiplier;

        // Split into integer and fractional parts
        let int_part = rounded.trunc() as i64;
        let frac_part = (rounded.fract().abs() * multiplier) as u64;

        // Format integer part with thousands separator
        let int_str = self.format_integer(int_part);

        // Combine with fractional part
        if decimals > 0 {
            alloc::format!("{}{}{:0width$}", int_str, self.decimal_separator, frac_part, width = decimals as usize)
        } else {
            int_str
        }
    }

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
            let group = (number % 1000) as u32;
            groups.push(alloc::format!("{:03}", group));
            number /= 1000;
        }

        // Remove leading zeros from the last group
        if let Some(last) = groups.last_mut() {
            *last = last.trim_start_matches('0').to_string();
            if last.is_empty() {
                *last = String::from("0");
            }
        }

        groups.reverse();
        let result = groups.join(&self.thousands_separator);

        if negative {
            alloc::format!("-{}", result)
        } else {
            result
        }
    }

    pub fn format_currency(&self, amount: f64, decimals: u32) -> String {
        let number_str = self.format_number(amount, decimals);

        if self.currency_symbol_position == 0 {
            alloc::format!("{}{}", self.currency_symbol, number_str)
        } else {
            alloc::format!("{}{}", number_str, self.currency_symbol)
        }
    }

    pub fn format_date(&self, year: u32, month: u8, day: u8, format: &str) -> String {
        let mut result = format.to_string();

        result = result.replace("%Y", &year.to_string());
        result = result.replace("%m", &alloc::format!("{:02}", month));
        result = result.replace("%d", &alloc::format!("{:02}", day));

        result
    }

    pub fn format_datetime(&self, year: u32, month: u8, day: u8,
                            hour: u8, minute: u8, second: u8, format: &str) -> String {
        let mut result = format.to_string();

        result = result.replace("%Y", &year.to_string());
        result = result.replace("%m", &alloc::format!("{:02}", month));
        result = result.replace("%d", &alloc::format!("{:02}", day));
        result = result.replace("%H", &alloc::format!("{:02}", hour));
        result = result.replace("%I", &alloc::format!("{:02}", if hour % 12 == 0 { 12 } else { hour % 12 }));
        result = result.replace("%M", &alloc::format!("{:02}", minute));
        result = result.replace("%S", &alloc::format!("{:02}", second));
        result = result.replace("%p", if hour < 12 { "AM" } else { "PM" });
        result = result.replace("%A", &self.weekday_names[0]);
        result = result.replace("%a", &self.short_weekday_names[0]);
        result = result.replace("%B", &self.month_names.get((month - 1) as usize).unwrap_or(&String::from("")));
        result = result.replace("%b", &self.short_month_names.get((month - 1) as usize).unwrap_or(&String::from("")));

        result
    }
}

// ============================================================================
// Locale Manager
// ============================================================================

/// Locale manager
pub struct LocaleManager {
    pub locales: Mutex<BTreeMap<String, Arc<LocaleData>>>,
    pub current_locale: Arc<LocaleData>,
    pub next_locale_id: AtomicU64,
    pub stats: Mutex<LocaleManagerStats>,
}

#[derive(Debug, Clone, Copy)]
pub struct LocaleManagerStats {
    pub total_locales: usize,
    pub switches: u64,
    pub format_operations: u64,
}

impl Default for LocaleManagerStats {
    fn default() -> Self {
        Self {
            total_locales: 0,
            switches: 0,
            format_operations: 0,
        }
    }
}

impl LocaleManager {
    pub fn new() -> Self {
        let mut locales = BTreeMap::new();
        let en_us = Arc::new(LocaleData::en_us());
        let zh_cn = Arc::new(LocaleData::zh_cn());
        
        let en_us_code = en_us.locale_code.to_string();
        let zh_cn_code = zh_cn.locale_code.to_string();
        
        locales.insert(en_us_code, en_us.clone());
        locales.insert(zh_cn_code, zh_cn);
        
        Self {
            locales: Mutex::new(locales),
            current_locale: en_us,
            next_locale_id: AtomicU64::new(1),
            stats: Mutex::new(LocaleManagerStats::default()),
        }
    }

    pub fn register_locale(&self, locale: Arc<LocaleData>) -> Result<(), String> {
        let mut locales = self.locales.lock();
        let code = locale.locale_code.to_string();
        
        if locales.contains_key(&code) {
            return Err(alloc::string::String::from("Locale ") + &code.to_string() + alloc::string::String::from(" already exists"));
        }
        
        locales.insert(code, locale);
        crate::println!("[locale] Registered locale: {}", code);
        
        let mut stats = self.stats.lock();
        stats.total_locales = locales.len();
        
        Ok(())
    }

    pub fn set_locale(&self, code: String) -> Result<(), String> {
        let locales = self.locales.lock();
        
        let locale = locales.get(&code)
            .ok_or(alloc::string::String::from("Locale ") + &code.to_string() + alloc::string::String::from(" not found"))?
            .clone();
        
        self.current_locale = locale;
        
        self.stats.lock().switches.fetch_add(1, Ordering::Relaxed);
        crate::println!("[locale] Switched to locale: {}", code);
        
        Ok(())
    }

    pub fn get_current_locale(&self) -> Arc<LocaleData> {
        self.current_locale.clone()
    }

    pub fn format_number(&self, number: f64, decimals: u32) -> String {
        self.stats.lock().format_operations.fetch_add(1, Ordering::Relaxed);
        self.current_locale.format_number(number, decimals)
    }

    pub fn format_currency(&self, amount: f64, decimals: u32) -> String {
        self.stats.lock().format_operations.fetch_add(1, Ordering::Relaxed);
        self.current_locale.format_currency(amount, decimals)
    }

    pub fn format_date(&self, year: u32, month: u8, day: u8) -> String {
        self.stats.lock().format_operations.fetch_add(1, Ordering::Relaxed);
        self.current_locale.format_date(year, month, day, &self.current_locale.date_format)
    }

    pub fn format_time(&self, hour: u8, minute: u8, second: u8) -> String {
        self.stats.lock().format_operations.fetch_add(1, Ordering::Relaxed);
        let now = crate::subsystems::time::timestamp_nanos();
        let ts = now / 1_000_000_000;
        let day = ((ts / 86400) % 7) as u32;
        let month = 1;
        let year = 1970;
        self.current_locale.format_datetime(year, month, day, hour, minute, second, &self.current_locale.time_format)
    }

    pub fn get_weekday_name(&self, day_index: usize) -> String {
        let idx = day_index.min(6);
        self.current_locale.weekday_names.get(idx).cloned().unwrap_or_default()
    }

    pub fn get_month_name(&self, month_index: usize) -> String {
        let idx = month_index.min(11);
        self.current_locale.month_names.get(idx).cloned().unwrap_or_default()
    }

    pub fn get_stats(&self) -> LocaleManagerStats {
        let mut stats = self.stats.lock();
        stats.total_locales = self.locales.lock().len();
        *stats
    }
}
