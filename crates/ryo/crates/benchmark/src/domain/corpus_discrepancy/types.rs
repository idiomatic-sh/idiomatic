//! Data of a corpus comparison's judgement.
use ryo_benchmark_contracts::domain::reports::types::{FindingCount, LocatedDifference};

/// Accepted differences admitted for one corpus: each rule's distinct accepted findings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AdmittedDifferences(
    /// The accepted findings of each rule.
    pub  std::collections::BTreeMap<
        ryo::linter::RuleId,
        std::collections::BTreeSet<LocatedDifference>,
    >,
);

/// What one selected rule's corpus comparison left unmatched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ObservedDifferences {
    /// The rule.
    pub rule: ryo::linter::RuleId,
    /// The unmatched findings, duplicates kept.
    pub differences: Vec<LocatedDifference>,
    /// How many paired findings end at different places.
    pub end_differences: FindingCount,
}

/// What a problem says of one unmatched finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProblemVerdict {
    /// The accepted differences do not name it.
    NotAccepted,
    /// The accepted differences name it, and it no longer occurs.
    NoLongerOccurs,
}
