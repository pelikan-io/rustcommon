use metriken::{metric, metrics, Counter};

#[metric]
static THE_METRIC: Counter = Counter::new();

#[test]
fn as_any_does_not_recurse_infinitely() {
    let metrics = metrics().static_metrics();
    let metric = metrics.iter().find(|entry| entry.is(&THE_METRIC)).unwrap();

    let counter = metric.as_any().unwrap().downcast_ref::<Counter>().unwrap();

    assert_eq!(counter.value(), THE_METRIC.value());
}
