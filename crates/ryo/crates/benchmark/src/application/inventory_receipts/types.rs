//! Data owned by receipt validation.

/// A count of receipt pairs of one kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tally(pub usize);

/// How many validated cases were raw exact, documented discrepancies or contract-only differences.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ReceiptCounts {
    /// Cases whose two raw sides are identical.
    pub raw_exact: Tally,
    /// Cases that differ with a documented reference discrepancy.
    pub documented: Tally,
    /// Cases that differ only by the comparison contract.
    pub contract_only: Tally,
}

/// The canonical text of one diagnostic row, ordered and compared as a multiset member.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DiagnosticRow(pub String);
