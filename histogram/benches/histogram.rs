use criterion::{criterion_group, criterion_main, Criterion, Throughput};

// To reduce duplication, we use this macro. It only works because the API for
// all the histogram types is roughly the same for some operations.
macro_rules! benchmark {
    ($name:tt, $histogram:ident, $c:ident) => {
        let mut group = $c.benchmark_group($name);
        group.throughput(Throughput::Elements(1));
        group.bench_function("increment/1", |b| b.iter(|| $histogram.increment(1)));
        group.bench_function("increment/max", |b| {
            b.iter(|| $histogram.increment(u64::MAX))
        });

        group.finish();
    };
}

fn histogram(c: &mut Criterion) {
    let mut histogram = histogram::Histogram::new(7, 64).unwrap();
    benchmark!("histogram", histogram, c);
}

fn atomic(c: &mut Criterion) {
    let histogram = histogram::AtomicHistogram::new(7, 64).unwrap();
    benchmark!("atomic_histogram", histogram, c);
}

criterion_group!(benches, histogram, atomic);
criterion_main!(benches);
