//! NOS Bootloader Build Script
//!
//! This build script handles the compilation of assembly files,
//! linker script generation, and platform-specific build configuration
//! for both UEFI and BIOS bootloaders.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=linker/");
    println!("cargo:rerun-if-changed=src/arch/x86_64/");

    let target = env::var("TARGET").unwrap_or_else(|_| "x86_64-unknown-none".to_string());
    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".to_string());

    // Determine the build type based on features
    let build_type = determine_build_type();

    match build_type.as_str() {
        "bios" => {
            println!("Building BIOS bootloader for target: {}", target);
            build_bios_bootloader(&target, &profile);
        }
        "uefi" => {
            println!("Building UEFI bootloader for target: {}", target);
            build_uefi_bootloader(&target, &profile);
        }
        "multiboot2" => {
            println!("Building Multiboot2 bootloader for target: {}", target);
            build_multiboot2_bootloader(&target, &profile);
        }
        _ => {
            println!("Building generic bootloader for target: {}", target);
            build_generic_bootloader(&target, &profile);
        }
    }

    // Validate required tools
    validate_build_tools();

    // Create build information
    create_build_info();

    // Post-build processing
    post_build_processing();
}

fn setup_x86_64_build() {
    // x86_64 specific build configuration
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-no-pie");
    println!("cargo:rustc-link-arg=-static");

    // Configure compiler flags for x86_64
    println!("cargo:rustc-cflags=-mno-red-zone");
    println!("cargo:rustc-cflags=-mno-sse");
    println!("cargo:rustc-cflags=-mno-sse2");
    println!("cargo:rustc-cflags=-mcmodel=large");

    // BIOS compatibility
    if cfg!(feature = "bios_support") {
        println!("cargo:rustc-link-arg=-Tlinker/x86_64_bios.ld");
    }

    // UEFI compatibility
    if cfg!(feature = "uefi_support") {
        println!("cargo:rustc-link-arg=-Tlinker/x86_64_uefi.ld");
    }
}

fn setup_aarch64_build() {
    // AArch64 specific build configuration
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-no-pie");
    println!("cargo:rustc-link-arg=-static");

    // Configure compiler flags for AArch64
    println!("cargo:rustc-cflags=-mgeneral-regs-only");
    println!("cargo:rustc-cflags=-mstrict-align");

    // UEFI for AArch64 (primary target)
    if cfg!(feature = "uefi_support") {
        println!("cargo:rustc-link-arg=-Tlinker/aarch64_uefi.ld");
    }

    // Custom boot protocol support
    if cfg!(feature = "bios_support") {
        println!("cargo:rustc-link-arg=-Tlinker/aarch64_custom.ld");
    }
}

fn setup_riscv64_build() {
    // RISC-V 64 specific build configuration
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-no-pie");
    println!("cargo:rustc-link-arg=-static");

    // Configure compiler flags for RISC-V
    println!("cargo:rustc-cflags=-march=rv64gc");
    println!("cargo:rustc-cflags=-mabi=lp64d");
    println!("cargo:rustc-cflags=-mcmodel=medany");

    // OpenSBI support
    if cfg!(feature = "bios_support") {
        println!("cargo:rustc-link-arg=-Tlinker/riscv64_opensbi.ld");
    }

    // UEFI for RISC-V
    if cfg!(feature = "uefi_support") {
        println!("cargo:rustc-link-arg=-Tlinker/riscv64_uefi.ld");
    }
}

fn setup_linker_script(arch: &str, os: &str) {
    let linker_dir = Path::new("linker");
    let out_dir = env::var("OUT_DIR").unwrap();
    let output_dir = PathBuf::from(out_dir);

    // Select appropriate linker script
    let linker_script = match (arch, os) {
        ("x86_64", "uefi") => "x86_64_uefi.ld",
        ("x86_64", "none") => "x86_64_bios.ld",
        ("aarch64", "uefi") => "aarch64_uefi.ld",
        ("aarch64", "none") => "aarch64_custom.ld",
        ("riscv64", "uefi") => "riscv64_uefi.ld",
        ("riscv64", "none") => "riscv64_opensbi.ld",
        _ => panic!("Unsupported target: {}-{}", arch, os),
    };

    let source_script = linker_dir.join(linker_script);
    let dest_script = output_dir.join("linker.ld");

    // Copy and process linker script
    if source_script.exists() {
        let content = fs::read_to_string(&source_script).unwrap_or_default();
        let processed_content = process_linker_script(&content, arch);

        fs::write(&dest_script, processed_content).unwrap_or_else(|e| {
            panic!("Failed to write linker script: {}", e);
        });

        // Tell cargo to use our processed linker script
        println!("cargo:rustc-link-arg=-T{}", dest_script.display());
    } else {
        panic!("Linker script not found: {}", source_script.display());
    }
}

fn process_linker_script(content: &str, arch: &str) -> String {
    let mut result = content.to_string();

    // Architecture-specific substitutions
    match arch {
        "x86_64" => {
            result = result.replace("{{ARCH_STARTUP}}", "_start");
            result = result.replace("{{ARCH_ENTRY}}", "boot_main");
        }
        "aarch64" => {
            result = result.replace("{{ARCH_STARTUP}}", "_start");
            result = result.replace("{{ARCH_ENTRY}}", "boot_main");
        }
        "riscv64" => {
            result = result.replace("{{ARCH_STARTUP}}", "_start");
            result = result.replace("{{ARCH_ENTRY}}", "boot_main");
        }
        _ => {}
    }

    // Memory layout substitutions
    match arch {
        "x86_64" => {
            result = result.replace("{{KERNEL_BASE}}", "0x100000");
            result = result.replace("{{KERNEL_SIZE}}", "0x100000");
        }
        "aarch64" => {
            result = result.replace("{{KERNEL_BASE}}", "0x40000000");
            result = result.replace("{{KERNEL_SIZE}}", "0x200000");
        }
        "riscv64" => {
            result = result.replace("{{KERNEL_BASE}}", "0x80000000");
            result = result.replace("{{KERNEL_SIZE}}", "0x200000");
        }
        _ => {}
    }

    result
}

fn determine_build_type() -> String {
    // Check which features are enabled to determine the build type
    let features = env::var("CARGO_CFG_FEATURE").unwrap_or_default();

    if features.contains("bios_support") && !features.contains("uefi_support") {
        if features.contains("multiboot2_support") {
            "multiboot2".to_string()
        } else {
            "bios".to_string()
        }
    } else if features.contains("uefi_support") && !features.contains("bios_support") {
        "uefi".to_string()
    } else {
        // Default to generic build if both or neither are enabled
        "generic".to_string()
    }
}

fn build_bios_bootloader(target: &str, profile: &str) {
    // Compile assembly files for BIOS bootloader
    if target.starts_with("x86_64") {
        compile_x86_64_bios_assembly(profile);
    }

    // Generate custom linker script
    let linker_script = generate_bios_linker_script(target);
    println!("cargo:rustc-link-arg=-T{}", linker_script.display());

    // Set linker-specific flags
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-Wl,--gc-sections");
    println!("cargo:rustc-link-arg=-Wl,--build-id=none");

    if profile == "release" {
        println!("cargo:rustc-link-arg=-Wl,--strip-debug");
    }

    // Set output format based on target
    if target.contains("uefi") {
        println!("cargo:rustc-link-arg=-Wl,--subsystem,efi_application");
    } else {
        println!("cargo:rustc-link-arg=-Wl,--oformat,binary");
    }

    // Export BIOS-specific cfg flags
    println!("cargo:rustc-cfg=bios_support");
    println!("cargo:rustc-cfg=bios_bootloader");
}

fn compile_x86_64_bios_assembly(profile: &str) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let asm_file = Path::new("src/arch/x86_64/bios.S");
    let obj_file = PathBuf::from(&out_dir).join("bios_x86_64.o");

    // Use NASM for BIOS assembly compilation
    let nasm_flags = if profile == "debug" {
        vec!["-g", "-F", "dwarf"]
    } else {
        vec![]
    };

    let output = Command::new("nasm")
        .arg("-f")
        .arg("elf64")
        .arg(asm_file)
        .arg("-o")
        .arg(&obj_file)
        .args(&nasm_flags)
        .output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                eprintln!("NASM compilation failed:");
                eprintln!("stdout: {}", String::from_utf8_lossy(&result.stdout));
                eprintln!("stderr: {}", String::from_utf8_lossy(&result.stderr));
                eprintln!("Falling back to rustc assembly compilation");
                compile_rust_assembly_fallback(&obj_file);
            } else {
                println!("Successfully compiled BIOS assembly to {}", obj_file.display());
            }
        }
        Err(e) => {
            eprintln!("Failed to run NASM: {}. Using fallback.", e);
            compile_rust_assembly_fallback(&obj_file);
        }
    }

    // Link the compiled object file
    println!("cargo:rustc-link-arg={}", obj_file.display());
}

fn compile_rust_assembly_fallback(obj_file: &Path) {
    println!("Using Rust assembly compilation fallback");

    let asm_file = Path::new("src/arch/x86_64/bios.S");

    // Use rustc to compile assembly (less optimal but works)
    let output = Command::new("rustc")
        .arg("--emit=obj")
        .arg("--crate-type=staticlib")
        .arg("--target=x86_64-unknown-none")
        .arg("-Copt-level=2")
        .arg("-Cpanic=abort")
        .arg("-o")
        .arg(obj_file)
        .arg(asm_file)
        .output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                eprintln!("Rust assembly compilation failed:");
                eprintln!("stderr: {}", String::from_utf8_lossy(&result.stderr));
                panic!("Assembly compilation failed");
            }
        }
        Err(e) => {
            panic!("Failed to compile assembly with rustc: {}", e);
        }
    }
}

fn generate_bios_linker_script(target: &str) -> PathBuf {
    let out_dir = env::var("OUT_DIR").unwrap();
    let linker_script = PathBuf::from(&out_dir).join("bios.ld");

    // Read the base linker script
    let base_script = fs::read_to_string("linker/bios.ld")
        .expect("Failed to read base linker script");

    // Customize based on target
    let customized_script = customize_linker_script(&base_script, target);

    // Write the customized script
    fs::write(&linker_script, customized_script)
        .expect("Failed to write linker script");

    println!("Generated BIOS linker script: {}", linker_script.display());
    linker_script
}

fn customize_linker_script(base_script: &str, target: &str) -> String {
    let mut script = base_script.to_string();

    // Apply target-specific customizations
    if target.starts_with("x86_64") {
        script = script.replace("{{ARCH}}", "x86_64");
        script = script.replace("{{ARCH_STARTUP}}", "_start");
        script = script.replace("{{KERNEL_BASE}}", "0x100000");
        script = script.replace("{{STACK_SIZE}}", "0x4000");
        script = script.replace("{{HEAP_SIZE}}", "0x1000");
    }

    script
}

fn build_uefi_bootloader(target: &str, profile: &str) {
    // UEFI-specific build configuration
    println!("Building for UEFI target: {}", target);

    // Export UEFI-specific cfg flags
    println!("cargo:rustc-cfg=uefi_support");
    println!("cargo:rustc-cfg=uefi_bootloader");

    // Use appropriate linker script
    let linker_script = if target.starts_with("x86_64") {
        "linker/x86_64_uefi.ld"
    } else if target.starts_with("aarch64") {
        "linker/aarch64_uefi.ld"
    } else if target.starts_with("riscv64") {
        "linker/riscv64_uefi.ld"
    } else {
        "linker/uefi.ld"
    };

    println!("cargo:rustc-link-arg=-T{}", linker_script);
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-Wl,--gc-sections");

    if target.starts_with("x86_64") {
        println!("cargo:rustc-link-arg=-Wl,--subsystem,efi_application");
    }
}

fn build_multiboot2_bootloader(target: &str, profile: &str) {
    // Multiboot2-specific build configuration
    println!("Building for Multiboot2 target: {}", target);

    // Export Multiboot2-specific cfg flags
    println!("cargo:rustc-cfg=multiboot2_support");
    println!("cargo:rustc-cfg=multiboot2_bootloader");

    // Use BIOS linker script but with ELF output format
    let linker_script = generate_bios_linker_script(target);
    println!("cargo:rustc-link-arg=-T{}", linker_script.display());
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-Wl,--gc-sections");
    println!("cargo:rustc-link-arg=-Wl,--oformat,elf");
}

fn build_generic_bootloader(target: &str, profile: &str) {
    // Generic build configuration
    println!("Building generic bootloader for target: {}", target);

    // Use default linker script if available
    if Path::new("linker/generic.ld").exists() {
        println!("cargo:rustc-link-arg=-Tlinker/generic.ld");
    }

    println!("cargo:rustc-link-arg=-nostdlib");
}

fn create_build_info() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let build_info_file = PathBuf::from(&out_dir).join("build_info.rs");

    let build_info = format!(
        r#"
/// Build information generated at compile time
pub const BUILD_TIMESTAMP: &str = "{}";
pub const BUILD_VERSION: &str = "{}";
pub const BUILD_PROFILE: &str = "{}";
pub const BUILD_TARGET: &str = "{}";
pub const BUILD_RUSTC_VERSION: &str = "{}";
"#,
        get_current_timestamp(),
        env::var("CARGO_PKG_VERSION").unwrap_or_default(),
        env::var("PROFILE").unwrap_or_default(),
        env::var("TARGET").unwrap_or_default(),
        get_rustc_version()
    );

    fs::write(&build_info_file, build_info)
        .expect("Failed to write build info");

    println!("cargo:rustc-env=BUILD_INFO_FILE={}", build_info_file.display());
}

fn get_current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    duration.as_secs().to_string()
}

fn get_rustc_version() -> String {
    Command::new("rustc")
        .arg("--version")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn validate_build_tools() {
    // Check for NASM if building BIOS bootloader
    if env::var("CARGO_CFG_FEATURE").unwrap_or_default().contains("bios_support") {
        match Command::new("nasm").arg("-v").output() {
            Ok(_) => println!("NASM found: OK"),
            Err(_) => {
                println!("Warning: NASM not found, will use fallback assembly compilation");
                println!("Install NASM for better BIOS bootloader compilation:");
                println!("  Ubuntu/Debian: sudo apt-get install nasm");
                println!("  macOS: brew install nasm");
                println!("  Windows: Download from https://www.nasm.us/");
            }
        }
    }

    // Check for objcopy for binary generation
    match Command::new("objcopy").arg("--version").output() {
        Ok(_) => println!("objcopy found: OK"),
        Err(_) => {
            println!("Warning: objcopy not found");
            println!("Install binutils for binary generation capabilities");
        }
    }
}

fn post_build_processing() {
    let target = env::var("TARGET").unwrap_or_default();
    let out_dir = env::var("OUT_DIR").unwrap();

    // Create bootable image for BIOS bootloader
    if target.starts_with("x86_64") && !target.contains("uefi") {
        create_bootable_image(&out_dir);
    }

    // Generate size report
    generate_size_report(&out_dir);
}

fn create_bootable_image(out_dir: &str) {
    let out_path = Path::new(out_dir);

    // This would create a proper bootable image with MBR, partition table, etc.
    println!("Post-build: Creating bootable image");

    // For now, just ensure the binary exists
    let bootloader_binary = out_path.join("bios-bootloader.bin");
    if bootloader_binary.exists() {
        let size = fs::metadata(&bootloader_binary).unwrap().len();
        println!("Bootloader binary size: {} bytes", size);

        if size > 512 {
            println!("Note: Bootloader exceeds 512 bytes, will need proper boot sector setup");
        }
    }
}

fn generate_size_report(out_dir: &str) {
    println!("Generating size report...");

    let out_path = Path::new(out_dir);

    // Find and analyze the output binary
    for entry in fs::read_dir(out_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_file() && (path.extension().map(|s| s == "bin") == Some(true) ||
                              path.extension().map(|s| s == "elf") == Some(true)) {
            if let Ok(metadata) = fs::metadata(&path) {
                let size = metadata.len();
                println!("{}: {} bytes", path.file_name().unwrap().to_string_lossy(), size);

                // Check size limits
                if path.file_name().unwrap().to_string_lossy().contains("bios") && size > 1024 * 1024 {
                    println!("Warning: BIOS bootloader is unusually large ({} bytes)", size);
                }
            }
        }
    }
}
