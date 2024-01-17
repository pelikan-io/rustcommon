use std::collections::HashMap;
use std::fmt;
use std::iter::FusedIterator;

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
    /// Create a new metadata map from a hashmap.
    pub fn new(map: HashMap<String, String>) -> Self {
        Self(Impl::Dynamic(map))
    }

    pub(crate) const fn default_const() -> Self {
        const EMPTY_MAP: phf::Map<&str, &str> = phf::Map::new();

        Self::new_static(&EMPTY_MAP)
    }

    pub(crate) const fn new_static(map: &'static phf::Map<&'static str, &'static str>) -> Self {
        Self(Impl::Static(map))
    }

    /// Indicates whether this metadata instance is empty.
    pub fn is_empty(&self) -> bool {
        match &self.0 {
            Impl::Static(map) => map.is_empty(),
            Impl::Dynamic(map) => map.is_empty(),
        }
    }

    /// Get the number of entries contained within this metadata instance.
    pub fn len(&self) -> usize {
        match &self.0 {
            Impl::Static(map) => map.len(),
            Impl::Dynamic(map) => map.len(),
        }
    }

    /// Determins if `key` is in `Metadata`.
    pub fn contains_key(&self, key: &str) -> bool {
        match &self.0 {
            Impl::Static(map) => map.contains_key(key),
            Impl::Dynamic(map) => map.contains_key(key),
        }
    }

    /// Get the value that `key` corresponds to.
    pub fn get(&self, key: &str) -> Option<&str> {
        match &self.0 {
            Impl::Static(map) => map.get(key).copied(),
            Impl::Dynamic(map) => map.get(key).map(|s| &**s),
        }
    }

    /// Return an iterator over the entries of this `Metadata`.
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

impl Default for Metadata {
    fn default() -> Self {
        Self::default_const()
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

/// An iterator over the entries of a [`Metadata`].
///
/// See [`Metadata::iter`].
#[derive(Clone, Debug)]
pub struct MetadataIter<'a>(IterImpl<'a>);

impl<'a> MetadataIter<'a> {
    fn map_entries((key, value): (&&'a str, &&'a str)) -> (&'a str, &'a str) {
        (*key, *value)
    }

    fn map_iter((key, value): (&'a String, &'a String)) -> (&'a str, &'a str) {
        (&**key, &**value)
    }
}

#[derive(Clone, Debug)]
enum IterImpl<'a> {
    Static(phf::map::Entries<'a, &'static str, &'static str>),
    Dynamic(std::collections::hash_map::Iter<'a, String, String>),
}

impl<'a> Iterator for MetadataIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            IterImpl::Static(iter) => iter.next().map(Self::map_entries),
            IterImpl::Dynamic(iter) => iter.next().map(Self::map_iter),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            IterImpl::Static(iter) => iter.size_hint(),
            IterImpl::Dynamic(iter) => iter.size_hint(),
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        match &mut self.0 {
            IterImpl::Static(iter) => iter.nth(n).map(Self::map_entries),
            IterImpl::Dynamic(iter) => iter.nth(n).map(Self::map_iter),
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

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.len()
    }
}

impl<'a> ExactSizeIterator for MetadataIter<'a> {}

impl<'a> FusedIterator for MetadataIter<'a> {}
