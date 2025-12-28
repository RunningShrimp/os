use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfilingMode {
    Off,
    Sampling,
    Tracing,
}

#[derive(Debug, Clone)]
pub enum ProfilingEvent {
    FunctionEnter {
        function_id: u64,
        timestamp: u64,
        cpu_id: u64,
    },
    FunctionExit {
        function_id: u64,
        timestamp: u64,
        cpu_id: u64,
    },
    Interrupt {
        interrupt_id: u64,
        timestamp: u64,
        cpu_id: u64,
    },
    ContextSwitch {
        from_task: u64,
        to_task: u64,
        timestamp: u64,
        cpu_id: u64,
    },
    PageFault {
        address: u64,
        timestamp: u64,
        task_id: u64,
    },
    CacheMiss {
        level: u8,
        address: u64,
        timestamp: u64,
    },
}

#[derive(Debug, Clone)]
pub struct ProfilingSample {
    pub timestamp: u64,
    pub cpu_id: u64,
    pub task_id: Option<u64>,
    pub stack_frames: Vec<Frame>,
    pub event_type: SampleEventType,
}

#[derive(Debug, Clone)]
pub enum SampleEventType {
    TimerInterrupt,
    FunctionCall,
    SystemCall,
    InterruptHandler,
    Idle,
}

#[derive(Debug, Clone)]
pub struct ProfilingSession {
    pub id: u64,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub mode: ProfilingMode,
    pub samples: Vec<ProfilingSample>,
    pub events: Vec<ProfilingEvent>,
    pub statistics: ProfilingStatistics,
    pub config: ProfilingConfig,
}

#[derive(Debug, Clone)]
pub struct ProfilingStatistics {
    pub total_samples: u64,
    pub total_events: u64,
    pub cpu_time_ns: u64,
    pub interrupt_count: u64,
    pub context_switch_count: u64,
    pub page_fault_count: u64,
    pub function_calls: u64,
    pub max_stack_depth: usize,
    pub average_stack_depth: f64,
}

#[derive(Debug, Clone)]
pub struct CallGraph {
    pub root: FunctionInfo,
    pub children: BTreeMap<String, CallGraph>,
    pub total_time_ns: u64,
    pub self_time_ns: u64,
    pub call_count: u64,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub id: u64,
    pub name: String,
    pub module: String,
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub struct ProfilingConfig {
    pub sampling_interval_ns: u64,
    pub max_samples: usize,
    pub max_stack_depth: usize,
    pub trace_functions: bool,
    pub trace_interrupts: bool,
    pub trace_context_switches: bool,
    pub trace_page_faults: bool,
    pub trace_cache_misses: bool,
    pub include_kernel: bool,
    pub include_userspace: bool,
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            sampling_interval_ns: 1_000_000,
            max_samples: 1_000_000,
            max_stack_depth: 64,
            trace_functions: true,
            trace_interrupts: true,
            trace_context_switches: true,
            trace_page_faultes: true,
            trace_cache_misses: false,
            include_kernel: true,
            include_userspace: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub function_id: u64,
    pub function_name: String,
    pub module_name: String,
    pub file: String,
    pub line: u32,
    pub frame_type: FrameType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    KernelFunction,
    UserFunction,
    InterruptHandler,
    SystemCall,
    Unknown,
}

pub struct Profiler {
    mode: AtomicU8,
    session_id: AtomicU64,
    samples: Mutex<Vec<ProfilingSample>>,
    events: Mutex<Vec<ProfilingEvent>>,
    config: Mutex<ProfilingConfig>,
    statistics: Mutex<ProfilingStatistics>,
    call_stack: Mutex<Vec<u64>>,
    active_session: Mutex<Option<ProfilingSession>>,
    function_map: Mutex<BTreeMap<u64, FunctionInfo>>,
    name_to_id: Mutex<BTreeMap<String, u64>>,
    next_function_id: AtomicU64,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            mode: AtomicU8::new(ProfilingMode::Off as u8),
            session_id: AtomicU64::new(0),
            samples: Mutex::new(Vec::new()),
            events: Mutex::new(Vec::new()),
            config: Mutex::new(ProfilingConfig::default()),
            statistics: Mutex::new(ProfilingStatistics::default()),
            call_stack: Mutex::new(Vec::new()),
            active_session: Mutex::new(None),
            function_map: Mutex::new(BTreeMap::new()),
            name_to_id: Mutex::new(BTreeMap::new()),
            next_function_id: AtomicU64::new(1),
        }
    }
    
    pub fn start_session(&self, mode: ProfilingMode, config: Option<ProfilingConfig>) -> u64 {
        let session_id = self.session_id.fetch_add(1, Ordering::SeqCst);
        
        if let Some(cfg) = config {
            *self.config.lock() = cfg;
        }
        
        self.mode.store(mode as u8, Ordering::Release);
        self.samples.lock().clear();
        self.events.lock().clear();
        self.call_stack.lock().clear();
        
        let statistics = ProfilingStatistics::default();
        *self.statistics.lock() = statistics;
        
        let session = ProfilingSession {
            id: session_id,
            start_time: self.get_timestamp(),
            end_time: None,
            mode,
            samples: Vec::new(),
            events: Vec::new(),
            statistics,
            config: self.config.lock().clone(),
        };
        
        *self.active_session.lock() = Some(session);
        
        session_id
    }
    
    pub fn stop_session(&self) -> Option<ProfilingSession> {
        let mode = self.mode.swap(ProfilingMode::Off as u8, Ordering::AcqRel);
        if mode == ProfilingMode::Off as u8 {
            return None;
        }
        
        let end_time = self.get_timestamp();
        let samples = self.samples.lock().clone();
        let events = self.events.lock().clone();
        let statistics = self.statistics.lock().clone();
        let config = self.config.lock().clone();
        
        let mut session = self.active_session.lock();
        if let Some(mut s) = session.take() {
            s.end_time = Some(end_time);
            s.samples = samples;
            s.events = events;
            s.statistics = statistics;
            s.config = config;
            return Some(s);
        }
        
        None
    }
    
    pub fn get_mode(&self) -> ProfilingMode {
        match self.mode.load(Ordering::Acquire) {
            0 => ProfilingMode::Off,
            1 => ProfilingMode::Sampling,
            2 => ProfilingMode::Tracing,
            _ => ProfilingMode::Off,
        }
    }
    
    pub fn is_active(&self) -> bool {
        self.mode.load(Ordering::Acquire) != ProfilingMode::Off as u8
    }
    
    pub fn record_sample(&self, sample: ProfilingSample) {
        if !self.is_active() {
            return;
        }
        
        let config = self.config.lock();
        if self.samples.lock().len() >= config.max_samples {
            return;
        }
        drop(config);
        
        self.samples.lock().push(sample);
        
        let mut stats = self.statistics.lock();
        stats.total_samples += 1;
        stats.max_stack_depth = stats.max_stack_depth.max(sample.stack_frames.len());
    }
    
    pub fn record_event(&self, event: ProfilingEvent) {
        if !self.is_active() {
            return;
        }
        
        self.events.lock().push(event);
        
        let mut stats = self.statistics.lock();
        stats.total_events += 1;
        
        match &event {
            ProfilingEvent::FunctionEnter { .. } | ProfilingEvent::FunctionExit { .. } => {
                stats.function_calls += 1;
            }
            ProfilingEvent::Interrupt { .. } => {
                stats.interrupt_count += 1;
            }
            ProfilingEvent::ContextSwitch { .. } => {
                stats.context_switch_count += 1;
            }
            ProfilingEvent::PageFault { .. } => {
                stats.page_fault_count += 1;
            }
            _ => {}
        }
    }
    
    pub fn register_function(&self, info: FunctionInfo) -> u64 {
        let key = format!("{}::{}", info.module, info.name);
        
        let mut name_to_id = self.name_to_id.lock();
        if let Some(&id) = name_to_id.get(&key) {
            return id;
        }
        
        let id = self.next_function_id.fetch_add(1, Ordering::SeqCst);
        
        name_to_id.insert(key.clone(), id);
        self.function_map.lock().insert(id, info);
        
        id
    }
    
    pub fn get_function_info(&self, id: u64) -> Option<FunctionInfo> {
        self.function_map.lock().get(&id).cloned()
    }
    
    pub fn trace_function_enter(&self, function_id: u64) {
        if self.get_mode() != ProfilingMode::Tracing {
            return;
        }
        
        self.call_stack.lock().push(function_id);
        
        self.record_event(ProfilingEvent::FunctionEnter {
            function_id,
            timestamp: self.get_timestamp(),
            cpu_id: self.get_cpu_id(),
        });
    }
    
    pub fn trace_function_exit(&self, function_id: u64) {
        if self.get_mode() != ProfilingMode::Tracing {
            return;
        }
        
        let mut stack = self.call_stack.lock();
        if let Some(&top) = stack.last() {
            if top == function_id {
                stack.pop();
            }
        }
        drop(stack);
        
        self.record_event(ProfilingEvent::FunctionExit {
            function_id,
            timestamp: self.get_timestamp(),
            cpu_id: self.get_cpu_id(),
        });
    }
    
    pub fn trace_interrupt(&self, interrupt_id: u64) {
        if !self.is_active() {
            return;
        }
        
        let config = self.config.lock();
        if !config.trace_interrupts {
            return;
        }
        drop(config);
        
        self.record_event(ProfilingEvent::Interrupt {
            interrupt_id,
            timestamp: self.get_timestamp(),
            cpu_id: self.get_cpu_id(),
        });
    }
    
    pub fn trace_context_switch(&self, from_task: u64, to_task: u64) {
        if !self.is_active() {
            return;
        }
        
        let config = self.config.lock();
        if !config.trace_context_switches {
            return;
        }
        drop(config);
        
        self.record_event(ProfilingEvent::ContextSwitch {
            from_task,
            to_task,
            timestamp: self.get_timestamp(),
            cpu_id: self.get_cpu_id(),
        });
    }
    
    pub fn trace_page_fault(&self, address: u64, task_id: u64) {
        if !self.is_active() {
            return;
        }
        
        let config = self.config.lock();
        if !config.trace_page_faultes {
            return;
        }
        drop(config);
        
        self.record_event(ProfilingEvent::PageFault {
            address,
            timestamp: self.get_timestamp(),
            task_id,
        });
    }
    
    pub fn take_sample(&self) {
        if self.get_mode() != ProfilingMode::Sampling {
            return;
        }
        
        let config = self.config.lock().clone();
        drop(config);
        
        let stack = self.capture_stack_trace();
        let sample = ProfilingSample {
            timestamp: self.get_timestamp(),
            cpu_id: self.get_cpu_id(),
            task_id: self.get_current_task_id(),
            stack_frames: stack,
            event_type: SampleEventType::TimerInterrupt,
        };
        
        self.record_sample(sample);
    }
    
    pub fn build_call_graph(&self, session: &ProfilingSession) -> CallGraph {
        let mut call_tree: BTreeMap<String, (u64, u64)> = BTreeMap::new();
        
        for event in &session.events {
            if let ProfilingEvent::FunctionEnter { function_id, .. } = event {
                let key = self.get_function_key(*function_id);
                call_tree.entry(key).or_insert((0, 0)).0 += 1;
            }
        }
        
        let total_time_ns = session.end_time.unwrap_or(self.get_timestamp()) - session.start_time;
        
        CallGraph {
            root: FunctionInfo {
                id: 0,
                name: "root".to_string(),
                module: "root".to_string(),
                file: String::new(),
                line: 0,
            },
            children: BTreeMap::new(),
            total_time_ns,
            self_time_ns: total_time_ns,
            call_count: session.statistics.function_calls,
        }
    }
    
    pub fn get_hot_functions(&self, session: &ProfilingSession, top_n: usize) -> Vec<(u64, u64)> {
        let mut counts: BTreeMap<u64, u64> = BTreeMap::new();
        
        for event in &session.events {
            if let ProfilingEvent::FunctionEnter { function_id, .. } = event {
                *counts.entry(*function_id).or_insert(0) += 1;
            }
        }
        
        let mut vec: Vec<(u64, u64)> = counts.into_iter().collect();
        vec.sort_by(|a, b| b.1.cmp(&a.1));
        vec.truncate(top_n);
        
        vec
    }
    
    pub fn get_flamegraph_data(&self, session: &ProfilingSession) -> String {
        let mut lines = Vec::new();
        
        for event in &session.events {
            if let ProfilingEvent::FunctionEnter { function_id, .. } = event {
                if let Some(info) = self.get_function_info(*function_id) {
                    lines.push(format!("{} {}", info.name, 1));
                }
            }
        }
        
        lines.join("\n")
    }
    
    pub fn export_session(&self, session: &ProfilingSession, format: ExportFormat) -> Result<String, String> {
        match format {
            ExportFormat::Json => self.export_json(session),
            ExportFormat::Flamegraph => Ok(self.get_flamegraph_data(session)),
            ExportFormat::Csv => self.export_csv(session),
        }
    }
    
    fn export_json(&self, session: &ProfilingSession) -> Result<String, String> {
        Ok(format!(
            r#"{{"id":{},"start_time":{},"end_time":{:?},"samples":{},"events":{}}}"#,
            session.id,
            session.start_time,
            session.end_time,
            session.samples.len(),
            session.events.len()
        ))
    }
    
    fn export_csv(&self, session: &ProfilingSession) -> Result<String, String> {
        let mut output = String::new();
        output.push_str("timestamp,event_type,cpu_id\n");
        
        for event in &session.events {
            let event_type = match event {
                ProfilingEvent::FunctionEnter { .. } => "function_enter",
                ProfilingEvent::FunctionExit { .. } => "function_exit",
                ProfilingEvent::Interrupt { .. } => "interrupt",
                ProfilingEvent::ContextSwitch { .. } => "context_switch",
                ProfilingEvent::PageFault { .. } => "page_fault",
                ProfilingEvent::CacheMiss { .. } => "cache_miss",
            };
            
            let cpu_id = match event {
                ProfilingEvent::FunctionEnter { cpu_id, .. } => *cpu_id,
                ProfilingEvent::FunctionExit { cpu_id, .. } => *cpu_id,
                ProfilingEvent::Interrupt { cpu_id, .. } => *cpu_id,
                ProfilingEvent::ContextSwitch { cpu_id, .. } => *cpu_id,
                ProfilingEvent::PageFault { .. } => 0,
                ProfilingEvent::CacheMiss { .. } => 0,
            };
            
            let timestamp = match event {
                ProfilingEvent::FunctionEnter { timestamp, .. } => *timestamp,
                ProfilingEvent::FunctionExit { timestamp, .. } => *timestamp,
                ProfilingEvent::Interrupt { timestamp, .. } => *timestamp,
                ProfilingEvent::ContextSwitch { timestamp, .. } => *timestamp,
                ProfilingEvent::PageFault { timestamp, .. } => *timestamp,
                ProfilingEvent::CacheMiss { timestamp, .. } => *timestamp,
            };
            
            output.push_str(&format!("{},{},{}\n", timestamp, event_type, cpu_id));
        }
        
        Ok(output)
    }
    
    fn capture_stack_trace(&self) -> Vec<Frame> {
        Vec::new()
    }
    
    fn get_function_key(&self, id: u64) -> String {
        if let Some(info) = self.get_function_info(id) {
            format!("{}::{}", info.module, info.name)
        } else {
            format!("unknown_{}", id)
        }
    }
    
    fn get_timestamp(&self) -> u64 {
        0
    }
    
    fn get_cpu_id(&self) -> u64 {
        0
    }
    
    fn get_current_task_id(&self) -> Option<u64> {
        None
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

pub static PROFILER: Profiler = Profiler::new();

#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Json,
    Flamegraph,
    Csv,
}

#[macro_export]
macro_rules! profile_function {
    ($name:expr) => {
        let _profiler_guard = $crate::subsystems::perf::profiler::ProfilerGuard::new($name);
    };
}

pub struct ProfilerGuard {
    function_id: u64,
}

impl ProfilerGuard {
    pub fn new(name: &'static str) -> Self {
        let info = FunctionInfo {
            id: 0,
            name: name.to_string(),
            module: module_path!().to_string(),
            file: file!().to_string(),
            line: line!(),
        };
        
        let function_id = PROFILER.register_function(info);
        PROFILER.trace_function_enter(function_id);
        
        Self { function_id }
    }
}

impl Drop for ProfilerGuard {
    fn drop(&mut self) {
        PROFILER.trace_function_exit(self.function_id);
    }
}
