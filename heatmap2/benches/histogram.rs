use std::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};
use heatmap2::Histogram;
use criterion::{criterion_group, criterion_main, Criterion};

fn histogram_0_7_64(c: &mut Criterion) {
    let histogram = Histogram::new(0, 7, 64);

    c.bench_function("increment linear", |b| b.iter(|| histogram.increment(1, 1)));
    c.bench_function("increment log", |b| b.iter(|| histogram.increment(95633239299398, 1)));

    // prepare to test contended performance
    let running = Arc::new(AtomicBool::new(true));
    let histogram = Arc::new(histogram);
    let h = histogram.clone();
    let r = running.clone();

    std::thread::spawn(move || {
        while r.load(Ordering::Relaxed) {
            h.increment(1, 1);
        }
    });

    c.bench_function("increment contended", |b| b.iter(|| histogram.increment(1, 1)));
    running.store(false, Ordering::Relaxed);
}

criterion_group!(benches, histogram_0_7_64);
criterion_main!(benches);