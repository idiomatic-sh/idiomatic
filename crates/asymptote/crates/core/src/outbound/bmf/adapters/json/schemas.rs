//! The shape of Bencher Metric Format: each wire shape is a `<Name>Schema`, read by serde, and its checked conversion
//! into the core's type is an `impl TryFrom`.

use std::collections::BTreeMap;

use super::constants::{COLON, NEGATIVE, NO_MEASUREMENT, NO_NAME, OUTSIDE, SLASH};
use crate::domain::operations::types::{BenchmarkName, MeasureName, Number};
use crate::domain::perf::types::{Measurement, Spread};
use crate::outbound::bmf::types::Measurements;

/// One metric as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MetricSchema {
    /// The measure's median, the number the lock judges against.
    value: f64,
    /// The low end of the run's spread, when the bench reports one.
    #[serde(default)]
    lower_value: Option<f64>,
    /// The high end of the run's spread, when the bench reports one.
    #[serde(default)]
    upper_value: Option<f64>,
}

/// The text as written: benchmarks, each with its measures.
#[derive(Debug, serde::Deserialize)]
pub(super) struct MetricsSchema(BTreeMap<String, BTreeMap<String, MetricSchema>>);

/// A spread bound as the core holds it: absent when the bench reported none.
fn spread(bound: Option<f64>) -> Spread {
    bound.map_or(Spread::Absent, |bound| Spread::Given(Number(bound)))
}

impl TryFrom<MetricsSchema> for Measurements {
    type Error = String;

    fn try_from(schema: MetricsSchema) -> Result<Self, Self::Error> {
        let mut measurements: Vec<Measurement> = Vec::new();
        for (benchmark, measures) in schema.0 {
            for (measure, metric) in measures {
                if benchmark.is_empty() || measure.is_empty() {
                    return Err(format!("{benchmark:?}{SLASH}{measure:?}{COLON}{NO_NAME}"));
                }
                let numbers = [Some(metric.value), metric.lower_value, metric.upper_value];
                if numbers
                    .iter()
                    .flatten()
                    .any(|number| !number.is_finite() || *number < 0.0_f64)
                {
                    return Err(format!("{benchmark}{SLASH}{measure}{COLON}{NEGATIVE}"));
                }
                // A spread that does not hold its own value is no spread: judged by it, a run could pass as
                // improved with a value far above the best.
                if metric.lower_value.is_some_and(|lower| lower > metric.value)
                    || metric.upper_value.is_some_and(|upper| upper < metric.value)
                {
                    return Err(format!("{benchmark}{SLASH}{measure}{COLON}{OUTSIDE}"));
                }
                measurements.push(Measurement {
                    benchmark: BenchmarkName(benchmark.as_str().into()),
                    measure: MeasureName(measure.into()),
                    value: Number(metric.value),
                    lower: spread(metric.lower_value),
                    upper: spread(metric.upper_value),
                });
            }
        }
        if measurements.is_empty() {
            return Err(NO_MEASUREMENT.to_owned());
        }
        Ok(Self(measurements))
    }
}

/// The measurements `text` holds.
///
/// # Errors
///
/// The reason, when it is not Bencher Metric Format or holds no measurement.
pub(super) fn read(text: &str) -> Result<Measurements, String> {
    let schema: MetricsSchema = serde_json::from_str(text).map_err(|error| error.to_string())?;
    Measurements::try_from(schema)
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
