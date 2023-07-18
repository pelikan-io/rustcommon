// use std::collections::btree_set::Iter;
use crate::*;
use std::collections::BTreeSet;

mod dynamic;
mod r#static;

pub use dynamic::*;
pub use r#static::*;

pub trait MetricEntry: Deref<Target = dyn Metric> {
    fn name(&self) -> &str;

    fn description(&self) -> Option<&str>;

    fn get_label(&self, label: &str) -> Option<&str>;

    fn metadata(&self) -> HashMap<&str, &str>;

    fn format(&self, format: Format) -> String;
}

pub struct Metrics {
    pub(crate) dynamic: RwLockReadGuard<'static, BTreeSet<DynamicEntry>>,
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
            dynamic_iter: Some(self.dynamic.iter()),
            static_iter: __private::STATIC_REGISTRY.iter(),
        }
    }
}

pub struct MetricIterator<'a> {
    dynamic_iter: Option<std::collections::btree_set::Iter<'a, DynamicEntry>>,
    static_iter: core::slice::Iter<'a, StaticEntry>,
}

impl<'a> Iterator for MetricIterator<'a> {
    type Item = &'a dyn MetricEntry;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if let Some(ref mut iter) = &mut self.dynamic_iter {
            match iter.next() {
                Some(v) => {
                    return Some(v as _);
                }
                None => {
                    self.dynamic_iter = None;
                }
            }
        }

        self.static_iter.next().map(|v| v as _)
    }
}
