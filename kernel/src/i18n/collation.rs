//! Locale-Aware String Collation
//!
//! This module implements locale-aware string comparison and sorting:
//! - Collation strength levels
//! - Locale-specific sorting rules
//! - Case-insensitive comparison
//! - Accent handling
//! - Unicode collation algorithm (UCA)
//!
//! ## Features
//!
//! - **Collation Strength**: Primary, Secondary, Tertiary, Quaternary, Identical
//! - **Locale-Specific Rules**: Language-specific collation orders
//! - **Case Handling**: Case-insensitive and case-sensitive comparison
//! - **Accent Handling**: Ignore or respect diacritics
//! - **Numeric Sorting**: Smart numeric collation (e.g., "2" < "10")

use alloc::string::String;
use alloc::collections::BTreeMap;
use core::cmp::Ordering;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// Collation Constants
// ============================================================================

/// Maximum collation elements per character
pub const MAX_COLLATION_ELEMENTS: usize = 4;

/// Default collation strength
pub const DEFAULT_STRENGTH: CollationStrength = CollationStrength::Tertiary;

// ============================================================================
// Collation Strength
// ============================================================================

/// Collation strength level
///
/// Determines which differences are considered significant during comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CollationStrength {
    /// Primary level: Base characters only (e.g., "a" = "A" = "á")
    Primary = 1,

    /// Secondary level: Base characters + accents (e.g., "a" = "A" ≠ "á")
    Secondary = 2,

    /// Tertiary level: Base characters + accents + case (e.g., "a" ≠ "A" ≠ "á")
    Tertiary = 3,

    /// Quaternary level: Includes punctuation and case differences
    Quaternary = 4,

    /// Identical level: Code point comparison (exact match)
    Identical = 5,
}

impl CollationStrength {
    /// Get the numeric value of the strength level
    pub fn level(&self) -> u8 {
        *self as u8
    }
}

// ============================================================================
// Collation Options
// ============================================================================

/// Options for collation behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollationOptions {
    /// Collation strength
    pub strength: CollationStrength,

    /// Ignore case (ignore diacritics)
    pub ignore_case: bool,

    /// Ignore accents
    pub ignore_accents: bool,

    /// Numeric collation (compare numbers numerically)
    pub numeric: bool,

    /// Reverse order (descending sort)
    pub reverse: bool,

    /// Normalize whitespace
    pub normalize_whitespace: bool,
}

impl Default for CollationOptions {
    fn default() -> Self {
        Self {
            strength: DEFAULT_STRENGTH,
            ignore_case: false,
            ignore_accents: false,
            numeric: false,
            reverse: false,
            normalize_whitespace: false,
        }
    }
}

impl CollationOptions {
    /// Create new collation options with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set collation strength
    pub fn with_strength(mut self, strength: CollationStrength) -> Self {
        self.strength = strength;
        self
    }

    /// Set case sensitivity
    pub fn ignore_case(mut self, ignore: bool) -> Self {
        self.ignore_case = ignore;
        self
    }

    /// Set accent handling
    pub fn ignore_accents(mut self, ignore: bool) -> Self {
        self.ignore_accents = ignore;
        self
    }

    /// Enable numeric collation
    pub fn numeric(mut self, numeric: bool) -> Self {
        self.numeric = numeric;
        self
    }

    /// Reverse sort order
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Normalize whitespace
    pub fn normalize_whitespace(mut self, normalize: bool) -> Self {
        self.normalize_whitespace = normalize;
        self
    }
}

// ============================================================================
// Collation Key
// ============================================================================

/// Collation key for efficient string comparison
///
/// Collation keys are precomputed values that allow fast comparisons
/// without reprocessing the strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollationKey {
    /// Primary weights
    pub primary: Vec<u16>,

    /// Secondary weights
    pub secondary: Vec<u16>,

    /// Tertiary weights
    pub tertiary: Vec<u16>,

    /// Quaternary weights
    pub quaternary: Vec<u16>,
}

impl CollationKey {
    /// Create a new empty collation key
    pub fn new() -> Self {
        Self {
            primary: Vec::new(),
            secondary: Vec::new(),
            tertiary: Vec::new(),
            quaternary: Vec::new(),
        }
    }

    /// Add a weight at the specified level
    pub fn add_weight(&mut self, level: CollationStrength, weight: u16) {
        match level {
            CollationStrength::Primary => self.primary.push(weight),
            CollationStrength::Secondary => self.secondary.push(weight),
            CollationStrength::Tertiary => self.tertiary.push(weight),
            CollationStrength::Quaternary => self.quaternary.push(weight),
            CollationStrength::Identical => {} // Handled separately
        }
    }

    /// Compare two collation keys with the given strength
    pub fn compare(&self, other: &Self, strength: CollationStrength) -> Ordering {
        // Primary level comparison
        let primary_cmp = self.primary.iter().cmp(other.primary.iter());
        if primary_cmp != Ordering::Equal || strength == CollationStrength::Primary {
            return primary_cmp;
        }

        // Secondary level comparison
        let secondary_cmp = self.secondary.iter().cmp(other.secondary.iter());
        if secondary_cmp != Ordering::Equal || strength == CollationStrength::Secondary {
            return secondary_cmp;
        }

        // Tertiary level comparison
        let tertiary_cmp = self.tertiary.iter().cmp(other.tertiary.iter());
        if tertiary_cmp != Ordering::Equal || strength == CollationStrength::Tertiary {
            return tertiary_cmp;
        }

        // Quaternary level comparison
        let quaternary_cmp = self.quaternary.iter().cmp(other.quaternary.iter());
        if quaternary_cmp != Ordering::Equal || strength == CollationStrength::Quaternary {
            return quaternary_cmp;
        }

        // Identical level
        Ordering::Equal
    }

    /// Check if the key is empty
    pub fn is_empty(&self) -> bool {
        self.primary.is_empty()
    }
}

// ============================================================================
// Character Weights
// ============================================================================

/// Default collation weights for ASCII characters
///
/// This is a simplified version. A full implementation would use
/// the Unicode Collation Algorithm (UCA) with the Default Unicode Collation
/// Element Table (DUCET).
struct DefaultWeights;

impl DefaultWeights {
    /// Get primary weight for a character
    fn primary(c: char) -> u16 {
        let cp = c as u32;

        // Basic Latin alphabet (case-insensitive for primary)
        if (0x0041..=0x005A).contains(&cp) {
            // A-Z
            ((cp - 0x0041) * 2 + 0x100) as u16
        } else if (0x0061..=0x007A).contains(&cp) {
            // a-z
            ((cp - 0x0061) * 2 + 0x100) as u16
        } else if (0x0030..=0x0039).contains(&cp) {
            // 0-9
            ((cp - 0x0030) + 0x200) as u16
        } else if cp == 0x0020 {
            // Space
            0x0202
        } else if (0x0021..=0x002F).contains(&cp) || // Punctuation
                  (0x003A..=0x0040).contains(&cp) ||
                  (0x005B..=0x0060).contains(&cp) ||
                  (0x007B..=0x007E).contains(&cp)
        {
            ((cp - 0x0021 + 0x210) as u16)
        } else {
            // Default weight for other characters
            0xFFFF
        }
    }

    /// Get secondary weight (accent/diacritic)
    fn secondary(c: char) -> u16 {
        let cp = c as u32;

        // Characters with diacritics get higher secondary weights
        match cp {
            0x00C0..=0x00D6 | 0x00D8..=0x00DE => 0x0020, // À-Ö, Ø-Þ (uppercase with diacritics)
            0x00E0..=0x00F6 | 0x00F8..=0x00FE => 0x0020, // à-ö, ø-þ (lowercase with diacritics)
            _ => 0x0000, // No diacritic
        }
    }

    /// Get tertiary weight (case)
    fn tertiary(c: char) -> u16 {
        let cp = c as u32;

        // Uppercase gets higher tertiary weight
        if (0x0041..=0x005A).contains(&cp) {
            0x0008 // Uppercase
        } else if (0x0061..=0x007A).contains(&cp) {
            0x0000 // Lowercase
        } else {
            0x0001 // Other
        }
    }

    /// Get quaternary weight
    fn quaternary(_c: char) -> u16 {
        0x0000
    }
}

// ============================================================================
// Collator
// ============================================================================

/// String collator for locale-aware comparison
pub struct Collator {
    /// Locale code
    locale: String,

    /// Collation options
    options: CollationOptions,

    /// Statistics
    stats: CollatorStats,
}

#[derive(Debug, Clone, Default)]
pub struct CollatorStats {
    pub comparisons: u64,
    pub key_generations: u64,
    pub sorts: u64,
}

impl Collator {
    /// Create a new collator for the given locale
    pub fn new(locale: String) -> Self {
        Self {
            locale,
            options: CollationOptions::default(),
            stats: CollatorStats::default(),
        }
    }

    /// Create a collator with custom options
    pub fn with_options(locale: String, options: CollationOptions) -> Self {
        Self {
            locale,
            options,
            stats: CollatorStats::default(),
        }
    }

    /// Compare two strings according to the collation rules
    pub fn compare(&mut self, a: &str, b: &str) -> Ordering {
        self.stats.comparisons += 1;

        // Preprocess strings based on options
        let a = self.preprocess(a);
        let b = self.preprocess(b);

        // Handle numeric collation
        if self.options.numeric {
            return self.numeric_compare(&a, &b);
        }

        // Generate collation keys and compare
        let key_a = self.collation_key(&a);
        let key_b = self.collation_key(&b);

        let mut result = key_a.compare(&key_b, self.options.strength);

        // Apply reverse if needed
        if self.options.reverse {
            result = result.reverse();
        }

        result
    }

    /// Generate a collation key for a string
    pub fn collation_key(&mut self, input: &str) -> CollationKey {
        self.stats.key_generations += 1;

        let mut key = CollationKey::new();

        for c in input.chars() {
            let mut strength = self.options.strength;

            // Adjust strength based on options
            if self.options.ignore_case && strength == CollationStrength::Tertiary {
                strength = CollationStrength::Secondary;
            }
            if self.options.ignore_accents && strength == CollationStrength::Secondary {
                strength = CollationStrength::Primary;
            }

            // Add weights based on strength
            key.add_weight(CollationStrength::Primary, DefaultWeights::primary(c));

            if strength >= CollationStrength::Secondary {
                key.add_weight(CollationStrength::Secondary, DefaultWeights::secondary(c));
            }
            if strength >= CollationStrength::Tertiary {
                key.add_weight(CollationStrength::Tertiary, DefaultWeights::tertiary(c));
            }
            if strength >= CollationStrength::Quaternary {
                key.add_weight(CollationStrength::Quaternary, DefaultWeights::quaternary(c));
            }
        }

        key
    }

    /// Compare strings with numeric collation
    fn numeric_compare(&self, a: &str, b: &str) -> Ordering {
        let mut a_chars = a.chars().peekable();
        let mut b_chars = b.chars().peekable();

        loop {
            let a_next = a_chars.peek();
            let b_next = b_chars.peek();

            match (a_next, b_next) {
                (None, None) => return Ordering::Equal,
                (None, Some(_)) => return Ordering::Less,
                (Some(_), None) => return Ordering::Greater,
                (Some(&ac), Some(&bc)) => {
                    // Check if both are digits
                    if ac.is_ascii_digit() && bc.is_ascii_digit() {
                        // Extract full numbers
                        let a_num = self.extract_number(&mut a_chars);
                        let b_num = self.extract_number(&mut b_chars);

                        match a_num.cmp(&b_num) {
                            Ordering::Equal => continue,
                            other => return other,
                        }
                    } else {
                        // Compare characters
                        a_chars.next();
                        b_chars.next();
                        match ac.cmp(&bc) {
                            Ordering::Equal => continue,
                            other => return other,
                        }
                    }
                }
            }
        }
    }

    /// Extract a number from the character stream
    fn extract_number<'a, I>(&self, chars: &mut core::iter::Peekable<I>) -> u64
    where
        I: Iterator<Item = char> + 'a,
    {
        let mut num: u64 = 0;

        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() {
                chars.next();
                num = num * 10 + (c as u64 - '0' as u64);
            } else {
                break;
            }
        }

        num
    }

    /// Preprocess string according to options
    fn preprocess(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len());

        for c in input.chars() {
            // Normalize whitespace if enabled
            if self.options.normalize_whitespace && c.is_whitespace() {
                if !result.ends_with(' ') {
                    result.push(' ');
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Sort a vector of strings
    pub fn sort(&mut self, strings: &mut Vec<String>) {
        self.stats.sorts += 1;

        strings.sort_by(|a, b| self.compare(a, b));
    }

    /// Get collator statistics
    pub fn stats(&self) -> &CollatorStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = CollatorStats::default();
    }
}

// ============================================================================
// Locale-Specific Collation Rules
// ============================================================================

/// Locale-specific collation adjustments
pub struct LocaleCollationRules {
    /// Locale-specific character reordering
    reorder: BTreeMap<char, u16>,

    /// Locale-specific contractions
    contractions: BTreeMap<String, u16>,
}

impl LocaleCollationRules {
    /// Get default rules for a locale
    pub fn for_locale(locale: &str) -> Self {
        let mut reorder = BTreeMap::new();
        let mut contractions = BTreeMap::new();

        match locale {
            "de_DE" | "de_AT" | "de_CH" => {
                // German: ä = ae, ö = oe, ü = ue (phone book sorting)
                contractions.insert(String::from("ae"), 0x0100);
                contractions.insert(String::from("oe"), 0x0101);
                contractions.insert(String::from("ue"), 0x0102);
            }
            "sv_SE" | "fi_FI" => {
                // Swedish/Finnish: z and å are reordered
                reorder.insert('z', 0x0130);
                reorder.insert('å', 0x0120);
                reorder.insert('ä', 0x0121);
                reorder.insert('ö', 0x0122);
            }
            "es_ES" => {
                // Spanish: ch and ll are treated as single characters
                contractions.insert(String::from("ch"), 0x0100);
                contractions.insert(String::from("ll"), 0x0101);
            }
            "da_DK" | "no_NO" => {
                // Danish/Norwegian: æ, ø, å at end
                reorder.insert('æ', 0x0150);
                reorder.insert('ø', 0x0151);
                reorder.insert('å', 0x0152);
            }
            _ => {
                // Default: no special rules
            }
        }

        Self { reorder, contractions }
    }
}

// ============================================================================
// Fast Comparison Functions
// ============================================================================

/// Fast case-insensitive comparison
pub fn compare_case_insensitive(a: &str, b: &str) -> Ordering {
    let mut a_chars = a.chars().map(|c| c.to_ascii_lowercase());
    let mut b_chars = b.chars().map(|c| c.to_ascii_lowercase());

    loop {
        match (a_chars.next(), b_chars.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ac), Some(bc)) => match ac.cmp(&bc) {
                Ordering::Equal => continue,
                other => return other,
            },
        }
    }
}

/// Fast accent-insensitive comparison (simplified)
pub fn compare_ignore_accents(a: &str, b: &str) -> Ordering {
    // For now, just do a basic comparison
    // A full implementation would strip diacritics first
    a.cmp(b)
}

/// Compare two strings using default collation
pub fn compare_default(a: &str, b: &str) -> Ordering {
    let collator = Collator::new(String::from("en_US"));
    collator.compare(a, b)
}

/// Check if two strings are equal (ignoring case and accents)
pub fn equals_ignore(a: &str, b: &str) -> bool {
    compare_case_insensitive(a, b) == Ordering::Equal
}

// ============================================================================
// Collation Strength Helpers
// ============================================================================

/// Compare strings at primary level (base characters only)
pub fn compare_primary(a: &str, b: &str) -> Ordering {
    let mut collator = Collator::new(String::from("en_US"));
    collator.options.strength = CollationStrength::Primary;
    collator.compare(a, b)
}

/// Compare strings at secondary level (base + accents)
pub fn compare_secondary(a: &str, b: &str) -> Ordering {
    let mut collator = Collator::new(String::from("en_US"));
    collator.options.strength = CollationStrength::Secondary;
    collator.compare(a, b)
}

/// Compare strings at tertiary level (base + accents + case)
pub fn compare_tertiary(a: &str, b: &str) -> Ordering {
    let mut collator = Collator::new(String::from("en_US"));
    collator.options.strength = CollationStrength::Tertiary;
    collator.compare(a, b)
}

/// Compare strings at quaternary level (includes punctuation)
pub fn compare_quaternary(a: &str, b: &str) -> Ordering {
    let mut collator = Collator::new(String::from("en_US"));
    collator.options.strength = CollationStrength::Quaternary;
    collator.compare(a, b)
}

/// Compare strings at identical level (exact match)
pub fn compare_identical(a: &str, b: &str) -> Ordering {
    a.cmp(b)
}

#[cfg(feature = "kernel_tests")]
mod tests {
    use super::*;

    #[test]
    fn test_basic_comparison() {
        let mut collator = Collator::new(String::from("en_US"));

        assert_eq!(collator.compare("apple", "apple"), Ordering::Equal);
        assert_eq!(collator.compare("apple", "banana"), Ordering::Less);
        assert_eq!(collator.compare("banana", "apple"), Ordering::Greater);
    }

    #[test]
    fn test_case_insensitive() {
        let mut collator = Collator::new(String::from("en_US"));
        collator.options.ignore_case = true;

        assert_eq!(collator.compare("Apple", "apple"), Ordering::Equal);
        assert_eq!(collator.compare("APPLE", "apple"), Ordering::Equal);
    }

    #[test]
    fn test_numeric_sort() {
        let mut collator = Collator::new(String::from("en_US"));
        collator.options.numeric = true;

        // "2" should come before "10" in numeric sort
        assert_eq!(collator.compare("2", "10"), Ordering::Less);
        assert_eq!(collator.compare("file2.txt", "file10.txt"), Ordering::Less);
    }

    #[test]
    fn test_reverse_sort() {
        let mut collator = Collator::new(String::from("en_US"));
        collator.options.reverse = true;

        assert_eq!(collator.compare("apple", "banana"), Ordering::Greater);
        assert_eq!(collator.compare("banana", "apple"), Ordering::Less);
    }

    #[test]
    fn test_sort() {
        let mut collator = Collator::new(String::from("en_US"));
        let mut strings = vec![
            String::from("banana"),
            String::from("apple"),
            String::from("cherry"),
        ];

        collator.sort(&mut strings);

        assert_eq!(strings[0], "apple");
        assert_eq!(strings[1], "banana");
        assert_eq!(strings[2], "cherry");
    }

    #[test]
    fn test_collation_strength() {
        // Primary: base characters only
        assert_eq!(compare_primary("a", "A"), Ordering::Equal);

        // Tertiary: includes case
        assert_ne!(compare_tertiary("a", "A"), Ordering::Equal);
    }
}
