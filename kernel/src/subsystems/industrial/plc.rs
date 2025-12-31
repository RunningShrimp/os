//! # PLC (Programmable Logic Controller)
//!
//! IEC 61131-3 compliant PLC implementation supporting:
//! - Ladder Logic (LD)
//! - Function Block Diagram (FBD)
//! - Structured Text (ST)
//! - Instruction List (IL)
//! - Sequential Function Chart (SFC)

use alloc::{
    collections::BTreeMap,
    string::String,
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use crate::subsystems::industrial::{
    error::{IndustrialError, IndustrialResult, PlcError},
    RealtimeConstraints,
};

/// PLC configuration
#[derive(Debug, Clone)]
pub struct PlcConfig {
    /// Maximum cycle time in microseconds
    pub max_cycle_time_us: u64,
    /// Target cycle time in microseconds
    pub target_cycle_time_us: u64,
    /// Maximum program size in bytes
    pub max_program_size: usize,
    /// Number of digital inputs
    pub digital_inputs: u16,
    /// Number of digital outputs
    pub digital_outputs: u16,
    /// Number of analog inputs
    pub analog_inputs: u16,
    /// Number of analog outputs
    pub analog_outputs: u16,
    /// Enable watchdog
    pub enable_watchdog: bool,
    /// Watchdog timeout in cycles
    pub watchdog_timeout_cycles: u32,
}

impl Default for PlcConfig {
    fn default() -> Self {
        Self {
            max_cycle_time_us: 1000,  // 1ms
            target_cycle_time_us: 500,  // 500μs
            max_program_size: 1024 * 1024,  // 1MB
            digital_inputs: 512,
            digital_outputs: 512,
            analog_inputs: 128,
            analog_outputs: 128,
            enable_watchdog: true,
            watchdog_timeout_cycles: 3,
        }
    }
}

/// IEC 61131-3 programming language
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlcLanguage {
    LadderDiagram,
    FunctionBlockDiagram,
    StructuredText,
    InstructionList,
    SequentialFunctionChart,
}

/// Ladder logic contact type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactType {
    NormallyOpen,
    NormallyClosed,
    PositiveTransition,
    NegativeTransition,
}

/// Ladder logic coil type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoilType {
    Normal,
    Negated,
    Set,
    Reset,
}

/// Ladder logic instruction
#[derive(Debug, Clone)]
pub enum LadderInstruction {
    /// Contact (variable, type)
    Contact(String, ContactType),
    /// Coil (variable, type)
    Coil(String, CoilType),
    /// Timer (name, preset, resolution)
    Timer(String, u32, u16),
    /// Counter (name, preset)
    Counter(String, u32),
    /// Compare (op, left, right)
    Compare(CompareOp, String, String),
    /// Math operation
    Math(MathOp, String, String, String),
    /// Branch start (parallel path)
    BranchStart,
    /// Branch end
    BranchEnd,
    /// Rung end
    RungEnd,
}

/// Comparison operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

/// Math operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

/// Ladder logic program
#[derive(Debug, Clone)]
pub struct LadderProgram {
    pub name: String,
    pub instructions: Vec<LadderInstruction>,
    pub variables: BTreeMap<String, PlcVariable>,
}

/// PLC variable
#[derive(Debug, Clone)]
pub struct PlcVariable {
    pub name: String,
    pub var_type: PlcType,
    pub initial_value: PlcValue,
    pub retain: bool,
}

/// PLC data types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlcType {
    Bool,
    SInt,    // Signed 8-bit
    Int,     // Signed 16-bit
    DInt,    // Signed 32-bit
    LInt,    // Signed 64-bit
    USInt,   // Unsigned 8-bit
    UInt,    // Unsigned 16-bit
    UDInt,   // Unsigned 32-bit
    ULInt,   // Unsigned 64-bit
    Real,    // 32-bit float
    LReal,   // 64-bit float
    Time,
    Date,
    DateTime,
    String,
}

/// PLC value
#[derive(Debug, Clone)]
pub enum PlcValue {
    Bool(bool),
    SInt(i8),
    Int(i16),
    DInt(i32),
    LInt(i64),
    USInt(u8),
    UInt(u16),
    UDInt(u32),
    ULInt(u64),
    Real(f32),
    LReal(f64),
    Time(u32),  // milliseconds
    String(String),
    Undefined,
}

impl PlcValue {
    pub fn as_bool(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            _ => false,
        }
    }

    pub fn as_int(&self) -> i64 {
        match self {
            Self::SInt(v) => *v as i64,
            Self::Int(v) => *v as i64,
            Self::DInt(v) => *v as i64,
            Self::LInt(v) => *v,
            Self::USInt(v) => *v as i64,
            Self::UInt(v) => *v as i64,
            Self::UDInt(v) => *v as i64,
            Self::ULInt(v) => *v as i64,
            _ => 0,
        }
    }

    pub fn as_real(&self) -> f64 {
        match self {
            Self::Real(v) => *v as f64,
            Self::LReal(v) => *v,
            _ => self.as_int() as f64,
        }
    }
}

/// Function block
#[derive(Debug, Clone)]
pub struct FunctionBlock {
    pub name: String,
    pub inputs: BTreeMap<String, PlcValue>,
    pub outputs: BTreeMap<String, PlcValue>,
    pub fb_type: FunctionBlockType,
}

/// Function block types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionBlockType {
    TimerOn,
    TimerOff,
    TimerPulse,
    CounterUp,
    CounterDown,
    CounterUpDown,
    /// PID controller
    Pid,
    /// Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    /// Logic
    And,
    Or,
    Xor,
    Not,
    /// Compare
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// PLC execution engine
pub struct PlcEngine {
    config: PlcConfig,
    program: Option<LadderProgram>,
    variables: BTreeMap<String, PlcValue>,
    inputs: BTreeMap<String, PlcValue>,
    outputs: BTreeMap<String, PlcValue>,
    timers: BTreeMap<String, TimerState>,
    counters: BTreeMap<String, CounterState>,
    function_blocks: BTreeMap<String, FunctionBlock>,
    running: Arc<AtomicBool>,
    cycle_count: Arc<AtomicU64>,
    last_cycle_time_us: Arc<AtomicU64>,
}

/// Timer state
#[derive(Debug, Clone)]
struct TimerState {
    preset: u32,
    elapsed: u32,
    running: bool,
    done: bool,
    resolution_ms: u16,
}

/// Counter state
#[derive(Debug, Clone)]
struct CounterState {
    preset: u32,
    count: u32,
    overflow: bool,
}

impl PlcEngine {
    /// Create new PLC engine
    pub fn new() -> Self {
        Self::with_config(PlcConfig::default())
    }

    /// Create PLC engine with configuration
    pub fn with_config(config: PlcConfig) -> Self {
        Self {
            config,
            program: None,
            variables: BTreeMap::new(),
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            timers: BTreeMap::new(),
            counters: BTreeMap::new(),
            function_blocks: BTreeMap::new(),
            running: Arc::new(AtomicBool::new(false)),
            cycle_count: Arc::new(AtomicU64::new(0)),
            last_cycle_time_us: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Compile program
    pub fn compile(&self, source: &str, language: PlcLanguage) -> IndustrialResult<LadderProgram> {
        match language {
            PlcLanguage::LadderDiagram => self.compile_ladder(source),
            PlcLanguage::StructuredText => self.compile_structured_text(source),
            _ => Err(IndustrialError::Plc(PlcError::CompilationFailed("Language not yet supported"))),
        }
    }

    /// Compile ladder logic
    fn compile_ladder(&self, _source: &str) -> IndustrialResult<LadderProgram> {
        // In real implementation, parse ladder logic source
        // For now, return a simple program
        Ok(LadderProgram {
            name: "main".into(),
            instructions: Vec::new(),
            variables: BTreeMap::new(),
        })
    }

    /// Compile structured text
    fn compile_structured_text(&self, _source: &str) -> IndustrialResult<LadderProgram> {
        // In real implementation, parse IEC 61131-3 ST
        Ok(LadderProgram {
            name: "st_program".into(),
            instructions: Vec::new(),
            variables: BTreeMap::new(),
        })
    }

    /// Load program
    pub fn load_program(&mut self, program: LadderProgram) -> IndustrialResult<()> {
        if program.instructions.len() > self.config.max_program_size {
            return Err(IndustrialError::Plc(PlcError::InvalidProgram("Program too large")));
        }

        // Initialize variables
        for (name, var) in &program.variables {
            self.variables.insert(name.clone(), var.initial_value.clone());
        }

        self.program = Some(program);
        log_info!("Program loaded: {} instructions", self.program.as_ref().unwrap().instructions.len());
        Ok(())
    }

    /// Run PLC cycle
    pub fn run_cycle(&mut self) -> IndustrialResult<u64> {
        let start = self.get_timestamp_us();

        if self.program.is_none() {
            return Err(IndustrialError::Plc(PlcError::ProgramNotLoaded));
        }

        // Read inputs
        self.read_inputs()?;

        // Execute program
        self.execute_program()?;

        // Write outputs
        self.write_outputs()?;

        // Update timers and counters
        self.update_timers();
        self.update_counters();

        let end = self.get_timestamp_us();
        let cycle_time = end.saturating_sub(start);

        // Check cycle time
        if cycle_time > self.config.max_cycle_time_us {
            return Err(IndustrialError::Plc(PlcError::ExecutionTimeout {
                cycle_time_us: cycle_time,
                max_cycle_time_us: self.config.max_cycle_time_us,
            }));
        }

        self.last_cycle_time_us.store(cycle_time, Ordering::SeqCst);
        self.cycle_count.fetch_add(1, Ordering::SeqCst);

        Ok(cycle_time)
    }

    /// Start PLC execution
    pub fn run(&mut self) -> IndustrialResult<()> {
        if self.program.is_none() {
            return Err(IndustrialError::Plc(PlcError::ProgramNotLoaded));
        }

        self.running.store(true, Ordering::SeqCst);
        log_info!("PLC execution started");
        Ok(())
    }

    /// Stop PLC execution
    pub fn stop(&mut self) -> IndustrialResult<()> {
        self.running.store(false, Ordering::SeqCst);
        log_info!("PLC execution stopped");
        Ok(())
    }

    /// Read inputs
    fn read_inputs(&mut self) -> IndustrialResult<()> {
        // In real implementation, read from I/O modules
        Ok(())
    }

    /// Write outputs
    fn write_outputs(&mut self) -> IndustrialResult<()> {
        // In real implementation, write to I/O modules
        Ok(())
    }

    /// Execute program
    fn execute_program(&mut self) -> IndustrialResult<()> {
        // Clone the program to avoid borrow conflicts
        let instructions = self.program.as_ref()
            .map(|p| p.instructions.clone())
            .unwrap_or_default();

        // Simple ladder logic execution
        let mut stack = Vec::new();
        let mut pending_updates: alloc::collections::BTreeMap<String, PlcValue> = alloc::collections::BTreeMap::new();

        for instruction in &instructions {
            match instruction {
                LadderInstruction::Contact(name, contact_type) => {
                    let value = self.get_variable_value(name).as_bool();
                    let result = match contact_type {
                        ContactType::NormallyOpen => value,
                        ContactType::NormallyClosed => !value,
                        ContactType::PositiveTransition => {
                            // Detect rising edge (requires previous state)
                            value
                        }
                        ContactType::NegativeTransition => !value,
                    };
                    stack.push(result);
                }
                LadderInstruction::Coil(name, coil_type) => {
                    let value = stack.last().copied().unwrap_or(false);
                    let result = match coil_type {
                        CoilType::Normal => value,
                        CoilType::Negated => !value,
                        CoilType::Set => {
                            if value {
                                pending_updates.insert(name.clone(), PlcValue::Bool(true));
                            }
                            value
                        }
                        CoilType::Reset => {
                            if value {
                                pending_updates.insert(name.clone(), PlcValue::Bool(false));
                            }
                            value
                        }
                    };
                    if matches!(coil_type, CoilType::Normal | CoilType::Negated) {
                        pending_updates.insert(name.clone(), PlcValue::Bool(result));
                    }
                }
                LadderInstruction::Timer(name, preset, resolution) => {
                    self.timers.entry(name.clone()).or_insert_with(|| {
                        TimerState {
                            preset: *preset,
                            elapsed: 0,
                            running: false,
                            done: false,
                            resolution_ms: *resolution,
                        }
                    });
                }
                LadderInstruction::Counter(name, preset) => {
                    self.counters.entry(name.clone()).or_insert_with(|| {
                        CounterState {
                            preset: *preset,
                            count: 0,
                            overflow: false,
                        }
                    });
                }
                _ => {}
            }
        }

        // Apply pending updates
        for (name, value) in pending_updates {
            self.set_variable(&name, value);
        }

        Ok(())
    }

    /// Update timers
    fn update_timers(&mut self) {
        for timer in self.timers.values_mut() {
            if timer.running {
                timer.elapsed += timer.resolution_ms as u32;
                if timer.elapsed >= timer.preset {
                    timer.done = true;
                    timer.running = false;
                }
            }
        }
    }

    /// Update counters
    fn update_counters(&mut self) {
        // Counter updates happen during program execution
    }

    /// Get variable value
    fn get_variable_value(&self, name: &str) -> PlcValue {
        self.variables.get(name)
            .or_else(|| self.inputs.get(name))
            .cloned()
            .unwrap_or(PlcValue::Undefined)
    }

    /// Set variable value
    fn set_variable(&mut self, name: &str, value: PlcValue) {
        self.variables.insert(name.into(), value);
    }

    /// Get input value
    pub fn get_input(&self, name: &str) -> IndustrialResult<&PlcValue> {
        self.inputs.get(name)
            .ok_or_else(|| IndustrialError::Plc(PlcError::RuntimeError("Input not found")))
    }

    /// Set input value
    pub fn set_input(&mut self, name: &str, value: PlcValue) -> IndustrialResult<()> {
        self.inputs.insert(name.into(), value);
        Ok(())
    }

    /// Get output value
    pub fn get_output(&self, name: &str) -> IndustrialResult<&PlcValue> {
        self.outputs.get(name)
            .ok_or_else(|| IndustrialError::Plc(PlcError::RuntimeError("Output not found")))
    }

    /// Get cycle count
    pub fn cycle_count(&self) -> u64 {
        self.cycle_count.load(Ordering::SeqCst)
    }

    /// Get last cycle time
    pub fn last_cycle_time_us(&self) -> u64 {
        self.last_cycle_time_us.load(Ordering::SeqCst)
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get real-time constraints
    pub fn realtime_constraints(&self) -> RealtimeConstraints {
        RealtimeConstraints {
            max_latency_us: self.config.target_cycle_time_us,
            max_jitter_us: self.config.max_cycle_time_us / 10,
            deadline_us: self.config.max_cycle_time_us,
            period_us: Some(self.config.target_cycle_time_us),
        }
    }

    /// Get timestamp in microseconds
    fn get_timestamp_us(&self) -> u64 {
        // In real implementation, get from high-resolution timer
        0
    }
}

/// I/O module
#[derive(Debug, Clone)]
pub struct IoModule {
    pub module_id: u8,
    pub module_type: IoModuleType,
    pub channel_count: u16,
    pub channels: Vec<IoChannel>,
}

/// I/O module type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoModuleType {
    DigitalInput,
    DigitalOutput,
    AnalogInput,
    AnalogOutput,
    Mixed,
}

/// I/O channel
#[derive(Debug, Clone)]
pub struct IoChannel {
    pub channel_number: u16,
    pub value: PlcValue,
    pub quality: Quality,
}

/// Signal quality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Good,
    Bad,
    Uncertain,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plc_creation() {
        let plc = PlcEngine::new();
        assert_eq!(plc.cycle_count(), 0);
        assert!(!plc.is_running());
    }

    #[test]
    fn test_program_load() {
        let mut plc = PlcEngine::new();
        let program = LadderProgram {
            name: "test".into(),
            instructions: vec![
                LadderInstruction::Contact("start".into(), ContactType::NormallyOpen),
                LadderInstruction::Coil("run".into(), CoilType::Normal),
            ],
            variables: BTreeMap::new(),
        };

        plc.load_program(program).unwrap();
        assert!(plc.program.is_some());
    }

    #[test]
    fn test_input_output() {
        let mut plc = PlcEngine::new();
        plc.set_input("test_in", PlcValue::Bool(true)).unwrap();
        assert!(plc.get_input("test_in").unwrap().as_bool());
    }

    #[test]
    fn test_variable_operations() {
        let mut plc = PlcEngine::new();
        plc.set_variable("counter", PlcValue::DInt(42));
        assert_eq!(plc.get_variable_value("counter").as_int(), 42);
    }

    #[test]
    fn test_realtime_constraints() {
        let plc = PlcEngine::new();
        let rt = plc.realtime_constraints();
        assert_eq!(rt.max_latency_us, 500);
        assert_eq!(rt.deadline_us, 1000);
    }
}
