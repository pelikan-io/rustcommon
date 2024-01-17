//! Easily registered distributed metrics.
//!
//! You should usually be using the [`metriken`] crate instead. This crate
//! contains the core distributed slice used by [`metriken`] so that multiple
//! major versions of [`metriken`]` can coexist.
//!
//! [`metriken`]: https://docs.rs/metriken

use std::any::Any;
use std::borrow::Cow;

/// A helper macro for marking imports as being used.
///
/// This is meant to be used for when a reference is made to an item from a doc
/// comment but that item isn't actually used for code anywhere.
macro_rules! used_in_docs {
    ($($item:ident),* $(,)?) => {
        const _: () = {
            #[allow(unused_imports)]
            mod _docs {
                $( use super::$item; )*
            }
        };
    };
}

pub mod dynmetrics;
mod formatter;
mod metadata;
mod metrics;
mod null;

pub use crate::formatter::{default_formatter, Format};
pub use crate::metadata::{Metadata, MetadataIter};
pub use crate::metrics::{metrics, DynMetricsIter, Metrics, MetricsIter};

/// Global interface to a metric.
///
/// Most use of metrics should use the directly declared constants.
pub trait Metric: Send + Sync + 'static {
    /// Indicate whether this metric has been set up.
    ///
    /// Generally, if this returns `false` then the other methods on this
    /// trait should return `None`.
    fn is_enabled(&self) -> bool {
        self.as_any().is_some()
    }

    /// Get the current metric as an [`Any`] instance. This is meant to allow
    /// custom processing for known metric types.
    ///
    /// [`Any`]: std::any::Any
    fn as_any(&self) -> Option<&dyn Any>;

    /// Get the value of the current metric, should it be enabled.
    ///
    /// # Note to Implementors
    /// If your metric's value does not correspond to one of the variants of
    /// [`Value`] then return [`Value::Other`] and metric consumers can use
    /// [`as_any`](crate::Metric::as_any) to specifically handle your metric.
    fn value(&self) -> Option<Value>;
}

/// The value of a metric.
///
/// See [`Metric::value`].
#[non_exhaustive]
pub enum Value<'a> {
    /// A counter value.
    Counter(u64),

    /// A gauge value.
    Gauge(i64),

    /// The value of the metric could not be represented using the other `Value`
    /// variants.
    ///
    /// Use [`Metric::as_any`] to specifically handle the type of this metric.
    Other(&'a dyn Any),
}

/// A statically declared metric entry.
pub struct MetricEntry {
    metric: *const dyn Metric,
    name: Cow<'static, str>,
    description: Option<Cow<'static, str>>,
    metadata: Metadata,
    formatter: fn(&Self, Format) -> String,
}

impl MetricEntry {
    /// Get a reference to the metric that this entry corresponds to.
    pub fn metric(&self) -> &dyn Metric {
        unsafe { &*self.metric }
    }

    /// Get the name of this metric.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the description of this metric.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Access the [`Metadata`] associated with this metrics entry.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Format the metric into a string with the given format.
    pub fn formatted(&self, format: Format) -> String {
        (self.formatter)(self, format)
    }

    /// Checks whether `metric` is the metric for this entry.
    ///
    /// This checks both the type id and the address. Note that it may have
    /// false positives if `metric` is a ZST since multiple ZSTs may share
    /// the same address.
    pub fn is(&self, metric: &dyn Metric) -> bool {
        if self.metric().type_id() != metric.type_id() {
            return false;
        }

        let a = self.metric() as *const _ as *const ();
        let b = metric as *const _ as *const ();
        a == b
    }
}

unsafe impl Send for MetricEntry {}
unsafe impl Sync for MetricEntry {}

impl std::ops::Deref for MetricEntry {
    type Target = dyn Metric;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.metric()
    }
}

impl std::fmt::Debug for MetricEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetricEntry")
            .field("name", &self.name())
            .field("metric", &"<dyn Metric>")
            .finish()
    }
}

/// Implementation detail exports for use by the `#[metric]`
#[doc(hidden)]
pub mod export {
    use crate::{Metadata, Metric};

    pub extern crate linkme;
    pub extern crate phf;

    #[linkme::distributed_slice]
    pub static METRICS: [crate::MetricEntry] = [..];

    pub const fn entry_v1(
        metric: &'static dyn Metric,
        name: &'static str,
        description: Option<&'static str>,
        metadata: &'static phf::Map<&'static str, &'static str>,
        formatter: fn(&crate::MetricEntry, crate::Format) -> String,
    ) -> crate::MetricEntry {
        use std::borrow::Cow;

        crate::MetricEntry {
            metric,
            name: Cow::Borrowed(name),
            description: match description {
                Some(desc) => Some(Cow::Borrowed(desc)),
                None => None,
            },
            metadata: Metadata::new_static(metadata),
            formatter,
        }
    }
}

/// Declare a new metric.
#[macro_export]
macro_rules! declare_metric_v1 {
    {
        metric: $metric:expr,
        name: $name:expr,
        description: $description:expr,
        metadata: { $( $key:expr => $value:expr ),* $(,)? },
        formatter: $formatter:expr $(,)?
    } => {
        const _: () = {
            use $crate::export::phf;

            static __METADATA: $crate::export::phf::Map<&'static str, &'static str> =
                $crate::export::phf::phf_map! { $( $key => $value, )* };

            #[$crate::export::linkme::distributed_slice($crate::export::METRICS)]
            #[linkme(crate = $crate::export::linkme)]
            static __ENTRY: $crate::MetricEntry = $crate::export::entry_v1(
                &$metric,
                $name,
                $description,
                &__METADATA,
                $formatter
            );
        };
    }
}
