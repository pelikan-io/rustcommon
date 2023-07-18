use metriken::Format;
use metriken::MetricEntry;
use metriken::{metric, metrics, Counter};

// Declare the metrics:

#[metric]
pub static COUNTER_NAME: Counter = Counter::new();

#[metric(name = "counter_name")]
pub static COUNTER_WITH_NAME: Counter = Counter::new();

#[metric(name = "counter_name", description = "description")]
pub static COUNTER_WITH_DESCRIPTION: Counter = Counter::new();

#[metric(name = "counter_name", description = "description", metadata = { key = "value" })]
pub static COUNTER_WITH_METADATA: Counter = Counter::new();

#[metric(name = "counter_name", description = "description", formatter = &custom_formatter)]
pub static COUNTER_WITH_FORMATTER: Counter = Counter::new();

#[metric(name = "counter_name", description = "description", formatter = &custom_formatter, metadata = { key = "value" })]
pub static COUNTER_WITH_FORMATTER_AND_METADATA: Counter = Counter::new();

pub fn custom_formatter(metric: &dyn MetricEntry, format: Format) -> String {
    match format {
        Format::Plain => {
            format!(
                "{}/{}/{}",
                metric.name(),
                "key",
                metric.get_label("key").unwrap_or("unknown")
            )
        }
        format => metriken::default_formatter(metric, format),
    }
}

const NAMES: &[&str] = &[
    "counter_name",
    "counter_name",
    "counter_name",
    "counter_name",
    "counter_name",
    "counter_name",
];

const DESCRIPTION: &[Option<&str>] = &[
    None,
    None,
    Some("description"),
    Some("description"),
    Some("description"),
    Some("description"),
];

const PLAIN: &[&str] = &[
    "counter_name",
    "counter_name",
    "counter_name",
    "counter_name",
    "counter_name/key/unknown",
    "counter_name/key/value",
];

fn main() {
    assert_eq!(metrics().iter().count(), NAMES.len());

    for (idx, metric) in metrics().iter().enumerate() {
        assert_eq!(metric.name(), NAMES[idx]);
        assert_eq!(metric.description(), DESCRIPTION[idx]);
        assert_eq!(metric.format(Format::Plain), PLAIN[idx]);
    }
}
