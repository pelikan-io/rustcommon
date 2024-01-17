use crate::MetricEntry;

#[non_exhaustive]
pub enum Format {
    /// A metrics format that represents the metric using only a simple name. If
    /// a metric's name depends on metadata associated with it, the formatter
    /// should ensure that the output name is unique.
    ///
    /// Examples:
    /// `cpu/usage/user`
    /// `cpu/usage/system`
    /// `network/eth0/transmit/bytes`
    Simple,
    /// A metrics format that represents the metric using a name and associated
    /// metadata in the Prometheus exposition format.
    ///
    /// Examples:
    /// `cpu_usage{mode="user"}`
    /// `cpu_usage{mode="system"}`
    /// `network_transmit_bytes{device="eth0"}`
    Prometheus,
}

/// The default formatter supports Prometheus-style exposition, and otherwise
/// simply prints the metric name.
pub fn default_formatter(metric: &MetricEntry, format: Format) -> String {
    match format {
        Format::Prometheus => {
            let metadata: Vec<String> = metric
                .metadata()
                .iter()
                .map(|(key, value)| format!("{key}=\"{value}\""))
                .collect();
            let metadata = metadata.join(", ");

            if metadata.is_empty() {
                metric.name().to_string()
            } else {
                format!("{}{{{metadata}}}", metric.name())
            }
        }
        _ => metric.name().to_string(),
    }
}
