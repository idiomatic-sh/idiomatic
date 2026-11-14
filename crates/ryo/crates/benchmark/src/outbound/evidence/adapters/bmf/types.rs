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
