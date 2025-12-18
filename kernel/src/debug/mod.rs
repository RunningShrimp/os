// Advanced Debugging Module
//
// 高级调试模块
// 提供系统调试、性能分析、内存分析等调试功能

// 现有子模块
pub mod log;
pub mod tracing;
pub mod profiling;
pub mod metrics;
pub mod monitoring;
pub mod symbols;
pub mod visualization;
pub mod fault_diagnosis;

// 新拆分的子模块
pub mod session;
pub mod breakpoint;
pub mod analyzer;
pub mod plugin;
pub mod types;
pub mod manager;

// 重新导出公共类型和函数
pub use session::{
    DebugSession, DebugSessionType, DebugSessionStatus,
    ProcessInfo, ProcessState, ProcessMemoryUsage,
    DebugEvent, DebugEventType, DebugLevel,
    ThreadInfo, ThreadState, SessionConfig,
};

pub use breakpoint::{
    BreakpointManager, Breakpoint, BreakpointType, BreakpointStatus,
    BreakpointCondition, ConditionType, SourceLocation,
};

pub use analyzer::{
    MemoryAnalyzer, MemorySnapshot, MemoryRegion, MemoryRegionType, MemoryPermissions,
    LeakDetector, AllocationRecord, DeallocationRecord, AllocationType, DeallocationType,
    LeakDetectionStats, MemoryUsageStatistics, MemoryMapping, MappingType, MappingStatus,
    StackAnalyzer, StackTrace, StackTraceType, StackOverflowDetector,
    StackFrame, CallFrame, VariableInfo,
    PerformanceAnalyzer, PerformanceCounter, CounterType, PerformanceSample,
    HotspotAnalysis, HotspotFunction, HotspotLine, HotspotModule,
    PerformanceReport, ReportType, TimeRange, PerformanceSummary,
    AnalysisResult, AnalysisResultType, AnalysisImportance,
    Recommendation, RecommendationType, RecommendationPriority, ImplementationDifficulty,
    PerformanceAnalysisConfig, SystemState, CPUInfo, MemoryInfo, NetworkInfo,
    InterfaceStatus, InterfaceState, InterfaceType, PerformanceSnapshot,
    HotspotAnalysisConfig,
};

pub use plugin::{
    DebugPlugin, PluginType, PluginConfig, PluginInterface,
};

pub use types::{
    SymbolManager, SymbolTable, SymbolTableType, Symbol, SymbolType, SymbolScope,
    SymbolTableStats, DebugInfo, DebugFormat, DebugFeature,
    SourceMapping, LineMapping, DebugConfig, DebugStats,
};

pub use manager::{
    DebugManager, create_debug_manager,
};

/// Initialize debug module (module-level function)
pub fn init() -> Result<(), &'static str> {
    // Initialize all debug subsystems
    if let Err(_) = metrics::init() {
        return Err("Failed to initialize metrics subsystem");
    }
    
    if let Err(_) = profiling::init() {
        return Err("Failed to initialize profiling subsystem");
    }
    
    if let Err(_) = monitoring::init() {
        return Err("Failed to initialize monitoring subsystem");
    }
    
    if let Err(_) = visualization::init() {
        return Err("Failed to initialize visualization subsystem");
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_manager_creation() {
        let manager = DebugManager::new();
        assert_eq!(manager.id, 1);
        assert!(manager.active_sessions.is_empty());
        assert!(manager.debug_plugins.is_empty());
    }

    #[test]
    fn test_debug_session_creation() {
        let mut manager = DebugManager::new();
        let session_id = manager.start_debug_session("test_session", DebugSessionType::ProcessDebug, None).unwrap();
        assert_eq!(manager.active_sessions.len(), 1);

        let session = manager.get_active_sessions().get(&session_id).unwrap();
        assert_eq!(session.name, "test_session");
        assert_eq!(session.session_type, DebugSessionType::ProcessDebug);
    }

    #[test]
    fn test_breakpoint_management() {
        let mut manager = DebugManager::new();
        manager.init().unwrap();

        let breakpoint_id = manager.set_breakpoint(0x400100, BreakpointType::Software, Some("Test breakpoint")).unwrap();
        assert_eq!(manager.breakpoint_manager.breakpoints.len(), 1);
        assert_eq!(breakpoint_id, 1);

        manager.remove_breakpoint(breakpoint_id).unwrap();
        assert_eq!(manager.breakpoint_manager.breakpoints.len(), 0);
    }

    #[test]
    fn test_memory_analyzer() {
        let mut manager = DebugManager::new();
        manager.init().unwrap();

        let snapshot_id = manager.create_memory_snapshot(1, 1).unwrap();
        assert_eq!(manager.memory_analyzer.memory_snapshots.len(), 1);
        assert_eq!(snapshot_id, 1);
    }

    #[test]
    fn test_performance_analyzer() {
        let mut manager = DebugManager::new();
        manager.init().unwrap();

        let sample_id = manager.create_performance_sample(1, 1).unwrap();
        assert!(!manager.performance_analyzer.sampling_data.is_empty());
        assert_eq!(manager.performance_analyzer.sampling_data.len(), 1);
        assert!(sample_id.starts_with("sample_"));
    }

    #[test]
    fn test_debug_config_default() {
        let config = DebugConfig::default();
        assert!(!config.enable_auto_debug);
        assert_eq!(config.max_concurrent_sessions, 5);
        assert_eq!(config.default_debug_level, DebugLevel::Info);
        assert!(config.enable_performance_analysis);
        assert!(config.enable_memory_analysis);
    }
}
