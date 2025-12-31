//! Gettext-Style Message Catalogs
//!
//! This module implements gettext-style message catalogs for localization:
//! - Message lookup by locale and context
//! - Plural form handling per language
//! - Message interpolation with parameters
//! - Catalog loading and management
//! - Translation memory and caching
//!
//! ## Features
//!
//! - **gettext Compatibility**: Compatible with .po/.mo file format
//! - **Plural Forms**: Language-specific plural rules
//! - **Context**: Message disambiguation with context strings
//! - **Interpolation**: Parameter substitution in messages
//! - **Fallback**: Automatic fallback to parent locale

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// Message Constants
// ============================================================================

/// Maximum messages per catalog
pub const MAX_MESSAGES_PER_CATALOG: usize = 1 << 16;

/// Maximum message context length
pub const MAX_CONTEXT_LENGTH: usize = 256;

/// Maximum message key length
pub const MAX_KEY_LENGTH: usize = 512;

/// Maximum message value length
pub const MAX_VALUE_LENGTH: usize = 4096;

// ============================================================================
// Message Context
// ============================================================================

/// Message context for disambiguation
///
/// Context allows the same key to have different translations based on usage.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MessageContext {
    /// General context (no specific context)
    General,

    /// User interface context
    UI,

    /// Button labels
    Button,

    /// Menu items
    Menu,

    /// Dialog messages
    Dialog,

    /// Error messages
    Error,

    /// Warning messages
    Warning,

    /// Information messages
    Info,

    /// Status messages
    Status,

    /// Tooltip text
    Tooltip,

    /// Help text
    Help,

    /// Custom context string
    Custom(String),
}

impl MessageContext {
    /// Convert context to string key
    pub fn as_str(&self) -> &str {
        match self {
            MessageContext::General => "",
            MessageContext::UI => "ui",
            MessageContext::Button => "button",
            MessageContext::Menu => "menu",
            MessageContext::Dialog => "dialog",
            MessageContext::Error => "error",
            MessageContext::Warning => "warning",
            MessageContext::Info => "info",
            MessageContext::Status => "status",
            MessageContext::Tooltip => "tooltip",
            MessageContext::Help => "help",
            MessageContext::Custom(s) => s.as_str(),
        }
    }

    /// Create context from string
    pub fn from_str(s: &str) -> Self {
        match s {
            "" => MessageContext::General,
            "ui" => MessageContext::UI,
            "button" => MessageContext::Button,
            "menu" => MessageContext::Menu,
            "dialog" => MessageContext::Dialog,
            "error" => MessageContext::Error,
            "warning" => MessageContext::Warning,
            "info" => MessageContext::Info,
            "status" => MessageContext::Status,
            "tooltip" => MessageContext::Tooltip,
            "help" => MessageContext::Help,
            s => MessageContext::Custom(String::from(s)),
        }
    }
}

// ============================================================================
// Plural Forms
// ============================================================================

/// Plural form type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluralForm {
    /// Singular form
    Singular,

    /// Plural form
    Plural,

    /// Custom form index
    Custom(u32),
}

/// Plural rule function type
///
/// Given a count, returns which plural form to use.
pub type PluralRuleFn = fn(n: u64) -> PluralForm;

/// Predefined plural rules for common languages
pub mod plural_rules {
    use super::PluralForm;

    /// English plural rule: n != 1
    pub fn english(n: u64) -> PluralForm {
        if n == 1 {
            PluralForm::Singular
        } else {
            PluralForm::Plural
        }
    }

    /// Chinese/Japanese/Korean plural rule: always singular
    pub fn cjk(_n: u64) -> PluralForm {
        PluralForm::Singular
    }

    /// Russian plural rule: complex
    pub fn russian(n: u64) -> PluralForm {
        let n100 = n % 100;
        let n10 = n % 10;

        if n100 >= 11 && n100 <= 14 {
            PluralForm::Custom(2);
        } else if n10 == 1 {
            PluralForm::Singular;
        } else if n10 >= 2 && n10 <= 4 {
            PluralForm::Plural;
        } else {
            PluralForm::Custom(2);
        }
    }

    /// Arabic plural rule: six forms
    pub fn arabic(n: u64) -> PluralForm {
        let n100 = n % 100;

        if n == 0 {
            PluralForm::Custom(0);
        } else if n == 1 {
            PluralForm::Singular;
        } else if n == 2 {
            PluralForm::Plural;
        } else if n100 >= 3 && n100 <= 10 {
            PluralForm::Custom(3);
        } else if n100 >= 11 && n100 <= 99 {
            PluralForm::Custom(4);
        } else {
            PluralForm::Custom(5);
        }
    }

    /// Polish plural rule
    pub fn polish(n: u64) -> PluralForm {
        let n100 = n % 100;
        let n10 = n % 10;

        if n == 1 {
            PluralForm::Singular;
        } else if n100 >= 12 && n100 <= 14 {
            PluralForm::Custom(2);
        } else if n10 >= 2 && n10 <= 4 {
            PluralForm::Plural;
        } else {
            PluralForm::Custom(2);
        }
    }

    /// German/French/Spanish/Italian plural rule: n > 1
    pub fn european(n: u64) -> PluralForm {
        if n == 1 {
            PluralForm::Singular
        } else {
            PluralForm::Plural
        }
    }
}

/// Get plural rule for a language
pub fn get_plural_rule(locale: &str) -> PluralRuleFn {
    let lang = locale.split('_').next().unwrap_or("en");

    match lang {
        "en" | "de" | "nl" | "sv" | "no" | "da" | "fi" | "es" | "it" | "pt" | "fr" => {
            plural_rules::european
        }
        "zh" | "ja" | "ko" | "vi" | "th" => plural_rules::cjk,
        "ru" | "uk" | "sr" | "hr" => plural_rules::russian,
        "ar" => plural_rules::arabic,
        "pl" => plural_rules::polish,
        _ => plural_rules::english, // Default to English
    }
}

// ============================================================================
// Message Entry
// ============================================================================

/// Single message entry in a catalog
pub struct MessageEntry {
    /// Message key (msgid)
    key: String,

    /// Translated message strings by context
    translations: Mutex<BTreeMap<String, String>>,

    /// Plural forms by form index
    plurals: Mutex<BTreeMap<u32, String>>,

    /// Has plural translations
    has_plurals: bool,

    /// Plural rule function
    plural_rule: PluralRuleFn,
}

impl MessageEntry {
    /// Create a new message entry
    pub fn new(key: String) -> Self {
        Self {
            key,
            translations: Mutex::new(BTreeMap::new()),
            plurals: Mutex::new(BTreeMap::new()),
            has_plurals: false,
            plural_rule: plural_rules::english,
        }
    }

    /// Add a translation for a specific context
    pub fn add_translation(&self, context: &str, translation: String) {
        let mut translations = self.translations.lock();
        translations.insert(context.to_string(), translation);
    }

    /// Add a plural form translation
    pub fn add_plural(&self, form: u32, translation: String) {
        let mut plurals = self.plurals.lock();
        plurals.insert(form, translation);
        self.has_plurals = true;
    }

    /// Set the plural rule
    pub fn set_plural_rule(&self, rule: PluralRuleFn) {
        self.plural_rule = rule;
        self.has_plurals = true;
    }

    /// Get translation for a context
    pub fn get_translation(&self, context: &str) -> Option<String> {
        let translations = self.translations.lock();

        // Try specific context first
        if let Some(translation) = translations.get(context) {
            return Some(translation.clone());
        }

        // Fall back to general context
        translations.get("").cloned()
    }

    /// Get plural form translation
    pub fn get_plural(&self, n: u64) -> Option<String> {
        if !self.has_plurals {
            return None;
        }

        let form = match (self.plural_rule)(n) {
            PluralForm::Singular => 0,
            PluralForm::Plural => 1,
            PluralForm::Custom(i) => i,
        };

        let plurals = self.plurals.lock();
        plurals.get(&form).cloned()
    }
}

// ============================================================================
// Message Catalog
// ============================================================================

/// Message catalog for a specific locale
///
/// Similar to gettext's .mo file structure.
pub struct MessageCatalog {
    /// Locale identifier (e.g., "en_US", "zh_CN")
    locale: String,

    /// Language code
    language: String,

    /// Territory/region code
    territory: String,

    /// Messages by key
    messages: Mutex<BTreeMap<String, Arc<MessageEntry>>>,

    /// Total message count
    message_count: AtomicU64,

    /// Plural rule for this locale
    plural_rule: PluralRuleFn,

    /// Catalog metadata
    metadata: Mutex<CatalogMetadata>,
}

/// Catalog metadata
#[derive(Debug, Clone)]
pub struct CatalogMetadata {
    /// Translation team
    pub team: String,

    /// Last revision date
    pub revision_date: String,

    /// Project name
    pub project: String,

    /// Version
    pub version: String,

    /// MIME version
    pub mime_version: String,

    /// Content type
    pub content_type: String,

    /// Character encoding
    pub charset: String,

    /// POT creation date
    pub pot_creation_date: String,
}

impl Default for CatalogMetadata {
    fn default() -> Self {
        Self {
            team: String::new(),
            revision_date: String::new(),
            project: String::new(),
            version: String::new(),
            mime_version: String::from("1.0"),
            content_type: String::from("text/plain; charset=UTF-8"),
            charset: String::from("UTF-8"),
            pot_creation_date: String::new(),
        }
    }
}

impl MessageCatalog {
    /// Create a new message catalog
    pub fn new(locale: String) -> Self {
        let (language, territory) = {
            let parts: Vec<&str> = locale.split('_').collect();
            (
                parts.get(0).unwrap_or(&"en").to_string(),
                parts.get(1).unwrap_or(&"US").to_string(),
            )
        };

        let plural_rule = get_plural_rule(&locale);

        Self {
            locale,
            language,
            territory,
            messages: Mutex::new(BTreeMap::new()),
            message_count: AtomicU64::new(0),
            plural_rule,
            metadata: Mutex::new(CatalogMetadata::default()),
        }
    }

    /// Get locale identifier
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Add a message entry
    pub fn add_entry(&self, entry: Arc<MessageEntry>) {
        let key = entry.key.clone();
        let mut messages = self.messages.lock();
        messages.insert(key.clone(), entry);
        self.message_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Add a simple translation
    pub fn add(&self, key: &str, context: &str, translation: &str) {
        let entry = Arc::new(MessageEntry::new(key.to_string()));
        entry.add_translation(context, translation.to_string());

        let mut messages = self.messages.lock();
        messages.insert(key.to_string(), entry);
        self.message_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Add a translation with plurals
    pub fn add_plural(&self, key: &str, translations: &[&str]) {
        let entry = Arc::new(MessageEntry::new(key.to_string()));
        entry.set_plural_rule(self.plural_rule);

        for (i, trans) in translations.iter().enumerate() {
            entry.add_plural(i as u32, trans.to_string());
        }

        let mut messages = self.messages.lock();
        messages.insert(key.to_string(), entry);
        self.message_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Translate a message key
    pub fn translate(&self, key: &str) -> Option<String> {
        self.translate_context(key, "")
    }

    /// Translate with context
    pub fn translate_context(&self, key: &str, context: &str) -> Option<String> {
        let messages = self.messages.lock();
        messages.get(key)?.get_translation(context)
    }

    /// Translate a plural form
    pub fn translate_plural(&self, key: &str, n: u64) -> Option<String> {
        let messages = self.messages.lock();
        messages.get(key)?.get_plural(n)
    }

    /// Translate with parameter interpolation
    pub fn translate_args(&self, key: &str, args: &[&str]) -> Option<String> {
        let mut translation = self.translate(key)?;

        // Replace placeholders: {0}, {1}, etc.
        for (i, arg) in args.iter().enumerate() {
            let placeholder = alloc::format!("{{{}}}", i);
            translation = translation.replace(&placeholder, arg);
        }

        Some(translation)
    }

    /// Get message count
    pub fn message_count(&self) -> u64 {
        self.message_count.load(Ordering::Relaxed)
    }

    /// Check if key exists
    pub fn has_key(&self, key: &str) -> bool {
        let messages = self.messages.lock();
        messages.contains_key(key)
    }

    /// Get catalog metadata
    pub fn metadata(&self) -> CatalogMetadata {
        self.metadata.lock().clone()
    }

    /// Set catalog metadata
    pub fn set_metadata(&self, metadata: CatalogMetadata) {
        *self.metadata.lock() = metadata;
    }
}

// ============================================================================
// Translation Manager
// ============================================================================

/// Translation manager for handling multiple catalogs
///
/// Similar to gettext's bindtextdomain() and textdomain() functionality.
pub struct TranslationManager {
    /// Message catalogs by locale
    catalogs: Mutex<BTreeMap<String, Arc<MessageCatalog>>>,

    /// Current locale
    current_locale: Mutex<String>,

    /// Fallback locale
    fallback_locale: Mutex<String>,

    /// Default domain
    default_domain: Mutex<String>,

    /// Statistics
    stats: Mutex<TranslationStats>,
}

/// Translation statistics
#[derive(Debug, Clone, Default)]
pub struct TranslationStats {
    pub total_catalogs: usize,
    pub total_messages: u64,
    pub lookup_count: u64,
    pub hit_count: u64,
    pub miss_count: u64,
    pub fallback_count: u64,
}

impl TranslationManager {
    /// Create a new translation manager
    pub fn new() -> Self {
        Self {
            catalogs: Mutex::new(BTreeMap::new()),
            current_locale: Mutex::new(String::from("en_US")),
            fallback_locale: Mutex::new(String::from("en_US")),
            default_domain: Mutex::new(String::from("messages")),
            stats: Mutex::new(TranslationStats::default()),
        }
    }

    /// Register a message catalog
    pub fn register_catalog(&self, catalog: Arc<MessageCatalog>) {
        let locale = catalog.locale().to_string();
        let mut catalogs = self.catalogs.lock();
        catalogs.insert(locale, catalog.clone());

        let mut stats = self.stats.lock();
        stats.total_catalogs = catalogs.len();
        stats.total_messages += catalog.message_count();
    }

    /// Set current locale
    pub fn set_locale(&self, locale: String) {
        *self.current_locale.lock() = locale;
    }

    /// Get current locale
    pub fn current_locale(&self) -> String {
        self.current_locale.lock().clone()
    }

    /// Set fallback locale
    pub fn set_fallback_locale(&self, locale: String) {
        *self.fallback_locale.lock() = locale;
    }

    /// Set default domain
    pub fn set_domain(&self, domain: String) {
        *self.default_domain.lock() = domain;
    }

    /// Translate a message
    pub fn translate(&self, key: &str) -> String {
        self.translate_context(key, "")
    }

    /// Translate with context
    pub fn translate_context(&self, key: &str, context: &str) -> String {
        let mut stats = self.stats.lock();
        stats.lookup_count += 1;
        drop(stats);

        let current_locale = self.current_locale.lock().clone();

        // Try current locale
        if let Some(translation) = self.lookup_in_locale(&current_locale, key, context) {
            self.stats.lock().hit_count += 1;
            return translation;
        }

        // Try fallback locale
        let fallback_locale = self.fallback_locale.lock().clone();
        self.stats.lock().fallback_count += 1;

        if let Some(translation) = self.lookup_in_locale(&fallback_locale, key, context) {
            return translation;
        }

        // Return key as fallback
        self.stats.lock().miss_count += 1;
        key.to_string()
    }

    /// Translate a plural form
    pub fn translate_plural(&self, key: &str, n: u64) -> String {
        let mut stats = self.stats.lock();
        stats.lookup_count += 1;
        drop(stats);

        let current_locale = self.current_locale.lock().clone();

        // Try current locale
        if let Some(translation) = self.lookup_plural_in_locale(&current_locale, key, n) {
            self.stats.lock().hit_count += 1;
            return translation;
        }

        // Try fallback locale
        let fallback_locale = self.fallback_locale.lock().clone();
        self.stats.lock().fallback_count += 1;

        if let Some(translation) = self.lookup_plural_in_locale(&fallback_locale, key, n) {
            return translation;
        }

        // Return key as fallback
        self.stats.lock().miss_count += 1;
        key.to_string()
    }

    /// Translate with arguments
    pub fn translate_args(&self, key: &str, args: &[&str]) -> String {
        let mut translation = self.translate(key);

        // Replace placeholders
        for (i, arg) in args.iter().enumerate() {
            let placeholder = alloc::format!("{{{}}}", i);
            translation = translation.replace(&placeholder, arg);
        }

        translation
    }

    /// Lookup in specific locale
    fn lookup_in_locale(&self, locale: &str, key: &str, context: &str) -> Option<String> {
        let catalogs = self.catalogs.lock();
        catalogs.get(locale)?.translate_context(key, context)
    }

    /// Lookup plural in specific locale
    fn lookup_plural_in_locale(&self, locale: &str, key: &str, n: u64) -> Option<String> {
        let catalogs = self.catalogs.lock();
        catalogs.get(locale)?.translate_plural(key, n)
    }

    /// Get statistics
    pub fn stats(&self) -> TranslationStats {
        self.stats.lock().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        *self.stats.lock() = TranslationStats::default();
    }

    /// Get catalog for a locale
    pub fn get_catalog(&self, locale: &str) -> Option<Arc<MessageCatalog>> {
        let catalogs = self.catalogs.lock();
        catalogs.get(locale).cloned()
    }

    /// Check if locale is available
    pub fn has_locale(&self, locale: &str) -> bool {
        let catalogs = self.catalogs.lock();
        catalogs.contains_key(locale)
    }

    /// Get all available locales
    pub fn available_locales(&self) -> Vec<String> {
        let catalogs = self.catalogs.lock();
        catalogs.keys().cloned().collect()
    }
}

impl Default for TranslationManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Convenience Macros
// ============================================================================

/// Translate a message (convenience function)
///
/// This would typically be implemented as a macro, but here we provide a function.
pub fn gettext(manager: &TranslationManager, key: &str) -> String {
    manager.translate(key)
}

/// Translate with context (convenience function)
pub fn pgettext(manager: &TranslationManager, context: &str, key: &str) -> String {
    manager.translate_context(key, context)
}

/// Translate plural form (convenience function)
pub fn ngettext(manager: &TranslationManager, key: &str, key_plural: &str, n: u64) -> String {
    manager.translate_plural(key, n)
}

/// Translate with arguments (convenience function)
pub fn gettext_args(manager: &TranslationManager, key: &str, args: &[&str]) -> String {
    manager.translate_args(key, args)
}

#[cfg(feature = "kernel_tests")]
mod tests {
    use super::*;

    #[test]
    fn test_message_context() {
        let ctx = MessageContext::from_str("button");
        assert_eq!(ctx.as_str(), "button");

        let ctx = MessageContext::Custom(String::from("custom"));
        assert_eq!(ctx.as_str(), "custom");
    }

    #[test]
    fn test_plural_rules() {
        // English: singular for 1, plural for others
        assert_eq!(plural_rules::english(1), PluralForm::Singular);
        assert_eq!(plural_rules::english(2), PluralForm::Plural);

        // CJK: always singular
        assert_eq!(plural_rules::cjk(1), PluralForm::Singular);
        assert_eq!(plural_rules::cjk(100), PluralForm::Singular);
    }

    #[test]
    fn test_catalog() {
        let catalog = MessageCatalog::new(String::from("en_US"));

        catalog.add("hello", "", "Hello, World!");
        assert_eq!(catalog.translate("hello"), Some(String::from("Hello, World!")));
        assert_eq!(catalog.translate("unknown"), None);
    }

    #[test]
    fn test_manager() {
        let manager = TranslationManager::new();

        // Create and register a catalog
        let catalog = Arc::new(MessageCatalog::new(String::from("en_US")));
        catalog.add("test", "", "Translated Test");
        manager.register_catalog(catalog);

        manager.set_locale(String::from("en_US"));

        assert_eq!(manager.translate("test"), "Translated Test");
        assert_eq!(manager.translate("missing"), "missing");
    }

    #[test]
    fn test_plural_translation() {
        let catalog = MessageCatalog::new(String::from("en_US"));
        catalog.add_plural("file", &["One file", "{0} files"]);

        assert_eq!(catalog.translate_plural("file", 1), Some(String::from("One file")));
        assert_eq!(catalog.translate_plural("file", 5), Some(String::from("{0} files")));
    }
}
