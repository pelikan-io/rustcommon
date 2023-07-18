use std::collections::HashMap;
use std::fmt;

/// Metadata for a metric.
///
/// Metrics can have arbitrary key-value pairs stored as metadata. This allows
/// for labelling them with whatever metadata you may find relevant.
#[derive(Clone)]
pub struct Metadata(Impl);

#[derive(Clone)]
enum Impl {
    Static(&'static phf::Map<&'static str, &'static str>),
    Dynamic(HashMap<String, String>),
}

impl Metadata {
    pub fn new(map: HashMap<String, String>) -> Self {
        Self(Impl::Dynamic(map))
    }

    pub(crate) const fn new_static(map: &'static phf::Map<&'static str, &'static str>) -> Self {
        Self(Impl::Static(map))
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        match &self.0 {
            Impl::Static(map) => map.get(key).copied(),
            Impl::Dynamic(map) => map.get(key).map(|s| &**s),
        }
    }

    pub fn iter(&self) -> MetadataIter {
        MetadataIter(match &self.0 {
            Impl::Static(map) => IterImpl::Static(map.entries()),
            Impl::Dynamic(map) => IterImpl::Dynamic(map.iter()),
        })
    }
}

impl From<HashMap<String, String>> for Metadata {
    fn from(value: HashMap<String, String>) -> Self {
        Self(Impl::Dynamic(value))
    }
}

impl fmt::Debug for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Impl::Static(map) => map.fmt(f),
            Impl::Dynamic(map) => map.fmt(f),
        }
    }
}

impl<'a> IntoIterator for &'a Metadata {
    type Item = (&'a str, &'a str);
    type IntoIter = MetadataIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct MetadataIter<'a>(IterImpl<'a>);

enum IterImpl<'a> {
    Static(phf::map::Entries<'a, &'static str, &'static str>),
    Dynamic(std::collections::hash_map::Iter<'a, String, String>),
}

impl<'a> Iterator for MetadataIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            IterImpl::Static(iter) => iter.next().map(|(k, v)| (*k, *v)),
            IterImpl::Dynamic(iter) => iter.next().map(|(k, v)| (&**k, &**v)),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            IterImpl::Static(iter) => iter.size_hint(),
            IterImpl::Dynamic(iter) => iter.size_hint(),
        }
    }

    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        match self.0 {
            IterImpl::Static(iter) => iter.fold(init, move |acc, (k, v)| f(acc, (*k, *v))),
            IterImpl::Dynamic(iter) => iter.fold(init, move |acc, (k, v)| f(acc, (&**k, &**v))),
        }
    }
}
