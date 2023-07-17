#[allow(unused_imports)]
use metriken::{metric, Counter};

#[metric(
    name = "a",
    name = "b"
)]
static DUMMY: Counter = Counter::new();

fn main() {}
