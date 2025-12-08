// Device drivers for xv6-rust
// Provides block device abstraction and implementations

pub mod uart;
pub mod console;
pub mod device_manager;
pub mod syscon;
pub mod gic;
pub mod gicv3;
pub mod platform;
pub mod nvme;
pub mod usb;
pub mod virtio_gpu;

use crate::sync::Mutex;
use crate::posix;

// Re-export driver manager functions
pub use crate::services::driver::get_driver_manager;

// ============================================================================
// Block Device Trait
// ============================================================================

/// Block device trait for storage devices
pub trait BlockDevice: Send + Sync {
    /// Read a block from the device
    fn read(&self, lba: usize, buf: &mut [u8]);
    
    /// Write a block to the device
    fn write(&self, lba: usize, buf: &[u8]);
    
    /// Get block size in bytes
    fn block_size(&self) -> usize {
        512
    }
    
    /// Get total number of blocks
    fn num_blocks(&self) -> usize;
    
    /// Flush any cached writes
    fn flush(&self) {}
}

// ============================================================================
// RAM Disk Implementation
// ============================================================================

/// Sector size
pub const SECTOR_SIZE: usize = 512;

/// RAM disk size (64KB)
pub const RAMDISK_SIZE: usize = 64 * 1024;

/// RAM disk storage
static mut RAMDISK_DATA: [u8; RAMDISK_SIZE] = [0; RAMDISK_SIZE];

/// RAM disk device
pub struct RamDisk;

impl BlockDevice for RamDisk {
    fn read(&self, lba: usize, buf: &mut [u8]) {
        let offset = lba * SECTOR_SIZE;
        let len = buf.len().min(SECTOR_SIZE);
        if offset + len <= RAMDISK_SIZE {
            unsafe {
                buf[..len].copy_from_slice(&RAMDISK_DATA[offset..offset + len]);
            }
        }
    }

    fn write(&self, lba: usize, buf: &[u8]) {
        let offset = lba * SECTOR_SIZE;
        let len = buf.len().min(SECTOR_SIZE);
        if offset + len <= RAMDISK_SIZE {
            unsafe {
                RAMDISK_DATA[offset..offset + len].copy_from_slice(&buf[..len]);
            }
        }
    }

    fn num_blocks(&self) -> usize {
        RAMDISK_SIZE / SECTOR_SIZE
    }
}

// ============================================================================
// VirtIO Block Device (placeholder)
// ============================================================================

/// VirtIO block device configuration
#[allow(dead_code)]
pub struct VirtioBlk {
    base: usize,
    capacity: u64,
}

#[allow(dead_code)]
impl VirtioBlk {
    pub fn new(base: usize) -> Option<Self> {
        // TODO: Initialize VirtIO device
        Some(Self {
            base,
            capacity: 0,
        })
    }

    /// Probe for VirtIO device
    pub fn probe(base: usize) -> bool {
        // Check magic number
        let magic = crate::mm::mmio_read32(base as *const u32);
        magic == 0x74726976 // "virt"
    }
}

impl BlockDevice for VirtioBlk {
    fn read(&self, _lba: usize, _buf: &mut [u8]) {
        // TODO: Implement VirtIO read
    }

    fn write(&self, _lba: usize, _buf: &[u8]) {
        // TODO: Implement VirtIO write
    }

    fn num_blocks(&self) -> usize {
        self.capacity as usize
    }
}

// ============================================================================
// Console Device
// ============================================================================

/// Console input buffer size
const CONSOLE_BUF_SIZE: usize = 128;

/// Console device state
pub struct Console {
    buf: [u8; CONSOLE_BUF_SIZE],
    read_idx: usize,
    write_idx: usize,
}

impl Console {
    pub const fn new() -> Self {
        Self {
            buf: [0; CONSOLE_BUF_SIZE],
            read_idx: 0,
            write_idx: 0,
        }
    }

    /// Add character to input buffer
    pub fn push(&mut self, c: u8) {
        let next = (self.write_idx + 1) % CONSOLE_BUF_SIZE;
        if next != self.read_idx {
            self.buf[self.write_idx] = c;
            self.write_idx = next;
        }
    }

    /// Get character from input buffer
    pub fn pop(&mut self) -> Option<u8> {
        if self.read_idx == self.write_idx {
            None
        } else {
            let c = self.buf[self.read_idx];
            self.read_idx = (self.read_idx + 1) % CONSOLE_BUF_SIZE;
            Some(c)
        }
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.read_idx == self.write_idx
    }

    /// Get number of characters in buffer
    pub fn len(&self) -> usize {
        if self.write_idx >= self.read_idx {
            self.write_idx - self.read_idx
        } else {
            CONSOLE_BUF_SIZE - self.read_idx + self.write_idx
        }
    }
}

static CONSOLE: Mutex<Console> = Mutex::new(Console::new());
static CONSOLE_SUBS: Mutex<Vec<usize>> = Mutex::new(Vec::new());

/// Handle console interrupt (character received)
pub fn console_intr(c: u8) {
    let mut console = CONSOLE.lock();
    
    // Handle special characters
    match c {
        // Backspace
        0x7F | 0x08 => {
            // TODO: Handle backspace
        }
        // Ctrl-C
        0x03 => {
            // TODO: Send SIGINT
        }
        // Ctrl-D (EOF)
        0x04 => {
            console.push(c);
        }
        // Regular character
        _ => {
            console.push(c);
            // Echo
            crate::drivers::uart::write_byte(c);
            if c == b'\r' {
                crate::drivers::uart::write_byte(b'\n');
            }
        }
    }
    
    // Wake up processes waiting for input
    crate::process::wakeup(&CONSOLE as *const _ as usize);
    let subs = CONSOLE_SUBS.lock();
    for &chan in subs.iter() { crate::process::wakeup(chan); }
}

/// Read from console
pub fn console_read(buf: &mut [u8]) -> usize {
    let mut count = 0;
    
    for byte in buf.iter_mut() {
        loop {
            let mut console = CONSOLE.lock();
            if let Some(c) = console.pop() {
                *byte = c;
                count += 1;
                break;
            }
            drop(console);
            
            // Sleep waiting for input
            crate::process::sleep(&CONSOLE as *const _ as usize);
        }
    }
    
    count
}

/// Write to console
pub fn console_write(buf: &[u8]) -> usize {
    for &byte in buf {
        crate::drivers::uart::write_byte(byte);
    }
    buf.len()
}

pub fn device_poll(major: i16, _minor: i16) -> i16 {
    match major {
        1 => {
            let c = CONSOLE.lock();
            let mut ev: i16 = 0;
            if !c.is_empty() { ev |= posix::POLLIN; }
            ev |= posix::POLLOUT;
            if !CONSOLE_SUBS.lock().is_empty() { ev |= posix::POLLPRI; }
            ev
        }
        _ => posix::POLLERR,
    }
}

pub fn device_subscribe(major: i16, _minor: i16, _events: i16, chan: usize) {
    match major {
        1 => {
            let mut subs = CONSOLE_SUBS.lock();
            if !subs.contains(&chan) { subs.push(chan); }
        }
        _ => {}
    }
}

pub fn device_unsubscribe(major: i16, _minor: i16, chan: usize) {
    match major {
        1 => {
            let mut subs = CONSOLE_SUBS.lock();
            if let Some(pos) = subs.iter().position(|c| *c == chan) { subs.remove(pos); }
        }
        _ => {}
    }
}

// ============================================================================
// Device initialization
// ============================================================================

/// Initialize all devices
pub fn init() {
    // RAM disk is always available
    crate::println!("drivers: ramdisk {} blocks", RamDisk.num_blocks());
    
    // TODO: Probe for other devices (VirtIO, etc.)
    
    #[cfg(target_arch = "aarch64")]
    {
        if let Some((dist, redist)) = crate::drivers::platform::gicv3_bases() {
            let gic = crate::drivers::gicv3::GicV3::new(dist, redist);
            gic.enable();
            crate::println!("drivers: gicv3 enabled dist={:#x} redist={:#x}", dist, redist);
        } else if let Some((dist, cpu)) = crate::drivers::platform::gicv2_bases() {
            let gic = crate::drivers::gic::GicV2::new(dist, cpu);
            gic.enable();
            crate::println!("drivers: gicv2 enabled dist={:#x} cpu={:#x}", dist, cpu);
        } else {
            crate::println!("drivers: gic not found in DTB; skipping init");
        }
    }
    crate::println!("drivers: initialized");
}

pub fn init_ap() {
    #[cfg(target_arch = "aarch64")]
    {
        if let Some((dist, _)) = crate::drivers::platform::gicv3_bases() {
            let mpidr: u64;
            unsafe { core::arch::asm!("mrs {}, mpidr_el1", out(reg) mpidr); }
            let redist = crate::drivers::platform::gicr_lookup(mpidr).or_else(|| crate::drivers::platform::gicr_default());
            if let Some(r) = redist {
                let gic = crate::drivers::gicv3::GicV3::new(dist, r);
                gic.cpu_enable();
                crate::println!("drivers: gicv3 cpu enabled redist={:#x}", r);
            } else {
                crate::println!("drivers: gicv3 cpu enable skipped (no redist)");
            }
        } else if let Some((dist, cpu)) = crate::drivers::platform::gicv2_bases() {
            let gic = crate::drivers::gic::GicV2::new(dist, cpu);
            gic.cpu_enable();
            crate::println!("drivers: gicv2 cpu enabled");
        }
    }
}
extern crate alloc;
use alloc::vec::Vec;
