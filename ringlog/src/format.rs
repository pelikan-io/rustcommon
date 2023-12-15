// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::*;

use clocksource::datetime::{DateTime};

pub type FormatFunction = fn(
    write: &mut dyn std::io::Write,
    now: DateTime,
    record: &Record,
) -> Result<(), std::io::Error>;

pub fn default_format(
    w: &mut dyn std::io::Write,
    now: DateTime,
    record: &Record,
) -> Result<(), std::io::Error> {
    writeln!(
        w,
        "{} {} [{}] {}",
        now,
        record.level(),
        record.module_path().unwrap_or("<unnamed>"),
        record.args()
    )
}

pub fn klog_format(
    w: &mut dyn std::io::Write,
    now: DateTime,
    record: &Record,
) -> Result<(), std::io::Error> {
    writeln!(
        w,
        "{} {}",
        now,
        record.args()
    )
}
