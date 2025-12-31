//! # 性能事件抽象
//!
//! 提供硬件和软件性能事件的统一接口。

use crate::prelude::*;

/// 性能事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfEventType {
    /// 硬件事件
    Hardware,
    /// 软件事件
    Software,
}

/// 硬件事件配置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum PerfHWEvent {
    /// CPU 周期
    CpuCycles = 0,
    /// 指令 retired
    Instructions = 1,
}

/// 性能事件属性
#[derive(Debug, Clone)]
pub struct PerfEventAttr {
    /// 事件类型
    pub event_type: PerfEventType,
    /// 事件配置
    pub config: u64,
    /// 采样周期
    pub sample_period: u64,
}

impl PerfEventAttr {
    /// 创建新的性能事件属性
    pub fn new(event_type: PerfEventType) -> Self {
        Self {
            event_type,
            config: 0,
            sample_period: 0,
        }
    }

    /// 设置硬件事件
    pub fn set_hw_event(mut self, event: PerfHWEvent) -> Self {
        self.config = event as u64;
        self
    }

    /// 设置采样周期
    pub fn set_sample_period(mut self, period: u64) -> Self {
        self.sample_period = period;
        self
    }
}

/// 性能事件计数
#[derive(Debug, Clone, Copy, Default)]
pub struct PerfCount {
    /// 计数值
    pub value: u64,
    /// 时间启用
    pub time_enabled: u64,
    /// 时间运行
    pub time_running: u64,
}

/// 性能事件
pub struct PerfEvent {
    /// 事件属性
    attr: PerfEventAttr,
    /// 事件 ID
    id: u64,
    /// 是否启用
    enabled: Mutex<bool>,
    /// 计数值
    count: Mutex<PerfCount>,
}

impl PerfEvent {
    /// 创建新的性能事件
    pub fn new(attr: PerfEventAttr) -> core::result::Result<Self, PerfError> {
        let id = Self::allocate_id();

        Ok(Self {
            attr,
            id,
            enabled: Mutex::new(false),
            count: Mutex::new(PerfCount::default()),
        })
    }

    /// 分配事件 ID
    fn allocate_id() -> u64 {
        static NEXT_ID: Mutex<u64> = Mutex::new(1);

        let mut id = NEXT_ID.lock();
        let current = *id;
        *id += 1;
        current
    }

    /// 启用事件
    pub fn enable(&self) -> core::result::Result<(), PerfError> {
        *self.enabled.lock() = true;
        log::debug!("Enabled perf event {}", self.id);
        Ok(())
    }

    /// 禁用事件
    pub fn disable(&self) -> core::result::Result<(), PerfError> {
        *self.enabled.lock() = false;
        Ok(())
    }

    /// 重置计数
    pub fn reset(&self) -> core::result::Result<(), PerfError> {
        let mut count = self.count.lock();
        count.value = 0;
        Ok(())
    }

    /// 读取计数值
    pub fn read(&self) -> core::result::Result<PerfCount, PerfError> {
        let count = self.count.lock();
        Ok(*count)
    }

    /// 获取事件 ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// 获取事件属性
    pub fn attr(&self) -> &PerfEventAttr {
        &self.attr
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

/// 性能事件管理器
pub struct PerfEventManager {
    events: Mutex<BTreeMap<u64, alloc::sync::Arc<PerfEvent>>>,
}

impl PerfEventManager {
    /// 创建新的管理器
    pub fn new() -> Self {
        Self {
            events: Mutex::new(BTreeMap::new()),
        }
    }

    /// 创建事件
    pub fn create_event(&self, attr: PerfEventAttr) -> core::result::Result<u64, PerfError> {
        let event = PerfEvent::new(attr)?;
        let id = event.id();

        let mut events = self.events.lock();
        events.insert(id, alloc::sync::Arc::new(event));

        Ok(id)
    }

    /// 获取事件
    pub fn get_event(&self, id: u64) -> Option<alloc::sync::Arc<PerfEvent>> {
        let events = self.events.lock();
        events.get(&id).cloned()
    }
}

impl Default for PerfEventManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perf_event_attr() {
        let attr = PerfEventAttr::new(PerfEventType::Hardware)
            .set_hw_event(PerfHWEvent::CpuCycles)
            .set_sample_period(1000);

        assert_eq!(attr.event_type, PerfEventType::Hardware);
        assert_eq!(attr.config, PerfHWEvent::CpuCycles as u64);
    }

    #[test]
    fn test_perf_event_create() {
        let attr = PerfEventAttr::new(PerfEventType::Hardware)
            .set_hw_event(PerfHWEvent::Instructions);

        let event = PerfEvent::new(attr);
        assert!(event.is_ok());
    }
}
