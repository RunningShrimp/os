/// Master Boot Record (MBR) Handler
///
/// Parses and validates MBR boot sector for disk-based bootloaders.
/// Handles partition table and boot signatures.

/// MBR signature (0xAA55 at offset 510-511)
pub const MBR_SIGNATURE: u16 = 0xAA55;

/// MBR signature offset
pub const MBR_SIGNATURE_OFFSET: usize = 510;

/// Partition entry size
pub const PARTITION_ENTRY_SIZE: usize = 16;

/// Number of partition entries in MBR
pub const MAX_PARTITIONS: usize = 4;

/// MBR boot code size
pub const MBR_BOOTCODE_SIZE: usize = 446;

/// Partition type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionType {
    Empty,
    FAT12,
    FAT16,
    FAT32,
    NTFS,
    Linux,
    LVM,
    Unknown(u8),
}

impl PartitionType {
    pub fn from_code(code: u8) -> Self {
        match code {
            0x00 => Self::Empty,
            0x01 => Self::FAT12,
            0x04 | 0x06 | 0x0E => Self::FAT16,
            0x0C | 0x1C => Self::FAT32,
            0x07 => Self::NTFS,
            0x83 => Self::Linux,
            0x8E => Self::LVM,
            _ => Self::Unknown(code),
        }
    }

    pub fn code(&self) -> u8 {
        match self {
            Self::Empty => 0x00,
            Self::FAT12 => 0x01,
            Self::FAT16 => 0x04,
            Self::FAT32 => 0x0C,
            Self::NTFS => 0x07,
            Self::Linux => 0x83,
            Self::LVM => 0x8E,
            Self::Unknown(code) => *code,
        }
    }

    pub fn is_bootable(&self) -> bool {
        matches!(self, Self::FAT12 | Self::FAT16 | Self::FAT32 | Self::Linux)
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Empty => "Empty",
            Self::FAT12 => "FAT12",
            Self::FAT16 => "FAT16",
            Self::FAT32 => "FAT32",
            Self::NTFS => "NTFS",
            Self::Linux => "Linux",
            Self::LVM => "LVM",
            Self::Unknown(_) => "Unknown",
        }
    }
}

/// MBR partition entry
#[derive(Debug, Clone, Copy)]
pub struct PartitionEntry {
    pub boot_flag: u8,
    pub start_head: u8,
    pub start_sector: u8,
    pub start_cylinder: u16,
    pub partition_type: PartitionType,
    pub end_head: u8,
    pub end_sector: u8,
    pub end_cylinder: u16,
    pub start_lba: u32,
    pub size_sectors: u32,
}

impl PartitionEntry {
    /// Create new partition entry
    pub fn new() -> Self {
        Self {
            boot_flag: 0,
            start_head: 0,
            start_sector: 0,
            start_cylinder: 0,
            partition_type: PartitionType::Empty,
            end_head: 0,
            end_sector: 0,
            end_cylinder: 0,
            start_lba: 0,
            size_sectors: 0,
        }
    }

    /// Check if partition is bootable
    pub fn is_bootable(&self) -> bool {
        self.boot_flag == 0x80 && self.partition_type.is_bootable()
    }

    /// Check if partition is valid
    pub fn is_valid(&self) -> bool {
        self.partition_type != PartitionType::Empty
            && self.start_lba != 0
            && self.size_sectors != 0
    }

    /// Get partition size in bytes
    pub fn size_bytes(&self) -> u64 {
        self.size_sectors as u64 * 512
    }

    /// Get partition end LBA
    pub fn end_lba(&self) -> u32 {
        self.start_lba.saturating_add(self.size_sectors)
    }
}

/// Master Boot Record
pub struct MasterBootRecord {
    pub bootcode: [u8; MBR_BOOTCODE_SIZE],
    pub partitions: [Option<PartitionEntry>; MAX_PARTITIONS],
    pub signature: u16,
    pub valid: bool,
}

impl MasterBootRecord {
    /// Create new MBR
    pub fn new() -> Self {
        Self {
            bootcode: [0; MBR_BOOTCODE_SIZE],
            partitions: [None; MAX_PARTITIONS],
            signature: 0,
            valid: false,
        }
    }

    /// Parse MBR from sector data
    pub fn parse(&mut self, sector: &[u8; 512]) -> Result<(), &'static str> {
        if sector.len() < 512 {
            return Err("Invalid sector size");
        }

        // Copy boot code (first 446 bytes)
        for i in 0..MBR_BOOTCODE_SIZE {
            self.bootcode[i] = sector[i];
        }

        // Parse partition entries
        for i in 0..MAX_PARTITIONS {
            let offset = 446 + (i * PARTITION_ENTRY_SIZE);

            let boot_flag = sector[offset];
            let partition_type = PartitionType::from_code(sector[offset + 4]);

            let start_lba = u32::from_le_bytes([
                sector[offset + 8],
                sector[offset + 9],
                sector[offset + 10],
                sector[offset + 11],
            ]);

            let size_sectors = u32::from_le_bytes([
                sector[offset + 12],
                sector[offset + 13],
                sector[offset + 14],
                sector[offset + 15],
            ]);

            let entry = PartitionEntry {
                boot_flag,
                start_head: sector[offset + 1],
                start_sector: sector[offset + 2],
                start_cylinder: u16::from_le_bytes([
                    sector[offset + 3],
                    (sector[offset + 2] >> 6) as u8,
                ]),
                partition_type,
                end_head: sector[offset + 5],
                end_sector: sector[offset + 6],
                end_cylinder: u16::from_le_bytes([
                    sector[offset + 7],
                    (sector[offset + 6] >> 6) as u8,
                ]),
                start_lba,
                size_sectors,
            };

            if entry.partition_type != PartitionType::Empty {
                self.partitions[i] = Some(entry);
            }
        }

        // Get signature (last 2 bytes)
        self.signature = u16::from_le_bytes([sector[510], sector[511]]);

        // Validate signature
        self.valid = self.signature == MBR_SIGNATURE;

        Ok(())
    }

    /// Find bootable partition
    pub fn find_bootable_partition(&self) -> Option<&PartitionEntry> {
        self.partitions
            .iter()
            .find_map(|p| p.as_ref().filter(|entry| entry.is_bootable()))
    }

    /// Get partition by index
    pub fn partition(&self, index: usize) -> Option<&PartitionEntry> {
        if index < MAX_PARTITIONS {
            self.partitions[index].as_ref()
        } else {
            None
        }
    }

    /// Count valid partitions
    pub fn partition_count(&self) -> usize {
        self.partitions.iter().filter(|p| p.is_some()).count()
    }

    /// Count bootable partitions
    pub fn bootable_count(&self) -> usize {
        self.partitions
            .iter()
            .filter(|p| p.as_ref().map_or(false, |e| e.is_bootable()))
            .count()
    }

    /// Check if MBR is valid
    pub fn is_valid(&self) -> bool {
        self.valid && self.signature == MBR_SIGNATURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_type_from_code() {
        assert_eq!(PartitionType::from_code(0x00), PartitionType::Empty);
        assert_eq!(PartitionType::from_code(0x83), PartitionType::Linux);
    }

    #[test]
    fn test_partition_type_is_bootable() {
        assert!(PartitionType::Linux.is_bootable());
        assert!(!PartitionType::Empty.is_bootable());
    }

    #[test]
    fn test_partition_entry_creation() {
        let entry = PartitionEntry::new();
        assert!(!entry.is_bootable());
    }

    #[test]
    fn test_partition_entry_is_bootable() {
        let mut entry = PartitionEntry::new();
        entry.boot_flag = 0x80;
        entry.partition_type = PartitionType::Linux;

        assert!(entry.is_bootable());
    }

    #[test]
    fn test_partition_entry_size_bytes() {
        let mut entry = PartitionEntry::new();
        entry.size_sectors = 2048;

        assert_eq!(entry.size_bytes(), 2048 * 512);
    }

    #[test]
    fn test_partition_entry_end_lba() {
        let mut entry = PartitionEntry::new();
        entry.start_lba = 2048;
        entry.size_sectors = 1024;

        assert_eq!(entry.end_lba(), 3072);
    }

    #[test]
    fn test_mbr_creation() {
        let mbr = MasterBootRecord::new();
        assert!(!mbr.is_valid());
    }

    #[test]
    fn test_mbr_signature_constant() {
        assert_eq!(MBR_SIGNATURE, 0xAA55);
    }

    #[test]
    fn test_mbr_partition_count() {
        let mut mbr = MasterBootRecord::new();
        let entry = PartitionEntry::new();

        mbr.partitions[0] = Some(entry);
        assert_eq!(mbr.partition_count(), 1);
    }

    #[test]
    fn test_mbr_find_bootable() {
        let mut mbr = MasterBootRecord::new();
        let mut entry = PartitionEntry::new();
        entry.boot_flag = 0x80;
        entry.partition_type = PartitionType::Linux;

        mbr.partitions[0] = Some(entry);
        assert!(mbr.find_bootable_partition().is_some());
    }
}
