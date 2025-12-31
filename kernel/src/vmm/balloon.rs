//! # Memory Ballooning Driver
//!
//! This module implements a memory balloon driver for dynamic memory management
//! in virtual machines. The balloon driver allows the host to reclaim memory from
//! the guest by inflating a "balloon" - effectively requesting pages from the guest
//! that can be reused by the host.
//!
//! ## Features
//!
//! - Virtio balloon device interface
//! - Dynamic memory allocation/deallocation
//! - Memory pressure handling
//! - Page reclaim strategies
//! - Statistics reporting
//! - Automatic ballooning based on memory pressure
//!
//! ## Architecture
//!
//! The balloon driver works by communicating with the host through the Virtio
//! balloon device. When the host needs memory, it sends requests to inflate the
//! balloon (reclaim pages). When memory pressure decreases, the balloon can
//! deflate (return pages to the guest).
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::vmm::balloon::{BalloonDriver, BalloonConfig};
//!
//! let config = BalloonConfig::default();
//! let balloon = BalloonDriver::new(config)?;
//! balloon.inflate(1024)?; // Inflate by 1024 pages
//! ```

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;

use crate::error::KernelError;
use crate::memory::PhysicalAddress;

/// Default number of pages for balloon operations
const DEFAULT_PAGE_SIZE: usize = 4096;

/// Maximum balloon size in pages
const MAX_BALLOON_PAGES: u64 = 1024 * 1024; // 4GB with 4KB pages

/// Balloon operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalloonOp {
    /// Inflate: reclaim pages from guest
    Inflate,
    /// Deflate: return pages to guest
    Deflate,
}

/// Balloon statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct BalloonStats {
    /// Current number of pages in balloon
    pub current_pages: u64,
    /// Target number of pages
    pub target_pages: u64,
    /// Total pages ever inflated
    pub total_inflated: u64,
    /// Total pages ever deflated
    pub total_deflated: u64,
    /// Number of inflate operations
    pub inflate_count: u64,
    /// Number of deflate operations
    pub deflate_count: u64,
    /// Memory pressure (0-100)
    pub memory_pressure: u8,
    /// Swap in operations
    pub swap_in: u64,
    /// Swap out operations
    pub swap_out: u64,
    /// Major page faults
    pub major_faults: u64,
    /// Minor page faults
    pub minor_faults: u64,
    /// Free memory in bytes
    pub free_memory: u64,
    /// Total memory in bytes
    pub total_memory: u64,
}

/// Memory reclaim strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReclaimStrategy {
    /// Reclaim from page cache
    PageCache,
    /// Reclaim from slab caches
    Slab,
    /// Reclaim from anonymous memory (swap)
    Anonymous,
    /// Reclaim from all sources
    All,
}

/// Balloon configuration
#[derive(Debug, Clone)]
pub struct BalloonConfig {
    /// Maximum number of pages
    pub max_pages: u64,
    /// Initial number of pages
    pub initial_pages: u64,
    /// Enable automatic ballooning
    pub auto_ballooning: bool,
    /// Memory pressure threshold (0-100)
    pub pressure_threshold: u8,
    /// Reclaim strategy
    pub reclaim_strategy: ReclaimStrategy,
    /// Statistics update interval (ms)
    pub stats_interval: u64,
    /// Enable deflate on low memory
    pub deflate_on_low_mem: bool,
}

impl Default for BalloonConfig {
    fn default() -> Self {
        Self {
            max_pages: MAX_BALLOON_PAGES,
            initial_pages: 0,
            auto_ballooning: true,
            pressure_threshold: 80,
            reclaim_strategy: ReclaimStrategy::All,
            stats_interval: 1000,
            deflate_on_low_mem: true,
        }
    }
}

/// Balloon page request
#[derive(Debug, Clone)]
pub struct PageRequest {
    /// Physical page numbers
    pub pages: Vec<u64>,
    /// Request type
    pub op: BalloonOp,
}

impl PageRequest {
    /// Create a new page request
    pub fn new(op: BalloonOp) -> Self {
        Self {
            pages: Vec::new(),
            op,
        }
    }

    /// Add pages to request
    pub fn add_pages(&mut self, page_numbers: &[u64]) {
        self.pages.extend_from_slice(page_numbers);
    }

    /// Get number of pages
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Check if request is empty
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }
}

/// Memory pressure event
#[derive(Debug, Clone, Copy)]
pub struct PressureEvent {
    /// Pressure level (0-100)
    pub level: u8,
    /// Available memory in bytes
    pub available: u64,
    /// Total memory in bytes
    pub total: u64,
    /// Whether to inflate or deflate
    pub action: PressureAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureAction {
    /// Inflate balloon
    Inflate,
    /// Deflate balloon
    Deflate,
    /// No action
    None,
}

/// Memory balloon driver
#[derive(Debug)]
pub struct BalloonDriver {
    /// Balloon configuration
    config: BalloonConfig,
    /// Current statistics
    stats: Arc<Mutex<BalloonStats>>,
    /// Ballooned pages
    ballooned_pages: Arc<Mutex<Vec<PhysicalAddress>>>,
    /// Page request queue
    request_queue: Arc<Mutex<VecDeque<PageRequest>>>,
    /// Driver enabled
    enabled: Arc<AtomicBool>,
    /// Should stop
    should_stop: Arc<AtomicBool>,
    /// Target number of pages
    target_pages: Arc<AtomicU64>,
    /// Total pages processed
    total_processed: Arc<AtomicU64>,
}

impl BalloonDriver {
    /// Create a new balloon driver
    pub fn new(config: BalloonConfig) -> Result<Self, KernelError> {
        let stats = BalloonStats {
            current_pages: config.initial_pages,
            target_pages: config.initial_pages,
            ..Default::default()
        };

        Ok(Self {
            config,
            stats: Arc::new(Mutex::new(stats)),
            ballooned_pages: Arc::new(Mutex::new(Vec::new())),
            request_queue: Arc::new(Mutex::new(VecDeque::new())),
            enabled: Arc::new(AtomicBool::new(false)),
            should_stop: Arc::new(AtomicBool::new(false)),
            target_pages: Arc::new(AtomicU64::new(0)),
            total_processed: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Get configuration
    pub fn config(&self) -> &BalloonConfig {
        &self.config
    }

    /// Enable the balloon driver
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable the balloon driver
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Get current statistics
    pub fn stats(&self) -> BalloonStats {
        *self.stats.lock()
    }

    /// Update statistics
    pub fn update_stats(&self, stats: BalloonStats) {
        *self.stats.lock() = stats;
    }

    /// Get current balloon size (pages)
    pub fn current_size(&self) -> u64 {
        self.stats.lock().current_pages
    }

    /// Get target balloon size (pages)
    pub fn target_size(&self) -> u64 {
        self.target_pages.load(Ordering::SeqCst)
    }

    /// Set target balloon size
    pub fn set_target_size(&self, pages: u64) -> Result<(), KernelError> {
        if pages > self.config.max_pages {
            return Err(KernelError::InvalidArgument(
                "Target pages exceeds maximum".into(),
            ));
        }

        self.target_pages.store(pages, Ordering::SeqCst);

        let mut stats = self.stats.lock();
        stats.target_pages = pages;

        Ok(())
    }

    /// Inflate balloon by specified number of pages
    pub fn inflate(&self, num_pages: u64) -> Result<(), KernelError> {
        if !self.is_enabled() {
            return Err(KernelError::InvalidState("Balloon driver not enabled".into()));
        }

        let current = self.current_size();
        let new_size = current.saturating_add(num_pages);

        if new_size > self.config.max_pages {
            return Err(KernelError::InvalidArgument(
                "Would exceed maximum balloon size".into(),
            ));
        }

        // Create page request
        let mut request = PageRequest::new(BalloonOp::Inflate);
        let start_page = current;
        request.add_pages(&(start_page..start_page + num_pages).collect::<Vec<_>>());

        // Queue request
        self.queue_request(request);

        // Update stats
        let mut stats = self.stats.lock();
        stats.current_pages = new_size;
        stats.total_inflated += num_pages;
        stats.inflate_count += 1;

        Ok(())
    }

    /// Deflate balloon by specified number of pages
    pub fn deflate(&self, num_pages: u64) -> Result<(), KernelError> {
        if !self.is_enabled() {
            return Err(KernelError::InvalidState("Balloon driver not enabled".into()));
        }

        let current = self.current_size();
        let num_pages = core::cmp::min(num_pages, current);
        let new_size = current.saturating_sub(num_pages);

        // Create page request
        let mut request = PageRequest::new(BalloonOp::Deflate);
        let start_page = new_size;
        request.add_pages(&(start_page..start_page + num_pages).collect::<Vec<_>>());

        // Queue request
        self.queue_request(request);

        // Update stats
        let mut stats = self.stats.lock();
        stats.current_pages = new_size;
        stats.total_deflated += num_pages;
        stats.deflate_count += 1;

        Ok(())
    }

    /// Process a page request
    pub fn process_request(&self, request: &PageRequest) -> Result<(), KernelError> {
        match request.op {
            BalloonOp::Inflate => {
                self.inflate_pages(&request.pages)?;
            }
            BalloonOp::Deflate => {
                self.deflate_pages(&request.pages)?;
            }
        }

        self.total_processed
            .fetch_add(request.pages.len() as u64, Ordering::SeqCst);

        Ok(())
    }

    /// Inflate pages (reclaim from guest)
    fn inflate_pages(&self, pages: &[u64]) -> Result<(), KernelError> {
        let mut ballooned = self.ballooned_pages.lock();

        for &page_num in pages {
            // In a real implementation, this would actually free the pages
            let addr = PhysicalAddress::new(page_num * DEFAULT_PAGE_SIZE as u64);
            ballooned.push(addr);
        }

        Ok(())
    }

    /// Deflate pages (return to guest)
    fn deflate_pages(&self, pages: &[u64]) -> Result<(), KernelError> {
        let mut ballooned = self.ballooned_pages.lock();

        for &page_num in pages {
            // In a real implementation, this would reallocate the pages
            let addr = PhysicalAddress::new(page_num * DEFAULT_PAGE_SIZE as u64);

            // Remove from ballooned list
            ballooned.retain(|&a| a != addr);
        }

        Ok(())
    }

    /// Queue a page request
    pub fn queue_request(&self, request: PageRequest) {
        let mut queue = self.request_queue.lock();
        queue.push_back(request);
    }

    /// Process next request in queue
    pub fn process_next_request(&self) -> Result<(), KernelError> {
        let mut queue = self.request_queue.lock();

        if let Some(request) = queue.pop_front() {
            drop(queue); // Release lock before processing
            self.process_request(&request)
        } else {
            Ok(())
        }
    }

    /// Process all pending requests
    pub fn process_all_requests(&self) -> Result<(), KernelError> {
        loop {
            let has_request = {
                let queue = self.request_queue.lock();
                !queue.is_empty()
            };

            if !has_request {
                break;
            }

            self.process_next_request()?;
        }

        Ok(())
    }

    /// Get number of pending requests
    pub fn pending_request_count(&self) -> usize {
        self.request_queue.lock().len()
    }

    /// Handle memory pressure event
    pub fn handle_pressure(&self, event: PressureEvent) -> Result<(), KernelError> {
        if !self.config.auto_ballooning {
            return Ok(());
        }

        match event.action {
            PressureAction::Inflate if event.level >= self.config.pressure_threshold => {
                // Inflate by 10% of target size
                let inflate_by = self.target_size() / 10;
                if inflate_by > 0 {
                    self.inflate(inflate_by)?;
                }
            }
            PressureAction::Deflate if event.level < self.config.pressure_threshold / 2 => {
                // Deflate by 10% of current size
                let deflate_by = self.current_size() / 10;
                if deflate_by > 0 && self.config.deflate_on_low_mem {
                    self.deflate(deflate_by)?;
                }
            }
            _ => {}
        }

        // Update pressure in stats
        let mut stats = self.stats.lock();
        stats.memory_pressure = event.level;
        stats.free_memory = event.available;
        stats.total_memory = event.total;

        Ok(())
    }

    /// Calculate memory pressure
    pub fn calculate_pressure(&self, available: u64, total: u64) -> u8 {
        if total == 0 {
            return 0;
        }

        let used_percent = ((total - available) * 100) / total;
        used_percent as u8
    }

    /// Create pressure event from memory stats
    pub fn create_pressure_event(&self, available: u64, total: u64) -> PressureEvent {
        let level = self.calculate_pressure(available, total);
        let action = if level >= self.config.pressure_threshold {
            PressureAction::Inflate
        } else if level < self.config.pressure_threshold / 2 && self.config.deflate_on_low_mem {
            PressureAction::Deflate
        } else {
            PressureAction::None
        };

        PressureEvent {
            level,
            available,
            total,
            action,
        }
    }

    /// Update page fault counters
    pub fn update_faults(&self, major: u64, minor: u64) {
        let mut stats = self.stats.lock();
        stats.major_faults = major;
        stats.minor_faults = minor;
    }

    /// Update swap counters
    pub fn update_swap(&self, swap_in: u64, swap_out: u64) {
        let mut stats = self.stats.lock();
        stats.swap_in = swap_in;
        stats.swap_out = swap_out;
    }

    /// Get total pages processed
    pub fn total_processed(&self) -> u64 {
        self.total_processed.load(Ordering::SeqCst)
    }

    /// Get ballooned pages
    pub fn ballooned_pages(&self) -> Vec<PhysicalAddress> {
        self.ballooned_pages.lock().clone()
    }

    /// Clear all ballooned pages
    pub fn clear(&self) -> Result<(), KernelError> {
        self.deflate(self.current_size())?;
        Ok(())
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        stats.total_inflated = 0;
        stats.total_deflated = 0;
        stats.inflate_count = 0;
        stats.deflate_count = 0;
        stats.major_faults = 0;
        stats.minor_faults = 0;
        stats.swap_in = 0;
        stats.swap_out = 0;
    }

    /// Check if balloon is at target size
    pub fn at_target(&self) -> bool {
        self.current_size() == self.target_size()
    }

    /// Get percentage of target size
    pub fn target_percentage(&self) -> f64 {
        let target = self.target_size();
        if target == 0 {
            0.0
        } else {
            (self.current_size() as f64 / target as f64) * 100.0
        }
    }
}

impl Default for BalloonDriver {
    fn default() -> Self {
        Self::new(BalloonConfig::default()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balloon_config() {
        let config = BalloonConfig::default();
        assert_eq!(config.initial_pages, 0);
        assert_eq!(config.pressure_threshold, 80);
        assert!(config.auto_ballooning);
    }

    #[test]
    fn test_balloon_driver_creation() {
        let driver = BalloonDriver::default();
        assert!(!driver.is_enabled());
        assert_eq!(driver.current_size(), 0);
    }

    #[test]
    fn test_enable_disable() {
        let driver = BalloonDriver::default();
        driver.enable();
        assert!(driver.is_enabled());
        driver.disable();
        assert!(!driver.is_enabled());
    }

    #[test]
    fn test_inflate() {
        let driver = BalloonDriver::default();
        driver.enable();
        driver.inflate(100).unwrap();

        assert_eq!(driver.current_size(), 100);
        assert_eq!(driver.stats().total_inflated, 100);
    }

    #[test]
    fn test_deflate() {
        let driver = BalloonDriver::default();
        driver.enable();
        driver.inflate(100).unwrap();
        driver.deflate(50).unwrap();

        assert_eq!(driver.current_size(), 50);
        assert_eq!(driver.stats().total_deflated, 50);
    }

    #[test]
    fn test_target_size() {
        let driver = BalloonDriver::default();
        driver.set_target_size(1000).unwrap();
        assert_eq!(driver.target_size(), 1000);
    }

    #[test]
    fn test_exceed_max_size() {
        let config = BalloonConfig {
            max_pages: 100,
            ..Default::default()
        };
        let driver = BalloonDriver::new(config).unwrap();
        driver.enable();

        let result = driver.inflate(200);
        assert!(result.is_err());
    }

    #[test]
    fn test_page_request() {
        let mut request = PageRequest::new(BalloonOp::Inflate);
        request.add_pages(&[1, 2, 3, 4, 5]);

        assert_eq!(request.page_count(), 5);
        assert!(!request.is_empty());
        assert_eq!(request.op, BalloonOp::Inflate);
    }

    #[test]
    fn test_pressure_calculation() {
        let driver = BalloonDriver::default();

        // 50% used
        let pressure = driver.calculate_pressure(500, 1000);
        assert_eq!(pressure, 50);

        // 90% used
        let pressure = driver.calculate_pressure(100, 1000);
        assert_eq!(pressure, 90);
    }

    #[test]
    fn test_pressure_event() {
        let driver = BalloonDriver::default();
        driver.enable();

        // High pressure event should trigger inflate
        let event = driver.create_pressure_event(100, 1000); // 90% used
        assert_eq!(event.action, PressureAction::Inflate);

        // Low pressure event might trigger deflate
        let event = driver.create_pressure_event(800, 1000); // 20% used
        assert_eq!(event.action, PressureAction::Deflate);
    }

    #[test]
    fn test_at_target() {
        let driver = BalloonDriver::default();
        driver.set_target_size(1000).unwrap();

        assert!(driver.at_target());

        driver.inflate(100).unwrap();
        assert!(!driver.at_target());
    }

    #[test]
    fn test_target_percentage() {
        let driver = BalloonDriver::default();
        driver.set_target_size(1000).unwrap();
        driver.enable();
        driver.inflate(500).unwrap();

        assert_eq!(driver.target_percentage(), 50.0);
    }

    #[test]
    fn test_clear() {
        let driver = BalloonDriver::default();
        driver.enable();
        driver.inflate(100).unwrap();
        driver.clear().unwrap();

        assert_eq!(driver.current_size(), 0);
    }

    #[test]
    fn test_reset_stats() {
        let driver = BalloonDriver::default();
        driver.enable();
        driver.inflate(100).unwrap();
        driver.deflate(50).unwrap();

        driver.reset_stats();

        let stats = driver.stats();
        assert_eq!(stats.total_inflated, 0);
        assert_eq!(stats.total_deflated, 0);
        assert_eq!(stats.inflate_count, 0);
        assert_eq!(stats.deflate_count, 0);
    }
}
