//! The counts the BMF adapter keeps between publications.

/// The name a parity report is published under.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ReportName(pub String);

/// A count of rules or differences.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ParityCount(pub usize);

/// The counts of one parity report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ParityCounts {
    /// Rules whose findings differ.
    pub differing: ParityCount,
    /// Differences no accepted-differences entry accepts.
    pub unaccepted: ParityCount,
}

/// The name of one benchmark of the document: `<language>/<project>`, `<language>/verify/<rule>` or
/// `<language>/growth/<shape>`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BenchmarkName(pub String);

/// The name of one measure of a benchmark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MeasureName(pub &'static str);

/// Every measure added to a bench run's one document, by benchmark.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct MeasureDocument(
    pub  std::collections::BTreeMap<
        BenchmarkName,
        std::collections::BTreeMap<MeasureName, super::schemas::Observed>,
    >,
);
