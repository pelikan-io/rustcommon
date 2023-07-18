use metriken::*;
use parking_lot::const_mutex;
use parking_lot::Mutex;

static MUTEX: Mutex<()> = const_mutex(());

#[test]
fn dynamic_counters() {
    let _guard = MUTEX.lock();

    // the registry is empty
    assert!(metrics().is_empty());

    // define a new counter
    let a = DynamicMetric::builder(Counter::new(), "counter-a").build();

    // show that the counter is added and functions as expected

    assert_eq!(a.value(), 0);
    assert_eq!(metrics().len(), 1);

    assert_eq!(a.increment(), 0);
    assert_eq!(a.value(), 1);
    assert_eq!(a.add(2), 1);
    assert_eq!(a.value(), 3);

    // add another counter
    let b = DynamicMetric::builder(Counter::new(), "counter-b").build();

    // show that the new gauge is added and functions independently

    assert_eq!(metrics().len(), 2);
    assert_eq!(a.value(), 3);

    assert_eq!(b.value(), 0);
    assert_eq!(b.add(10), 0);
    assert_eq!(b.value(), 10);

    assert_eq!(a.value(), 3);

    // drop one of the counters and see that the registry length reflects
    // successful deregistration

    drop(a);

    assert_eq!(metrics().len(), 1);
}

#[test]
fn dynamic_gauges() {
    let _guard = MUTEX.lock();

    // the registry is empty
    assert_eq!(metrics().len(), 0);

    // define a new gauge
    let a = DynamicMetric::builder(Gauge::new(), "gauge-a").build();

    // show that the counter is added and functions as expected

    assert_eq!(a.value(), 0);
    assert_eq!(metrics().len(), 1);

    assert_eq!(a.increment(), 0);
    assert_eq!(a.value(), 1);
    assert_eq!(a.add(2), 1);
    assert_eq!(a.value(), 3);

    assert_eq!(a.decrement(), 3);
    assert_eq!(a.value(), 2);
    assert_eq!(a.sub(3), 2);
    assert_eq!(a.value(), -1);

    // add another gauge
    let b = DynamicMetric::builder(Counter::new(), "gauge-b").build();

    // show that the new gauge is added and functions independently

    assert_eq!(metrics().len(), 2);
    assert_eq!(a.value(), -1);

    assert_eq!(b.value(), 0);
    assert_eq!(b.add(10), 0);
    assert_eq!(b.value(), 10);

    assert_eq!(a.value(), -1);

    // drop one of the gauges and see that the registry length reflects
    // successful deregistration

    drop(a);

    assert_eq!(metrics().len(), 1);
}

#[test]
fn dynamic_lazy_counter() {
    let _guard = MUTEX.lock();

    // the registry is empty
    assert_eq!(metrics().len(), 0);

    // add a lazy counter to the registry
    let a = DynamicMetric::builder(LazyCounter::default(), "counter-a").build();

    assert_eq!(metrics().len(), 1);

    // since the counter has only been defined and not used, it remains
    // uninitialized

    for metric in &metrics() {
        if let Some(counter) = metric.as_any().downcast_ref::<LazyCounter>() {
            assert!(counter.value().is_none());
        } else {
            panic!("unexpected metric type");
        }
    }

    // after using the counter, the metric is initialized

    a.increment();

    for metric in &metrics() {
        if let Some(counter) = metric.as_any().downcast_ref::<LazyCounter>() {
            assert!(counter.value().is_some());
        } else {
            panic!("unexpected metric type");
        }
    }
}

#[test]
fn format() {
    let _guard = MUTEX.lock();

    let _a = DynamicMetric::builder(Counter::new(), "counter")
        .metadata("key", "value")
        .build();
    let metrics = metrics();
    let entry = metrics.iter().next().unwrap();

    assert_eq!(entry.format(Format::Plain), Some("counter".to_string()));
    assert_eq!(
        entry.format(Format::Prometheus),
        Some("counter{key=\"value\"}".to_string())
    );
}
