//! Performance benchmarks for NOS kernel
//! Measures system call dispatch and process management performance

#![cfg(test)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kernel::syscalls;
use kernel::process::manager::ProcTable;

fn benchmark_syscall_dispatch(c: &mut Criterion) {
    c.bench_function("dispatch_invalid_syscall", |b| {
        b.iter(|| {
            syscalls::dispatch(black_box(0xFFFFFFFF), &[])
        });
    });

    c.bench_function("dispatch_process_syscall", |b| {
        b.iter(|| {
            syscalls::dispatch(black_box(0x1000), &[])
        });
    });

    c.bench_function("dispatch_with_args", |b| {
        let args = black_box(vec![1, 2, 3, 4, 5, 6]);
        b.iter(|| {
            syscalls::dispatch(0x1000, &args)
        });
    });
}

fn benchmark_process_allocation(c: &mut Criterion) {
    c.bench_function("process_alloc", |b| {
        b.iter(|| {
            let mut table = ProcTable::new();
            black_box(table.alloc())
        });
    });

    c.bench_function("process_alloc_multiple", |b| {
        b.iter(|| {
            let mut table = ProcTable::new();
            for _ in 0..10 {
                black_box(table.alloc());
            }
        });
    });
}

fn benchmark_process_lookup(c: &mut Criterion) {
    let mut table = ProcTable::new();
    let mut pids = Vec::new();
    
    // Pre-allocate some processes
    for _ in 0..16 {
        if let Some(proc) = table.alloc() {
            pids.push(proc.pid);
        }
    }

    c.bench_function("process_find_mutable", |b| {
        b.iter(|| {
            let pid = black_box(pids[0]);
            table.find(pid)
        });
    });

    c.bench_function("process_find_immutable", |b| {
        b.iter(|| {
            let pid = black_box(pids[0]);
            table.find_ref(pid)
        });
    });

    c.bench_function("process_lookup_miss", |b| {
        b.iter(|| {
            table.find_ref(black_box(9999))
        });
    });
}

criterion_group!(
    benches,
    benchmark_syscall_dispatch,
    benchmark_process_allocation,
    benchmark_process_lookup
);
criterion_main!(benches);
