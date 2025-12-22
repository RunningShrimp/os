/// Boot Loader Integration
///
/// Integrates MBR, GPT, and disk I/O for complete bootable system.

use alloc::vec::Vec;
use crate::firmware::mbr_handler::MasterBootRecord;
use crate::firmware::gpt_handler::GPTHandler;
use crate::firmware::disk_reader::{DiskReader, DiskResult, DiskError};

/// Boot media type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootMediaType {
    MBR,
    GPT,
    Unknown,
}

impl BootMediaType {
    pub fn description(&self) -> &'static str {
        match self {
            Self::MBR => "Master Boot Record",
            Self::GPT => "GUID Partition Table",
            Self::Unknown => "Unknown",
        }
    }
}

/// Boot device information
#[derive(Debug, Clone, Copy)]
pub struct BootDevice {
    pub drive_number: u8,
    pub media_type: BootMediaType,
    pub media_valid: bool,
}

impl BootDevice {
    pub fn new(drive_number: u8) -> Self {
        Self {
            drive_number,
            media_type: BootMediaType::Unknown,
            media_valid: false,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.media_valid && self.drive_number != 0
    }
}

/// Boot loader for actual disk-based boot
pub struct BootableSystemLoader {
    device: BootDevice,
    disk_reader: Option<DiskReader>,
    mbr: Option<MasterBootRecord>,
    gpt: Option<GPTHandler>,
    kernel_lba: Option<u32>,
    kernel_sectors: Option<u16>,
}

impl BootableSystemLoader {
    /// Create new bootable system loader
    pub fn new(drive_number: u8) -> Self {
        Self {
            device: BootDevice::new(drive_number),
            disk_reader: Some(DiskReader::new(drive_number)),
            mbr: None,
            gpt: None,
            kernel_lba: None,
            kernel_sectors: None,
        }
    }

    /// Initialize and detect boot media
    pub fn init_boot_media(&mut self) -> DiskResult<()> {
        let reader = self.disk_reader.as_mut().ok_or(DiskError::ReadFailed)?;

        // Read sector 0 (MBR)
        let sector = reader.read_sector(0)?;

        // Try to parse as MBR
        let mut mbr = MasterBootRecord::new();
        if mbr.parse(&sector).is_ok() && mbr.is_valid() {
            self.device.media_type = BootMediaType::MBR;
            self.device.media_valid = true;
            self.mbr = Some(mbr);
            return Ok(());
        }

        // Try to parse as GPT (sector 1)
        let sector_gpt = reader.read_sector(1)?;
        let mut gpt = GPTHandler::new();
        if gpt.parse_header(&sector_gpt).is_ok() && gpt.is_valid() {
            self.device.media_type = BootMediaType::GPT;
            self.device.media_valid = true;
            self.gpt = Some(gpt);
            return Ok(());
        }

        Err(DiskError::ReadFailed)
    }

    /// Find bootable kernel partition
    pub fn find_kernel_partition(&mut self) -> DiskResult<(u32, u16)> {
        match self.device.media_type {
            BootMediaType::MBR => {
                let mbr = self.mbr.as_ref().ok_or(DiskError::ReadFailed)?;
                let partition = mbr
                    .find_bootable_partition()
                    .ok_or(DiskError::SectorNotFound)?;

                self.kernel_lba = Some(partition.start_lba);
                self.kernel_sectors = Some(1); // Typically kernel loader is 1 sector

                Ok((partition.start_lba, 1))
            }
            BootMediaType::GPT => {
                let gpt = self.gpt.as_ref().ok_or(DiskError::ReadFailed)?;
                let partition = gpt
                    .find_bootable_partition()
                    .ok_or(DiskError::SectorNotFound)?;

                self.kernel_lba = Some(partition.start_lba as u32);
                self.kernel_sectors = Some(1);

                Ok((partition.start_lba as u32, 1))
            }
            BootMediaType::Unknown => Err(DiskError::ReadFailed),
        }
    }

    /// Load kernel from disk
    pub fn load_kernel_from_disk(&mut self, kernel_lba: u32, sectors: u16) -> DiskResult<Vec<u8>> {
        let reader = self.disk_reader.as_mut().ok_or(DiskError::ReadFailed)?;
        reader.read_sectors(kernel_lba, sectors)
    }

    /// Get boot media type
    pub fn media_type(&self) -> BootMediaType {
        self.device.media_type
    }

    /// Check if boot media is valid
    pub fn is_media_valid(&self) -> bool {
        self.device.is_valid()
    }

    /// Get MBR reference
    pub fn mbr(&self) -> Option<&MasterBootRecord> {
        self.mbr.as_ref()
    }

    /// Get GPT reference
    pub fn gpt(&self) -> Option<&GPTHandler> {
        self.gpt.as_ref()
    }

    /// Get kernel location
    pub fn kernel_location(&self) -> Option<(u32, u16)> {
        match (self.kernel_lba, self.kernel_sectors) {
            (Some(lba), Some(sectors)) => Some((lba, sectors)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_media_type_description() {
        assert!(BootMediaType::MBR.description().contains("Master"));
        assert!(BootMediaType::GPT.description().contains("GUID"));
    }

    #[test]
    fn test_boot_device_creation() {
        let device = BootDevice::new(0x80);
        assert_eq!(device.drive_number, 0x80);
        assert!(!device.is_valid());
    }

    #[test]
    fn test_boot_device_valid() {
        let mut device = BootDevice::new(0x80);
        device.media_valid = true;

        assert!(device.is_valid());
    }

    #[test]
    fn test_bootable_system_loader_creation() {
        let loader = BootableSystemLoader::new(0x80);
        assert_eq!(loader.device.drive_number, 0x80);
        assert!(!loader.is_media_valid());
    }

    #[test]
    fn test_bootable_system_loader_media_type() {
        let loader = BootableSystemLoader::new(0x80);
        assert_eq!(loader.media_type(), BootMediaType::Unknown);
    }

    #[test]
    fn test_bootable_system_loader_disk_reader() {
        let loader = BootableSystemLoader::new(0x80);
        assert!(loader.disk_reader.is_some());
    }
}
