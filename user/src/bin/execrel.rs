#![no_std]
#![no_main]

use user::*;
const O_CREAT_U: i32 = 0o100;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    main();
    exit(0)
}

fn main() {
    println("execrel: begin");
    let _ = mkdir(b"/tmp\0".as_ptr());
    let _ = chdir(b"/tmp\0".as_ptr());
    let mut elf = [0u8; 4096];
    elf[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
    elf[4] = 2;
    elf[5] = 1;
    elf[16+16] = 2; elf[16+17] = 0;
    #[cfg(target_arch="riscv64")] { elf[16+18] = (243u16 & 0xFF) as u8; elf[16+19] = (243u16 >> 8) as u8; }
    #[cfg(target_arch="aarch64")] { elf[16+18] = (183u16 & 0xFF) as u8; elf[16+19] = (183u16 >> 8) as u8; }
    #[cfg(target_arch="x86_64")] { elf[16+18] = (62u16 & 0xFF) as u8; elf[16+19] = (62u16 >> 8) as u8; }
    elf[16+20] = 1; elf[16+21] = 0; elf[16+22] = 0; elf[16+23] = 0;
    let entry: u64 = 0x400000;
    elf[24..32].copy_from_slice(&entry.to_le_bytes());
    let phoff: u64 = 64;
    elf[32..40].copy_from_slice(&phoff.to_le_bytes());
    elf[52..54].copy_from_slice(&(64u16).to_le_bytes());
    elf[54..56].copy_from_slice(&(56u16).to_le_bytes());
    elf[56..58].copy_from_slice(&(1u16).to_le_bytes());
    let ph = phoff as usize;
    elf[ph..ph+4].copy_from_slice(&(1u32).to_le_bytes());
    elf[ph+4..ph+8].copy_from_slice(&(5u32).to_le_bytes());
    elf[ph+8..ph+16].copy_from_slice(&(0u64).to_le_bytes());
    elf[ph+16..ph+24].copy_from_slice(&entry.to_le_bytes());
    elf[ph+24..ph+32].copy_from_slice(&(0u64).to_le_bytes());
    elf[ph+32..ph+40].copy_from_slice(&(0u64).to_le_bytes());
    elf[ph+40..ph+48].copy_from_slice(&(4096u64).to_le_bytes());
    elf[ph+48..ph+56].copy_from_slice(&(4096u64).to_le_bytes());
    let fd = open(b"hello\0".as_ptr(), O_CREAT_U | O_RDWR);
    if fd < 0 { perror("open hello"); println("execrel: fail"); return; }
    let wrote = write(fd as i32, elf.as_ptr(), elf.len());
    if wrote < 0 { perror("write hello"); println("execrel: fail"); return; }
    let argv: [*const u8; 2] = [b"hello\0".as_ptr(), core::ptr::null()];
    let r = exec(b"hello\0".as_ptr(), argv.as_ptr());
    if r < 0 { perror("exec hello"); println("execrel: fail"); }
}
