//! 安全模块
//! 
//! 提供增强的安全功能，包括细粒度权限控制、
//! 能力安全、审计和强制访问控制。

pub mod enhanced_permissions;
pub mod memory_security;
pub mod stack_canaries;
pub mod aslr;

// 只导出在其他地方直接使用的安全函数
pub use enhanced_permissions::init_permission_manager;
pub use memory_security::{init_security, create_process_security_context, remove_process_security_context};
pub use stack_canaries::CanaryConfig;
pub use aslr::{initialize_aslr, randomize_memory_region, MemoryRegionType, is_aslr_enabled};

// Global ASLR subsystem instance
use spin::Mutex;
use aslr::AslrSubsystem;
pub static ASLR: Mutex<Option<AslrSubsystem>> = Mutex::new(None);



/// Initialize security subsystem
pub fn init_security_subsystem() -> Result<(), SecurityError> {
    // Initialize enhanced permission system
    init_permission_manager();
    
    // Initialize memory security system
    memory_security::init_security()?;
    
    // Initialize stack canaries system
    let canary_config = CanaryConfig::default();
    stack_canaries::init_stack_canaries(canary_config)
        .map_err(|_| SecurityError::PermissionDenied)?;
    
    Ok(())
}

/// Get current security level for a process
pub fn get_current_security_level(pid: u32) -> memory_security::SecurityLevel {
    // For now, return Medium security level for all non-kernel processes
    // In a real implementation, this would be based on process credentials
    if pid == 0 {
        memory_security::SecurityLevel::System
    } else {
        memory_security::SecurityLevel::Medium
    }
}

/// Security errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    /// Permission denied
    PermissionDenied,
    /// Process not found
    ProcessNotFound,
    /// Memory security error
    MemorySecurityError(memory_security::SecurityError),
}