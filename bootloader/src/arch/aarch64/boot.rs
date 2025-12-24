use core::arch::asm;
use core::mem::MaybeUninit;

extern "C" {
    static __bss_start: u8;
    static __bss_end: u8;
    fn boot_main() -> !;
}

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    disable_all_interrupts();
    setup_stack();
    zero_bss();
    enable_fpu();
    memory_barriers();
    
    boot_main();
}

#[inline(always)]
unsafe fn disable_all_interrupts() {
    asm!("msr daifset, #0xf", options(nomem, nostack));
}

#[inline(always)]
unsafe fn enable_interrupts() {
    asm!("msr daifclr, #0xf", options(nomem, nostack));
}

#[inline(always)]
unsafe fn disable_irq() {
    asm!("msr daifset, #2", options(nomem, nostack));
}

#[inline(always)]
unsafe fn enable_irq() {
    asm!("msr daifclr, #2", options(nomem, nostack));
}

fn setup_stack() -> u64 {
    const STACK_BASE: u64 = 0x200000;
    const STACK_SIZE: usize = 16 * 1024;
    
    STACK_BASE + STACK_SIZE as u64
}

unsafe fn zero_bss() {
    let start = &raw const __bss_start as *mut u8;
    let end = &raw const __bss_end as *const u8;
    let size = end.offset_from(start) as usize;
    
    if size > 0 {
        core::ptr::write_bytes(start, 0, size);
    }
}

unsafe fn enable_fpu() {
    let mut cpacr: u64;
    asm!("mrs {}, cpacr_el1", out(reg) cpacr, options(nomem, nostack));
    cpacr |= 3 << 20;
    asm!("msr cpacr_el1, {}", in(reg) cpacr, options(nostack));
}

unsafe fn memory_barriers() {
    asm!("dsb sy", options(nomem, nostack));
    asm!("isb", options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn halt() -> ! {
    loop {
        asm!("wfi", options(nomem, nostack));
    }
}

#[inline(always)]
pub unsafe fn dsb(option: u8) {
    asm!("dsb #{}", in(reg) option, options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn dmb(option: u8) {
    asm!("dmb #{}", in(reg) option, options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn isb() {
    asm!("isb", options(nomem, nostack));
}

pub unsafe fn read_msr(reg: u32) -> u64 {
    let value: u64;
    asm!("mrs {}, {}", out(reg) value, in(reg) reg, options(nomem, nostack));
    value
}

pub unsafe fn write_msr(reg: u32, value: u64) {
    asm!("msr {}, {}", in(reg) reg, in(reg) value, options(nostack));
}

pub unsafe fn read_sctlr_el1() -> u64 {
    read_msr(0b11 << 14 | 0b000 | 0b0000)
}

pub unsafe fn write_sctlr_el1(value: u64) {
    write_msr(0b11 << 14 | 0b000 | 0b0000, value);
}

pub unsafe fn read_ttbr0_el1() -> u64 {
    read_msr(0b11 << 14 | 0b010 | 0b0000)
}

pub unsafe fn write_ttbr0_el1(value: u64) {
    write_msr(0b11 << 14 | 0b010 | 0b0000, value);
}

pub unsafe fn read_ttbr1_el1() -> u64 {
    read_msr(0b11 << 14 | 0b010 | 0b0001)
}

pub unsafe fn write_ttbr1_el1(value: u64) {
    write_msr(0b11 << 14 | 0b010 | 0b0001, value);
}

pub unsafe fn read_tcr_el1() -> u64 {
    read_msr(0b11 << 14 | 0b010 | 0b0010)
}

pub unsafe fn write_tcr_el1(value: u64) {
    write_msr(0b11 << 14 | 0b010 | 0b0010, value);
}

pub unsafe fn read_far_el1() -> u64 {
    read_msr(0b11 << 14 | 0b100 | 0b0000)
}

pub unsafe fn read_esr_el1() -> u64 {
    read_msr(0b11 << 14 | 0b101 | 0b0000)
}

pub unsafe fn read_elr_el1() -> u64 {
    read_msr(0b11 << 14 | 0b110 | 0b0000)
}

pub unsafe fn write_elr_el1(value: u64) {
    write_msr(0b11 << 14 | 0b110 | 0b0000, value);
}

pub unsafe fn read_sp_el0() -> u64 {
    read_msr(0b11 << 14 | 0b100 | 0b0100)
}

pub unsafe fn write_sp_el0(value: u64) {
    write_msr(0b11 << 14 | 0b100 | 0b0100, value);
}

pub unsafe fn read_cntfrq_el0() -> u64 {
    read_msr(0b11 << 11 | 0b000 | 0b0000)
}

pub unsafe fn write_cntfrq_el0(value: u64) {
    write_msr(0b11 << 11 | 0b000 | 0b0000, value);
}

pub unsafe fn read_cntvct_el0() -> u64 {
    read_msr(0b11 << 11 | 0b010 | 0b0010)
}

pub unsafe fn read_cntkctl_el1() -> u64 {
    read_msr(0b11 << 14 | 0b111 | 0b0001)
}

pub unsafe fn write_cntkctl_el1(value: u64) {
    write_msr(0b11 << 14 | 0b111 | 0b0001, value);
}

pub fn get_cpu_id() -> u64 {
    unsafe {
        let midr: u64;
        asm!("mrs {}, midr_el1", out(reg) midr, options(nomem, nostack));
        midr
    }
}

pub fn get_cache_type() -> u64 {
    unsafe {
        let ctr: u64;
        asm!("mrs {}, ctr_el0", out(reg) ctr, options(nomem, nostack));
        ctr
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CacheInfo {
    pub icache_line_size: usize,
    pub dcache_line_size: usize,
}

pub fn get_cache_info() -> CacheInfo {
    let ctr = get_cache_type();
    
    CacheInfo {
        icache_line_size: 4 << ((ctr >> 0) & 0xF),
        dcache_line_size: 4 << ((ctr >> 16) & 0xF),
    }
}

#[inline(always)]
pub unsafe fn icache_invalidate_all() {
    asm!("ic iallu", options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn dcache_invalidate_all() {
    dcache_clean_and_invalidate_all();
}

#[inline(always)]
pub unsafe fn dcache_clean_all() {
    asm!("dc civac, {}", in(reg) 0, options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn dcache_invalidate(addr: usize) {
    asm!("dc ivac, {}", in(reg) addr, options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn dcache_clean(addr: usize) {
    asm!("dc cvac, {}", in(reg) addr, options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn dcache_clean_and_invalidate(addr: usize) {
    asm!("dc civac, {}", in(reg) addr, options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn dcache_clean_and_invalidate_all() {
    asm!("dc civac, {}", in(reg) 0, options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn icache_invalidate(addr: usize) {
    asm!("ic ivac, {}", in(reg) addr, options(nomem, nostack));
}

#[no_mangle]
pub unsafe extern "C" fn rust_panic() -> ! {
    disable_all_interrupts();
    halt()
}

#[no_mangle]
#[link_section = ".bss"]
pub static mut BOOT_STACK: [MaybeUninit<u8>; 65536] = [MaybeUninit::uninit(); 65536];

#[no_mangle]
pub static BOOT_STACK_TOP: *const u8 = unsafe { BOOT_STACK.as_ptr().add(65536) };
