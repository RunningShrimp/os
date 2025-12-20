//! POSIX compatibility layer for user space

/// epoll data union (simplified for user space)
#[derive(Debug, Clone, Copy)]
pub struct EpollData {
    pub u64: u64,
}

/// epoll event structure
#[repr(C)]
#[derive(Debug, Clone)]
pub struct EpollEvent {
    /// Events mask
    pub events: u32,
    /// User data
    pub data: EpollData,
}

impl Default for EpollEvent {
    fn default() -> Self {
        Self {
            events: 0,
            data: EpollData { u64: 0 },
        }
    }
}