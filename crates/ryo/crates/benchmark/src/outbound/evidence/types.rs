//! What one rule's verification took and where the inventory left it.

/// How long the cases of one rule took to verify.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuleTiming {
    /// The time of every case of the rule together.
    pub total: std::time::Duration,
    /// The time of the rule's slowest case.
    pub slowest: std::time::Duration,
    /// The time the rule's query took to compile before its first case, none where each case compiles for itself.
    pub compile: std::time::Duration,
}

/// Every verified rule's times.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleTimings(pub std::collections::BTreeMap<ryo::linter::RuleId, RuleTiming>);

/// The file one rule's verification writes its times to: `<inventory>/<rule>/verify-times.bmf`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationTimesFile(pub std::path::PathBuf);

/// The times file of every default the inventory verified (PASS), by rule; a DEFERRED default has none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VerificationTimesFiles(
    pub std::collections::BTreeMap<ryo::linter::RuleId, VerificationTimesFile>,
);
