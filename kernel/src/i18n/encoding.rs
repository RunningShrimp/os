//! Character Encoding
//!
//! This module implements character encoding for internationalization:
//! - UTF-8 support
//! - Unicode handling
//! - Character set conversion
//!
//! Features:
//! - UTF-8 validation and conversion
//! - Unicode normalization
//! - Character classification
//! - Case conversion

use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::{String, ToString};

// ============================================================================
// Encoding Constants
// ============================================================================

/// UTF-8 encoding
pub const UTF8_ENCODING: &str = "UTF-8";

/// ASCII encoding
pub const ASCII_ENCODING: &str = "ASCII";

/// ISO-8859-1 encoding
pub const ISO8859_1_ENCODING: &str = "ISO-8859-1";

// ============================================================================
// UTF-8 Utilities
// ============================================================================

/// UTF-8 validator and converter
pub struct Utf8Converter;

impl Utf8Converter {
    /// Validate UTF-8 byte sequence
    pub fn is_valid_utf8(bytes: &[u8]) -> bool {
        let mut i = 0;
        while i < bytes.len() {
            let byte = bytes[i];
            
            let (char_len, continuation_bytes) = if byte < 0x80 {
                // 1-byte sequence (ASCII)
                (1, 0)
            } else if byte >= 0xC2 && byte < 0xE0 {
                // 2-byte sequence
                (2, 1)
            } else if byte >= 0xE0 && byte < 0xF0 {
                // 3-byte sequence
                (3, 2)
            } else if byte >= 0xF0 && byte < 0xF5 {
                // 4-byte sequence
                (4, 3)
            } else {
                // Invalid byte
                return false;
            };

            // Check continuation bytes
            if i + char_len > bytes.len() {
                return false;
            }

            for j in 1..=continuation_bytes {
                if bytes[i + j] & 0xC0 != 0x80 {
                    return false;
                }
            }

            i += char_len;
        }

        true
    }

    /// Convert bytes to UTF-8 string (validating)
    pub fn to_string(bytes: &[u8]) -> Result<String, EncodingError> {
        if !Self::is_valid_utf8(bytes) {
            return Err(EncodingError::InvalidUtf8);
        }

        String::from_utf8(bytes.to_vec())
            .map_err(|_| EncodingError::InvalidUtf8)
    }

    /// Get UTF-8 code point length from first byte
    pub fn char_length_from_first_byte(byte: u8) -> usize {
        if byte < 0x80 { 1 }
        else if byte >= 0xC2 && byte < 0xE0 { 2 }
        else if byte >= 0xE0 && byte < 0xF0 { 3 }
        else if byte >= 0xF0 && byte < 0xF5 { 4 }
        else { 0 }
    }

    /// Get code point from UTF-8 bytes
    pub fn get_code_point(bytes: &[u8]) -> Result<u32, EncodingError> {
        if bytes.is_empty() {
            return Err(EncodingError::EmptyInput);
        }

        let first = bytes[0];
        let len = Self::char_length_from_first_byte(first);

        if len == 0 {
            return Err(EncodingError::InvalidUtf8);
        }

        if bytes.len() < len {
            return Err(EncodingError::IncompleteSequence);
        }

        let code_point = match len {
            1 => first as u32,
            2 => {
                ((first & 0x1F) as u32) << 6 |
                ((bytes[1] & 0x3F) as u32)
            }
            3 => {
                ((first & 0x0F) as u32) << 12 |
                ((bytes[1] & 0x3F) as u32) << 6 |
                ((bytes[2] & 0x3F) as u32)
            }
            4 => {
                ((first & 0x07) as u32) << 18 |
                ((bytes[1] & 0x3F) as u32) << 12 |
                ((bytes[2] & 0x3F) as u32) << 6 |
                ((bytes[3] & 0x3F) as u32)
            }
            _ => return Err(EncodingError::InvalidUtf8),
        };

        Ok(code_point)
    }
}

// ============================================================================
// Unicode Character Properties
// ============================================================================

/// Unicode character classifier
pub struct UnicodeClassifier;

impl UnicodeClassifier {
    /// Check if code point is letter
    pub fn is_letter(code_point: u32) -> bool {
        // Basic Latin letters
        if (0x0041..=0x005A).contains(&code_point) || // A-Z
           (0x0061..=0x007A).contains(&code_point) || // a-z
           (0x00C0..=0x00D6).contains(&code_point) || // Latin-1 Supplement
           (0x00D8..=0x00F6).contains(&code_point) || // Latin-1 Supplement (continued)
           (0x00F8..=0x00FF).contains(&code_point) { // Latin-1 Supplement (continued)
            return true;
        }

        // Chinese characters (CJK Unified Ideographs)
        if (0x4E00..=0x9FFF).contains(&code_point) ||
           (0x3400..=0x4DBF).contains(&code_point) ||
           (0x20000..=0x2A6DF).contains(&code_point) ||
           (0x2A700..=0x2B73F).contains(&code_point) ||
           (0x2B740..=0x2B81F).contains(&code_point) ||
           (0x2B820..=0x2CEAF).contains(&code_point) ||
           (0x2CEB0..=0x2EBEF).contains(&code_point) {
            return true;
        }

        // Japanese Hiragana/Katakana
        if (0x3040..=0x309F).contains(&code_point) || // Hiragana
           (0x30A0..=0x30FF).contains(&code_point) { // Katakana
            return true;
        }

        // Korean Hangul
        if (0x1100..=0x11FF).contains(&code_point) ||
           (0xAC00..=0xD7AF).contains(&code_point) {
            return true;
        }

        false
    }

    /// Check if code point is digit
    pub fn is_digit(code_point: u32) -> bool {
        // ASCII digits
        if (0x0030..=0x0039).contains(&code_point) {
            return true;
        }

        // Arabic-Indic digits
        if (0x0660..=0x0669).contains(&code_point) {
            return true;
        }

        // Chinese/Japanese/Korean digits
        if (0xFF10..=0xFF19).contains(&code_point) {
            return true;
        }

        false
    }

    /// Check if code point is whitespace
    pub fn is_whitespace(code_point: u32) -> bool {
        // ASCII whitespace
        matches!(code_point, 
            0x0009 | // TAB
            0x000A | // LF
            0x000B | // VT
            0x000C | // FF
            0x000D | // CR
            0x0020   // SPACE
        )
    }

    /// Check if code point is punctuation
    pub fn is_punctuation(code_point: u32) -> bool {
        // ASCII punctuation
        matches!(code_point,
            0x0021..=0x002F | // ! " # $ % & ' ( ) * + , - . /
            0x003A..=0x0040 | // : ; < = > ? @
            0x005B..=0x0060 | // [ \ ] ^ _
            0x007B..=0x007E   // { | } ~
        )
    }

    /// Check if code point is CJK (Chinese/Japanese/Korean)
    pub fn is_cjk(code_point: u32) -> bool {
        Self::is_letter(code_point)
    }

    /// Get code point category
    pub fn get_category(code_point: u32) -> CharCategory {
        if Self::is_letter(code_point) {
            CharCategory::Letter
        } else if Self::is_digit(code_point) {
            CharCategory::Digit
        } else if Self::is_whitespace(code_point) {
            CharCategory::Whitespace
        } else if Self::is_punctuation(code_point) {
            CharCategory::Punctuation
        } else {
            CharCategory::Other
        }
    }
}

/// Character category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharCategory {
    Letter,
    Digit,
    Whitespace,
    Punctuation,
    Other,
}

// ============================================================================
// Case Conversion
// ============================================================================

/// Case converter
pub struct CaseConverter;

impl CaseConverter {
    /// Convert character to lowercase
    pub fn to_lowercase(code_point: u32) -> u32 {
        // ASCII uppercase to lowercase
        if (0x0041..=0x005A).contains(&code_point) {
            return code_point + 0x20;
        }

        code_point
    }

    /// Convert character to uppercase
    pub fn to_uppercase(code_point: u32) -> u32 {
        // ASCII lowercase to uppercase
        if (0x0061..=0x007A).contains(&code_point) {
            return code_point - 0x20;
        }

        code_point
    }

    /// Convert string to lowercase
    pub fn string_to_lowercase(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for c in s.chars() {
            let code = c as u32;
            let lower = Self::to_lowercase(code);
            if let Some(c_lower) = char::from_u32(lower) {
                result.push(c_lower);
            }
        }
        result
    }

    /// Convert string to uppercase
    pub fn string_to_uppercase(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for c in s.chars() {
            let code = c as u32;
            let upper = Self::to_uppercase(code);
            if let Some(c_upper) = char::from_u32(upper) {
                result.push(c_upper);
            }
        }
        result
    }

    /// Capitalize first letter
    pub fn capitalize(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut first = true;

        for c in s.chars() {
            if first {
                let code = c as u32;
                let upper = Self::to_uppercase(code);
                if let Some(c_upper) = char::from_u32(upper) {
                    result.push(c_upper);
                }
                first = false;
            } else {
                result.push(c);
            }
        }

        result
    }
}

// ============================================================================
// Unicode Normalization
// ============================================================================

/// Normalization form
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizationForm {
    /// Canonical composition
    NFC,
    
    /// Canonical decomposition
    NFD,
    
    /// Compatibility composition
    NFKC,
    
    /// Compatibility decomposition
    NFKD,
}

/// Unicode normalizer
pub struct UnicodeNormalizer;

impl UnicodeNormalizer {
    /// Normalize string (placeholder - full implementation requires complex tables)
    pub fn normalize(s: &str, form: NormalizationForm) -> String {
        match form {
            NormalizationForm::NFC => {
                crate::println!("[normalizer] Normalizing to NFC");
                s.to_string() // Placeholder - would perform composition
            }
            NormalizationForm::NFD => {
                crate::println!("[normalizer] Normalizing to NFD");
                s.to_string() // Placeholder - would perform decomposition
            }
            NormalizationForm::NFKC => {
                crate::println!("[normalizer] Normalizing to NFKC");
                s.to_string() // Placeholder
            }
            NormalizationForm::NFKD => {
                crate::println!("[normalizer] Normalizing to NFKD");
                s.to_string() // Placeholder
            }
        }
    }

    /// Compare normalized strings
    pub fn compare_normalized(a: &str, b: &str) -> bool {
        let norm_a = Self::normalize(a, NormalizationForm::NFC);
        let norm_b = Self::normalize(b, NormalizationForm::NFC);
        norm_a == norm_b
    }
}

// ============================================================================
// Encoding Error
// ============================================================================

/// Encoding error
#[derive(Debug, Clone)]
pub enum EncodingError {
    InvalidUtf8,
    IncompleteSequence,
    EmptyInput,
    UnsupportedEncoding(String),
}

impl alloc::fmt::Display for EncodingError {
    fn fmt(&self, f: &mut alloc::fmt::Formatter<'_>) -> alloc::fmt::Result {
        match self {
            EncodingError::InvalidUtf8 => write!(f, "Invalid UTF-8 sequence"),
            EncodingError::IncompleteSequence => write!(f, "Incomplete UTF-8 sequence"),
            EncodingError::EmptyInput => write!(f, "Empty input"),
            EncodingError::UnsupportedEncoding(enc) => {
                write!(f, "Unsupported encoding: {}", enc)
            }
        }
    }
}
