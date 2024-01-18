# metriken

Easily registered distributed metrics.

`metriken` allows you to easily declare static metrics throughout your codebase.
Then, when you want to expose those metrics, you can access them all in one
place.

```rust
use metriken::{metric, Counter, Gauge, Value};

/// A counter metric named "<crate name>::COUNTER"
#[metric]
static COUNTER: Counter = Counter::new();

/// A gauge metric named "my.metric"
#[metric(name = "my.metric")]
static GAUGE: Gauge = Gauge::new();

fn main() {
    COUNTER.increment();

    for metric in &metriken::metrics() {
        let name = metric.name();

        match metric.value() {
            Some(Value::Counter(val)) => println!("{name}: {val}"),
            Some(Value::Gauge(val)) => println!("{name}: {val}"),
            _ => println!("{name}: <custom>")
        }
    }
}
```

Code updating the metrics can always access them without needing to go through
any indirections. (It just means accessing a static!). Using `linkme`, the
metrics are all gathered into a single global array that can then be used to
read all of them and expose them.
