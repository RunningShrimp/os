// Boot state tracking for debugging and diagnostics

use core::sync::atomic::{AtomicU32, Ordering};

#[repr(u32)]
pub enum BootPhase {
    Start = 0,
    ArchInit = 1,
    GdtLoaded = 2,
    IdtLoaded = 3,
    MemoryInit = 4,
    InterruptInit = 5,
    PagingInit = 6,
    BootInfoCreated = 7,
    KernelLoaded = 8,
    ReadyToJump = 9,
    Error = 0xFFFF_FFFF,
}

pub struct BootState {
    current_phase: AtomicU32,
}

impl BootState {
    pub fn new() -> Self {
        Self {
            current_phase: AtomicU32::new(BootPhase::Start as u32),
        }
    }

    pub fn set_phase(&self, phase: BootPhase) {
        self.current_phase
            .store(phase as u32, Ordering::Release);
    }

    pub fn get_phase(&self) -> u32 {
        self.current_phase.load(Ordering::Acquire)
    }

    pub fn set_error(&self) {
        self.current_phase
            .store(BootPhase::Error as u32, Ordering::Release);
    }

    pub fn is_error(&self) -> bool {
        self.get_phase() == BootPhase::Error as u32
    }
}

impl Default for BootState {
    fn default() -> Self {
        Self::new()
    }
}

pub static BOOT_STATE: BootState = BootState {
    current_phase: AtomicU32::new(0),
};

pub fn set_phase(phase: BootPhase) {
    BOOT_STATE.set_phase(phase);
}

pub fn get_phase() -> u32 {
    BOOT_STATE.get_phase()
}

pub fn set_error() {
    BOOT_STATE.set_error();
}
