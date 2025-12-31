//! PCI MSI/MSI-X 中断支持
//!
//! 本模块提供PCI设备的Message Signaled Interrupts (MSI)和MSI-X支持，包括：
//! - MSI向量分配
//! - MSI-X表管理
//! - 中断亲和性控制
//! - 中断掩码和使能

use crate::prelude::*;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use core::sync::atomic::{AtomicU32, Ordering};

// ============================================================================
// MSI/MSI-X 常量
// ============================================================================

/// MSI使能位
pub const MSI_ENABLE: u16 = 0x0001;

/// MSI-X使能位
pub const MSI_X_ENABLE: u16 = 0x8000;

/// MSI-X函数掩码
pub const MSI_X_FUNCTION_MASK: u16 = 0x4000;

/// MSI-X表大小掩码
pub const MSI_X_TABLE_SIZE_MASK: u16 = 0x07FF;

/// MSI-X表条目大小
pub const MSI_X_TABLE_ENTRY_SIZE: usize = 16;

/// 最大MSI向量数
pub const MAX_MSI_VECTORS: u16 = 32;

/// 最大MSI-X向量数
pub const MAX_MSIX_VECTORS: u16 = 2048;

// ============================================================================
// MSI 向量
// ============================================================================

/// MSI向量描述符
#[derive(Debug, Clone)]
pub struct MsiVector {
    /// 向量编号
    pub vector: u32,
    /// 中断号
    pub irq: u32,
    /// CPU亲和性
    pub cpu_affinity: Option<u32>,
    /// 是否使能
    pub enabled: bool,
    /// 是否被掩码
    pub masked: bool,
    /// 地址（用于MSI）
    pub address: u64,
    /// 数据（用于MSI）
    pub data: u32,
}

impl MsiVector {
    /// 创建新的MSI向量
    pub fn new(vector: u32, irq: u32) -> Self {
        Self {
            vector,
            irq,
            cpu_affinity: None,
            enabled: false,
            masked: false,
            address: 0,
            data: 0,
        }
    }

    /// 设置CPU亲和性
    pub fn with_cpu_affinity(mut self, cpu: u32) -> Self {
        self.cpu_affinity = Some(cpu);
        self
    }

    /// 使能向量
    pub fn enable(&mut self) {
        self.enabled = true;
        self.masked = false;
    }

    /// 掩码向量
    pub fn mask(&mut self) {
        self.masked = true;
    }

    /// 取消掩码向量
    pub fn unmask(&mut self) {
        self.masked = false;
    }
}

// ============================================================================
// MSI-X 表
// ============================================================================

/// MSI-X表条目
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MsiXTableEntry {
    /// 消息地址（低32位）
    pub msg_addr_lo: u32,
    /// 消息地址（高32位）
    pub msg_addr_hi: u32,
    /// 消息数据
    pub msg_data: u32,
    /// 向量控制（bit 0: 掩码位）
    pub vector_control: u32,
}

impl Default for MsiXTableEntry {
    fn default() -> Self {
        Self {
            msg_addr_lo: 0,
            msg_addr_hi: 0,
            msg_data: 0,
            vector_control: 1, // 默认掩码
        }
    }
}

impl MsiXTableEntry {
    /// 创建新的表条目
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置消息地址
    pub fn set_address(&mut self, address: u64) {
        self.msg_addr_lo = (address & 0xFFFFFFFF) as u32;
        self.msg_addr_hi = ((address >> 32) & 0xFFFFFFFF) as u32;
    }

    /// 设置消息数据
    pub fn set_data(&mut self, data: u32) {
        self.msg_data = data;
    }

    /// 掩码向量
    pub fn mask(&mut self) {
        self.vector_control |= 0x1;
    }

    /// 取消掩码向量
    pub fn unmask(&mut self) {
        self.vector_control &= !0x1;
    }

    /// 检查是否被掩码
    pub fn is_masked(&self) -> bool {
        (self.vector_control & 0x1) != 0
    }
}

/// MSI-X表
#[derive(Debug, Clone)]
pub struct MsiXTable {
    /// 表条目
    entries: Vec<MsiXTableEntry>,
    /// Pending Bit Array (PBA)
    pba: Vec<u64>,
    /// 表偏移（在BAR中）
    table_offset: u32,
    /// PBA偏移（在BAR中）
    pba_offset: u32,
    /// BAR索引
    bar_index: u8,
}

impl MsiXTable {
    /// 创建新的MSI-X表
    pub fn new(num_entries: u16, table_offset: u32, pba_offset: u32, bar_index: u8) -> Self {
        let table_size = num_entries as usize;
        let pba_size = ((num_entries as usize) + 63) / 64; // 每个条目1 bit

        Self {
            entries: vec![MsiXTableEntry::default(); table_size],
            pba: vec![0u64; pba_size],
            table_offset,
            pba_offset,
            bar_index,
        }
    }

    /// 获取条目数量
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// 获取条目
    pub fn get_entry(&self, index: usize) -> Option<&MsiXTableEntry> {
        self.entries.get(index)
    }

    /// 设置条目
    pub fn set_entry(&mut self, index: usize, entry: MsiXTableEntry) -> Result<()> {
        if index >= self.entries.len() {
            return Err(KernelError::InvalidArgument);
        }

        self.entries[index] = entry;
        Ok(())
    }

    /// 掩码条目
    pub fn mask_entry(&mut self, index: usize) -> Result<()> {
        if index >= self.entries.len() {
            return Err(KernelError::InvalidArgument);
        }

        self.entries[index].mask();
        Ok(())
    }

    /// 取消掩码条目
    pub fn unmask_entry(&mut self, index: usize) -> Result<()> {
        if index >= self.entries.len() {
            return Err(KernelError::InvalidArgument);
        }

        self.entries[index].unmask();
        Ok(())
    }

    /// 检查pending位
    pub fn is_pending(&self, index: usize) -> bool {
        let word_index = index / 64;
        let bit_index = index % 64;

        if word_index >= self.pba.len() {
            return false;
        }

        (self.pba[word_index] & (1 << bit_index)) != 0
    }

    /// 清除pending位
    pub fn clear_pending(&mut self, index: usize) -> Result<()> {
        let word_index = index / 64;
        let bit_index = index % 64;

        if word_index >= self.pba.len() {
            return Err(KernelError::InvalidArgument);
        }

        self.pba[word_index] &= !(1 << bit_index);
        Ok(())
    }

    /// 获取表偏移
    pub fn table_offset(&self) -> u32 {
        self.table_offset
    }

    /// 获取PBA偏移
    pub fn pba_offset(&self) -> u32 {
        self.pba_offset
    }

    /// 获取BAR索引
    pub fn bar_index(&self) -> u8 {
        self.bar_index
    }
}

// ============================================================================
// MSI 管理器
// ============================================================================

/// MSI管理器统计信息
#[derive(Debug, Default, Clone)]
pub struct MsiStats {
    /// 已分配的MSI向量数
    pub allocated_msi_vectors: u32,
    /// 已分配的MSI-X向量数
    pub allocated_msix_vectors: u32,
    /// 总MSI向量数
    pub total_msi_vectors: u32,
    /// 总MSI-X向量数
    pub total_msix_vectors: u32,
    /// 中断处理次数
    pub interrupt_count: u64,
}

/// MSI管理器
pub struct MsiManager {
    /// 下一个可用的IRQ向量
    next_irq: AtomicU32,
    /// 已分配的MSI向量
    msi_vectors: Mutex<BTreeMap<u32, MsiVector>>,
    /// 已分配的MSI-X表
    msix_tables: Mutex<BTreeMap<u32, MsiXTable>>,
    /// 设备ID到MSI-X表的映射
    device_msix_map: Mutex<BTreeMap<u32, Vec<MsiVector>>>,
    /// 统计信息
    stats: Mutex<MsiStats>,
    /// 基础IRQ号
    base_irq: u32,
}

impl MsiManager {
    /// 创建新的MSI管理器
    pub fn new(base_irq: u32) -> Self {
        Self {
            next_irq: AtomicU32::new(base_irq),
            msi_vectors: Mutex::new(BTreeMap::new()),
            msix_tables: Mutex::new(BTreeMap::new()),
            device_msix_map: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(MsiStats::default()),
            base_irq,
        }
    }

    /// 分配MSI向量
    pub fn allocate_msi(
        &self,
        device_id: u32,
        count: u16,
    ) -> Result<Vec<MsiVector>> {
        if count == 0 || count > MAX_MSI_VECTORS {
            return Err(KernelError::InvalidArgument);
        }

        let mut vectors = Vec::new();

        for _ in 0..count {
            let irq = self.next_irq.fetch_add(1, Ordering::SeqCst);
            let vector = MsiVector::new(irq, irq);
            vectors.push(vector);
        }

        // 记录分配
        {
            let mut msi_vecs = self.msi_vectors.lock();
            for vector in &vectors {
                msi_vecs.insert(vector.irq, vector.clone());
            }
        }

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.allocated_msi_vectors += count as u32;
            stats.total_msi_vectors += count as u32;
        }

        crate::println!(
            "msi: allocated {} MSI vectors for device {}",
            count,
            device_id
        );

        Ok(vectors)
    }

    /// 分配MSI-X表
    pub fn allocate_msix(
        &self,
        device_id: u32,
        num_entries: u16,
        table_offset: u32,
        pba_offset: u32,
        bar_index: u8,
    ) -> Result<MsiXTable> {
        if num_entries == 0 || num_entries > MAX_MSIX_VECTORS {
            return Err(KernelError::InvalidArgument);
        }

        let table = MsiXTable::new(num_entries, table_offset, pba_offset, bar_index);

        // 分配IRQ向量
        let mut vectors = Vec::new();
        for i in 0..num_entries {
            let irq = self.next_irq.fetch_add(1, Ordering::SeqCst);
            let vector = MsiVector::new(irq, irq);
            vectors.push(vector);
        }

        // 记录映射
        {
            let mut map = self.device_msix_map.lock();
            map.insert(device_id, vectors);
        }

        // 保存表
        {
            let mut tables = self.msix_tables.lock();
            tables.insert(device_id, table.clone());
        }

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.allocated_msix_vectors += num_entries as u32;
            stats.total_msix_vectors += num_entries as u32;
        }

        crate::println!(
            "msi: allocated MSI-X table with {} entries for device {}",
            num_entries,
            device_id
        );

        Ok(table)
    }

    /// 释放MSI向量
    pub fn free_msi(&self, vectors: &[MsiVector]) -> Result<()> {
        for vector in vectors {
            let mut msi_vecs = self.msi_vectors.lock();
            msi_vecs.remove(&vector.irq);
        }

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.allocated_msi_vectors = stats.allocated_msi_vectors.saturating_sub(vectors.len() as u32);
        }

        crate::println!("msi: freed {} MSI vectors", vectors.len());
        Ok(())
    }

    /// 释放MSI-X表
    pub fn free_msix(&self, device_id: u32) -> Result<()> {
        // 移除设备映射
        {
            let mut map = self.device_msix_map.lock();
            map.remove(&device_id);
        }

        // 移除表
        {
            let mut tables = self.msix_tables.lock();
            if let Some(table) = tables.remove(&device_id) {
                // 更新统计
                let mut stats = self.stats.lock();
                stats.allocated_msix_vectors = stats.allocated_msix_vectors.saturating_sub(table.entry_count() as u32);
            }
        }

        crate::println!("msi: freed MSI-X table for device {}", device_id);
        Ok(())
    }

    /// 配置MSI向量
    pub fn configure_msi(
        &self,
        device_id: u32,
        vector: &mut MsiVector,
        address: u64,
        data: u32,
    ) -> Result<()> {
        vector.address = address;
        vector.data = data;

        crate::println!(
            "msi: configured MSI vector {} for device {} (addr: {:#x}, data: {:#x})",
            vector.vector,
            device_id,
            address,
            data
        );

        Ok(())
    }

    /// 配置MSI-X表条目
    pub fn configure_msix_entry(
        &self,
        device_id: u32,
        entry_index: usize,
        address: u64,
        data: u32,
    ) -> Result<()> {
        let mut tables = self.msix_tables.lock();
        let table = tables.get_mut(&device_id).ok_or(KernelError::NotFound)?;

        if entry_index >= table.entry_count() {
            return Err(KernelError::InvalidArgument);
        }

        let mut entry = MsiXTableEntry::new();
        entry.set_address(address);
        entry.set_data(data);

        table.set_entry(entry_index, entry)?;

        crate::println!(
            "msi: configured MSI-X entry {} for device {} (addr: {:#x}, data: {:#x})",
            entry_index,
            device_id,
            address,
            data
        );

        Ok(())
    }

    /// 使能MSI
    pub fn enable_msi(&self, device_id: u32, vectors: &mut [MsiVector]) -> Result<()> {
        for vector in &mut *vectors {
            vector.enable();
        }

        crate::println!(
            "msi: enabled {} MSI vectors for device {}",
            vectors.len(),
            device_id
        );

        Ok(())
    }

    /// 禁用MSI
    pub fn disable_msi(&self, device_id: u32, vectors: &mut [MsiVector]) -> Result<()> {
        for vector in &mut *vectors {
            vector.enabled = false;
        }

        crate::println!(
            "msi: disabled {} MSI vectors for device {}",
            vectors.len(),
            device_id
        );

        Ok(())
    }

    /// 使能MSI-X
    pub fn enable_msix(&self, device_id: u32) -> Result<()> {
        let tables = self.msix_tables.lock();
        let _table = tables.get(&device_id).ok_or(KernelError::NotFound)?;

        crate::println!("msi: enabled MSI-X for device {}", device_id);
        Ok(())
    }

    /// 禁用MSI-X
    pub fn disable_msix(&self, device_id: u32) -> Result<()> {
        let tables = self.msix_tables.lock();
        let _table = tables.get(&device_id).ok_or(KernelError::NotFound)?;

        crate::println!("msi: disabled MSI-X for device {}", device_id);
        Ok(())
    }

    /// 掩码MSI-X向量
    pub fn mask_msix_vector(&self, device_id: u32, index: usize) -> Result<()> {
        let mut tables = self.msix_tables.lock();
        let table = tables.get_mut(&device_id).ok_or(KernelError::NotFound)?;

        table.mask_entry(index)?;

        crate::println!(
            "msi: masked MSI-X vector {} for device {}",
            index,
            device_id
        );

        Ok(())
    }

    /// 取消掩码MSI-X向量
    pub fn unmask_msix_vector(&self, device_id: u32, index: usize) -> Result<()> {
        let mut tables = self.msix_tables.lock();
        let table = tables.get_mut(&device_id).ok_or(KernelError::NotFound)?;

        table.unmask_entry(index)?;

        crate::println!(
            "msi: unmasked MSI-X vector {} for device {}",
            index,
            device_id
        );

        Ok(())
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> MsiStats {
        self.stats.lock().clone()
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = MsiStats::default();
    }
}

impl Default for MsiManager {
    fn default() -> Self {
        Self::new(32) // 默认基础IRQ为32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msix_table_entry() {
        let mut entry = MsiXTableEntry::new();
        assert!(entry.is_masked());

        entry.unmask();
        assert!(!entry.is_masked());

        entry.set_address(0xFEEx0000);
        assert_eq!(entry.msg_addr_lo, 0x0000);
        assert_eq!(entry.msg_addr_hi, 0xFEE);

        entry.set_data(0x1234);
        assert_eq!(entry.msg_data, 0x1234);
    }

    #[test]
    fn test_msix_table() {
        let table = MsiXTable::new(16, 0x1000, 0x2000, 0);
        assert_eq!(table.entry_count(), 16);

        assert!(!table.is_pending(0));

        table.mask_entry(0).unwrap();
        assert!(table.get_entry(0).unwrap().is_masked());

        table.unmask_entry(0).unwrap();
        assert!(!table.get_entry(0).unwrap().is_masked());
    }

    #[test]
    fn test_msi_allocation() {
        let manager = MsiManager::new(100);

        let vectors = manager.allocate_msi(1, 4).unwrap();
        assert_eq!(vectors.len(), 4);
        assert_eq!(vectors[0].irq, 100);
        assert_eq!(vectors[3].irq, 103);

        let stats = manager.get_stats();
        assert_eq!(stats.allocated_msi_vectors, 4);
    }
}
