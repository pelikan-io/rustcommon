// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::Metric;
use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

// Note: This implementation is mostly copied from the Lazy implementation
//       within once_cell. It only adds the `get` option to try to access
//       the value without initializing the Lazy instance.
//
//       This should be replaced with the new primitives in std::lazy once
//       those stabilize.

/// A value which is initialized on the first access.
///
/// This type is thread-safe and can be used in statics. It uses the [`Default`]
/// implementation for `T` in order to initialize the inner value.
pub struct Lazy<T> {
    cell: OnceLock<T>,
}

impl<T> Lazy<T> {
    /// Create a new lazy value with the given initializing function.
    pub const fn new() -> Self {
        Self {
            cell: OnceLock::new(),
        }
    }

    /// If this lazy has been initialized, then return a reference to the
    /// contained value.
    pub fn get(this: &Self) -> Option<&T> {
        this.cell.get()
    }

    /// If this lazy has been initialized, then return a reference to the
    /// contained value.
    pub fn get_mut(this: &mut Self) -> Option<&mut T> {
        this.cell.get_mut()
    }
}

impl<T: Default> Lazy<T> {
    /// Force the evaluation of this lazy value and return a reference to
    /// the result. This is equivalent to the `Deref` impl.
    pub fn force(this: &Self) -> &T {
        this.cell.get_or_init(T::default)
    }
}

impl<T: Default> Deref for Lazy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        Self::force(self)
    }
}

impl<T: Default> DerefMut for Lazy<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::force(self);
        self.cell.get_mut().unwrap_or_else(|| unreachable!())
    }
}

impl<T> Default for Lazy<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Metric> Metric for Lazy<T> {
    fn is_enabled(&self) -> bool {
        Lazy::get(self).is_some()
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        match Lazy::get(self) {
            Some(metric) => Some(metric),
            None => None,
        }
    }
}
