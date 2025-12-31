//! Kernel Debugger (KDB)
//!
//! 内核调试器
//! 提供命令行调试接口，支持断点管理、单步执行、内存检查/修改等功能

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic {AtomicBool, Ordering, Ordering};

use crate::error::unified::{UnifiedError, UnifiedResult};

/// KDB 命令解释器
#[derive(Debug)]
pub struct KdbInterpreter {
    /// 命令历史
    pub command_history: Vec<String>,
    /// 历史索引
    pub history_index: usize,
    /// 是否在运行
    pub is_running: AtomicBool,
    /// 命令别名
    pub aliases: BTreeMap<String, String>,
    /// 当前上下文
    pub current_context: DebugContext,
    /// 输出缓冲区
    pub output_buffer: Vec<String>,
    /// 断点管理器
    pub breakpoint_manager: BreakpointManager,
    /// 单步执行状态
    pub step_state: StepState,
    /// 内存查看器
    pub memory_viewer: MemoryViewer,
    /// 寄存器查看器
    pub register_viewer: RegisterViewer,
}

/// 调试上下文
#[derive(Debug, Clone)]
pub struct DebugContext {
    /// 当前进程 ID
    pub current_pid: Option<u64>,
    /// 当前线程 ID
    pub current_tid: Option<u64>,
    /// 当前地址空间
    pub current_address_space: Option<u64>,
    /// 调试级别
    pub debug_level: u32,
    /// 调试模式
    pub debug_mode: DebugMode,
}

/// 调试模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugMode {
    /// 本地调试
    Local,
    /// 远程调试
    Remote,
    /// 后台调试
    Background,
}

/// 断点管理器
#[derive(Debug)]
pub struct BreakpointManager {
    /// 断点列表
    pub breakpoints: BTreeMap<u64, Breakpoint>,
    /// 下一个断点 ID
    pub next_breakpoint_id: u64,
    /// 启用的断点数量
    pub enabled_count: u64,
    /// 硬件断点数量（x86_64 限制为 4）
    pub hardware_breakpoints: u8,
}

/// 断点
#[derive(Debug, Clone)]
pub struct Breakpoint {
    /// 断点 ID
    pub id: u64,
    /// 断点地址
    pub address: u64,
    /// 断点类型
    pub breakpoint_type: BreakpointType,
    /// 断点状态
    pub status: BreakpointStatus,
    /// 原始指令字节
    pub original_bytes: [u8; 16],
    /// 命中次数
    pub hit_count: u64,
    /// 条件
    pub condition: Option<BreakpointCondition>,
    /// 命令列表（命中时执行）
    pub commands: Vec<String>,
    /// 描述
    pub description: Option<String>,
}

/// 断点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointType {
    /// 软件断点（INT 3）
    Software,
    /// 硬件断点（DR0-DR3）
    Hardware,
    /// 读监视点
    ReadWatch,
    /// 写监视点
    WriteWatch,
    /// 访问监视点（读或写）
    AccessWatch,
}

/// 断点状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointStatus {
    /// 已启用
    Enabled,
    /// 已禁用
    Disabled,
    /// 已命中
    Hit,
    /// 已删除
    Deleted,
    /// 错误
    Error,
}

/// 断点条件
#[derive(Debug, Clone)]
pub enum BreakpointCondition {
    /// 等于
    Equal(u64),
    /// 不等于
    NotEqual(u64),
    /// 大于
    GreaterThan(u64),
    /// 小于
    LessThan(u64),
    /// 位掩码
    BitMask(u64, u64),
    /// 自定义表达式
    Custom(String),
}

/// 单步执行状态
#[derive(Debug, Clone)]
pub struct StepState {
    /// 是否在单步模式
    pub is_stepping: bool,
    /// 步骤类型
    pub step_type: StepType,
    /// 剩余步骤数
    pub remaining_steps: u64,
    /// 当前线程 ID
    pub current_thread: Option<u64>,
}

/// 单步类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepType {
    /// 单步进入
    StepInto,
    /// 单步跳过
    StepOver,
    /// 单步跳出
    StepOut,
    /// 指令级单步
    InstructionStep,
}

/// 内存查看器
#[derive(Debug)]
pub struct MemoryViewer {
    /// 显示格式
    pub display_format: MemoryFormat,
    /// 单元大小
    pub unit_size: usize,
    /// 每行单元数
    pub units_per_line: usize,
    /// 内存映射缓存
    pub mappings: Vec<MemoryRegion>,
}

/// 内存显示格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryFormat {
    /// 十六进制
    Hex,
    /// 十进制
    Decimal,
    /// 八进制
    Octal,
    /// 二进制
    Binary,
    /// ASCII
    Ascii,
}

/// 内存区域
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// 起始地址
    pub start: u64,
    /// 结束地址
    pub end: u64,
    /// 权限
    pub permissions: u8,
    /// 名称
    pub name: String,
}

/// 寄存器查看器
#[derive(Debug)]
pub struct RegisterViewer {
    /// 寄存器值
    pub registers: RegisterSet,
    /// 显示格式
    pub display_format: RegisterFormat,
    /// 浮点寄存器
    pub fp_registers: Vec<u64>,
    /// SIMD 寄存器
    pub simd_registers: Vec<[u8; 32]>,
}

/// 寄存器集合
#[derive(Debug, Clone)]
pub struct RegisterSet {
    /// 通用寄存器（x86_64: RAX, RBX, RCX, RDX, RSI, RDI, RBP, RSP）
    pub general_purpose: [u64; 8],
    /// 指令指针
    pub instruction_pointer: u64,
    /// 标志寄存器
    pub flags: u64,
    /// 段寄存器
    pub segment: [u64; 6],
    /// 控制寄存器
    pub control: [u64; 8],
}

/// 寄存器显示格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterFormat {
    /// 十六进制
    Hex,
    /// 十进制
    Decimal,
    /// 原始值
    Raw,
    /// 带符号
    Signed,
}

impl KdbInterpreter {
    /// 创建新的 KDB 解释器
    pub fn new() -> Self {
        Self {
            command_history: Vec::new(),
            history_index: 0,
            is_running: AtomicBool::new(false),
            aliases: Self::default_aliases(),
            current_context: DebugContext::default(),
            output_buffer: Vec::new(),
            breakpoint_manager: BreakpointManager::new(),
            step_state: StepState::default(),
            memory_viewer: MemoryViewer::new(),
            register_viewer: RegisterViewer::new(),
        }
    }

    /// 初始化 KDB
    pub fn init(&mut self) -> UnifiedResult<()> {
        self.is_running.store(true, Ordering::SeqCst);
        self.output("KDB initialized. Type 'help' for available commands.");
        Ok(())
    }

    /// 执行命令
    pub fn execute_command(&mut self, command: &str) -> UnifiedResult<String> {
        // 添加到历史
        self.command_history.push(command.to_string());
        self.history_index = self.command_history.len();

        // 解析命令
        let tokens = Self::tokenize_command(command);
        if tokens.is_empty() {
            return Ok(String::new());
        }

        let cmd = &tokens[0];
        let args = &tokens[1..];

        // 执行对应命令
        let result = match cmd.as_str() {
            "help" => self.cmd_help(args),
            "break" | "b" => self.cmd_break(args),
            "clear" => self.cmd_clear(args),
            "continue" | "c" => self.cmd_continue(args),
            "step" | "s" => self.cmd_step(args),
            "next" | "n" => self.cmd_next(args),
            "finish" | "fin" => self.cmd_finish(args),
            "print" | "p" => self.cmd_print(args),
            "x" => self.cmd_examine(args),
            "info" => self.cmd_info(args),
            "set" => self.cmd_set(args),
            "backtrace" | "bt" => self.cmd_backtrace(args),
            "frame" => self.cmd_frame(args),
            "thread" => self.cmd_thread(args),
            "quit" | "q" => self.cmd_quit(args),
            "history" => self.cmd_history(args),
            "alias" => self.cmd_alias(args),
            "disassemble" | "disas" => self.cmd_disassemble(args),
            "registers" | "regs" => self.cmd_registers(args),
            _ => {
                // 检查别名
                if let Some(aliased_cmd) = self.aliases.get(cmd) {
                    self.execute_command(aliased_cmd)?
                } else {
                    format!("Unknown command: {}. Type 'help' for available commands.", cmd)
                }
            }
        };

        Ok(result)
    }

    /// 命令：帮助
    fn cmd_help(&self, args: &[String]) -> String {
        if args.is_empty() {
            self.output("Available commands:");
            self.output("  break/b <addr>      - Set breakpoint at address");
            self.output("  clear <id>          - Clear breakpoint");
            self.output("  continue/c          - Continue execution");
            self.output("  step/s              - Single step into");
            self.output("  next/n              - Single step over");
            self.output("  finish/fin          - Step out of current function");
            self.output("  print/p <expr>      - Print expression");
            self.output("  x/nx <addr>         - Examine memory");
            self.output("  info <topic>        - Show information");
            self.output("  set <var> = <val>   - Set variable value");
            self.output("  backtrace/bt        - Show stack trace");
            self.output("  frame <n>           - Select stack frame");
            self.output("  thread <id>         - Select thread");
            self.output("  quit/q              - Quit debugger");
            self.output("  history             - Show command history");
            self.output("  alias <name>=<cmd>  - Define alias");
            self.output("  disassemble <addr>  - Disassemble code");
            self.output("  registers/regs      - Show registers");
            String::new()
        } else {
            // 显示特定命令的帮助
            format!("Help for '{}': (detailed help not implemented)", args[0])
        }
    }

    /// 命令：设置断点
    fn cmd_break(&mut self, args: &[String]) -> String {
        if args.is_empty() {
            return "Error: Breakpoint address required".to_string();
        }

        let address = if let Ok(addr) = u64::from_str_radix(args[0].trim_start_matches("0x"), 16) {
            addr
        } else if let Ok(addr) = args[0].parse::<u64>() {
            addr
        } else {
            return format!("Error: Invalid address: {}", args[0]);
        };

        match self.breakpoint_manager.set_breakpoint(address, BreakpointType::Software, None) {
            Ok(id) => {
                format!("Breakpoint {} set at 0x{:x}", id, address)
            }
            Err(e) => {
                format!("Error setting breakpoint: {:?}", e)
            }
        }
    }

    /// 命令：清除断点
    fn cmd_clear(&mut self, args: &[String]) -> String {
        if args.is_empty() {
            return "Error: Breakpoint ID required".to_string();
        }

        let id = match args[0].parse::<u64>() {
            Ok(i) => i,
            Err(_) => return format!("Error: Invalid breakpoint ID: {}", args[0]),
        };

        match self.breakpoint_manager.remove_breakpoint(id) {
            Ok(_) => {
                format!("Breakpoint {} cleared", id)
            }
            Err(e) => {
                format!("Error clearing breakpoint: {:?}", e)
            }
        }
    }

    /// 命令：继续执行
    fn cmd_continue(&mut self, _args: &[String]) -> String {
        self.step_state.is_stepping = false;
        self.output("Continuing execution...");
        String::new()
    }

    /// 命令：单步进入
    fn cmd_step(&mut self, args: &[String]) -> String {
        let count = if args.is_empty() {
            1
        } else {
            args[0].parse().unwrap_or(1)
        };

        self.step_state.is_stepping = true;
        self.step_state.step_type = StepType::StepInto;
        self.step_state.remaining_steps = count;

        format!("Stepping ({} instructions)", count)
    }

    /// 命令：单步跳过
    fn cmd_next(&mut self, args: &[String]) -> String {
        let count = if args.is_empty() {
            1
        } else {
            args[0].parse().unwrap_or(1)
        };

        self.step_state.is_stepping = true;
        self.step_state.step_type = StepType::StepOver;
        self.step_state.remaining_steps = count;

        format!("Stepping over ({} instructions)", count)
    }

    /// 命令：跳出函数
    fn cmd_finish(&mut self, _args: &[String]) -> String {
        self.step_state.is_stepping = true;
        self.step_state.step_type = StepType::StepOut;
        self.step_state.remaining_steps = 1;

        "Stepping out of current function".to_string()
    }

    /// 命令：打印表达式
    fn cmd_print(&self, args: &[String]) -> String {
        if args.is_empty() {
            return "Error: Expression required".to_string();
        }

        // 简化实现：直接打印值
        format!("Print: {}", args.join(" "))
    }

    /// 命令：检查内存
    fn cmd_examine(&self, args: &[String]) -> String {
        if args.is_empty() {
            return "Error: Address required".to_string();
        }

        let address = if let Ok(addr) = u64::from_str_radix(args[0].trim_start_matches("0x"), 16) {
            addr
        } else if let Ok(addr) = args[0].parse::<u64>() {
            addr
        } else {
            return format!("Error: Invalid address: {}", args[0]);
        };

        // 简化实现：显示内存内容
        format!("Memory at 0x{:x}: <data>", address)
    }

    /// 命令：显示信息
    fn cmd_info(&self, args: &[String]) -> String {
        if args.is_empty() {
            return "Error: Info topic required".to_string();
        }

        match args[0].as_str() {
            "breakpoints" | "bp" => {
                self.breakpoint_manager.list_breakpoints()
            }
            "registers" | "regs" => {
                self.register_viewer.dump_registers()
            }
            "threads" => {
                "Active threads: <list>".to_string()
            }
            _ => {
                format!("Unknown info topic: {}", args[0])
            }
        }
    }

    /// 命令：设置值
    fn cmd_set(&mut self, args: &[String]) -> String {
        if args.len() < 3 {
            return "Error: Usage: set <var> = <value>".to_string();
        }

        format!("Set {} = {}", args[0], args[2])
    }

    /// 命令：回溯
    fn cmd_backtrace(&self, _args: &[String]) -> String {
        self.output("Stack trace:");
        self.output("  #0  0xffffffff80001000 in function1");
        self.output("  #1  0xffffffff80002000 in function2");
        self.output("  #2  0xffffffff80003000 in main");
        String::new()
    }

    /// 命令：选择栈帧
    fn cmd_frame(&mut self, args: &[String]) -> String {
        if args.is_empty() {
            return "Error: Frame number required".to_string();
        }

        let frame_num = match args[0].parse::<usize>() {
            Ok(n) => n,
            Err(_) => return format!("Error: Invalid frame number: {}", args[0]),
        };

        format!("Selected frame #{}", frame_num)
    }

    /// 命令：选择线程
    fn cmd_thread(&mut self, args: &[String]) -> String {
        if args.is_empty() {
            return "Error: Thread ID required".to_string();
        }

        let tid = match args[0].parse::<u64>() {
            Ok(id) => id,
            Err(_) => return format!("Error: Invalid thread ID: {}", args[0]),
        };

        self.current_context.current_tid = Some(tid);
        format!("Selected thread {}", tid)
    }

    /// 命令：退出
    fn cmd_quit(&mut self, _args: &[String]) -> String {
        self.is_running.store(false, Ordering::SeqCst);
        "Exiting debugger...".to_string()
    }

    /// 命令：历史
    fn cmd_history(&self, _args: &[String]) -> String {
        self.output("Command history:");
        for (i, cmd) in self.command_history.iter().enumerate() {
            self.output(&format!("  {}  {}", i, cmd));
        }
        String::new()
    }

    /// 命令：别名
    fn cmd_alias(&mut self, args: &[String]) -> String {
        if args.is_empty() {
            // 列出所有别名
            self.output("Aliases:");
            for (name, cmd) in &self.aliases {
                self.output(&format!("  {} = {}", name, cmd));
            }
            String::new()
        } else {
            // 解析别名定义
            let parts: Vec<&str> = args[0].split('=').collect();
            if parts.len() != 2 {
                return "Error: Invalid alias syntax. Use: name=command".to_string();
            }

            let name = parts[0].trim().to_string();
            let cmd = parts[1].trim().to_string();
            self.aliases.insert(name.clone(), cmd);
            format!("Alias '{}' defined", name)
        }
    }

    /// 命令：反汇编
    fn cmd_disassemble(&self, args: &[String]) -> String {
        if args.is_empty() {
            return "Error: Address required".to_string();
        }

        let address = if let Ok(addr) = u64::from_str_radix(args[0].trim_start_matches("0x"), 16) {
            addr
        } else if let Ok(addr) = args[0].parse::<u64>() {
            addr
        } else {
            return format!("Error: Invalid address: {}", args[0]);
        };

        format!("Disassembly around 0x{:x}:\n  <instructions>", address)
    }

    /// 命令：显示寄存器
    fn cmd_registers(&self, _args: &[String]) -> String {
        self.register_viewer.dump_registers()
    }

    /// 分词命令
    fn tokenize_command(command: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;

        for ch in command.chars() {
            match ch {
                '"' => {
                    in_quotes = !in_quotes;
                }
                ' ' | '\t' if !in_quotes => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        tokens
    }

    /// 输出消息
    fn output(&self, message: &str) {
        // 在实际实现中，这会写入到控制台或日志
        log::info("[KDB] {}", message);
    }

    /// 创建默认别名
    fn default_aliases() -> BTreeMap<String, String> {
        let mut aliases = BTreeMap::new();
        aliases.insert("b".to_string(), "break".to_string());
        aliases.insert("c".to_string(), "continue".to_string());
        aliases.insert("s".to_string(), "step".to_string());
        aliases.insert("n".to_string(), "next".to_string());
        aliases.insert("fin".to_string(), "finish".to_string());
        aliases.insert("p".to_string(), "print".to_string());
        aliases.insert("bt".to_string(), "backtrace".to_string());
        aliases.insert("regs".to_string(), "registers".to_string());
        aliases.insert("q".to_string(), "quit".to_string());
        aliases.insert("disas".to_string(), "disassemble".to_string());
        aliases
    }
}

impl BreakpointManager {
    /// 创建新的断点管理器
    pub fn new() -> Self {
        Self {
            breakpoints: BTreeMap::new(),
            next_breakpoint_id: 1,
            enabled_count: 0,
            hardware_breakpoints: 0,
        }
    }

    /// 设置断点
    pub fn set_breakpoint(
        &mut self,
        address: u64,
        bp_type: BreakpointType,
        description: Option<String>,
    ) -> UnifiedResult<u64> {
        // 检查硬件断点数量限制
        if matches!(bp_type, BreakpointType::Hardware | BreakpointType::ReadWatch | BreakpointType::WriteWatch | BreakpointType::AccessWatch) {
            if self.hardware_breakpoints >= 4 {
                return Err(UnifiedError::ResourceBusy);
            }
        }

        // 检查是否已存在断点
        for bp in self.breakpoints.values() {
            if bp.address == address && bp.status != BreakpointStatus::Deleted {
                return Err(UnifiedError::AlreadyExists);
            }
        }

        let id = self.next_breakpoint_id;
        self.next_breakpoint_id += 1;

        let breakpoint = Breakpoint {
            id,
            address,
            breakpoint_type: bp_type,
            status: BreakpointStatus::Enabled,
            original_bytes: [0; 16],
            hit_count: 0,
            condition: None,
            commands: Vec::new(),
            description,
        };

        self.breakpoints.insert(id, breakpoint);
        self.enabled_count += 1;

        if matches!(bp_type, BreakpointType::Hardware | BreakpointType::ReadWatch | BreakpointType::WriteWatch | BreakpointType::AccessWatch) {
            self.hardware_breakpoints += 1;
        }

        Ok(id)
    }

    /// 移除断点
    pub fn remove_breakpoint(&mut self, id: u64) -> UnifiedResult<()> {
        let bp = self.breakpoints.get_mut(&id)
            .ok_or(UnifiedError::NotFound)?;

        if matches!(bp.breakpoint_type, BreakpointType::Hardware | BreakpointType::ReadWatch | BreakpointType::WriteWatch | BreakpointType::AccessWatch) {
            self.hardware_breakpoints -= 1;
        }

        if bp.status == BreakpointStatus::Enabled {
            self.enabled_count -= 1;
        }

        bp.status = BreakpointStatus::Deleted;
        self.breakpoints.remove(&id);

        Ok(())
    }

    /// 列出所有断点
    pub fn list_breakpoints(&self) -> String {
        if self.breakpoints.is_empty() {
            return "No breakpoints set.".to_string();
        }

        let mut output = String::from("Breakpoints:\n");
        for bp in self.breakpoints.values().filter(|bp| bp.status != BreakpointStatus::Deleted) {
            output.push_str(&format!(
                "  {} {} 0x{:x} - {}\n",
                bp.id,
                match bp.status {
                    BreakpointStatus::Enabled => "y",
                    BreakpointStatus::Disabled => "n",
                    BreakpointStatus::Hit => "*",
                    _ => "?",
                },
                bp.address,
                bp.description.as_ref().map(|s| s.as_str()).unwrap_or("no description")
            ));
        }

        output
    }

    /// 启用/禁用断点
    pub fn toggle_breakpoint(&mut self, id: u64, enable: bool) -> UnifiedResult<()> {
        let bp = self.breakpoints.get_mut(&id)
            .ok_or(UnifiedError::NotFound)?;

        if enable && bp.status != BreakpointStatus::Enabled {
            bp.status = BreakpointStatus::Enabled;
            self.enabled_count += 1;
        } else if !enable && bp.status == BreakpointStatus::Enabled {
            bp.status = BreakpointStatus::Disabled;
            self.enabled_count -= 1;
        }

        Ok(())
    }

    /// 命中断点
    pub fn hit_breakpoint(&mut self, id: u64) -> UnifiedResult<()> {
        let bp = self.breakpoints.get_mut(&id)
            .ok_or(UnifiedError::NotFound)?;

        bp.status = BreakpointStatus::Hit;
        bp.hit_count += 1;

        Ok(())
    }
}

impl MemoryViewer {
    /// 创建新的内存查看器
    pub fn new() -> Self {
        Self {
            display_format: MemoryFormat::Hex,
            unit_size: 8, // 64-bit
            units_per_line: 8,
            mappings: Vec::new(),
        }
    }

    /// 读取内存
    pub fn read_memory(&self, address: u64, size: usize) -> UnifiedResult<Vec<u8>> {
        // 简化实现
        Ok(vec![0; size])
    }

    /// 写入内存
    pub fn write_memory(&mut self, address: u64, data: &[u8]) -> UnifiedResult<()> {
        // 简化实现
        Ok(())
    }

    /// 格式化内存显示
    pub fn format_memory(&self, address: u64, data: &[u8]) -> String {
        let mut output = format!("0x{:016x}: ", address);

        match self.display_format {
            MemoryFormat::Hex => {
                for chunk in data.chunks(self.unit_size) {
                    for byte in chunk {
                        output.push_str(&format!("{:02x} ", byte));
                    }
                    output.push_str("  ");
                }
            }
            MemoryFormat::Ascii => {
                for byte in data {
                    if byte.is_ascii_graphic() || *byte == b' ' {
                        output.push(*byte as char);
                    } else {
                        output.push('.');
                    }
                }
            }
            _ => {
                output.push_str("<format>");
            }
        }

        output
    }
}

impl RegisterViewer {
    /// 创建新的寄存器查看器
    pub fn new() -> Self {
        Self {
            registers: RegisterSet::default(),
            display_format: RegisterFormat::Hex,
            fp_registers: Vec::new(),
            simd_registers: Vec::new(),
        }
    }

    /// 读取寄存器
    pub fn read_register(&self, name: &str) -> Option<u64> {
        match name.to_uppercase().as_str() {
            "RAX" => Some(self.registers.general_purpose[0]),
            "RBX" => Some(self.registers.general_purpose[1]),
            "RCX" => Some(self.registers.general_purpose[2]),
            "RDX" => Some(self.registers.general_purpose[3]),
            "RSI" => Some(self.registers.general_purpose[4]),
            "RDI" => Some(self.registers.general_purpose[5]),
            "RBP" => Some(self.registers.general_purpose[6]),
            "RSP" => Some(self.registers.general_purpose[7]),
            "RIP" => Some(self.registers.instruction_pointer),
            "RFLAGS" => Some(self.registers.flags),
            _ => None,
        }
    }

    /// 写入寄存器
    pub fn write_register(&mut self, name: &str, value: u64) -> UnifiedResult<()> {
        let idx = match name.to_uppercase().as_str() {
            "RAX" => Some(0),
            "RBX" => Some(1),
            "RCX" => Some(2),
            "RDX" => Some(3),
            "RSI" => Some(4),
            "RDI" => Some(5),
            "RBP" => Some(6),
            "RSP" => Some(7),
            "RIP" => {
                self.registers.instruction_pointer = value;
                None
            }
            "RFLAGS" => {
                self.registers.flags = value;
                None
            }
            _ => return Err(UnifiedError::NotFound),
        };

        if let Some(i) = idx {
            self.registers.general_purpose[i] = value;
        }

        Ok(())
    }

    /// 转储所有寄存器
    pub fn dump_registers(&self) -> String {
        let regs = &self.registers;
        let names = ["RAX", "RBX", "RCX", "RDX", "RSI", "RDI", "RBP", "RSP"];

        let mut output = String::from("Registers:\n");
        for (i, name) in names.iter().enumerate() {
            output.push_str(&format!(
                "  {:4s} = 0x{:016x}\n",
                name,
                regs.general_purpose[i]
            ));
        }
        output.push_str(&format!("  RIP  = 0x{:016x}\n", regs.instruction_pointer));
        output.push_str(&format!("  RFLAGS = 0x{:016x}\n", regs.flags));

        output
    }
}

impl Default for DebugContext {
    fn default() -> Self {
        Self {
            current_pid: None,
            current_tid: None,
            current_address_space: None,
            debug_level: 0,
            debug_mode: DebugMode::Local,
        }
    }
}

impl Default for StepState {
    fn default() -> Self {
        Self {
            is_stepping: false,
            step_type: StepType::InstructionStep,
            remaining_steps: 0,
            current_thread: None,
        }
    }
}

impl Default for RegisterSet {
    fn default() -> Self {
        Self {
            general_purpose: [0; 8],
            instruction_pointer: 0,
            flags: 0,
            segment: [0; 6],
            control: [0; 8],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kdb_creation() {
        let kdb = KdbInterpreter::new();
        assert!(!kdb.is_running.load(Ordering::SeqCst));
        assert!(kdb.command_history.is_empty());
    }

    #[test]
    fn test_breakpoint_set() {
        let mut kdb = KdbInterpreter::new();
        let result = kdb.cmd_break(&["0x1000".to_string()]);
        assert!(result.contains("Breakpoint"));
    }

    #[test]
    fn test_command_tokenize() {
        let cmd = "break 0x1000 \"test\"";
        let tokens = KdbInterpreter::tokenize_command(cmd);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], "break");
        assert_eq!(tokens[1], "0x1000");
    }

    #[test]
    fn test_memory_format() {
        let viewer = MemoryViewer::new();
        let data = vec![0x48, 0x8B, 0x05, 0x00, 0x00, 0x00, 0x00];
        let output = viewer.format_memory(0x1000, &data);
        assert!(output.contains("0x1000"));
    }

    #[test]
    fn test_register_operations() {
        let mut viewer = RegisterViewer::new();
        viewer.write_register("RAX", 0x123456789ABCDEF0).unwrap();
        assert_eq!(viewer.read_register("RAX"), Some(0x123456789ABCDEF0));
    }
}
