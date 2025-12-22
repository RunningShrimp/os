//! Memory Mirroring - Failover mechanism for DIMM redundancy
//!
//! Provides:
//! - Memory mirroring configuration
//! - Failover on DIMM failure
//! - Mirror status tracking
//! - Synchronization management

/// Mirror states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorState {
    /// Not mirrored
    NotMirrored,
    /// Mirroring in progress
    Mirroring,
    /// Synchronized
    Synchronized,
    /// Degraded (primary failed)
    Degraded,
    /// Failed (both failed)
    Failed,
}

/// Memory mirror pair
#[derive(Debug, Clone, Copy)]
pub struct MemoryMirrorPair {
    /// Primary DIMM slot ID
    pub primary_slot: u32,
    /// Secondary DIMM slot ID
    pub secondary_slot: u32,
    /// Primary address
    pub primary_address: u64,
    /// Secondary address
    pub secondary_address: u64,
    /// Size in MB
    pub size_mb: u32,
    /// Mirror state
    pub state: MirrorState,
    /// Synchronization percentage
    pub sync_percentage: u8,
    /// Primary healthy
    pub primary_healthy: bool,
    /// Secondary healthy
    pub secondary_healthy: bool,
}

impl MemoryMirrorPair {
    /// Create memory mirror pair
    pub fn new(primary_slot: u32, secondary_slot: u32, size_mb: u32) -> Self {
        MemoryMirrorPair {
            primary_slot,
            secondary_slot,
            primary_address: 0,
            secondary_address: 0,
            size_mb,
            state: MirrorState::NotMirrored,
            sync_percentage: 0,
            primary_healthy: true,
            secondary_healthy: true,
        }
    }

    /// Start mirroring
    pub fn start_mirroring(&mut self) -> bool {
        if self.state == MirrorState::NotMirrored {
            self.state = MirrorState::Mirroring;
            self.sync_percentage = 0;
            true
        } else {
            false
        }
    }

    /// Set synchronization progress
    pub fn set_sync_percentage(&mut self, percentage: u8) -> bool {
        if percentage <= 100 {
            self.sync_percentage = percentage;
            if percentage == 100 && self.state == MirrorState::Mirroring {
                self.state = MirrorState::Synchronized;
            }
            true
        } else {
            false
        }
    }

    /// Complete synchronization
    pub fn complete_sync(&mut self) {
        self.sync_percentage = 100;
        self.state = MirrorState::Synchronized;
    }

    /// Mark primary as unhealthy
    pub fn primary_failed(&mut self) {
        self.primary_healthy = false;
        if self.state == MirrorState::Synchronized {
            self.state = MirrorState::Degraded;
        }
    }

    /// Mark secondary as unhealthy
    pub fn secondary_failed(&mut self) {
        self.secondary_healthy = false;
        if self.state == MirrorState::Synchronized && !self.primary_healthy {
            self.state = MirrorState::Failed;
        }
    }

    /// Check if mirror is functional
    pub fn is_functional(&self) -> bool {
        self.state != MirrorState::Failed && (self.primary_healthy || self.secondary_healthy)
    }

    /// Get active address for reads
    pub fn get_read_address(&self) -> u64 {
        if self.primary_healthy {
            self.primary_address
        } else {
            self.secondary_address
        }
    }

    /// Get write addresses
    pub fn get_write_addresses(&self) -> (Option<u64>, Option<u64>) {
        let primary = if self.primary_healthy {
            Some(self.primary_address)
        } else {
            None
        };

        let secondary = if self.secondary_healthy {
            Some(self.secondary_address)
        } else {
            None
        };

        (primary, secondary)
    }
}

/// Memory mirroring manager
pub struct MemoryMirroringManager {
    /// Mirror pairs
    pairs: [Option<MemoryMirrorPair>; 16],
    /// Pair count
    pair_count: usize,
    /// Total mirrored memory in MB
    total_mirrored_mb: u32,
    /// Mirroring enabled
    enabled: bool,
    /// Failover count
    failover_count: u32,
}

impl MemoryMirroringManager {
    /// Create memory mirroring manager
    pub fn new() -> Self {
        MemoryMirroringManager {
            pairs: [None; 16],
            pair_count: 0,
            total_mirrored_mb: 0,
            enabled: false,
            failover_count: 0,
        }
    }

    /// Add mirror pair
    pub fn add_mirror_pair(&mut self, pair: MemoryMirrorPair) -> bool {
        if self.pair_count < 16 {
            self.pairs[self.pair_count] = Some(pair);
            self.pair_count += 1;
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Get mirror pair
    pub fn get_pair(&self, index: usize) -> Option<&MemoryMirrorPair> {
        if index < self.pair_count {
            self.pairs[index].as_ref()
        } else {
            None
        }
    }

    /// Get mutable mirror pair
    pub fn get_pair_mut(&mut self, index: usize) -> Option<&mut MemoryMirrorPair> {
        if index < self.pair_count {
            self.pairs[index].as_mut()
        } else {
            None
        }
    }

    /// Find pair by primary slot
    pub fn find_pair_by_primary(&self, slot_id: u32) -> Option<&MemoryMirrorPair> {
        for i in 0..self.pair_count {
            if let Some(p) = &self.pairs[i] {
                if p.primary_slot == slot_id {
                    return Some(p);
                }
            }
        }
        None
    }

    /// Start all mirrors
    pub fn start_all_mirrors(&mut self) -> u32 {
        let mut count = 0;
        for i in 0..self.pair_count {
            if let Some(p) = &mut self.pairs[i] {
                if p.start_mirroring() {
                    count += 1;
                }
            }
        }
        self.enabled = true;
        count
    }

    /// Handle DIMM failure
    pub fn handle_dimm_failure(&mut self, slot_id: u32) -> bool {
        for i in 0..self.pair_count {
            if let Some(p) = &mut self.pairs[i] {
                if p.primary_slot == slot_id {
                    p.primary_failed();
                    self.failover_count += 1;
                    return true;
                } else if p.secondary_slot == slot_id {
                    p.secondary_failed();
                    self.failover_count += 1;
                    return true;
                }
            }
        }
        false
    }

    /// Get pair count
    pub fn get_pair_count(&self) -> usize {
        self.pair_count
    }

    /// Get total mirrored memory
    pub fn get_total_mirrored_memory(&self) -> u32 {
        self.total_mirrored_mb
    }

    /// Check if mirroring enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get failover count
    pub fn get_failover_count(&self) -> u32 {
        self.failover_count
    }

    /// Get synchronized pair count
    pub fn get_synchronized_count(&self) -> u32 {
        let mut count = 0;
        for i in 0..self.pair_count {
            if let Some(p) = &self.pairs[i] {
                if p.state == MirrorState::Synchronized {
                    count += 1;
                }
            }
        }
        count
    }

    /// Get degraded pair count
    pub fn get_degraded_count(&self) -> u32 {
        let mut count = 0;
        for i in 0..self.pair_count {
            if let Some(p) = &self.pairs[i] {
                if p.state == MirrorState::Degraded {
                    count += 1;
                }
            }
        }
        count
    }

    /// Update statistics
    fn update_stats(&mut self) {
        let mut total = 0;
        for i in 0..self.pair_count {
            if let Some(p) = &self.pairs[i] {
                total += p.size_mb;
            }
        }
        self.total_mirrored_mb = total;
    }

    /// Synchronize all pairs
    pub fn sync_all_pairs(&mut self) -> u32 {
        let mut count = 0;
        for i in 0..self.pair_count {
            if let Some(p) = &mut self.pairs[i] {
                if p.state == MirrorState::Mirroring {
                    p.complete_sync();
                    count += 1;
                }
            }
        }
        count
    }

    /// Get all functional pairs
    pub fn get_functional_count(&self) -> u32 {
        let mut count = 0;
        for i in 0..self.pair_count {
            if let Some(p) = &self.pairs[i] {
                if p.is_functional() {
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
    fn test_mirror_states() {
        assert_ne!(MirrorState::Synchronized, MirrorState::Degraded);
    }

    #[test]
    fn test_mirror_pair_creation() {
        let pair = MemoryMirrorPair::new(0, 1, 8192);
        assert_eq!(pair.primary_slot, 0);
        assert_eq!(pair.secondary_slot, 1);
    }

    #[test]
    fn test_start_mirroring() {
        let mut pair = MemoryMirrorPair::new(0, 1, 8192);
        assert!(pair.start_mirroring());
        assert_eq!(pair.state, MirrorState::Mirroring);
    }

    #[test]
    fn test_set_sync_percentage() {
        let mut pair = MemoryMirrorPair::new(0, 1, 8192);
        pair.start_mirroring();
        assert!(pair.set_sync_percentage(50));
        assert_eq!(pair.sync_percentage, 50);
    }

    #[test]
    fn test_complete_sync() {
        let mut pair = MemoryMirrorPair::new(0, 1, 8192);
        pair.start_mirroring();
        pair.complete_sync();
        assert_eq!(pair.state, MirrorState::Synchronized);
    }

    #[test]
    fn test_primary_failure() {
        let mut pair = MemoryMirrorPair::new(0, 1, 8192);
        pair.complete_sync();
        pair.primary_failed();
        assert_eq!(pair.state, MirrorState::Degraded);
    }

    #[test]
    fn test_secondary_failure_while_degraded() {
        let mut pair = MemoryMirrorPair::new(0, 1, 8192);
        pair.complete_sync();
        pair.primary_failed();
        pair.secondary_failed();
        assert_eq!(pair.state, MirrorState::Failed);
    }

    #[test]
    fn test_is_functional() {
        let mut pair = MemoryMirrorPair::new(0, 1, 8192);
        pair.complete_sync();
        assert!(pair.is_functional());
        pair.primary_failed();
        assert!(pair.is_functional());
    }

    #[test]
    fn test_get_read_address_primary() {
        let mut pair = MemoryMirrorPair::new(0, 1, 8192);
        pair.primary_address = 0x1000;
        pair.secondary_address = 0x2000;
        assert_eq!(pair.get_read_address(), 0x1000);
    }

    #[test]
    fn test_get_read_address_secondary() {
        let mut pair = MemoryMirrorPair::new(0, 1, 8192);
        pair.primary_address = 0x1000;
        pair.secondary_address = 0x2000;
        pair.primary_failed();
        assert_eq!(pair.get_read_address(), 0x2000);
    }

    #[test]
    fn test_manager_creation() {
        let mgr = MemoryMirroringManager::new();
        assert_eq!(mgr.get_pair_count(), 0);
        assert!(!mgr.is_enabled());
    }

    #[test]
    fn test_add_mirror_pair() {
        let mut mgr = MemoryMirroringManager::new();
        let pair = MemoryMirrorPair::new(0, 1, 8192);
        assert!(mgr.add_mirror_pair(pair));
    }

    #[test]
    fn test_get_pair() {
        let mut mgr = MemoryMirroringManager::new();
        let pair = MemoryMirrorPair::new(0, 1, 8192);
        mgr.add_mirror_pair(pair);
        assert!(mgr.get_pair(0).is_some());
    }

    #[test]
    fn test_find_pair_by_primary() {
        let mut mgr = MemoryMirroringManager::new();
        let pair = MemoryMirrorPair::new(0, 1, 8192);
        mgr.add_mirror_pair(pair);
        assert!(mgr.find_pair_by_primary(0).is_some());
    }

    #[test]
    fn test_start_all_mirrors() {
        let mut mgr = MemoryMirroringManager::new();
        mgr.add_mirror_pair(MemoryMirrorPair::new(0, 1, 8192));
        let count = mgr.start_all_mirrors();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_handle_dimm_failure() {
        let mut mgr = MemoryMirroringManager::new();
        mgr.add_mirror_pair(MemoryMirrorPair::new(0, 1, 8192));
        assert!(mgr.handle_dimm_failure(0));
    }

    #[test]
    fn test_total_mirrored_memory() {
        let mut mgr = MemoryMirroringManager::new();
        mgr.add_mirror_pair(MemoryMirrorPair::new(0, 1, 4096));
        mgr.add_mirror_pair(MemoryMirrorPair::new(2, 3, 4096));
        assert_eq!(mgr.get_total_mirrored_memory(), 8192);
    }

    #[test]
    fn test_synchronized_count() {
        let mut mgr = MemoryMirroringManager::new();
        let mut pair1 = MemoryMirrorPair::new(0, 1, 4096);
        let pair2 = MemoryMirrorPair::new(2, 3, 4096);
        pair1.complete_sync();
        mgr.add_mirror_pair(pair1);
        mgr.add_mirror_pair(pair2);
        assert_eq!(mgr.get_synchronized_count(), 1);
    }

    #[test]
    fn test_degraded_count() {
        let mut mgr = MemoryMirroringManager::new();
        let mut pair = MemoryMirrorPair::new(0, 1, 8192);
        pair.complete_sync();
        pair.primary_failed();
        mgr.add_mirror_pair(pair);
        assert_eq!(mgr.get_degraded_count(), 1);
    }

    #[test]
    fn test_sync_all_pairs() {
        let mut mgr = MemoryMirroringManager::new();
        mgr.add_mirror_pair(MemoryMirrorPair::new(0, 1, 8192));
        mgr.start_all_mirrors();
        let count = mgr.sync_all_pairs();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_failover_count() {
        let mut mgr = MemoryMirroringManager::new();
        mgr.add_mirror_pair(MemoryMirrorPair::new(0, 1, 8192));
        mgr.handle_dimm_failure(0);
        assert_eq!(mgr.get_failover_count(), 1);
    }

    #[test]
    fn test_functional_count() {
        let mut mgr = MemoryMirroringManager::new();
        let mut pair = MemoryMirrorPair::new(0, 1, 8192);
        pair.complete_sync();
        mgr.add_mirror_pair(pair);
        assert_eq!(mgr.get_functional_count(), 1);
    }

    #[test]
    fn test_multiple_mirror_pairs() {
        let mut mgr = MemoryMirroringManager::new();
        for i in 0..8 {
            mgr.add_mirror_pair(MemoryMirrorPair::new(i * 2, i * 2 + 1, 4096));
        }
        assert_eq!(mgr.get_pair_count(), 8);
    }

    #[test]
    fn test_get_write_addresses_both_healthy() {
        let mut pair = MemoryMirrorPair::new(0, 1, 8192);
        pair.primary_address = 0x1000;
        pair.secondary_address = 0x2000;
        let (primary, secondary) = pair.get_write_addresses();
        assert!(primary.is_some());
        assert!(secondary.is_some());
    }

    #[test]
    fn test_get_write_addresses_degraded() {
        let mut pair = MemoryMirrorPair::new(0, 1, 8192);
        pair.primary_address = 0x1000;
        pair.secondary_address = 0x2000;
        pair.primary_failed();
        let (primary, secondary) = pair.get_write_addresses();
        assert!(primary.is_none());
        assert!(secondary.is_some());
    }

    #[test]
    fn test_get_pair_mut() {
        let mut mgr = MemoryMirroringManager::new();
        mgr.add_mirror_pair(MemoryMirrorPair::new(0, 1, 8192));
        if let Some(p) = mgr.get_pair_mut(0) {
            p.primary_address = 0x1000;
        }
        assert_eq!(mgr.get_pair(0).unwrap().primary_address, 0x1000);
    }

    #[test]
    fn test_mirroring_not_functional_if_both_failed() {
        let mut pair = MemoryMirrorPair::new(0, 1, 8192);
        pair.primary_failed();
        pair.secondary_failed();
        assert!(!pair.is_functional());
    }

    #[test]
    fn test_sync_percentage_100_triggers_synchronized() {
        let mut pair = MemoryMirrorPair::new(0, 1, 8192);
        pair.start_mirroring();
        pair.set_sync_percentage(100);
        assert_eq!(pair.state, MirrorState::Synchronized);
    }

    #[test]
    fn test_invalid_sync_percentage() {
        let mut pair = MemoryMirrorPair::new(0, 1, 8192);
        pair.start_mirroring();
        assert!(!pair.set_sync_percentage(150)); // > 100
    }

    #[test]
    fn test_find_pair_by_secondary() {
        let mut mgr = MemoryMirroringManager::new();
        let pair = MemoryMirrorPair::new(0, 1, 8192);
        mgr.add_mirror_pair(pair);
        // find_pair_by_primary doesn't search secondary, check None
        assert!(mgr.find_pair_by_primary(1).is_none());
    }
}
