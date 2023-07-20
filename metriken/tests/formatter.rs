use metriken::*;

fn custom_formatter(metric: &MetricEntry, format: Format) -> String {
    match format {
        Format::Simple => {
            format!("{}_instance_{}", metric.name(), metric.metadata().get("instance").unwrap_or("unknown"))
        },
        _ => metriken::default_formatter(metric, format),
    }
}

#[metric(name = "metric", metadata = { instance = "a"}, formatter = &custom_formatter)]
static METRIC_A: Counter = Counter::new();

#[metric(name = "metric", metadata = { instance = "b"}, formatter = &custom_formatter)]
static METRIC_B: Counter = Counter::new();

#[test]
fn metric_name_as_expected() {
    let metrics = metrics().static_metrics();
    let metric = metrics //
        .iter()
        .find(|entry| entry.is(&METRIC_A))
        .unwrap();

    assert_eq!(metrics.len(), 2);
    assert_eq!(metric.name(), "metric");
    assert_eq!(metric.formatted(Format::Simple), "metric_instance_a");
}

#[test]
fn metric_name_and_description_as_expected() {
    let metrics = metrics().static_metrics();
    let metric = metrics
        .iter()
        .find(|entry| entry.is(&METRIC_B))
        .unwrap();

    assert_eq!(metrics.len(), 2);
    assert_eq!(metric.name(), "metric");
    assert_eq!(metric.formatted(Format::Simple), "metric_instance_b");
}
