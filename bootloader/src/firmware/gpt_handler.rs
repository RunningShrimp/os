/// GUID Partition Table (GPT) Handler
///
/// Parses and validates GPT boot sectors for UEFI bootloaders.
/// Handles GPT headers and partition entries.

/// GPT signature ("EFI PART")
pub const GPT_SIGNATURE: [u8; 8] = [0x45, 0x46, 0x49, 0x20, 0x50, 0x41, 0x52, 0x54];

/// GPT header revision
pub const GPT_REVISION: u32 = 0x00010000;

/// Partition entry size
pub const GPT_ENTRY_SIZE: usize = 128;

/// Maximum partition entries
pub const MAX_GPT_PARTITIONS: usize = 128;

/// GPT partition type GUID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionTypeGUID {
    Unused,
    MBRPartition,
    EFISystem,
    BIOSBoot,
    LinuxFilesystem,
    LinuxBoot,
    Unknown,
}

impl PartitionTypeGUID {
    pub fn guid_string(&self) -> &'static str {
        match self {
            Self::Unused => "00000000-0000-0000-0000-000000000000",
            Self::MBRPartition => "024DEE41-33E7-11D3-9D69-0008C781F39F",
            Self::EFISystem => "C12A7328-F81F-11D2-BA4B-00A0C93EC93B",
            Self::BIOSBoot => "21686148-6449-6E6F-744E-656564454649",
            Self::LinuxFilesystem => "0FC63DAF-8483-4772-8E79-3D69D8477DE4",
            Self::LinuxBoot => "BC13C2FF-59E6-4262-A352-B275FD6F7172",
            Self::Unknown => "UNKNOWN",
        }
    }

    pub fn is_bootable(&self) -> bool {
        matches!(
            self,
            Self::EFISystem | Self::LinuxBoot | Self::BIOSBoot
        )
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Unused => "Unused",
            Self::MBRPartition => "MBR Partition",
            Self::EFISystem => "EFI System",
            Self::BIOSBoot => "BIOS Boot",
            Self::LinuxFilesystem => "Linux Filesystem",
            Self::LinuxBoot => "Linux Boot",
            Self::Unknown => "Unknown",
        }
    }
}

/// GPT partition entry
#[derive(Debug, Clone, Copy)]
pub struct GPTPartitionEntry {
    pub partition_type_guid: PartitionTypeGUID,
    pub unique_guid: [u8; 16],
    pub start_lba: u64,
    pub end_lba: u64,
    pub flags: u64,
    pub name: [u8; 72],
}

impl GPTPartitionEntry {
    /// Create new GPT partition entry
    pub fn new() -> Self {
        Self {
            partition_type_guid: PartitionTypeGUID::Unused,
            unique_guid: [0; 16],
            start_lba: 0,
            end_lba: 0,
            flags: 0,
            name: [0; 72],
        }
    }

    /// Check if partition is bootable
    pub fn is_bootable(&self) -> bool {
        self.partition_type_guid.is_bootable()
            && self.start_lba != 0
            && self.end_lba > self.start_lba
    }

    /// Check if partition is valid
    pub fn is_valid(&self) -> bool {
        self.partition_type_guid != PartitionTypeGUID::Unused
            && self.start_lba != 0
            && self.end_lba > self.start_lba
    }

    /// Get partition size in sectors
    pub fn size_sectors(&self) -> u64 {
        self.end_lba.saturating_sub(self.start_lba) + 1
    }

    /// Get partition size in bytes
    pub fn size_bytes(&self) -> u64 {
        self.size_sectors() * 512
    }
}

/// GPT header
#[derive(Debug, Clone, Copy)]
pub struct GPTHeader {
    pub signature: [u8; 8],
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub primary_lba: u64,
    pub backup_lba: u64,
    pub first_usable_lba: u64,
    pub last_usable_lba: u64,
    pub partition_entry_lba: u64,
    pub partition_entry_count: u32,
    pub partition_entry_size: u32,
}

impl GPTHeader {
    /// Create new GPT header
    pub fn new() -> Self {
        Self {
            signature: GPT_SIGNATURE,
            revision: GPT_REVISION,
            header_size: 92,
            crc32: 0,
            primary_lba: 1,
            backup_lba: 0,
            first_usable_lba: 34,
            last_usable_lba: 0,
            partition_entry_lba: 2,
            partition_entry_count: 128,
            partition_entry_size: 128,
        }
    }

    /// Verify signature
    pub fn verify_signature(&self) -> bool {
        self.signature == GPT_SIGNATURE
    }

    /// Verify revision
    pub fn verify_revision(&self) -> bool {
        self.revision == GPT_REVISION
    }
}

/// GUID Partition Table handler
pub struct GPTHandler {
    pub header: GPTHeader,
    pub partitions: [Option<GPTPartitionEntry>; MAX_GPT_PARTITIONS],
    pub valid: bool,
}

impl GPTHandler {
    /// Create new GPT handler
    pub fn new() -> Self {
        Self {
            header: GPTHeader::new(),
            partitions: [None; MAX_GPT_PARTITIONS],
            valid: false,
        }
    }

    /// Parse GPT header from sector
    pub fn parse_header(&mut self, sector: &[u8; 512]) -> Result<(), &'static str> {
        if sector.len() < 92 {
            return Err("Invalid sector size");
        }

        // Copy signature (8 bytes at offset 0)
        for i in 0..8 {
            self.header.signature[i] = sector[i];
        }

        // Verify signature
        if !self.header.verify_signature() {
            return Err("Invalid GPT signature");
        }

        // Get revision (4 bytes at offset 8)
        self.header.revision = u32::from_le_bytes([
            sector[8],
            sector[9],
            sector[10],
            sector[11],
        ]);

        // Get header size (4 bytes at offset 12)
        self.header.header_size =
            u32::from_le_bytes([sector[12], sector[13], sector[14], sector[15]]);

        // Get primary LBA (8 bytes at offset 24)
        self.header.primary_lba = u64::from_le_bytes([
            sector[24], sector[25], sector[26], sector[27], sector[28], sector[29],
            sector[30], sector[31],
        ]);

        // Get partition entry LBA (8 bytes at offset 72)
        self.header.partition_entry_lba = u64::from_le_bytes([
            sector[72], sector[73], sector[74], sector[75], sector[76], sector[77],
            sector[78], sector[79],
        ]);

        // Get partition count (4 bytes at offset 80)
        self.header.partition_entry_count = u32::from_le_bytes([
            sector[80],
            sector[81],
            sector[82],
            sector[83],
        ]);

        self.valid = self.header.verify_signature();

        Ok(())
    }

    /// Find bootable partition
    pub fn find_bootable_partition(&self) -> Option<&GPTPartitionEntry> {
        self.partitions
            .iter()
            .find_map(|p| p.as_ref().filter(|entry| entry.is_bootable()))
    }

    /// Get partition by index
    pub fn partition(&self, index: usize) -> Option<&GPTPartitionEntry> {
        if index < MAX_GPT_PARTITIONS {
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

    /// Check if GPT is valid
    pub fn is_valid(&self) -> bool {
        self.valid
            && self.header.verify_signature()
            && self.header.verify_revision()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpt_signature_constant() {
        assert_eq!(GPT_SIGNATURE, [0x45, 0x46, 0x49, 0x20, 0x50, 0x41, 0x52, 0x54]);
    }

    #[test]
    fn test_partition_type_guid_bootable() {
        assert!(PartitionTypeGUID::EFISystem.is_bootable());
        assert!(!PartitionTypeGUID::Unused.is_bootable());
    }

    #[test]
    fn test_partition_type_guid_description() {
        assert!(PartitionTypeGUID::LinuxFilesystem
            .description()
            .contains("Linux"));
    }

    #[test]
    fn test_gpt_partition_entry_creation() {
        let entry = GPTPartitionEntry::new();
        assert!(!entry.is_bootable());
    }

    #[test]
    fn test_gpt_partition_entry_size_bytes() {
        let mut entry = GPTPartitionEntry::new();
        entry.start_lba = 2048;
        entry.end_lba = 4095;

        assert_eq!(entry.size_sectors(), 2048);
        assert_eq!(entry.size_bytes(), 2048 * 512);
    }

    #[test]
    fn test_gpt_header_creation() {
        let header = GPTHeader::new();
        assert_eq!(header.signature, GPT_SIGNATURE);
        assert_eq!(header.revision, GPT_REVISION);
    }

    #[test]
    fn test_gpt_header_verify_signature() {
        let header = GPTHeader::new();
        assert!(header.verify_signature());
    }

    #[test]
    fn test_gpt_handler_creation() {
        let handler = GPTHandler::new();
        assert!(!handler.is_valid());
    }

    #[test]
    fn test_gpt_handler_partition_count() {
        let mut handler = GPTHandler::new();
        let entry = GPTPartitionEntry::new();

        handler.partitions[0] = Some(entry);
        assert_eq!(handler.partition_count(), 1);
    }

    #[test]
    fn test_gpt_handler_find_bootable() {
        let mut handler = GPTHandler::new();
        let mut entry = GPTPartitionEntry::new();
        entry.partition_type_guid = PartitionTypeGUID::EFISystem;
        entry.start_lba = 2048;
        entry.end_lba = 4095;

        handler.partitions[0] = Some(entry);
        assert!(handler.find_bootable_partition().is_some());
    }
}
