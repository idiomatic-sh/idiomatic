//! Audit, inventory and certification belong to the benchmark workflow.
pub mod audit;
pub mod certificate;
pub mod inventory;

/// Whether a profile's authored defaults own a rule's case evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaseOwnership {
    /// The rule is an explicit default and owns case evidence.
    Owns,
    /// The rule only supplies an executable query.
    Supports,
}

/// Whether a context declaration names a logical input as absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextAbsence {
    /// The context seals the input as absent.
    Sealed,
    /// The context does not name the input as absent.
    Unnamed,
}

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
impl RuleTiming {
    /// Count one case that took `elapsed`.
    pub fn record(&mut self, elapsed: std::time::Duration) {
        self.total += elapsed;
        self.slowest = self.slowest.max(elapsed);
    }
}
