//! # 性能追踪
//!
//! 提供 kprobe 和 tracepoint 追踪功能。

use crate::prelude::*;

/// 探针类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeType {
    /// Kprobe
    Kprobe,
    /// Tracepoint
    Tracepoint,
}

/// Kprobe 探针
pub struct Kprobe {
    /// 函数名称
    func_name: String,
    /// 是否入口探针
    is_entry: bool,
    /// 触发次数
    hit_count: Mutex<u64>,
}

impl Kprobe {
    /// 创建新的 kprobe
    pub fn new(func_name: &str, is_entry: bool) -> core::result::Result<Self, PerfError> {
        Ok(Self {
            func_name: func_name.to_string(),
            is_entry,
            hit_count: Mutex::new(0),
        })
    }

    /// 获取函数名称
    pub fn func_name(&self) -> &str {
        &self.func_name
    }

    /// 是否入口探针
    pub fn is_entry(&self) -> bool {
        self.is_entry
    }

    /// 附加探针
    pub fn attach(&self) -> core::result::Result<(), PerfError> {
        log::info!("Attached kprobe to {}", self.func_name);
        Ok(())
    }

    /// 获取触发次数
    pub fn hit_count(&self) -> u64 {
        *self.hit_count.lock()
    }
}

/// Tracepoint 探针
pub struct Tracepoint {
    /// 系统名称
    system: String,
    /// 事件名称
    event: String,
    /// 触发次数
    hit_count: Mutex<u64>,
}

impl Tracepoint {
    /// 创建新的 tracepoint
    pub fn new(system: &str, event: &str) -> core::result::Result<Self, PerfError> {
        Ok(Self {
            system: system.to_string(),
            event: event.to_string(),
            hit_count: Mutex::new(0),
        })
    }

    /// 获取系统名称
    pub fn system(&self) -> &str {
        &self.system
    }

    /// 获取事件名称
    pub fn event(&self) -> &str {
        &self.event
    }

    /// 附加探针
    pub fn attach(&self) -> core::result::Result<(), PerfError> {
        log::info!("Attached tracepoint to {}:{}", self.system, self.event);
        Ok(())
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
    fn test_kprobe_create() {
        let kprobe = Kprobe::new("do_sys_open", true).unwrap();
        assert_eq!(kprobe.func_name(), "do_sys_open");
        assert!(kprobe.is_entry());
    }

    #[test]
    fn test_tracepoint_create() {
        let tp = Tracepoint::new("sched", "sched_switch").unwrap();
        assert_eq!(tp.system(), "sched");
        assert_eq!(tp.event(), "sched_switch");
    }
}
