use metriken::Format;
use metriken::MetricEntry;
use metriken::{metric, metrics, Gauge};

// Declare the metrics:

#[metric]
pub static GAUGE_NAME: Gauge = Gauge::new();

#[metric(name = "gauge_name")]
pub static GAUGE_WITH_NAME: Gauge = Gauge::new();

#[metric(name = "gauge_name", description = "description")]
pub static GAUGE_WITH_DESCRIPTION: Gauge = Gauge::new();

#[metric(name = "gauge_name", description = "description", key = "value")]
pub static GAUGE_WITH_METADATA: Gauge = Gauge::new();

#[metric(name = "gauge_name", description = "description", formatter = &custom_formatter)]
pub static GAUGE_WITH_FORMATTER: Gauge = Gauge::new();

#[metric(name = "gauge_name", description = "description", formatter = &custom_formatter, key = "value")]
pub static GAUGE_WITH_FORMATTER_AND_METADATA: Gauge = Gauge::new();

pub fn custom_formatter(metric: &dyn MetricEntry, format: Format) -> Option<String> {
    match format {
        Format::Plain => metric.name().map(|v| {
            format!(
                "{}/{}/{}",
                v,
                "key",
                metric.get_label("key").unwrap_or("unknown")
            )
        }),
        format => metriken::default_formatter(metric, format),
    }
}

const NAMES: &[&str] = &[
    "gauge_name",
    "gauge_name",
    "gauge_name",
    "gauge_name",
    "gauge_name",
    "gauge_name",
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
    "gauge_name",
    "gauge_name",
    "gauge_name",
    "gauge_name",
    "gauge_name/key/unknown",
    "gauge_name/key/value",
];

fn main() {
    assert_eq!(metrics().iter().count(), NAMES.len());

    for (idx, metric) in metrics().iter().enumerate() {
        assert_eq!(metric.name().unwrap(), NAMES[idx]);
        assert_eq!(metric.description(), DESCRIPTION[idx]);
        assert_eq!(metric.format(Format::Plain).unwrap(), PLAIN[idx]);
    }
}
