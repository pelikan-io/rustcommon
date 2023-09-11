use core::time::Duration;
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
    let mut histogram = histogram::Histogram::new(0, 7, 64).unwrap();
    benchmark!("histogram", histogram, c);
}

// fn histogram_atomic(c: &mut Criterion) {
//     let histogram = histogram::atomic::Histogram::new(0, 7, 64).unwrap();
//     benchmark!("histogram::atomic", histogram, c);
// }

// fn sliding_window(c: &mut Criterion) {
//     // millisecond resolution

//     let mut histogram =
//         histogram::sliding_window::Histogram::new(0, 7, 64, Duration::from_millis(1), 100).unwrap();
//     benchmark!("histogram::sliding_window/milliseconds", histogram, c);

//     // second resolution

//     let mut histogram =
//         histogram::sliding_window::Histogram::new(0, 7, 64, Duration::from_secs(1), 100).unwrap();
//     benchmark!("histogram::sliding_window/seconds", histogram, c);
// }

// fn sliding_window_atomic(c: &mut Criterion) {
//     // millisecond resolution

//     let histogram =
//         histogram::sliding_window::atomic::Histogram::new(0, 7, 64, Duration::from_millis(1), 100)
//             .unwrap();
//     benchmark!(
//         "histogram::sliding_window::atomic/milliseconds",
//         histogram,
//         c
//     );

//     // second resolution

//     let histogram =
//         histogram::sliding_window::atomic::Histogram::new(0, 7, 64, Duration::from_secs(1), 100)
//             .unwrap();
//     benchmark!("histogram::sliding_window::atomic/seconds", histogram, c);
// }

criterion_group!(
    benches,
    histogram // histogram_atomic,
              // sliding_window,
              // sliding_window_atomic
);
criterion_main!(benches);
