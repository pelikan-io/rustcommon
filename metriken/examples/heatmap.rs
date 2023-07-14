use core::time::Duration;
use metriken::Format;
use metriken::MetricEntry;
use metriken::{metric, metrics, Heatmap};

// Declare the metrics:

#[metric]
pub static HEATMAP_NAME: Heatmap =
    Heatmap::new(0, 8, 64, Duration::from_secs(60), Duration::from_secs(1));

#[metric(name = "heatmap_name")]
pub static HEATMAP_WITH_NAME: Heatmap =
    Heatmap::new(0, 8, 64, Duration::from_secs(60), Duration::from_secs(1));

#[metric(name = "heatmap_name", description = "description")]
pub static HEATMAP_WITH_DESCRIPTION: Heatmap =
    Heatmap::new(0, 8, 64, Duration::from_secs(60), Duration::from_secs(1));

#[metric(name = "heatmap_name", description = "description", key = "value")]
pub static HEATMAP_WITH_METADATA: Heatmap =
    Heatmap::new(0, 8, 64, Duration::from_secs(60), Duration::from_secs(1));

#[metric(name = "heatmap_name", description = "description", formatter = &custom_formatter)]
pub static HEATMAP_WITH_FORMATTER: Heatmap =
    Heatmap::new(0, 8, 64, Duration::from_secs(60), Duration::from_secs(1));

#[metric(name = "heatmap_name", description = "description", formatter = &custom_formatter, key = "value")]
pub static HEATMAP_WITH_FORMATTER_AND_METADATA: Heatmap =
    Heatmap::new(0, 8, 64, Duration::from_secs(60), Duration::from_secs(1));

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
    "metric_name",
    "metric_name",
    "metric_name",
    "metric_name",
    "metric_name",
    "metric_name",
    "heatmap_name",
    "heatmap_name",
    "heatmap_name",
    "heatmap_name",
    "heatmap_name",
    "heatmap_name",
];

const DESCRIPTION: &[Option<&str>] = &[
    None,
    None,
    Some("description"),
    Some("description"),
    Some("description"),
    Some("description"),
    None,
    None,
    Some("description"),
    Some("description"),
    Some("description"),
    Some("description"),
];

const PLAIN: &[&str] = &[
    "metric_name",
    "metric_name",
    "metric_name",
    "metric_name",
    "metric_name/key/unknown",
    "metric_name/key/value",
    "heatmap_name",
    "heatmap_name",
    "heatmap_name",
    "heatmap_name",
    "heatmap_name/key/unknown",
    "heatmap_name/key/value",
];

fn main() {
    assert_eq!(metrics().iter().count(), NAMES.len());

    for (idx, metric) in metrics().iter().enumerate() {
        assert_eq!(metric.name().unwrap(), NAMES[idx]);
        assert_eq!(metric.description(), DESCRIPTION[idx]);
        assert_eq!(metric.format(Format::Plain).unwrap(), PLAIN[idx]);
    }
}
