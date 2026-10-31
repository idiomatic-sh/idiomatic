//! Only this boundary knows the BMF field and metric names.
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    measurements::types::sample::value::BenchmarkSample,
};
use serde::Serialize;
use std::collections::BTreeMap;

/// Median and observed extrema for one BMF metric.
#[derive(Serialize)]
struct MetricSchema {
    /// Median across the complete sample group.
    value: f64,
    /// Smallest observed sample.
    lower_value: f64,
    /// Largest observed sample.
    upper_value: f64,
}

/// Wire-level measurements extracted without validation or loss from one sample.
struct ObservationSchema<'a> {
    /// Stable project, contestant and condition identity.
    name: &'a str,
    /// Target process elapsed milliseconds.
    wall: f64,
    /// Target process CPU milliseconds, absent without collection.
    cpu: Option<f64>,
    /// Target process peak resident bytes, absent without collection.
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
/// Serialize exact resource samples into grouped median and extrema metrics.
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
            return Err(Error::Invalid(super::constants::MISSING_RESOURCES.into()));
        };
        if resident > super::constants::MAX_EXACT_RESIDENT {
            return Err(Error::Invalid(super::constants::INEXACT_RESIDENT.into()));
        }
        let resident = serde_json::Number::from(resident)
            .as_f64()
            .ok_or_else(|| Error::Invalid(super::constants::INEXACT_RESIDENT.into()))?;
        let metrics = grouped.entry(sample.name).or_default();
        for (metric, value) in [
            ("wall-ms", sample.wall),
            ("cpu-ms", cpu),
            ("peak-rss-bytes", resident),
        ] {
            metrics.entry(metric).or_default().push(value);
        }
    }
    let mut document = BTreeMap::new();
    for (name, metrics) in grouped {
        let mut summaries = BTreeMap::new();
        for (metric, mut values) in metrics {
            values.sort_by(f64::total_cmp);
            let (lower_value, upper_value) = values
                .first()
                .zip(values.last())
                .ok_or_else(|| Error::Invalid(super::constants::EMPTY_METRIC.into()))?;
            let middle = values.len() / 2;
            let (lower, upper) = values.split_at(middle);
            let upper_middle = upper
                .first()
                .ok_or_else(|| Error::Invalid(super::constants::EMPTY_METRIC.into()))?;
            let median = if values.len().is_multiple_of(2_usize) {
                let lower_middle = lower
                    .last()
                    .ok_or_else(|| Error::Invalid(super::constants::EMPTY_METRIC.into()))?;
                (lower_middle + upper_middle) / 2.0
            } else {
                *upper_middle
            };
            let _previous = summaries.insert(
                metric.to_owned(),
                MetricSchema {
                    value: median,
                    lower_value: *lower_value,
                    upper_value: *upper_value,
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
