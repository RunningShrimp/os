//! # eBPF 子系统
//!
//! 扩展伯克利包过滤器（Extended Berkeley Packet Filter）子系统。
//!
//! ## 主要组件
//!
//! - [`verifier`]: eBPF 指令验证器
//! - [`programs`]: eBPF 程序管理
//! - [`maps`]: eBPF maps 数据结构
//! - [`tracer`]: eBPF 追踪器（kprobe/tracepoint/uprobe）

pub mod verifier;
pub mod verifier_enhanced;
pub mod programs;
pub mod maps;
pub mod maps_advanced;
pub mod tracer;

use crate::prelude::*;

// Re-export commonly used types
pub use verifier::{BpfVerifier, BpfVerifierConfig, VerifierError};
pub use verifier_enhanced::{
    EnhancedVerifier,
    EnhancedVerifierConfig,
    EnhancedVerifierError,
    EnhancedVerifierResult,
    TaintAnalyzer,
    LoopDetector,
    ResourceLimiter,
    TaintSource,
    RegisterType,
    ValueRange,
};
pub use programs::{BpfProgram, BpfProgramType};
pub use maps::{BpfMap, BpfMapType};

// Re-export advanced map types
pub use maps_advanced::{
    RingBufferMap, PerfEventMap, BloomFilter, SkewSketch,
    BpfMapBatchOps, BpfMapPersistence, BpfMapStats,
    RingBufferStatistics, PerfEventStatistics, BloomFilterStatistics,
    SkewSketchStatistics, MapStatistics, PerfEventType, PerfEventConfig,
    PerfEventAttrs,
};

// Re-export tracer types
pub use tracer::{
    BpfTracer,
    TracerConfig,
    TracerError,
    TracerResult,
    TraceEvent,
    TraceEventType,
    EventFilter,
    KprobeType,
    KprobeHandler,
    TracepointHandler,
    UprobeHandler,
    PerCpuBufferStats,
    ProbeStats,
};

/// eBPF 错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BpfError {
    /// 验证失败
    VerificationFailed(String),
    /// 程序过大
    ProgramTooLarge,
    /// 指令数超限
    InstructionLimitExceeded,
    /// 资源不足
    InsufficientResources,
    /// Map 不存在
    MapNotFound(i32),
    /// 程序不存在
    ProgramNotFound(i32),
}

/// eBPF 配置
#[derive(Debug, Clone)]
pub struct BpfConfig {
    /// 最大指令数
    pub max_instructions: usize,
    /// 最大程序数
    pub max_programs: usize,
    /// 最大 map 数
    pub max_maps: usize,
}

impl Default for BpfConfig {
    fn default() -> Self {
        Self {
            max_instructions: 4096,
            max_programs: 256,
            max_maps: 512,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpf_config_default() {
        let config = BpfConfig::default();
        assert_eq!(config.max_instructions, 4096);
    }
}
