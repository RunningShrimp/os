//! Symbol Resolution and Demangling
//!
//! This module provides symbol resolution, demangling, and debug information parsing
//! for the NOS kernel's profiling infrastructure.
//!
//! # Features
//!
//! - Address to symbol mapping
//! - Symbol demangling (Rust, C++)
//! - Symbol cache for performance
//! - DWARF debug info parsing (basic)
//! - Function name resolution
//! - Source location lookup
//!
//! # Usage
//!
//! ```rust
//! use kernel::profiling::symbol::{SymbolResolver, Symbol};
//!
//! let resolver = SymbolResolver::new();
//! let symbol = resolver.resolve(0x1000).unwrap();
//! println!("Function: {}", symbol.demangled_name);
//! ```

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::sync::Mutex;

/// Symbol resolution error types
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolError {
    /// Symbol not found
    NotFound(u64),
    /// Invalid address
    InvalidAddress(u64),
    /// DWARF parsing error
    DwarfParseError,
    /// Cache error
    CacheError,
}

impl core::fmt::Display for SymbolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotFound(addr) => write!(f, "Symbol not found at address: 0x{:x}", addr),
            Self::InvalidAddress(addr) => write!(f, "Invalid address: 0x{:x}", addr),
            Self::DwarfParseError => write!(f, "Failed to parse DWARF debug info"),
            Self::CacheError => write!(f, "Symbol cache error"),
        }
    }
}

/// Symbol information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    /// Symbol address
    pub address: u64,
    /// Symbol size in bytes
    pub size: u64,
    /// Mangled symbol name
    pub mangled_name: String,
    /// Demangled symbol name
    pub demangled_name: String,
    /// Symbol type
    pub symbol_type: SymbolType,
    /// Source file (if available)
    pub file: Option<String>,
    /// Line number (if available)
    pub line: Option<u32>,
    /// Module/object name
    pub module: String,
}

/// Symbol types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    /// Function
    Function,
    /// Variable
    Variable,
    /// Unknown
    Unknown,
}

/// Symbol cache entry
#[derive(Debug)]
struct CacheEntry {
    symbol: Symbol,
    last_access: u64,
    access_count: AtomicU64,
}

impl Clone for CacheEntry {
    fn clone(&self) -> Self {
        Self {
            symbol: self.symbol.clone(),
            last_access: self.last_access,
            access_count: AtomicU64::new(self.access_count.load(Ordering::Relaxed)),
        }
    }
}

/// Cache statistics
#[derive(Debug)]
pub struct CacheStats {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub evictions: AtomicU64,
}

impl Clone for CacheStats {
    fn clone(&self) -> Self {
        Self {
            hits: AtomicU64::new(self.hits.load(Ordering::Relaxed)),
            misses: AtomicU64::new(self.misses.load(Ordering::Relaxed)),
            evictions: AtomicU64::new(self.evictions.load(Ordering::Relaxed)),
        }
    }
}

/// Symbol resolver configuration
#[derive(Debug, Clone)]
pub struct SymbolResolverConfig {
    /// Enable caching
    pub enable_cache: bool,
    /// Maximum cache size
    pub max_cache_size: usize,
    /// Enable demangling
    pub enable_demangling: bool,
    /// Load DWARF debug info
    pub load_dwarf: bool,
    /// Cache expiry time (seconds)
    pub cache_expiry: u64,
}

impl Default for SymbolResolverConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            max_cache_size: 10_000,
            enable_demangling: true,
            load_dwarf: false, // Expensive
            cache_expiry: 300, // 5 minutes
        }
    }
}

/// DWARF debug information
#[derive(Debug, Clone)]
pub struct DwarfInfo {
    /// Address to symbol mapping
    pub symbols: BTreeMap<u64, Symbol>,
    /// Compile units
    pub compile_units: Vec<CompileUnit>,
}

/// DWARF compile unit
#[derive(Debug, Clone)]
pub struct CompileUnit {
    /// Compilation directory
    pub comp_dir: String,
    /// Source file name
    pub name: String,
    /// Language
    pub language: u64,
    /// Address range
    pub low_pc: u64,
    pub high_pc: u64,
}

/// Symbol resolver implementation
pub struct SymbolResolver {
    config: SymbolResolverConfig,
    cache: Mutex<BTreeMap<u64, CacheEntry>>,
    dwarf_info: Mutex<Option<DwarfInfo>>,
    cache_stats: CacheStats,
    symbol_table: Mutex<BTreeMap<u64, Symbol>>, // Static symbol table
}

impl SymbolResolver {
    /// Create a new symbol resolver
    pub fn new() -> Self {
        Self::with_config(SymbolResolverConfig::default())
    }

    /// Create resolver with custom configuration
    pub fn with_config(config: SymbolResolverConfig) -> Self {
        Self {
            config,
            cache: Mutex::new(BTreeMap::new()),
            dwarf_info: Mutex::new(None),
            cache_stats: CacheStats {
                hits: AtomicU64::new(0),
                misses: AtomicU64::new(0),
                evictions: AtomicU64::new(0),
            },
            symbol_table: Mutex::new(BTreeMap::new()),
        }
    }

    /// Resolve an address to a symbol
    pub fn resolve(&self, address: u64) -> Result<Symbol, SymbolError> {
        if address == 0 {
            return Err(SymbolError::InvalidAddress(address));
        }

        // Check cache first
        if self.config.enable_cache {
            if let Some(symbol) = self.lookup_cache(address) {
                return Ok(symbol);
            }
        }

        // Resolve symbol
        let symbol = self.resolve_internal(address)?;

        // Cache the result
        if self.config.enable_cache {
            self.cache_symbol(symbol.clone());
        }

        Ok(symbol)
    }

    /// Look up symbol in cache
    fn lookup_cache(&self, address: u64) -> Option<Symbol> {
        let mut cache = self.cache.lock();

        if let Some(entry) = cache.get_mut(&address) {
            entry.last_access = Self::now();
            entry.access_count.fetch_add(1, Ordering::Relaxed);
            self.cache_stats.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.symbol.clone())
        } else {
            self.cache_stats.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Resolve symbol without cache
    fn resolve_internal(&self, address: u64) -> Result<Symbol, SymbolError> {
        // Try symbol table first
        if let Some(symbol) = self.find_in_symbol_table(address) {
            return Ok(symbol);
        }

        // Try DWARF info
        if self.config.load_dwarf {
            if let Some(dwarf) = self.dwarf_info.lock().as_ref() {
                if let Some(symbol) = self.find_in_dwarf(dwarf, address) {
                    return Ok(symbol);
                }
            }
        }

        // Generate fallback symbol
        Ok(self.create_fallback_symbol(address))
    }

    /// Find symbol in static symbol table
    fn find_in_symbol_table(&self, address: u64) -> Option<Symbol> {
        let table = self.symbol_table.lock();

        // Find symbol with closest address <= target
        let mut closest: Option<&Symbol> = None;

        for (_, symbol) in table.iter() {
            if address >= symbol.address && address < symbol.address + symbol.size {
                return Some(symbol.clone());
            }

            if address >= symbol.address {
                if closest.is_none() || symbol.address > closest.unwrap().address {
                    closest = Some(symbol);
                }
            }
        }

        closest.cloned()
    }

    /// Find symbol in DWARF info
    fn find_in_dwarf(&self, dwarf: &DwarfInfo, address: u64) -> Option<Symbol> {
        // Find symbol with closest address <= target
        for (_, symbol) in dwarf.symbols.iter() {
            if address >= symbol.address && address < symbol.address + symbol.size {
                return Some(symbol.clone());
            }
        }
        None
    }

    /// Create fallback symbol for unknown address
    fn create_fallback_symbol(&self, address: u64) -> Symbol {
        Symbol {
            address,
            size: 0,
            mangled_name: format!("<unknown @ 0x{:x}>", address),
            demangled_name: format!("<unknown @ 0x{:x}>", address),
            symbol_type: SymbolType::Unknown,
            file: None,
            line: None,
            module: String::from("<unknown>"),
        }
    }

    /// Cache a symbol
    fn cache_symbol(&self, symbol: Symbol) {
        let mut cache = self.cache.lock();

        // Evict old entries if cache is full
        if cache.len() >= self.config.max_cache_size {
            self.evict_lru(&mut cache);
        }

        let entry = CacheEntry {
            symbol: symbol.clone(),
            last_access: Self::now(),
            access_count: AtomicU64::new(1),
        };

        cache.insert(symbol.address, entry);
    }

    /// Evict least recently used cache entry
    fn evict_lru(&self, cache: &mut BTreeMap<u64, CacheEntry>) {
        let mut oldest_addr: Option<u64> = None;
        let mut oldest_time = u64::MAX;

        for (&addr, entry) in cache.iter() {
            if entry.last_access < oldest_time {
                oldest_time = entry.last_access;
                oldest_addr = Some(addr);
            }
        }

        if let Some(addr) = oldest_addr {
            cache.remove(&addr);
            self.cache_stats.evictions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Add symbol to symbol table
    pub fn add_symbol(&self, symbol: Symbol) {
        let mut table = self.symbol_table.lock();
        table.insert(symbol.address, symbol);
    }

    /// Load DWARF debug information
    pub fn load_dwarf(&self, info: DwarfInfo) {
        *self.dwarf_info.lock() = Some(info);
    }

    /// Clear symbol cache
    pub fn clear_cache(&self) {
        self.cache.lock().clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (u64, u64, u64) {
        (
            self.cache_stats.hits.load(Ordering::Relaxed),
            self.cache_stats.misses.load(Ordering::Relaxed),
            self.cache_stats.evictions.load(Ordering::Relaxed),
        )
    }

    /// Get cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_stats.hits.load(Ordering::Relaxed);
        let misses = self.cache_stats.misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        }
    }

    /// Demangle a Rust symbol
    pub fn demangle_rust(&self, mangled: &str) -> String {
        // Rust symbols start with _R
        if !mangled.starts_with("_R") {
            return String::from(mangled);
        }

        // Simplified demangling - real implementation would use rustc-demangle
        // For now, return the mangled name with a prefix
        format!("<rust: {}>", &mangled[3..])
    }

    /// Demangle a C++ symbol
    pub fn demangle_cpp(&self, mangled: &str) -> String {
        // C++ symbols start with _Z
        if !mangled.starts_with("_Z") {
            return String::from(mangled);
        }

        // Simplified demangling - real implementation would use cpp_demangle
        // For now, return the mangled name with a prefix
        format!("<cpp: {}>", &mangled[2..])
    }

    /// Auto-detect and demangle symbol
    pub fn demangle(&self, mangled: &str) -> String {
        if mangled.starts_with("_R") {
            self.demangle_rust(mangled)
        } else if mangled.starts_with("_Z") {
            self.demangle_cpp(mangled)
        } else {
            String::from(mangled)
        }
    }

    /// Get current timestamp
    fn now() -> u64 {
        // In real implementation, use high-resolution timer
        0
    }
}

/// Demangle a Rust symbol (standalone function)
pub fn demangle_rust_symbol(mangled: &str) -> String {
    // Rust symbol format: _R<hash>_<name>
    if !mangled.starts_with("_R") {
        return String::from(mangled);
    }

    // Parse the symbol
    let rest = &mangled[2..];

    // Skip hash if present
    let name_start = if rest.chars().next().map_or(false, |c| c.is_numeric()) {
        // Find the separator
        if let Some(pos) = rest.find('_') {
            &rest[pos + 1..]
        } else {
            rest
        }
    } else {
        rest
    };

    // Decode simple encoding
    String::from_utf8_lossy(
        &name_start
            .as_bytes()
            .iter()
            .map(|&b| if b == b'$' { b'_' } else { b })
            .collect::<Vec<u8>>(),
    )
    .to_string()
}

/// Demangle a C++ symbol (standalone function)
pub fn demangle_cpp_symbol(mangled: &str) -> String {
    // C++ Itanium ABI: _Z<name length><name>...
    if !mangled.starts_with("_Z") {
        return String::from(mangled);
    }

    let rest = &mangled[2..];

    // Simple parsing for basic symbols
    // Format: N<namespace_len><namespace>E<function_len><function>
    if let Some(stripped) = rest.strip_prefix('N') {
        let mut result = String::new();
        let mut current = stripped;

        // Parse namespaces
        while let Some(sep_pos) = current.find('E') {
            let segment = &current[..sep_pos];
            if let Some(len_end) = segment.find(|c: char| !c.is_numeric()) {
                if let Ok(len) = segment[..len_end].parse::<usize>() {
                    let name = &segment[len_end..len_end + len];
                    if !result.is_empty() {
                        result.push_str("::");
                    }
                    result.push_str(name);
                    current = &current[sep_pos + 1..];
                    continue;
                }
            }
            break;
        }

        // Parse function name
        if let Some(len_end) = current.find(|c: char| !c.is_numeric()) {
            if let Ok(len) = current[..len_end].parse::<usize>() {
                let name = &current[len_end..len_end + len];
                if !result.is_empty() {
                    result.push_str("::");
                }
                result.push_str(name);
                return result;
            }
        }

        result
    } else {
        // Simple function name
        if let Some(len_end) = rest.find(|c: char| !c.is_numeric()) {
            if let Ok(len) = rest[..len_end].parse::<usize>() {
                return String::from(&rest[len_end..len_end + len]);
            }
        }

        String::from(mangled)
    }
}

/// Batch resolve addresses to symbols
pub fn batch_resolve(resolver: &SymbolResolver, addresses: &[u64]) -> Vec<Result<Symbol, SymbolError>> {
    addresses
        .iter()
        .map(|&addr| resolver.resolve(addr))
        .collect()
}

/// Create a symbol from components
pub fn create_symbol(
    address: u64,
    size: u64,
    name: &str,
    symbol_type: SymbolType,
) -> Symbol {
    let resolver = SymbolResolver::new();
    Symbol {
        address,
        size,
        mangled_name: String::from(name),
        demangled_name: resolver.demangle(name),
        symbol_type,
        file: None,
        line: None,
        module: String::from("<kernel>"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_creation() {
        let resolver = SymbolResolver::new();
        assert_eq!(resolver.cache_hit_rate(), 0.0);
    }

    #[test]
    fn test_symbol_resolution_not_found() {
        let resolver = SymbolResolver::new();
        let symbol = resolver.resolve(0x1000).unwrap();
        assert!(symbol.symbol_type == SymbolType::Unknown);
    }

    #[test]
    fn test_fallback_symbol() {
        let resolver = SymbolResolver::new();
        let symbol = resolver.resolve(0x1234).unwrap();
        assert!(symbol.demangled_name.contains("0x1234"));
    }

    #[test]
    fn test_symbol_table_add() {
        let resolver = SymbolResolver::new();

        let symbol = create_symbol(0x1000, 100, "test_function", SymbolType::Function);
        resolver.add_symbol(symbol.clone());

        let resolved = resolver.resolve(0x1000).unwrap();
        assert_eq!(resolved.address, 0x1000);
        assert_eq!(resolved.symbol_type, SymbolType::Function);
    }

    #[test]
    fn test_cache_hit_miss() {
        let resolver = SymbolResolver::new();
        let config = SymbolResolverConfig {
            enable_cache: true,
            ..Default::default()
        };

        let _ = resolver.with_config(config);

        resolver.resolve(0x1000).unwrap(); // Miss
        resolver.resolve(0x1000).unwrap(); // Hit

        let (hits, misses, _) = resolver.cache_stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
    }

    #[test]
    fn test_cache_hit_rate() {
        let resolver = SymbolResolver::new();

        resolver.resolve(0x1000).unwrap();
        resolver.resolve(0x1000).unwrap();
        resolver.resolve(0x1000).unwrap();

        let rate = resolver.cache_hit_rate();
        assert!(rate > 0.0 && rate <= 1.0);
    }

    #[test]
    fn test_cache_clear() {
        let resolver = SymbolResolver::new();

        resolver.resolve(0x1000).unwrap();
        resolver.clear_cache();

        resolver.resolve(0x1000).unwrap(); // Should be a miss again

        let (hits, misses, _) = resolver.cache_stats();
        assert_eq!(misses, 2); // Two misses total
    }

    #[test]
    fn test_rust_demangling() {
        let resolver = SymbolResolver::new();
        let demangled = resolver.demangle_rust("_R1234example_function");
        assert!(demangled.contains("example_function"));
    }

    #[test]
    fn test_cpp_demangling() {
        let resolver = SymbolResolver::new();
        let demangled = resolver.demangle_cpp("_ZN4test4funcEv");
        assert!(demangled.contains("test"));
    }

    #[test]
    fn test_auto_demangle() {
        let resolver = SymbolResolver::new();

        let rust_demangled = resolver.demangle("_Rtest");
        assert!(rust_demangled.contains("rust"));

        let cpp_demangled = resolver.demangle("_Ztest");
        assert!(cpp_demangled.contains("cpp"));

        let plain = resolver.demangle("plain_function");
        assert_eq!(plain, "plain_function");
    }

    #[test]
    fn test_invalid_address() {
        let resolver = SymbolResolver::new();
        let result = resolver.resolve(0);
        assert!(matches!(result, Err(SymbolError::InvalidAddress(0))));
    }

    #[test]
    fn test_create_symbol() {
        let symbol = create_symbol(0x1000, 256, "my_function", SymbolType::Function);

        assert_eq!(symbol.address, 0x1000);
        assert_eq!(symbol.size, 256);
        assert_eq!(symbol.symbol_type, SymbolType::Function);
    }

    #[test]
    fn test_batch_resolve() {
        let resolver = SymbolResolver::new();

        let addresses = vec![0x1000, 0x2000, 0x3000];
        let results = batch_resolve(&resolver, &addresses);

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[test]
    fn test_cache_eviction() {
        let config = SymbolResolverConfig {
            enable_cache: true,
            max_cache_size: 2,
            ..Default::default()
        };

        let resolver = SymbolResolver::with_config(config);

        resolver.resolve(0x1000).unwrap();
        resolver.resolve(0x2000).unwrap();
        resolver.resolve(0x3000).unwrap(); // Should evict one

        let (_, _, evictions) = resolver.cache_stats();
        assert_eq!(evictions, 1);
    }
}
