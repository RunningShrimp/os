//! # 火焰图生成
//!
//! 提供性能分析火焰图的数据收集功能。

use crate::prelude::*;

/// 调用栈帧
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackFrame {
    /// 函数地址
    pub addr: u64,
    /// 函数名称
    pub name: String,
}

impl StackFrame {
    /// 创建新的栈帧
    pub fn new(addr: u64, name: String) -> Self {
        Self { addr, name }
    }
}

/// 调用栈样本
#[derive(Debug, Clone)]
pub struct StackSample {
    /// PID
    pub pid: u32,
    /// 时间戳
    pub timestamp: u64,
    /// 调用栈
    pub stack: Vec<StackFrame>,
}

impl StackSample {
    /// 创建新的栈样本
    pub fn new(pid: u32, timestamp: u64) -> Self {
        Self {
            pid,
            timestamp,
            stack: Vec::new(),
        }
    }

    /// 添加栈帧
    pub fn push_frame(&mut self, frame: StackFrame) {
        self.stack.push(frame);
    }

    /// 获取栈深度
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

/// 火焰图配置
#[derive(Debug, Clone)]
pub struct FlameGraphConfig {
    /// 采样频率 (Hz)
    pub sample_freq: u32,
    /// 采样时长（秒）
    pub duration: u32,
}

impl Default for FlameGraphConfig {
    fn default() -> Self {
        Self {
            sample_freq: 99,
            duration: 10,
        }
    }
}

/// 火焰图收集器
pub struct FlameGraphCollector {
    /// 配置
    config: FlameGraphConfig,
    /// 是否正在采样
    sampling: Mutex<bool>,
    /// 收集的样本
    samples: Mutex<Vec<StackSample>>,
}

impl FlameGraphCollector {
    /// 创建新的火焰图收集器
    pub fn new(config: FlameGraphConfig) -> core::result::Result<Self, PerfError> {
        Ok(Self {
            config,
            sampling: Mutex::new(false),
            samples: Mutex::new(Vec::new()),
        })
    }

    /// 开始采样
    pub fn start_sampling(&self) -> core::result::Result<(), PerfError> {
        *self.sampling.lock() = true;

        log::info!(
            "Started flamegraph sampling at {}Hz for {}s",
            self.config.sample_freq,
            self.config.duration
        );

        Ok(())
    }

    /// 停止采样
    pub fn stop_sampling(&self) -> core::result::Result<(), PerfError> {
        *self.sampling.lock() = false;
        log::info!("Stopped flamegraph sampling");
        Ok(())
    }

    /// 添加栈样本
    pub fn add_sample(&self, sample: StackSample) {
        if *self.sampling.lock() {
            let mut samples = self.samples.lock();
            samples.push(sample);
        }
    }

    /// 获取样本数量
    pub fn sample_count(&self) -> usize {
        self.samples.lock().len()
    }
}

/// 性能错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PerfError {
    /// 事件不存在
    EventNotFound,
    /// 权限不足
    PermissionDenied,
    /// 不支持的事件
    UnsupportedEvent,
    /// 无效参数
    InvalidParameter,
    /// 资源不足
    InsufficientResources,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_frame() {
        let frame = StackFrame::new(0x1000, "test_func".to_string());
        assert_eq!(frame.addr, 0x1000);
        assert_eq!(frame.name, "test_func");
    }

    #[test]
    fn test_stack_sample() {
        let mut sample = StackSample::new(1, 100);
        assert_eq!(sample.depth(), 0);

        sample.push_frame(StackFrame::new(0x1000, "func1".to_string()));
        assert_eq!(sample.depth(), 1);
    }

    #[test]
    fn test_flamegraph_collector() {
        let config = FlameGraphConfig::default();
        let collector = FlameGraphCollector::new(config);

        assert!(collector.is_ok());
    }

    #[test]
    fn test_flamegraph_config() {
        let config = FlameGraphConfig::default();
        assert_eq!(config.sample_freq, 99);
        assert_eq!(config.duration, 10);
    }
}
