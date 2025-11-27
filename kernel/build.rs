fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let ld = match arch.as_str() {
        "riscv64" => "linker-riscv64.ld",
        "aarch64" => "linker-aarch64.ld",
        "x86_64" => "linker-x86_64.ld",
        _ => "linker-riscv64.ld",
    };
    if os == "none" {
        println!("cargo:rustc-link-arg=-T{}/{}", manifest_dir, ld);
    }
    println!("cargo:rerun-if-changed={}", ld);
    println!("cargo:rerun-if-changed=start-riscv64.S");
    println!("cargo:rerun-if-changed=start-aarch64.S");
    println!("cargo:rerun-if-changed=start-x86_64.S");
}
