// Copyright 2022 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use metriken::*;

#[metric(description = "some metric with a description")]
static METRIC_WITH_DESCRIPTION_NO_NAME: Counter = Counter::new();

#[metric]
static METRIC_WITH_BLANK_DESCRIPTION: Counter = Counter::new();

#[test]
fn metric_description_as_expected_when_only_description_set() {
    let metrics = metrics().static_metrics();
    let metric = metrics
        .iter()
        .find(|entry| entry.is(&METRIC_WITH_DESCRIPTION_NO_NAME))
        .unwrap();

    assert_eq!(metrics.len(), 2);
    assert_eq!(metric.description(), Some("some metric with a description"));
}

#[test]
fn metric_description_as_expected_when_only_description_set_to_blank() {
    let metrics = metrics().static_metrics();
    let metric = metrics
        .iter()
        .find(|entry| entry.is(&METRIC_WITH_BLANK_DESCRIPTION))
        .unwrap();

    assert_eq!(metrics.len(), 2);
    assert_eq!(metric.description(), None);
}
