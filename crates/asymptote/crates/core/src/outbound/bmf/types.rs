//! The types of the `bmf` port: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::perf::types::Measurement;

/// A tool's Bencher Metric Format output.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BmfText(pub Box<str>);

/// What a text measures: sorted by benchmark, then measure.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Measurements(pub Vec<Measurement>);
