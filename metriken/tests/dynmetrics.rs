// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use parking_lot::{Mutex, MutexGuard};
use std::mem::ManuallyDrop;
use std::pin::Pin;

use metriken::*;

// All tests manipulate global state. Need a mutex to ensure test execution
// doesn't overlap.
static TEST_MUTEX: Mutex<()> = parking_lot::const_mutex(());

/// RAII guard that ensures
/// - All dynamic metrics are removed after each test
/// - No two tests run concurrently
struct TestGuard {
    _lock: MutexGuard<'static, ()>,
}

impl TestGuard {
    pub fn new() -> Self {
        Self {
            _lock: TEST_MUTEX.lock(),
        }
    }
}

#[test]
fn wrapped_register_unregister() {
    let _guard = TestGuard::new();

    let metric = MetricBuilder::new("wrapped_register_unregister").build(Counter::new());

    assert_eq!(metrics().dynamic_metrics().len(), 1);
    drop(metric);
    assert_eq!(metrics().dynamic_metrics().len(), 0);
}

#[test]
fn pinned_register_unregister() {
    let _guard = TestGuard::new();

    let mut metric_ = ManuallyDrop::new(DynPinnedMetric::new(Counter::new()));
    let metric = unsafe { Pin::new_unchecked(&*metric_) };
    metric.register(MetricBuilder::new("pinned_register_unregister").into_entry());

    assert_eq!(metrics().dynamic_metrics().len(), 1);
    unsafe { ManuallyDrop::drop(&mut metric_) };
    assert_eq!(metrics().dynamic_metrics().len(), 0);
}

#[test]
fn pinned_scope() {
    let _guard = TestGuard::new();

    {
        let metric = DynPinnedMetric::new(Counter::new());
        let metric = unsafe { Pin::new_unchecked(&metric) };
        metric.register(MetricBuilder::new("pinned_scope").into_entry());

        assert_eq!(metrics().dynamic_metrics().len(), 1);
    }
    assert_eq!(metrics().dynamic_metrics().len(), 0);
}

#[test]
fn pinned_dup_register() {
    let _guard = TestGuard::new();

    {
        let metric = DynPinnedMetric::new(Counter::new());
        let metric = unsafe { Pin::new_unchecked(&metric) };
        metric.register(MetricBuilder::new("pinned_dup_1").into_entry());
        metric.register(MetricBuilder::new("pinned_dup_2").into_entry());

        assert_eq!(metrics().dynamic_metrics().len(), 1);
    }
    assert_eq!(metrics().dynamic_metrics().len(), 0);
}

#[test]
fn multi_metric() {
    let _guard = TestGuard::new();

    let m1 = MetricBuilder::new("multi_metric_1").build(Counter::new());
    let m2 = MetricBuilder::new("multi_metric_2").build(Counter::new());

    assert_eq!(metrics().dynamic_metrics().len(), 2);
    drop(m1);
    assert_eq!(metrics().dynamic_metrics().len(), 1);
    drop(m2);
    assert_eq!(metrics().dynamic_metrics().len(), 0);
}
