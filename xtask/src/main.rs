use std::process::Command;

fn main() {
    let mut args = std::env::args().skip(1);
    let task = args.next().unwrap_or_else(|| "help".to_string());
    match task.as_str() {
        "kernel" => build_kernel(args.collect()),
        "user" => build_user(args.collect()),
        _ => print_help(),
    }
}

fn build_kernel(args: Vec<String>) {
    let target = args.get(0).map(String::as_str).unwrap_or("aarch64");
    let tests = args.iter().any(|a| a == "--tests");
    let target_json = match target {
        "aarch64" => "targets/aarch64-nostd.json",
        "riscv64" => "targets/riscv64-nostd.json",
        "x86_64" => "targets/x86_64-nostd.json",
        _ => "targets/aarch64-nostd.json",
    };
    let mut cmd = Command::new("cargo");
    cmd.arg("+nightly")
        .arg("build")
        .arg("-p").arg("kernel")
        .arg("--target").arg(target_json)
        .arg("-Z").arg("build-std=core,alloc")
        .arg("-Z").arg("build-std-features=compiler-builtins-mem")
        .arg("--features").arg(if tests { "baremetal,kernel_tests" } else { "baremetal" });
    run(&mut cmd, "kernel build");
}

fn build_user(args: Vec<String>) {
    let target = args.get(0).map(String::as_str).unwrap_or("aarch64");
    let target_json = match target {
        "aarch64" => "targets/aarch64-nostd.json",
        "riscv64" => "targets/riscv64-nostd.json",
        "x86_64" => "targets/x86_64-nostd.json",
        _ => "targets/aarch64-nostd.json",
    };
    let mut cmd = Command::new("cargo");
    cmd.arg("+nightly")
        .arg("build")
        .arg("-p").arg("user")
        .arg("--target").arg(target_json)
        .arg("-Z").arg("build-std=core,alloc")
        .arg("-Z").arg("build-std-features=compiler-builtins-mem")
        .arg("--features").arg("user-bin");
    run(&mut cmd, "user build");
}

fn run(cmd: &mut Command, name: &str) {
    println!("[xtask] {}: {:?}", name, cmd);
    let status = cmd.status().expect("failed to run command");
    if !status.success() {
        eprintln!("[xtask] {} failed with status {:?}", name, status);
        std::process::exit(1);
    }
}

fn print_help() {
    println!("xtask usage:\n  cargo run -p xtask -- kernel <aarch64|riscv64|x86_64> [--tests]\n  cargo run -p xtask -- user <aarch64|riscv64|x86_64>");
}
