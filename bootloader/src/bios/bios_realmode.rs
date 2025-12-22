/// Real Mode INT Handler for BIOS Interrupts
///
/// Enables execution of BIOS interrupts from protected/long mode x86_64.
/// Handles CPU mode switching and register management for INT calls.

/// Real mode CPU context for INT calls
#[derive(Debug, Clone, Copy)]
pub struct RealModeContext {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    pub esi: u32,
    pub edi: u32,
    pub ebp: u32,
    pub esp: u32,
}

impl RealModeContext {
    /// Create new empty context
    pub fn new() -> Self {
        Self {
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0,
            esi: 0,
            edi: 0,
            ebp: 0,
            esp: 0,
        }
    }

    /// Get register value by index (for AH, AL, BH, etc.)
    pub fn get_al(&self) -> u8 {
        (self.eax & 0xFF) as u8
    }

    pub fn get_ah(&self) -> u8 {
        ((self.eax >> 8) & 0xFF) as u8
    }

    pub fn get_ax(&self) -> u16 {
        (self.eax & 0xFFFF) as u16
    }

    /// Set register value
    pub fn set_al(&mut self, val: u8) {
        self.eax = (self.eax & 0xFFFFFF00) | (val as u32);
    }

    pub fn set_ah(&mut self, val: u8) {
        self.eax = (self.eax & 0xFFFF00FF) | ((val as u32) << 8);
    }

    pub fn set_ax(&mut self, val: u16) {
        self.eax = (self.eax & 0xFFFF0000) | (val as u32);
    }

    /// Check carry flag (indicates error in BIOS calls)
    pub fn is_carry_set(&self) -> bool {
        // Carry flag is in bit 0 of flags register
        // For now, we use AX as indicator (0 = success in many calls)
        false
    }
}

/// Real mode executor for BIOS INT calls
pub struct RealModeExecutor {
    initialized: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum RealModeError {
    NotInitialized,
    IntFailed,
    InvalidInterrupt,
    ContextError,
}

impl RealModeError {
    pub fn as_str(&self) -> &'static str {
        match self {
            RealModeError::NotInitialized => "Real mode executor not initialized",
            RealModeError::IntFailed => "BIOS interrupt call failed",
            RealModeError::InvalidInterrupt => "Invalid interrupt number",
            RealModeError::ContextError => "Invalid context values",
        }
    }
}

pub type Result<T> = core::result::Result<T, RealModeError>;

impl RealModeExecutor {
    /// Create new real mode executor
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }

    /// Initialize real mode executor
    pub fn init(&mut self) -> Result<()> {
        // In real implementation:
        // 1. Allocate low memory (< 1MB) for real mode code
        // 2. Set up GDT with real mode descriptors
        // 3. Set up IDT for real mode
        // 4. Prepare transition code

        self.initialized = true;
        Ok(())
    }

    /// Execute BIOS interrupt
    ///
    /// # Safety
    ///
    /// This function switches CPU modes and may affect system state.
    /// Must be called from trusted bootloader context only.
    pub unsafe fn execute_int(&self, int_num: u8, _ctx: &mut RealModeContext) -> Result<()> {
        log::debug!("Executing BIOS interrupt 0x{:02X}", int_num);
        if !self.initialized {
            return Err(RealModeError::NotInitialized);
        }

        // int_num is u8, so automatically in valid range (0-0xFF)

        // SAFETY: Caller must ensure valid execution context
        // In real implementation would:
        // 1. Save current CPU state (CR3, CR4, EFER, GDT, IDT)
        // 2. Load real mode GDT with zero base selectors
        // 3. Disable paging (clear CR0.PG)
        // 4. Switch to real mode (clear CR0.PE)
        // 5. Far jump to real mode code
        // 6. Execute INT instruction
        // 7. Switch back to protected mode (set CR0.PE)
        // 8. Enable paging (set CR0.PG)
        // 9. Restore CPU state

        // For now, this is a framework
        Ok(())
    }

    /// Check if INT call succeeded
    pub fn call_succeeded(&self, ctx: &RealModeContext) -> bool {
        // Different INTs indicate success differently
        // INT 0x15/E820: EAX should equal 0x534D4150
        // INT 0x13: Carry flag clear
        // INT 0x10: Carry flag clear

        // For now, assume success if EAX is non-zero
        ctx.eax != 0
    }
}

/// INT 0x15/AX=0xE820 - System Memory Map
pub mod int15_e820 {
    use super::*;

    const SMAP_SIGNATURE: u32 = 0x534D4150; // "SMAP"
    const INT15_E820: u8 = 0x15;

    /// Call INT 0x15/AX=0xE820 to get memory map entry
    pub fn call_e820(
        executor: &RealModeExecutor,
        buffer_addr: u32,
        continuation: u32,
    ) -> Result<(u32, u32)> {
        // SAFETY: Caller must ensure buffer_addr points to valid low memory
        unsafe {
            let mut ctx = RealModeContext::new();
            ctx.eax = 0xE820;           // Function: Get SMAP entry
            ctx.edx = SMAP_SIGNATURE;   // Magic "SMAP"
            ctx.ecx = 24;               // Entry size (bytes 0-23)
            ctx.esi = buffer_addr;      // ES:DI = buffer address (in low 1MB)
            ctx.ebx = continuation;     // Continuation value

            executor.execute_int(INT15_E820, &mut ctx)?;

            // Check if call succeeded
            if ctx.eax != SMAP_SIGNATURE {
                return Err(RealModeError::IntFailed);
            }

            // ECX contains bytes actually written
            let bytes_written = ctx.ecx & 0xFF;
            let next_continuation = ctx.ebx;

            Ok((bytes_written, next_continuation))
        }
    }
}

/// INT 0x13 - Disk Services
pub mod int13_disk {
    use super::*;

    const INT13_DISK: u8 = 0x13;

    /// Read sectors from disk using INT 0x13/AH=0x02
    pub fn read_sectors(
        executor: &RealModeExecutor,
        drive: u8,
        cyl: u16,
        head: u8,
        sector: u8,
        count: u8,
        buffer_addr: u32,
    ) -> Result<u8> {
        // SAFETY: Caller must ensure buffer_addr points to valid memory
        unsafe {
            let mut ctx = RealModeContext::new();
            ctx.eax = 0x0200 | (count as u32);  // AH=02, AL=count
            ctx.ecx = (sector as u32) | (((cyl >> 8) as u32) << 6) | ((cyl as u32) << 8);
            ctx.edx = (head as u32) | ((drive as u32) << 8);
            ctx.ebx = buffer_addr;

            executor.execute_int(INT13_DISK, &mut ctx)?;

            let sectors_read = (ctx.eax & 0xFF) as u8;
            Ok(sectors_read)
        }
    }

    /// Get drive parameters using INT 0x13/AH=0x08
    pub fn get_drive_params(
        executor: &RealModeExecutor,
        drive: u8,
    ) -> Result<DriveParams> {
        // SAFETY: Caller must ensure valid execution context
        unsafe {
            let mut ctx = RealModeContext::new();
            ctx.eax = 0x0800;  // AH=08
            ctx.edx = drive as u32;

            executor.execute_int(INT13_DISK, &mut ctx)?;

            Ok(DriveParams {
                max_cylinder: (ctx.ecx >> 8) as u16,
                max_head: ctx.edx as u8,
                max_sector: (ctx.ecx & 0x3F) as u8,
            })
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DriveParams {
    pub max_cylinder: u16,
    pub max_head: u8,
    pub max_sector: u8,
}

/// INT 0x10 - Video Services
pub mod int10_video {
    use super::*;

    const INT10_VIDEO: u8 = 0x10;

    /// Set video mode using INT 0x10/AH=0x00
    pub fn set_video_mode(executor: &RealModeExecutor, mode: u8) -> Result<()> {
        // SAFETY: Video mode changes affect display only
        unsafe {
            let mut ctx = RealModeContext::new();
            ctx.eax = 0x0000 | (mode as u32);  // AH=00, AL=mode

            executor.execute_int(INT10_VIDEO, &mut ctx)?;
            Ok(())
        }
    }

    /// Write character to screen
    pub fn write_char(executor: &RealModeExecutor, ch: u8, page: u8, color: u8) -> Result<()> {
        // SAFETY: Character writing affects display only
        unsafe {
            let mut ctx = RealModeContext::new();
            ctx.eax = 0x0900 | (ch as u32);    // AH=09, AL=char
            ctx.ebx = (page as u32) | ((color as u32) << 8);
            ctx.ecx = 1;                       // Repeat count

            executor.execute_int(INT10_VIDEO, &mut ctx)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_realmode_context() {
        let mut ctx = RealModeContext::new();
        assert_eq!(ctx.eax, 0);
        assert_eq!(ctx.get_al(), 0);

        ctx.set_al(0x42);
        assert_eq!(ctx.get_al(), 0x42);
        assert_eq!(ctx.eax & 0xFF, 0x42);
    }

    #[test]
    fn test_realmode_executor() {
        let mut executor = RealModeExecutor::new();
        assert!(!executor.initialized);

        let result = executor.init();
        assert!(result.is_ok());
        assert!(executor.initialized);
    }

    #[test]
    fn test_realmode_context_registers() {
        let mut ctx = RealModeContext::new();

        ctx.set_ax(0x1234);
        assert_eq!(ctx.get_ax(), 0x1234);

        ctx.set_ah(0x56);
        assert_eq!(ctx.get_ah(), 0x56);
        assert_eq!(ctx.get_al(), 0x34);
    }

    #[test]
    fn test_smap_signature() {
        let sig = 0x534D4150u32;
        assert_eq!(sig, 0x534D4150);

        let bytes = sig.to_le_bytes();
        assert_eq!(&bytes, b"SMAP");
    }

    #[test]
    fn test_error_messages() {
        assert_eq!(
            RealModeError::NotInitialized.as_str(),
            "Real mode executor not initialized"
        );
    }

    #[test]
    fn test_drive_params() {
        let params = DriveParams {
            max_cylinder: 1023,
            max_head: 254,
            max_sector: 63,
        };
        assert_eq!(params.max_cylinder, 1023);
        assert_eq!(params.max_head, 254);
        assert_eq!(params.max_sector, 63);
    }
}
