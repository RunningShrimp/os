use criterion::{criterion_group, criterion_main, Criterion};
use core::hint::black_box;

fn sim_copy_chunked(dst: &mut [u8], src: &[u8], page: usize) {
    let mut copied = 0usize;
    let len = dst.len().min(src.len());
    while copied < len {
        let va = core::hint::black_box(copied);
        let page_off = va & (page - 1);
        let chunk = core::cmp::min(len - copied, page - page_off);
        dst[copied..copied+chunk].copy_from_slice(&src[copied..copied+chunk]);
        copied += chunk;
    }
}

fn bench_copyin_copyout(c: &mut Criterion) {
    let mut buf_dst = vec![0u8; 64 * 1024];
    let buf_src = vec![1u8; 64 * 1024];
    c.bench_function("copy_chunked_page4k", |b| {
        b.iter(|| {
            sim_copy_chunked(&mut buf_dst, &buf_src, 4096);
        });
    });
    c.bench_function("copy_chunked_page64k", |b| {
        b.iter(|| {
            sim_copy_chunked(&mut buf_dst, &buf_src, 65536);
        });
    });
}

criterion_group!(benches, bench_copyin_copyout);
criterion_main!(benches);

