// Boot handoff - prepare for kernel jump

use core::mem::size_of;

#[repr(C)]
pub struct BootHandoff {
    pub magic: u32,
    pub version: u32,
    pub kernel_entry: u64,
    pub boot_params: u64,
    pub kernel_cmdline: u64,
    pub memory_map_addr: u64,
    pub memory_map_size: u32,
    pub framebuffer_addr: u64,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub framebuffer_pitch: u32,
    pub cpu_count: u32,
    pub reserved: [u64; 8],
}

impl BootHandoff {
    pub const MAGIC: u32 = 0x5A5A5A5A;
    pub const VERSION: u32 = 1;

    pub fn new() -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            kernel_entry: 0,
            boot_params: 0,
            kernel_cmdline: 0,
            memory_map_addr: 0,
            memory_map_size: 0,
            framebuffer_addr: 0,
            framebuffer_width: 0,
            framebuffer_height: 0,
            framebuffer_pitch: 0,
            cpu_count: 1,
            reserved: [0; 8],
        }
    }

    pub fn set_kernel_entry(&mut self, entry: u64) {
        self.kernel_entry = entry;
    }

    pub fn set_boot_params(&mut self, params: u64) {
        self.boot_params = params;
    }

    pub fn set_cmdline(&mut self, addr: u64) {
        self.kernel_cmdline = addr;
    }

    pub fn set_memory_map(&mut self, addr: u64, size: u32) {
        self.memory_map_addr = addr;
        self.memory_map_size = size;
    }

    pub fn set_framebuffer(
        &mut self,
        addr: u64,
        width: u32,
        height: u32,
        pitch: u32,
    ) {
        self.framebuffer_addr = addr;
        self.framebuffer_width = width;
        self.framebuffer_height = height;
        self.framebuffer_pitch = pitch;
    }

    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC && self.kernel_entry != 0
    }

    pub fn size() -> usize {
        size_of::<Self>()
    }
}

impl Default for BootHandoff {
    fn default() -> Self {
        Self::new()
    }
}

pub static mut BOOT_HANDOFF: Option<BootHandoff> = None;

pub fn init_handoff() -> &'static mut BootHandoff {
    unsafe {
        BOOT_HANDOFF = Some(BootHandoff::new());
        match &mut *(&raw mut BOOT_HANDOFF) {
            Some(h) => h,
            None => panic!("Failed to init handoff"),
        }
    }
}

pub fn get_handoff() -> Option<&'static mut BootHandoff> {
    unsafe { (*(&raw mut BOOT_HANDOFF)).as_mut() }
}

/// Prepare for kernel jump - setup all parameters
pub fn prepare_kernel_jump(
    entry: u64,
    params: u64,
) -> Option<BootHandoff> {
    let mut handoff = BootHandoff::new();
    handoff.set_kernel_entry(entry);
    handoff.set_boot_params(params);

    if handoff.is_valid() {
        Some(handoff)
    } else {
        None
    }
}
