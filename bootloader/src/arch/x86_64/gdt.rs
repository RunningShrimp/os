// x86_64 Global Descriptor Table

use core::mem::size_of;

#[repr(C, packed)]
pub struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

impl GdtEntry {
    pub fn new(base: u32, limit: u32, access: u8, gran: u8) -> Self {
        Self {
            limit_low: (limit & 0xFFFF) as u16,
            base_low: (base & 0xFFFF) as u16,
            base_mid: ((base >> 16) & 0xFF) as u8,
            access,
            granularity: (((limit >> 16) & 0x0F) as u8) | (gran & 0xF0),
            base_high: ((base >> 24) & 0xFF) as u8,
        }
    }

    pub fn null() -> Self {
        Self {
            limit_low: 0,
            base_low: 0,
            base_mid: 0,
            access: 0,
            granularity: 0,
            base_high: 0,
        }
    }
}

pub const GDT_ACCESS_KERNEL_CODE: u8 = 0x9A;
pub const GDT_ACCESS_KERNEL_DATA: u8 = 0x92;
pub const GDT_GRANULARITY_4K: u8 = 0xC0;
pub const GDT_GRANULARITY_1B: u8 = 0x00;

#[repr(C, packed)]
pub struct GdtPointer {
    pub limit: u16,
    pub base: u32,
}

pub static mut GDT: [GdtEntry; 3] = [
    // Null descriptor
    GdtEntry {
        limit_low: 0,
        base_low: 0,
        base_mid: 0,
        access: 0,
        granularity: 0,
        base_high: 0,
    },
    // Kernel code
    GdtEntry {
        limit_low: 0xFFFF,
        base_low: 0,
        base_mid: 0,
        access: 0x9A,
        granularity: 0xCF,
        base_high: 0,
    },
    // Kernel data
    GdtEntry {
        limit_low: 0xFFFF,
        base_low: 0,
        base_mid: 0,
        access: 0x92,
        granularity: 0xCF,
        base_high: 0,
    },
];

pub fn load_gdt() {
    unsafe {
        let gdt_ptr = GdtPointer {
            limit: (size_of::<[GdtEntry; 3]>() - 1) as u16,
            base: &GDT as *const _ as u32,
        };

        core::arch::asm!(
            "lgdt [{}]",
            in(reg) &gdt_ptr,
            options(nostack)
        );

        // Load code segment (0x08 = offset in GDT)
        core::arch::asm!(
            "pushl $0x08",
            "pushl $1f",
            "lretl",
            "1:",
            options(nostack)
        );

        // Load data segments (0x10 = offset in GDT)
        core::arch::asm!(
            "movw $0x10, %ax",
            "movw %ax, %ds",
            "movw %ax, %es",
            "movw %ax, %fs",
            "movw %ax, %gs",
            "movw %ax, %ss",
            options(nostack)
        );
    }
}
