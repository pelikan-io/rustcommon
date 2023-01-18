// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::*;
use std::io::Error;

/// Implements a no-op logger which drops all log messages.
pub(crate) struct NopLogger {}

impl NopLogger {
    pub fn level_filter(&self) -> LevelFilter {
        LevelFilter::Off
    }
}

impl Log for NopLogger {
    fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
        false
    }

    fn log(&self, _record: &log::Record<'_>) {}

    fn flush(&self) {}
}

/// Implements a no-op drain type which does nothing.
pub(crate) struct NopLogDrain {}

impl Drain for NopLogDrain {
    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

/// A type to construct a basic `RingLog` which drops all log messages.
pub struct NopLogBuilder {}

impl Default for NopLogBuilder {
    fn default() -> Self {
        Self {}
    }
}

impl NopLogBuilder {
    /// Create a new log builder.
    pub fn new() -> Self {
        Default::default()
    }

    /// Consumes the builder and returns an `RingLog`.
    pub fn build(self) -> RingLog {
        let logger = NopLogger {};
        let drain = NopLogDrain {};
        let level_filter = logger.level_filter();
        RingLog {
            logger: Box::new(logger),
            drain: Box::new(drain),
            level_filter,
        }
    }
}
