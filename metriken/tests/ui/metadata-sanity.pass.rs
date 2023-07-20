#[allow(unused_imports)]
use metriken::{metric, Counter};

#[metric(
    metadata = {
        "a.value" = "b",
        test = "c"
    }
)]
static DUMMY: Counter = Counter::new();

fn main() {}
