//! Foreign ABI Support Layer
//!
//! Provides support for multiple calling conventions and ABI specifications:
//! - Microsoft x64 calling convention (Windows)
//! - System V AMD64 ABI (Linux/macOS)
//! - ARM AArch64 AAPCS (ARM64)
//! - ARM AArch32 AAPCS (ARM32)
//! - RISC-V calling convention

extern crate alloc;
extern crate hashbrown;

use core::ffi::{c_void, c_char, c_int, c_uint};
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;
use alloc::{format, vec};
use alloc::boxed::Box;
use hashbrown::HashMap;
use crate::compat::DefaultHasherBuilder;
use spin::Mutex;

use crate::compat::*;

/// ABI converter for handling different calling conventions
pub struct AbiConverter {
    /// Platform calling conventions
    calling_conventions: HashMap<TargetPlatform, CallingConvention, DefaultHasherBuilder>,
    /// Argument conversion tables
    arg_converters: HashMap<(TargetPlatform, CallingConvention), Box<dyn ArgumentConverter>, DefaultHasherBuilder>,
    /// Register mappings for different architectures
    register_mappings: HashMap<Architecture, RegisterMapping, DefaultHasherBuilder>,
    /// Stack layout specifications
    stack_layouts: HashMap<CallingConvention, StackLayout, DefaultHasherBuilder>,
}

impl AbiConverter {
    /// Create a new ABI converter
    pub fn new() -> Self {
        let mut converter = Self {
            calling_conventions: HashMap::with_hasher(DefaultHasherBuilder),
            arg_converters: HashMap::with_hasher(DefaultHasherBuilder),
            register_mappings: HashMap::with_hasher(DefaultHasherBuilder),
            stack_layouts: HashMap::with_hasher(DefaultHasherBuilder),
        };

        // Initialize calling conventions for each platform
        converter.calling_conventions.insert(TargetPlatform::Windows, CallingConvention::MicrosoftX64);
        converter.calling_conventions.insert(TargetPlatform::Linux, CallingConvention::SystemVAMD64);
        converter.calling_conventions.insert(TargetPlatform::MacOS, CallingConvention::SystemVAMD64);
        converter.calling_conventions.insert(TargetPlatform::Android, CallingConvention::SystemVAMD64);
        converter.calling_conventions.insert(TargetPlatform::IOS, CallingConvention::AArch64AAPCS);

        // Initialize register mappings
        converter.init_register_mappings();

        // Initialize stack layouts
        converter.init_stack_layouts();

        converter
    }

    /// Convert argument from foreign ABI to NOS ABI
    pub fn convert_argument(&mut self, from_platform: TargetPlatform, to_platform: TargetPlatform,
                           arg: usize, arg_index: usize) -> Result<usize> {
        let from_cc = self.calling_conventions.get(&from_platform)
            .ok_or(CompatibilityError::UnsupportedArchitecture)?;
        let to_cc = self.calling_conventions.get(&to_platform)
            .unwrap_or(&CallingConvention::SystemVAMD64); // Default NOS ABI

        // If conventions are the same, no conversion needed
        if from_cc == to_cc {
            return Ok(arg);
        }

        // Get converter for this platform combination
        let converter_key = (from_platform, *from_cc);
        let converter = self.arg_converters.get(&converter_key);

        if let Some(conv) = converter {
            conv.convert_argument(arg, arg_index)
        } else {
            // Default conversion (no-op)
            Ok(arg)
        }
    }

    /// Convert function arguments array from foreign ABI to NOS ABI
    pub fn convert_arguments(&mut self, from_platform: TargetPlatform, to_platform: TargetPlatform,
                           args: &[usize]) -> Result<Vec<usize>> {
        let from_cc = self.calling_conventions.get(&from_platform)
            .ok_or(CompatibilityError::UnsupportedArchitecture)?;
        let to_cc = self.calling_conventions.get(&to_platform)
            .unwrap_or(&CallingConvention::SystemVAMD64);

        if from_cc == to_cc {
            return Ok(args.to_vec());
        }

        let mut converted_args = Vec::with_capacity(args.len());
        for (i, &arg) in args.iter().enumerate() {
            converted_args.push(self.convert_argument(from_platform, to_platform, arg, i)?);
        }

        Ok(converted_args)
    }

    /// Prepare call frame for foreign function call
    pub fn prepare_call_frame(&self, platform: TargetPlatform, architecture: Architecture,
                           args: &[usize]) -> Result<CallFrame> {
        let calling_convention = self.calling_conventions.get(&platform)
            .ok_or(CompatibilityError::UnsupportedArchitecture)?;

        let register_mapping = self.register_mappings.get(&architecture)
            .ok_or(CompatibilityError::UnsupportedArchitecture)?;

        let stack_layout = self.stack_layouts.get(calling_convention)
            .ok_or(CompatibilityError::UnsupportedArchitecture)?;

        let mut call_frame = CallFrame::new();

        // Set up registers according to calling convention
        match calling_convention {
            CallingConvention::MicrosoftX64 => {
                self.setup_microsoft_x64_frame(&mut call_frame, register_mapping, args)?;
            }
            CallingConvention::SystemVAMD64 => {
                self.setup_systemv_amd64_frame(&mut call_frame, register_mapping, args)?;
            }
            CallingConvention::AArch64AAPCS => {
                self.setup_aarch64_frame(&mut call_frame, register_mapping, args)?;
            }
            CallingConvention::AArch32AAPCS => {
                self.setup_aarch32_frame(&mut call_frame, register_mapping, args)?;
            }
            CallingConvention::RiscV => {
                self.setup_riscv_frame(&mut call_frame, register_mapping, args)?;
            }
        }

        call_frame.stack_layout = Some(stack_layout.clone());

        Ok(call_frame)
    }

    /// Convert return value from foreign ABI to NOS ABI
    pub fn convert_return_value(&self, from_platform: TargetPlatform, to_platform: TargetPlatform,
                              value: usize) -> usize {
        // For most simple return values, no conversion is needed
        // In a real implementation, this would handle struct returns, etc.
        value
    }

    /// Set up Microsoft x64 call frame
    fn setup_microsoft_x64_frame(&self, call_frame: &mut CallFrame, _register_mapping: &RegisterMapping,
                                args: &[usize]) -> Result<()> {
        // Microsoft x64 ABI:
        // - First 4 args: RCX, RDX, R8, R9
        // - Remaining args: stack (right-to-left)
        // - Stack must be 16-byte aligned before call
        // - Shadow space for register save area (32 bytes)

        call_frame.clear_registers();

        // Set register arguments
        if args.len() > 0 { call_frame.set_register("rcx", args[0]); }
        if args.len() > 1 { call_frame.set_register("rdx", args[1]); }
        if args.len() > 2 { call_frame.set_register("r8", args[2]); }
        if args.len() > 3 { call_frame.set_register("r9", args[3]); }

        // Set stack arguments
        for i in 4..args.len() {
            call_frame.push_stack(args[i]);
        }

        // Allocate shadow space
        call_frame.push_stack(0); // Will be adjusted for proper alignment

        Ok(())
    }

    /// Set up System V AMD64 call frame
    fn setup_systemv_amd64_frame(&self, call_frame: &mut CallFrame, _register_mapping: &RegisterMapping,
                                 args: &[usize]) -> Result<()> {
        // System V AMD64 ABI:
        // - First 6 args: RDI, RSI, RDX, RCX, R8, R9
        // - Remaining args: stack (right-to-left)
        // - Stack must be 16-byte aligned before call

        call_frame.clear_registers();

        // Set register arguments
        if args.len() > 0 { call_frame.set_register("rdi", args[0]); }
        if args.len() > 1 { call_frame.set_register("rsi", args[1]); }
        if args.len() > 2 { call_frame.set_register("rdx", args[2]); }
        if args.len() > 3 { call_frame.set_register("rcx", args[3]); }
        if args.len() > 4 { call_frame.set_register("r8", args[4]); }
        if args.len() > 5 { call_frame.set_register("r9", args[5]); }

        // Set stack arguments
        for i in 6..args.len() {
            call_frame.push_stack(args[i]);
        }

        Ok(())
    }

    /// Set up AArch64 call frame
    fn setup_aarch64_frame(&self, call_frame: &mut CallFrame, _register_mapping: &RegisterMapping,
                          args: &[usize]) -> Result<()> {
        // AArch64 AAPCS:
        // - First 8 args: X0-X7
        // - Remaining args: stack
        // - Stack must be 16-byte aligned

        call_frame.clear_registers();

        // Set register arguments
        for i in 0..core::cmp::min(args.len(), 8) {
            let reg_name = format!("x{}", i);
            call_frame.set_register(&reg_name, args[i]);
        }

        // Set stack arguments
        for i in 8..args.len() {
            call_frame.push_stack(args[i]);
        }

        Ok(())
    }

    /// Set up AArch32 call frame
    fn setup_aarch32_frame(&self, call_frame: &mut CallFrame, _register_mapping: &RegisterMapping,
                          args: &[usize]) -> Result<()> {
        // AArch32 AAPCS:
        // - First 4 args: R0-R3
        // - Remaining args: stack
        // - Stack must be 8-byte aligned

        call_frame.clear_registers();

        // Set register arguments
        for i in 0..core::cmp::min(args.len(), 4) {
            let reg_name = format!("r{}", i);
            call_frame.set_register(&reg_name, args[i]);
        }

        // Set stack arguments
        for i in 4..args.len() {
            call_frame.push_stack(args[i]);
        }

        Ok(())
    }

    /// Set up RISC-V call frame
    fn setup_riscv_frame(&self, call_frame: &mut CallFrame, _register_mapping: &RegisterMapping,
                         args: &[usize]) -> Result<()> {
        // RISC-V calling convention:
        // - First 8 args: a0-a7
        // - Remaining args: stack
        // - Stack must be 16-byte aligned

        call_frame.clear_registers();

        // Set register arguments
        for i in 0..core::cmp::min(args.len(), 8) {
            let reg_name = format!("a{}", i);
            call_frame.set_register(&reg_name, args[i]);
        }

        // Set stack arguments
        for i in 8..args.len() {
            call_frame.push_stack(args[i]);
        }

        Ok(())
    }

    /// Initialize register mappings for different architectures
    fn init_register_mappings(&mut self) {
        // x86_64 register mapping
        self.register_mappings.insert(Architecture::X86_64, RegisterMapping {
            integer_registers: vec![
                "rax".to_string(), "rbx".to_string(), "rcx".to_string(), "rdx".to_string(),
                "rsi".to_string(), "rdi".to_string(), "rbp".to_string(), "rsp".to_string(),
                "r8".to_string(), "r9".to_string(), "r10".to_string(), "r11".to_string(),
                "r12".to_string(), "r13".to_string(), "r14".to_string(), "r15".to_string(),
            ],
            floating_registers: vec![
                "xmm0".to_string(), "xmm1".to_string(), "xmm2".to_string(), "xmm3".to_string(),
                "xmm4".to_string(), "xmm5".to_string(), "xmm6".to_string(), "xmm7".to_string(),
                "xmm8".to_string(), "xmm9".to_string(), "xmm10".to_string(), "xmm11".to_string(),
                "xmm12".to_string(), "xmm13".to_string(), "xmm14".to_string(), "xmm15".to_string(),
            ],
            special_registers: vec![
                "rip".to_string(), "rflags".to_string(),
            ],
        });

        // AArch64 register mapping
        self.register_mappings.insert(Architecture::AArch64, RegisterMapping {
            integer_registers: (0..31).map(|i| format!("x{}", i)).collect(),
            floating_registers: (0..31).map(|i| format!("v{}", i)).collect(),
            special_registers: vec![
                "sp".to_string(), "pc".to_string(), "pstate".to_string(),
            ],
        });

        // RISC-V register mapping
        self.register_mappings.insert(Architecture::RiscV64, RegisterMapping {
            integer_registers: vec![
                "zero".to_string(), "ra".to_string(), "sp".to_string(), "gp".to_string(),
                "tp".to_string(), "t0".to_string(), "t1".to_string(), "t2".to_string(),
                "s0".to_string(), "s1".to_string(), "a0".to_string(), "a1".to_string(),
                "a2".to_string(), "a3".to_string(), "a4".to_string(), "a5".to_string(),
                "a6".to_string(), "a7".to_string(), "s2".to_string(), "s3".to_string(),
                "s4".to_string(), "s5".to_string(), "s6".to_string(), "s7".to_string(),
                "s8".to_string(), "s9".to_string(), "s10".to_string(), "s11".to_string(),
                "t3".to_string(), "t4".to_string(), "t5".to_string(), "t6".to_string(),
            ],
            floating_registers: (0..32).map(|i| format!("f{}", i)).collect(),
            special_registers: vec![
                "pc".to_string(),
            ],
        });
    }

    /// Initialize stack layout specifications
    fn init_stack_layouts(&mut self) {
        // Microsoft x64 stack layout
        self.stack_layouts.insert(CallingConvention::MicrosoftX64, StackLayout {
            alignment: 16,
            shadow_space: 32,
            return_address_size: 8,
            grows_down: true,
        });

        // System V AMD64 stack layout
        self.stack_layouts.insert(CallingConvention::SystemVAMD64, StackLayout {
            alignment: 16,
            shadow_space: 0,
            return_address_size: 8,
            grows_down: true,
        });

        // AArch64 stack layout
        self.stack_layouts.insert(CallingConvention::AArch64AAPCS, StackLayout {
            alignment: 16,
            shadow_space: 0,
            return_address_size: 8,
            grows_down: true,
        });

        // AArch32 stack layout
        self.stack_layouts.insert(CallingConvention::AArch32AAPCS, StackLayout {
            alignment: 8,
            shadow_space: 0,
            return_address_size: 4,
            grows_down: true,
        });

        // RISC-V stack layout
        self.stack_layouts.insert(CallingConvention::RiscV, StackLayout {
            alignment: 16,
            shadow_space: 0,
            return_address_size: 8,
            grows_down: true,
        });
    }
}

/// Trait for argument converters
pub trait ArgumentConverter: Send + Sync {
    /// Convert a single argument
    fn convert_argument(&self, arg: usize, arg_index: usize) -> Result<usize>;
}

/// Default argument converter (no conversion)
struct DefaultArgumentConverter;

impl ArgumentConverter for DefaultArgumentConverter {
    fn convert_argument(&self, arg: usize, _arg_index: usize) -> Result<usize> {
        Ok(arg)
    }
}

/// Register mapping for an architecture
#[derive(Debug, Clone)]
pub struct RegisterMapping {
    /// Integer register names
    pub integer_registers: Vec<String>,
    /// Floating-point register names
    pub floating_registers: Vec<String>,
    /// Special register names (PC, flags, etc.)
    pub special_registers: Vec<String>,
}

/// Stack layout specification
#[derive(Debug, Clone)]
pub struct StackLayout {
    /// Stack alignment requirement (in bytes)
    pub alignment: usize,
    /// Shadow space size (for Windows x64)
    pub shadow_space: usize,
    /// Size of return address on stack
    pub return_address_size: usize,
    /// Whether stack grows downward
    pub grows_down: bool,
}

/// Function call frame
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// Register values
    pub registers: HashMap<String, usize, DefaultHasherBuilder>,
    /// Stack contents (bottom to top)
    pub stack: Vec<usize>,
    /// Stack pointer value
    pub sp: usize,
    /// Instruction pointer value
    pub ip: usize,
    /// Stack layout specification
    pub stack_layout: Option<StackLayout>,
}

impl CallFrame {
    /// Create a new call frame
    pub fn new() -> Self {
        Self {
            registers: HashMap::with_hasher(DefaultHasherBuilder),
            stack: Vec::new(),
            sp: 0,
            ip: 0,
            stack_layout: None,
        }
    }

    /// Clear all registers
    pub fn clear_registers(&mut self) {
        self.registers.clear();
    }

    /// Set a register value
    pub fn set_register(&mut self, name: &str, value: usize) {
        self.registers.insert(name.to_string(), value);
    }

    /// Get a register value
    pub fn get_register(&self, name: &str) -> Option<usize> {
        self.registers.get(name).cloned()
    }

    /// Push value onto stack
    pub fn push_stack(&mut self, value: usize) {
        self.stack.push(value);
        if let Some(layout) = &self.stack_layout {
            self.sp = self.sp.wrapping_sub(core::mem::size_of::<usize>());
            // Align stack if needed
            if layout.alignment > 0 && self.sp % layout.alignment != 0 {
                self.sp = self.sp - (self.sp % layout.alignment);
            }
        }
    }

    /// Pop value from stack
    pub fn pop_stack(&mut self) -> Option<usize> {
        let value = self.stack.pop();
        if let Some(layout) = &self.stack_layout {
            self.sp = self.sp.wrapping_add(core::mem::size_of::<usize>());
        }
        value
    }

    /// Align stack according to layout
    pub fn align_stack(&mut self) {
        if let Some(layout) = &self.stack_layout {
            if layout.alignment > 0 && self.sp % layout.alignment != 0 {
                let alignment_offset = layout.alignment - (self.sp % layout.alignment);
                self.sp = self.sp.wrapping_sub(alignment_offset);
                // Push padding if needed
                for _ in 0..(alignment_offset / core::mem::size_of::<usize>()) {
                    self.stack.push(0);
                }
            }
        }
    }

    /// Get stack frame size
    pub fn frame_size(&self) -> usize {
        self.stack.len() * core::mem::size_of::<usize>()
    }
}

impl Default for CallFrame {
    fn default() -> Self {
        Self::new()
    }
}

/// ABI conversion utilities
pub mod utils {
    use super::*;

    /// Convert between different endiannesses
    pub fn convert_endianess(value: usize, from_be: bool, to_be: bool) -> usize {
        if from_be == to_be {
            return value;
        }

        // Simple byte swap for 64-bit values
        // In a real implementation, this would be more sophisticated
        value.swap_bytes()
    }

    /// Convert file descriptor between platforms
    pub fn convert_fd(fd: usize, from_platform: TargetPlatform, _to_platform: TargetPlatform) -> usize {
        // Different platforms have different fd conventions
        // For now, return the same fd - real implementation would map them
        match from_platform {
            TargetPlatform::Windows => {
                // Windows handles are different from Unix fds
                if fd <= 2 {
                    // Map standard handles
                    match fd {
                        0 => 0, // stdin
                        1 => 1, // stdout
                        2 => 2, // stderr
                        _ => fd,
                    }
                } else {
                    fd + 1000 // Offset to avoid conflicts
                }
            }
            _ => fd,
        }
    }

    /// Convert file permissions between platforms
    pub fn convert_permissions(permissions: u32, from_platform: TargetPlatform, _to_platform: TargetPlatform) -> u32 {
        match from_platform {
            TargetPlatform::Windows => {
                // Convert Windows attributes to Unix permissions
                let mut unix_perms = 0o644; // Default
                if permissions & 0x01 != 0 { unix_perms |= 0o111; } // Execute
                if permissions & 0x02 != 0 { unix_perms |= 0o222; } // Write
                if permissions & 0x04 != 0 { unix_perms |= 0o444; } // Read
                unix_perms
            }
            _ => permissions,
        }
    }

    /// Convert error codes between platforms
    pub fn convert_error_code(error: i32, from_platform: TargetPlatform, to_platform: TargetPlatform) -> i32 {
        // Map error codes between platforms
        match (from_platform, to_platform) {
            (TargetPlatform::Windows, TargetPlatform::Linux) => {
                // Windows to Linux error mapping
                match error {
                    2 => 2,   // ERROR_FILE_NOT_FOUND -> ENOENT
                    3 => 2,   // ERROR_PATH_NOT_FOUND -> ENOENT
                    5 => 13,  // ERROR_ACCESS_DENIED -> EACCES
                    6 => 9,   // ERROR_INVALID_HANDLE -> EBADF
                    _ => 1,   // Default to EPERM
                }
            }
            _ => error,
        }
    }
}

/// Create a new ABI converter
pub fn create_abi_converter() -> AbiConverter {
    AbiConverter::new()
}