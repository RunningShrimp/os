use std::process::Command;
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use regex::Regex;

fn main() {
    let mut args = std::env::args().skip(1);
    let task = args.next().unwrap_or_else(|| "help".to_string());
    match task.as_str() {
        "kernel" => build_kernel(args.collect()),
        "user" => build_user(args.collect()),
        "syscalls" => {
            if let Err(e) = generate_syscall_matrix() {
                eprintln!("[xtask] syscalls error: {:#}", e);
                std::process::exit(1);
            }
        }
        "user-rel-exec" => {
            let target = args.next().unwrap_or_else(|| "aarch64".to_string());
            build_user(vec![target]);
            println!("[xtask] built user execrel; run kernel with user-bin to execute /bin/execrel or chdir /tmp and exec relative hello");
        }
        "bench" => {
            if let Err(e) = run_bench() {
                eprintln!("[xtask] bench error: {:#}", e);
                std::process::exit(1);
            }
        }
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
    println!("xtask usage:\n  cargo run -p xtask -- kernel <aarch64|riscv64|x86_64> [--tests]\n  cargo run -p xtask -- user <aarch64|riscv64|x86_64>\n  cargo run -p xtask -- syscalls\n  cargo run -p xtask -- user-rel-exec <aarch64|riscv64|x86_64>\n  cargo run -p xtask -- bench");
}

fn run_bench() -> Result<(), anyhow::Error> {
    let mut cmd = Command::new("cargo");
    cmd.arg("bench").arg("-p").arg("microbench");
    run(&mut cmd, "microbench");
    Ok(())
}

fn generate_syscall_matrix() -> Result<()> {
    let root = Path::new(".");
    let kernel_syscall = root.join("kernel/src/syscall.rs");
    let kernel_syscalls = root.join("kernel/src/syscalls.rs");
    let out_path = root.join(".trae/documents/Syscall 覆盖矩阵.md");

    let content = fs::read_to_string(&kernel_syscall).with_context(|| format!("read {}", kernel_syscall.display()))?;
    let nums = fs::read_to_string(&kernel_syscalls).unwrap_or_default();

    let enum_re = Regex::new(r"(?m)^\s*pub\s+enum\s+SysNum\s*\{([\s\S]*?)\}")?;
    let variant_re = Regex::new(r"(?m)^(\s*)([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(\d+),")?;
    let match_re = Regex::new(r"(?m)^\s*match\s+syscall\s*\{([\s\S]*?)\}")?;
    let arm_re = Regex::new(r"(?m)\s*SysNum::([A-Za-z_][A-Za-z0-9_]*)\s*=>\s*(sys_[a-z0-9_]+)\(")?;

    let mut variants: Vec<(String, usize)> = Vec::new();
    if let Some(cap) = enum_re.captures(&content) {
        let body = &cap[1];
        for v in variant_re.captures_iter(body) {
            let name = v[2].to_string();
            let num: usize = v[3].parse().unwrap_or(0);
            variants.push((name, num));
        }
    }

    let mut impls: Vec<(String, String)> = Vec::new();
    if let Some(cap) = match_re.captures(&content) {
        let body = &cap[1];
        for a in arm_re.captures_iter(body) {
            let name = a[1].to_string();
            let func = a[2].to_string();
            impls.push((name, func));
        }
    }

    impls.sort_by(|a,b| a.0.cmp(&b.0));
    variants.sort_by(|a,b| a.1.cmp(&b.1));

    let mut lines = Vec::new();
    lines.push(String::from("# 系统调用覆盖矩阵"));
    lines.push(String::from(""));
    lines.push(String::from("来源: kernel/src/syscall.rs, kernel/src/syscalls.rs"));
    lines.push(String::from(""));
    lines.push(String::from("| 编号 | 名称 | 实现函数 | 备注 |"));
    lines.push(String::from("|---:|---|---|---|"));
    for (name, num) in variants.iter() {
        let func = impls.iter().find(|(n, _)| n == name).map(|(_, f)| f.clone()).unwrap_or_else(|| String::from("未实现"));
        let note = if func == "未实现" { "未匹配到 dispatch 分支" } else { "已实现" };
        lines.push(format!("| {} | {} | {} | {} |", num, name, func, note));
    }

    // 附加 syscalls.rs 中的常量差异（如果存在未出现在枚举中的）
    let const_re = Regex::new(r"(?m)^\s*pub\s+const\s+SYS_([A-Za-z0-9_]+)\s*:\s*usize\s*=\s*(\d+);")?;
    for c in const_re.captures_iter(&nums) {
        let name = c[1].to_string();
        let num: usize = c[2].parse().unwrap_or(0);
        if !variants.iter().any(|(n, _)| n.eq_ignore_ascii_case(&name)) {
            lines.push(format!("| {} | {} | 未在枚举 | 待统一 |", num, name));
        }
    }

    fs::create_dir_all(out_path.parent().unwrap())?;
    fs::write(&out_path, lines.join("\n"))?;
    println!("[xtask] generated {}", out_path.display());
    Ok(())
}
