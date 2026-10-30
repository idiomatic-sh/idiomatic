//! The fake of the `bmf` port: the measurements each text stands for, given up front.

use std::collections::BTreeMap;

use crate::domain::operations::types::{BenchmarkName, MeasureName, Number};
use crate::domain::perf::types::{Measurement, Spread};
use crate::outbound::bmf::errors::BmfError;
use crate::outbound::bmf::interface::BmfPort;
use crate::outbound::bmf::types::{BmfText, Measurements};

/// What the core's tests stand in for `bmf` with.
#[derive(Debug, Default)]
pub struct BmfFake {
    /// What each text reads as, by the text itself.
    texts: BTreeMap<Box<str>, Measurements>,
}

impl BmfPort for BmfFake {
    fn read(&self, text: &BmfText) -> Result<Measurements, BmfError> {
        self.texts
            .get(&text.0)
            .cloned()
            .ok_or_else(|| BmfError::Unparsable {
                reason: "the fake holds no such text".to_owned(),
            })
    }
}

/// A fake whose text `said` reads as `measurements`.
#[must_use]
pub fn reading(said: &str, measurements: Vec<Measurement>) -> BmfFake {
    BmfFake {
        texts: BTreeMap::from([(said.into(), Measurements(measurements))]),
    }
}

/// The measurement of `measure` of `benchmark` at `value`, with no spread.
#[must_use]
pub fn measurement(benchmark: &str, measure: &str, value: f64) -> Measurement {
    Measurement {
        benchmark: BenchmarkName(benchmark.into()),
        measure: MeasureName(measure.into()),
        value: Number(value),
        lower: Spread::Absent,
        upper: Spread::Absent,
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
