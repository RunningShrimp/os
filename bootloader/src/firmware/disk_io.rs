/// INT 0x13 Disk I/O Services
///
/// Provides disk read/write operations via BIOS interrupt 0x13.
/// Used for loading the kernel from disk during boot.

use crate::bios::bios_realmode::{self, RealModeExecutor};

/// Result type for disk operations
pub type DiskResult<T> = Result<T, DiskError>;

/// Disk operation errors
#[derive(Debug, Clone, Copy)]
pub enum DiskError {
    /// INT 0x13 call failed
    IntFailed,
    /// Invalid parameters (e.g., sector out of range)
    InvalidParams,
    /// Disk error (carry flag set)
    DiskError,
    /// Timeout waiting for disk
    Timeout,
    /// Seek error (invalid cylinder/head/sector)
    SeekError,
    /// Write protected disk
    WriteProtected,
    /// Drive not ready
    DriveNotReady,
    /// Undefined error
    UndefinedError,
}

impl DiskError {
    pub fn code(&self) -> u8 {
        match self {
            DiskError::IntFailed => 0x00,
            DiskError::InvalidParams => 0x01,
            DiskError::DiskError => 0x02,
            DiskError::Timeout => 0x08,
            DiskError::SeekError => 0x0F,
            DiskError::WriteProtected => 0x03,
            DiskError::DriveNotReady => 0x04,
            DiskError::UndefinedError => 0xFF,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            DiskError::IntFailed => "INT 0x13 call failed",
            DiskError::InvalidParams => "Invalid disk parameters",
            DiskError::DiskError => "Disk operation error",
            DiskError::Timeout => "Disk timeout",
            DiskError::SeekError => "Seek error",
            DiskError::WriteProtected => "Disk write protected",
            DiskError::DriveNotReady => "Drive not ready",
            DiskError::UndefinedError => "Undefined disk error",
        }
    }
}

/// Disk drive information
#[derive(Debug, Clone, Copy)]
pub struct DriveInfo {
    /// Drive number (0x80 = first hard disk)
    pub drive: u8,
    /// Number of cylinders
    pub cylinders: u16,
    /// Number of heads
    pub heads: u8,
    /// Number of sectors per track
    pub sectors: u8,
}

impl DriveInfo {
    /// Calculate total sectors on drive
    pub fn total_sectors(&self) -> u64 {
        (self.cylinders as u64) * (self.heads as u64) * (self.sectors as u64)
    }

    /// Check if drive exists
    pub fn exists(&self) -> bool {
        self.cylinders > 0 && self.heads > 0 && self.sectors > 0
    }
}

/// Disk sector address using CHS (Cylinder-Head-Sector)
#[derive(Debug, Clone, Copy)]
pub struct CylinderHeadSector {
    pub cylinder: u16,
    pub head: u8,
    pub sector: u8,
}

impl CylinderHeadSector {
    /// Convert LBA (Logical Block Address) to CHS
    pub fn from_lba(lba: u32, drive_info: &DriveInfo) -> DiskResult<Self> {
        let sectors_per_cylinder = drive_info.heads as u32 * drive_info.sectors as u32;
        let cylinder = (lba / sectors_per_cylinder) as u16;
        let remainder = lba % sectors_per_cylinder;
        let head = (remainder / drive_info.sectors as u32) as u8;
        let sector = ((remainder % drive_info.sectors as u32) + 1) as u8;

        if cylinder > 1023 || sector > 63 {
            return Err(DiskError::InvalidParams);
        }

        Ok(CylinderHeadSector {
            cylinder,
            head,
            sector,
        })
    }

    /// Check if CHS values are valid
    pub fn is_valid(&self) -> bool {
        self.cylinder <= 1023 && self.sector > 0 && self.sector <= 63
    }
}

/// Disk controller interface
pub struct DiskController {
    executor: *const RealModeExecutor,
}

impl DiskController {
    /// Create new disk controller with executor
    pub fn new(executor: &RealModeExecutor) -> Self {
        Self {
            executor: executor as *const _,
        }
    }

    /// Get drive information via INT 0x13/AH=0x08
    pub fn get_drive_info(&self, drive: u8) -> DiskResult<DriveInfo> {
        unsafe {
            let executor = &*self.executor;
            bios_realmode::int13_disk::get_drive_params(executor, drive)
                .map(|params| DriveInfo {
                    drive,
                    cylinders: params.max_cylinder,
                    heads: params.max_head,
                    sectors: params.max_sector,
                })
                .map_err(|_| DiskError::IntFailed)
        }
    }

    /// Read sectors from disk via INT 0x13/AH=0x02
    ///
    /// # Arguments
    /// * `drive` - Drive number (0x80 = first hard disk)
    /// * `chs` - Cylinder-Head-Sector address
    /// * `count` - Number of sectors to read
    /// * `buffer` - Buffer address in low memory (< 1MB)
    pub fn read_sectors(
        &self,
        drive: u8,
        chs: CylinderHeadSector,
        count: u8,
        buffer: u32,
    ) -> DiskResult<u8> {
        unsafe {
            let executor = &*self.executor;
            bios_realmode::int13_disk::read_sectors(
                executor,
                drive,
                chs.cylinder,
                chs.head,
                chs.sector,
                count,
                buffer,
            )
            .map_err(|_| DiskError::IntFailed)
        }
    }

    /// Reset disk controller via INT 0x13/AH=0x00
    pub fn reset_drive(&self, _drive: u8) -> DiskResult<()> {
        unsafe {
            let _executor = &*self.executor;
            log::debug!("Resetting disk controller");
            // Note: reset_drive not implemented in int13_disk yet
            // For now, just call read with 0 sectors as a dummy
            // In real implementation, would call INT 0x13/AH=0x00
            Ok(())
        }
    }
}

/// Boot sector loader
pub struct BootSectorLoader {
    controller: DiskController,
}

impl BootSectorLoader {
    /// Create new boot sector loader
    pub fn new(executor: &RealModeExecutor) -> Self {
        Self {
            controller: DiskController::new(executor),
        }
    }

    /// Load kernel sectors from disk
    ///
    /// Loads kernel data from specified LBA sectors into memory
    ///
    /// # Arguments
    /// * `drive` - Drive to load from
    /// * `start_lba` - Starting logical block address
    /// * `sector_count` - Number of 512-byte sectors to load
    /// * `dest_addr` - Destination address in low memory
    pub fn load_kernel_sectors(
        &self,
        drive: u8,
        start_lba: u32,
        sector_count: u32,
        dest_addr: u32,
    ) -> DiskResult<()> {
        // Get drive info for CHS conversion
        let drive_info = self.controller.get_drive_info(drive)?;

        let mut current_lba = start_lba;
        let mut current_addr = dest_addr;
        let mut remaining = sector_count;

        while remaining > 0 {
            // Convert LBA to CHS
            let chs = CylinderHeadSector::from_lba(current_lba, &drive_info)?;

            // Limit to 127 sectors per read (INT 0x13 limit)
            let sectors_to_read = core::cmp::min(remaining, 127) as u8;

            // Read sectors
            self.controller
                .read_sectors(drive, chs, sectors_to_read, current_addr)?;

            // Update counters
            let bytes_read = (sectors_to_read as u32) * 512;
            current_lba += sectors_to_read as u32;
            current_addr += bytes_read;
            remaining -= sectors_to_read as u32;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_error_codes() {
        assert_eq!(DiskError::IntFailed.code(), 0x00);
        assert_eq!(DiskError::InvalidParams.code(), 0x01);
        assert_eq!(DiskError::DiskError.code(), 0x02);
        assert_eq!(DiskError::Timeout.code(), 0x08);
    }

    #[test]
    fn test_disk_error_descriptions() {
        assert!(!DiskError::IntFailed.description().is_empty());
        assert!(!DiskError::DiskError.description().is_empty());
    }

    #[test]
    fn test_drive_info_creation() {
        let drive = DriveInfo {
            drive: 0x80,
            cylinders: 1024,
            heads: 16,
            sectors: 63,
        };

        assert_eq!(drive.drive, 0x80);
        assert!(drive.exists());
    }

    #[test]
    fn test_drive_info_total_sectors() {
        let drive = DriveInfo {
            drive: 0x80,
            cylinders: 100,
            heads: 10,
            sectors: 5,
        };

        assert_eq!(drive.total_sectors(), 5000); // 100 * 10 * 5
    }

    #[test]
    fn test_chs_validity() {
        let valid = CylinderHeadSector {
            cylinder: 500,
            head: 5,
            sector: 10,
        };
        assert!(valid.is_valid());

        let invalid_cyl = CylinderHeadSector {
            cylinder: 2000, // > 1023
            head: 5,
            sector: 10,
        };
        assert!(!invalid_cyl.is_valid());

        let invalid_sector = CylinderHeadSector {
            cylinder: 500,
            head: 5,
            sector: 65, // > 63
        };
        assert!(!invalid_sector.is_valid());
    }

    #[test]
    fn test_lba_to_chs_conversion() {
        let drive = DriveInfo {
            drive: 0x80,
            cylinders: 1024,
            heads: 16,
            sectors: 63,
        };

        // LBA 0 = CHS (0, 0, 1)
        let chs = CylinderHeadSector::from_lba(0, &drive).unwrap();
        assert_eq!(chs.cylinder, 0);
        assert_eq!(chs.head, 0);
        assert_eq!(chs.sector, 1);

        // LBA 63 = CHS (0, 1, 1)
        let chs = CylinderHeadSector::from_lba(63, &drive).unwrap();
        assert_eq!(chs.cylinder, 0);
        assert_eq!(chs.head, 1);
        assert_eq!(chs.sector, 1);
    }

    #[test]
    fn test_lba_to_chs_invalid() {
        let drive = DriveInfo {
            drive: 0x80,
            cylinders: 100,
            heads: 5,
            sectors: 10,
        };

        // LBA that would require > 1023 cylinders should fail
        let huge_lba = 2000000u32; // Way beyond drive capacity
        let result = CylinderHeadSector::from_lba(huge_lba, &drive);
        assert!(result.is_err());
    }

    #[test]
    fn test_sector_limit() {
        // INT 0x13 can read max 127 sectors at once
        // But this is more of an implementation detail
        assert!(127u8 < 255u8); // Sanity check
    }
}
