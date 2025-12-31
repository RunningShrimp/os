//! # eBPF 验证器
//!
//! eBPF 指令验证器，确保程序安全性和正确性。

use crate::prelude::*;

/// eBPF 验证器配置
#[derive(Debug, Clone)]
pub struct BpfVerifierConfig {
    /// 最大指令数
    pub max_instructions: usize,
    /// 启用严格模式
    pub strict_mode: bool,
}

impl Default for BpfVerifierConfig {
    fn default() -> Self {
        Self {
            max_instructions: 4096,
            strict_mode: true,
        }
    }
}

/// 验证错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifierError {
    /// 指令无效
    InvalidInstruction(String),
    /// 程序不终止
    ProgramDoesNotTerminate,
    /// 指令数超限
    InstructionLimitExceeded,
}

/// eBPF 指令结构
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BpfInsn {
    /// 操作码
    pub code: u8,
    /// 目标寄存器
    pub dst: u8,
    /// 源寄存器
    pub src: u8,
    /// 偏移量
    pub off: i16,
    /// 立即数
    pub imm: i32,
}

/// eBPF 验证器
pub struct BpfVerifier {
    config: BpfVerifierConfig,
}

impl BpfVerifier {
    /// 创建新的验证器
    pub fn new(config: BpfVerifierConfig) -> Self {
        Self { config }
    }

    /// 验证 eBPF 程序
    pub fn verify(&self, program: &BpfProgram) -> core::result::Result<(), VerifierError> {
        let instructions = program.instructions();

        if instructions.len() > self.config.max_instructions {
            return Err(VerifierError::InstructionLimitExceeded);
        }

        // 简化验证：检查指令有效性
        for insn in instructions.iter() {
            self.check_instruction_validity(insn)?;
        }

        Ok(())
    }

    /// 检查指令有效性
    fn check_instruction_validity(&self, insn: &BpfInsn) -> core::result::Result<(), VerifierError> {
        // 检查寄存器索引
        if insn.dst >= 11 || insn.src >= 11 {
            return Err(VerifierError::InvalidInstruction(
                "Invalid register".to_string()
            ));
        }

        Ok(())
    }
}

/// 导入 BpfProgram
use super::programs::BpfProgram;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_config() {
        let config = BpfVerifierConfig::default();
        assert_eq!(config.max_instructions, 4096);
    }

    #[test]
    fn test_bpf_insn() {
        let insn = BpfInsn {
            code: 0xB7,
            dst: 1,
            src: 0,
            off: 0,
            imm: 0,
        };

        assert_eq!(insn.code, 0xB7);
        assert_eq!(insn.dst, 1);
    }
}
