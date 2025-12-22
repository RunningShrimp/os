fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // Check if we're building for tests
    let is_test = std::env::var("CARGO_CFG_TEST").is_ok();

    let ld = match arch.as_str() {
        "riscv64" => "linker-riscv64.ld",
        "aarch64" => "linker-aarch64.ld",
        "x86_64" => "linker-x86_64.ld",
        _ => "linker-riscv64.ld",
    };

    // Only use custom linker for bare-metal builds, not for tests
    if os == "none" && !is_test {
        println!("cargo:rustc-link-arg=-T{}/{}", manifest_dir, ld);
    }

    // For test builds, enable additional features
    if is_test {
        println!("cargo:rustc-cfg=test_build");
    }

    println!("cargo:rerun-if-changed={}", ld);

    // Rerun if test files change
    println!("cargo:rerun-if-changed=tests/");
    println!("cargo:rerun-if-changed=benches/");
}
