//! VSync (Vertical Synchronization) support
//!
//! Implements VSync timing for smooth frame presentation and power efficiency.

extern crate alloc;

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use crate::reliability::errno::{EINVAL, EIO};

/// VSync manager - handles vertical synchronization
pub struct VsyncManager {
    /// VSync enabled flag
    enabled: AtomicBool,
    /// Last VSync time (nanoseconds)
    last_vsync_time: AtomicU64,
    /// VSync interval (nanoseconds) - typically 16.67ms for 60Hz
    vsync_interval_ns: u64,
    /// Frame counter
    frame_counter: AtomicU64,
}

impl VsyncManager {
    /// Create a new VSync manager
    pub fn new() -> Self {
        // Default to 60Hz (16.67ms per frame)
        let vsync_interval_ns = 16_666_667; // ~16.67ms
        
        Self {
            enabled: AtomicBool::new(false),
            last_vsync_time: AtomicU64::new(0),
            vsync_interval_ns,
            frame_counter: AtomicU64::new(0),
        }
    }
    
    /// Enable VSync
    pub fn enable(&self) -> Result<(), i32> {
        self.enabled.store(true, Ordering::Release);
        self.last_vsync_time.store(crate::time::hrtime_nanos(), Ordering::Release);
        crate::println!("[vsync] VSync enabled ({} Hz)", 1_000_000_000 / self.vsync_interval_ns);
        Ok(())
    }
    
    /// Disable VSync
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Release);
        crate::println!("[vsync] VSync disabled");
    }
    
    /// Wait for next VSync
    /// Returns immediately if VSync is disabled
    pub fn wait_for_vsync(&self) -> Result<(), i32> {
        if !self.enabled.load(Ordering::Acquire) {
            return Ok(()); // VSync disabled - don't wait
        }
        
        let current_time = crate::time::hrtime_nanos();
        let last_vsync = self.last_vsync_time.load(Ordering::Acquire);
        let elapsed = current_time.saturating_sub(last_vsync);
        
        if elapsed < self.vsync_interval_ns {
            // Wait until next VSync
            let wait_time = self.vsync_interval_ns - elapsed;
            crate::time::sleep_ms(wait_time / 1_000_000);
        }
        
        // Update VSync time
        self.last_vsync_time.store(crate::time::hrtime_nanos(), Ordering::Release);
        self.frame_counter.fetch_add(1, Ordering::Relaxed);
        
        Ok(())
    }
    
    /// Check if VSync is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }
    
    /// Get VSync interval in nanoseconds
    pub fn get_interval_ns(&self) -> u64 {
        self.vsync_interval_ns
    }
    
    /// Set VSync interval (for different refresh rates)
    pub fn set_interval_ns(&mut self, interval_ns: u64) -> Result<(), i32> {
        if interval_ns == 0 || interval_ns > 1_000_000_000 {
            return Err(EINVAL);
        }
        
        self.vsync_interval_ns = interval_ns;
        crate::println!("[vsync] VSync interval set to {} ns ({} Hz)", interval_ns, 1_000_000_000 / interval_ns);
        Ok(())
    }
    
    /// Get frame counter
    pub fn get_frame_count(&self) -> u64 {
        self.frame_counter.load(Ordering::Relaxed)
    }
    
    /// VSync interrupt handler (called by display driver)
    pub fn vsync_interrupt(&self) {
        if self.enabled.load(Ordering::Acquire) {
            self.last_vsync_time.store(crate::time::hrtime_nanos(), Ordering::Release);
            self.frame_counter.fetch_add(1, Ordering::Relaxed);
            
            // Wake up compositor if it's waiting
            // In real implementation, this would use a wait queue
        }
    }
}

impl Default for VsyncManager {
    fn default() -> Self {
        Self::new()
    }
}

