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
    memory_barriers();
    
    boot_main();
}

#[inline(always)]
unsafe fn disable_interrupts() {
    asm!("csrci mstatus, 0x8", options(nomem, nostack));
}

#[inline(always)]
unsafe fn enable_interrupts() {
    asm!("csrsi mstatus, 0x8", options(nomem, nostack));
}

#[inline(always)]
unsafe fn disable_machine_interrupts() {
    asm!("csrci mie, 0x8", options(nomem, nostack));
}

#[inline(always)]
unsafe fn enable_machine_interrupts() {
    asm!("csrsi mie, 0x8", options(nomem, nostack));
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

#[inline(always)]
pub unsafe fn memory_barrier() {
    asm!("fence", options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn fence_i() {
    asm!("fence.i", options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn memory_acquire() {
    asm!("fence r, rw", options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn memory_release() {
    asm!("fence rw, w", options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn memory_acq_rel() {
    asm!("fence rw, rw", options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn halt() -> ! {
    loop {
        asm!("wfi", options(nomem, nostack));
    }
}

pub unsafe fn read_csr(csr: usize) -> usize {
    let value: usize;
    asm!("csrr {}, {}", out(reg) value, in(reg) csr, options(nomem, nostack));
    value
}

pub unsafe fn write_csr(csr: usize, value: usize) {
    asm!("csrw {}, {}", in(reg) csr, in(reg) value, options(nostack));
}

pub unsafe fn read_set_csr(csr: usize, set: usize) -> usize {
    let value: usize;
    asm!("csrrs {}, {}, {}", out(reg) value, in(reg) csr, in(reg) set, options(nomem, nostack));
    value
}

pub unsafe fn read_clear_csr(csr: usize, clear: usize) -> usize {
    let value: usize;
    asm!("csrrc {}, {}, {}", out(reg) value, in(reg) csr, in(reg) clear, options(nomem, nostack));
    value
}

pub unsafe fn read_write_csr(csr: usize, value: usize, mask: usize) -> usize {
    let result: usize;
    asm!("csrrw {}, {}, {}", out(reg) result, in(reg) csr, in(reg) value, options(nomem, nostack));
    result
}

pub fn read_mhartid() -> usize {
    unsafe { read_csr(0xC14) }
}

pub fn read_mstatus() -> usize {
    unsafe { read_csr(0x300) }
}

pub fn write_mstatus(value: usize) {
    unsafe { write_csr(0x300, value) }
}

pub fn read_misa() -> usize {
    unsafe { read_csr(0x301) }
}

pub fn read_medeleg() -> usize {
    unsafe { read_csr(0x302) }
}

pub fn write_medeleg(value: usize) {
    unsafe { write_csr(0x302, value) }
}

pub fn read_mideleg() -> usize {
    unsafe { read_csr(0x303) }
}

pub fn write_mideleg(value: usize) {
    unsafe { write_csr(0x303, value) }
}

pub fn read_mie() -> usize {
    unsafe { read_csr(0x304) }
}

pub fn write_mie(value: usize) {
    unsafe { write_csr(0x304, value) }
}

pub fn read_mtvec() -> usize {
    unsafe { read_csr(0x305) }
}

pub fn write_mtvec(value: usize) {
    unsafe { write_csr(0x305, value) }
}

pub fn read_mcounteren() -> usize {
    unsafe { read_csr(0x306) }
}

pub fn write_mcounteren(value: usize) {
    unsafe { write_csr(0x306, value) }
}

pub fn read_mscratch() -> usize {
    unsafe { read_csr(0x340) }
}

pub fn write_mscratch(value: usize) {
    unsafe { write_csr(0x340, value) }
}

pub fn read_mepc() -> usize {
    unsafe { read_csr(0x341) }
}

pub fn write_mepc(value: usize) {
    unsafe { write_csr(0x341, value) }
}

pub fn read_mcause() -> usize {
    unsafe { read_csr(0x342) }
}

pub fn write_mcause(value: usize) {
    unsafe { write_csr(0x342, value) }
}

pub fn read_mtval() -> usize {
    unsafe { read_csr(0x343) }
}

pub fn write_mtval(value: usize) {
    unsafe { write_csr(0x343, value) }
}

pub fn read_mip() -> usize {
    unsafe { read_csr(0x344) }
}

pub fn write_mip(value: usize) {
    unsafe { write_csr(0x344, value) }
}

pub fn read_sstatus() -> usize {
    unsafe { read_csr(0x100) }
}

pub fn write_sstatus(value: usize) {
    unsafe { write_csr(0x100, value) }
}

pub fn read_sie() -> usize {
    unsafe { read_csr(0x104) }
}

pub fn write_sie(value: usize) {
    unsafe { write_csr(0x104, value) }
}

pub fn read_stvec() -> usize {
    unsafe { read_csr(0x105) }
}

pub fn write_stvec(value: usize) {
    unsafe { write_csr(0x105, value) }
}

pub fn read_scounteren() -> usize {
    unsafe { read_csr(0x106) }
}

pub fn write_scounteren(value: usize) {
    unsafe { write_csr(0x106, value) }
}

pub fn read_sscratch() -> usize {
    unsafe { read_csr(0x140) }
}

pub fn write_sscratch(value: usize) {
    unsafe { write_csr(0x140, value) }
}

pub fn read_sepc() -> usize {
    unsafe { read_csr(0x141) }
}

pub fn write_sepc(value: usize) {
    unsafe { write_csr(0x141, value) }
}

pub fn read_scause() -> usize {
    unsafe { read_csr(0x142) }
}

pub fn write_scause(value: usize) {
    unsafe { write_csr(0x142, value) }
}

pub fn read_stval() -> usize {
    unsafe { read_csr(0x143) }
}

pub fn write_stval(value: usize) {
    unsafe { write_csr(0x143, value) }
}

pub fn read_sip() -> usize {
    unsafe { read_csr(0x144) }
}

pub fn write_sip(value: usize) {
    unsafe { write_csr(0x144, value) }
}

pub fn read_satp() -> usize {
    unsafe { read_csr(0x180) }
}

pub fn write_satp(value: usize) {
    unsafe { write_csr(0x180, value) }
}

pub fn read_mcycle() -> u64 {
    unsafe {
        let lo: usize;
        let hi: usize;
        asm!("csrrs {}, {}, 0", out(reg) lo, in(reg) 0xB00, options(nomem, nostack));
        asm!("csrrs {}, {}, 0", out(reg) hi, in(reg) 0xB80, options(nomem, nostack));
        ((hi as u64) << 32) | (lo as u64)
    }
}

pub fn read_minstret() -> u64 {
    unsafe {
        let lo: usize;
        let hi: usize;
        asm!("csrrs {}, {}, 0", out(reg) lo, in(reg) 0xB02, options(nomem, nostack));
        asm!("csrrs {}, {}, 0", out(reg) hi, in(reg) 0xB82, options(nomem, nostack));
        ((hi as u64) << 32) | (lo as u64)
    }
}

pub fn read_time() -> u64 {
    unsafe {
        let lo: usize;
        let hi: usize;
        asm!("csrrs {}, {}, 0", out(reg) lo, in(reg) 0xC01, options(nomem, nostack));
        asm!("csrrs {}, {}, 0", out(reg) hi, in(reg) 0xC81, options(nomem, nostack));
        ((hi as u64) << 32) | (lo as u64)
    }
}

pub fn get_mxl() -> u8 {
    let misa = read_misa();
    ((misa >> (usize::BITS - 2)) & 0x3) as u8
}

pub fn get_xlen() -> usize {
    match get_mxl() {
        1 => 32,
        2 => 64,
        3 => 128,
        _ => 0,
    }
}

pub fn has_extension(ext: char) -> bool {
    let misa = read_misa();
    let bit = ext as usize;
    (misa & (1 << bit)) != 0
}

#[derive(Debug, Clone, Copy)]
pub struct CpuFeatures {
    pub a: bool,
    pub c: bool,
    pub d: bool,
    pub f: bool,
    pub i: bool,
    pub m: bool,
    pub s: bool,
    pub u: bool,
}

pub fn get_cpu_features() -> CpuFeatures {
    CpuFeatures {
        a: has_extension('A'),
        c: has_extension('C'),
        d: has_extension('D'),
        f: has_extension('F'),
        i: has_extension('I'),
        m: has_extension('M'),
        s: has_extension('S'),
        u: has_extension('U'),
    }
}

pub fn enable_interrupts_bit(bit: usize) {
    unsafe {
        write_mie(read_mie() | (1 << bit));
    }
}

pub fn disable_interrupts_bit(bit: usize) {
    unsafe {
        write_mie(read_mie() & !(1 << bit));
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_panic() -> ! {
    disable_interrupts();
    halt()
}

#[no_mangle]
#[link_section = ".bss"]
pub static mut BOOT_STACK: [MaybeUninit<u8>; 65536] = [MaybeUninit::uninit(); 65536];

#[no_mangle]
pub static BOOT_STACK_TOP: *const u8 = unsafe { BOOT_STACK.as_ptr().add(65536) };
