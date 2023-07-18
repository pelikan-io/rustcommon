use crate::*;

/// Unlike a standard `Gauge`, a `LazyGauge` will not report a value unless it
/// has been initialized by writing to at least once. This is useful for when
/// you want to declare metrics statically, but only report metrics that are
/// being used.
pub type LazyGauge = Lazy<Gauge>;

impl LazyGauge {
    pub fn value(&self) -> Option<i64> {
        Lazy::get(self).map(|v| v.value())
    }
}
