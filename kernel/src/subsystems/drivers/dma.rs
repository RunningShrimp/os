//! DMA Engine Abstraction
//!
//! This module provides a comprehensive DMA (Direct Memory Access) engine abstraction
//! for device drivers, including:
//! - Scatter-gather DMA operations
//! - Coherent DMA mapping management
//! - Streaming DMA mapping
//! - DMA pool allocation
//! - DMA synchronization
//! - Error handling and recovery

use crate::prelude::*;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use core::sync::atomic {AtomicU32, AtomicU64,, Ordering};

// ============================================================================
// DMA Constants
// ============================================================================

/// Default DMA pool size (4MB)
pub const DEFAULT_DMA_POOL_SIZE: usize = 4 * 1024 * 1024;

/// Minimum DMA alignment (64 bytes)
pub const DMA_MIN_ALIGNMENT: usize = 64;

/// Page size for DMA operations
pub const DMA_PAGE_SIZE: usize = 4096;

/// Maximum scatter-gather entries
pub const MAX_SG_ENTRIES: usize = 256;

/// DMA mapping direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaDirection {
    /// Bidirectional DMA
    Bidirectional,
    /// Device to memory (read)
   ToDevice,
    /// Memory to device (write)
    ToMemory,
    /// No direction (for unmapping)
    None,
}

/// DMA mapping attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmaAttr {
    /// Aligned address
    pub aligned: bool,
    /// Coherent mapping
    pub coherent: bool,
    /// No scatter-gather
    pub no_sg: bool,
    /// No highmem (use low memory only)
    pub no_highmem: bool,
    /// Write combining
    pub write_combine: bool,
    /// Weak ordering
    pub weak_ordering: bool,
}

impl Default for DmaAttr {
    fn default() -> Self {
        Self {
            aligned: true,
            coherent: false,
            no_sg: false,
            no_highmem: false,
            write_combine: false,
            weak_ordering: false,
        }
    }
}

// ============================================================================
// DMA Scatter-Gather Structures
// ============================================================================

/// Scatter-gather list entry
#[derive(Debug, Clone, Copy)]
pub struct ScatterGatherEntry {
    /// Physical address
    pub address: u64,
    /// Length in bytes
    pub length: u32,
    /// Reserved for future use
    pub reserved: u32,
}

impl ScatterGatherEntry {
    /// Create new scatter-gather entry
    pub fn new(address: u64, length: u32) -> Self {
        Self {
            address,
            length,
            reserved: 0,
        }
    }

    /// Check if entry is valid
    pub fn is_valid(&self) -> bool {
        self.address != 0 && self.length != 0
    }
}

/// Scatter-gather list
#[derive(Debug, Clone)]
pub struct ScatterGatherList {
    /// List entries
    entries: Vec<ScatterGatherEntry>,
    /// Total length in bytes
    total_length: usize,
    /// Number of entries
    nents: usize,
}

impl ScatterGatherList {
    /// Create new scatter-gather list
    pub fn new(entries: Vec<ScatterGatherEntry>) -> Self {
        let total_length = entries.iter()
            .map(|e| e.length as usize)
            .sum();

        let nents = entries.len();

        Self {
            entries,
            total_length,
            nents,
        }
    }

    /// Create empty scatter-gather list
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
            total_length: 0,
            nents: 0,
        }
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.nents
    }

    /// Check if list is empty
    pub fn is_empty(&self) -> bool {
        self.nents == 0
    }

    /// Get total length
    pub fn total_length(&self) -> usize {
        self.total_length
    }

    /// Get entry at index
    pub fn get(&self, index: usize) -> Option<&ScatterGatherEntry> {
        self.entries.get(index)
    }

    /// Get mutable entry at index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut ScatterGatherEntry> {
        self.entries.get_mut(index)
    }

    /// Add entry to list
    pub fn add(&mut self, entry: ScatterGatherEntry) -> Result<()> {
        if self.nents >= MAX_SG_ENTRIES {
            return Err(KernelError::InvalidArgument);
        }

        self.total_length += entry.length as usize;
        self.nents += 1;
        self.entries.push(entry);

        Ok(())
    }

    /// Get entries slice
    pub fn entries(&self) -> &[ScatterGatherEntry] {
        &self.entries
    }

    /// Clear the list
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_length = 0;
        self.nents = 0;
    }
}

// ============================================================================
// DMA Mapping Structures
// ============================================================================

/// DMA mapping descriptor
#[derive(Debug, Clone)]
pub struct DmaMapping {
    /// Virtual address
    pub virt_addr: u64,
    /// Physical/DMA address
    pub dma_addr: u64,
    /// Size in bytes
    pub size: usize,
    /// Direction
    pub direction: DmaDirection,
    /// Is coherent
    pub coherent: bool,
    /// Mapping ID
    pub mapping_id: u32,
}

impl DmaMapping {
    /// Create new DMA mapping
    pub fn new(
        virt_addr: u64,
        dma_addr: u64,
        size: usize,
        direction: DmaDirection,
        coherent: bool,
        mapping_id: u32,
    ) -> Self {
        Self {
            virt_addr,
            dma_addr,
            size,
            direction,
            coherent,
            mapping_id,
        }
    }

    /// Check if mapping is valid
    pub fn is_valid(&self) -> bool {
        self.dma_addr != 0 && self.size != 0
    }

    /// Get physical address
    pub fn physical_address(&self) -> u64 {
        self.dma_addr
    }

    /// Get virtual address
    pub fn virtual_address(&self) -> u64 {
        self.virt_addr
    }
}

// ============================================================================
// DMA Channel
// ============================================================================

/// DMA channel status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaChannelStatus {
    /// Channel is idle
    Idle,
    /// Channel is busy
    Busy,
    /// Channel has error
    Error,
    /// Channel is paused
    Paused,
}

/// DMA transfer descriptor
#[derive(Debug)]
pub struct DmaTransfer {
    /// Transfer ID
    pub transfer_id: u32,
    /// Source physical address
    pub src_addr: u64,
    /// Destination physical address
    pub dst_addr: u64,
    /// Transfer size in bytes
    pub size: usize,
    /// Transfer direction
    pub direction: DmaDirection,
    /// Scatter-gather list (optional)
    pub sg_list: Option<ScatterGatherList>,
    /// Transfer completion callback
    pub callback: Option<u64>,
    /// Callback data
    pub callback_data: u64,
    /// Bytes transferred
    pub bytes_transferred: AtomicU64,
    /// Is completed
    pub completed: AtomicU32,
}

// Manual implementation of Clone for DmaTransfer
impl Clone for DmaTransfer {
    fn clone(&self) -> Self {
        Self {
            transfer_id: self.transfer_id,
            src_addr: self.src_addr,
            dst_addr: self.dst_addr,
            size: self.size,
            direction: self.direction,
            sg_list: self.sg_list.clone(),
            callback: self.callback,
            callback_data: self.callback_data,
            bytes_transferred: AtomicU64::new(self.bytes_transferred.load(Ordering::Relaxed)),
            completed: AtomicU32::new(self.completed.load(Ordering::Relaxed)),
        }
    }
}

impl DmaTransfer {
    /// Create new DMA transfer
    pub fn new(
        transfer_id: u32,
        src_addr: u64,
        dst_addr: u64,
        size: usize,
        direction: DmaDirection,
    ) -> Self {
        Self {
            transfer_id,
            src_addr,
            dst_addr,
            size,
            direction,
            sg_list: None,
            callback: None,
            callback_data: 0,
            bytes_transferred: AtomicU64::new(0),
            completed: AtomicU32::new(0),
        }
    }

    /// Create scatter-gather transfer
    pub fn with_sg_list(
        transfer_id: u32,
        src_addr: u64,
        dst_addr: u64,
        direction: DmaDirection,
        sg_list: ScatterGatherList,
    ) -> Self {
        let size = sg_list.total_length();

        Self {
            transfer_id,
            src_addr,
            dst_addr,
            size,
            direction,
            sg_list: Some(sg_list),
            callback: None,
            callback_data: 0,
            bytes_transferred: AtomicU64::new(0),
            completed: AtomicU32::new(0),
        }
    }

    /// Set completion callback
    pub fn with_callback(mut self, callback: u64, data: u64) -> Self {
        self.callback = Some(callback);
        self.callback_data = data;
        self
    }

    /// Check if transfer uses scatter-gather
    pub fn is_sg(&self) -> bool {
        self.sg_list.is_some()
    }

    /// Get scatter-gather list
    pub fn get_sg_list(&self) -> Option<&ScatterGatherList> {
        self.sg_list.as_ref()
    }

    /// Mark transfer as completed
    pub fn mark_completed(&self) {
        self.completed.store(1, Ordering::Release);
    }

    /// Check if transfer is completed
    pub fn is_completed(&self) -> bool {
        self.completed.load(Ordering::Acquire) != 0
    }

    /// Update bytes transferred
    pub fn update_transferred(&self, bytes: u64) {
        self.bytes_transferred.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Get bytes transferred
    pub fn get_bytes_transferred(&self) -> u64 {
        self.bytes_transferred.load(Ordering::Relaxed)
    }
}

/// DMA channel
#[derive(Debug)]
pub struct DmaChannel {
    /// Channel number
    channel_num: u32,
    /// Channel status
    status: Mutex<DmaChannelStatus>,
    /// Current transfer
    current_transfer: Mutex<Option<DmaTransfer>>,
    /// Transfer queue
    transfer_queue: Mutex<Vec<DmaTransfer>>,
    /// Max transfer size
    max_transfer_size: usize,
    /// Alignment requirement
    alignment: usize,
}

impl DmaChannel {
    /// Create new DMA channel
    pub fn new(channel_num: u32, max_transfer_size: usize, alignment: usize) -> Self {
        Self {
            channel_num,
            status: Mutex::new(DmaChannelStatus::Idle),
            current_transfer: Mutex::new(None),
            transfer_queue: Mutex::new(Vec::new()),
            max_transfer_size,
            alignment,
        }
    }

    /// Get channel number
    pub fn channel_num(&self) -> u32 {
        self.channel_num
    }

    /// Get channel status
    pub fn status(&self) -> DmaChannelStatus {
        *self.status.lock()
    }

    /// Submit transfer to channel
    pub fn submit_transfer(&self, transfer: DmaTransfer) -> Result<()> {
        // Validate transfer size
        if transfer.size > self.max_transfer_size {
            return Err(KernelError::InvalidArgument);
        }

        // Check alignment
        if transfer.src_addr % self.alignment as u64 != 0 ||
           transfer.dst_addr % self.alignment as u64 != 0 {
            return Err(KernelError::InvalidArgument);
        }

        let transfer_id = transfer.transfer_id;
        {
            let mut queue = self.transfer_queue.lock();
            queue.push(transfer);
        }

        crate::println!(
            "dma: submitted transfer {} to channel {}",
            transfer_id,
            self.channel_num
        );

        Ok(())
    }

    /// Start next transfer from queue
    pub fn start_next_transfer(&self) -> Result<Option<DmaTransfer>> {
        let mut status = self.status.lock();

        if *status != DmaChannelStatus::Idle {
            return Ok(None);
        }

        let mut queue = self.transfer_queue.lock();
        if queue.is_empty() {
            return Ok(None);
        }

        let transfer = queue.remove(0);

        {
            let mut current = self.current_transfer.lock();
            *current = Some(transfer.clone());
        }

        *status = DmaChannelStatus::Busy;

        crate::println!(
            "dma: started transfer {} on channel {}",
            transfer.transfer_id,
            self.channel_num
        );

        Ok(Some(transfer))
    }

    /// Complete current transfer
    pub fn complete_transfer(&self) -> Result<Option<DmaTransfer>> {
        let mut status = self.status.lock();

        if *status != DmaChannelStatus::Busy {
            return Ok(None);
        }

        let mut current = self.current_transfer.lock();
        let transfer = current.take();

        if let Some(ref tx) = transfer {
            tx.mark_completed();

            // Call callback if set
            if let Some(callback) = tx.callback {
                crate::println!(
                    "dma: invoking callback {:#x} for transfer {}",
                    callback,
                    tx.transfer_id
                );
                // In real implementation, this would call the callback
            }
        }

        *status = DmaChannelStatus::Idle;

        if let Some(ref tx) = transfer {
            crate::println!(
                "dma: completed transfer {} on channel {} ({} bytes)",
                tx.transfer_id,
                self.channel_num,
                tx.get_bytes_transferred()
            );
        }

        Ok(transfer)
    }

    /// Cancel current transfer
    pub fn cancel_transfer(&self) -> Result<bool> {
        let mut status = self.status.lock();

        if *status != DmaChannelStatus::Busy {
            return Ok(false);
        }

        let mut current = self.current_transfer.lock();
        *current = None;

        *status = DmaChannelStatus::Idle;

        crate::println!(
            "dma: canceled transfer on channel {}",
            self.channel_num
        );

        Ok(true)
    }

    /// Pause channel
    pub fn pause(&self) -> Result<()> {
        let mut status = self.status.lock();

        if *status != DmaChannelStatus::Busy {
            return Err(KernelError::InvalidState);
        }

        *status = DmaChannelStatus::Paused;

        crate::println!("dma: paused channel {}", self.channel_num);

        Ok(())
    }

    /// Resume channel
    pub fn resume(&self) -> Result<()> {
        let mut status = self.status.lock();

        if *status != DmaChannelStatus::Paused {
            return Err(KernelError::InvalidState);
        }

        *status = DmaChannelStatus::Busy;

        crate::println!("dma: resumed channel {}", self.channel_num);

        Ok(())
    }
}

// ============================================================================
// DMA Engine
// ============================================================================

/// DMA engine statistics
#[derive(Debug, Default, Clone)]
pub struct DmaStats {
    /// Total transfers submitted
    pub transfers_submitted: u64,
    /// Total transfers completed
    pub transfers_completed: u64,
    /// Total bytes transferred
    pub bytes_transferred: u64,
    /// Transfer errors
    pub transfer_errors: u64,
    /// Active channels
    pub active_channels: u32,
}

/// DMA engine - manages DMA channels and mappings
pub struct DmaEngine {
    /// DMA channels
    channels: Mutex<BTreeMap<u32, DmaChannel>>,
    /// DMA mappings (mapping_id -> mapping)
    mappings: Mutex<BTreeMap<u32, DmaMapping>>,
    /// Next channel number
    next_channel: AtomicU32,
    /// Next mapping ID
    next_mapping_id: AtomicU32,
    /// Statistics
    stats: Mutex<DmaStats>,
    /// Max transfer size
    max_transfer_size: usize,
    /// Default alignment
    default_alignment: usize,
}

impl DmaEngine {
    /// Create new DMA engine
    pub fn new(max_transfer_size: usize, default_alignment: usize) -> Self {
        Self {
            channels: Mutex::new(BTreeMap::new()),
            mappings: Mutex::new(BTreeMap::new()),
            next_channel: AtomicU32::new(0),
            next_mapping_id: AtomicU32::new(0),
            stats: Mutex::new(DmaStats::default()),
            max_transfer_size,
            default_alignment,
        }
    }

    /// Allocate DMA channel
    pub fn allocate_channel(&self, alignment: usize) -> Result<u32> {
        let channel_num = self.next_channel.fetch_add(1, Ordering::SeqCst);

        let channel = DmaChannel::new(channel_num, self.max_transfer_size, alignment);

        {
            let mut channels = self.channels.lock();
            channels.insert(channel_num, channel);
        }

        {
            let mut stats = self.stats.lock();
            stats.active_channels += 1;
        }

        crate::println!(
            "dma: allocated channel {} (alignment: {})",
            channel_num,
            alignment
        );

        Ok(channel_num)
    }

    /// Free DMA channel
    pub fn free_channel(&self, channel_num: u32) -> Result<()> {
        {
            let mut channels = self.channels.lock();
            channels.remove(&channel_num).ok_or(KernelError::NotFound)?;
        }

        {
            let mut stats = self.stats.lock();
            stats.active_channels = stats.active_channels.saturating_sub(1);
        }

        crate::println!("dma: freed channel {}", channel_num);

        Ok(())
    }

    /// Get DMA channel
    pub fn get_channel(&self, _channel_num: u32) -> Result<DmaChannel> {
        // Note: This is a clone for simplicity; in real implementation would use Arc
        Err(KernelError::NotSupported)
    }

    /// Submit transfer to channel
    pub fn submit_transfer(&self, channel_num: u32, transfer: DmaTransfer) -> Result<()> {
        let channels = self.channels.lock();
        let channel = channels.get(&channel_num).ok_or(KernelError::NotFound)?;

        channel.submit_transfer(transfer)?;

        {
            let mut stats = self.stats.lock();
            stats.transfers_submitted += 1;
        }

        Ok(())
    }

    /// Start transfer on channel
    pub fn start_transfer(&self, channel_num: u32) -> Result<Option<DmaTransfer>> {
        let channels = self.channels.lock();
        let channel = channels.get(&channel_num).ok_or(KernelError::NotFound)?;

        channel.start_next_transfer()
    }

    /// Complete transfer on channel
    pub fn complete_transfer(&self, channel_num: u32) -> Result<Option<DmaTransfer>> {
        let channels = self.channels.lock();
        let channel = channels.get(&channel_num).ok_or(KernelError::NotFound)?;

        let transfer = channel.complete_transfer()?;

        if let Some(ref tx) = transfer {
            let mut stats = self.stats.lock();
            stats.transfers_completed += 1;
            stats.bytes_transferred += tx.get_bytes_transferred();
        }

        Ok(transfer)
    }

    /// Create DMA mapping
    pub fn map(
        &self,
        virt_addr: u64,
        size: usize,
        direction: DmaDirection,
        coherent: bool,
    ) -> Result<DmaMapping> {
        let mapping_id = self.next_mapping_id.fetch_add(1, Ordering::SeqCst);

        // In real implementation, this would:
        // 1. Allocate physical memory or get physical pages
        // 2. Create IOMMU mapping if available
        // 3. Return physical address suitable for DMA

        let dma_addr = virt_addr; // Simplified: identity mapping

        let mapping = DmaMapping::new(
            virt_addr,
            dma_addr,
            size,
            direction,
            coherent,
            mapping_id,
        );

        {
            let mut mappings = self.mappings.lock();
            mappings.insert(mapping_id, mapping.clone());
        }

        crate::println!(
            "dma: created mapping {} (virt: {:#x}, dma: {:#x}, size: {}, coherent: {})",
            mapping_id,
            virt_addr,
            dma_addr,
            size,
            coherent
        );

        Ok(mapping)
    }

    /// Unmap DMA mapping
    pub fn unmap(&self, mapping: &DmaMapping) -> Result<()> {
        let mut mappings = self.mappings.lock();
        mappings.remove(&mapping.mapping_id).ok_or(KernelError::NotFound)?;

        crate::println!(
            "dma: unmapped mapping {} (addr: {:#x}, size: {})",
            mapping.mapping_id,
            mapping.dma_addr,
            mapping.size
        );

        Ok(())
    }

    /// Sync DMA mapping for CPU
    pub fn sync_for_cpu(&self, mapping: &DmaMapping) -> Result<()> {
        if mapping.coherent {
            return Ok(()); // Coherent mappings don't need sync
        }

        // In real implementation, this would invalidate cache
        crate::println!(
            "dma: sync mapping {} for CPU (addr: {:#x}, size: {})",
            mapping.mapping_id,
            mapping.dma_addr,
            mapping.size
        );

        Ok(())
    }

    /// Sync DMA mapping for device
    pub fn sync_for_device(&self, mapping: &DmaMapping) -> Result<()> {
        if mapping.coherent {
            return Ok(()); // Coherent mappings don't need sync
        }

        // In real implementation, this would clean cache
        crate::println!(
            "dma: sync mapping {} for device (addr: {:#x}, size: {})",
            mapping.mapping_id,
            mapping.dma_addr,
            mapping.size
        );

        Ok(())
    }

    /// Allocate scatter-gather list
    pub fn alloc_sg(&self, entries: Vec<ScatterGatherEntry>) -> Result<ScatterGatherList> {
        if entries.len() > MAX_SG_ENTRIES {
            return Err(KernelError::InvalidArgument);
        }

        let sg_list = ScatterGatherList::new(entries);

        crate::println!(
            "dma: allocated SG list with {} entries (total: {} bytes)",
            sg_list.len(),
            sg_list.total_length()
        );

        Ok(sg_list)
    }

    /// Free scatter-gather list
    pub fn free_sg(&self, _sg_list: &ScatterGatherList) -> Result<()> {
        // In real implementation, this would free associated resources
        Ok(())
    }

    /// Get statistics
    pub fn get_stats(&self) -> DmaStats {
        self.stats.lock().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = DmaStats::default();
    }
}

impl Default for DmaEngine {
    fn default() -> Self {
        Self::new(DMA_PAGE_SIZE, DMA_MIN_ALIGNMENT)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sg_entry() {
        let entry = ScatterGatherEntry::new(0x1000, 0x1000);
        assert!(entry.is_valid());
        assert_eq!(entry.address, 0x1000);
        assert_eq!(entry.length, 0x1000);
    }

    #[test]
    fn test_sg_list() {
        let entries = vec![
            ScatterGatherEntry::new(0x1000, 0x1000),
            ScatterGatherEntry::new(0x2000, 0x2000),
            ScatterGatherEntry::new(0x4000, 0x1000),
        ];

        let sg_list = ScatterGatherList::new(entries);
        assert_eq!(sg_list.len(), 3);
        assert_eq!(sg_list.total_length(), 0x4000);
        assert!(!sg_list.is_empty());

        let entry = sg_list.get(0).unwrap();
        assert_eq!(entry.address, 0x1000);
        assert_eq!(entry.length, 0x1000);
    }

    #[test]
    fn test_dma_transfer() {
        let transfer = DmaTransfer::new(1, 0x1000, 0x2000, 0x1000, DmaDirection::ToMemory);
        assert_eq!(transfer.transfer_id, 1);
        assert_eq!(transfer.src_addr, 0x1000);
        assert_eq!(transfer.dst_addr, 0x2000);
        assert!(!transfer.is_sg());

        transfer.mark_completed();
        assert!(transfer.is_completed());

        transfer.update_transferred(0x800);
        assert_eq!(transfer.get_bytes_transferred(), 0x800);
    }

    #[test]
    fn test_sg_transfer() {
        let entries = vec![
            ScatterGatherEntry::new(0x1000, 0x1000),
            ScatterGatherEntry::new(0x2000, 0x1000),
        ];
        let sg_list = ScatterGatherList::new(entries);

        let transfer = DmaTransfer::with_sg_list(
            2,
            0x3000,
            0x5000,
            DmaDirection::ToDevice,
            sg_list,
        );

        assert!(transfer.is_sg());
        assert_eq!(transfer.size, 0x2000);
        assert!(transfer.get_sg_list().is_some());
    }

    #[test]
    fn test_dma_channel() {
        let channel = DmaChannel::new(0, 0x10000, 64);

        assert_eq!(channel.channel_num(), 0);
        assert_eq!(channel.status(), DmaChannelStatus::Idle);

        let transfer = DmaTransfer::new(1, 0x1000, 0x2000, 0x1000, DmaDirection::ToMemory);
        channel.submit_transfer(transfer).unwrap();

        let started = channel.start_next_transfer().unwrap();
        assert!(started.is_some());
        assert_eq!(channel.status(), DmaChannelStatus::Busy);

        channel.pause().unwrap();
        assert_eq!(channel.status(), DmaChannelStatus::Paused);

        channel.resume().unwrap();
        assert_eq!(channel.status(), DmaChannelStatus::Busy);

        let completed = channel.complete_transfer().unwrap();
        assert!(completed.is_some());
        assert_eq!(channel.status(), DmaChannelStatus::Idle);
    }

    #[test]
    fn test_dma_mapping() {
        let mapping = DmaMapping::new(
            0xFFFF0000,
            0x1000,
            0x1000,
            DmaDirection::Bidirectional,
            true,
            1,
        );

        assert!(mapping.is_valid());
        assert_eq!(mapping.physical_address(), 0x1000);
        assert_eq!(mapping.virtual_address(), 0xFFFF0000);
        assert!(mapping.coherent);
    }

    #[test]
    fn test_dma_engine() {
        let engine = DmaEngine::new(0x10000, 64);

        // Allocate channel
        let channel_num = engine.allocate_channel(64).unwrap();
        assert_eq!(channel_num, 0);

        // Submit transfer
        let transfer = DmaTransfer::new(1, 0x1000, 0x2000, 0x1000, DmaDirection::ToMemory);
        engine.submit_transfer(channel_num, transfer).unwrap();

        // Create mapping
        let mapping = engine.map(0xFFFF0000, 0x1000, DmaDirection::ToMemory, true).unwrap();
        assert!(mapping.is_valid());

        // Sync mapping
        engine.sync_for_device(&mapping).unwrap();
        engine.sync_for_cpu(&mapping).unwrap();

        // Unmap mapping
        engine.unmap(&mapping).unwrap();

        // Free channel
        engine.free_channel(channel_num).unwrap();

        let stats = engine.get_stats();
        assert_eq!(stats.transfers_submitted, 1);
        assert_eq!(stats.active_channels, 0);
    }
}
