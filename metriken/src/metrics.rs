use std::collections::BTreeMap;
use std::iter::FusedIterator;

use parking_lot::RwLockReadGuard;

use crate::dynmetrics::DynMetricsRegistry;
use crate::MetricEntry;

/// The list of all metrics registered via the either [`metric`] attribute or by
/// using the types within the [`dynmetrics`] module.
///
/// Names within metrics are not guaranteed to be unique and no aggregation of
/// metrics with the same name is done.
pub fn metrics() -> Metrics {
    Metrics {
        dyn_metrics: crate::dynmetrics::get_registry(),
    }
}

/// Provides access to all registered metrics both static and dynamic.
///
/// **IMPORTANT:** Note that while any instance of this struct is live
/// attempting to register or unregister any dynamic metrics will block.
/// If this is done on the same thread as is currently working with an instance
/// of `Metrics` then it will cause a deadlock. If your application will be
/// registering and unregistering dynamic metrics then you should avoid holding
/// on to `Metrics` instances for long periods of time.
///
/// `Metrics` instances can be created via the [`metrics`] function.
pub struct Metrics {
    dyn_metrics: RwLockReadGuard<'static, DynMetricsRegistry>,
}

impl Metrics {
    /// A list containing all metrics that were registered via the [`metric`]
    /// attribute macro.
    ///
    /// Note that the entries may be in any order and that this order may
    /// change depending on compiler settings and the linker you are using.
    pub fn static_metrics(&self) -> &'static [MetricEntry] {
        &crate::export::METRICS
    }

    /// A list containing all metrics that were dynamically registered.
    pub fn dynamic_metrics(&self) -> DynMetricsIter {
        DynMetricsIter(self.dyn_metrics.metrics().values())
    }

    pub fn iter(&self) -> MetricsIter {
        self.into_iter()
    }
}

impl<'a> IntoIterator for &'a Metrics {
    type Item = &'a MetricEntry;
    type IntoIter = MetricsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        MetricsIter::new(self.static_metrics(), self.dyn_metrics.metrics())
    }
}

/// An iterator over all registered metrics.
///
/// See [`Metrics::static_metrics`].
pub struct MetricsIter<'a> {
    sm: std::slice::Iter<'a, MetricEntry>,
    dm: std::collections::btree_map::Values<'a, usize, MetricEntry>,
}

impl<'a> MetricsIter<'a> {
    fn new(sm: &'a [MetricEntry], dm: &'a BTreeMap<usize, MetricEntry>) -> Self {
        Self {
            sm: sm.iter(),
            dm: dm.values(),
        }
    }
}

impl<'a> Iterator for MetricsIter<'a> {
    type Item = &'a MetricEntry;

    fn next(&mut self) -> Option<Self::Item> {
        self.sm.next().or_else(|| self.dm.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (slo, shi) = self.sm.size_hint();
        let (dlo, dhi) = self.sm.size_hint();

        match (shi, dhi) {
            (Some(shi), Some(dhi)) => (slo.saturating_add(dlo), shi.checked_add(dhi)),
            _ => (slo.saturating_add(dlo), None),
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let len = self.sm.len();
        self.sm.nth(n).or_else(|| self.dm.nth(n - len))
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.sm.count() + self.dm.count()
    }

    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        let init = self.sm.fold(init, &mut f);
        self.dm.fold(init, f)
    }
}

impl<'a> DoubleEndedIterator for MetricsIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.dm.next_back().or_else(|| self.sm.next_back())
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let len = self.dm.len();
        self.dm.nth_back(n).or_else(|| self.dm.nth_back(n - len))
    }

    fn rfold<B, F>(self, init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        let init = self.dm.rfold(init, &mut f);
        self.sm.rfold(init, f)
    }
}

impl<'a> FusedIterator for MetricsIter<'a> {}

/// An iterator over all dynamically registered metrics.
///
/// See [`Metrics::dynamic_metrics`].
pub struct DynMetricsIter<'a>(std::collections::btree_map::Values<'a, usize, MetricEntry>);

impl<'a> Iterator for DynMetricsIter<'a> {
    type Item = &'a MetricEntry;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth(n)
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.0.count()
    }

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        self.0.fold(init, f)
    }
}

impl<'a> DoubleEndedIterator for DynMetricsIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth_back(n)
    }

    fn rfold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        self.0.rfold(init, f)
    }
}

impl<'a> ExactSizeIterator for DynMetricsIter<'a> {}

impl<'a> FusedIterator for DynMetricsIter<'a> {}
