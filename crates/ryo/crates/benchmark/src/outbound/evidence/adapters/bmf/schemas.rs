//! Only this boundary knows the BMF field and metric names.
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error, measurements::types::BenchmarkSample,
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
struct MetricSchema {
    value: f64,
    lower_value: f64,
    upper_value: f64,
}

/// Wire-level measurements extracted without validation or loss from one sample.
struct ObservationSchema<'a> {
    name: &'a str,
    wall: f64,
    cpu: Option<f64>,
    resident: Option<u64>,
}
impl<'a> From<&'a BenchmarkSample> for ObservationSchema<'a> {
    fn from(sample: &'a BenchmarkSample) -> Self {
        Self {
            name: &sample.name.0,
            wall: sample.measurement.wall.as_secs_f64() * 1000.0,
            cpu: match sample.measurement.resources {
                crate::domain::measurements::types::ResourceObservation::WallOnly => None,
                crate::domain::measurements::types::ResourceObservation::Collected(resources) => {
                    Some(resources.cpu.as_secs_f64() * 1000.0)
                }
            },
            resident: match sample.measurement.resources {
                crate::domain::measurements::types::ResourceObservation::WallOnly => None,
                crate::domain::measurements::types::ResourceObservation::Collected(resources) => {
                    Some(resources.peak_rss_bytes.0)
                }
            },
        }
    }
}
pub(super) fn encode(samples: &[BenchmarkSample]) -> Result<Vec<u8>, Error> {
    if samples.is_empty() {
        return Err(Error::Invalid("no measured evidence".into()));
    }
    let mut grouped: BTreeMap<&str, BTreeMap<&str, Vec<f64>>> = BTreeMap::new();
    for sample in samples.iter().map(ObservationSchema::from) {
        if sample.name.is_empty() {
            return Err(Error::Invalid("empty measurement identity".into()));
        }
        let (Some(cpu), Some(resident)) = (sample.cpu, sample.resident) else {
            return Err(Error::Invalid("resource measurements absent".into()));
        };
        let metrics = grouped.entry(sample.name).or_default();
        for (metric, value) in [
            ("wall-ms", sample.wall),
            ("cpu-ms", cpu),
            ("peak-rss-bytes", resident as f64),
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
            let _previous = summaries.insert(
                metric.to_owned(),
                MetricSchema {
                    value: median,
                    lower_value: values[0],
                    upper_value: values[values.len() - 1],
                },
            );
        }
        let _previous = document.insert(name.to_owned(), summaries);
    }
    serde_json::to_vec(&document).map_err(|error| Error::Json(Box::new(error)))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
