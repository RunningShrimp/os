//! # eBPF 程序管理
//!
//! eBPF 程序的加载、卸载和执行。

use crate::prelude::*;

/// eBPF 程序类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfProgramType {
    /// 套接字过滤器
    SocketFilter,
    /// Kprobe
    Kprobe,
    /// Tracepoint
    Tracepoint,
    /// XDP
    Xdp,
}

/// 程序状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfProgramState {
    /// 未加载
    Unloaded,
    /// 已加载
    Loaded,
    /// 运行中
    Running,
}

/// eBPF 程序
pub struct BpfProgram {
    /// 程序类型
    prog_type: BpfProgramType,
    /// 指令
    instructions: Vec<super::verifier::BpfInsn>,
    /// 程序名称
    name: String,
    /// 程序状态
    state: Mutex<BpfProgramState>,
}

impl BpfProgram {
    /// 创建新的 eBPF 程序
    pub fn new(
        prog_type: BpfProgramType,
        instructions: &[u8],
        name: &str,
    ) -> core::result::Result<Self, crate::bpf::BpfError> {
        // 检查指令长度
        if instructions.len() % 8 != 0 {
            return Err(crate::bpf::BpfError::ProgramTooLarge);
        }

        let insn_count = instructions.len() / 8;
        if insn_count > 4096 {
            return Err(crate::bpf::BpfError::InstructionLimitExceeded);
        }

        // 解析指令
        let mut insns = Vec::with_capacity(insn_count);
        for i in 0..insn_count {
            let offset = i * 8;
            let insn = super::verifier::BpfInsn {
                code: instructions[offset],
                dst: instructions[offset + 1],
                src: instructions[offset + 2],
                off: i16::from_le_bytes([
                    instructions[offset + 3],
                    instructions[offset + 4],
                ]),
                imm: i32::from_le_bytes([
                    instructions[offset + 5],
                    instructions[offset + 6],
                    instructions[offset + 7],
                    instructions[offset + 8],
                ]),
            };
            insns.push(insn);
        }

        Ok(Self {
            prog_type,
            instructions: insns,
            name: name.to_string(),
            state: Mutex::new(BpfProgramState::Unloaded),
        })
    }

    /// 获取程序类型
    pub fn prog_type(&self) -> BpfProgramType {
        self.prog_type
    }

    /// 获取指令
    pub fn instructions(&self) -> &[super::verifier::BpfInsn] {
        &self.instructions
    }

    /// 获取指令数量
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// 获取程序名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取程序状态
    pub fn state(&self) -> BpfProgramState {
        *self.state.lock()
    }

    /// 设置程序状态
    pub fn set_state(&self, state: BpfProgramState) {
        *self.state.lock() = state;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prog_type() {
        let prog_type = BpfProgramType::SocketFilter;
        assert_eq!(prog_type, BpfProgramType::SocketFilter);
    }

    #[test]
    fn test_bpf_program_create() {
        let instructions: Vec<u8> = vec![
            0xB7, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // mov r1, 0
            0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // exit
        ];

        let program = BpfProgram::new(BpfProgramType::SocketFilter, &instructions, "test");
        assert!(program.is_ok());

        let program = program.unwrap();
        assert_eq!(program.instruction_count(), 2);
        assert_eq!(program.name(), "test");
    }
}
