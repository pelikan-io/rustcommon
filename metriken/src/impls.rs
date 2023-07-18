//! Various impls for [`Metric`].
//! 
//! This way they avoid cluttering up lib.rs.

use std::any::Any;
use std::sync::{Arc, OnceLock};

use crate::Metric;

impl<T: Metric> Metric for &'static T {
    fn is_enabled(&self) -> bool {
        <T as Metric>::is_enabled(self)
    }

    fn as_any(&self) -> Option<&dyn Any> {
        <T as Metric>::as_any(self)
    }
}

impl<T: Metric> Metric for Box<T> {
    fn is_enabled(&self) -> bool {
        <T as Metric>::is_enabled(&self)
    }

    fn as_any(&self) -> Option<&dyn Any> {
        <T as Metric>::as_any(&self)
    }
}

impl<T: Metric> Metric for Arc<T> {
    fn is_enabled(&self) -> bool {
        <T as Metric>::is_enabled(&self)
    }

    fn as_any(&self) -> Option<&dyn Any> {
        <T as Metric>::as_any(&self)
    }
}

impl<T: Metric> Metric for OnceLock<T> {
    fn is_enabled(&self) -> bool {
        self.get().map(T::is_enabled).unwrap_or(false)
    }

    fn as_any(&self) -> Option<&dyn Any> {
        self.get().and_then(T::as_any)
    }
}

impl Metric for ::heatmap::Heatmap {
    fn is_enabled(&self) -> bool {
        true
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}

impl Metric for ::histogram::Histogram {
    fn is_enabled(&self) -> bool {
        true
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}
