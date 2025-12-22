//! 错误恢复机制
//!
//! 提供多级错误恢复策略，包括图形模式回退、文本模式回退和串行控制台回退。

use crate::drivers::vga::VGAWriter;
use crate::utils::error::{BootError, Result as BootResult};
use itoa;

/// 错误严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// 低严重性 - 可以继续执行
    Low = 1,
    /// 中等严重性 - 需要回退到备用模式
    Medium = 2,
    /// 高严重性 - 需要重大回退
    High = 3,
    /// 致命错误 - 系统无法恢复
    Critical = 4,
}

/// 输出模式类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// 高分辨率图形模式
    HighResolutionGraphics,
    /// 低分辨率图形模式
    LowResolutionGraphics,
    /// 标准文本模式
    TextMode,
    /// 串行控制台
    SerialConsole,
    /// 无输出（静默模式）
    Silent,
}

/// 恢复状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStatus {
    /// 无需恢复
    NoRecovery,
    /// 成功恢复
    RecoverySuccessful,
    /// 部分恢复
    PartialRecovery,
    /// 恢复失败
    RecoveryFailed,
    /// 恢复进行中
    RecoveryInProgress,
}

/// 错误恢复管理器
pub struct ErrorRecoveryManager {
    /// 当前输出模式
    pub current_mode: OutputMode,
    /// 恢复状态
    pub recovery_status: RecoveryStatus,
    /// 尝试过的恢复策略
    pub attempted_strategies: u8,
    /// 最大重试次数
    pub max_retries: u8,
}

impl ErrorRecoveryManager {
    /// 创建新的错误恢复管理器
    pub fn new() -> Self {
        Self {
            current_mode: OutputMode::HighResolutionGraphics,
            recovery_status: RecoveryStatus::NoRecovery,
            attempted_strategies: 0,
            max_retries: 3,
        }
    }

    /// 评估错误严重程度
    pub fn assess_error_severity(&self, error: &BootError) -> ErrorSeverity {
        match error {
            // 低严重性错误
            BootError::Timeout
            | BootError::ConnectionFailed
            | BootError::DeviceNotFound
            | BootError::FileNotFound
            | BootError::KernelNotFound => ErrorSeverity::Low,

            // 中等严重性错误
            BootError::MemoryAllocationFailed
            | BootError::MemoryMapError
            | BootError::ProtocolNotSupported
            | BootError::ProtocolDetectionFailed
            | BootError::FileSystemError
            | BootError::NetworkError
            | BootError::InvalidFileFormat
            | BootError::ConfigurationError(_)
            | BootError::ValidationError(_)
            | BootError::BiosNotSupported
            | BootError::UserRequestedRecovery => ErrorSeverity::Medium,

            // 高严重性错误
            BootError::InitializationFailed(_)
            | BootError::ProtocolInitializationFailed(_)
            | BootError::UefiError(_)
            | BootError::BiosInterruptFailed(_)
            | BootError::DeviceError(_)
            | BootError::HardwareError(_)
            | BootError::KernelLoadFailed
            | BootError::InvalidKernelFormat
            | BootError::InvalidBootConfig
            | BootError::NotInitialized
            | BootError::InvalidState
            | BootError::CorruptionDetected
            | BootError::Generic(_)
            | BootError::FeatureNotEnabled(_)
            | BootError::ServiceResolutionFailed(_)
            | BootError::EventError(_) => ErrorSeverity::High,

            // 致命错误
            BootError::OutOfMemory
            | BootError::InsufficientMemory
            | BootError::UnsupportedArchitecture
            | BootError::RecoveryModeFailed
            | BootError::RecoveryNotSupported
            | BootError::SystemHalted
            | BootError::SystemRebooted
            | BootError::SystemShutdown
            | BootError::KernelReturned
            | BootError::UefiNotFound
            | BootError::UefiUnsupported
            | BootError::UefiNullSystemTable
            | BootError::UefiSystemTableNotInitialized
            | BootError::NotImplemented => ErrorSeverity::Critical,
        }
    }

    /// 执行错误恢复
    pub fn recover_from_error(&mut self, error: &BootError) -> BootResult<OutputMode> {
        let severity = self.assess_error_severity(error);
        self.recovery_status = RecoveryStatus::RecoveryInProgress;

        // 根据严重程度选择恢复策略
        match severity {
            ErrorSeverity::Low => {
                // 低严重性错误，尝试简单重试
                self.log_recovery_attempt("Low severity error - attempting retry");
                Ok(self.current_mode)
            }
            ErrorSeverity::Medium => {
                // 中等严重性错误，尝试降级输出模式
                self.log_recovery_attempt("Medium severity error - attempting fallback");
                self.fallback_output_mode()
            }
            ErrorSeverity::High => {
                // 高严重性错误，尝试重大回退
                self.log_recovery_attempt("High severity error - attempting major fallback");
                self.major_fallback()
            }
            ErrorSeverity::Critical => {
                // 致命错误，尝试最后的恢复手段
                self.log_recovery_attempt("Critical error - attempting last resort recovery");
                self.last_resort_recovery()
            }
        }
    }

    /// 降级输出模式
    fn fallback_output_mode(&mut self) -> BootResult<OutputMode> {
        self.attempted_strategies += 1;
        
        if self.attempted_strategies > self.max_retries {
            self.recovery_status = RecoveryStatus::RecoveryFailed;
            return Err(BootError::RecoveryModeFailed);
        }

        match self.current_mode {
            OutputMode::HighResolutionGraphics => {
                self.log_recovery_attempt("Falling back to low resolution graphics");
                self.current_mode = OutputMode::LowResolutionGraphics;
                Ok(OutputMode::LowResolutionGraphics)
            }
            OutputMode::LowResolutionGraphics => {
                self.log_recovery_attempt("Falling back to text mode");
                self.current_mode = OutputMode::TextMode;
                Ok(OutputMode::TextMode)
            }
            OutputMode::TextMode => {
                self.log_recovery_attempt("Falling back to serial console");
                self.current_mode = OutputMode::SerialConsole;
                Ok(OutputMode::SerialConsole)
            }
            OutputMode::SerialConsole => {
                self.log_recovery_attempt("Falling back to silent mode");
                self.current_mode = OutputMode::Silent;
                Ok(OutputMode::Silent)
            }
            OutputMode::Silent => {
                self.recovery_status = RecoveryStatus::RecoveryFailed;
                Err(BootError::RecoveryModeFailed)
            }
        }
    }

    /// 重大回退
    fn major_fallback(&mut self) -> BootResult<OutputMode> {
        self.attempted_strategies += 1;
        
        if self.attempted_strategies > self.max_retries {
            self.recovery_status = RecoveryStatus::RecoveryFailed;
            return Err(BootError::RecoveryModeFailed);
        }

        // 直接跳到文本模式
        self.log_recovery_attempt("Major fallback - jumping to text mode");
        self.current_mode = OutputMode::TextMode;
        self.recovery_status = RecoveryStatus::PartialRecovery;
        Ok(OutputMode::TextMode)
    }

    /// 最后手段恢复
    fn last_resort_recovery(&mut self) -> BootResult<OutputMode> {
        self.attempted_strategies += 1;
        
        if self.attempted_strategies > self.max_retries {
            self.recovery_status = RecoveryStatus::RecoveryFailed;
            return Err(BootError::RecoveryModeFailed);
        }

        // 尝试串行控制台
        self.log_recovery_attempt("Last resort - attempting serial console");
        self.current_mode = OutputMode::SerialConsole;
        self.recovery_status = RecoveryStatus::PartialRecovery;
        Ok(OutputMode::SerialConsole)
    }

    /// 记录恢复尝试
    fn log_recovery_attempt(&mut self, message: &str) {
        // 根据当前模式尝试记录日志
        match self.current_mode {
            OutputMode::HighResolutionGraphics | OutputMode::LowResolutionGraphics => {
                // 尝试图形模式日志记录
                self.log_to_graphics(message);
            }
            OutputMode::TextMode => {
                // 尝试文本模式日志记录
                self.log_to_text(message);
            }
            OutputMode::SerialConsole => {
                // 尝试串行控制台日志记录
                self.log_to_serial(message);
            }
            OutputMode::Silent => {
                // 静默模式，不记录日志
            }
        }
    }

    /// 图形模式日志记录
    fn log_to_graphics(&self, message: &str) {
        // 尝试使用VGA记录日志
        if let Ok(mut vga) = self.init_vga_text_mode() {
            vga.set_fg_color(crate::drivers::vga::Color::Yellow);
            vga.write_str("[RECOVERY] ");
            vga.write_str(message);
            vga.write_str("\n");
        }
    }

    /// 文本模式日志记录
    fn log_to_text(&self, message: &str) {
        if let Ok(mut vga) = self.init_vga_text_mode() {
            vga.set_fg_color(crate::drivers::vga::Color::Yellow);
            vga.write_str("[RECOVERY] ");
            vga.write_str(message);
            vga.write_str("\n");
        }
    }

    /// 串行控制台日志记录
    fn log_to_serial(&self, message: &str) {
        // 简单的串行日志记录实现
        // 在实际系统中，这里会使用串口端口
        for &_byte in message.as_bytes() {
            log::trace!("Writing byte to serial console");
            // 这里应该使用实际的串口输出
            // 目前只是一个占位符
        }
    }

    /// 初始化VGA文本模式
    fn init_vga_text_mode(&self) -> BootResult<VGAWriter> {
        let mut vga = VGAWriter::new();
        vga.clear();
        vga.set_fg_color(crate::drivers::vga::Color::White);
        vga.set_bg_color(crate::drivers::vga::Color::Black);
        Ok(vga)
    }

    /// 获取当前恢复状态
    pub fn recovery_status(&self) -> RecoveryStatus {
        self.recovery_status
    }

    /// 获取当前输出模式
    pub fn current_mode(&self) -> OutputMode {
        self.current_mode
    }

    /// 重置恢复状态
    pub fn reset(&mut self) {
        self.current_mode = OutputMode::HighResolutionGraphics;
        self.recovery_status = RecoveryStatus::NoRecovery;
        self.attempted_strategies = 0;
    }
}

impl Default for ErrorRecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Panic handler for bootloader
pub fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    // Try to use VGA output for panic message
    let mut vga = VGAWriter::new();
    vga.set_fg_color(crate::drivers::vga::Color::Red);
    vga.set_bg_color(crate::drivers::vga::Color::Black);
    vga.write_str("\n=== BOOTLOADER PANIC ===\n");
    vga.write_str("Location: ");
    if let Some(location) = info.location() {
        vga.write_str(location.file());
        vga.write_str(":");
        vga.write_str(itoa::Buffer::new().format(location.line()));
    } else {
        vga.write_str("unknown location");
    }
    vga.write_str("\nMessage: ");
    vga.write_str(info.message().as_str().unwrap_or("No message"));
    vga.write_str("\n");
    
    // Halt the system
    loop {
        core::hint::spin_loop();
    }
}
