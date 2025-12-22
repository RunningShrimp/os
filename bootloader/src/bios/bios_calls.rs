/// BIOS Call Wrappers and Interrupt Stubs
///
/// High-level wrappers for common BIOS interrupt calls.
/// Currently provides framework; actual execution happens after real mode switch.

use core::fmt;
use crate::bios::bios_realmode::RealModeExecutor;
// RealModeContext在当前文件中未使用，暂时注释掉
// use crate::bios::bios_realmode::RealModeContext;

/// BIOS call error types
#[derive(Debug, Clone)]
pub enum BIOSCallError {
    NotInitialized,
    InvalidParameters,
    InterruptFailed,
    TimeoutOccurred,
    UnsupportedFunction,
    DeviceError,
}

impl BIOSCallError {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotInitialized => "BIOS not initialized",
            Self::InvalidParameters => "Invalid BIOS parameters",
            Self::InterruptFailed => "BIOS interrupt call failed",
            Self::TimeoutOccurred => "BIOS call timeout",
            Self::UnsupportedFunction => "Unsupported BIOS function",
            Self::DeviceError => "Device error from BIOS",
        }
    }
}

impl fmt::Display for BIOSCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub type BIOSResult<T> = Result<T, BIOSCallError>;

/// INT 0x10 - Video Services
pub struct VideoServices {
    initialized: bool,
}

impl VideoServices {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    pub fn init(&mut self) -> BIOSResult<()> {
        self.initialized = true;
        Ok(())
    }

    /// Set video mode (AH=00)
    pub fn set_video_mode(&self, _mode: u8) -> BIOSResult<()> {
        if !self.initialized {
            return Err(BIOSCallError::NotInitialized);
        }
        // Framework: actual INT 0x10 call would go here
        Ok(())
    }

    /// Get cursor position (AH=03)
    pub fn get_cursor_position(&self) -> BIOSResult<(u8, u8, u8, u8)> {
        if !self.initialized {
            return Err(BIOSCallError::NotInitialized);
        }
        // Returns (page, row, col, unknown)
        Ok((0, 0, 0, 0))
    }

    /// Set cursor position (AH=02)
    pub fn set_cursor_position(&self, _page: u8, _row: u8, _col: u8) -> BIOSResult<()> {
        if !self.initialized {
            return Err(BIOSCallError::NotInitialized);
        }
        Ok(())
    }

    /// Print character (AH=0E)
    pub fn print_char(&self, _char: u8) -> BIOSResult<()> {
        if !self.initialized {
            return Err(BIOSCallError::NotInitialized);
        }
        Ok(())
    }
}

/// INT 0x13 - Disk Services  
pub struct DiskServices {
    initialized: bool,
}

impl DiskServices {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    pub fn init(&mut self) -> BIOSResult<()> {
        self.initialized = true;
        Ok(())
    }

    /// Reset disk drive (AH=00)
    pub fn reset_drive(&self, _drive: u8) -> BIOSResult<()> {
        if !self.initialized {
            return Err(BIOSCallError::NotInitialized);
        }
        Ok(())
    }

    /// Read sectors (AH=02)
    pub fn read_sectors(
        &self,
        _drive: u8,
        _cylinder: u16,
        _head: u8,
        _sector: u8,
        _count: u8,
    ) -> BIOSResult<()> {
        if !self.initialized {
            return Err(BIOSCallError::NotInitialized);
        }
        Ok(())
    }

    /// Get drive parameters (AH=08)
    pub fn get_drive_params(&self, _drive: u8) -> BIOSResult<(u16, u8, u8)> {
        if !self.initialized {
            return Err(BIOSCallError::NotInitialized);
        }
        // Returns (cylinders, heads, sectors_per_track)
        Ok((0, 0, 0))
    }

    /// Get disk status (AH=01)
    pub fn get_status(&self, _drive: u8) -> BIOSResult<u8> {
        if !self.initialized {
            return Err(BIOSCallError::NotInitialized);
        }
        Ok(0)  // No error
    }
}

/// INT 0x15 - System Services
pub struct SystemServices {
    initialized: bool,
}

impl SystemServices {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    pub fn init(&mut self) -> BIOSResult<()> {
        self.initialized = true;
        Ok(())
    }

    /// Get extended memory size (AH=88)
    pub fn get_extended_memory(&self) -> BIOSResult<u16> {
        if !self.initialized {
            return Err(BIOSCallError::NotInitialized);
        }
        Ok(0)  // In KB
    }

    /// E820 memory map (AX=E820, EDX=534D4150)
    pub fn get_memory_map(&self) -> BIOSResult<()> {
        if !self.initialized {
            return Err(BIOSCallError::NotInitialized);
        }
        Ok(())
    }

    /// Get system time (AH=00)
    pub fn get_system_time(&self) -> BIOSResult<u32> {
        if !self.initialized {
            return Err(BIOSCallError::NotInitialized);
        }
        Ok(0)  // 1/18 second ticks since midnight
    }

    /// Wait for key press (AH=00)
    pub fn wait_key(&self) -> BIOSResult<u16> {
        if !self.initialized {
            return Err(BIOSCallError::NotInitialized);
        }
        Ok(0)  // Key code
    }
}

/// INT 0x1A - Real Time Clock Services
pub struct RTCServices {
    initialized: bool,
}

impl RTCServices {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    pub fn init(&mut self) -> BIOSResult<()> {
        self.initialized = true;
        Ok(())
    }

    /// Get current date (AH=04)
    pub fn get_date(&self) -> BIOSResult<(u8, u8, u16)> {
        if !self.initialized {
            return Err(BIOSCallError::NotInitialized);
        }
        Ok((0, 0, 0))  // (day, month, year)
    }

    /// Get current time (AH=02)
    pub fn get_time(&self) -> BIOSResult<(u8, u8, u8)> {
        if !self.initialized {
            return Err(BIOSCallError::NotInitialized);
        }
        Ok((0, 0, 0))  // (hours, minutes, seconds)
    }
}

/// BIOS Services Manager
pub struct BIOSServices {
    video: VideoServices,
    disk: DiskServices,
    system: SystemServices,
    rtc: RTCServices,
    executor: RealModeExecutor,
    initialized: bool,
}

impl BIOSServices {
    pub fn new() -> Self {
        Self {
            video: VideoServices::new(),
            disk: DiskServices::new(),
            system: SystemServices::new(),
            rtc: RTCServices::new(),
            executor: RealModeExecutor::new(),
            initialized: false,
        }
    }

    /// Initialize all BIOS services
    pub fn init(&mut self) -> BIOSResult<()> {
        // Initialize real mode executor first
        let init_result = self.executor.init();
        if init_result.is_err() {
            return Err(BIOSCallError::NotInitialized);
        }
        
        self.video.init()?;
        self.disk.init()?;
        self.system.init()?;
        self.rtc.init()?;
        self.initialized = true;
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn video(&self) -> &VideoServices {
        &self.video
    }

    pub fn disk(&self) -> &DiskServices {
        &self.disk
    }

    pub fn system(&self) -> &SystemServices {
        &self.system
    }

    pub fn rtc(&self) -> &RTCServices {
        &self.rtc
    }

    pub fn video_mut(&mut self) -> &mut VideoServices {
        &mut self.video
    }

    pub fn disk_mut(&mut self) -> &mut DiskServices {
        &mut self.disk
    }

    pub fn system_mut(&mut self) -> &mut SystemServices {
        &mut self.system
    }

    pub fn rtc_mut(&mut self) -> &mut RTCServices {
        &mut self.rtc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_services_creation() {
        let video = VideoServices::new();
        assert!(!video.initialized);
    }

    #[test]
    fn test_disk_services_creation() {
        let disk = DiskServices::new();
        assert!(!disk.initialized);
    }

    #[test]
    fn test_system_services_creation() {
        let system = SystemServices::new();
        assert!(!system.initialized);
    }

    #[test]
    fn test_rtc_services_creation() {
        let rtc = RTCServices::new();
        assert!(!rtc.initialized);
    }

    #[test]
    fn test_bios_services_manager() {
        let mut bios = BIOSServices::new();
        assert!(!bios.is_initialized());

        assert!(bios.init().is_ok());
        assert!(bios.is_initialized());

        assert!(bios.video().initialized);
        assert!(bios.disk().initialized);
        assert!(bios.system().initialized);
        assert!(bios.rtc().initialized);
    }

    #[test]
    fn test_uninitialized_calls_fail() {
        let video = VideoServices::new();
        let result = video.set_video_mode(0x03);
        assert!(result.is_err());
    }

    #[test]
    fn test_initialized_calls_work() {
        let mut video = VideoServices::new();
        assert!(video.init().is_ok());
        let result = video.set_video_mode(0x03);
        assert!(result.is_ok());
    }
}
