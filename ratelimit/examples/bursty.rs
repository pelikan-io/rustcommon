// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use clocksource::precise::UnixInstant;
use clocksource::datetime::DateTime;
use ratelimit::{Ratelimiter, Refill};

fn main() {
    for strategy in &[Refill::Normal, Refill::Uniform] {
        println!("strategy: {:?}", strategy);
        let limiter = Ratelimiter::new(1, 1, 1);
        limiter.set_strategy(*strategy);
        for i in 0..10 {
            limiter.wait();
            println!(
                "{}: T -{}",
                DateTime::from(UnixInstant::now()),
                10 - i
            );
        }
        limiter.wait();
        println!();
    }
}
