
#[allow(unused_imports)]
use metriken::{metric, Counter};

#[metric(
    description = "a dummy metric",
    metadata = {
        description = "no really",
    }
)]
static DUMMY: Counter = Counter::new();

fn main() {}
