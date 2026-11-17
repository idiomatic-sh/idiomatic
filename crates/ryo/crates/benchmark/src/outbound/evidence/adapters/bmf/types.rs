//! The names and numbers of the one document the BMF adapter writes.

/// The name of one benchmark of the document: `<language>/<project>`, `<language>/verify/<rule>` or
/// `<language>/growth/<shape>`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BenchmarkName(pub String);

/// The name of one measure of a benchmark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MeasureName(pub &'static str);

/// One number of a measure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MeasureValue(pub f64);

/// What one measure observed: its value, and the lowest and highest it was.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Observed {
    /// The median of the trials, or the one observation.
    pub value: MeasureValue,
    /// The lowest trial.
    pub lower: MeasureValue,
    /// The highest trial.
    pub upper: MeasureValue,
}

/// Every measure added to a bench run's one document, by benchmark.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct MeasureDocument(
    pub std::collections::BTreeMap<BenchmarkName, std::collections::BTreeMap<MeasureName, Observed>>,
);

/// The one document of a run, encoded and ready to write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EncodedDocument(pub Vec<u8>);
