use crate::*;

mod dynamic;
mod r#static;

pub use dynamic::*;
pub use r#static::*;

pub trait MetricEntry: Deref<Target = dyn Metric> {
    fn name(&self) -> Option<&str> {
        self.get_label("name")
    }

    fn description(&self) -> Option<&str> {
        self.get_label("description")
    }

    fn get_label(&self, label: &str) -> Option<&str>;

    fn metadata(&self) -> HashMap<&str, &str>;

    fn format(&self, format: Format) -> Option<String>;
}

pub struct Metrics {
    pub(crate) dynamic: RwLockReadGuard<'static, Vec<DynamicEntry>>,
}

impl Metrics {
    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }

    pub fn len(&self) -> usize {
        self.dynamic.len() + crate::__private::STATIC_REGISTRY.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a> IntoIterator for &'a Metrics {
    type Item = &'a dyn MetricEntry;

    type IntoIter = MetricIterator<'a>;
    fn into_iter(self) -> <Self as std::iter::IntoIterator>::IntoIter {
        MetricIterator {
            dynamic_index: 0,
            static_index: 0,
            metrics: self,
        }
    }
}

pub struct MetricIterator<'a> {
    dynamic_index: usize,
    static_index: usize,
    metrics: &'a Metrics,
}

impl<'a> Iterator for MetricIterator<'a> {
    type Item = &'a dyn MetricEntry;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.dynamic_index < self.metrics.dynamic.len() {
            let idx = self.dynamic_index;
            self.dynamic_index += 1;
            self.metrics.dynamic.get(idx).map(|v| v as _)
        } else {
            let idx = self.static_index;
            self.static_index += 1;
            __private::STATIC_REGISTRY.get(idx).map(|v| v as _)
        }
    }
}
