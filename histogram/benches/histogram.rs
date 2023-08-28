use core::time::Duration;
use criterion::Throughput;
use histogram::Histogram;
use histogram::atomic::Histogram as AtomicHistogram;

// use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::{criterion_group, criterion_main};

fn histogram(c: &mut Criterion) {
    let mut histogram = Histogram::new(0,7,64).unwrap();

    let mut group = c.benchmark_group("histogram");
    group.throughput(Throughput::Elements(1));
    group.bench_function("increment/1", |b| b.iter(|| histogram.increment(1)));

    histogram.clear();
    group.bench_function("increment/max", |b| b.iter(|| histogram.increment(u64::MAX)));

    group.finish();
}

fn histogram_atomic(c: &mut Criterion) {
    let histogram = AtomicHistogram::new(0,7,64).unwrap();

    let mut group = c.benchmark_group("histogram::atomic");
    group.throughput(Throughput::Elements(1));
    group.bench_function("increment/1", |b| b.iter(|| histogram.increment(1)));

    histogram.clear();
    group.bench_function("increment/max", |b| b.iter(|| histogram.increment(u64::MAX)));

    group.finish();
}

fn sliding_window(c: &mut Criterion) {
    // microsecond resolution

    let mut histogram = histogram::sliding_window::Histogram::new(0,7,64, Duration::from_micros(1), 100).unwrap();

    let mut group = c.benchmark_group("histogram::sliding_window/microseconds");
    group.throughput(Throughput::Elements(1));
    group.bench_function("increment/1", |b| b.iter(|| histogram.increment(1)));
    group.bench_function("increment/max", |b| b.iter(|| histogram.increment(u64::MAX)));
    group.finish();

    // millisecond resolution

    let mut histogram = histogram::sliding_window::Histogram::new(0,7,64, Duration::from_millis(1), 100).unwrap();

    let mut group = c.benchmark_group("histogram::sliding_window/milliseconds");
    group.throughput(Throughput::Elements(1));
    group.bench_function("increment/1", |b| b.iter(|| histogram.increment(1)));
    group.bench_function("increment/max", |b| b.iter(|| histogram.increment(u64::MAX)));
    group.finish();

    // second resolution

    let mut histogram = histogram::sliding_window::Histogram::new(0,7,64, Duration::from_secs(1), 100).unwrap();

    let mut group = c.benchmark_group("histogram::sliding_window/seconds");
    group.throughput(Throughput::Elements(1));
    group.bench_function("increment/1", |b| b.iter(|| histogram.increment(1)));
    group.bench_function("increment/max", |b| b.iter(|| histogram.increment(u64::MAX)));
    group.finish();
}

fn sliding_window_atomic(c: &mut Criterion) {
    // // microsecond resolution

    // let histogram = histogram::sliding_window::atomic::Histogram::new(0,7,64, Duration::from_micros(1), 100).unwrap();

    // let mut group = c.benchmark_group("histogram::sliding_window::atomic/microseconds");
    // group.throughput(Throughput::Elements(1));
    // group.bench_function("increment/1", |b| b.iter(|| histogram.increment(1)));
    // group.bench_function("increment/max", |b| b.iter(|| histogram.increment(u64::MAX)));
    // group.finish();

    // millisecond resolution

    let histogram = histogram::sliding_window::atomic::Histogram::new(0,7,64, Duration::from_millis(1), 100).unwrap();

    let mut group = c.benchmark_group("histogram::sliding_window::atomic/milliseconds");
    group.throughput(Throughput::Elements(1));
    group.bench_function("increment/1", |b| b.iter(|| histogram.increment(1)));
    group.bench_function("increment/max", |b| b.iter(|| histogram.increment(u64::MAX)));
    group.finish();

    // second resolution

    let histogram = histogram::sliding_window::atomic::Histogram::new(0,7,64, Duration::from_secs(1), 100).unwrap();

    let mut group = c.benchmark_group("histogram::sliding_window::atomic/seconds");
    group.throughput(Throughput::Elements(1));
    group.bench_function("increment/1", |b| b.iter(|| histogram.increment(1)));
    group.bench_function("increment/max", |b| b.iter(|| histogram.increment(u64::MAX)));
    group.finish();
}


criterion_group!(benches, histogram, histogram_atomic, sliding_window, sliding_window_atomic);
criterion_main!(benches);