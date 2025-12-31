//! Clock source management
//!
//! This module provides support for various hardware clock sources including:
//! - TSC (Time Stamp Counter) - CPU cycle counter
//! - HPET (High Precision Event Timer) - High precision timer
//! - ACPI PMT (Power Management Timer) - Legacy timer
//! - RTC (Real-Time Clock) - Battery-backed CMOS clock
//!
//! The clock subsystem implements:
//! - Automatic clock source selection
//! - Quality assessment and ranking
//! - Fallback mechanisms
//! - Cross-clock validation
//!
//! # Example
//!
//! ```no_run
//! use kernel::time::clock::{ClockSource, ClockManager};
//!
//! let manager = ClockManager::new();
//! let tsc = TSCClock::new();
//! manager.register_clock(Box::new(tsc));
//! manager.select_best_clock();
//! ```

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

/// Nanoseconds per second
const NSEC_PER_SEC: u64 = 1_000_000_000;

/// Clock source quality score
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClockQuality {
    /// Unusable clock
    Unusable = 0,
    /// Low quality (±1000 ppm)
    Low = 1,
    /// Medium quality (±100 ppm)
    Medium = 2,
    /// High quality (±10 ppm)
    High = 3,
    /// Excellent quality (±1 ppm)
    Excellent = 4,
}

/// Clock source type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClockType {
    /// Time Stamp Counter (CPU cycles)
    TSC,
    /// High Precision Event Timer
    HPET,
    /// ACPI Power Management Timer
    ACPI_PM,
    /// Real-Time Clock (CMOS)
    RTC,
}

/// Clock source characteristics
#[derive(Debug, Clone)]
pub struct ClockCharacteristics {
    /// Clock type
    pub clock_type: ClockType,
    /// Quality assessment
    pub quality: ClockQuality,
    /// Resolution in nanoseconds
    pub resolution: u64,
    /// Maximum offset in nanoseconds
    pub max_offset: u64,
    /// Clock frequency in Hz
    pub frequency: u64,
    /// Whether clock is monotonic
    pub is_monotonic: bool,
    /// Whether clock stops in deep sleep
    pub stops_in_sleep: bool,
}

/// Clock reading with metadata
#[derive(Debug, Clone, Copy)]
pub struct ClockReading {
    /// Timestamp in nanoseconds
    pub ns: u64,
    /// Clock source ID
    pub source_id: u32,
    /// Reading sequence number
    pub seq: u64,
}

/// Clock source trait
pub trait ClockSource: Send + Sync {
    /// Get clock characteristics
    fn characteristics(&self) -> &ClockCharacteristics;

    /// Read current timestamp
    fn read(&self) -> ClockReading;

    /// Get clock source ID
    fn id(&self) -> u32;

    /// Check if clock is available
    fn is_available(&self) -> bool;

    /// Calibrate against reference clock
    fn calibrate(&mut self, reference: &ClockReading) -> Result<(), ClockError>;

    /// Get estimated error in nanoseconds
    fn estimated_error(&self) -> u64;
}

/// Clock operation errors
#[derive(Debug)]
pub enum ClockError {
    /// Clock not available
    NotAvailable,
    /// Calibration failed
    CalibrationFailed,
    /// Invalid reading
    InvalidReading,
    /// Clock skew too large
    ClockSkew,
    /// Unsupported operation
    Unsupported,
}

/// TSC (Time Stamp Counter) clock source
pub struct TSCClock {
    id: u32,
    characteristics: ClockCharacteristics,
    frequency: AtomicU64,
    base_tsc: AtomicU64,
    base_ns: AtomicU64,
    seq: AtomicU64,
}

impl TSCClock {
    /// Create new TSC clock
    pub fn new() -> Self {
        // Calibrate TSC frequency (simplified - actual implementation would measure)
        let frequency = Self::calibrate_frequency();

        Self {
            id: Self::generate_id(),
            characteristics: ClockCharacteristics {
                clock_type: ClockType::TSC,
                quality: ClockQuality::Excellent,
                resolution: 1, // 1 ns at 1GHz
                max_offset: 100,
                frequency,
                is_monotonic: true,
                stops_in_sleep: false,
            },
            frequency: AtomicU64::new(frequency),
            base_tsc: AtomicU64::new(0),
            base_ns: AtomicU64::new(0),
            seq: AtomicU64::new(0),
        }
    }

    /// Read TSC register
    #[inline]
    fn read_tsc() -> u64 {
        unsafe {
            let mut high: u32;
            let mut low: u32;
            core::arch::asm!(
                "rdtsc",
                out("edx") high,
                out("eax") low,
                options(nomem, nostack)
            );
            ((high as u64) << 32) | (low as u64)
        }
    }

    /// Calibrate TSC frequency
    fn calibrate_frequency() -> u64 {
        // In a real implementation, this would:
        // 1. Use a known time source (HPET/PIT)
        // 2. Measure TSC over a known interval
        // 3. Calculate frequency
        // For now, assume 2.5 GHz (modern CPU)
        2_500_000_000
    }

    fn generate_id() -> u32 {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_ID.fetch_add(1, Ordering::SeqCst) as u32
    }
}

impl ClockSource for TSCClock {
    fn characteristics(&self) -> &ClockCharacteristics {
        &self.characteristics
    }

    fn read(&self) -> ClockReading {
        let tsc = Self::read_tsc();
        let freq = self.frequency.load(Ordering::Relaxed);
        let base_tsc = self.base_tsc.load(Ordering::Relaxed);
        let base_ns = self.base_ns.load(Ordering::Relaxed);
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);

        let ns = if tsc >= base_tsc {
            base_ns + ((tsc - base_tsc) * NSEC_PER_SEC / freq)
        } else {
            // Handle TSC wraparound (very rare)
            base_ns + ((u64::MAX - base_tsc + tsc + 1) * NSEC_PER_SEC / freq)
        };

        ClockReading {
            ns,
            source_id: self.id,
            seq,
        }
    }

    fn id(&self) -> u32 {
        self.id
    }

    fn is_available(&self) -> bool {
        // Check for invariant TSC support via CPUID
        // For now, assume available
        true
    }

    fn calibrate(&mut self, reference: &ClockReading) -> Result<(), ClockError> {
        let tsc = Self::read_tsc();
        self.base_tsc.store(tsc, Ordering::Relaxed);
        self.base_ns.store(reference.ns, Ordering::Relaxed);
        Ok(())
    }

    fn estimated_error(&self) -> u64 {
        // TSC error depends on CPU
        // Invariant TSC: ±10 ppm
        // Non-invariant: ±100 ppm
        self.characteristics.max_offset
    }
}

/// HPET (High Precision Event Timer) clock source
pub struct HPETClock {
    id: u32,
    characteristics: ClockCharacteristics,
    base_addr: usize,
    seq: AtomicU64,
    enabled: Mutex<bool>,
}

impl HPETClock {
    /// Create new HPET clock
    pub fn new(base_addr: usize) -> Self {
        Self {
            id: Self::generate_id(),
            characteristics: ClockCharacteristics {
                clock_type: ClockType::HPET,
                quality: ClockQuality::Excellent,
                resolution: 100, // 100 ns typical
                max_offset: 500,
                frequency: 10_000_000, // 10 MHz typical
                is_monotonic: true,
                stops_in_sleep: false,
            },
            base_addr,
            seq: AtomicU64::new(0),
            enabled: Mutex::new(false),
        }
    }

    /// Read HPET main counter
    #[inline]
    fn read_counter(&self) -> u64 {
        unsafe {
            let ptr = (self.base_addr + 0xF0) as *const u32;
            let low = ptr.read_volatile();
            let high = ptr.add(1).read_volatile();
            ((high as u64) << 32) | (low as u64)
        }
    }

    /// Check if HPET is available
    fn check_available(&self) -> bool {
        // In real implementation, would check:
        // - ACPI HPET table exists
        // - MMIO region is accessible
        // - Counter is incrementing
        self.base_addr != 0
    }

    fn generate_id() -> u32 {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_ID.fetch_add(1, Ordering::SeqCst) as u32
    }
}

impl ClockSource for HPETClock {
    fn characteristics(&self) -> &ClockCharacteristics {
        &self.characteristics
    }

    fn read(&self) -> ClockReading {
        let counter = self.read_counter();
        let freq = self.characteristics.frequency;
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);

        let ns = counter * NSEC_PER_SEC / freq;

        ClockReading {
            ns,
            source_id: self.id,
            seq,
        }
    }

    fn id(&self) -> u32 {
        self.id
    }

    fn is_available(&self) -> bool {
        self.check_available()
    }

    fn calibrate(&mut self, _reference: &ClockReading) -> Result<(), ClockError> {
        // HPET is self-calibrating
        Ok(())
    }

    fn estimated_error(&self) -> u64 {
        self.characteristics.max_offset
    }
}

/// ACPI PM (Power Management Timer) clock source
pub struct ACPMPMClock {
    id: u32,
    characteristics: ClockCharacteristics,
    seq: AtomicU64,
    last_value: Mutex<u16>,
}

impl ACPMPMClock {
    /// Create new ACPI PM clock
    pub fn new() -> Self {
        Self {
            id: Self::generate_id(),
            characteristics: ClockCharacteristics {
                clock_type: ClockType::ACPI_PM,
                quality: ClockQuality::Medium,
                resolution: 279, // ~279 ns (3.579545 MHz)
                max_offset: 1000,
                frequency: 3_579_545,
                is_monotonic: true,
                stops_in_sleep: true,
            },
            seq: AtomicU64::new(0),
            last_value: Mutex::new(0),
        }
    }

    /// Read PM timer port (0x808 or 0x804 depending on TMR_VAL_EXT)
    #[inline]
    fn read_port() -> u32 {
        unsafe {
            let mut value: u32;
            core::arch::asm!(
                "in al, dx",
                in("dx") 0x808u16,
                out("al") value,
                options(nomem, nostack)
            );
            // PM timer is 24-bit or 32-bit
            value
        }
    }

    fn generate_id() -> u32 {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_ID.fetch_add(1, Ordering::SeqCst) as u32
    }
}

impl ClockSource for ACPMPMClock {
    fn characteristics(&self) -> &ClockCharacteristics {
        &self.characteristics
    }

    fn read(&self) -> ClockReading {
        let current = Self::read_port();
        let mut last = self.last_value.lock();

        // Handle 24-bit wraparound (common for PM timers)
        let ticks = if current < *last {
            // Wrapped around
            (u32::MAX - *last as u32 + current as u32)
        } else {
            current as u32 - *last as u32
        };

        *last = current as u16;
        drop(last);

        let freq = self.characteristics.frequency;
        let ns = ticks * NSEC_PER_SEC / freq;
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);

        ClockReading {
            ns,
            source_id: self.id,
            seq,
        }
    }

    fn id(&self) -> u32 {
        self.id
    }

    fn is_available(&self) -> bool {
        // Check FADT for PM timer availability
        true
    }

    fn calibrate(&mut self, _reference: &ClockReading) -> Result<(), ClockError> {
        // PM timer frequency is fixed
        Ok(())
    }

    fn estimated_error(&self) -> u64 {
        self.characteristics.max_offset
    }
}

/// RTC (Real-Time Clock) clock source
pub struct RTCClock {
    id: u32,
    characteristics: ClockCharacteristics,
    base_ns: AtomicU64,
    base_rtc: AtomicU64,
    seq: AtomicU64,
}

impl RTCClock {
    /// Create new RTC clock
    pub fn new() -> Self {
        Self {
            id: Self::generate_id(),
            characteristics: ClockCharacteristics {
                clock_type: ClockType::RTC,
                quality: ClockQuality::Low,
                resolution: 1_000_000_000, // 1 second
                max_offset: 2_000_000_000, // ±2 seconds
                frequency: 1,
                is_monotonic: false, // Can be set backwards
                stops_in_sleep: false, // Battery-backed
            },
            base_ns: AtomicU64::new(0),
            base_rtc: AtomicU64::new(0),
            seq: AtomicU64::new(0),
        }
    }

    /// Read RTC time (simplified - would read from CMOS ports 0x70/0x71)
    fn read_rtc() -> u64 {
        unsafe {
            // Read seconds (port 0x00)
            core::arch::asm!(
                "out al, dx",
                in("al") 0u8,
                in("dx") 0x70u16,
                options(nostack)
            );
            let mut seconds: u8;
            core::arch::asm!(
                "in al, dx",
                in("dx") 0x71u16,
                out("al") seconds,
                options(nomem, nostack)
            );

            seconds as u64
        }
    }

    /// Read full RTC time as seconds since epoch
    fn read_time() -> u64 {
        // Simplified - would convert RTC BCD to Unix time
        Self::read_rtc()
    }

    fn generate_id() -> u32 {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_ID.fetch_add(1, Ordering::SeqCst) as u32
    }
}

impl ClockSource for RTCClock {
    fn characteristics(&self) -> &ClockCharacteristics {
        &self.characteristics
    }

    fn read(&self) -> ClockReading {
        let rtc_ns = Self::read_time() * NSEC_PER_SEC;
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);

        ClockReading {
            ns: rtc_ns,
            source_id: self.id,
            seq,
        }
    }

    fn id(&self) -> u32 {
        self.id
    }

    fn is_available(&self) -> bool {
        // RTC is always available on PC platforms
        true
    }

    fn calibrate(&mut self, reference: &ClockReading) -> Result<(), ClockError> {
        let rtc_ns = Self::read_time() * NSEC_PER_SEC;
        self.base_ns.store(reference.ns, Ordering::Relaxed);
        self.base_rtc.store(rtc_ns, Ordering::Relaxed);
        Ok(())
    }

    fn estimated_error(&self) -> u64 {
        self.characteristics.max_offset
    }
}

/// Clock manager for clock source selection
pub struct ClockManager {
    clocks: Mutex<Vec<Box<dyn ClockSource>>>,
    active_clock: Mutex<Option<Arc<dyn ClockSource>>>,
    fallback_clocks: Mutex<Vec<Arc<dyn ClockSource>>>,
}

impl ClockManager {
    /// Create new clock manager
    pub fn new() -> Self {
        Self {
            clocks: Mutex::new(Vec::new()),
            active_clock: Mutex::new(None),
            fallback_clocks: Mutex::new(Vec::new()),
        }
    }

    /// Register a clock source
    pub fn register_clock(&self, clock: Box<dyn ClockSource>) {
        self.clocks.lock().push(clock);
    }

    /// Select best clock based on quality
    pub fn select_best_clock(&self) -> Result<(), ClockError> {
        let clocks = self.clocks.lock();

        // Filter available clocks
        let mut available: Vec<_> = clocks
            .iter()
            .filter(|c| c.is_available())
            .collect();

        if available.is_empty() {
            return Err(ClockError::NotAvailable);
        }

        // Sort by quality (descending)
        available.sort_by(|a, b| {
            b.characteristics().quality
                .cmp(&a.characteristics().quality)
        });

        // Select best clock
        let best = available.first().ok_or(ClockError::NotAvailable)?;

        // Set active clock
        let best_arc = Arc::clone(&**best);
        *self.active_clock.lock() = Some(best_arc);

        // Set up fallback chain
        let mut fallback = self.fallback_clocks.lock();
        fallback.clear();
        for clock in available.iter().skip(1) {
            fallback.push(Arc::clone(&**clock));
        }

        Ok(())
    }

    /// Get current timestamp from active clock
    pub fn read(&self) -> Result<ClockReading, ClockError> {
        let active = self.active_clock.lock();
        let clock = active.as_ref().ok_or(ClockError::NotAvailable)?;
        Ok(clock.read())
    }

    /// Get active clock characteristics
    pub fn active_characteristics(&self) -> Result<ClockCharacteristics, ClockError> {
        let active = self.active_clock.lock();
        let clock = active.as_ref().ok_or(ClockError::NotAvailable)?;
        Ok(clock.characteristics().clone())
    }

    /// Get clock by ID
    pub fn get_clock(&self, id: u32) -> Option<Arc<dyn ClockSource>> {
        let clocks = self.clocks.lock();
        clocks
            .iter()
            .find(|c| c.id() == id)
            .map(|c| Arc::clone(&**c))
    }

    /// Calibrate all clocks against active clock
    pub fn calibrate_all(&self) -> Result<(), ClockError> {
        let active = self.active_clock.lock();
        let active_clock = active.as_ref().ok_or(ClockError::NotAvailable)?;
        let reference = active_clock.read();
        drop(active);

        let clocks = self.clocks.lock();
        for mut clock in clocks.iter() {
            if clock.id() != active_clock.id() {
                let _ = clock.calibrate(&reference);
            }
        }

        Ok(())
    }

    /// Validate clocks against each other
    pub fn validate_clocks(&self) -> Result<(), ClockError> {
        let readings: Vec<_> = {
            let clocks = self.clocks.lock();
            clocks
                .iter()
                .filter(|c| c.is_available())
                .map(|c| c.read())
                .collect()
        };

        if readings.len() < 2 {
            return Ok(()); // Can't validate with single clock
        }

        // Check for outliers (2x median)
        let mut sorted_ns: Vec<_> = readings.iter().map(|r| r.ns).collect();
        sorted_ns.sort();
        let median = sorted_ns[sorted_ns.len() / 2];

        for reading in &readings {
            let diff = if reading.ns > median {
                reading.ns - median
            } else {
                median - reading.ns
            };

            if diff > median * 2 {
                return Err(ClockError::ClockSkew);
            }
        }

        Ok(())
    }

    /// Get all clock characteristics
    pub fn all_characteristics(&self) -> Vec<ClockCharacteristics> {
        let clocks = self.clocks.lock();
        clocks.iter().map(|c| c.characteristics().clone()).collect()
    }
}

impl Default for ClockManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global clock manager instance
static GLOBAL_CLOCK_MANAGER: ClockManager = ClockManager::new();

/// Initialize global clock subsystem
pub fn init() {
    let manager = &GLOBAL_CLOCK_MANAGER;

    // Register TSC clock
    let tsc = TSCClock::new();
    manager.register_clock(Box::new(tsc));

    // Register HPET (if available)
    // let hpet = HPETClock::new(0xFED00000);
    // if hpet.is_available() {
    //     manager.register_clock(Box::new(hpet));
    // }

    // Register ACPI PM timer
    let pm = ACPMPMClock::new();
    manager.register_clock(Box::new(pm));

    // Register RTC
    let rtc = RTCClock::new();
    manager.register_clock(Box::new(rtc));

    // Select best clock
    let _ = manager.select_best_clock();
}

/// Get global clock manager
pub fn global_manager() -> &'static ClockManager {
    &GLOBAL_CLOCK_MANAGER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tsc_creation() {
        let tsc = TSCClock::new();
        assert!(tsc.is_available());
        assert_eq!(tsc.characteristics().clock_type, ClockType::TSC);
    }

    #[test]
    fn test_rtc_creation() {
        let rtc = RTCClock::new();
        assert!(rtc.is_available());
        assert_eq!(rtc.characteristics().clock_type, ClockType::RTC);
    }

    #[test]
    fn test_clock_manager() {
        let manager = ClockManager::new();
        let tsc = TSCClock::new();
        manager.register_clock(Box::new(tsc));

        let rtc = RTCClock::new();
        manager.register_clock(Box::new(rtc));

        assert!(manager.select_best_clock().is_ok());
        assert!(manager.read().is_ok());
    }
}
