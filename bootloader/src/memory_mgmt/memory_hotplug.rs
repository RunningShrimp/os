//! Memory Hotplug - DIMM hot-add support
//!
//! Provides:
//! - Memory hot-add detection
//! - DIMM management
//! - Memory space probing
//! - Online/offline control

/// DIMM states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimmState {
    /// Offline
    Offline,
    /// Online
    Online,
    /// Suspect (potential failure)
    Suspect,
    /// Failed
    Failed,
}

/// DIMM information
#[derive(Debug, Clone, Copy)]
pub struct DimmInfo {
    /// DIMM slot ID
    pub slot_id: u32,
    /// Base address
    pub base_address: u64,
    /// Size in MB
    pub size_mb: u32,
    /// Current state
    pub state: DimmState,
    /// Speed in MHz
    pub speed_mhz: u16,
    /// Detected flag
    pub detected: bool,
    /// Active flag
    pub active: bool,
}

impl DimmInfo {
    /// Create DIMM info
    pub fn new(slot_id: u32, size_mb: u32) -> Self {
        DimmInfo {
            slot_id,
            base_address: 0,
            size_mb,
            state: DimmState::Offline,
            speed_mhz: 2400,
            detected: false,
            active: false,
        }
    }

    /// Set base address
    pub fn set_base_address(&mut self, addr: u64) {
        self.base_address = addr;
    }

    /// Get end address
    pub fn get_end_address(&self) -> u64 {
        self.base_address + (self.size_mb as u64 * 1024 * 1024)
    }

    /// Online DIMM
    pub fn online(&mut self) -> bool {
        if self.detected && !self.active {
            self.state = DimmState::Online;
            self.active = true;
            true
        } else {
            false
        }
    }

    /// Offline DIMM
    pub fn offline(&mut self) -> bool {
        if self.active {
            self.state = DimmState::Offline;
            self.active = false;
            true
        } else {
            false
        }
    }

    /// Mark as suspect
    pub fn mark_suspect(&mut self) {
        self.state = DimmState::Suspect;
    }

    /// Mark as failed
    pub fn mark_failed(&mut self) {
        self.state = DimmState::Failed;
        self.active = false;
    }

    /// Check if healthy
    pub fn is_healthy(&self) -> bool {
        self.state != DimmState::Failed && self.state != DimmState::Suspect
    }
}

/// Memory hotplug manager
pub struct MemoryHotplugManager {
    /// DIMMs
    dimms: [Option<DimmInfo>; 32],
    /// DIMM count
    dimm_count: usize,
    /// Total memory in MB
    total_memory_mb: u32,
    /// Available memory in MB
    available_memory_mb: u32,
    /// Hotplug supported
    hotplug_supported: bool,
}

impl MemoryHotplugManager {
    /// Create memory hotplug manager
    pub fn new() -> Self {
        MemoryHotplugManager {
            dimms: [None; 32],
            dimm_count: 0,
            total_memory_mb: 0,
            available_memory_mb: 0,
            hotplug_supported: true,
        }
    }

    /// Add DIMM
    pub fn add_dimm(&mut self, dimm: DimmInfo) -> bool {
        if self.dimm_count < 32 {
            self.dimms[self.dimm_count] = Some(dimm);
            self.dimm_count += 1;
            self.update_memory_stats();
            true
        } else {
            false
        }
    }

    /// Get DIMM by slot ID
    pub fn get_dimm(&self, slot_id: u32) -> Option<&DimmInfo> {
        for i in 0..self.dimm_count {
            if let Some(d) = &self.dimms[i] {
                if d.slot_id == slot_id {
                    return Some(d);
                }
            }
        }
        None
    }

    /// Get mutable DIMM by slot ID
    pub fn get_dimm_mut(&mut self, slot_id: u32) -> Option<&mut DimmInfo> {
        let dimm_count = self.dimm_count;
        let dimms_ptr = self.dimms.as_mut_ptr();
        
        for i in 0..dimm_count {
            unsafe {
                if let Some(d) = (*dimms_ptr.add(i)).as_mut() {
                    if d.slot_id == slot_id {
                        return Some(d);
                    }
                }
            }
        }
        None
    }

    /// Probe and detect DIMMs
    pub fn probe_dimms(&mut self, start_address: u64) -> u32 {
        let mut address = start_address;
        let mut detected = 0;

        for i in 0..self.dimm_count {
            if let Some(d) = &mut self.dimms[i] {
                if !d.detected {
                    d.base_address = address;
                    d.detected = true;
                    address += d.size_mb as u64 * 1024 * 1024;
                    detected += 1;
                }
            }
        }

        detected
    }

    /// Online DIMM
    pub fn online_dimm(&mut self, slot_id: u32) -> bool {
        if let Some(dimm) = self.get_dimm_mut(slot_id) {
            dimm.online()
        } else {
            false
        }
    }

    /// Offline DIMM
    pub fn offline_dimm(&mut self, slot_id: u32) -> bool {
        if let Some(dimm) = self.get_dimm_mut(slot_id) {
            dimm.offline()
        } else {
            false
        }
    }

    /// Online all DIMMs
    pub fn online_all(&mut self) -> u32 {
        let mut count = 0;
        for i in 0..self.dimm_count {
            if let Some(d) = &mut self.dimms[i] {
                if d.online() {
                    count += 1;
                }
            }
        }
        self.update_memory_stats();
        count
    }

    /// Get DIMM count
    pub fn get_dimm_count(&self) -> usize {
        self.dimm_count
    }

    /// Get total memory
    pub fn get_total_memory(&self) -> u32 {
        self.total_memory_mb
    }

    /// Get available memory
    pub fn get_available_memory(&self) -> u32 {
        self.available_memory_mb
    }

    /// Update memory statistics
    fn update_memory_stats(&mut self) {
        let mut total = 0;
        let mut available = 0;

        for i in 0..self.dimm_count {
            if let Some(d) = &self.dimms[i] {
                total += d.size_mb;
                if d.active && d.is_healthy() {
                    available += d.size_mb;
                }
            }
        }

        self.total_memory_mb = total;
        self.available_memory_mb = available;
    }

    /// Get healthy DIMM count
    pub fn get_healthy_dimm_count(&self) -> u32 {
        let mut count = 0;
        for i in 0..self.dimm_count {
            if let Some(d) = &self.dimms[i] {
                if d.is_healthy() {
                    count += 1;
                }
            }
        }
        count
    }

    /// Get active DIMM count
    pub fn get_active_dimm_count(&self) -> u32 {
        let mut count = 0;
        for i in 0..self.dimm_count {
            if let Some(d) = &self.dimms[i] {
                if d.active {
                    count += 1;
                }
            }
        }
        count
    }

    /// Check hotplug support
    pub fn is_hotplug_supported(&self) -> bool {
        self.hotplug_supported
    }

    /// Set hotplug support
    pub fn set_hotplug_supported(&mut self, supported: bool) {
        self.hotplug_supported = supported;
    }

    /// Get failed DIMM count
    pub fn get_failed_dimm_count(&self) -> u32 {
        let mut count = 0;
        for i in 0..self.dimm_count {
            if let Some(d) = &self.dimms[i] {
                if d.state == DimmState::Failed {
                    count += 1;
                }
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimm_states() {
        assert_ne!(DimmState::Online, DimmState::Offline);
    }

    #[test]
    fn test_dimm_creation() {
        let dimm = DimmInfo::new(0, 8192);
        assert_eq!(dimm.slot_id, 0);
        assert_eq!(dimm.size_mb, 8192);
    }

    #[test]
    fn test_dimm_online() {
        let dimm = DimmInfo::new(0, 8192);
        dimm.detected = true;
        assert!(dimm.online());
        assert_eq!(dimm.state, DimmState::Online);
    }

    #[test]
    fn test_dimm_offline() {
        let dimm = DimmInfo::new(0, 8192);
        dimm.detected = true;
        dimm.online();
        assert!(dimm.offline());
        assert_eq!(dimm.state, DimmState::Offline);
    }

    #[test]
    fn test_dimm_address_range() {
        let dimm = DimmInfo::new(0, 8192);
        dimm.set_base_address(0x80000000);
        let end = dimm.get_end_address();
        assert!(end > dimm.base_address);
    }

    #[test]
    fn test_dimm_suspect() {
        let dimm = DimmInfo::new(0, 8192);
        dimm.mark_suspect();
        assert_eq!(dimm.state, DimmState::Suspect);
        assert!(!dimm.is_healthy());
    }

    #[test]
    fn test_dimm_failed() {
        let dimm = DimmInfo::new(0, 8192);
        dimm.detected = true;
        dimm.online();
        dimm.mark_failed();
        assert_eq!(dimm.state, DimmState::Failed);
        assert!(!dimm.active);
    }

    #[test]
    fn test_manager_creation() {
        let mgr = MemoryHotplugManager::new();
        assert_eq!(mgr.get_dimm_count(), 0);
        assert!(mgr.is_hotplug_supported());
    }

    #[test]
    fn test_add_dimm() {
        let mut mgr = MemoryHotplugManager::new();
        let dimm = DimmInfo::new(0, 8192);
        assert!(mgr.add_dimm(dimm));
        assert_eq!(mgr.get_dimm_count(), 1);
    }

    #[test]
    fn test_get_dimm() {
        let mut mgr = MemoryHotplugManager::new();
        let dimm = DimmInfo::new(0, 8192);
        mgr.add_dimm(dimm);
        assert!(mgr.get_dimm(0).is_some());
    }

    #[test]
    fn test_probe_dimms() {
        let mut mgr = MemoryHotplugManager::new();
        let dimm = DimmInfo::new(0, 8192);
        mgr.add_dimm(dimm);
        let detected = mgr.probe_dimms(0x80000000);
        assert!(detected > 0);
    }

    #[test]
    fn test_online_dimm() {
        let mut mgr = MemoryHotplugManager::new();
        let dimm = DimmInfo::new(0, 8192);
        dimm.detected = true;
        mgr.add_dimm(dimm);
        assert!(mgr.online_dimm(0));
    }

    #[test]
    fn test_offline_dimm() {
        let mut mgr = MemoryHotplugManager::new();
        let dimm = DimmInfo::new(0, 8192);
        dimm.detected = true;
        dimm.online();
        mgr.add_dimm(dimm);
        mgr.get_dimm_mut(0).unwrap().online();
        assert!(mgr.offline_dimm(0));
    }

    #[test]
    fn test_online_all() {
        let mut mgr = MemoryHotplugManager::new();
        for i in 0..4 {
            let mut dimm = DimmInfo::new(i, 4096);
            dimm.detected = true;
            mgr.add_dimm(dimm);
        }
        let count = mgr.online_all();
        assert_eq!(count, 4);
    }

    #[test]
    fn test_total_memory() {
        let mut mgr = MemoryHotplugManager::new();
        mgr.add_dimm(DimmInfo::new(0, 4096));
        mgr.add_dimm(DimmInfo::new(1, 4096));
        assert_eq!(mgr.get_total_memory(), 8192);
    }

    #[test]
    fn test_healthy_dimm_count() {
        let mut mgr = MemoryHotplugManager::new();
        let dimm1 = DimmInfo::new(0, 4096);
        let mut dimm2 = DimmInfo::new(1, 4096);
        dimm2.mark_failed();
        mgr.add_dimm(dimm1);
        mgr.add_dimm(dimm2);
        assert_eq!(mgr.get_healthy_dimm_count(), 1);
    }

    #[test]
    fn test_active_dimm_count() {
        let mut mgr = MemoryHotplugManager::new();
        let mut dimm = DimmInfo::new(0, 4096);
        dimm.detected = true;
        dimm.online();
        mgr.add_dimm(dimm);
        assert_eq!(mgr.get_active_dimm_count(), 1);
    }

    #[test]
    fn test_get_dimm_mut() {
        let mut mgr = MemoryHotplugManager::new();
        let dimm = DimmInfo::new(0, 4096);
        mgr.add_dimm(dimm);
        if let Some(d) = mgr.get_dimm_mut(0) {
            d.set_base_address(0x80000000);
        }
        assert_eq!(mgr.get_dimm(0).unwrap().base_address, 0x80000000);
    }

    #[test]
    fn test_multiple_dimms() {
        let mut mgr = MemoryHotplugManager::new();
        for i in 0..8 {
            mgr.add_dimm(DimmInfo::new(i, 8192));
        }
        assert_eq!(mgr.get_dimm_count(), 8);
    }

    #[test]
    fn test_hotplug_support() {
        let mut mgr = MemoryHotplugManager::new();
        mgr.set_hotplug_supported(false);
        assert!(!mgr.is_hotplug_supported());
    }

    #[test]
    fn test_failed_dimm_count() {
        let mut mgr = MemoryHotplugManager::new();
        let dimm1 = DimmInfo::new(0, 4096);
        let mut dimm2 = DimmInfo::new(1, 4096);
        dimm1.mark_failed();
        dimm2.mark_failed();
        mgr.add_dimm(dimm1);
        mgr.add_dimm(dimm2);
        assert_eq!(mgr.get_failed_dimm_count(), 2);
    }

    #[test]
    fn test_dimm_speed() {
        let dimm = DimmInfo::new(0, 8192);
        dimm.speed_mhz = 3200;
        assert_eq!(dimm.speed_mhz, 3200);
    }

    #[test]
    fn test_available_memory() {
        let mut mgr = MemoryHotplugManager::new();
        let dimm = DimmInfo::new(0, 8192);
        dimm.detected = true;
        dimm.online();
        mgr.add_dimm(dimm);
        assert!(mgr.get_available_memory() > 0);
    }

    #[test]
    fn test_dimm_end_address() {
        let mut dimm = DimmInfo::new(0, 1024);
        dimm.set_base_address(0x100000000);
        let end = dimm.get_end_address();
        assert!(end > dimm.base_address);
    }

    #[test]
    fn test_memory_stats_update() {
        let mut mgr = MemoryHotplugManager::new();
        let mut dimm = DimmInfo::new(0, 4096);
        dimm.detected = true;
        mgr.add_dimm(dimm);
        assert!(mgr.get_total_memory() > 0);
    }

    #[test]
    fn test_dimm_healthy_check() {
        let dimm = DimmInfo::new(0, 8192);
        assert!(dimm.is_healthy());
    }

    #[test]
    fn test_online_undetected_fails() {
        let dimm = DimmInfo::new(0, 8192);
        assert!(!dimm.online()); // Not detected yet
    }

    #[test]
    fn test_offline_undetected_fails() {
        let dimm = DimmInfo::new(0, 8192);
        assert!(!dimm.offline()); // Not active
    }

    #[test]
    fn test_mixed_dimm_health() {
        let mut mgr = MemoryHotplugManager::new();
        let healthy = DimmInfo::new(0, 4096);
        let mut suspect = DimmInfo::new(1, 4096);
        suspect.mark_suspect();
        mgr.add_dimm(healthy);
        mgr.add_dimm(suspect);
        assert_eq!(mgr.get_healthy_dimm_count(), 1);
    }

    #[test]
    fn test_large_dimm() {
        let dimm = DimmInfo::new(0, 131072); // 128GB
        assert_eq!(dimm.size_mb, 131072);
    }

    #[test]
    fn test_dimm_address_calculation() {
        let mut dimm = DimmInfo::new(0, 2048);
        dimm.set_base_address(0x200000000);
        let end = dimm.get_end_address();
        assert_eq!(end, 0x200000000 + (2048 * 1024 * 1024) as u64);
    }
}
