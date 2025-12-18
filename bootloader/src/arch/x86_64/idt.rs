// x86_64 Interrupt Descriptor Table

use core::mem::size_of;

#[repr(C, packed)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    flags: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    pub fn new(handler: u64, flags: u8) -> Self {
        Self {
            offset_low: (handler & 0xFFFF) as u16,
            selector: 0x08, // Kernel code segment
            ist: 0,
            flags,
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: ((handler >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }

    pub fn null() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            flags: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }
}

pub const IDT_FLAG_PRESENT: u8 = 0x80;
pub const IDT_FLAG_INTERRUPT: u8 = 0x0E;
pub const IDT_FLAG_TRAP: u8 = 0x0F;

#[repr(C, packed)]
pub struct IdtPointer {
    pub limit: u16,
    pub base: u64,
}

pub static mut IDT: [IdtEntry; 256] = [IdtEntry {
    offset_low: 0,
    selector: 0,
    ist: 0,
    flags: 0,
    offset_mid: 0,
    offset_high: 0,
    reserved: 0,
}; 256];

pub fn load_idt() {
    unsafe {
        let idt_ptr = IdtPointer {
            limit: (size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: &IDT as *const _ as u64,
        };

        core::arch::asm!(
            "lidt [{}]",
            in(reg) &idt_ptr,
            options(nostack)
        );
    }
}

pub fn set_handler(index: usize, handler: u64) {
    if index < 256 {
        unsafe {
            IDT[index] = IdtEntry::new(
                handler,
                IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT,
            );
        }
    }
}
