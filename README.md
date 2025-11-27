# xv6-rust 多架构移植项目

## 构建与运行
### riscv64
- `cargo build --release -p kernel`
- `qemu-system-riscv64 -M virt -bios none -nographic -kernel target/riscv64imac-unknown-none-elf/release/kernel`

### aarch64
- `cargo build --release --target aarch64-unknown-none -p kernel`
- `qemu-system-aarch64 -M virt -cpu cortex-a57 -serial mon:stdio -kernel target/aarch64-unknown-none/release/kernel`

### x86_64（编译验证）
- `cargo build --release --target x86_64-unknown-none -p kernel`
