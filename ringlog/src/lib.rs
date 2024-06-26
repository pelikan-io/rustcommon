// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

//! This crate provides an asynchronous logging backend that can direct logs to
//! one or more outputs.
//!
//! The core of this crate is the `RingLog` type, which is constructed using a
//! builder that is specific to your logging needs. After building the
//! `RingLog`, it can be registered as the global logger using the `start`
//! method. You will be left with a `Box<dyn Drain>` which should be
//! periodically flushed outside of any critical path. For example, in an admin
//! thread or dedicated logging thread.
//!
//! For logging to a single file, the `LogBuilder` type can be used to construct
//! an `RingLog` which has low overhead, but directs log messages to a single
//! `Output`.
//!
//! A `SamplingLogBuilder` can be used to construct an `RingLog` which will
//! filter the log messages using sampling before directing the log messages to
//! a single `Output`.
//!
//! A `MultiLogBuilder` can be used to construct an `RingLog` which routes log
//! messages based on the `target` metadata of the log `Record`. If there is an
//! `RingLog` registered for that specific `target`, then the log message will
//! be routed to that instance of `RingLog`. Log messages that do not match any
//! specific target will be routed to the default `RingLog` that has been added
//! to the `MultiLogBuilder`. If there is no default, messages that do not match
//! any specific target will be simply dropped.
//!
//! This combination of logging types allows us to compose a logging backend
//! which meets the application's needs. For example, you can use a local log
//! macro to set the target to some specific category and log those messages to
//! a file, while letting all other log messages pass to standard out. This
//! could allow splitting command/access/audit logs from the normal logging.

pub use log::*;

mod format;
#[macro_use]
mod macros;
mod multi;
mod nop;
mod outputs;
mod sampling;
mod single;
mod traits;

pub use format::*;
pub use multi::*;
pub use nop::*;
pub use outputs::*;
pub use sampling::*;
pub use single::*;
pub use traits::*;

#[cfg(feature = "metrics")]
mod metrics;

#[cfg(feature = "metrics")]
use metrics::*;

use clocksource::datetime::DateTime;
use mpmc::Queue;

pub(crate) type LogBuffer = Vec<u8>;

/// A type which implements an asynchronous logging backend.
pub struct RingLog {
    pub(crate) logger: Box<dyn Log>,
    pub(crate) drain: Box<dyn Drain>,
    pub(crate) level_filter: LevelFilter,
}

impl RingLog {
    /// Register the logger and return a type which implements `Drain`. It is
    /// up to the user to periodically call flush on the resulting drain.
    pub fn start(self) -> Box<dyn Drain> {
        let level_filter = self.level_filter;
        log::set_boxed_logger(self.logger)
            .map(|()| log::set_max_level(level_filter))
            .expect("failed to start logger");
        self.drain
    }
}
