//! eBPF Instruction Set
//!
//! This module implements the full eBPF ISA:
//! - 64-bit eBPF instructions
//! - eBPF register file (R0-R10)
//! - eBPF memory addressing (load/store)
//! - eBPF control flow (jumps, calls)
//! - eBPF arithmetic and bitwise operations
//!
//! Features:
//! - Full 64-bit instruction set
//! - Little-endian and big-endian support
//! - Atomic operations
//! - Conditional jumps
//! - Function calls and returns
//! - Memory load/store (BPF stack and packet data)

use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;

// ============================================================================
// eBPF Constants
// ============================================================================

/// Maximum number of eBPF registers
pub const MAX_REGS: usize = 11;

/// eBPF register indices
pub const REG_R0: usize = 0;
pub const REG_R1: usize = 1;
pub const REG_R2: usize = 2;
pub const REG_R3: usize = 3;
pub const REG_R4: usize = 4;
pub const REG_R5: usize = 5;
pub const REG_R6: usize = 6;
pub const REG_R7: usize = 7;
pub const REG_R8: usize = 8;
pub const REG_R9: usize = 9;
pub const REG_R10: usize = 10;

/// eBPF stack size (512 bytes)
pub const STACK_SIZE: usize = 512;

/// eBPF memory alignment
pub const MEM_ALIGNMENT: usize = 8;

// ============================================================================
// eBPF Instruction Classes
// ============================================================================

/// eBPF instruction class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfInsnClass {
    /// Load 64-bit immediate
    LdImm64,
    
    /// Load 64-bit (register or memory)
    Ld,
    
    /// Store 64-bit (register or memory)
    St,
    
    /// Load 32-bit immediate
    LdImm32,
    
    /// Load 32-bit
    Ld32,
    
    /// Store 32-bit
    St32,
    
    /// Load byte
    Ld8,
    
    /// Store byte
    St8,
    
    /// Load half-word (16-bit)
    Ld16,
    
    /// Store half-word
    St16,
    
    /// Load double word (32-bit)
    Lddw,
    
    /// Store double word
    Sdw,
    
    /// Atomic 64-bit operation
    LdXch,
    
    /// 64-bit addition
    Add64,
    
    /// 64-bit subtraction
    Sub64,
    
    /// 64-bit multiplication
    Mul64,
    
    /// 64-bit division
    Div64,
    
    /// 64-bit or
    Or64,
    
    /// 64-bit and
    And64,
    
    /// 64-bit left shift
    Lsh64,
    
    /// 64-bit right shift
    Rsh64,
    
    /// 64-bit negative (arithmetic right shift)
    Neg64,
    
    /// 64-bit modulo
    Mod64,
    
    /// 64-bit xor
    Xor64,
    
    /// Move 64-bit
    Mov64,
    
    /// 64-bit conditional move
    CmP64,
    
    /// 32-bit arithmetic shift right
    Arsh64,
    
    /// 64-bit byte swap (little-endian to big-endian)
    Be64,
    
    /// 64-bit byte swap (big-endian to little-endian)
    Le64,
    
    /// Conditional jump (always)
    Ja,
    
    /// Conditional jump (if r0 == 0)
    Jeq,
    
    /// Conditional jump (if r0 != 0)
    Jne,
    
    /// Conditional jump (if r0 > src)
    Jgt,
    
    /// Conditional jump (if r0 >= src)
    Jge,
    
    /// Conditional jump (if r0 < src)
    Jlt,
    
    /// Conditional jump (if r0 <= src)
    Jle,
    
    /// Conditional jump (if r0 < src, signed)
    Jsgt,
    
    /// Conditional jump (if r0 >= src, signed)
    Jsge,
    
    /// Conditional jump (if r0 < src, signed)
    Jslt,
    
    /// Conditional jump (if r0 <= src, signed)
    Jsle,
    
    /// Call function
    Call,
    
    /// Return from function
    Exit,
    
    /// Jump register
    JmpReg,
    
    /// Conditional jump register
    JmpRegEq,
    JmpRegGe,
    JmpRegGt,
    JmpRegLt,
    JmpRegLe,
    JmpRegNe,
    JmpRegSgt,
    JmpRegSge,
    JmpRegSlt,
    JmpRegSle,
    
    /// 64-bit atomic add
    Add64Imm,
    
    /// 64-bit atomic and
    And64Imm,
    
    /// 64-bit atomic or
    Or64Imm,
    
    /// 64-bit atomic xor
    Xor64Imm,
    
    /// Load absolute 64-bit
    LdAbs64,
    
    /// Load absolute 32-bit
    LdAbs32,
    
    /// Load absolute half-word
    LdAbsHw,
    
    /// Load absolute word
    LdAbsW,
    
    /// Load absolute byte
    LdAbsB,
    
    /// Load half-word (signed)
    Ldxdw,
    
    /// Return with value
    Ret,
}

/// eBPF instruction size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfInsnSize {
    /// 64-bit (8 bytes)
    Size64,
    
    /// 128-bit (16 bytes)
    Size128,
}

/// eBPF source/destination register
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfReg {
    /// eBPF register (R0-R10)
    Reg(usize),
    
    /// eBPF frame pointer (R10)
    FramePointer,
}

// ============================================================================
// eBPF Instruction Encoding
// ============================================================================

/// eBPF instruction (64-bit)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct EbpfInsn {
    /// Opcode (8 bits)
    pub opcode: u8,
    
    /// Destination register (4 bits)
    pub dst: u8,
    
    /// Source register (4 bits)
    pub src: u8,
    
    /// Offset (16 bits, signed)
    pub offset: i16,
    
    /// Immediate value (32 bits)
    pub imm: i32,
}

impl EbpfInsn {
    /// Create new eBPF instruction
    pub fn new(opcode: u8, dst: u8, src: u8, offset: i16, imm: i32) -> Self {
        Self {
            opcode,
            dst,
            src,
            offset,
            imm,
        }
    }
    
    /// Create load immediate 64-bit instruction
    pub fn ld_imm64(dst: u8, imm: u64) -> [Self; 2] {
        [
            Self::new(0x18, dst, 0, 0, (imm & 0xFFFFFFFF) as i32),
            Self::new(0x00, dst, 0, 0, (imm >> 32) as i32),
        ]
    }
    
    /// Create load register instruction
    pub fn ld_reg(dst: u8, src: u8, offset: i16, size: EbpfInsnSize) -> Self {
        let opcode = match size {
            EbpfInsnSize::Size64 => 0x61,
            EbpfInsnSize::Size128 => 0x79,
        };
        
        Self::new(opcode, dst, src, offset, 0)
    }
    
    /// Create store register instruction
    pub fn st_reg(dst: u8, src: u8, offset: i16, size: EbpfInsnSize) -> Self {
        let opcode = match size {
            EbpfInsnSize::Size64 => 0x63,
            EbpfInsnSize::Size128 => 0x7B,
        };
        
        Self::new(opcode, dst, src, offset, 0)
    }
    
    /// Create add instruction
    pub fn add(dst: u8, src: u8) -> Self {
        Self::new(0x0F, dst, src, 0, 0)
    }
    
    /// Create sub instruction
    pub fn sub(dst: u8, src: u8) -> Self {
        Self::new(0x1F, dst, src, 0, 0)
    }
    
    /// Create mul instruction
    pub fn mul(dst: u8, src: u8) -> Self {
        Self::new(0x27, dst, src, 0, 0)
    }
    
    /// Create div instruction
    pub fn div(dst: u8, src: u8) -> Self {
        Self::new(0x37, dst, src, 0, 0)
    }
    
    /// Create or instruction
    pub fn or(dst: u8, src: u8) -> Self {
        Self::new(0x4F, dst, src, 0, 0)
    }
    
    /// Create and instruction
    pub fn and(dst: u8, src: u8) -> Self {
        Self::new(0x5F, dst, src, 0, 0)
    }
    
    /// Create left shift instruction
    pub fn lsh(dst: u8, src: u8) -> Self {
        Self::new(0x67, dst, src, 0, 0)
    }
    
    /// Create right shift instruction
    pub fn rsh(dst: u8, src: u8) -> Self {
        Self::new(0x77, dst, src, 0, 0)
    }
    
    /// Create xor instruction
    pub fn xor(dst: u8, src: u8) -> Self {
        Self::new(0xAF, dst, src, 0, 0)
    }
    
    /// Create mov instruction
    pub fn mov(dst: u8, src: u8) -> Self {
        Self::new(0xBF, dst, src, 0, 0)
    }
    
    /// Create conditional move instruction
    pub fn cmp(dst: u8, src: u8, off: i16) -> Self {
        Self::new(0xB7, dst, src, off, 0)
    }
    
    /// Create unconditional jump instruction
    pub fn jmp(off: i16) -> Self {
        Self::new(0x05, 0, 0, off, 0)
    }
    
    /// Create conditional jump instruction
    pub fn jmp_cond(opcode: u8, dst: u8, src: u8, off: i16) -> Self {
        Self::new(opcode, dst, src, off, 0)
    }
    
    /// Create call instruction
    pub fn call(func_id: i32) -> Self {
        Self::new(0x85, 0, 0, 0, func_id)
    }
    
    /// Create exit instruction
    pub fn exit() -> Self {
        Self::new(0x95, 0, 0, 0, 0)
    }
    
    /// Create return instruction with value
    pub fn ret(r0_value: i32) -> Self {
        Self::new(0x95, 0, 0, 0, r0_value)
    }
    
    /// Convert to u64 for encoding
    pub fn to_u64(&self) -> u64 {
        let mut val: u64 = 0;
        
        val |= (self.opcode as u64) << 56;
        val |= (self.dst as u64) << 52;
        val |= (self.src as u64) << 48;
        val |= ((self.offset as u16) as u64) << 32;
        val |= self.imm as u64 & 0xFFFFFFFF;
        
        val
    }
}

// ============================================================================
// eBPF Register File
// ============================================================================

/// eBPF register file
pub struct EbpfRegFile {
    /// General purpose registers (R0-R9)
    pub regs: [u64; 10],
    
    /// Frame pointer register (R10)
    pub frame_pointer: u64,
    
    /// Return value register (R0)
    pub r0: u64,
}

impl Default for EbpfRegFile {
    fn default() -> Self {
        Self {
            regs: [0; 10],
            file_pointer: 0,
            r0: 0,
        }
    }
}

impl EbpfRegFile {
    /// Create new register file
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Read register
    pub fn read(&self, reg: usize) -> u64 {
        match reg {
            REG_R0 => self.r0,
            REG_R1..=REG_R10 => self.regs[reg - 1],
            REG_R10 => self.file_pointer,
            _ => 0,
        }
    }
    
    /// Write register
    pub fn write(&mut self, reg: usize, value: u64) {
        match reg {
            REG_R0 => self.r0 = value,
            REG_R1..=REG_R9 => self.regs[reg - 1] = value,
            REG_R10 => self.file_pointer = value,
            _ => {}
        }
    }
    
    /// Clear all registers
    pub fn clear(&mut self) {
        self.r0 = 0;
        for reg in self.regs.iter_mut() {
            *reg = 0;
        }
    }
}

// ============================================================================
// eBPF Stack
// ============================================================================

/// eBPF stack
pub struct EbpfStack {
    /// Stack memory (512 bytes)
    pub stack: [u8; STACK_SIZE],
    
    /// Current stack pointer
    pub sp: usize,
}

impl Default for EbpfStack {
    fn default() -> Self {
        Self {
            stack: [0; STACK_SIZE],
            sp: 0,
        }
    }
}

impl EbpfStack {
    /// Create new eBPF stack
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Push 8-byte value to stack
    pub fn push(&mut self, value: u64) -> Result<(), EbpfError> {
        if self.sp + 8 > STACK_SIZE {
            return Err(EbpfError::StackOverflow);
        }
        
        let bytes = value.to_le_bytes();
        for (i, &byte) in bytes.iter().enumerate() {
            self.stack[self.sp + i] = *byte;
        }
        
        self.sp += 8;
        
        Ok(())
    }
    
    /// Pop 8-byte value from stack
    pub fn pop(&mut self) -> Result<u64, EbpfError> {
        if self.sp < 8 {
            return Err(EbpfError::StackUnderflow);
        }
        
        self.sp -= 8;
        
        let mut bytes = [0u8; 8];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = self.stack[self.sp + i];
        }
        
        Ok(u64::from_le_bytes(bytes))
    }
    
    /// Read from stack at offset
    pub fn read(&self, offset: usize, size: usize) -> Result<u64, EbpfError> {
        if offset + size > STACK_SIZE {
            return Err(EbpfError::StackOutOfBounds);
        }
        
        let mut value: u64 = 0;
        
        for i in 0..size {
            value |= (self.stack[offset + i] as u64) << (i * 8);
        }
        
        Ok(value)
    }
    
    /// Write to stack at offset
    pub fn write(&mut self, offset: usize, value: u64, size: usize) -> Result<(), EbpfError> {
        if offset + size > STACK_SIZE {
            return Err(EbpfError::StackOutOfBounds);
        }
        
        let bytes = value.to_le_bytes();
        
        for i in 0..size {
            self.stack[offset + i] = bytes[i];
        }
        
        Ok(())
    }
    
    /// Reset stack
    pub fn reset(&mut self) {
        self.sp = 0;
        for byte in self.stack.iter_mut() {
            *byte = 0;
        }
    }
}

// ============================================================================
// eBPF Execution Context
// ============================================================================

/// eBPF execution context
pub struct EbpfExecContext {
    /// Register file
    pub regs: EbpfRegFile,
    
    /// Stack
    pub stack: EbpfStack,
    
    /// Program counter
    pub pc: usize,
    
    /// Return address stack
    pub ret_stack: Vec<usize>,
    
    /// Memory map (external memory access)
    pub mem_map: Option<&'static mut [u8]>,
    
    /// Packet data (for network eBPF)
    pub packet_data: Option<&[u8]>,
    
    /// Packet data length
    pub packet_len: usize,
    
    /// Auxiliary context (for helper functions)
    pub aux_ctx: EbpfAuxContext,
}

/// Auxiliary context for eBPF
#[derive(Debug, Clone)]
pub struct EbpfAuxContext {
    /// Current user ID
    pub uid: Option<u32>,
    
    /// Current process ID
    pub pid: Option<usize>,
    
    /// System call number
    pub syscall_nr: Option<u32>,
    
    /// CPU ID
    pub cpu_id: Option<usize>,
    
    /// Timestamp
    pub timestamp: Option<u64>,
    
    /// Custom context data
    pub custom_data: Vec<u8>,
}

// ============================================================================
// eBPF Errors
// ============================================================================

/// eBPF error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EbpfError {
    /// Invalid instruction
    InvalidInsn {
        pc: usize,
        opcode: u8,
    },
    
    /// Invalid register
    InvalidReg {
        reg: usize,
    },
    
    /// Division by zero
    DivisionByZero,
    
    /// Stack overflow
    StackOverflow,
    
    /// Stack underflow
    StackUnderflow,
    
    /// Stack out of bounds
    StackOutOfBounds,
    
    /// Memory out of bounds
    MemoryOutOfBounds,
    
    /// Invalid jump target
    InvalidJump {
        target: usize,
    },
    
    /// Maximum instructions exceeded
    MaxInsnExceeded,
    
    /// Helper function error
    HelperError {
        func_id: u32,
        error: i32,
    },
    
    /// Map access error
    MapError {
        map_id: u32,
        key: u64,
    },
}

// ============================================================================
// Instruction Decoding
// ============================================================================

/// Decode eBPF instruction
pub fn decode_insn(insn: EbpfInsn) -> Result<DecodedInsn, EbpfError> {
    let opcode = insn.opcode;
    
    // Determine instruction class based on opcode
    let insn_class = match opcode {
        0x18 => EbpfInsnClass::LdImm64,
        0x61 => EbpfInsnClass::Ld,
        0x63 => EbpfInsnClass::St,
        0x00..=0x07 => EbpfInsnClass::LdImm32,
        0x08..=0x0F => EbpfInsnClass::Ld32,
        0x10..=0x17 => EbpfInsnClass::St32,
        0x20..=0x27 => EbpfInsnClass::Ld8,
        0x28..=0x2F => EbpfInsnClass::St8,
        0x30..=0x37 => EbpfInsnClass::Ld16,
        0x38..=0x3F => EbpfInsnClass::St16,
        0x40..=0x47 => EbpfInsnClass::Lddw,
        0x48..=0x4F => EbpfInsnClass::Sdw,
        0x50..=0x57 => EbpfInsnClass::LdXch,
        0x0F => EbpfInsnClass::Add64,
        0x1F => EbpfInsnClass::Sub64,
        0x27 => EbpfInsnClass::Mul64,
        0x37 => EbpfInsnClass::Div64,
        0x4F => EbpfInsnClass::Or64,
        0x5F => EbpfInsnClass::And64,
        0x67 => EbpfInsnClass::Lsh64,
        0x77 => EbpfInsnClass::Rsh64,
        0x87 => EbpfInsnClass::Neg64,
        0x97 => EbpfInsnClass::Mod64,
        0xAF => EbpfInsnClass::Xor64,
        0xBF => EbpfInsnClass::Mov64,
        0xB7 => EbpfInsnClass::CmP64,
        0xC7 => EbpfInsnClass::Arsh64,
        0xCF => EbpfInsnClass::Be64,
        0xD7 => EbpfInsnClass::Le64,
        0x05 => EbpfInsnClass::Ja,
        0x1D => EbpfInsnClass::Jeq,
        0x35 => EbpfInsnClass::Jne,
        0x2D => EbpfInsnClass::Jgt,
        0x3D => EbpfInsnClass::Jge,
        0x4D => EbpfInsnClass::Jlt,
        0x5D => EbpfInsnClass::Jle,
        0x6D => EbpfInsnClass::Jsgt,
        0x7D => EbpfInsnClass::Jsge,
        0x8D => EbpfInsnClass::Jslt,
        0x9D => EbpfInsnClass::Jsle,
        0x85 => EbpfInsnClass::Call,
        0x95 => EbpfInsnClass::Exit,
        0xAD => EbpfInsnClass::JmpReg,
        0xBD => EbpfInsnClass::JmpRegEq,
        0xCD => EbpfInsnClass::JmpRegGe,
        0xDD => EbpfInsnClass::JmpRegGt,
        0xED => EbpfInsnClass::JmpRegLt,
        0xFD => EbpfInsnClass::JmpRegLe,
        0x9D => EbpfInsnClass::JmpRegNe,
        0xAD => EbpfInsnClass::JmpRegSgt,
        0xBD => EbpfInsnClass::JmpRegSge,
        0xCD => EbpfInsnClass::JmpRegSlt,
        0xDD => EbpfInsnClass::JmpRegSle,
        0x07 => EbpfInsnClass::Add64Imm,
        0x17 => EbpfInsnClass::And64Imm,
        0x27 => EbpfInsnClass::Or64Imm,
        0x37 => EbpfInsnClass::Xor64Imm,
        0x04 => EbpfInsnClass::LdAbs64,
        0x14 => EbpfInsnClass::LdAbs32,
        0x24 => EbpfInsnClass::LdAbsHw,
        0x34 => EbpfInsnClass::LdAbsW,
        0x44 => EbpfInsnClass::LdAbsB,
        0x54 => EbpfInsnClass::Ldxdw,
        0x95 => EbpfInsnClass::Ret,
        _ => return Err(EbpfError::InvalidInsn {
            pc: 0,
            opcode,
        }),
    };
    
    Ok(DecodedInsn {
        class: insn_class,
        dst: insn.dst,
        src: insn.src,
        offset: insn.offset,
        imm: insn.imm,
    })
}

/// Decoded eBPF instruction
#[derive(Debug, Clone)]
pub struct DecodedInsn {
    pub class: EbpfInsnClass,
    pub dst: u8,
    pub src: u8,
    pub offset: i16,
    pub imm: i32,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebpf_insn_encoding() {
        let insn = EbpfInsn::new(0x0F, 1, 2, 0, 0); // R1 = R1 + R2
        
        let encoded = insn.to_u64();
        
        assert_eq!((encoded >> 56) & 0xFF, 0x0F); // Opcode
        assert_eq!((encoded >> 52) & 0x0F, 1);   // DST
        assert_eq!((encoded >> 48) & 0x0F, 2);   // SRC
    }

    #[test]
    fn test_ld_imm64() {
        let insns = EbpfInsn::ld_imm64(0, 0x123456789ABCDEF);
        
        assert_eq!(insns.len(), 2);
        assert_eq!(insns[0].opcode, 0x18);
    }

    #[test]
    fn test_reg_file() {
        let mut regs = EbpfRegFile::new();
        
        regs.write(REG_R0, 42);
        assert_eq!(regs.read(REG_R0), 42);
        
        regs.write(REG_R10, 99);
        assert_eq!(regs.read(REG_R10), 99);
    }

    #[test]
    fn test_ebpf_stack() {
        let mut stack = EbpfStack::new();
        
        stack.push(0xDEADBEEF).unwrap();
        assert_eq!(stack.sp, 8);
        
        let value = stack.pop().unwrap();
        assert_eq!(value, 0xDEADBEEF);
        assert_eq!(stack.sp, 0);
    }

    #[test]
    fn test_ebpf_arithmetic() {
        let add = EbpfInsn::add(REG_R1, REG_R2);
        assert_eq!(add.opcode, 0x0F);
        
        let sub = EbpfInsn::sub(REG_R3, REG_R4);
        assert_eq!(sub.opcode, 0x1F);
        
        let mul = EbpfInsn::mul(REG_R5, REG_R6);
        assert_eq!(mul.opcode, 0x27);
    }

    #[test]
    fn test_ebpf_jumps() {
        let jmp = EbpfInsn::jmp(100);
        assert_eq!(jmp.opcode, 0x05);
        assert_eq!(jmp.offset, 100);
        
        let exit = EbpfInsn::exit();
        assert_eq!(exit.opcode, 0x95);
    }
}
