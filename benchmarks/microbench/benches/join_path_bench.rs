use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn join_path(base: &str, rel: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let is_abs = rel.starts_with('/');
    if !is_abs {
        for p in base.split('/').filter(|s| !s.is_empty()) { out.push(String::from(p)); }
    }
    for p in rel.split('/').filter(|s| !s.is_empty()) {
        if p == "." { continue; }
        if p == ".." { if !out.is_empty() { out.pop(); } continue; }
        out.push(String::from(p));
    }
    let mut s = String::from("/");
    for (i, seg) in out.iter().enumerate() {
        if i > 0 { s.push('/'); }
        s.push_str(seg);
    }
    s
}

fn bench_join_path(c: &mut Criterion) {
    c.bench_function("join_path_deep", |b| {
        b.iter(|| {
            let base = std::hint::black_box("/usr/local/lib/rust");
            let rel = black_box("../../bin/../share/doc/./examples");
            let _ = join_path(base, rel);
        });
    });
    c.bench_function("join_path_abs", |b| {
        b.iter(|| {
            let base = std::hint::black_box("/var/log");
            let rel = black_box("/etc/../etc/systemd");
            let _ = join_path(base, rel);
        });
    });
}

criterion_group!(benches, bench_join_path);
criterion_main!(benches);

