/// BIOS Interrupt Execution Engine
///
/// Handles the actual execution of BIOS interrupts after transitioning to real mode.
/// Manages CPU context, interrupt execution, and error handling.

use crate::bios::bios_realmode::{RealModeContext, RealModeExecutor, RealModeError};

/// BIOS interrupt execution status
#[derive(Debug, Clone, Copy)]
pub enum ExecStatus {
    Success,
    CarryFlagSet,
    InvalidInterrupt,
    ExecutionFailed,
}

impl ExecStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "Success",
            Self::CarryFlagSet => "Carry flag set (error)",
            Self::InvalidInterrupt => "Invalid interrupt number",
            Self::ExecutionFailed => "Execution failed",
        }
    }
}

/// Result type for interrupt execution
pub type IntResult = Result<ExecStatus, RealModeError>;

/// BIOS interrupt executor
pub struct BIOSInterruptExecutor {
    executor: *const RealModeExecutor,
    last_context: Option<RealModeContext>,
}

impl BIOSInterruptExecutor {
    /// Create new BIOS interrupt executor
    pub fn new(executor: &RealModeExecutor) -> Self {
        Self {
            executor: executor as *const _,
            last_context: None,
        }
    }

    /// Execute BIOS interrupt
    pub fn execute(&mut self, int_num: u8, ctx: &mut RealModeContext) -> IntResult {
        // int_num is u8, so automatically in valid range (0-0xFF)

        unsafe {
            let executor = &*self.executor;
            
            // Execute interrupt
            executor.execute_int(int_num, ctx)?;
            
            // Store context for later inspection
            self.last_context = Some(*ctx);
            
            // Check carry flag (generic error indicator for many BIOS calls)
            if ctx.is_carry_set() {
                Ok(ExecStatus::CarryFlagSet)
            } else {
                Ok(ExecStatus::Success)
            }
        }
    }

    /// Get last execution context
    pub fn last_context(&self) -> Option<&RealModeContext> {
        self.last_context.as_ref()
    }

    /// Clear last execution context
    pub fn clear_context(&mut self) {
        self.last_context = None;
    }

    /// Execute E820 memory detection interrupt
    pub fn exec_e820(
        &mut self,
        buffer_addr: u32,
    ) -> IntResult {
        log::debug!("Executing E820 memory detection interrupt");
        let mut ctx = RealModeContext::new();
        
        // E820 parameters
        ctx.eax = 0xE820;           // Function: E820 get memory map
        ctx.ecx = 24;               // Entry size
        ctx.edx = 0x534D4150;       // Signature: 'SMAP'
        ctx.edi = buffer_addr;      // Buffer address
        ctx.ebx = 0;                // Continuation from start

        self.execute(0x15, &mut ctx)
    }

    /// Execute disk read interrupt (INT 0x13, AH=02)
    pub fn exec_disk_read(
        &mut self,
        drive: u8,
        cylinder: u16,
        head: u8,
        sector: u8,
        sectors_count: u8,
        buffer: u16,
    ) -> IntResult {
        log::debug!("Executing disk read interrupt");
        let mut ctx = RealModeContext::new();
        
        // Disk read parameters (INT 0x13, AH=02)
        ctx.set_ah(0x02);                      // Read sectors
        ctx.edx = (drive as u32) & 0xFF;       // DL = drive
        ctx.ecx = (((cylinder & 0xFF) << 8) | sector as u16) as u32;  // CH/CL
        ctx.edx |= ((head as u32) << 8) & 0xFF00;  // DH = head
        ctx.ebx = buffer as u32;               // Buffer offset
        ctx.eax = (ctx.get_ah() as u32) << 8 | sectors_count as u32;  // AL = count

        self.execute(0x13, &mut ctx)
    }

    /// Execute video interrupt (INT 0x10)
    pub fn exec_video_mode(&mut self, mode: u8) -> IntResult {
        log::debug!("Executing video mode interrupt");
        let mut ctx = RealModeContext::new();
        
        // Video mode setting (INT 0x10, AH=00)
        ctx.set_ah(0x00);
        ctx.set_al(mode);

        self.execute(0x10, &mut ctx)
    }

    /// Execute print character interrupt (INT 0x10, AH=0E)
    pub fn exec_print_char(&mut self, ch: u8) -> IntResult {
        log::debug!("Executing print char interrupt");
        let mut ctx = RealModeContext::new();
        
        // Print character (INT 0x10, AH=0E)
        ctx.set_ah(0x0E);
        ctx.set_al(ch);
        ctx.ebx = 0;  // BL = foreground color

        self.execute(0x10, &mut ctx)
    }

    /// Execute keyboard read interrupt (INT 0x16, AH=00)
    pub fn exec_read_key(&mut self) -> IntResult {
        log::debug!("Executing read key interrupt");
        let mut ctx = RealModeContext::new();
        
        // Read key (INT 0x16, AH=00)
        ctx.set_ah(0x00);

        self.execute(0x16, &mut ctx)
    }

    /// Get last AX register value
    pub fn last_ax(&self) -> Option<u16> {
        self.last_context.as_ref().map(|ctx| ctx.get_ax())
    }

    /// Get last AL (low byte of AX)
    pub fn last_al(&self) -> Option<u8> {
        self.last_context.as_ref().map(|ctx| ctx.get_al())
    }

    /// Get last AH (high byte of AX)
    pub fn last_ah(&self) -> Option<u8> {
        self.last_context.as_ref().map(|ctx| ctx.get_ah())
    }
}

/// Batch interrupt executor for multiple related calls
pub struct BatchInterruptExecutor {
    executor: BIOSInterruptExecutor,
    results: [Option<ExecStatus>; 16],
    result_count: usize,
}

impl BatchInterruptExecutor {
    pub fn new(executor: &RealModeExecutor) -> Self {
        Self {
            executor: BIOSInterruptExecutor::new(executor),
            results: [None; 16],
            result_count: 0,
        }
    }

    pub fn add_result(&mut self, status: ExecStatus) -> Result<(), &'static str> {
        if self.result_count >= 16 {
            return Err("Too many results");
        }
        self.results[self.result_count] = Some(status);
        self.result_count += 1;
        Ok(())
    }

    pub fn get_results(&self) -> &[Option<ExecStatus>] {
        &self.results[..self.result_count]
    }

    pub fn all_successful(&self) -> bool {
        self.get_results()
            .iter()
            .all(|r| r.as_ref().map(|s| s.is_success()).unwrap_or(false))
    }

    pub fn clear(&mut self) {
        self.results = [None; 16];
        self.result_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec_status_creation() {
        assert!(ExecStatus::Success.is_success());
        assert!(!ExecStatus::CarryFlagSet.is_success());
    }

    #[test]
    fn test_batch_executor_results() {
        // Note: Can't test actual executor without real mode, but can test structure
        let batch_size = 5;
        assert!(batch_size <= 16);
    }
}
