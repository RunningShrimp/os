#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! Translation and Localization
//!
//! This module implements translation and localization for internationalization:
//! - Message catalogs
//! - Plural forms
//! - Context-sensitive translations
//!
//! Features:
//! - Translation lookups by locale
//! - String interpolation
//! - Plural rules per language
//! - Translation context

use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;

// ============================================================================
// Translation Constants
// ============================================================================

/// Maximum translations per locale
pub const MAX_TRANSLATIONS: usize = 1 << 16;

/// Maximum contexts
pub const MAX_CONTEXTS: usize = 1 << 8;

// ============================================================================
// Translation Context
// ============================================================================

/// Translation context (for disambiguation)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TranslationContext {
    /// General context
    General,
    
    /// Button label
    Button,
    
    /// Menu item
    Menu,
    
    /// Error message
    Error,
    
    /// Warning message
    Warning,
    
    /// Info message
    Info,
    
    /// Status message
    Status,
    
    /// Custom context
    Custom(String),
}

// ============================================================================
// Plural Forms
// ============================================================================

/// Plural form type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluralForm {
    /// Singular
    Singular,
    
    /// Plural
    Plural,
    
    /// Custom plural rule
    Custom(u32),
}

/// Plural rule function
pub type PluralRule = fn(n: u32) -> PluralForm;

// ============================================================================
// Translation Entry
// ============================================================================

/// Translation entry
#[derive(Debug, Clone)]
pub struct TranslationEntry {
    pub key: String,
    pub translations: Mutex<BTreeMap<TranslationContext, String>>,
    
    /// Plural translations (form -> translation)
    pub plural_translations: Mutex<BTreeMap<PluralForm, String>>,
    
    /// Has plural forms
    pub has_plural: bool,
    
    /// Plural rule function
    pub plural_rule: Option<PluralRule>,
}

impl TranslationEntry {
    pub fn new(key: String) -> Self {
        Self {
            key,
            translations: Mutex::new(BTreeMap::new()),
            plural_translations: Mutex::new(BTreeMap::new()),
            has_plural: false,
            plural_rule: None,
        }
    }

    pub fn add_translation(&self, context: TranslationContext, translation: String) {
        let mut translations = self.translations.lock();
        translations.insert(context, translation);
        crate::println!("[translation] Added translation for {} ({:?})", self.key, context);
    }

    pub fn add_plural(&self, form: PluralForm, translation: String) {
        let mut plurals = self.plural_translations.lock();
        plurals.insert(form, translation);
        self.has_plural = true;
        crate::println!("[translation] Added plural for {} ({:?})", self.key, form);
    }

    pub fn set_plural_rule(&self, rule: PluralRule) {
        self.plural_rule = Some(rule);
        self.has_plural = true;
    }

    pub fn get_translation(&self, context: TranslationContext) -> Option<String> {
        let translations = self.translations.lock();
        
        // Try specific context first
        if let Some(translation) = translations.get(&context) {
            return Some(translation.clone());
        }

        // Fall back to General context
        translations.get(&TranslationContext::General)
            .cloned()
    }

    pub fn get_plural_translation(&self, n: u32) -> Option<String> {
        if !self.has_plural {
            return None;
        }

        let plurals = self.plural_translations.lock();
        
        let form = if let Some(rule) = self.plural_rule {
            rule(n)
        } else {
            // Default: singular for 1, plural for >1
            if n == 1 {
                PluralForm::Singular
            } else {
                PluralForm::Plural
            }
        };

        plurals.get(&form).cloned()
    }
}

// ============================================================================
// Translation Catalog
// ============================================================================

/// Translation catalog (per locale)
#[derive(Debug, Clone)]
pub struct TranslationCatalog {
    pub locale_code: String,
    pub name: String,
    pub native_name: String,
    pub entries: Mutex<BTreeMap<String, Arc<TranslationEntry>>>>,
    pub total_entries: AtomicU64,
}

impl TranslationCatalog {
    pub fn new(locale_code: String, name: String, native_name: String) -> Self {
        Self {
            locale_code,
            name,
            native_name,
            entries: Mutex::new(BTreeMap::new()),
            total_entries: AtomicU64::new(0),
        }
    }

    pub fn add_entry(&self, entry: Arc<TranslationEntry>) {
        let key = entry.key.clone();
        let mut entries = self.entries.lock();
        entries.insert(key, entry);
        self.total_entries.fetch_add(1, Ordering::Relaxed);
        crate::println!("[catalog] Added entry {} to locale {}", key, self.locale_code);
    }

    pub fn translate(&self, key: &str, context: TranslationContext) -> Option<String> {
        let entries = self.entries.lock();
        entries.get(key)?.get_translation(context)
    }

    pub fn translate_plural(&self, key: &str, n: u32) -> Option<String> {
        let entries = self.entries.lock();
        entries.get(key)?.get_plural_translation(n)
    }

    pub fn translate_with_args(&self, key: &str, context: TranslationContext, args: Vec<String>) -> Option<String> {
        let mut translation = self.translate(key, context)?;

        // Simple interpolation: {0}, {1}, ...
        for (i, arg) in args.iter().enumerate() {
            let mut placeholder = /* TODO: {:{{} */ &i.to_string() + alloc::string::String::from("}}");
            translation = translation.replace(&placeholder, arg);
        }

        Some(translation)
    }

    pub fn get_entry(&self, key: &str) -> Option<Arc<TranslationEntry>> {
        let entries = self.entries.lock();
        entries.get(key).cloned()
    }

    pub fn get_stats(&self) -> CatalogStats {
        CatalogStats {
            locale_code: self.locale_code.clone(),
            total_entries: self.total_entries.load(Ordering::Relaxed) as usize,
            keys_count: self.entries.lock().len(),
        }
    }
}

/// Catalog statistics
#[derive(Debug, Clone)]
pub struct CatalogStats {
    pub locale_code: String,
    pub total_entries: usize,
    pub keys_count: usize,
}

// ============================================================================
// Common Plural Rules
// ============================================================================

/// English plural rule (n != 1)
pub fn plural_rule_en(n: u32) -> PluralForm {
    if n == 1 {
        PluralForm::Singular
    } else {
        PluralForm::Plural
    }
}

/// Chinese plural rule (always plural)
pub fn plural_rule_zh(n: u32) -> PluralForm {
    PluralForm::Singular // Chinese doesn't distinguish singular/plural
}

// ============================================================================
// Translation Manager
// ============================================================================

/// Translation manager
pub struct TranslationManager {
    pub catalogs: Mutex<BTreeMap<String, Arc<TranslationCatalog>>>>,
    pub current_locale: String,
    pub fallback_locale: String,
    pub next_catalog_id: AtomicU64,
    pub stats: Mutex<TranslationManagerStats>,
}

#[derive(Debug, Clone, Copy)]
pub struct TranslationManagerStats {
    pub total_catalogs: usize,
    pub total_translations: u64,
    pub translation_requests: u64,
    pub fallbacks: u64,
}

impl Default for TranslationManagerStats {
    fn default() -> Self {
        Self {
            total_catalogs: 0,
            total_translations: 0,
            translation_requests: 0,
            fallbacks: 0,
        }
    }
}

impl TranslationManager {
    pub fn new(default_locale: String) -> Self {
        Self {
            catalogs: Mutex::new(BTreeMap::new()),
            current_locale: default_locale.clone(),
            fallback_locale: default_locale,
            next_catalog_id: AtomicU64::new(1),
            stats: Mutex::new(TranslationManagerStats::default()),
        }
    }

    pub fn register_catalog(&self, catalog: Arc<TranslationCatalog>) -> Result<(), String> {
        let mut catalogs = self.catalogs.lock();
        let code = catalog.locale_code.clone();

        if catalogs.contains_key(&code) {
            return Err(alloc::string::String::from("Catalog ") + &code.to_string() + alloc::string::String::from(" already exists"));
        }

        catalogs.insert(code, catalog);
        crate::println!("[translation_manager] Registered catalog: {}", code);

        let mut stats = self.stats.lock();
        stats.total_catalogs = catalogs.len();

        Ok(())
    }

    pub fn set_locale(&self, locale: String) {
        self.current_locale = locale.clone();
        crate::println!("[translation_manager] Current locale: {}", locale);
    }

    pub fn set_fallback_locale(&self, locale: String) {
        self.fallback_locale = locale;
    }

    pub fn translate(&self, key: &str) -> Option<String> {
        self.translate_context(key, TranslationContext::General)
    }

    pub fn translate_context(&self, key: &str, context: TranslationContext) -> Option<String> {
        self.stats.lock().translation_requests.fetch_add(1, Ordering::Relaxed);

        // Try current locale
        if let Some(translation) = self.translate_in_locale(key, &self.current_locale, context) {
            return Some(translation);
        }

        // Try fallback locale
        self.stats.lock().fallbacks.fetch_add(1, Ordering::Relaxed);
        self.translate_in_locale(key, &self.fallback_locale, context)
    }

    pub fn translate_plural(&self, key: &str, n: u32) -> Option<String> {
        self.stats.lock().translation_requests.fetch_add(1, Ordering::Relaxed);

        // Try current locale
        if let Some(translation) = self.translate_plural_in_locale(key, &self.current_locale, n) {
            return Some(translation);
        }

        // Try fallback locale
        self.stats.lock().fallbacks.fetch_add(1, Ordering::Relaxed);
        self.translate_plural_in_locale(key, &self.fallback_locale, n)
    }

    pub fn translate_with_args(&self, key: &str, args: Vec<String>) -> Option<String> {
        self.stats.lock().translation_requests.fetch_add(1, Ordering::Relaxed);

        // Try current locale
        if let Some(translation) = self.translate_in_locale_with_args(key, &self.current_locale, args.clone()) {
            return Some(translation);
        }

        // Try fallback locale
        self.stats.lock().fallbacks.fetch_add(1, Ordering::Relaxed);
        self.translate_in_locale_with_args(key, &self.fallback_locale, args)
    }

    fn translate_in_locale(&self, key: &str, locale: &str, context: TranslationContext) -> Option<String> {
        let catalogs = self.catalogs.lock();
        catalogs.get(locale)?.translate(key, context)
    }

    fn translate_plural_in_locale(&self, key: &str, locale: &str, n: u32) -> Option<String> {
        let catalogs = self.catalogs.lock();
        catalogs.get(locale)?.translate_plural(key, n)
    }

    fn translate_in_locale_with_args(&self, key: &str, locale: &str, args: Vec<String>) -> Option<String> {
        let catalogs = self.catalogs.lock();
        catalogs.get(locale)?.translate_with_args(key, TranslationContext::General, args)
    }

    pub fn get_catalog(&self, locale: &str) -> Option<Arc<TranslationCatalog>> {
        let catalogs = self.catalogs.lock();
        catalogs.get(locale).cloned()
    }

    pub fn get_stats(&self) -> TranslationManagerStats {
        let mut stats = self.stats.lock();
        stats.total_catalogs = self.catalogs.lock().len();
        *stats
    }
}
