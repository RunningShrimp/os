fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if os == "none" {
        let ld = match arch.as_str() {
            "riscv64" => "linker-riscv64.ld",
            "aarch64" => "linker-aarch64.ld",
            "x86_64" => "linker-x86_64.ld",
            _ => "linker-aarch64.ld",
        };
        println!("cargo:rustc-link-arg=-T{}/user/{}", manifest_dir, ld);
        println!("cargo:rerun-if-changed=user/{}", ld);
    }
}
