use crate::*;

/// Unlike a standard `Counter`, a `LazyCounter` will not report a value unless
/// it has been initialized by writing to at least once. This is useful for when
/// you want to declare metrics statically, but only report metrics that are
/// being used.
pub type LazyCounter = Lazy<Counter>;

impl LazyCounter {
    pub fn value(&self) -> Option<u64> {
        Lazy::get(self).map(|v| v.value())
    }
}
