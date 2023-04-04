// Copyright 2023 IOP Systems, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use clocksource::{DateTime, SecondsFormat};
use ratelimit::Ratelimiter;

fn main() {
    let limiter = Ratelimiter::new(10, 10, 1).unwrap();
    println!(
        "{}: Running",
        DateTime::now().to_rfc3339_opts(SecondsFormat::Millis, false),
    );
    for i in 0..100 {
        limiter.wait();
        println!(
            "{}: {}",
            DateTime::now().to_rfc3339_opts(SecondsFormat::Millis, false),
            i
        );
    }
    limiter.wait();
}
