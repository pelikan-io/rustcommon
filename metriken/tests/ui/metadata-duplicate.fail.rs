#[allow(unused_imports)]
use metriken::{metric, Counter};

#[metric(
    metadata = {
        "a" = "test",
        "a" = "value"
    }
)]
static DUMMY: Counter = Counter::new();

fn main() {}
