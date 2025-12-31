//! Internationalization (i18n) for the NOS Kernel
//!
//! This module provides comprehensive internationalization support including:
//! - Locale management and detection
//! - Unicode support (UTF-8, UTF-16, UTF-32)
//! - Character encoding conversion
//! - Locale-aware string collation
//! - Date/time formatting per locale
//! - Number and currency formatting
//! - Gettext-style message catalogs
//! - Right-to-left (RTL) text support
//!
//! ## Architecture
//!
//! The i18n module is organized into the following submodules:
//!
//! - **locale**: Locale management and data
//! - **encoding**: Character encoding support (UTF-8, etc.)
//! - **translation**: Translation and localization (legacy, use messages)
//! - **unicode**: Full Unicode support (UTF-8/16/32, normalization)
//! - **collation**: Locale-aware string comparison and sorting
//! - **formatting**: Date/time/number/currency formatting
//! - **messages**: Gettext-style message catalogs
//!
//! ## Usage
//!
//! ### Basic Locale Management
//!
//! ```rust
//! use kernel::i18n::{LocaleManager, LocaleCode};
//!
//! // Get the global locale manager
//! let manager = LocaleManager::global();
//!
//! // Set current locale
//! manager.set_locale("zh_CN".to_string()).ok();
//!
//! // Format a number according to locale
//! let formatted = manager.format_number(1234.56, 2);
//! println!("{}", formatted); // "1,234.56" or "1 234,56" depending on locale
//! ```
//!
//! ### Unicode Handling
//!
//! ```rust
//! use kernel::i18n::unicode::{Utf8Codec, UnicodeNormalizer, NormalizationForm};
//!
//! // Validate UTF-8
//! let valid = Utf8Codec::validate(bytes);
//!
//! // Normalize text
//! let normalized = UnicodeNormalizer::normalize("café", NormalizationForm::NFC);
//!
//! // Check text direction
//! use kernel::i18n::unicode::{BidiAnalyzer, TextDirection};
//! let direction = BidiAnalyzer::base_direction("مرحبا"); // RTL
//! ```
//!
//! ### String Collation
//!
//! ```rust
//! use kernel::i18n::collation::{Collator, CollationOptions, CollationStrength};
//!
//! let mut collator = Collator::with_options(
//!     "en_US".to_string(),
//!     CollationOptions::new()
//!         .with_strength(CollationStrength::Primary)
//!         .ignore_case(true)
//! );
//!
//! let result = collator.compare("apple", "Apple"); // Equal at primary level
//! ```
//!
//! ### Date/Time Formatting
//!
//! ```rust
//! use kernel::i18n::formatting::{DateTimeFormatter, DateStyle};
//!
//! let mut formatter = DateTimeFormatter::new("zh_CN".to_string());
//! let now = DateTime::now();
//!
//! let formatted = formatter.format_date(&now, DateStyle::Long);
//! // Outputs: "2024年12月31日"
//! ```
//!
//! ### Message Translation
//!
//! ```rust
//! use kernel::i18n::messages::{TranslationManager, gettext, ngettext};
//!
//! let manager = TranslationManager::new();
//! manager.set_locale("zh_CN".to_string());
//!
//! // Simple translation
//! let msg = gettext(&manager, "Hello, World!");
//!
//! // Plural translation
//! let files = ngettext(&manager, "One file", "{0} files", count);
//! ```
//!
//! ## Features
//!
//! - **100+ Supported Locales**: ICU-like locale database
//! - **Unicode 14.0**: Full Unicode support including all normalization forms
//! - **Collation**: Locale-aware sorting with UCA (Unicode Collation Algorithm)
//! - **Formatting**: Date, time, number, currency, percentage formatting per locale
//! - **Translation**: Gettext-compatible message catalogs with plural forms
//! - **Encoding**: UTF-8, UTF-16, UTF-32 encoding/decoding
//! - **RTL Support**: Right-to-left text detection and handling
//! - **Zero-Copy**: Efficient string handling without unnecessary allocations

// Re-export legacy modules for backward compatibility
pub mod locale;
pub mod encoding;
pub mod translation;

// New comprehensive i18n modules
pub mod unicode;
pub mod collation;
pub mod formatting;
pub mod messages;

// ============================================================================
// Public API Re-exports
// ============================================================================

// Locale management
pub use locale::{
    LocaleCode,
    LocaleData,
    LocaleManager,
    LocaleManagerStats,
    DEFAULT_LOCALE,
    MAX_LOCALES,
};

// Encoding
pub use encoding::{
    Utf8Converter,
    UnicodeClassifier,
    CaseConverter,
    UnicodeNormalizer,
    NormalizationForm,
    EncodingError,
    CharCategory,
    UTF8_ENCODING,
    ASCII_ENCODING,
    ISO8859_1_ENCODING,
};

// Translation (legacy)
pub use translation::{
    TranslationContext,
    TranslationEntry,
    TranslationCatalog,
    CatalogStats,
    PluralForm,
    PluralRule,
    plural_rule_en,
    plural_rule_zh,
    TranslationManager as LegacyTranslationManager,
};

// Unicode support
pub use unicode::{
    // UTF codecs
    Utf8Codec,
    Utf16Codec,
    Utf32Codec,

    // Constants
    MAX_CODE_POINT,
    SURROGATE_HIGH_START,
    SURROGATE_HIGH_END,
    SURROGATE_LOW_START,
    SURROGATE_LOW_END,
    MAX_BEFORE_SURROGATES,
    MIN_AFTER_SURROGATES,
    REPLACEMENT_CHARACTER,
    BYTE_ORDER_MARK,

    // Normalization
    NormalizationForm as UnicodeNormalizationForm,
    UnicodeNormalizer as UnicodeNormalizerNew,

    // Grapheme clusters
    GraphemeSegmenter,

    // Character properties
    CharProperties,
    Category as UnicodeCategory,

    // Bidirectional text
    BidiAnalyzer,
    TextDirection,

    // Errors
    UnicodeError,
};

// Collation
pub use collation::{
    Collator,
    CollationStrength,
    CollationOptions,
    CollationKey,
    LocaleCollationRules,
    CollatorStats,
    DEFAULT_STRENGTH,
    MAX_COLLATION_ELEMENTS,

    // Helper functions
    compare_case_insensitive,
    compare_ignore_accents,
    compare_default,
    equals_ignore,
    compare_primary,
    compare_secondary,
    compare_tertiary,
    compare_quaternary,
    compare_identical,
};

// Formatting
pub use formatting::{
    DateTimeFormatter,
    NumberFormatter,
    CurrencyFormatter,

    DateStyle,
    TimeStyle,
    DateTime,

    FormatterStats,

    DEFAULT_DATE_FORMAT,
    DEFAULT_TIME_FORMAT,
    DEFAULT_DATETIME_FORMAT,
    DEFAULT_CURRENCY_DECIMALS,
};

// Messages (gettext-style)
pub use messages::{
    MessageEntry,
    MessageCatalog,
    TranslationManager,
    MessageContext,
    PluralForm as MessagePluralForm,
    PluralRuleFn,
    CatalogMetadata,
    TranslationStats,

    plural_rules,
    get_plural_rule,

    MAX_MESSAGES_PER_CATALOG,
    MAX_CONTEXT_LENGTH,
    MAX_KEY_LENGTH,
    MAX_VALUE_LENGTH,

    // Convenience functions
    gettext,
    pgettext,
    ngettext,
    gettext_args,
};

// ============================================================================
// Global Managers
// ============================================================================

use spin::Mutex;
use alloc::sync::Arc;
use core::sync::atomic;

/// Global locale manager
static GLOBAL_LOCALE_MANAGER: Mutex<Option<Arc<LocaleManager>>> = Mutex::new(None);

/// Global translation manager
static GLOBAL_TRANSLATION_MANAGER: Mutex<Option<Arc<TranslationManager>>> = Mutex::new(None);

/// Initialize the global i18n system
///
/// This should be called during kernel initialization.
pub fn init() -> Result<(), String> {
    // Initialize global locale manager
    let locale_manager = Arc::new(LocaleManager::new());
    *GLOBAL_LOCALE_MANAGER.lock() = Some(locale_manager);

    // Initialize global translation manager
    let translation_manager = Arc::new(TranslationManager::new());
    *GLOBAL_TRANSLATION_MANAGER.lock() = Some(translation_manager);

    crate::println!("[i18n] Internationalization system initialized");
    Ok(())
}

/// Get the global locale manager
pub fn global_locale_manager() -> Arc<LocaleManager> {
    GLOBAL_LOCALE_MANAGER.lock().as_ref().unwrap().clone()
}

/// Get the global translation manager
pub fn global_translation_manager() -> Arc<TranslationManager> {
    GLOBAL_TRANSLATION_MANAGER.lock().as_ref().unwrap().clone()
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Translate a message using the global translation manager
pub fn translate(key: &str) -> String {
    let manager = global_translation_manager();
    manager.translate(key)
}

/// Translate a message with context
pub fn translate_context(key: &str, context: &str) -> String {
    let manager = global_translation_manager();
    manager.translate_context(key, context)
}

/// Translate a plural form
pub fn translate_plural(key: &str, n: u64) -> String {
    let manager = global_translation_manager();
    manager.translate_plural(key, n)
}

/// Translate with arguments
pub fn translate_args(key: &str, args: &[&str]) -> String {
    let manager = global_translation_manager();
    manager.translate_args(key, args)
}

/// Format a number using the global locale
pub fn format_number(number: f64, decimals: u8) -> String {
    let manager = global_locale_manager();
    manager.format_number(number, decimals as u32)
}

/// Format currency using the global locale
pub fn format_currency(amount: f64, decimals: u8) -> String {
    let manager = global_locale_manager();
    manager.format_currency(amount, decimals as u32)
}

/// Get the current locale
pub fn get_locale() -> String {
    let manager = global_translation_manager();
    manager.current_locale()
}

/// Set the current locale
pub fn set_locale(locale: String) {
    let translation_manager = global_translation_manager();
    translation_manager.set_locale(locale.clone());

    let locale_manager = global_locale_manager();
    let _ = locale_manager.set_locale(locale);
}

/// Check if text is right-to-left
pub fn is_rtl(text: &str) -> bool {
    BidiAnalyzer::base_direction(text) == TextDirection::RTL
}

/// Detect text direction
pub fn detect_direction(text: &str) -> TextDirection {
    BidiAnalyzer::base_direction(text)
}

// ============================================================================
// High-Level API
// ============================================================================

/// I18n configuration
#[derive(Debug, Clone)]
pub struct I18nConfig {
    /// Default locale
    pub default_locale: String,

    /// Fallback locale
    pub fallback_locale: String,

    /// Enable RTL support
    pub enable_rtl: bool,

    /// Enable Unicode normalization
    pub enable_normalization: bool,

    /// Cache size for translations
    pub translation_cache_size: usize,
}

impl Default for I18nConfig {
    fn default() -> Self {
        Self {
            default_locale: String::from("en_US"),
            fallback_locale: String::from("en_US"),
            enable_rtl: true,
            enable_normalization: true,
            translation_cache_size: 1024,
        }
    }
}

/// High-level i18n facade
pub struct I18n {
    config: I18nConfig,
    locale_manager: Arc<LocaleManager>,
    translation_manager: Arc<TranslationManager>,
}

impl I18n {
    /// Create a new i18n facade with default configuration
    pub fn new() -> Self {
        let config = I18nConfig::default();

        // Initialize managers if not already initialized
        let locale_manager = global_locale_manager();
        let translation_manager = global_translation_manager();

        // Set default locale
        set_locale(config.default_locale.clone());

        Self {
            config,
            locale_manager,
            translation_manager,
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: I18nConfig) -> Self {
        let locale_manager = global_locale_manager();
        let translation_manager = global_translation_manager();

        set_locale(config.default_locale.clone());

        Self {
            config,
            locale_manager,
            translation_manager,
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &I18nConfig {
        &self.config
    }

    /// Get locale manager
    pub fn locale_manager(&self) -> &LocaleManager {
        &self.locale_manager
    }

    /// Get translation manager
    pub fn translation_manager(&self) -> &TranslationManager {
        &self.translation_manager
    }

    /// Translate a message
    pub fn t(&self, key: &str) -> String {
        self.translation_manager.translate(key)
    }

    /// Translate with context
    pub fn tc(&self, context: &str, key: &str) -> String {
        self.translation_manager.translate_context(key, context)
    }

    /// Translate plural
    pub fn tn(&self, key: &str, n: u64) -> String {
        self.translation_manager.translate_plural(key, n)
    }

    /// Format number
    pub fn format_number(&self, number: f64, decimals: u8) -> String {
        self.locale_manager.format_number(number, decimals as u32)
    }

    /// Format currency
    pub fn format_currency(&self, amount: f64) -> String {
        self.locale_manager.format_currency(amount, DEFAULT_CURRENCY_DECIMALS as u32)
    }

    /// Format date/time
    pub fn format_datetime(&self, datetime: &formatting::DateTime, date_style: DateStyle, time_style: TimeStyle) -> String {
        let mut formatter = DateTimeFormatter::new(self.translation_manager.current_locale());
        formatter.format_datetime(datetime, date_style, time_style)
    }

    /// Check if current locale is RTL
    pub fn is_rtl_locale(&self) -> bool {
        let locale = self.translation_manager.current_locale();
        matches!(locale.as_str(), "ar" | "he" | "fa" | "ur")
    }

    /// Get system statistics
    pub fn get_stats(&self) -> I18nStats {
        I18nStats {
            locale: self.translation_manager.current_locale(),
            translation_stats: self.translation_manager.stats(),
            locale_stats: self.locale_manager.get_stats(),
        }
    }
}

impl Default for I18n {
    fn default() -> Self {
        Self::new()
    }
}

/// i18n system statistics
#[derive(Debug, Clone)]
pub struct I18nStats {
    /// Current locale
    pub locale: String,

    /// Translation statistics
    pub translation_stats: TranslationStats,

    /// Locale manager statistics
    pub locale_stats: LocaleManagerStats,
}

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(feature = "kernel_tests")]
mod tests {
    use super::*;

    #[test]
    fn test_i18n_init() {
        let result = init();
        assert!(result.is_ok());
    }

    #[test]
    fn test_locale_manager() {
        let manager = LocaleManager::new();
        assert!(manager.register_locale(Arc::new(LocaleData::en_us())).is_ok());
        assert!(manager.set_locale("en_US.UTF-8".to_string()).is_ok());
    }

    #[test]
    fn test_unicode_validation() {
        let valid = b"Hello, \xE2\x82\xAC"; // Euro sign in UTF-8
        assert!(Utf8Codec::validate(valid));
    }

    #[test]
    fn test_collation() {
        let mut collator = Collator::new("en_US".to_string());
        assert_eq!(collator.compare("apple", "apple"), core::cmp::Ordering::Equal);
        assert_eq!(collator.compare("apple", "banana"), core::cmp::Ordering::Less);
    }

    #[test]
    fn test_formatting() {
        let mut formatter = DateTimeFormatter::new("en_US".to_string());
        let dt = DateTime::new(2024, 12, 31, 23, 59, 59);
        let formatted = formatter.format_date(&dt, DateStyle::Short);
        assert!(formatted.contains("2024"));
    }

    #[test]
    fn test_translation() {
        let manager = TranslationManager::new();
        let catalog = Arc::new(MessageCatalog::new("en_US".to_string()));
        catalog.add("test", "", "Test Translation");
        manager.register_catalog(catalog);
        manager.set_locale("en_US".to_string());

        assert_eq!(manager.translate("test"), "Test Translation");
    }

    #[test]
    fn test_i18n_facade() {
        let i18n = I18n::new();
        let translated = i18n.t("test_key");
        // Should return the key itself if no translation exists
        assert_eq!(translated, "test_key");
    }

    #[test]
    fn test_rtl_detection() {
        // Arabic text
        assert!(is_rtl("مرحبا بالعالم"));

        // English text
        assert!(!is_rtl("Hello World"));

        // Mixed text
        assert_eq!(detect_direction("Hello مرحبا"), TextDirection::Mixed);
    }

    #[test]
    fn test_number_formatting() {
        let mut formatter = NumberFormatter::new("en_US".to_string());
        assert_eq!(formatter.format(1234.56, 2), "1,234.56");

        let mut formatter = NumberFormatter::new("de_DE".to_string());
        assert_eq!(formatter.format(1234.56, 2), "1.234,56");
    }

    #[test]
    fn test_currency_formatting() {
        let mut formatter = CurrencyFormatter::new("en_US".to_string());
        assert_eq!(formatter.format(1234.56), "$1,234.56");

        let mut formatter = CurrencyFormatter::new("ja_JP".to_string());
        assert_eq!(formatter.format(1000), "¥1,000");
    }
}
