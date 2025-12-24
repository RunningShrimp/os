use core::arch::asm;
use core::mem::MaybeUninit;

extern "C" {
    static __bss_start: u8;
    static __bss_end: u8;
    fn boot_main() -> !;
}

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    disable_interrupts();
    setup_stack();
    zero_bss();
    
    boot_main();
}

#[inline(always)]
unsafe fn disable_interrupts() {
    asm!("cli", options(nostack));
}

#[inline(always)]
unsafe fn enable_interrupts() {
    asm!("sti", options(nostack));
}

fn setup_stack() -> u64 {
    const STACK_BASE: u64 = 0x200000;
    const STACK_SIZE: usize = 16 * 1024;
    const STACK_ALIGN: u64 = 16;
    
    let stack_top = STACK_BASE + STACK_SIZE as u64;
    stack_top & !(STACK_ALIGN - 1)
}

unsafe fn zero_bss() {
    let start = &raw const __bss_start as *mut u8;
    let end = &raw const __bss_end as *const u8;
    let size = end.offset_from(start) as usize;
    
    if size > 0 {
        core::ptr::write_bytes(start, 0, size);
    }
}

#[inline(always)]
pub unsafe fn halt() -> ! {
    loop {
        asm!("hlt", options(nomem, nostack));
    }
}

#[inline(always)]
pub unsafe fn pause() {
    asm!("pause", options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn mfence() {
    asm!("mfence", options(nostack));
}

#[inline(always)]
pub unsafe fn lfence() {
    asm!("lfence", options(nostack));
}

#[inline(always)]
pub unsafe fn sfence() {
    asm!("sfence", options(nostack));
}

pub unsafe fn read_cr0() -> u64 {
    let value: u64;
    asm!("mov {}, cr0", out(reg) value, options(nomem, nostack));
    value
}

pub unsafe fn write_cr0(value: u64) {
    asm!("mov cr0, {}", in(reg) value, options(nostack));
}

pub unsafe fn read_cr2() -> u64 {
    let value: u64;
    asm!("mov {}, cr2", out(reg) value, options(nomem, nostack));
    value
}

pub unsafe fn write_cr2(value: u64) {
    asm!("mov cr2, {}", in(reg) value, options(nostack));
}

pub unsafe fn read_cr3() -> u64 {
    let value: u64;
    asm!("mov {}, cr3", out(reg) value, options(nomem, nostack));
    value
}

pub unsafe fn write_cr3(value: u64) {
    asm!("mov cr3, {}", in(reg) value, options(nostack));
}

pub unsafe fn read_cr4() -> u64 {
    let value: u64;
    asm!("mov {}, cr4", out(reg) value, options(nomem, nostack));
    value
}

pub unsafe fn write_cr4(value: u64) {
    asm!("mov cr4, {}", in(reg) value, options(nostack));
}

pub unsafe fn read_msr(msr: u32) -> u64 {
    let (high, low): (u32, u32);
    asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nomem, nostack)
    );
    ((high as u64) << 32) | (low as u64)
}

pub unsafe fn write_msr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nostack)
    );
}

pub unsafe fn read_rflags() -> u64 {
    let value: u64;
    asm!("pushfq; pop {}", out(reg) value, options(nomem, nostack));
    value
}

pub unsafe fn write_rflags(value: u64) {
    asm!("push {}; popfq", in(reg) value, options(nostack));
}

pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack));
    value
}

pub unsafe fn outb(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nostack));
}

pub unsafe fn inw(port: u16) -> u16 {
    let value: u16;
    asm!("in ax, dx", in("dx") port, out("ax") value, options(nomem, nostack));
    value
}

pub unsafe fn outw(port: u16, value: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") value, options(nostack));
}

pub unsafe fn inl(port: u16) -> u32 {
    let value: u32;
    asm!("in eax, dx", in("dx") port, out("eax") value, options(nomem, nostack));
    value
}

pub unsafe fn outl(port: u16, value: u32) {
    asm!("out dx, eax", in("dx") port, in("eax") value, options(nostack));
}

pub unsafe fn invlpg(addr: usize) {
    asm!("invlpg [{}]", in(reg) addr, options(nostack));
}

pub unsafe fn invpcid(type_: u32, descriptor: usize) {
    asm!("invpcid {}, [{}]", in(reg) type_, in(reg) descriptor, options(nostack));
}

#[inline(always)]
pub unsafe fn cpuid(eax: u32) -> (u32, u32, u32, u32) {
    let (eax, ebx, ecx, edx): (u32, u32, u32, u32);
    asm!(
        "cpuid",
        inlateout("eax") eax => eax,
        out("ebx") ebx,
        inlateout("ecx") 0 => ecx,
        out("edx") edx,
        options(nomem, nostack)
    );
    (eax, ebx, ecx, edx)
}

pub fn get_cpu_vendor_string() -> [u8; 12] {
    unsafe {
        let (_, ebx, ecx, edx) = cpuid(0);
        let mut vendor = [0u8; 12];
        
        vendor[0..4].copy_from_slice(&ebx.to_le_bytes());
        vendor[4..8].copy_from_slice(&edx.to_le_bytes());
        vendor[8..12].copy_from_slice(&ecx.to_le_bytes());
        
        vendor
    }
}

pub fn get_cpu_features() -> CpuFeatures {
    unsafe {
        let (_, ebx, ecx, edx) = cpuid(1);
        
        CpuFeatures {
            sse: edx & (1 << 25) != 0,
            sse2: edx & (1 << 26) != 0,
            sse3: ecx & 1 != 0,
            ssse3: ecx & (1 << 9) != 0,
            sse4_1: ecx & (1 << 19) != 0,
            sse4_2: ecx & (1 << 20) != 0,
            avx: ecx & (1 << 28) != 0,
            avx2: (cpuid(7).1) & (1 << 5) != 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CpuFeatures {
    pub sse: bool,
    pub sse2: bool,
    pub sse3: bool,
    pub ssse3: bool,
    pub sse4_1: bool,
    pub sse4_2: bool,
    pub avx: bool,
    pub avx2: bool,
}

#[no_mangle]
pub unsafe extern "C" fn rust_panic() -> ! {
    halt()
}

#[no_mangle]
#[link_section = ".bss"]
pub static mut BOOT_STACK: [MaybeUninit<u8>; 65536] = [MaybeUninit::uninit(); 65536];

#[no_mangle]
pub static BOOT_STACK_TOP: *const u8 = unsafe { BOOT_STACK.as_ptr().add(65536) };
