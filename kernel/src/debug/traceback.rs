//! Stack Traceback
//!
//! 栈回溯模块
//! 提供栈回溯、调用栈重建、符号化栈帧、打印友好格式等功能

extern crate alloc;

use alloc::fmt;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write;

use crate::error::unified::{UnifiedError, UnifiedResult};
use crate::debug::symbol::{SymbolResolver, SourceLocation};

/// 栈回溯器
#[derive(Debug)]
pub struct StackTraceback {
    /// 符号解析器
    pub symbol_resolver: SymbolResolver,
    /// 最大栈帧深度
    pub max_depth: usize,
    /// 输出格式
    pub output_format: OutputFormat,
    /// 是否包含内联帧
    pub include_inline_frames: bool,
}

/// 输出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// 简短格式
    Short,
    /// 标准格式
    Standard,
    /// 详细格式
    Verbose,
    /// JSON 格式
    Json,
}

/// 栈帧
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// 帧编号
    pub frame_number: usize,
    /// 返回地址
    pub return_address: u64,
    /// 帧指针
    pub frame_pointer: u64,
    /// 栈指针
    pub stack_pointer: u64,
    /// 指令指针
    pub instruction_pointer: u64,
    /// 符号名称
    pub symbol_name: Option<String>,
    /// 源代码位置
    pub source_location: Option<SourceLocation>,
    /// 是否为内联帧
    pub is_inline: bool,
    /// 参数（如果可用）
    pub arguments: Vec<StackArgument>,
    /// 局部变量（如果可用）
    pub locals: Vec<StackVariable>,
}

/// 栈参数
#[derive(Debug, Clone)]
pub struct StackArgument {
    /// 参数名称
    pub name: String,
    /// 参数值
    pub value: u64,
    /// 参数类型
    pub type_name: Option<String>,
}

/// 栈变量
#[derive(Debug, Clone)]
pub struct StackVariable {
    /// 变量名称
    pub name: String,
    /// 变量值
    pub value: u64,
    /// 变量类型
    pub type_name: Option<String>,
    /// 相对于帧指针的偏移
    pub offset: i64,
}

/// 回溯选项
#[derive(Debug, Clone, Copy)]
pub struct TracebackOptions {
    /// 最大深度
    pub max_depth: usize,
    /// 跳过帧数
    pub skip_frames: usize,
    /// 是否包含内联帧
    pub include_inline: bool,
    /// 是否尝试符号化
    pub symbolize: bool,
    /// 输出格式
    pub output_format: OutputFormat,
}

impl Default for TracebackOptions {
    fn default() -> Self {
        Self {
            max_depth: 64,
            skip_frames: 0,
            include_inline: false,
            symbolize: true,
            output_format: OutputFormat::Standard,
        }
    }
}

impl StackTraceback {
    /// 创建新的栈回溯器
    pub fn new() -> Self {
        Self {
            symbol_resolver: SymbolResolver::new(),
            max_depth: 64,
            output_format: OutputFormat::Standard,
            include_inline_frames: false,
        }
    }

    /// 从当前栈捕获回溯
    pub fn capture(&self, options: Option<TracebackOptions>) -> UnifiedResult<Vec<StackFrame>> {
        let opts = options.unwrap_or_default();

        // 获取当前栈指针和帧指针
        let (rsp, rbp) = self.get_stack_pointers();

        self.unwind_stack(rsp, rbp, opts)
    }

    /// 从指定寄存器状态解栈
    pub fn unwind_from_registers(
        &self,
        rsp: u64,
        rbp: u64,
        rip: u64,
        options: Option<TracebackOptions>,
    ) -> UnifiedResult<Vec<StackFrame>> {
        let opts = options.unwrap_or_default();
        let mut frames = Vec::new();

        // 添加第一帧（当前函数）
        if opts.skip_frames == 0 {
            let frame = self.create_frame(0, rip, rbp, rsp);
            frames.push(frame);
        }

        // 解栈
        frames.extend(self.unwind_stack(rsp, rbp, opts)?);

        Ok(frames)
    }

    /// 解栈实现
    fn unwind_stack(&self, rsp: u64, rbp: u64, options: TracebackOptions) -> UnifiedResult<Vec<StackFrame>> {
        let mut frames = Vec::new();
        let mut current_rbp = rbp;
        let mut current_rsp = rsp;

        for depth in 0..options.max_depth {
            // 安全检查
            if current_rbp == 0 || current_rbp < current_rsp {
                break;
            }

            // 读取返回地址（位于 rbp + 8）
            let return_address = self.read_memory(current_rbp + 8)?;

            if return_address == 0 {
                break;
            }

            // 创建栈帧
            let frame_number = depth + options.skip_frames;
            if frame_number >= options.skip_frames {
                let mut frame = self.create_frame(frame_number, return_address, current_rbp, current_rsp);

                // 符号化（如果启用）
                if options.symbolize {
                    if let Some(symbol) = self.symbol_resolver.lookup_address(return_address) {
                        frame.symbol_name = Some(symbol.name);
                        frame.source_location = symbol.source_location;
                    }
                }

                frames.push(frame);
            }

            // 移动到上一帧
            let prev_rbp = self.read_memory(current_rbp)?;
            if prev_rbp <= current_rbp {
                break;
            }

            current_rsp = current_rbp + 16;
            current_rbp = prev_rbp;
        }

        Ok(frames)
    }

    /// 创建栈帧
    fn create_frame(&self, frame_number: usize, rip: u64, rbp: u64, rsp: u64) -> StackFrame {
        StackFrame {
            frame_number,
            return_address: rip,
            frame_pointer: rbp,
            stack_pointer: rsp,
            instruction_pointer: rip,
            symbol_name: None,
            source_location: None,
            is_inline: false,
            arguments: Vec::new(),
            locals: Vec::new(),
        }
    }

    /// 获取栈指针和帧指针
    fn get_stack_pointers(&self) -> (u64, u64) {
        // 在实际实现中，这里会读取实际的 RSP 和 RBP 寄存器
        // 简化实现返回 0
        (0, 0)
    }

    /// 读取内存（简化实现）
    fn read_memory(&self, address: u64) -> UnifiedResult<u64> {
        // 在实际实现中，这里会安全地读取指定地址的内存
        // 简化实现返回 0
        if address == 0 {
            return Err(UnifiedError::InvalidAddress);
        }
        Ok(0)
    }

    /// 格式化栈回溯
    pub fn format_traceback(&self, frames: &[StackFrame]) -> String {
        match self.output_format {
            OutputFormat::Short => self.format_short(frames),
            OutputFormat::Standard => self.format_standard(frames),
            OutputFormat::Verbose => self.format_verbose(frames),
            OutputFormat::Json => self.format_json(frames),
        }
    }

    /// 简短格式
    fn format_short(&self, frames: &[StackFrame]) -> String {
        let mut output = String::new();

        for frame in frames {
            if let Some(ref symbol) = frame.symbol_name {
                writeln!(output, "#{} {}", frame.frame_number, symbol).ok();
            } else {
                writeln!(output, "#{} 0x{:016x}", frame.frame_number, frame.return_address).ok();
            }
        }

        output
    }

    /// 标准格式
    fn format_standard(&self, frames: &[StackFrame]) -> String {
        let mut output = String::new();

        writeln!(output, "Stack trace ({} frames):", frames.len()).ok();

        for frame in frames {
            write!(output, "  #{} ", frame.frame_number).ok();

            if let Some(ref symbol) = frame.symbol_name {
                write!(output, "{}", symbol).ok();

                if let Some(ref location) = frame.source_location {
                    write!(output, " at {}:{}", location.file, location.line).ok();
                }
            } else {
                write!(output, "0x{:016x}", frame.return_address).ok();
            }

            writeln!(output).ok();
        }

        output
    }

    /// 详细格式
    fn format_verbose(&self, frames: &[StackFrame]) -> String {
        let mut output = String::new();

        writeln!(output, "Detailed stack trace ({} frames):", frames.len()).ok();

        for frame in frames {
            writeln!(output, "Frame #{}:", frame.frame_number).ok();
            writeln!(output, "  Return address: 0x{:016x}", frame.return_address).ok();
            writeln!(output, "  Frame pointer:  0x{:016x}", frame.frame_pointer).ok();
            writeln!(output, "  Stack pointer:  0x{:016x}", frame.stack_pointer).ok();

            if let Some(ref symbol) = frame.symbol_name {
                writeln!(output, "  Function:       {}", symbol).ok();
            }

            if let Some(ref location) = frame.source_location {
                writeln!(output, "  Location:       {}:{}", location.file, location.line).ok();
                if location.column > 0 {
                    writeln!(output, "  Column:         {}", location.column).ok();
                }
            }

            if !frame.arguments.is_empty() {
                writeln!(output, "  Arguments:").ok();
                for arg in &frame.arguments {
                    if let Some(ref type_name) = arg.type_name {
                        writeln!(output, "    {}: {} = 0x{:x}", type_name, arg.name, arg.value).ok();
                    } else {
                        writeln!(output, "    {} = 0x{:x}", arg.name, arg.value).ok();
                    }
                }
            }

            if !frame.locals.is_empty() {
                writeln!(output, "  Locals:").ok();
                for local in &frame.locals {
                    if let Some(ref type_name) = local.type_name {
                        writeln!(output, "    {}: {} = 0x{:x} (offset {})", type_name, local.name, local.value, local.offset).ok();
                    } else {
                        writeln!(output, "    {} = 0x{:x} (offset {})", local.name, local.value, local.offset).ok();
                    }
                }
            }

            writeln!(output).ok();
        }

        output
    }

    /// JSON 格式
    fn format_json(&self, frames: &[StackFrame]) -> String {
        let mut output = String::from("[");

        for (i, frame) in frames.iter().enumerate() {
            if i > 0 {
                output.push_str(", ");
            }

            output.push_str("{");
            output.push_str(&format!("\"frame_number\":{},", frame.frame_number));
            output.push_str(&format!("\"return_address\":\"0x{:x}\",", frame.return_address));
            output.push_str(&format!("\"frame_pointer\":\"0x{:x}\",", frame.frame_pointer));
            output.push_str(&format!("\"stack_pointer\":\"0x{:x}\"", frame.stack_pointer));

            if let Some(ref symbol) = frame.symbol_name {
                output.push_str(&format!(",\"symbol_name\":\"{}\"", symbol));
            }

            if let Some(ref location) = frame.source_location {
                output.push_str(&format!(",\"source_file\":\"{}\"", location.file));
                output.push_str(&format!(",\"source_line\":{}", location.line));
                if location.column > 0 {
                    output.push_str(&format!(",\"source_column\":{}", location.column));
                }
            }

            output.push_str("}");
        }

        output.push_str("]");
        output
    }

    /// 比较两个栈回溯
    pub fn compare_traces(&self, trace1: &[StackFrame], trace2: &[StackFrame]) -> StackDiff {
        let common_depth = trace1.iter()
            .zip(trace2.iter())
            .take_while(|(f1, f2)| {
                f1.return_address == f2.return_address || match (&f1.symbol_name, &f2.symbol_name) {
                    (Some(s1), Some(s2)) => s1 == s2,
                    _ => false,
                }
            })
            .count();

        StackDiff {
            common_depth,
            trace1_only: if trace1.len() > common_depth {
                Some(trace1[common_depth..].to_vec())
            } else {
                None
            },
            trace2_only: if trace2.len() > common_depth {
                Some(trace2[common_depth..].to_vec())
            } else {
                None
            },
        }
    }

    /// 查找最深的公共栈帧
    pub fn find_common_ancestor(&self, traces: &[Vec<StackFrame>]) -> Option<&StackFrame> {
        if traces.is_empty() {
            return None;
        }

        let first_trace = &traces[0];
        if first_trace.is_empty() {
            return None;
        }

        'outer: for (i, frame) in first_trace.iter().enumerate() {
            for trace in &traces[1..] {
                if i >= trace.len() {
                    return Some(frame);
                }

                let other_frame = &trace[i];
                if frame.return_address != other_frame.return_address {
                    if i > 0 {
                        return Some(&first_trace[i - 1]);
                    } else {
                        continue 'outer;
                    }
                }
            }

            return Some(frame);
        }

        None
    }

    /// 设置符号解析器
    pub fn set_symbol_resolver(&mut self, resolver: SymbolResolver) {
        self.symbol_resolver = resolver;
    }

    /// 设置最大深度
    pub fn set_max_depth(&mut self, depth: usize) {
        self.max_depth = depth;
    }

    /// 设置输出格式
    pub fn set_output_format(&mut self, format: OutputFormat) {
        self.output_format = format;
    }
}

/// 栈差异
#[derive(Debug, Clone)]
pub struct StackDiff {
    /// 公共深度
    pub common_depth: usize,
    /// 仅在 trace1 中的帧
    pub trace1_only: Option<Vec<StackFrame>>,
    /// 仅在 trace2 中的帧
    pub trace2_only: Option<Vec<StackFrame>>,
}

impl Default for StackTraceback {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{} 0x{:016x}", self.frame_number, self.return_address)?;

        if let Some(ref symbol) = self.symbol_name {
            write!(f, " in {}", symbol)?;

            if let Some(ref location) = self.source_location {
                write!(f, " at {}:{}", location.file, location.line)?;
            }
        }

        Ok(())
    }
}

/// 便捷函数：捕获并格式化当前栈回溯
pub fn print_traceback() -> String {
    let traceback = StackTraceback::new();
    let frames = traceback.capture(None).unwrap_or_default();
    traceback.format_traceback(&frames)
}

/// 便捷函数：捕获当前栈回溯（仅返回帧）
pub fn capture_traceback() -> Vec<StackFrame> {
    let traceback = StackTraceback::new();
    traceback.capture(None).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traceback_creation() {
        let traceback = StackTraceback::new();
        assert_eq!(traceback.max_depth, 64);
    }

    #[test]
    fn test_frame_formatting() {
        let frame = StackFrame {
            frame_number: 0,
            return_address: 0x1000,
            frame_pointer: 0x2000,
            stack_pointer: 0x1ff0,
            instruction_pointer: 0x1000,
            symbol_name: Some("test_function".to_string()),
            source_location: Some(SourceLocation {
                file: "test.rs".to_string(),
                line: 42,
                column: 10,
                function: "test_function".to_string(),
            }),
            is_inline: false,
            arguments: Vec::new(),
            locals: Vec::new(),
        };

        let formatted = format!("{}", frame);
        assert!(formatted.contains("test_function"));
        assert!(formatted.contains("test.rs"));
        assert!(formatted.contains("42"));
    }

    #[test]
    fn test_format_standard() {
        let traceback = StackTraceback::new();
        let frames = vec![
            StackFrame {
                frame_number: 0,
                return_address: 0x1000,
                frame_pointer: 0x2000,
                stack_pointer: 0x1ff0,
                instruction_pointer: 0x1000,
                symbol_name: Some("func1".to_string()),
                source_location: None,
                is_inline: false,
                arguments: Vec::new(),
                locals: Vec::new(),
            },
            StackFrame {
                frame_number: 1,
                return_address: 0x2000,
                frame_pointer: 0x3000,
                stack_pointer: 0x2ff0,
                instruction_pointer: 0x2000,
                symbol_name: None,
                source_location: None,
                is_inline: false,
                arguments: Vec::new(),
                locals: Vec::new(),
            },
        ];

        let output = traceback.format_standard(&frames);
        assert!(output.contains("Stack trace"));
        assert!(output.contains("func1"));
        assert!(output.contains("0x2000"));
    }

    #[test]
    fn test_compare_traces() {
        let traceback = StackTraceback::new();

        let trace1 = vec![
            StackFrame {
                frame_number: 0,
                return_address: 0x1000,
                frame_pointer: 0,
                stack_pointer: 0,
                instruction_pointer: 0x1000,
                symbol_name: Some("common".to_string()),
                source_location: None,
                is_inline: false,
                arguments: Vec::new(),
                locals: Vec::new(),
            },
            StackFrame {
                frame_number: 1,
                return_address: 0x2000,
                frame_pointer: 0,
                stack_pointer: 0,
                instruction_pointer: 0x2000,
                symbol_name: Some("trace1_only".to_string()),
                source_location: None,
                is_inline: false,
                arguments: Vec::new(),
                locals: Vec::new(),
            },
        ];

        let trace2 = vec![
            StackFrame {
                frame_number: 0,
                return_address: 0x1000,
                frame_pointer: 0,
                stack_pointer: 0,
                instruction_pointer: 0x1000,
                symbol_name: Some("common".to_string()),
                source_location: None,
                is_inline: false,
                arguments: Vec::new(),
                locals: Vec::new(),
            },
            StackFrame {
                frame_number: 1,
                return_address: 0x3000,
                frame_pointer: 0,
                stack_pointer: 0,
                instruction_pointer: 0x3000,
                symbol_name: Some("trace2_only".to_string()),
                source_location: None,
                is_inline: false,
                arguments: Vec::new(),
                locals: Vec::new(),
            },
        ];

        let diff = traceback.compare_traces(&trace1, &trace2);
        assert_eq!(diff.common_depth, 1);
        assert!(diff.trace1_only.is_some());
        assert!(diff.trace2_only.is_some());
    }
}
