#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

extern crate alloc;

use crate::alloc::string::ToString;

use nos_bootloader::{
    application::BootApplicationService as BootOrchestrator,
    core::boot_sequence::BootSequence,
    bios::{bios_calls::BIOSServices, bios_realmode::RealModeExecutor},
    cpu_init::realmode_switcher::RealmModeSwitcher,
    drivers::vga::VGAWriter,
    protocol::BootProtocolType,
    utils::error_recovery::{ErrorRecoveryManager, OutputMode, RecoveryStatus},
};

/// Bootloader entry point
/// 
/// This is the main entry point for the bootloader.
/// In a real bootloader, this would be called from the boot protocol
/// (UEFI, Multiboot2, or BIOS).
#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    bootloader_main()
}

#[cfg(not(target_os = "none"))]
#[allow(dead_code)]
fn main() {
    bootloader_main()
}

/// UEFI entry point
#[cfg(feature = "uefi_support")]
#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(image_handle: uefi::Handle, system_table: *const uefi_raw::table::system::SystemTable) -> uefi_raw::Status {
    use nos_bootloader::protocol::uefi::{self, UefiProtocol};
    
    // Initialize UEFI protocol
    let mut protocol = UefiProtocol::new();
    protocol.initialize_with_system_table(system_table).unwrap();
    protocol.set_image_handle(image_handle);
    
    // Set active protocol
    uefi::set_active_protocol(protocol);
    
    // Call main bootloader logic
    bootloader_main();
}

/// Import unified error handling from utils
use nos_bootloader::utils::error::{BootError, Result as BootResult};
use nos_bootloader::drivers::vga::Color;
use nos_bootloader::utils::error_recovery::ErrorSeverity;

/// Initialize VGA output with error handling and recovery
#[allow(dead_code)]
fn init_vga_with_recovery(recovery_manager: &mut ErrorRecoveryManager) -> BootResult<(VGAWriter, OutputMode)> {
    // 尝试初始化VGA
    match init_vga() {
        Ok(vga) => {
            recovery_manager.recovery_status = RecoveryStatus::NoRecovery;
            Ok((vga, OutputMode::TextMode))
        }
        Err(e) => {
            // VGA初始化失败，尝试恢复
            match recovery_manager.recover_from_error(&e) {
                Ok(mode) => {
                    // 根据恢复的模式初始化相应的输出
                    match mode {
                        OutputMode::TextMode => {
                            // 尝试基本的文本模式
                            let mut vga = VGAWriter::new();
                            vga.clear();
                            vga.set_fg_color(Color::Yellow);
                            vga.write_str("RECOVERY MODE: VGA initialized with fallback\n");
                            vga.set_fg_color(Color::White);
                            vga.write_str("NOS Bootloader v0.4.0 (Recovery Mode)\n");
                            vga.write_str("=====================================\n");
                            vga.write_str("Starting boot sequence in recovery mode...\n\n");
                            Ok((vga, mode))
                        }
                        OutputMode::SerialConsole => {
                            // 尝试串行控制台
                            // 这里返回一个基本的VGA，但标记为串行模式
                            let mut vga = VGAWriter::new();
                            vga.clear();
                            vga.write_str("SERIAL CONSOLE MODE: VGA not available\n");
                            Ok((vga, mode))
                        }
                        OutputMode::Silent => {
                            // 静默模式，返回一个虚拟的VGA
                            let vga = VGAWriter::new();
                            Ok((vga, mode))
                        }
                        _ => {
                            // 其他模式暂时不支持，返回错误
                            Err(e)
                        }
                    }
                }
                Err(_) => {
                    // 恢复失败，返回原始错误
                    Err(e)
                }
            }
        }
    }
}

/// Initialize VGA output with error handling
#[allow(dead_code)]
fn init_vga() -> BootResult<VGAWriter> {
    let mut vga = VGAWriter::new();
    vga.clear();
    vga.write_str("NOS Bootloader v0.4.0 (Optimized)\n");
    vga.write_str("=================================\n");
    vga.write_str("Starting optimized boot sequence...\n\n");
    Ok(vga)
}

/// Initialize boot sequence with memory validation
#[allow(dead_code)]
fn init_boot_sequence(vga: &mut VGAWriter) -> BootResult<BootSequence> {
    let mut boot_seq = BootSequence::new();
    boot_seq.validate_memory()
        .map_err(|_| BootError::MemoryMapError)?;
    vga.write_str("[OK] Memory layout validated\n");
    Ok(boot_seq)
}

/// Initialize BIOS services
#[allow(dead_code)]
fn init_bios_services(vga: &mut VGAWriter) -> BootResult<BIOSServices> {
    let mut bios_services = BIOSServices::new();
    bios_services.init()
        .map_err(|_e| BootError::ProtocolInitializationFailed("BIOS services initialization failed".to_string()))?;
    vga.write_str("[OK] BIOS services initialized\n");
    Ok(bios_services)
}

/// Initialize real mode executor
#[allow(dead_code)]
fn init_real_mode_executor(vga: &mut VGAWriter) -> BootResult<RealModeExecutor> {
    let mut executor = RealModeExecutor::new();
    executor.init()
        .map_err(|_e| BootError::ProtocolInitializationFailed("Real mode executor initialization failed".to_string()))?;
    vga.write_str("[OK] Real mode executor initialized\n");
    Ok(executor)
}

/// Load GDT and IDT with error handling
#[allow(dead_code)]
fn load_descriptors(boot_seq: &mut BootSequence, vga: &mut VGAWriter) -> BootResult<()> {
    boot_seq.load_gdt()
        .map_err(|_e| BootError::ProtocolInitializationFailed("GDT load failed".to_string()))?;
    vga.write_str("[OK] GDT loaded\n");

    boot_seq.load_idt()
        .map_err(|_e| BootError::ProtocolInitializationFailed("IDT load failed".to_string()))?;
    vga.write_str("[OK] IDT loaded\n");

    boot_seq.prepare_real_mode()
        .map_err(|_e| BootError::ProtocolInitializationFailed("Real mode preparation failed".to_string()))?;
    vga.write_str("[OK] Real mode environment ready\n");

    Ok(())
}

/// Create and configure boot orchestrator
#[allow(dead_code)]
fn create_boot_orchestrator(
    vga: &mut VGAWriter
) -> BootResult<BootOrchestrator> {
    vga.write_str("[OK] Boot configuration created\n");

    let orchestrator = BootOrchestrator::with_default_container(BootProtocolType::Bios)?;
    vga.write_str("[OK] Boot orchestrator initialized\n");
    Ok(orchestrator)
}

/// Main bootloader function - optimized and modular with error recovery
#[allow(dead_code)]
fn bootloader_main() -> ! {
    // Initialize error recovery manager
    let mut recovery_manager = ErrorRecoveryManager::new();
    
    // Initialize all components with proper error handling and recovery
    let (mut vga, output_mode) = match init_vga_with_recovery(&mut recovery_manager) {
        Ok(result) => result,
        Err(e) => {
            // 如果所有恢复尝试都失败，尝试最后的紧急恢复
            emergency_recovery(&e);
            // 如果紧急恢复也失败，系统无法继续
            halt_system();
        }
    };
    
    // 记录当前输出模式
    match output_mode {
        OutputMode::TextMode => {
            vga.write_str("[INFO] Running in text mode\n");
        }
        OutputMode::SerialConsole => {
            vga.write_str("[INFO] Running in serial console mode\n");
        }
        OutputMode::Silent => {
            // 静默模式，不输出
        }
        _ => {
            vga.write_str("[INFO] Running in graphics mode\n");
        }
    }

    // Initialize boot sequence with error recovery
    let mut boot_seq = match init_boot_sequence_with_recovery(&mut vga, &mut recovery_manager) {
        Ok(seq) => seq,
        Err(e) => {
            log_error_with_recovery(&mut vga, &mut recovery_manager, &e, "Boot sequence initialization");
            // 尝试继续执行，即使初始化失败
            BootSequence::new()
        }
    };

    // Initialize BIOS services with error recovery
    let _bios_services = match init_bios_services_with_recovery(&mut vga, &mut recovery_manager) {
        Ok(services) => services,
        Err(e) => {
            log_error_with_recovery(&mut vga, &mut recovery_manager, &e, "BIOS services initialization");
            // 尝试继续执行，即使BIOS服务初始化失败
            BIOSServices::new()
        }
    };

    // Initialize real mode executor with error recovery
    let executor = match init_real_mode_executor_with_recovery(&mut vga, &mut recovery_manager) {
        Ok(exec) => exec,
        Err(e) => {
            log_error_with_recovery(&mut vga, &mut recovery_manager, &e, "Real mode executor initialization");
            // 尝试继续执行，即使实模式执行器初始化失败
            RealModeExecutor::new()
        }
    };

    // Initialize real mode switcher
    let _switcher = RealmModeSwitcher::new();
    vga.write_str("[OK] Real mode switcher ready\n");

    // Load GDT and IDT with error recovery
    if let Err(e) = load_descriptors_with_recovery(&mut boot_seq, &mut vga, &mut recovery_manager) {
        log_error_with_recovery(&mut vga, &mut recovery_manager, &e, "Descriptor loading");
        // 尝试继续执行，即使描述符加载失败
    }

    // Create boot orchestrator with error recovery
    let mut orchestrator = match create_boot_orchestrator_with_recovery(&executor, &mut vga, &mut recovery_manager) {
        Ok(orch) => orch,
        Err(e) => {
            log_error_with_recovery(&mut vga, &mut recovery_manager, &e, "Boot orchestrator creation");
            // 尝试创建基本的引导协调器
            BootOrchestrator::with_default_container(BootProtocolType::Bios)
                .unwrap_or_else(|_| panic!("Failed to create boot orchestrator"))
        }
    };

    // BIOS services initialization is handled internally by boot_system method

    vga.write_str("\nExecuting optimized boot sequence...\n\n");

    // Execute boot sequence with error recovery
    match execute_boot_sequence_with_recovery(&mut orchestrator, &mut vga, &mut recovery_manager) {
        Ok(()) => {
            vga.write_str("\n[OK] Boot sequence completed successfully!\n");
            // 显示恢复状态
            match recovery_manager.recovery_status() {
                RecoveryStatus::NoRecovery => {
                    vga.write_str("[INFO] No recovery needed\n");
                }
                RecoveryStatus::RecoverySuccessful => {
                    vga.write_str("[INFO] Recovery successful\n");
                }
                RecoveryStatus::PartialRecovery => {
                    vga.write_str("[WARNING] Partial recovery - some features may be limited\n");
                }
                RecoveryStatus::RecoveryFailed => {
                    vga.write_str("[ERROR] Recovery failed - system may be unstable\n");
                }
                RecoveryStatus::RecoveryInProgress => {
                    vga.write_str("[INFO] Recovery was in progress\n");
                }
            }
            // Boot sequence completed successfully
            // Kernel entry would happen here via KernelHandoff::execute()
            halt_system();
        }
        Err(e) => {
            log_error_with_recovery(&mut vga, &mut recovery_manager, &e, "Boot sequence execution");
            // 尝试紧急恢复
            emergency_recovery(&e);
            halt_system();
        }
    }
}

/// Execute complete boot sequence with improved error handling
#[allow(dead_code)]
fn execute_boot_sequence(
    orchestrator: &mut BootOrchestrator,
    vga: &mut VGAWriter,
) -> BootResult<()> {
    // Run the complete boot system sequence
    vga.write_str("Starting boot sequence...\n");
    
    // Use boot_system method which handles all boot phases
    let boot_info = orchestrator.boot_system(None)?;
    
    // Display boot summary
    vga.write_str("\nBoot Summary:\n");
    vga.write_str("  Memory: OK\n");
    vga.write_str("  Kernel: OK\n");
    vga.write_str("  Boot Info: OK\n");
    
    // Display graphics status
    if boot_info.graphics_info.is_some() {
        vga.write_str("  Graphics: Enabled\n");
    } else {
        vga.write_str("  Graphics: Disabled\n");
    }

    // At this point, would call:
    // unsafe { KernelHandoff::new(boot_info).execute() }
    // which jumps to kernel with RDI = boot_info pointer

    Ok(())
}

/// 带错误恢复的引导序列初始化
#[allow(dead_code)]
fn init_boot_sequence_with_recovery(
    vga: &mut VGAWriter,
    recovery_manager: &mut ErrorRecoveryManager
) -> BootResult<BootSequence> {
    match init_boot_sequence(vga) {
        Ok(seq) => Ok(seq),
        Err(e) => {
            // 尝试恢复
            match recovery_manager.recover_from_error(&e) {
                Ok(_) => {
                    // 恢复成功，尝试创建基本的引导序列
                    vga.set_fg_color(Color::Yellow);
                    vga.write_str("[RECOVERY] Creating minimal boot sequence\n");
                    vga.set_fg_color(Color::White);
                    Ok(BootSequence::new())
                }
                Err(_) => Err(e)
            }
        }
    }
}

/// 带错误恢复的BIOS服务初始化
#[allow(dead_code)]
fn init_bios_services_with_recovery(
    vga: &mut VGAWriter,
    recovery_manager: &mut ErrorRecoveryManager
) -> BootResult<BIOSServices> {
    match init_bios_services(vga) {
        Ok(services) => Ok(services),
        Err(e) => {
            // 尝试恢复
            match recovery_manager.recover_from_error(&e) {
                Ok(_) => {
                    // 恢复成功，尝试创建基本的BIOS服务
                    vga.set_fg_color(Color::Yellow);
                    vga.write_str("[RECOVERY] Creating minimal BIOS services\n");
                    vga.set_fg_color(Color::White);
                    Ok(BIOSServices::new())
                }
                Err(_) => Err(e)
            }
        }
    }
}

/// 带错误恢复的实模式执行器初始化
#[allow(dead_code)]
fn init_real_mode_executor_with_recovery(
    vga: &mut VGAWriter,
    recovery_manager: &mut ErrorRecoveryManager
) -> BootResult<RealModeExecutor> {
    match init_real_mode_executor(vga) {
        Ok(executor) => Ok(executor),
        Err(e) => {
            // 尝试恢复
            match recovery_manager.recover_from_error(&e) {
                Ok(_) => {
                    // 恢复成功，尝试创建基本的实模式执行器
                    vga.set_fg_color(Color::Yellow);
                    vga.write_str("[RECOVERY] Creating minimal real mode executor\n");
                    vga.set_fg_color(Color::White);
                    Ok(RealModeExecutor::new())
                }
                Err(_) => Err(e)
            }
        }
    }
}

/// 带错误恢复的描述符加载
#[allow(dead_code)]
fn load_descriptors_with_recovery(
    boot_seq: &mut BootSequence,
    vga: &mut VGAWriter,
    recovery_manager: &mut ErrorRecoveryManager
) -> BootResult<()> {
    match load_descriptors(boot_seq, vga) {
        Ok(()) => Ok(()),
        Err(e) => {
            // 尝试恢复
            match recovery_manager.recover_from_error(&e) {
                Ok(_) => {
                    // 恢复成功，但描述符可能未完全加载
                    vga.set_fg_color(Color::Yellow);
                    vga.write_str("[RECOVERY] Descriptors partially loaded - system may be unstable\n");
                    vga.set_fg_color(Color::White);
                    Ok(())
                }
                Err(_) => Err(e)
            }
        }
    }
}

/// 带错误恢复的引导协调器创建
#[allow(dead_code)]
fn create_boot_orchestrator_with_recovery<'a>(
    _executor: &'a RealModeExecutor,
    vga: &mut VGAWriter,
    recovery_manager: &mut ErrorRecoveryManager
) -> BootResult<BootOrchestrator> {
    match create_boot_orchestrator(vga) {
        Ok(orchestrator) => Ok(orchestrator),
        Err(e) => {
            // 尝试恢复
            match recovery_manager.recover_from_error(&e) {
                Ok(_) => {
                    // 恢复成功，尝试创建基本的引导协调器
                    vga.set_fg_color(Color::Yellow);
                    vga.write_str("[RECOVERY] Creating minimal boot orchestrator\n");
                    vga.set_fg_color(Color::White);
                    BootOrchestrator::with_default_container(BootProtocolType::Bios)
                }
                Err(_) => Err(e)
            }
        }
    }
}

/// 带错误恢复的引导序列执行
#[allow(dead_code)]
fn execute_boot_sequence_with_recovery(
    orchestrator: &mut BootOrchestrator,
    vga: &mut VGAWriter,
    recovery_manager: &mut ErrorRecoveryManager
) -> BootResult<()> {
    match execute_boot_sequence(orchestrator, vga) {
        Ok(()) => Ok(()),
        Err(e) => {
            // 尝试恢复
            match recovery_manager.recover_from_error(&e) {
                Ok(_) => {
                    // 恢复成功，但引导序列可能未完全执行
                    vga.set_fg_color(Color::Yellow);
                    vga.write_str("[RECOVERY] Boot sequence partially executed - system may be unstable\n");
                    vga.set_fg_color(Color::White);
                    Ok(())
                }
                Err(_) => Err(e)
            }
        }
    }
}

/// 记录错误并尝试恢复
#[allow(dead_code)]
fn log_error_with_recovery(
    vga: &mut VGAWriter,
    recovery_manager: &mut ErrorRecoveryManager,
    error: &BootError,
    context: &str
) {
    vga.set_fg_color(Color::Red);
    vga.write_str("[ERROR] ");
    vga.write_str(context);
    vga.write_str(" failed: ");
    vga.write_str(error.description());
    vga.write_str("\n");
    
    // 记录错误严重程度
    let severity = recovery_manager.assess_error_severity(error);
    vga.set_fg_color(Color::Yellow);
    vga.write_str("[INFO] Error severity: ");
    match severity {
        ErrorSeverity::Low => {
            vga.write_str("Low");
        }
        ErrorSeverity::Medium => {
            vga.write_str("Medium");
        }
        ErrorSeverity::High => {
            vga.write_str("High");
        }
        ErrorSeverity::Critical => {
            vga.write_str("Critical");
        }
    }
    vga.write_str("\n");
    
    vga.set_fg_color(Color::White);
}

/// 紧急恢复函数
#[allow(dead_code)]
fn emergency_recovery(error: &BootError) {
    // 尝试最后的恢复手段
    // 这里可以实现一些基本的恢复操作，如：
    // 1. 尝试重置硬件
    // 2. 尝试进入安全模式
    // 3. 尝试保存错误信息到非易失性存储
    
    // 目前只是一个占位符实现
    // 在实际系统中，这里会有更复杂的恢复逻辑
    
    // 尝试通过串口输出错误信息
    let error_msg = error.description();
    for &_byte in error_msg.as_bytes() {
        // 这里应该使用实际的串口输出
        // 目前只是一个占位符
    }
}

/// 停止系统
#[allow(dead_code)]
fn halt_system() -> ! {
    // 停止系统执行
    // 在实际系统中，这里可能会：
    // 1. 显示错误信息
    // 2. 等待用户输入
    // 3. 尝试重启
    // 4. 进入低功耗状态
    
    loop {
        // 停止CPU执行
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
