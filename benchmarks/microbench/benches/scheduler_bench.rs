use criterion::{criterion_group, criterion_main, Criterion};

#[derive(Clone, Copy)]
struct Node { pid: usize, next: usize }

fn sim_runqueue(n: usize) {
    let mut nodes = vec![Node { pid: 0, next: 0 }; n + 1];
    let mut head = 0usize;
    let mut tail = 0usize;
    nodes[1..=n].iter_mut().enumerate().for_each(|(i, node)| {
        *node = Node { pid: i + 1, next: 0 };
    });
    for i in 1..=n {
        if head == 0 { head = i; tail = i; } else { nodes[tail].next = i; tail = i; }
    }
    let mut picked = 0usize;
    while head != 0 {
        let cur = head;
        picked += nodes[cur].pid;
        head = nodes[cur].next;
    }
    core::hint::black_box(picked);
}

fn bench_scheduler(c: &mut Criterion) {
    c.bench_function("runqueue_round_robin_1k", |b| {
        b.iter(|| sim_runqueue(1024));
    });
    c.bench_function("runqueue_round_robin_10k", |b| {
        b.iter(|| sim_runqueue(10_000));
    });
}

criterion_group!(benches, bench_scheduler);
criterion_main!(benches);

