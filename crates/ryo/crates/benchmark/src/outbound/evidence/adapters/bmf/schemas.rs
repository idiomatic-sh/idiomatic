//! Only this boundary knows the BMF field and metric names.
use crate::domain::{diagnostics::errors::Error, measurements::types::Sample};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
struct MetricSchema {
    value: f64,
    lower_value: f64,
    upper_value: f64,
}

pub(super) fn encode(samples: &[Sample]) -> Result<Vec<u8>, Error> {
    if samples.is_empty() {
        return Err(Error::Invalid("no measured evidence".into()));
    }
    let mut grouped: BTreeMap<&str, BTreeMap<&str, Vec<f64>>> = BTreeMap::new();
    for sample in samples {
        if sample.name.is_empty() {
            return Err(Error::Invalid("empty measurement identity".into()));
        }
        let resources = sample
            .measurement
            .resources
            .ok_or_else(|| Error::Invalid("resource measurements absent".into()))?;
        let metrics = grouped.entry(&sample.name).or_default();
        for (metric, value) in [
            ("wall-ms", sample.measurement.wall.as_secs_f64() * 1000.0),
            ("cpu-ms", resources.cpu.as_secs_f64() * 1000.0),
            ("peak-rss-bytes", resources.peak_rss_bytes as f64),
        ] {
            metrics.entry(metric).or_default().push(value);
        }
    }
    let mut document = BTreeMap::new();
    for (name, metrics) in grouped {
        let mut summaries = BTreeMap::new();
        for (metric, mut values) in metrics {
            values.sort_by(f64::total_cmp);
            let middle = values.len() / 2;
            let median = if values.len() % 2 == 0 {
                (values[middle - 1] + values[middle]) / 2.0
            } else {
                values[middle]
            };
            summaries.insert(
                metric,
                MetricSchema {
                    value: median,
                    lower_value: values[0],
                    upper_value: values[values.len() - 1],
                },
            );
        }
        document.insert(name, summaries);
    }
    serde_json::to_vec(&document).map_err(Into::into)
}
