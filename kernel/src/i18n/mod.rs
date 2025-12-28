//! Internationalization (i18n)
//!
//! This module implements internationalization for NOS:
//! - Locale management
//! - Character encoding
//! - Translation and localization
//!
//! Features:
//! - Multi-locale support (en_US, zh_CN, etc.)
//! - UTF-8 encoding and Unicode handling
//! - Translation catalogs with context
//! - Plural forms per language
//! - Number/currency/date formatting

pub mod locale;
pub mod encoding;
pub mod translation;

// Re-export i18n types
pub use locale::*;
pub use encoding::*;
pub use translation::*;
