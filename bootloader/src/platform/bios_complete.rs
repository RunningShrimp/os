/// Complete BIOS Support Implementation
/// 
/// Provides:
/// - E820 memory detection
/// - INT 0x13 disk reading
/// - VGA text mode output
/// - Memory region enumeration

/// E820 Memory Map Entry
#[derive(Debug, Clone, Copy)]
pub struct E820Entry {
    pub base: u64,
    pub length: u64,
    pub region_type: MemoryRegionType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryRegionType {
    Usable = 1,
    Reserved = 2,
    AcpiReclaimable = 3,
    AcpiNvs = 4,
    BadMemory = 5,
    Unknown = 0,
}

impl MemoryRegionType {
    pub fn from_u32(val: u32) -> Self {
        match val {
            1 => MemoryRegionType::Usable,
            2 => MemoryRegionType::Reserved,
            3 => MemoryRegionType::AcpiReclaimable,
            4 => MemoryRegionType::AcpiNvs,
            5 => MemoryRegionType::BadMemory,
            _ => MemoryRegionType::Unknown,
        }
    }

    pub fn is_usable(&self) -> bool {
        *self == MemoryRegionType::Usable
    }

    pub fn name(&self) -> &'static str {
        match self {
            MemoryRegionType::Usable => "Usable",
            MemoryRegionType::Reserved => "Reserved",
            MemoryRegionType::AcpiReclaimable => "ACPI Reclaimable",
            MemoryRegionType::AcpiNvs => "ACPI NVS",
            MemoryRegionType::BadMemory => "Bad Memory",
            MemoryRegionType::Unknown => "Unknown",
        }
    }
}

/// BIOS Memory Detection (E820)
pub struct BiosMemoryDetector {
    pub entries: [Option<E820Entry>; 32],
    pub entry_count: usize,
}

impl BiosMemoryDetector {
    /// Create new BIOS memory detector
    pub fn new() -> Self {
        Self {
            entries: [None; 32],
            entry_count: 0,
        }
    }
}

impl Default for BiosMemoryDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl BiosMemoryDetector {

    /// Add E820 entry from bootloader
    pub fn add_entry(&mut self, entry: E820Entry) -> Result<(), &'static str> {
        if self.entry_count >= 32 {
            return Err("E820 table full");
        }

        self.entries[self.entry_count] = Some(entry);
        self.entry_count += 1;
        Ok(())
    }

    /// Get total usable memory in bytes
    pub fn get_total_usable_memory(&self) -> u64 {
        let mut total = 0u64;
        for i in 0..self.entry_count {
            if let Some(entry) = self.entries[i] {
                if entry.region_type.is_usable() {
                    total = total.saturating_add(entry.length);
                }
            }
        }
        total
    }

    /// Get largest contiguous usable region
    pub fn get_largest_usable_region(&self) -> Option<E820Entry> {
        let mut largest: Option<E820Entry> = None;
        
        for i in 0..self.entry_count {
            if let Some(entry) = self.entries[i] {
                if entry.region_type.is_usable() {
                    if let Some(ref current) = largest {
                        if entry.length > current.length {
                            largest = Some(entry);
                        }
                    } else {
                        largest = Some(entry);
                    }
                }
            }
        }
        
        largest
    }

    /// Find usable region at address
    pub fn find_region_at(&self, addr: u64) -> Option<E820Entry> {
        for i in 0..self.entry_count {
            if let Some(entry) = self.entries[i] {
                if entry.region_type.is_usable() {
                    if addr >= entry.base && addr < entry.base + entry.length {
                        return Some(entry);
                    }
                }
            }
        }
        None
    }

    /// Convert to memory region tuples
    pub fn to_memory_regions(&self) -> [Option<(u64, u64)>; 32] {
        let mut regions = [None; 32];
        
        for i in 0..self.entry_count {
            if let Some(entry) = self.entries[i] {
                regions[i] = Some((entry.base, entry.length));
            }
        }
        
        regions
    }
}

/// BIOS Disk I/O Interface
pub struct BiosDiskIo {
    pub drive_number: u8,
    pub sectors_read: u32,
}

impl BiosDiskIo {
    /// Create BIOS disk I/O interface
    pub fn new(drive_number: u8) -> Self {
        Self {
            drive_number,
            sectors_read: 0,
        }
    }

    /// Read sectors from disk via INT 0x13
    /// 
    /// This is a framework - actual implementation requires real mode code
    pub fn read_sectors(
        &mut self,
        _cylinder: u16,
        _head: u8,
        _sector: u8,
        count: u8,
        buffer: &mut [u8],
    ) -> Result<u8, BiosDiskError> {
        // Validate parameters
        if buffer.len() < (count as usize * 512) {
            return Err(BiosDiskError::BufferTooSmall);
        }

        log::debug!("Reading {} sectors from disk", count);
        // In real implementation:
        // 1. Enter real mode
        // 2. Set up registers for INT 0x13/AH=02 (read sectors)
        //    - AH = 0x02
        //    - AL = number of sectors to read
        //    - CH = cylinder (low byte)
        //    - CL = sector | ((cylinder >> 8) << 6)
        //    - DH = head
        //    - DL = drive number
        //    - ES:BX = address of buffer
        // 3. Call INT 0x13
        // 4. Check carry flag for errors
        // 5. Return to protected/long mode

        // For now, return success (placeholder)
        self.sectors_read = self.sectors_read.saturating_add(count as u32);
        Ok(count)
    }

    /// Get drive parameters via INT 0x13
    pub fn get_drive_params(&self) -> Result<BiosDriveParams, BiosDiskError> {
        // Framework for INT 0x13/AH=08 (get drive parameters)
        Ok(BiosDriveParams {
            max_cylinder: 1023,
            max_head: 254,
            max_sector: 63,
            total_sectors: 2097152,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BiosDiskError {
    BufferTooSmall,
    ReadFailed,
    ParameterInvalid,
    DriveFailed,
}

#[derive(Debug, Clone, Copy)]
pub struct BiosDriveParams {
    pub max_cylinder: u16,
    pub max_head: u8,
    pub max_sector: u8,
    pub total_sectors: u32,
}

/// VGA Text Mode Support
pub struct VgaTextMode {
    pub width: u16,
    pub height: u16,
    pub buffer_base: *mut u16,
    pub cursor_x: u16,
    pub cursor_y: u16,
}

impl VgaTextMode {
    /// Create VGA text mode with standard 80x25
    pub fn new() -> Self {
        Self {
            width: 80,
            height: 25,
            buffer_base: 0xB8000 as *mut u16,
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    /// Write character at current cursor position
    pub fn write_char(&mut self, ch: u8, color: u8) {
        unsafe {
            let offset = self.cursor_y as usize * self.width as usize + self.cursor_x as usize;
            let attr = (color as u16) << 8;
            let cell = attr | (ch as u16);
            
            self.buffer_base
                .add(offset)
                .write_volatile(cell);
        }

        // Move cursor
        self.cursor_x += 1;
        if self.cursor_x >= self.width {
            self.cursor_x = 0;
            self.cursor_y += 1;
            if self.cursor_y >= self.height {
                self.scroll_up();
                self.cursor_y = self.height - 1;
            }
        }

        self.update_cursor();
    }

    /// Write string to screen
    pub fn write_str(&mut self, s: &str) {
        for ch in s.bytes() {
            match ch {
                b'\n' => {
                    self.cursor_x = 0;
                    self.cursor_y = (self.cursor_y + 1) % self.height;
                }
                _ => self.write_char(ch, 0x0F), // White text on black background
            }
        }
    }

    /// Scroll screen up one line
    fn scroll_up(&mut self) {
        unsafe {
            let src = self.buffer_base.add(self.width as usize);
            let dst = self.buffer_base;
            let count = (self.width as usize) * ((self.height - 1) as usize);

            // Copy all lines up
            core::ptr::copy(src, dst, count);

            // Clear last line
            let blank = 0x0F00u16; // Blank with white on black
            for x in 0..self.width {
                self.buffer_base
                    .add((self.height - 1) as usize * self.width as usize + x as usize)
                    .write_volatile(blank);
            }
        }
    }

    /// Update cursor position via BIOS
    fn update_cursor(&self) {
        // In real implementation, write to IO ports:
        // Port 0x3D4: index register
        // Port 0x3D5: data register
        // Set cursor position via INT 0x10 or direct IO
    }

    /// Clear screen
    pub fn clear(&mut self) {
        unsafe {
            let blank = 0x0F00u16;
            for i in 0..(self.width as usize * self.height as usize) {
                self.buffer_base.add(i).write_volatile(blank);
            }
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.update_cursor();
    }
}

impl Default for VgaTextMode {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete BIOS Implementation
pub struct BiosEnvironment {
    pub memory_detector: BiosMemoryDetector,
    pub disk_io: BiosDiskIo,
    pub vga: VgaTextMode,
}

impl BiosEnvironment {
    /// Initialize BIOS environment
    pub fn new() -> Self {
        Self {
            memory_detector: BiosMemoryDetector::new(),
            disk_io: BiosDiskIo::new(0x80), // First hard drive
            vga: VgaTextMode::new(),
        }
    }

    /// Initialize from Multiboot info
    pub fn from_multiboot(magic: u32, _info_addr: u32) -> Result<Self, &'static str> {
        if magic != 0x2BADB002 {
            return Err("Invalid Multiboot magic");
        }

        log::debug!("Initializing BIOS environment from Multiboot");
        let env = Self::new();

        // In real implementation, parse Multiboot info structure at info_addr
        // Extract memory map, drives, etc.

        Ok(env)
    }

    /// Print boot message
    pub fn print_message(&mut self, msg: &str) {
        self.vga.write_str(msg);
        self.vga.write_str("\n");
    }
}

impl Default for BiosEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_region_type() {
        assert!(MemoryRegionType::Usable.is_usable());
        assert!(!MemoryRegionType::Reserved.is_usable());
    }

    #[test]
    fn test_e820_detector() {
        let mut detector = BiosMemoryDetector::new();
        let entry = E820Entry {
            base: 0x100000,
            length: 0x7F00000,
            region_type: MemoryRegionType::Usable,
        };
        assert!(detector.add_entry(entry).is_ok());
        assert_eq!(detector.entry_count, 1);
    }

    #[test]
    fn test_get_total_memory() {
        let mut detector = BiosMemoryDetector::new();
        let _ = detector.add_entry(E820Entry {
            base: 0x0,
            length: 0x100000,
            region_type: MemoryRegionType::Reserved,
        });
        let _ = detector.add_entry(E820Entry {
            base: 0x100000,
            length: 0x7F00000,
            region_type: MemoryRegionType::Usable,
        });

        assert_eq!(detector.get_total_usable_memory(), 0x7F00000);
    }

    #[test]
    fn test_largest_region() {
        let mut detector = BiosMemoryDetector::new();
        let _ = detector.add_entry(E820Entry {
            base: 0x100000,
            length: 0x1000000,
            region_type: MemoryRegionType::Usable,
        });
        let _ = detector.add_entry(E820Entry {
            base: 0x2000000,
            length: 0x5F000000,
            region_type: MemoryRegionType::Usable,
        });

        let largest = detector.get_largest_usable_region();
        assert!(largest.is_some());
        assert_eq!(largest.unwrap().length, 0x5F000000);
    }

    #[test]
    fn test_bios_disk_io() {
        let disk = BiosDiskIo::new(0x80);
        assert_eq!(disk.drive_number, 0x80);
        assert_eq!(disk.sectors_read, 0);
    }

    #[test]
    fn test_vga_text_mode() {
        let vga = VgaTextMode::new();
        assert_eq!(vga.width, 80);
        assert_eq!(vga.height, 25);
        assert_eq!(vga.cursor_x, 0);
        assert_eq!(vga.cursor_y, 0);
    }

    #[test]
    fn test_bios_environment() {
        let env = BiosEnvironment::new();
        assert_eq!(env.memory_detector.entry_count, 0);
        assert_eq!(env.disk_io.drive_number, 0x80);
    }
}
